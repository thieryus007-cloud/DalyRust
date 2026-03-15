//! daly-bms-probe — outil de diagnostic RS485 brut
//!
//! Envoie une requête SOC (0x90) à chaque adresse BMS spécifiée,
//! puis lit TOUT ce qui arrive pendant LISTEN_SECS secondes.
//! Aucun décodage : hex brut uniquement.

use clap::Parser;
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio_serial::SerialPortBuilderExt;

/// Durée d'écoute après chaque envoi de trame
const LISTEN_SECS: u64 = 30;

/// Pause entre les deux sessions (BMS 0x01 → BMS 0x02)
const PAUSE_BETWEEN_SECS: u64 = 3;

/// Commande PackStatus (SOC)
const CMD_PACK_STATUS: u8 = 0x90;

/// Adresse source PC dans le protocole Daly
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

/// Calcule le checksum Daly : somme de tous les octets, tronquée à u8
fn checksum(data: &[u8]) -> u8 {
    data.iter().fold(0u8, |acc, &b| acc.wrapping_add(b))
}

/// Construit la trame de requête pour une adresse BMS donnée (commande SOC / 0x90)
///
/// Format standard Daly (13 octets) :
///   [A5] [40] [cmd] [08] [data0..7] [checksum]
///
/// data[0] = adresse BMS cible (pour adressage parallèle Daly)
fn build_request(bms_addr: u8, cmd: u8) -> [u8; 13] {
    let mut frame = [0u8; 13];
    frame[0] = 0xA5;        // start
    frame[1] = PC_ADDR;     // source : PC (0x40)
    frame[2] = cmd;         // commande
    frame[3] = 0x08;        // longueur data
    frame[4] = bms_addr;    // data[0] = adresse BMS cible
    // frame[5..11] = 0x00
    frame[12] = checksum(&frame[..12]);
    frame
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    // Parse les adresses BMS
    let addresses: Vec<u8> = cli
        .bms
        .split(',')
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
    println!("  daly-bms-probe  —  Diagnostic RS485 brut");
    println!("  Port : {}  Baud : {}", cli.port, cli.baud);
    println!("  Ecoute : {}s par BMS", LISTEN_SECS);
    println!("=============================================================");
    println!();

    // Ouvre le port série
    let mut port = tokio_serial::new(&cli.port, cli.baud)
        .timeout(Duration::from_millis(100))
        .open_native_async()
        .unwrap_or_else(|e| {
            eprintln!("ERREUR ouverture {} : {}", cli.port, e);
            std::process::exit(1);
        });

    for (i, &addr) in addresses.iter().enumerate() {
        // Pause entre sessions (sauf la première)
        if i > 0 {
            println!();
            println!("--- Pause {}s avant session suivante ---", PAUSE_BETWEEN_SECS);
            tokio::time::sleep(Duration::from_secs(PAUSE_BETWEEN_SECS)).await;
        }

        let request = build_request(addr, CMD_PACK_STATUS);

        println!("=============================================================");
        println!("  SESSION BMS {:#04x}  (commande {:#04x} = PackStatus/SOC)", addr, CMD_PACK_STATUS);
        println!("=============================================================");
        println!();
        println!(">>> TX trame ({} octets) :", request.len());
        print!("    [");
        for (j, b) in request.iter().enumerate() {
            if j > 0 { print!(", "); }
            print!("{:02X}", b);
        }
        println!("]");
        println!();

        // Vider le buffer en entrée (lecture rapide jusqu'à timeout)
        {
            let mut drain = [0u8; 256];
            let _ = tokio::time::timeout(
                Duration::from_millis(200),
                port.read(&mut drain),
            ).await;
        }

        // Envoyer la requête
        if let Err(e) = port.write_all(&request).await {
            eprintln!("ERREUR envoi : {}", e);
            continue;
        }
        let t0 = Instant::now();

        println!("<<< RX brut pendant {}s :", LISTEN_SECS);
        println!();

        let mut total_bytes = 0usize;
        let mut frame_buf: Vec<u8> = Vec::new();
        let deadline = Duration::from_secs(LISTEN_SECS);

        loop {
            let elapsed = t0.elapsed();
            if elapsed >= deadline {
                break;
            }
            let remaining = deadline - elapsed;

            let mut byte = [0u8; 1];
            match tokio::time::timeout(remaining.min(Duration::from_millis(100)), port.read_exact(&mut byte)).await {
                Ok(Ok(_)) => {
                    frame_buf.push(byte[0]);
                    total_bytes += 1;

                    // Afficher la trame dès qu'on a 13 octets (taille standard Daly)
                    if frame_buf.len() == 13 {
                        let ms = t0.elapsed().as_millis();
                        print!("  +{:>6}ms  [", ms);
                        for (j, b) in frame_buf.iter().enumerate() {
                            if j > 0 { print!(", "); }
                            print!("{:02X}", b);
                        }
                        println!("]");
                        frame_buf.clear();
                    }
                    // Sécurité : si on accumule sans voir de trame valide, vider
                    if frame_buf.len() >= 64 {
                        let ms = t0.elapsed().as_millis();
                        print!("  +{:>6}ms  GARBAGE ({} octets) [", ms, frame_buf.len());
                        for (j, b) in frame_buf.iter().enumerate() {
                            if j > 0 { print!(", "); }
                            print!("{:02X}", b);
                        }
                        println!("]");
                        frame_buf.clear();
                    }
                }
                Ok(Err(e)) => {
                    eprintln!("  erreur lecture : {}", e);
                    break;
                }
                Err(_timeout) => {
                    // Rien reçu dans les 100ms — afficher le reste s'il y en a
                    if !frame_buf.is_empty() {
                        let ms = t0.elapsed().as_millis();
                        print!("  +{:>6}ms  PARTIEL ({} octets) [", ms, frame_buf.len());
                        for (j, b) in frame_buf.iter().enumerate() {
                            if j > 0 { print!(", "); }
                            print!("{:02X}", b);
                        }
                        println!("]");
                        frame_buf.clear();
                    }
                }
            }
        }

        // Résidu final
        if !frame_buf.is_empty() {
            let ms = t0.elapsed().as_millis();
            print!("  +{:>6}ms  RÉSIDU ({} octets) [", ms, frame_buf.len());
            for (j, b) in frame_buf.iter().enumerate() {
                if j > 0 { print!(", "); }
                print!("{:02X}", b);
            }
            println!("]");
        }

        println!();
        println!("  Total reçu : {} octets ({} trames de 13 octets)", total_bytes, total_bytes / 13);
    }

    println!();
    println!("=============================================================");
    println!("  Probe terminé. Copiez tout le contenu ci-dessus.");
    println!("=============================================================");
}
