//! daly-bms-probe — outil de diagnostic RS485 brut
//!
//! Protocole Daly V1.21 §2.1 multi-BMS confirmé :
//!   byte[1] requête = 0x3F + board_number
//!   Board 1 → byte[1]=0x40 → réponse byte[1]=0x01
//!   Board 2 → byte[1]=0x41 → réponse byte[1]=0x02

use clap::Parser;
use daly_bms_core::protocol::{checksum, pc_address_for, DataId, RequestFrame, FRAME_LEN, START_FLAG};
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio_serial::SerialPortBuilderExt;

const LISTEN_SECS: u64 = 5;
const PAUSE_MS:    u64 = 800;

#[derive(Parser)]
#[command(name = "daly-bms-probe", about = "Diagnostic RS485 Daly multi-BMS")]
struct Cli {
    #[arg(long, default_value = "COM6")]
    port: String,

    #[arg(long, default_value_t = 9600)]
    baud: u32,

    /// Board numbers à interroger (ex: 1,2)
    #[arg(long, default_value = "1,2")]
    boards: String,
}

fn fmt_frame(f: &[u8]) -> String {
    let parts: Vec<String> = f.iter().map(|b| format!("{:02X}", b)).collect();
    format!("[{}]", parts.join(", "))
}

fn decode_response(f: &[u8]) {
    if f.len() != FRAME_LEN || f[0] != START_FLAG { return; }
    let bms_addr = f[1];
    let cmd      = f[2];
    let chk_ok   = if checksum(&f[..12]) == f[12] { "chk=OK" } else { "chk=ERREUR" };

    if cmd == DataId::PackStatus as u8 {
        // frame[4..11] = data[0..7]
        // D0-D1 : tension totale (0.1 V)
        // D2-D3 : tension acquisition (ignoré)
        // D4-D5 : courant (offset 30000, 0.1 A)  → f[8], f[9]
        // D6-D7 : SOC (0.1 %)                    → f[10], f[11]
        let voltage     = u16::from_be_bytes([f[4], f[5]]);
        let current_raw = u16::from_be_bytes([f[8], f[9]]) as i32;
        let soc         = u16::from_be_bytes([f[10], f[11]]);
        println!("      → BMS={:#04x}  V={:.1}V  I={:+.1}A  SOC={:.1}%  {}",
            bms_addr,
            voltage as f32 / 10.0,
            (current_raw - 30_000) as f32 / 10.0,
            soc as f32 / 10.0,
            chk_ok,
        );
    } else if cmd == DataId::MosStatus as u8 {
        // D0=state(0=repos,1=chg,2=dch), D1=CHG_MOS, D2=DCH_MOS, D3=cycles, D4..7=mAh
        let state    = f[4];
        let chg_mos  = f[5];
        let dch_mos  = f[6];
        let cycles   = f[7];
        let capacity = u32::from_be_bytes([f[8], f[9], f[10], f[11]]);
        let state_str = match state { 0 => "repos", 1 => "charge", 2 => "décharge", _ => "?" };
        println!("      → BMS={:#04x}  state={}  CHG_MOS={}  DCH_MOS={}  cycles={}  cap={}mAh  {}",
            bms_addr, state_str, chg_mos, dch_mos, cycles, capacity, chk_ok);
    } else if cmd == DataId::StatusInfo1 as u8 {
        // D0=nb_cells, D1=nb_temp, D2=charger, D3=load, D4=DIO, D5-D6=cycle_count
        let cells    = f[4];
        let temps    = f[5];
        let charger  = f[6];
        let load     = f[7];
        let cycles   = u16::from_be_bytes([f[9], f[10]]);
        println!("      → BMS={:#04x}  cells={}  temp_sensors={}  charger={}  load={}  cycles={}  {}",
            bms_addr, cells, temps, charger, load, cycles, chk_ok);
    } else {
        println!("      → BMS={:#04x}  cmd={:#04x}  {}", bms_addr, cmd, chk_ok);
    }
}

async fn probe_once(port: &mut tokio_serial::SerialStream, label: &str, frame: &[u8]) {
    println!("  {}  TX : {}", label, fmt_frame(frame));

    // Vider buffer entrant
    let mut drain = [0u8; 256];
    let _ = tokio::time::timeout(Duration::from_millis(100), port.read(&mut drain)).await;

    if let Err(e) = port.write_all(frame).await {
        println!("      ERREUR envoi : {}", e);
        return;
    }

    let t0 = Instant::now();
    let deadline = Duration::from_secs(LISTEN_SECS);
    let mut buf: Vec<u8> = Vec::new();
    let mut got = false;

    loop {
        let elapsed = t0.elapsed();
        if elapsed >= deadline { break; }
        let remaining = deadline - elapsed;

        let mut byte = [0u8; 1];
        match tokio::time::timeout(remaining.min(Duration::from_millis(100)), port.read_exact(&mut byte)).await {
            Ok(Ok(_)) => {
                buf.push(byte[0]);
                if buf.len() == FRAME_LEN {
                    println!("      +{:>5}ms  RX : {}", t0.elapsed().as_millis(), fmt_frame(&buf));
                    decode_response(&buf);
                    got = true;
                    buf.clear();
                }
                if buf.len() >= 64 { buf.clear(); }
            }
            _ => {}
        }
    }
    if !got { println!("      (aucune réponse en {}s)", LISTEN_SECS); }
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let board_numbers: Vec<u8> = cli.boards.split(',')
        .filter_map(|s| s.trim().parse::<u8>().ok())
        .collect();

    if board_numbers.is_empty() {
        eprintln!("ERREUR : aucun board number valide (ex: --boards 1,2)");
        std::process::exit(1);
    }

    println!("=============================================================");
    println!("  daly-bms-probe  —  Daly multi-BMS RS485");
    println!("  Port : {}  Baud : {}", cli.port, cli.baud);
    println!("  Board N → requête byte[1] = 0x{:02X}+N", 0x3Fu8);
    println!("=============================================================");

    let mut port = tokio_serial::new(&cli.port, cli.baud)
        .timeout(Duration::from_millis(100))
        .open_native_async()
        .unwrap_or_else(|e| {
            eprintln!("ERREUR ouverture {} : {}", cli.port, e);
            std::process::exit(1);
        });

    for &board in &board_numbers {
        let pc_addr = pc_address_for(board);
        println!();
        println!("-------------------------------------------------------------");
        println!("  Board {}  (byte[1] requête = {:#04x})", board, pc_addr);
        println!("-------------------------------------------------------------");

        let frame = RequestFrame::read(board, DataId::PackStatus);
        probe_once(&mut port, "PackStatus (0x90)", frame.as_bytes()).await;
        tokio::time::sleep(Duration::from_millis(PAUSE_MS)).await;

        let frame = RequestFrame::read(board, DataId::MosStatus);
        probe_once(&mut port, "MosStatus  (0x93)", frame.as_bytes()).await;
        tokio::time::sleep(Duration::from_millis(PAUSE_MS)).await;

        let frame = RequestFrame::read(board, DataId::StatusInfo1);
        probe_once(&mut port, "StatusInfo (0x94)", frame.as_bytes()).await;
        tokio::time::sleep(Duration::from_millis(PAUSE_MS)).await;
    }

    println!();
    println!("=============================================================");
    println!("  Probe terminé.");
    println!("=============================================================");
}
