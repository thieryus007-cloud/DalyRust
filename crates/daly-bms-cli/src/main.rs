//! # daly-bms-cli
//!
//! Outil de ligne de commande pour interagir directement avec les BMS Daly.
//!
//! ## Usage
//! ```bash
//! # Lire le statut d'un BMS
//! daly-bms-cli --port /dev/ttyUSB0 --addr 0x01 status
//!
//! # Lire les tensions de cellules
//! daly-bms-cli --port /dev/ttyUSB0 --addr 0x01 cells
//!
//! # Scanner le bus
//! daly-bms-cli --port /dev/ttyUSB0 discover --start 1 --end 10
//!
//! # Activer le MOS de charge
//! daly-bms-cli --port /dev/ttyUSB0 --addr 0x01 set-charge-mos --enable
//!
//! # Calibrer le SOC
//! daly-bms-cli --port /dev/ttyUSB0 --addr 0x01 set-soc --value 80.0
//!
//! # Polling continu (JSON)
//! daly-bms-cli --port /dev/ttyUSB0 --addr 0x01 poll --interval 2
//! ```

use anyhow::Result;
use clap::{Parser, Subcommand};
use daly_bms_core::{
    bus::{BmsConfig, DalyBusManager, DalyPort},
    commands,
    write,
};
use std::sync::Arc;

// =============================================================================
// CLI Arguments
// =============================================================================

/// DalyBMS CLI — outil de diagnostic et contrôle RS485
#[derive(Parser, Debug)]
#[command(
    name    = "daly-bms-cli",
    version = env!("CARGO_PKG_VERSION"),
    author,
    about   = "Outil CLI pour tester et contrôler les BMS Daly via RS485"
)]
struct Cli {
    /// Port série (ex: /dev/ttyUSB0, COM3)
    #[arg(short, long, default_value = "/dev/ttyUSB0", env = "DALY_PORT")]
    port: String,

    /// Vitesse en bauds
    #[arg(short, long, default_value_t = 9600, env = "DALY_BAUD")]
    baud: u32,

    /// Adresse BMS cible (hex ou décimal, ex: 0x01 ou 1)
    #[arg(short, long, default_value = "0x01")]
    addr: String,

    /// Timeout par commande (ms)
    #[arg(long, default_value_t = 500)]
    timeout_ms: u64,

    /// Niveau de log
    #[arg(long, default_value = "warn", env = "RUST_LOG")]
    log_level: String,

    #[command(subcommand)]
    command: Cmd,
}

#[derive(Subcommand, Debug)]
enum Cmd {
    /// Lire le statut complet (SOC, tension, courant)
    Status,

    /// Lire les tensions individuelles des cellules
    Cells {
        /// Nombre de cellules
        #[arg(short, long, default_value_t = 16)]
        count: u8,
    },

    /// Lire les températures des capteurs
    Temps {
        /// Nombre de capteurs
        #[arg(short, long, default_value_t = 4)]
        count: u8,
    },

    /// Lire l'état des MOSFET
    Mos,

    /// Lire les alarmes
    Alarms,

    /// Scanner le bus pour détecter les BMS
    Discover {
        #[arg(long, default_value_t = 1)]
        start: u8,
        #[arg(long, default_value_t = 16)]
        end: u8,
    },

    /// Polling continu (Ctrl+C pour arrêter)
    Poll {
        /// Intervalle en secondes
        #[arg(short, long, default_value_t = 1)]
        interval: u64,
        /// Nombre de cellules
        #[arg(short, long, default_value_t = 16)]
        cells: u8,
    },

    /// Activer/désactiver le MOS de charge
    SetChargeMos {
        #[arg(long)]
        enable: bool,
        /// Mode read-only (simulé, désactive l'écriture)
        #[arg(long)]
        dry_run: bool,
    },

    /// Activer/désactiver le MOS de décharge
    SetDischargeMos {
        #[arg(long)]
        enable: bool,
        #[arg(long)]
        dry_run: bool,
    },

    /// Calibrer le SOC
    SetSoc {
        #[arg(long)]
        value: f32,
        #[arg(long)]
        dry_run: bool,
    },

    /// Envoyer une trame brute (hex, ex: A540900800000000000000C1)
    Raw {
        /// Trame hex sans espaces
        hex: String,
    },
}

// =============================================================================
// Main
// =============================================================================

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    tracing_subscriber::fmt()
        .with_env_filter(&cli.log_level)
        .init();

    let addr = parse_addr(&cli.addr)?;

    // Ouvrir le port série
    let port = DalyPort::open(&cli.port, cli.baud, cli.timeout_ms)?;

    match cli.command {
        Cmd::Status => cmd_status(&port, addr).await?,

        Cmd::Cells { count } => cmd_cells(&port, addr, count).await?,

        Cmd::Temps { count } => cmd_temps(&port, addr, count).await?,

        Cmd::Mos => cmd_mos(&port, addr).await?,

        Cmd::Alarms => cmd_alarms(&port, addr).await?,

        Cmd::Discover { start, end } => cmd_discover(&port, start, end).await,

        Cmd::Poll { interval, cells } => cmd_poll(&port, addr, cells, interval).await?,

        Cmd::SetChargeMos { enable, dry_run } => {
            write::set_charge_mos(&port, addr, enable, dry_run).await?;
            println!("MOS charge → {}", if enable { "ON" } else { "OFF" });
        }

        Cmd::SetDischargeMos { enable, dry_run } => {
            write::set_discharge_mos(&port, addr, enable, dry_run).await?;
            println!("MOS décharge → {}", if enable { "ON" } else { "OFF" });
        }

        Cmd::SetSoc { value, dry_run } => {
            write::set_soc(&port, addr, value, dry_run).await?;
            println!("SOC calibré à {:.1}%", value);
        }

        Cmd::Raw { hex } => cmd_raw(&port, addr, &hex).await?,
    }

    Ok(())
}

// =============================================================================
// Commandes
// =============================================================================

async fn cmd_status(port: &Arc<DalyPort>, addr: u8) -> Result<()> {
    let soc = commands::get_pack_status(port, addr).await?;
    let mos = commands::get_mos_status(port, addr).await?;

    println!("BMS {:#04x} — Status", addr);
    println!("  Tension     : {:.2} V", soc.voltage);
    println!("  Courant     : {:.1} A", soc.current);
    println!("  Puissance   : {:.1} W", soc.voltage * soc.current);
    println!("  SOC         : {:.1} %", soc.soc);
    println!("  MOS charge  : {}", mos.charge_mos);
    println!("  MOS décharge: {}", mos.discharge_mos);
    println!("  Cycles      : {}", mos.charge_cycles);
    Ok(())
}

async fn cmd_cells(port: &Arc<DalyPort>, addr: u8, count: u8) -> Result<()> {
    let (min_v, min_id, max_v, max_id) = commands::get_cell_voltage_minmax(port, addr).await?;
    let voltages = commands::get_cell_voltages(port, addr, count).await?;

    println!("BMS {:#04x} — Cellules ({} cells)", addr, count);
    println!("  Min : {:.3} V (C{})", min_v, min_id);
    println!("  Max : {:.3} V (C{})", max_v, max_id);
    println!("  Δ   : {:.1} mV", (max_v - min_v) * 1000.0);
    println!();
    for (i, v) in voltages.voltages.iter().enumerate() {
        println!("  C{:02} : {:.3} V", i + 1, v);
    }
    Ok(())
}

async fn cmd_temps(port: &Arc<DalyPort>, addr: u8, count: u8) -> Result<()> {
    let (min_t, min_id, max_t, max_id) = commands::get_temperature_minmax(port, addr).await?;
    let temps = commands::get_temperatures(port, addr, count).await?;

    println!("BMS {:#04x} — Températures", addr);
    println!("  Min : {:.1} °C (capteur {})", min_t, min_id);
    println!("  Max : {:.1} °C (capteur {})", max_t, max_id);
    for (i, t) in temps.temperatures.iter().enumerate() {
        println!("  T{:02} : {:.1} °C", i + 1, t);
    }
    Ok(())
}

async fn cmd_mos(port: &Arc<DalyPort>, addr: u8) -> Result<()> {
    let mos = commands::get_mos_status(port, addr).await?;
    println!("BMS {:#04x} — MOS", addr);
    println!("  Charge MOS  : {}", mos.charge_mos);
    println!("  Décharge MOS: {}", mos.discharge_mos);
    println!("  Cycles      : {}", mos.charge_cycles);
    println!("  Capacité    : {} mAh", mos.residual_capacity_mah);
    Ok(())
}

async fn cmd_alarms(port: &Arc<DalyPort>, addr: u8) -> Result<()> {
    let (_, _, bytes) = commands::get_alarm_flags(port, addr).await?;
    let alarms = commands::parse_alarm_flags(&bytes);
    let json = serde_json::to_string_pretty(&alarms)?;
    println!("{}", json);
    Ok(())
}

async fn cmd_discover(port: &Arc<DalyPort>, start: u8, end: u8) {
    println!("Scan bus RS485 de {:#04x} à {:#04x}…", start, end);
    let dummy = daly_bms_core::bus::BmsConfig::new(start);
    let manager = Arc::new(DalyBusManager::new(port.clone(), vec![dummy]));
    let found = manager.discover(start, end).await;
    if found.is_empty() {
        println!("Aucun BMS détecté.");
    } else {
        println!("BMS trouvés : {:?}", found.iter().map(|a| format!("{:#04x}", a)).collect::<Vec<_>>());
    }
}

async fn cmd_poll(port: &Arc<DalyPort>, addr: u8, cells: u8, interval_sec: u64) -> Result<()> {
    use tokio::time::{sleep, Duration};
    println!("Polling BMS {:#04x} (Ctrl+C pour arrêter)…", addr);
    loop {
        match commands::get_pack_status(port, addr).await {
            Ok(soc) => {
                println!(
                    "[{}] SOC={:.1}% V={:.2}V I={:.1}A",
                    chrono::Utc::now().format("%H:%M:%S"),
                    soc.soc,
                    soc.voltage,
                    soc.current,
                );
            }
            Err(e) => eprintln!("Erreur : {:?}", e),
        }
        sleep(Duration::from_secs(interval_sec)).await;
    }
}

async fn cmd_raw(port: &Arc<DalyPort>, addr: u8, hex: &str) -> Result<()> {
    use daly_bms_core::protocol::DataId;
    let bytes = hex::decode(hex.replace(' ', ""))
        .map_err(|e| anyhow::anyhow!("Hex invalide : {}", e))?;
    if bytes.len() < 3 {
        anyhow::bail!("Trame trop courte");
    }
    let cmd_byte = bytes[2];
    let cmd = DataId::from_u8(cmd_byte)
        .ok_or_else(|| anyhow::anyhow!("Data ID inconnu : {:#04x}", cmd_byte))?;
    let mut data = [0u8; 8];
    if bytes.len() >= 12 {
        data.copy_from_slice(&bytes[4..12]);
    }
    let resp = port.send_command(addr, cmd, data).await?;
    println!("Réponse : {:02X?}", resp.bytes);
    Ok(())
}

// =============================================================================
// Utilitaires
// =============================================================================

fn parse_addr(s: &str) -> Result<u8> {
    let s = s.trim();
    if s.starts_with("0x") || s.starts_with("0X") {
        Ok(u8::from_str_radix(&s[2..], 16)?)
    } else {
        Ok(s.parse::<u8>()?)
    }
}
