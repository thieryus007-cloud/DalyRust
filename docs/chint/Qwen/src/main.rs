use actix_web::{web, App, HttpResponse, HttpServer, Responder, HttpRequest, middleware};
use actix_files::NamedFile;
use actix_cors::Cors;
use serde::{Serialize, Deserialize};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use serialport::{self, SerialPort, ClearBuffer};
use std::io::{Write, Read};
use std::thread;
use chrono::Local;
use std::env;
use dotenv::dotenv;
use log::{info, error};

// ==================== CONFIGURATION ====================
#[derive(Clone)]
struct Config {
    port_name: String,
    baud_rate: u32,
    parity: serialport::Parity,
    modbus_addr: u8,
    host: String,
    port_http: u16,
    debug_enabled: bool,
}

impl Config {
    fn from_env() -> Self {
        dotenv().ok();
        let parity_str = env::var("SERIAL_PARITY").unwrap_or_else(|_| "Even".to_string());
        let parity = match parity_str.as_str() {
            "None" => serialport::Parity::None,
            "Odd" => serialport::Parity::Odd,
            _ => serialport::Parity::Even,
        };

        Config {
            port_name: env::var("SERIAL_PORT").unwrap_or_else(|_| "COM5".to_string()),
            baud_rate: env::var("SERIAL_BAUD").unwrap_or_else(|_| "9600".to_string()).parse().unwrap_or(9600),
            parity,
            modbus_addr: env::var("MODBUS_ADDR").unwrap_or_else(|_| "6".to_string()).parse().unwrap_or(6),
            host: env::var("SERVER_HOST").unwrap_or_else(|_| "127.0.0.1".to_string()),
            port_http: env::var("SERVER_PORT").unwrap_or_else(|_| "5000".to_string()).parse().unwrap_or(5000),
            debug_enabled: env::var("DEBUG_LOG_ENABLED").unwrap_or_else(|_| "false".to_string()) == "true",
        }
    }
}

// ==================== STRUCTURES ====================
struct AppState {
    config: Config,
    port: Mutex<Option<Box<dyn SerialPort + Send>>>,
    debug_log: Mutex<bool>,
    model_type: Mutex<String>,
    last_success: Mutex<Instant>,
    last_error: Mutex<Option<String>>,
}

#[derive(Serialize)]
struct ModbusResponse {
    success: bool,
    values: std::collections::HashMap<String, String>,
    model: String,
    error: Option<String>,
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct RegValue {
    value: u16,
}

#[derive(Deserialize)]
struct RawFrame {
    frame: String,
}

// ==================== LOGS ====================
fn write_debug_log(message: &str, debug: bool) {
    if !debug { return; }
    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
    let log_line = format!("[{}] {}\n", timestamp, message);
    let _ = std::fs::OpenOptions::new().create(true).append(true)
        .open("modbus_debug.log").and_then(|mut f| f.write_all(log_line.as_bytes()));
}

fn write_command_log(send: &str, recv: &str) {
    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
    let log_line = format!("[{}] SEND: {} | RECV: {}\n", timestamp, send, recv);
    let _ = std::fs::OpenOptions::new().create(true).append(true)
        .open("modbus_commands.log").and_then(|mut f| f.write_all(log_line.as_bytes()));
}

// ==================== CRC & FRAME ====================
fn calculate_crc(data: &[u8]) -> u16 {
    let mut crc = 0xFFFFu16;
    for &byte in data {
        crc ^= byte as u16;
        for _ in 0..8 {
            if crc & 0x0001 != 0 {
                crc = (crc >> 1) ^ 0xA001;
            } else {
                crc >>= 1;
            }
        }
    }
    crc
}

fn build_frame(addr: u8, func: u8, reg: u16, value: Option<u16>) -> Vec<u8> {
    let mut data = vec![addr, func, (reg >> 8) as u8, reg as u8];
    if func == 0x03 {
        data.extend_from_slice(&[0x00, 0x01]);
    } else if func == 0x06 {
        if let Some(v) = value {
            data.extend_from_slice(&[(v >> 8) as u8, v as u8]);
        }
    }
    let crc = calculate_crc(&data);
    data.push((crc & 0xFF) as u8);
    data.push((crc >> 8) as u8);
    data
}

// ==================== PORT & RETRY ====================
fn open_port(cfg: &Config) -> Option<Box<dyn SerialPort + Send>> {
    match serialport::new(&cfg.port_name, cfg.baud_rate)
        .data_bits(serialport::DataBits::Eight)
        .parity(cfg.parity)
        .stop_bits(serialport::StopBits::One)
        .timeout(Duration::from_millis(600))
        .open()
    {
        Ok(p) => {
            info!("Port série {} ouvert avec succès", cfg.port_name);
            Some(p as Box<dyn SerialPort + Send>)
        },
        Err(e) => {
            error!("Erreur ouverture port {}: {}", cfg.port_name, e);
            None
        }
    }
}

fn with_retry<F, T>(mut op: F, max_retries: u8, debug: bool, prefix: &str) -> Option<T>
where
    F: FnMut() -> Option<T>,
{
    for attempt in 0..=max_retries {
        if let Some(res) = op() { return Some(res); }
        if attempt < max_retries {
            let delay = 50u64 * (1u64 << attempt.min(5));
            thread::sleep(Duration::from_millis(delay));
            if debug {
                write_debug_log(&format!("{} - Retry {}/{}", prefix, attempt + 1, max_retries), debug);
            }
        }
    }
    None
}

fn read_register(state: &AppState, addr: u8, reg: u16, debug: bool) -> Option<u16> {
    let frame = build_frame(addr, 0x03, reg, None);
    if debug { write_debug_log(&format!("📤 READ 0x{:04X}", reg), debug); }
    
    let res = with_retry(|| {
        let mut guard = state.port.lock().unwrap();
        let port = match guard.as_mut() {
            Some(p) => p,
            None => {
                if let Some(newp) = open_port(&state.config) {
                    *guard = Some(newp);
                    guard.as_mut().unwrap()
                } else { return None; }
            }
        };
        
        let _ = port.clear(ClearBuffer::All);
        let _ = port.write_all(&frame);
        thread::sleep(Duration::from_millis(90));
        
        let mut buf = vec![0u8; 256];
        match port.read(&mut buf) {
            Ok(n) if n >= 5 => {
                let resp = &buf[0..n];
                if resp[1] == 0x83 { None }
                else if resp[1] == 0x03 && resp.len() >= 5 {
                    Some(((resp[3] as u16) << 8) | resp[4] as u16)
                } else { None }
            }
            _ => None,
        }
    }, 3, debug, &format!("Read 0x{:04X}", reg));

    if res.is_some() {
        *state.last_success.lock().unwrap() = Instant::now();
        *state.last_error.lock().unwrap() = None;
    } else if debug {
        write_debug_log(&format!("❌ Échec lecture 0x{:04X}", reg), debug);
    }
    res
}

fn write_register(state: &AppState, addr: u8, reg: u16, value: u16, debug: bool) -> bool {
    let frame = build_frame(addr, 0x06, reg, Some(value));
    if debug { write_debug_log(&format!("📝 WRITE 0x{:04X} = {}", reg, value), debug); }
    
    let success = with_retry(|| {
        let mut guard = state.port.lock().unwrap();
        let port = match guard.as_mut() {
            Some(p) => p,
            None => {
                if let Some(newp) = open_port(&state.config) {
                    *guard = Some(newp);
                    guard.as_mut().unwrap()
                } else { return None; }
            }
        };

        let _ = port.clear(ClearBuffer::All);
        let _ = port.write_all(&frame);
        thread::sleep(Duration::from_millis(90));
        
        let mut buf = vec![0u8; 256];
        match port.read(&mut buf) {
            Ok(n) if n > 0 => Some(buf[1] != 0x86),
            _ => None,
        }
    }, 3, debug, &format!("Write 0x{:04X}", reg)).unwrap_or(false);

    if success {
        *state.last_success.lock().unwrap() = Instant::now();
        *state.last_error.lock().unwrap() = None;
    } else if debug {
        write_debug_log(&format!("❌ Échec écriture 0x{:04X}", reg), debug);
    }
    success
}

fn send_raw_frame(state: &AppState, frame_hex: &str, debug: bool) -> Result<(Vec<u8>, Vec<u8>), String> {
    let bytes: Result<Vec<u8>, _> = frame_hex.split_whitespace().map(|b| u8::from_str_radix(b, 16)).collect();
    let frame = match bytes { Ok(f) => f, Err(e) => return Err(format!("Trame invalide: {}", e)) };
    
    if debug { write_debug_log(&format!("📤 RAW: {:02X?}", frame), debug); }
    
    let res = with_retry(|| {
        let mut guard = state.port.lock().unwrap();
        let port = match guard.as_mut() {
            Some(p) => p,
            None => {
                if let Some(newp) = open_port(&state.config) {
                    *guard = Some(newp);
                    guard.as_mut().unwrap()
                } else { return None; }
            }
        };

        let _ = port.clear(ClearBuffer::All);
        let _ = port.write_all(&frame);
        thread::sleep(Duration::from_millis(150));
        
        let mut buf = vec![0u8; 256];
        match port.read(&mut buf) {
            Ok(n) if n > 0 => Some((frame.clone(), buf[0..n].to_vec())),
            _ => None,
        }
    }, 2, debug, "Raw frame");

    match res {
        Some((s, r)) => { *state.last_success.lock().unwrap() = Instant::now(); Ok((s, r)) }
        None => Err("Timeout".to_string()),
    }
}

fn detect_model(state: &AppState, addr: u8, debug: bool) -> String {
    if read_register(state, addr, 0x2065, debug).is_some() { 
        "MN".to_string() 
    } else { 
        "BN".to_string() 
    }
}

// ==================== MONITORING ====================
async fn start_monitoring(state: web::Data<Arc<Mutex<AppState>>>) {
    let clone = state.clone();
    actix_web::rt::spawn(async move {
        loop {
            actix_web::rt::time::sleep(Duration::from_secs(10)).await;
            let app = clone.lock().unwrap();
            let debug = *app.debug_log.lock().unwrap();
            let model = app.model_type.lock().unwrap().clone();
            let test_reg = if model == "MN" { 0x2065u16 } else { 0x0050u16 };
            
            if read_register(&app, app.config.modbus_addr, test_reg, debug).is_none() {
                write_debug_log("⚠️ Monitoring : tentative de reconnexion du port...", debug);
                let mut g = app.port.lock().unwrap();
                *g = None;
                *g = open_port(&app.config);
            }
        }
    });
}

// ==================== ROUTES ====================
async fn index(req: HttpRequest) -> impl Responder {
    match NamedFile::open_async("index.html").await {
        Ok(f) => f.into_response(&req),
        Err(_) => HttpResponse::NotFound().body("index.html non trouvé"),
    }
}

async fn health(data: web::Data<Arc<Mutex<AppState>>>) -> impl Responder {
    let state = data.lock().unwrap();
    let success = state.last_success.lock().unwrap().elapsed().as_secs() < 60;
    let model = state.model_type.lock().unwrap().clone();
    HttpResponse::Ok().json(serde_json::json!({
        "status": if success { "healthy" } else { "degraded" },
        "model": model,
        "port": state.config.port_name
    }))
}

async fn read_all(data: web::Data<Arc<Mutex<AppState>>>) -> impl Responder {
    let state = data.clone();
    let result = actix_web::rt::task::spawn_blocking(move || {
        let guard = state.lock().unwrap();
        let debug = *guard.debug_log.lock().unwrap();
        let model = guard.model_type.lock().unwrap().clone();
        let addr = guard.config.modbus_addr;
        
        let mut values = std::collections::HashMap::new();
        let fmt_v = |x: u16| format!("{} V", x);
        let fmt_ver = |x: u16| format!("{:.2}", x as f32 / 100.0);
        let fmt_cnt = |x: u16| x.to_string();
        let fmt_h = |x: u16| format!("{} h", x);
        let fmt_s = |x: u16| format!("{} s", x);

        let regs = vec![
            (0x0006, "v1a", Box::new(fmt_v) as Box<dyn Fn(u16)->String>),
            (0x0007, "v1b", Box::new(fmt_v)),
            (0x0008, "v1c", Box::new(fmt_v)),
            (0x0009, "v2a", Box::new(fmt_v)),
            (0x000A, "v2b", Box::new(fmt_v)),
            (0x000B, "v2c", Box::new(fmt_v)),
            (0x000C, "swVer", Box::new(fmt_ver)),
            (0x0015, "cnt1", Box::new(fmt_cnt)),
            (0x0016, "cnt2", Box::new(fmt_cnt)),
            (0x0017, "runtime", Box::new(fmt_h)),
            (0x2069, "t1", Box::new(fmt_s)),
            (0x206A, "t2", Box::new(fmt_s)),
        ];

        for (reg, key, f) in regs {
            if let Some(v) = read_register(&guard, addr, reg, debug) {
                values.insert(key.to_string(), f(v));
            } else {
                values.insert(key.to_string(), "---".to_string());
            }
        }

        let max_regs = vec![(0x000F,"max1a"),(0x0010,"max1b"),(0x0011,"max1c"),
                            (0x0012,"max2a"),(0x0013,"max2b"),(0x0014,"max2c")];
        for (r, k) in max_regs {
            if let Some(v) = read_register(&guard, addr, r, debug) {
                values.insert(k.to_string(), format!("{} V", v));
            }
        }
        
        if let (Some(a), Some(b), Some(c)) = (values.get("max1a"), values.get("max1b"), values.get("max1c")) {
            values.insert("max1".to_string(), format!("{}/{}/{}", a, b, c));
        }
        if let (Some(a), Some(b), Some(c)) = (values.get("max2a"), values.get("max2b"), values.get("max2c")) {
            values.insert("max2".to_string(), format!("{}/{}/{}", a, b, c));
        }

        if model == "MN" {
            let extra = vec![(0x2065,"uv1"),(0x2066,"uv2"),(0x2067,"ov1"),(0x2068,"ov2"),
                            (0x206B,"t3"),(0x206C,"t4")];
            for (reg, key) in extra {
                if let Some(v) = read_register(&guard, addr, reg, debug) {
                    let s = if key.starts_with('t') { format!("{} s", v) } else { format!("{} V", v) };
                    values.insert(key.to_string(), s);
                } else {
                    values.insert(key.to_string(), "---".to_string());
                }
            }
        } else {
            for k in ["uv1","uv2","ov1","ov2","t3","t4"] {
                values.insert(k.to_string(), "N/A".to_string());
            }
        }

        if model == "MN" {
            if let Some(freq) = read_register(&guard, addr, 0x000D, debug) {
                values.insert("freq1".to_string(), format!("{} Hz", (freq >> 8) & 0xFF));
                values.insert("freq2".to_string(), format!("{} Hz", freq & 0xFF));
            }
        } else {
            values.insert("freq1".to_string(), "N/A".to_string());
            values.insert("freq2".to_string(), "N/A".to_string());
        }

        if let Some(power) = read_register(&guard, addr, 0x004F, debug) {
            let decode = |bit: u8| -> String {
                match (power >> bit) & 0x03 {
                    0 => "✅ Normal".to_string(),
                    1 => "⚠️ Sous-tension".to_string(),
                    2 => "⚠️ Surtension".to_string(),
                    _ => "❌ Erreur".to_string(),
                }
            };
            values.insert("s1a".to_string(), decode(8));
            values.insert("s1b".to_string(), decode(10));
            values.insert("s1c".to_string(), decode(12));
            values.insert("s2a".to_string(), decode(0));
            values.insert("s2b".to_string(), decode(2));
            values.insert("s2c".to_string(), decode(4));
        }

        if let Some(switch) = read_register(&guard, addr, 0x0050, debug) {
            values.insert("sw1".to_string(), if switch & 0x02 != 0 { "✅ Fermé" } else { "⭕ Ouvert" }.to_string());
            values.insert("sw2".to_string(), if switch & 0x04 != 0 { "✅ Fermé" } else { "⭕ Ouvert" }.to_string());
            values.insert("swMode".to_string(), if switch & 0x01 != 0 { "🤖 Auto" } else { "👆 Manuel" }.to_string());
            values.insert("swRemote".to_string(), if switch & 0x0100 != 0 { "📡 Activé" } else { "🔒 Désactivé" }.to_string());
            let fault = (switch >> 4) & 0x07;
            values.insert("swFault".to_string(), match fault {
                0 => "Aucun", 1 => "Interconnexion incendie", 2 => "Surcharge du moteur",
                3 => "Disjonction I Onduleur", 4 => "Disjonction II Réseau",
                5 => "Signal de fermeture anormal", 6 => "Phase anormal I",
                7 => "Phase anormal II", _ => "Inconnu"
            }.to_string());
        }

        if let Some(mode) = read_register(&guard, addr, 0x206D, debug) {
            values.insert("operation_mode".to_string(), match mode {
                0 => "Auto-réarmement automatique", 1 => "Auto-non-réarmement",
                2 => "Secours", 3 => "Mode générateur", 4 => "Générateur non réarmé",
                5 => "Générateur de secours", _ => "Inconnu"
            }.to_string());
        }

        if let Some(addr_val) = read_register(&guard, addr, 0x0100, debug) {
            values.insert("modbus_addr".to_string(), addr_val.to_string());
        }
        if let Some(baud) = read_register(&guard, addr, 0x0101, debug) {
            values.insert("modbus_baud".to_string(), match baud {
                0 => "4800", 1 => "9600", 2 => "19200", 3 => "38400", _ => "?",
            }.to_string());
        }
        if let Some(parity) = read_register(&guard, addr, 0x000E, debug) {
            values.insert("modbus_parity".to_string(), match parity {
                0 => "None", 1 => "Odd", 2 => "Even", _ => "?",
            }.to_string());
        }

        let success = values.values().any(|v| v != "---" && v != "N/A");
        (success, values, model)
    }).await.expect("spawn_blocking failed");

    let (success, values, model) = result;
    HttpResponse::Ok().json(ModbusResponse {
        success,
        values,
        model,
        error: if success { None } else { Some("Aucune réponse du matériel".to_string()) },
    })
}

// Macro commandes - POST uniquement
macro_rules! make_cmd {
    ($name:ident, $reg:expr, $val:expr, $msg:literal) => {
        async fn $name(data: web::Data<Arc<Mutex<AppState>>>) -> impl Responder {
            let state = data.clone();
            let success = actix_web::rt::task::spawn_blocking(move || {
                let guard = state.lock().unwrap();
                let debug = *guard.debug_log.lock().unwrap();
                write_register(&guard, guard.config.modbus_addr, $reg, $val, debug)
            }).await.unwrap();
            HttpResponse::Ok().json(serde_json::json!({ "success": success, "message": $msg }))
        }
    };
}

make_cmd!(remote_on, 0x2800, 0x0004, "Télécommande activée");
make_cmd!(remote_off, 0x2800, 0x0000, "Télécommande désactivée");
make_cmd!(force_double, 0x2700, 0x00FF, "Forçage double déclenché");
make_cmd!(force_source1, 0x2700, 0x0000, "Forçage Onduleur");
make_cmd!(force_source2, 0x2700, 0x00AA, "Forçage Réseau");

async fn set_setting(data: web::Data<Arc<Mutex<AppState>>>, query: web::Query<RegValue>, reg: u16, name: &str) -> impl Responder {
    let state = data.clone();
    let result = actix_web::rt::task::spawn_blocking(move || {
        let guard = state.lock().unwrap();
        let model = guard.model_type.lock().unwrap().clone();
        if model != "MN" {
            return (false, format!("Lecture seule sur modèle {}", model));
        }
        let debug = *guard.debug_log.lock().unwrap();
        let success = write_register(&guard, guard.config.modbus_addr, reg, query.value, debug);
        (success, if success { "OK".to_string() } else { "Échec écriture".to_string() })
    }).await.unwrap();

    HttpResponse::Ok().json(serde_json::json!({ 
        "success": result.0, 
        "message": result.1,
        "setting": name
    }))
}

async fn set_undervoltage1(data: web::Data<Arc<Mutex<AppState>>>, query: web::Query<RegValue>) -> impl Responder {
    set_setting(data, query, 0x2065, "undervoltage1").await
}
async fn set_undervoltage2(data: web::Data<Arc<Mutex<AppState>>>, query: web::Query<RegValue>) -> impl Responder {
    set_setting(data, query, 0x2066, "undervoltage2").await
}
async fn set_overvoltage1(data: web::Data<Arc<Mutex<AppState>>>, query: web::Query<RegValue>) -> impl Responder {
    set_setting(data, query, 0x2067, "overvoltage1").await
}
async fn set_overvoltage2(data: web::Data<Arc<Mutex<AppState>>>, query: web::Query<RegValue>) -> impl Responder {
    set_setting(data, query, 0x2068, "overvoltage2").await
}

async fn send_raw(data: web::Data<Arc<Mutex<AppState>>>, body: web::Json<RawFrame>) -> impl Responder {
    let state = data.clone();
    let frame = body.frame.clone();
    let result = actix_web::rt::task::spawn_blocking(move || {
        let guard = state.lock().unwrap();
        let debug = *guard.debug_log.lock().unwrap();
        match send_raw_frame(&guard, &frame, debug) {
            Ok((s, r)) => {
                let sh = s.iter().map(|b| format!("{:02X}", b)).collect::<Vec<_>>().join(" ");
                let rh = r.iter().map(|b| format!("{:02X}", b)).collect::<Vec<_>>().join(" ");
                write_command_log(&sh, &rh);
                (true, rh, r.len())
            }
            Err(e) => (false, e, 0usize),
        }
    }).await.unwrap();

    if result.0 {
        HttpResponse::Ok().json(serde_json::json!({ "success": true, "response_hex": result.1, "response_length": result.2 }))
    } else {
        HttpResponse::Ok().json(serde_json::json!({ "success": false, "error": result.1 }))
    }
}

async fn debug_on(data: web::Data<Arc<Mutex<AppState>>>) -> impl Responder {
    let state = data.lock().unwrap();
    *state.debug_log.lock().unwrap() = true;
    write_debug_log("=== DEBUG ACTIVÉ ===", true);
    HttpResponse::Ok().json(serde_json::json!({ "success": true, "message": "Debug activé" }))
}

async fn debug_off(data: web::Data<Arc<Mutex<AppState>>>) -> impl Responder {
    let state = data.lock().unwrap();
    *state.debug_log.lock().unwrap() = false;
    HttpResponse::Ok().json(serde_json::json!({ "success": true, "message": "Debug désactivé" }))
}

// ==================== MAIN ====================
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));

    let config = Config::from_env();
    
    println!("========================================");
    println!("  CHINT ATS - Serveur Rust v3.1");
    println!("  Port: {} | {} | Adresse {}", config.port_name, config.baud_rate, config.modbus_addr);
    println!("========================================");

    let initial_port = open_port(&config);
    let temp_state = AppState {
        config: config.clone(),
        port: Mutex::new(initial_port),
        debug_log: Mutex::new(config.debug_enabled),
        model_type: Mutex::new("?".to_string()),
        last_success: Mutex::new(Instant::now()),
        last_error: Mutex::new(None),
    };

    let model = detect_model(&temp_state, config.modbus_addr, config.debug_enabled);
    info!("✅ Modèle détecté : {}", model);
    *temp_state.model_type.lock().unwrap() = model.clone();

    let app_state = web::Data::new(Arc::new(Mutex::new(temp_state)));
    
    start_monitoring(app_state.clone()).await;

    info!("🌐 Serveur démarré → http://{}:{}", config.host, config.port_http);

    HttpServer::new(move || {
        App::new()
            .wrap(Cors::default()
                .allow_any_origin()
                .allow_any_method()
                .allow_any_header()
                .max_age(3600))
            .wrap(middleware::Logger::default())
            .app_data(app_state.clone())
            .route("/", web::get().to(index))
            .route("/api/health", web::get().to(health))
            .route("/api/read_all", web::get().to(read_all))
            // Commandes - POST
            .route("/api/remote_on", web::post().to(remote_on))
            .route("/api/remote_off", web::post().to(remote_off))
            .route("/api/force_double", web::post().to(force_double))
            .route("/api/force_source1", web::post().to(force_source1))
            .route("/api/force_source2", web::post().to(force_source2))
            .route("/api/send_raw", web::post().to(send_raw))
            .route("/api/debug_on", web::get().to(debug_on))
            .route("/api/debug_off", web::get().to(debug_off))
            // Réglages - POST (MN uniquement)
            .route("/api/set_undervoltage1", web::post().to(set_undervoltage1))
            .route("/api/set_undervoltage2", web::post().to(set_undervoltage2))
            .route("/api/set_overvoltage1", web::post().to(set_overvoltage1))
            .route("/api/set_overvoltage2", web::post().to(set_overvoltage2))
    })
    .bind((config.host.as_str(), config.port_http))?
    .run()
    .await
}
