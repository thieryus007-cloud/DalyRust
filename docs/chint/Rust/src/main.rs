use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use actix_files::NamedFile;
use serde::{Serialize, Deserialize};
use std::sync::Mutex;
use std::time::Duration;
use serialport::{self};
use std::io::{Write, Read};
use std::thread;
use chrono::Local;

// ==================== STRUCTURES ====================

struct AppState {
    port_name: Mutex<String>,
    debug_log: Mutex<bool>,
    model_type: Mutex<String>,  // "MN", "BN", "unknown"
}

#[derive(Serialize)]
struct ModbusResponse {
    success: bool,
    values: std::collections::HashMap<String, String>,
    model: String,
    error: Option<String>,
}

#[derive(Deserialize)]
struct RegValue {
    value: u16,
}

// ==================== FONCTIONS DE LOG ====================

fn write_debug_log(message: &str, debug: bool) {
    if !debug { return; }
    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
    let log_line = format!("[{}] {}\n", timestamp, message);
    let _ = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("modbus_debug.log")
        .and_then(|mut file| file.write_all(log_line.as_bytes()));
}

// ==================== FONCTIONS MODBUS ====================

fn calculate_crc(data: &[u8]) -> u16 {
    let mut crc = 0xFFFF;
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
        if let Some(val) = value {
            data.extend_from_slice(&[(val >> 8) as u8, val as u8]);
        }
    }
    
    let crc = calculate_crc(&data);
    data.push((crc & 0xFF) as u8);
    data.push((crc >> 8) as u8);
    data
}

fn read_register(port_name: &str, addr: u8, reg: u16, debug: bool) -> Option<u16> {
    let frame = build_frame(addr, 0x03, reg, None);
    
    if debug {
        write_debug_log(&format!("📤 READ REG 0x{:04X} | Trame: {}", reg, frame.iter().map(|b| format!("{:02X}", b)).collect::<Vec<_>>().join(" ")), debug);
    }
    
    let mut port = match serialport::new(port_name, 9600)
        .data_bits(serialport::DataBits::Eight)
        .parity(serialport::Parity::Even)
        .stop_bits(serialport::StopBits::One)
        .timeout(Duration::from_millis(500))
        .open()
    {
        Ok(p) => p,
        Err(e) => {
            if debug { write_debug_log(&format!("❌ Erreur ouverture port: {}", e), debug); }
            return None;
        }
    };
    
    if port.write_all(&frame).is_err() {
        if debug { write_debug_log("❌ Erreur écriture port", debug); }
        return None;
    }
    
    thread::sleep(Duration::from_millis(100));
    
    let mut buffer = vec![0u8; 256];
    match port.read(&mut buffer) {
        Ok(n) if n >= 5 => {
            let resp = &buffer[..n];
            if debug {
                write_debug_log(&format!("📥 READ REG 0x{:04X} | Réponse: {}", reg, resp.iter().map(|b| format!("{:02X}", b)).collect::<Vec<_>>().join(" ")), debug);
            }
            // Vérifier si c'est une erreur Modbus (fonction 0x83)
            if resp[1] == 0x83 {
                if debug { write_debug_log(&format!("⚠️ Erreur Modbus: code {}", resp[2]), debug); }
                return None;
            }
            if resp.len() >= 5 && resp[1] == 0x03 {
                let value = ((resp[3] as u16) << 8) | resp[4] as u16;
                if debug {
                    write_debug_log(&format!("📊 Valeur: {} (0x{:04X})", value, value), debug);
                }
                Some(value)
            } else {
                if debug { write_debug_log(&format!("⚠️ Réponse invalide (fonction: 0x{:02X})", resp[1]), debug); }
                None
            }
        }
        _ => None,
    }
}

fn write_register(port_name: &str, addr: u8, reg: u16, value: u16, debug: bool) -> bool {
    let frame = build_frame(addr, 0x06, reg, Some(value));
    
    if debug {
        write_debug_log(&format!("📝 WRITE REG 0x{:04X} = {} | Trame: {}", reg, value, frame.iter().map(|b| format!("{:02X}", b)).collect::<Vec<_>>().join(" ")), debug);
    }
    
    let mut port = match serialport::new(port_name, 9600)
        .data_bits(serialport::DataBits::Eight)
        .parity(serialport::Parity::Even)
        .stop_bits(serialport::StopBits::One)
        .timeout(Duration::from_millis(500))
        .open()
    {
        Ok(p) => p,
        Err(e) => {
            if debug { write_debug_log(&format!("❌ Erreur ouverture port: {}", e), debug); }
            return false;
        }
    };
    
    if port.write_all(&frame).is_err() {
        if debug { write_debug_log("❌ Erreur écriture port", debug); }
        return false;
    }
    
    thread::sleep(Duration::from_millis(100));
    
    let mut buffer = vec![0u8; 256];
    match port.read(&mut buffer) {
        Ok(n) if n > 0 => {
            let resp = &buffer[..n];
            if debug {
                write_debug_log(&format!("📥 WRITE REG 0x{:04X} | Réponse: {}", reg, resp.iter().map(|b| format!("{:02X}", b)).collect::<Vec<_>>().join(" ")), debug);
            }
            // Vérifier si c'est une erreur Modbus (fonction 0x86)
            if resp[1] == 0x86 {
                if debug { write_debug_log(&format!("⚠️ Erreur écriture Modbus: code {}", resp[2]), debug); }
                return false;
            }
            true
        }
        _ => false,
    }
}

// Détection du modèle
fn detect_model(port_name: &str, addr: u8, debug: bool) -> String {
    // Test registre MN (0x2065) - si existe, c'est un modèle MN
    if let Some(_) = read_register(port_name, addr, 0x2065, false) {
        return "MN".to_string();
    }
    "BN".to_string()  // Modèle BN (base)
}

// ==================== API ROUTES ====================

async fn read_all(data: web::Data<Mutex<AppState>>) -> impl Responder {
    let state = data.lock().unwrap();
    let port_name = state.port_name.lock().unwrap();
    let debug = *state.debug_log.lock().unwrap();
    let model = state.model_type.lock().unwrap().clone();
    
    let mut values = std::collections::HashMap::new();
    let addr = 6u8;
    
    let fmt_v = |x: u16| format!("{} V", x);
    let fmt_ver = |x: u16| format!("{:.2}", x as f32 / 100.0);
    let fmt_cnt = |x: u16| x.to_string();
    let fmt_h = |x: u16| format!("{} h", x);
    let fmt_s = |x: u16| format!("{} s", x);
    
    // Registres de base (tous modèles)
    let regs: Vec<(u16, &str, Box<dyn Fn(u16) -> String>)> = vec![
        (0x0006, "v1a", Box::new(fmt_v)),
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
    
    for (reg, key, formatter) in regs {
        if let Some(val) = read_register(&port_name, addr, reg, debug) {
            values.insert(key.to_string(), formatter(val));
        } else {
            values.insert(key.to_string(), "---".to_string());
        }
    }
    
    // T3/T4 seulement pour MN
    if model == "MN" {
        if let Some(t3) = read_register(&port_name, addr, 0x206B, debug) {
            values.insert("t3".to_string(), format!("{} s", t3));
        } else {
            values.insert("t3".to_string(), "---".to_string());
        }
        if let Some(t4) = read_register(&port_name, addr, 0x206C, debug) {
            values.insert("t4".to_string(), format!("{} s", t4));
        } else {
            values.insert("t4".to_string(), "---".to_string());
        }
    } else {
        values.insert("t3".to_string(), "N/A".to_string());
        values.insert("t4".to_string(), "N/A".to_string());
    }
    
    // Seuils seulement pour MN
    if model == "MN" {
        if let Some(uv1) = read_register(&port_name, addr, 0x2065, debug) {
            values.insert("uv1".to_string(), format!("{} V", uv1));
        } else {
            values.insert("uv1".to_string(), "---".to_string());
        }
        if let Some(uv2) = read_register(&port_name, addr, 0x2066, debug) {
            values.insert("uv2".to_string(), format!("{} V", uv2));
        } else {
            values.insert("uv2".to_string(), "---".to_string());
        }
        if let Some(ov1) = read_register(&port_name, addr, 0x2067, debug) {
            values.insert("ov1".to_string(), format!("{} V", ov1));
        } else {
            values.insert("ov1".to_string(), "---".to_string());
        }
        if let Some(ov2) = read_register(&port_name, addr, 0x2068, debug) {
            values.insert("ov2".to_string(), format!("{} V", ov2));
        } else {
            values.insert("ov2".to_string(), "---".to_string());
        }
    } else {
        values.insert("uv1".to_string(), "N/A".to_string());
        values.insert("uv2".to_string(), "N/A".to_string());
        values.insert("ov1".to_string(), "N/A".to_string());
        values.insert("ov2".to_string(), "N/A".to_string());
    }
    
    // Fréquence
    if let Some(freq) = read_register(&port_name, addr, 0x000D, debug) {
        values.insert("freq1".to_string(), format!("{} Hz", (freq >> 8) & 0xFF));
        values.insert("freq2".to_string(), format!("{} Hz", freq & 0xFF));
    }
    
    // État des sources
    if let Some(power) = read_register(&port_name, addr, 0x004F, debug) {
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
    
    // État commutateur
    if let Some(switch) = read_register(&port_name, addr, 0x0050, debug) {
        values.insert("sw1".to_string(), if switch & 0x02 != 0 { "✅ Fermé" } else { "⭕ Ouvert" }.to_string());
        values.insert("sw2".to_string(), if switch & 0x04 != 0 { "✅ Fermé" } else { "⭕ Ouvert" }.to_string());
        values.insert("swMode".to_string(), if switch & 0x01 != 0 { "🤖 Auto" } else { "👆 Manuel" }.to_string());
        values.insert("swRemote".to_string(), if switch & 0x0100 != 0 { "📡 Activé" } else { "🔒 Désactivé" }.to_string());
        let fault = (switch >> 4) & 0x07;
        values.insert("swFault".to_string(), match fault {
            0 => "Aucun", 1 => "消防联动", 2 => "电机超时", 3 => "电源I跳闸",
            4 => "电源II跳闸", 5 => "合闸信号异常", 6 => "相序异常 I", 7 => "相序异常 II",
            _ => "Inconnu",
        }.to_string());
    }
    
    // Mode fonctionnement
    if let Some(mode) = read_register(&port_name, addr, 0x206D, debug) {
        values.insert("operation_mode".to_string(), match mode {
            0 => "自投自复", 1 => "自投不自复", 2 => "互为备用",
            3 => "发电机模式", 4 => "发电机不自复", 5 => "发电机备用",
            _ => "Inconnu",
        }.to_string());
    }
    
    // Tensions maximales (pour BN aussi)
    let max_regs = vec![
        (0x000F, "max1a"), (0x0010, "max1b"), (0x0011, "max1c"),
        (0x0012, "max2a"), (0x0013, "max2b"), (0x0014, "max2c"),
    ];
    for (reg, key) in max_regs {
        if let Some(val) = read_register(&port_name, addr, reg, debug) {
            values.insert(key.to_string(), format!("{} V", val));
        }
    }
    if let (Some(a), Some(b), Some(c)) = (
        values.get("max1a"), values.get("max1b"), values.get("max1c")
    ) {
        values.insert("max1".to_string(), format!("{}/{}/{}", a, b, c));
    }
    if let (Some(a), Some(b), Some(c)) = (
        values.get("max2a"), values.get("max2b"), values.get("max2c")
    ) {
        values.insert("max2".to_string(), format!("{}/{}/{}", a, b, c));
    }
    
    // Configuration Modbus
    if let Some(addr_val) = read_register(&port_name, addr, 0x0100, debug) {
        values.insert("modbus_addr".to_string(), addr_val.to_string());
    }
    if let Some(baud) = read_register(&port_name, addr, 0x0101, debug) {
        values.insert("modbus_baud".to_string(), match baud {
            0 => "4800", 1 => "9600", 2 => "19200", 3 => "38400", _ => "?",
        }.to_string());
    }
    if let Some(parity) = read_register(&port_name, addr, 0x000E, debug) {
        values.insert("modbus_parity".to_string(), match parity {
            0 => "None", 1 => "Odd", 2 => "Even", _ => "?",
        }.to_string());
    }
    
    let success = values.values().any(|v| v != "---" && v != "N/A");
    HttpResponse::Ok().json(ModbusResponse {
        success,
        values,
        model: model.clone(),
        error: if success { None } else { Some("Aucune réponse".to_string()) },
    })
}

// ==================== ROUTES DE COMMANDES ====================

async fn remote_on(data: web::Data<Mutex<AppState>>) -> impl Responder {
    let state = data.lock().unwrap();
    let port_name = state.port_name.lock().unwrap();
    let debug = *state.debug_log.lock().unwrap();
    let success = write_register(&port_name, 6, 0x2800, 0x0004, debug);
    HttpResponse::Ok().json(serde_json::json!({
        "success": success,
        "message": "Télécommande activée"
    }))
}

async fn remote_off(data: web::Data<Mutex<AppState>>) -> impl Responder {
    let state = data.lock().unwrap();
    let port_name = state.port_name.lock().unwrap();
    let debug = *state.debug_log.lock().unwrap();
    let success = write_register(&port_name, 6, 0x2800, 0x0000, debug);
    HttpResponse::Ok().json(serde_json::json!({
        "success": success,
        "message": "Télécommande désactivée"
    }))
}

async fn force_double(data: web::Data<Mutex<AppState>>) -> impl Responder {
    let state = data.lock().unwrap();
    let port_name = state.port_name.lock().unwrap();
    let debug = *state.debug_log.lock().unwrap();
    let success = write_register(&port_name, 6, 0x2700, 0x00FF, debug);
    HttpResponse::Ok().json(serde_json::json!({
        "success": success,
        "message": "Forçage double déclenché"
    }))
}

async fn force_source1(data: web::Data<Mutex<AppState>>) -> impl Responder {
    let state = data.lock().unwrap();
    let port_name = state.port_name.lock().unwrap();
    let debug = *state.debug_log.lock().unwrap();
    let success = write_register(&port_name, 6, 0x2700, 0x0000, debug);
    HttpResponse::Ok().json(serde_json::json!({
        "success": success,
        "message": "Forçage Source I"
    }))
}

async fn force_source2(data: web::Data<Mutex<AppState>>) -> impl Responder {
    let state = data.lock().unwrap();
    let port_name = state.port_name.lock().unwrap();
    let debug = *state.debug_log.lock().unwrap();
    let success = write_register(&port_name, 6, 0x2700, 0x00AA, debug);
    HttpResponse::Ok().json(serde_json::json!({
        "success": success,
        "message": "Forçage Source II"
    }))
}

// ==================== ROUTES DE RÉGLAGE (seulement si modèle MN) ====================

async fn set_undervoltage1(data: web::Data<Mutex<AppState>>, query: web::Query<RegValue>) -> impl Responder {
    let state = data.lock().unwrap();
    let port_name = state.port_name.lock().unwrap();
    let debug = *state.debug_log.lock().unwrap();
    let model = state.model_type.lock().unwrap().clone();
    let value = query.value;
    
    if model != "MN" {
        return HttpResponse::Ok().json(serde_json::json!({
            "success": false,
            "error": "Cette fonction n'est pas disponible sur ce modèle (BN)"
        }));
    }
    
    let remote_status = read_register(&port_name, 6, 0x0050, debug).map(|s| (s & 0x0100) != 0).unwrap_or(false);
    if !remote_status {
        return HttpResponse::Ok().json(serde_json::json!({
            "success": false,
            "error": "Activez d'abord la télécommande"
        }));
    }
    
    if value < 150 || value > 200 {
        return HttpResponse::Ok().json(serde_json::json!({
            "success": false,
            "error": "La valeur doit être entre 150 et 200 V"
        }));
    }
    
    let success = write_register(&port_name, 6, 0x2065, value, debug);
    HttpResponse::Ok().json(serde_json::json!({
        "success": success,
        "message": format!("Sous-tension Source I réglée à {} V", value)
    }))
}

// ... (autres fonctions de réglage similaires avec vérification model)

async fn index() -> impl Responder {
    NamedFile::open_async("index.html").await.unwrap()
}

// ==================== MAIN ====================

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Détection du modèle au démarrage
    let port_name = "COM5";
    let addr = 6;
    let debug = false;
    
    println!("========================================");
    println!("  CHINT ATS - Serveur Rust v2");
    println!("  Port: COM5 | 9600 Even | Adresse 6");
    println!("  Détection du modèle en cours...");
    
    // Test registre MN (0x2065)
    let model = if let Some(_) = read_register(port_name, addr, 0x2065, false) {
        println!("  ✅ Modèle détecté: MN (série complète avec réglages)");
        "MN".to_string()
    } else {
        println!("  ✅ Modèle détecté: BN (série de base, réglages non disponibles)");
        "BN".to_string()
    };
    
    println!("  Ouvrez http://localhost:5000");
    println!("  Actualisation automatique toutes les 5s");
    println!("========================================");
    
    let app_state = web::Data::new(Mutex::new(AppState {
        port_name: Mutex::new(port_name.to_string()),
        debug_log: Mutex::new(false),
        model_type: Mutex::new(model),
    }));
    
    HttpServer::new(move || {
        let mut app = App::new()
            .app_data(app_state.clone())
            .route("/", web::get().to(index))
            .route("/api/read_all", web::get().to(read_all))
            .route("/api/remote_on", web::get().to(remote_on))
            .route("/api/remote_off", web::get().to(remote_off))
            .route("/api/force_double", web::get().to(force_double))
            .route("/api/force_source1", web::get().to(force_source1))
            .route("/api/force_source2", web::get().to(force_source2));
        
        // N'ajouter les routes de réglage que si le modèle le supporte
        let model = app_state.lock().unwrap().model_type.lock().unwrap().clone();
        if model == "MN" {
            app = app
                .route("/api/set_undervoltage1", web::get().to(set_undervoltage1))
                .route("/api/set_undervoltage2", web::get().to(set_undervoltage2))
                .route("/api/set_overvoltage1", web::get().to(set_overvoltage1))
                .route("/api/set_overvoltage2", web::get().to(set_overvoltage2));
        }
        
        app
    })
    .bind("localhost:5000")?
    .run()
    .await
}
