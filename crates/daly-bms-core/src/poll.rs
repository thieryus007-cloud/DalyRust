//! Boucle de polling asynchrone avec reconnexion automatique et backoff exponentiel.
//!
//! La fonction principale [`poll_loop`] interroge cycliquement tous les BMS
//! configurés et appelle le callback `on_snapshot` pour chaque snapshot produit.

use crate::bus::{BmsConfig, DalyBusManager, DalyPort};
use crate::commands;
use crate::error::DalyError;
use crate::types::{
    Alarms, BmsSnapshot, DcData, HistoryData, InfoData, IoData, SystemData,
};
use chrono::Utc;
use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;
use tracing::{error, info, warn};

/// Configuration de la boucle de polling.
#[derive(Debug, Clone)]
pub struct PollConfig {
    /// Intervalle entre deux cycles de polling complets (ms).
    pub interval_ms: u64,
    /// Nombre de tentatives par commande avant de marquer un BMS en erreur.
    pub retries: u8,
    /// Délai initial pour le backoff exponentiel en cas d'erreur (ms).
    pub backoff_initial_ms: u64,
    /// Délai maximum de backoff (ms).
    pub backoff_max_ms: u64,
}

impl Default for PollConfig {
    fn default() -> Self {
        Self {
            interval_ms:       1000,
            retries:           3,
            backoff_initial_ms: 2000,
            backoff_max_ms:    30_000,
        }
    }
}

/// Exécute la boucle de polling infinie pour tous les BMS du manager.
///
/// Pour chaque BMS, toutes les commandes de lecture sont émises séquentiellement.
/// Le snapshot résultant est passé au callback `on_snapshot`.
///
/// En cas d'erreur série (port perdu), la boucle attend `backoff` ms et retente.
pub async fn poll_loop<F>(
    manager: Arc<DalyBusManager>,
    config: PollConfig,
    on_snapshot: F,
) where
    F: Fn(BmsSnapshot) + Send + Sync + 'static,
{
    let on_snapshot = Arc::new(on_snapshot);
    let mut backoff_ms = config.backoff_initial_ms;

    loop {
        let cycle_start = std::time::Instant::now();

        for device in &manager.devices {
            match poll_device(&manager.port, device, &config).await {
                Ok(snapshot) => {
                    backoff_ms = config.backoff_initial_ms; // reset backoff
                    on_snapshot(snapshot);
                }
                Err(DalyError::Timeout { .. }) => {
                    warn!(
                        bms = format!("{:#04x}", device.address),
                        "Timeout — BMS peut-être hors ligne"
                    );
                }
                Err(DalyError::Serial(e)) => {
                    error!("Erreur port série : {} — backoff {}ms", e, backoff_ms);
                    tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
                    backoff_ms = (backoff_ms * 2).min(config.backoff_max_ms);
                    break; // sortir de la boucle devices et réessayer le cycle
                }
                Err(e) => {
                    warn!(
                        bms = format!("{:#04x}", device.address),
                        "Erreur : {:?}", e
                    );
                }
            }
        }

        // Attendre le reste de l'intervalle configuré
        let elapsed = cycle_start.elapsed();
        let interval = Duration::from_millis(config.interval_ms);
        if elapsed < interval {
            tokio::time::sleep(interval - elapsed).await;
        }
    }
}

/// Poll complet d'un seul BMS : toutes les commandes de lecture.
///
/// Retourne un [`BmsSnapshot`] agrégé ou une [`DalyError`].
async fn poll_device(
    port: &Arc<DalyPort>,
    device: &BmsConfig,
    config: &PollConfig,
) -> crate::error::Result<BmsSnapshot> {
    let addr = device.address;

    // ── 0x90 : Pack status (tension, courant, SOC) ────────────────────────────
    let soc_data = retry(config.retries, || {
        commands::get_pack_status(port, addr)
    }).await?;

    // ── 0x91 : Min/max tensions cellules ──────────────────────────────────────
    let (min_cell_v, min_cell_id, max_cell_v, max_cell_id) = retry(config.retries, || {
        commands::get_cell_voltage_minmax(port, addr)
    }).await?;

    // ── 0x92 : Min/max températures ───────────────────────────────────────────
    let (min_temp, min_temp_id, max_temp, max_temp_id) = retry(config.retries, || {
        commands::get_temperature_minmax(port, addr)
    }).await?;

    // ── 0x93 : État MOS, cycles, capacité ─────────────────────────────────────
    let mos = retry(config.retries, || {
        commands::get_mos_status(port, addr)
    }).await?;

    // ── 0x94 : Status info 1 ──────────────────────────────────────────────────
    let status = retry(config.retries, || {
        commands::get_status_info(port, addr)
    }).await?;

    // ── 0x95 : Tensions individuelles ─────────────────────────────────────────
    let cell_voltages = retry(config.retries, || {
        commands::get_cell_voltages(port, addr, device.cell_count)
    }).await.unwrap_or_default();

    // ── 0x96 : Températures individuelles ─────────────────────────────────────
    let temperatures = retry(config.retries, || {
        commands::get_temperatures(port, addr, device.temp_sensor_count)
    }).await.unwrap_or_default();

    // ── 0x97 : Flags d'équilibrage ────────────────────────────────────────────
    let balance_flags = retry(config.retries, || {
        commands::get_balance_flags(port, addr, device.cell_count)
    }).await.unwrap_or_default();

    // ── 0x98 : Alarmes ────────────────────────────────────────────────────────
    let (_charge_en, _discharge_en, alarm_bytes) = retry(config.retries, || {
        commands::get_alarm_flags(port, addr)
    }).await.unwrap_or((true, true, [0u8; 7]));

    let alarms = commands::parse_alarm_flags(&alarm_bytes);

    // ── Assemblage du snapshot ────────────────────────────────────────────────
    let dc = DcData {
        voltage:     soc_data.voltage,
        current:     soc_data.current,
        power:       soc_data.voltage * soc_data.current,
        temperature: max_temp,
    };

    let capacity_ah = device.installed_capacity_ah * soc_data.soc / 100.0;
    let consumed_ah = device.installed_capacity_ah - capacity_ah;

    let system = SystemData {
        min_voltage_cell_id: format!("C{}", min_cell_id),
        min_cell_voltage:    min_cell_v,
        max_voltage_cell_id: format!("C{}", max_cell_id),
        max_cell_voltage:    max_cell_v,
        min_temperature_cell_id: format!("C{}", min_temp_id),
        min_cell_temperature:    min_temp,
        max_temperature_cell_id: format!("C{}", max_temp_id),
        max_cell_temperature:    max_temp,
        mos_temperature:     max_temp, // MOS temp from external sensor if available
        nr_of_modules_online: 1,
        nr_of_modules_offline: 0,
        nr_of_cells_per_battery: device.cell_count,
        nr_of_modules_blocking_charge:    u8::from(!mos.charge_mos),
        nr_of_modules_blocking_discharge: u8::from(!mos.discharge_mos),
    };

    let io = IoData {
        allow_to_charge:    u8::from(mos.charge_mos),
        allow_to_discharge: u8::from(mos.discharge_mos),
        allow_to_balance:   1,
        allow_to_heat:      0,
        external_relay:     status.charger_status,
    };

    let history = HistoryData {
        charge_cycles:   mos.charge_cycles,
        minimum_voltage: 0.0, // non disponible en temps réel
        maximum_voltage: 0.0,
        total_ah_drawn:  0.0,
    };

    // TimeToSoc simplifié : interpolation linéaire
    let time_to_soc = compute_time_to_soc(soc_data.soc, soc_data.current, device.installed_capacity_ah);

    let snapshot = BmsSnapshot {
        address:            addr,
        timestamp:          Utc::now(),
        dc,
        installed_capacity: device.installed_capacity_ah,
        consumed_amphours:  consumed_ah,
        capacity:           capacity_ah,
        soc:                soc_data.soc,
        soh:                100.0, // non disponible directement
        time_to_go:         compute_time_to_go(capacity_ah, soc_data.current),
        balancing:          balance_flags.flags.iter().any(|&f| f) as u8,
        system_switch:      u8::from(mos.charge_mos || mos.discharge_mos),
        alarms,
        info:               InfoData::default(),
        history,
        system,
        voltages:           cell_voltages.to_named_map(),
        balances:           balance_flags.to_named_map(),
        io,
        heating:            0,
        time_to_soc,
    };

    info!(
        bms = format!("{:#04x}", addr),
        soc = format!("{:.1}%", snapshot.soc),
        voltage = format!("{:.2}V", snapshot.dc.voltage),
        current = format!("{:.1}A", snapshot.dc.current),
        "Snapshot OK"
    );

    Ok(snapshot)
}

// =============================================================================
// Utilitaires
// =============================================================================

/// Calcule le temps estimé jusqu'à la décharge complète (secondes).
fn compute_time_to_go(capacity_ah: f32, current_a: f32) -> u32 {
    if current_a >= 0.0 || capacity_ah <= 0.0 {
        return 0;
    }
    let hours = capacity_ah / (-current_a);
    (hours * 3600.0) as u32
}

/// Calcule la map TimeToSoC : SOC% → secondes pour atteindre ce palier.
fn compute_time_to_soc(
    current_soc: f32,
    current_a: f32,
    installed_ah: f32,
) -> BTreeMap<u8, u32> {
    let mut map = BTreeMap::new();
    for soc_target in (0..=100u8).step_by(5) {
        let delta_soc = soc_target as f32 - current_soc;
        let delta_ah  = installed_ah * delta_soc / 100.0;
        let seconds = if current_a.abs() < 0.1 {
            0u32
        } else {
            let hours = (delta_ah / current_a.abs()).abs();
            (hours * 3600.0) as u32
        };
        map.insert(soc_target, seconds);
    }
    map
}

/// Exécute `f` jusqu'à `retries` fois en cas d'erreur non-fatale.
async fn retry<F, Fut, T>(retries: u8, mut f: F) -> crate::error::Result<T>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = crate::error::Result<T>>,
{
    let mut last_err = DalyError::Other(anyhow::anyhow!("Aucune tentative effectuée"));
    for attempt in 0..=retries {
        match f().await {
            Ok(v) => return Ok(v),
            Err(e @ DalyError::Serial(_)) | Err(e @ DalyError::Io(_)) => {
                // Erreur fatale (port série) — ne pas réessayer
                return Err(e);
            }
            Err(e) => {
                if attempt < retries {
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
                last_err = e;
            }
        }
    }
    Err(last_err)
}
