//! daly-bms-probe — outil de diagnostic RS485 brut
//!
//! Teste 3 variantes d'adressage pour chaque BMS afin de déterminer
//! quelle trame provoque une réponse.
//!
//! Variante A : byte[1]=0x40 (PC), data[0]=bms_addr  ← actuel
//! Variante B : byte[1]=bms_addr, data[0]=0x00       ← mode parallèle Daly
//! Variante C : byte[1]=0x40 (PC), data[0]=0x00      ← broadcast standard

use clap::Parser;
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio_serial::SerialPortBuilderExt;

/// Durée d'écoute après chaque envoi (secondes)
const LISTEN_SECS: u64 = 5;

/// Pause entre deux envois (ms)
const PAUSE_MS: u64 = 1000;

/// Commande PackStatus (SOC)
const CMD_PACK_STATUS: u8 = 0x90;

/// Adresse source PC
const PC_ADDR: u8 = 0x40;

#[derive(Parser)]
#[command(name = "daly-bms-probe", about = "Diagnostic RS485 brut pour BMS Daly")]
struct Cli {
    /// Port série (ex: COM6 ou /dev/ttyUSB0)
    #[arg(long, default_value = "COM6")]
    port: String,

    /// Baud rate
    #[arg(long, default_value_t = 9600)]
    baud: u32,

    /// Adresses BMS à sonder (séparées par virgule, ex: 0x01,0x02)
    #[arg(long, default_value = "0x01,0x02")]
    bms: String,
}

fn checksum(data: &[u8]) -> u8 {
    data.iter().fold(0u8, |acc, &b| acc.wrapping_add(b))
}

/// Variante A : byte[1]=0x40 (PC), data[0]=bms_addr
fn frame_a(bms_addr: u8) -> [u8; 13] {
    let mut f = [0u8; 13];
    f[0] = 0xA5;
    f[1] = PC_ADDR;
    f[2] = CMD_PACK_STATUS;
    f[3] = 0x08;
    f[4] = bms_addr;   // data[0] = adresse BMS
    f[12] = checksum(&f[..12]);
    f
}

/// Variante B : byte[1]=bms_addr, data[0]=0x00  (mode parallèle Daly)
fn frame_b(bms_addr: u8) -> [u8; 13] {
    let mut f = [0u8; 13];
    f[0] = 0xA5;
    f[1] = bms_addr;   // adresse BMS dans byte[1]
    f[2] = CMD_PACK_STATUS;
    f[3] = 0x08;
    // data[0..7] = 0x00
    f[12] = checksum(&f[..12]);
    f
}

/// Variante C : byte[1]=0x40 (PC), data[0]=0x00  (broadcast standard)
fn frame_c() -> [u8; 13] {
    let mut f = [0u8; 13];
    f[0] = 0xA5;
    f[1] = PC_ADDR;
    f[2] = CMD_PACK_STATUS;
    f[3] = 0x08;
    // data[0..7] = 0x00 (pas d'adresse spécifique)
    f[12] = checksum(&f[..12]);
    f
}

fn fmt_frame(f: &[u8]) -> String {
    let parts: Vec<String> = f.iter().map(|b| format!("{:02X}", b)).collect();
    format!("[{}]", parts.join(", "))
}

async fn probe_variant(
    port: &mut tokio_serial::SerialStream,
    label: &str,
    frame: &[u8],
) {
    println!("  Variante {} — TX : {}", label, fmt_frame(frame));

    // Vider buffer entrant
    {
        let mut drain = [0u8; 256];
        let _ = tokio::time::timeout(Duration::from_millis(200), port.read(&mut drain)).await;
    }

    if let Err(e) = port.write_all(frame).await {
        println!("    ERREUR envoi : {}", e);
        return;
    }

    let t0 = Instant::now();
    let deadline = Duration::from_secs(LISTEN_SECS);
    let mut frame_buf: Vec<u8> = Vec::new();
    let mut got_response = false;

    loop {
        let elapsed = t0.elapsed();
        if elapsed >= deadline { break; }
        let remaining = deadline - elapsed;

        let mut byte = [0u8; 1];
        match tokio::time::timeout(remaining.min(Duration::from_millis(100)), port.read_exact(&mut byte)).await {
            Ok(Ok(_)) => {
                frame_buf.push(byte[0]);
                if frame_buf.len() == 13 {
                    let ms = t0.elapsed().as_millis();
                    println!("    +{:>5}ms  RX : {}", ms, fmt_frame(&frame_buf));
                    got_response = true;
                    frame_buf.clear();
                }
                if frame_buf.len() >= 64 {
                    println!("    GARBAGE ({} octets) : {}", frame_buf.len(), fmt_frame(&frame_buf));
                    frame_buf.clear();
                }
            }
            Ok(Err(e)) => { println!("    erreur lecture : {}", e); break; }
            Err(_) => {
                // timeout 100ms — afficher partiel si fin de deadline
                if !frame_buf.is_empty() && t0.elapsed() >= deadline {
                    println!("    PARTIEL ({} octets) : {}", frame_buf.len(), fmt_frame(&frame_buf));
                }
            }
        }
    }

    if !got_response {
        println!("    (aucune réponse en {}s)", LISTEN_SECS);
    }
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let addresses: Vec<u8> = cli.bms.split(',')
        .filter_map(|s| {
            let s = s.trim();
            if s.starts_with("0x") || s.starts_with("0X") {
                u8::from_str_radix(&s[2..], 16).ok()
            } else {
                s.parse().ok()
            }
        })
        .collect();

    if addresses.is_empty() {
        eprintln!("ERREUR : aucune adresse BMS valide");
        std::process::exit(1);
    }

    println!("=============================================================");
    println!("  daly-bms-probe  —  Diagnostic adressage RS485");
    println!("  Port : {}  Baud : {}", cli.port, cli.baud);
    println!("  Ecoute : {}s par variante", LISTEN_SECS);
    println!("=============================================================");
    println!();
    println!("  Variante A : byte[1]=0x40(PC), data[0]=bms_addr  (actuel)");
    println!("  Variante B : byte[1]=bms_addr, data[0]=0x00      (mode parallèle)");
    println!("  Variante C : byte[1]=0x40(PC), data[0]=0x00      (broadcast)");
    println!();

    let mut port = tokio_serial::new(&cli.port, cli.baud)
        .timeout(Duration::from_millis(100))
        .open_native_async()
        .unwrap_or_else(|e| {
            eprintln!("ERREUR ouverture {} : {}", cli.port, e);
            std::process::exit(1);
        });

    for &addr in &addresses {
        println!("-------------------------------------------------------------");
        println!("  BMS {:#04x}", addr);
        println!("-------------------------------------------------------------");

        probe_variant(&mut port, "A", &frame_a(addr)).await;
        tokio::time::sleep(Duration::from_millis(PAUSE_MS)).await;

        probe_variant(&mut port, "B", &frame_b(addr)).await;
        tokio::time::sleep(Duration::from_millis(PAUSE_MS)).await;

        probe_variant(&mut port, "C", &frame_c()).await;
        tokio::time::sleep(Duration::from_millis(PAUSE_MS)).await;
        println!();
    }

    println!("=============================================================");
    println!("  Probe terminé. Copiez tout le contenu ci-dessus.");
    println!("=============================================================");
}
