//! Bridge InfluxDB 2.x — écriture en batch des snapshots BMS.
//!
//! Chaque snapshot est converti en points InfluxDB et accumulé dans un batch.
//! Le batch est flushé soit quand il atteint `batch_size`, soit toutes les
//! `batch_flush_interval_sec` secondes.

use crate::config::InfluxConfig;
use crate::state::AppState;
use daly_bms_core::types::BmsSnapshot;
use influxdb2::Client;
use influxdb2::models::DataPoint;
use tracing::{error, info, warn};
use std::time::Duration;
use tokio::sync::mpsc;

/// Démarre la tâche d'écriture InfluxDB en arrière-plan.
pub async fn run_influx_bridge(state: AppState, cfg: InfluxConfig) {
    if !cfg.enabled {
        info!("InfluxDB bridge désactivé (enabled = false)");
        return;
    }
    if cfg.token.is_empty() {
        warn!("InfluxDB : token vide, bridge désactivé");
        return;
    }

    info!(url = %cfg.url, org = %cfg.org, bucket = %cfg.bucket, "Démarrage InfluxDB bridge");

    let client = Client::new(&cfg.url, &cfg.org, &cfg.token);

    let mut batch: Vec<DataPoint> = Vec::with_capacity(cfg.batch_size);
    let mut rx = state.subscribe_ws();
    let flush_interval = Duration::from_secs_f64(cfg.batch_flush_interval_sec.max(1.0));
    let mut flush_ticker = tokio::time::interval(flush_interval);

    loop {
        tokio::select! {
            Ok(snaps) = rx.recv() => {
                for snap in snaps.iter() {
                    let points = snapshot_to_points(snap);
                    batch.extend(points);
                }
                if batch.len() >= cfg.batch_size {
                    flush_batch(&client, &cfg.bucket, &mut batch).await;
                }
            }
            _ = flush_ticker.tick() => {
                if !batch.is_empty() {
                    flush_batch(&client, &cfg.bucket, &mut batch).await;
                }
            }
        }
    }
}

/// Flush le batch vers InfluxDB et vide le vecteur.
async fn flush_batch(client: &Client, bucket: &str, batch: &mut Vec<DataPoint>) {
    let points = std::mem::take(batch);
    let count = points.len();
    match client.write(bucket, futures::stream::iter(points)).await {
        Ok(_) => info!(count, "InfluxDB flush OK"),
        Err(e) => error!("InfluxDB flush erreur : {:?}", e),
    }
}

/// Convertit un [`BmsSnapshot`] en plusieurs points InfluxDB.
///
/// Measurement principal : `bms_status`
/// Tags : `address` (hex)
fn snapshot_to_points(snap: &BmsSnapshot) -> Vec<DataPoint> {
    let addr_tag = format!("{:#04x}", snap.address);
    let ts_ns = snap.timestamp.timestamp_nanos_opt().unwrap_or(0) as u128;

    let mut points = vec![
        // ── Status principal ─────────────────────────────────────────────────
        DataPoint::builder("bms_status")
            .tag("address", addr_tag.clone())
            .field("soc",         snap.soc as f64)
            .field("voltage",     snap.dc.voltage as f64)
            .field("current",     snap.dc.current as f64)
            .field("power",       snap.dc.power as f64)
            .field("capacity",    snap.capacity as f64)
            .field("consumed_ah", snap.consumed_amphours as f64)
            .field("temp_max",    snap.system.max_cell_temperature as f64)
            .field("temp_min",    snap.system.min_cell_temperature as f64)
            .field("cell_delta_mv", snap.system.cell_delta_mv() as f64)
            .field("min_cell_v",  snap.system.min_cell_voltage as f64)
            .field("max_cell_v",  snap.system.max_cell_voltage as f64)
            .field("charge_mos",  snap.io.allow_to_charge as i64)
            .field("discharge_mos", snap.io.allow_to_discharge as i64)
            .field("any_alarm",   snap.alarms.any_active() as i64)
            .timestamp(ts_ns as i64)
            .build()
            .expect("point valide"),
    ];

    // ── Tensions individuelles ─────────────────────────────────────────────
    for (name, &v) in &snap.voltages {
        if let Ok(p) = DataPoint::builder("bms_cell_voltage")
            .tag("address", addr_tag.clone())
            .tag("cell", name.clone())
            .field("voltage", v as f64)
            .timestamp(ts_ns as i64)
            .build()
        {
            points.push(p);
        }
    }

    points
}
