//! Boucle de polling pour le capteur d'irradiance PRALRAN RS485.
//!
//! ## Protocole
//!
//! Modbus RTU FC=04, adresse configurable (défaut 0x05).
//!
//! | Registre | Grandeur      | Format | Unité |
//! |----------|--------------|--------|-------|
//! | 0x0000   | Irradiance   | uint16 | W/m²  |
//!
//! ## Intégration
//!
//! Le snapshot est transmis via callback puis stocké dans `AppState`.
//! Le bridge MQTT publie sur `santuario/irradiance/raw` (retain=true)
//! — même topic que l'ancien `irradiance_reader.py`, Node-RED inchangé.

use super::types::IrradianceSnapshot;
use crate::config::IrradianceConfig;
use chrono::Local;
use rs485_bus::{modbus_rtu, SharedBus};
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, info, warn};

/// Lance la boucle de polling irradiance.
///
/// # Paramètres
/// - `bus`         : bus RS485 partagé (même instance que le bus Daly BMS)
/// - `cfg`         : configuration du capteur
/// - `on_snapshot` : callback appelé à chaque mesure valide
pub async fn run_irradiance_poll_loop<F>(
    bus: Arc<SharedBus>,
    cfg: IrradianceConfig,
    mut on_snapshot: F,
)
where
    F: FnMut(IrradianceSnapshot) + Send + 'static,
{
    let addr = cfg.parsed_address();
    let poll_interval = Duration::from_millis(cfg.poll_interval_ms);

    info!(
        addr = format!("{:#04x}", addr),
        name = %cfg.name,
        interval_ms = cfg.poll_interval_ms,
        "Irradiance PRALRAN polling démarré"
    );

    let mut consecutive_errors: u32 = 0;

    loop {
        match poll_irradiance(&bus, addr, &cfg.name).await {
            Ok(snap) => {
                debug!(
                    addr = format!("{:#04x}", addr),
                    wm2  = snap.irradiance_wm2,
                    "Irradiance OK"
                );
                consecutive_errors = 0;
                on_snapshot(snap);
            }
            Err(e) => {
                consecutive_errors += 1;
                // Log seulement à la 1ère erreur, puis toutes les 12 (même fréquence que Python)
                if consecutive_errors == 1 || consecutive_errors % 12 == 0 {
                    warn!(
                        addr   = format!("{:#04x}", addr),
                        errors = consecutive_errors,
                        "Irradiance erreur lecture : {:#}",
                        e
                    );
                }
            }
        }

        tokio::time::sleep(poll_interval).await;
    }
}

/// Interroge le capteur PRALRAN et retourne un snapshot.
async fn poll_irradiance(
    bus: &SharedBus,
    addr: u8,
    name: &str,
) -> anyhow::Result<IrradianceSnapshot> {
    // FC04, registre 0x0000, 1 registre → irradiance W/m² (uint16)
    let req = modbus_rtu::build_fc04(addr, 0x0000, 1);
    let resp_len = modbus_rtu::response_len(1); // 7 octets

    let resp = bus.transact(&req, resp_len).await
        .map_err(|e| anyhow::anyhow!("PRALRAN {:#04x} transact: {}", addr, e))?;

    let regs = modbus_rtu::parse_read_response(addr, 0x04, &resp)
        .map_err(|e| anyhow::anyhow!("PRALRAN {:#04x} parse: {}", addr, e))?;

    if regs.is_empty() {
        anyhow::bail!("PRALRAN {:#04x}: réponse vide", addr);
    }

    let irradiance_wm2 = regs[0] as f32; // uint16 direct, unité W/m²

    Ok(IrradianceSnapshot {
        address: addr,
        name: name.to_string(),
        timestamp: Local::now(),
        irradiance_wm2,
    })
}
