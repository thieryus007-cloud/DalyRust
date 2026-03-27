use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use actix_files::NamedFile;
use serde::Serialize;
use std::sync::Mutex;
use std::time::Duration;
use serialport::{self, SerialPort};
use std::io::{Write, Read};
use std::thread;

// ==================== STRUCTURES ====================

struct AppState {
    port_name: Mutex<String>,
}

#[derive(Serialize)]
struct ModbusResponse {
    success: bool,
    values: std::collections::HashMap<String, String>,
    error: Option<String>,
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

fn read_register(port_name: &str, addr: u8, reg: u16) -> Option<u16> {
    let frame = build_frame(addr, 0x03, reg, None);
    
    let mut port = match serialport::new(port_name, 9600)
        .data_bits(serialport::DataBits::Eight)
        .parity(serialport::Parity::Even)
        .stop_bits(serialport::StopBits::One)
        .timeout(Duration::from_millis(500))
        .open()
    {
        Ok(p) => p,
        Err(_) => return None,
    };
    
    if port.write_all(&frame).is_err() {
        return None;
    }
    
    thread::sleep(Duration::from_millis(100));
    
    let mut buffer = vec![0u8; 256];
    match port.read(&mut buffer) {
        Ok(n) if n >= 5 => {
            let resp = &buffer[..n];
            if resp.len() >= 5 && resp[1] == 0x03 {
                Some(((resp[3] as u16) << 8) | resp[4] as u16)
            } else {
                None
            }
        }
        _ => None,
    }
}

fn write_register(port_name: &str, addr: u8, reg: u16, value: u16) -> bool {
    let frame = build_frame(addr, 0x06, reg, Some(value));
    
    let mut port = match serialport::new(port_name, 9600)
        .data_bits(serialport::DataBits::Eight)
        .parity(serialport::Parity::Even)
        .stop_bits(serialport::StopBits::One)
        .timeout(Duration::from_millis(500))
        .open()
    {
        Ok(p) => p,
        Err(_) => return false,
    };
    
    if port.write_all(&frame).is_err() {
        return false;
    }
    
    thread::sleep(Duration::from_millis(100));
    
    let mut buffer = vec![0u8; 256];
    match port.read(&mut buffer) {
        Ok(n) if n > 0 => true,
        _ => false,
    }
}

// ==================== API ROUTES ====================

async fn read_all(data: web::Data<Mutex<AppState>>) -> impl Responder {
    let state = data.lock().unwrap();
    let port_name = state.port_name.lock().unwrap();
    
    let mut values = std::collections::HashMap::new();
    let addr = 6u8;
    
    // Formateurs
    let fmt_v = |x: u16| format!("{} V", x);
    let fmt_ver = |x: u16| format!("{:.2}", x as f32 / 100.0);
    let fmt_cnt = |x: u16| x.to_string();
    let fmt_h = |x: u16| format!("{} h", x);
    let fmt_s = |x: u16| format!("{} s", x);
    
    // ===== LECTURES DE BASE =====
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
    ];
    
    for (reg, key, formatter) in regs {
        if let Some(val) = read_register(&port_name, addr, reg) {
            values.insert(key.to_string(), formatter(val));
        } else {
            values.insert(key.to_string(), "---".to_string());
        }
    }
    
    // ===== TENSIONS MAXIMALES =====
    let max_regs: Vec<(u16, &str)> = vec![
        (0x000F, "max1a"), (0x0010, "max1b"), (0x0011, "max1c"),
        (0x0012, "max2a"), (0x0013, "max2b"), (0x0014, "max2c"),
    ];
    
    for (reg, key) in max_regs {
        if let Some(val) = read_register(&port_name, addr, reg) {
            values.insert(key.to_string(), format!("{} V", val));
        } else {
            values.insert(key.to_string(), "---".to_string());
        }
    }
    
    // ===== FRÉQUENCE (0x000D) =====
    if let Some(freq) = read_register(&port_name, addr, 0x000D) {
        let f1 = (freq >> 8) & 0xFF;
        let f2 = freq & 0xFF;
        values.insert("freq1".to_string(), format!("{} Hz", f1));
        values.insert("freq2".to_string(), format!("{} Hz", f2));
    } else {
        values.insert("freq1".to_string(), "---".to_string());
        values.insert("freq2".to_string(), "---".to_string());
    }
    
    // ===== ÉTAT DES SOURCES (0x004F) =====
    if let Some(power) = read_register(&port_name, addr, 0x004F) {
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
    } else {
        for k in ["s1a", "s1b", "s1c", "s2a", "s2b", "s2c"] {
            values.insert(k.to_string(), "---".to_string());
        }
    }
    
    // ===== ÉTAT DU COMMUTATEUR (0x0050) =====
    if let Some(switch) = read_register(&port_name, addr, 0x0050) {
        values.insert("sw1".to_string(), if switch & 0x02 != 0 { "✅ Fermé" } else { "⭕ Ouvert" }.to_string());
        values.insert("sw2".to_string(), if switch & 0x04 != 0 { "✅ Fermé" } else { "⭕ Ouvert" }.to_string());
        values.insert("swMid".to_string(), if switch & 0x08 != 0 { "⚠️ Oui" } else { "⭕ Non" }.to_string());
        values.insert("swMode".to_string(), if switch & 0x01 != 0 { "🤖 Automatique" } else { "👆 Manuel" }.to_string());
        values.insert("swRemote".to_string(), if switch & 0x0100 != 0 { "📡 Activé" } else { "🔒 Désactivé" }.to_string());
        values.insert("swGen".to_string(), if switch & 0x10 != 0 { "🟢 Marche" } else { "🔴 Arrêt" }.to_string());
        
        // Type de défaut
        let fault = (switch >> 4) & 0x07;
        let fault_str = match fault {
            0 => "Aucun",
            1 => "消防联动 (Fire)",
            2 => "电机超时 (Motor timeout)",
            3 => "电源I跳闸 (Trip I)",
            4 => "电源II跳闸 (Trip II)",
            5 => "合闸信号异常 (Close signal)",
            6 => "电源I相序异常 (Phase order I)",
            7 => "电源II相序异常 (Phase order II)",
            _ => "Inconnu",
        };
        values.insert("swFault".to_string(), fault_str.to_string());
    } else {
        for k in ["sw1", "sw2", "swMid", "swMode", "swRemote", "swGen", "swFault"] {
            values.insert(k.to_string(), "---".to_string());
        }
    }
    
    // ===== PARAMÈTRES MODBUS =====
    if let Some(addr_val) = read_register(&port_name, addr, 0x0100) {
        values.insert("modbus_addr".to_string(), addr_val.to_string());
    } else {
        values.insert("modbus_addr".to_string(), "---".to_string());
    }
    
    if let Some(baud) = read_register(&port_name, addr, 0x0101) {
        let baud_str = match baud {
            0 => "4800",
            1 => "9600",
            2 => "19200",
            3 => "38400",
            _ => "Inconnu",
        };
        values.insert("modbus_baud".to_string(), baud_str.to_string());
    } else {
        values.insert("modbus_baud".to_string(), "---".to_string());
    }
    
    if let Some(parity) = read_register(&port_name, addr, 0x000E) {
        let parity_str = match parity {
            0 => "None",
            1 => "Odd",
            2 => "Even",
            _ => "Inconnu",
        };
        values.insert("modbus_parity".to_string(), parity_str.to_string());
    } else {
        values.insert("modbus_parity".to_string(), "---".to_string());
    }
    
    // ===== PARAMÈTRES DE RÉGLAGE (NXZ(H)MN uniquement) =====
    // Seuils sous-tension (150-200V)
    if let Some(uv1) = read_register(&port_name, addr, 0x2065) {
        values.insert("uv1".to_string(), format!("{} V", uv1));
    }
    if let Some(uv2) = read_register(&port_name, addr, 0x2066) {
        values.insert("uv2".to_string(), format!("{} V", uv2));
    }
    
    // Seuils surtension (240-290V)
    if let Some(ov1) = read_register(&port_name, addr, 0x2067) {
        values.insert("ov1".to_string(), format!("{} V", ov1));
    }
    if let Some(ov2) = read_register(&port_name, addr, 0x2068) {
        values.insert("ov2".to_string(), format!("{} V", ov2));
    }
    
    // Temporisations
    if let Some(t1) = read_register(&port_name, addr, 0x2069) {
        values.insert("t1".to_string(), format!("{} s", t1));
    }
    if let Some(t2) = read_register(&port_name, addr, 0x206A) {
        values.insert("t2".to_string(), format!("{} s", t2));
    }
    if let Some(t3) = read_register(&port_name, addr, 0x206B) {
        values.insert("t3".to_string(), format!("{} s", t3));
    }
    if let Some(t4) = read_register(&port_name, addr, 0x206C) {
        values.insert("t4".to_string(), format!("{} s", t4));
    }
    
    // Mode de fonctionnement
    if let Some(mode) = read_register(&port_name, addr, 0x206D) {
        let mode_str = match mode {
            0 => "自投自复 (Auto-recovery)",
            1 => "自投不自复 (Auto no-recovery)",
            2 => "互为备用 (Mutual backup)",
            3 => "发电机模式 (Generator)",
            4 => "发电机不自复 (Gen no-recovery)",
            5 => "发电机互为备用 (Gen backup)",
            _ => "Inconnu",
        };
        values.insert("operation_mode".to_string(), mode_str.to_string());
    } else {
        values.insert("operation_mode".to_string(), "---".to_string());
    }
    
    let success = values.values().any(|v| v != "---");
    HttpResponse::Ok().json(ModbusResponse {
        success,
        values,
        error: if success { None } else { Some("Aucune réponse".to_string()) },
    })
}

async fn remote_on(data: web::Data<Mutex<AppState>>) -> impl Responder {
    let state = data.lock().unwrap();
    let port_name = state.port_name.lock().unwrap();
    let success = write_register(&port_name, 6, 0x2800, 0x0004);
    HttpResponse::Ok().json(serde_json::json!({
        "success": success,
        "message": "Télécommande activée"
    }))
}

async fn remote_off(data: web::Data<Mutex<AppState>>) -> impl Responder {
    let state = data.lock().unwrap();
    let port_name = state.port_name.lock().unwrap();
    let success = write_register(&port_name, 6, 0x2800, 0x0000);
    HttpResponse::Ok().json(serde_json::json!({
        "success": success,
        "message": "Télécommande désactivée"
    }))
}

async fn force_double(data: web::Data<Mutex<AppState>>) -> impl Responder {
    let state = data.lock().unwrap();
    let port_name = state.port_name.lock().unwrap();
    let success = write_register(&port_name, 6, 0x2700, 0x00FF);
    HttpResponse::Ok().json(serde_json::json!({
        "success": success,
        "message": "Forçage double déclenché"
    }))
}

async fn force_source1(data: web::Data<Mutex<AppState>>) -> impl Responder {
    let state = data.lock().unwrap();
    let port_name = state.port_name.lock().unwrap();
    let success = write_register(&port_name, 6, 0x2700, 0x0000);
    HttpResponse::Ok().json(serde_json::json!({
        "success": success,
        "message": "Forçage Source I"
    }))
}

async fn force_source2(data: web::Data<Mutex<AppState>>) -> impl Responder {
    let state = data.lock().unwrap();
    let port_name = state.port_name.lock().unwrap();
    let success = write_register(&port_name, 6, 0x2700, 0x00AA);
    HttpResponse::Ok().json(serde_json::json!({
        "success": success,
        "message": "Forçage Source II"
    }))
}

async fn index() -> impl Responder {
    NamedFile::open_async("index.html").await.unwrap()
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    println!("========================================");
    println!("  CHINT ATS - Serveur Rust v2");
    println!("  Port: COM5 | 9600 Even | Adresse 6");
    println!("  Ouvrez http://localhost:5000");
    println!("  Actualisation automatique toutes les 5s");
    println!("========================================");
    
    let app_state = web::Data::new(Mutex::new(AppState {
        port_name: Mutex::new("COM5".to_string()),
    }));
    
    HttpServer::new(move || {
        App::new()
            .app_data(app_state.clone())
            .route("/", web::get().to(index))
            .route("/api/read_all", web::get().to(read_all))
            .route("/api/remote_on", web::get().to(remote_on))
            .route("/api/remote_off", web::get().to(remote_off))
            .route("/api/force_double", web::get().to(force_double))
            .route("/api/force_source1", web::get().to(force_source1))
            .route("/api/force_source2", web::get().to(force_source2))
    })
    .bind("localhost:5000")?
    .run()
    .await
}
