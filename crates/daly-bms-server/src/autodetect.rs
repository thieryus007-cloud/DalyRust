//! Auto-détection du port série et des BMS Daly connectés.
//!
//! Utilisé au démarrage quand aucun port ni adresse n'est configuré
//! (pas de fichier config.toml, pas d'argument CLI).

use daly_bms_core::bus::DalyPort;
use daly_bms_core::protocol::DataId;
use std::sync::Arc;
use tracing::{debug, info, warn};

/// Scanne tous les ports série disponibles et retourne le premier sur lequel
/// un Daly BMS répond à PackStatus (0x90, adresse 0x01).
///
/// Retourne le nom du port ET le port déjà ouvert pour éviter une double
/// ouverture (problème "Accès refusé" sur Windows).
///
/// Timeout par port : 800 ms (un Daly répond normalement en < 200 ms).
pub async fn find_daly_port(baud: u32) -> Option<(String, Arc<DalyPort>)> {
    let ports = match tokio_serial::available_ports() {
        Ok(p) => p,
        Err(e) => {
            warn!("Impossible de lister les ports série : {}", e);
            return None;
        }
    };

    if ports.is_empty() {
        warn!("Aucun port série détecté sur ce système.");
        return None;
    }

    info!(
        "Auto-détection port Daly : {} port(s) disponible(s) : {}",
        ports.len(),
        ports.iter().map(|p| p.port_name.as_str()).collect::<Vec<_>>().join(", ")
    );

    for port_info in ports {
        let name = port_info.port_name.clone();
        debug!("Test port {} ...", name);

        let port = match DalyPort::open(&name, baud, 800) {
            Ok(p) => p,
            Err(e) => {
                debug!("{} : ouverture impossible ({})", name, e);
                continue;
            }
        };

        match port.send_command(0x01, DataId::PackStatus, [0u8; 8]).await {
            Ok(_) => {
                info!("Daly BMS détecté sur {}", name);
                // Retourner le port déjà ouvert — évite une 2ème ouverture sur Windows
                return Some((name, port));
            }
            Err(e) => {
                debug!("{} : pas de réponse Daly ({})", name, e);
            }
        }
    }

    warn!("Aucun Daly BMS trouvé sur les ports disponibles.");
    None
}
