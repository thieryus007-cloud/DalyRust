//! Endpoints système : status global, config, découverte.

use crate::state::AppState;
use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::atomic::Ordering;

/// GET /api/v1/system/status
///
/// Retourne l'état global : BMS connectés, polling actif, clients WS.
pub async fn get_status(State(state): State<AppState>) -> Json<Value> {
    let buffers = state.buffers.read().await;
    let bms_list: Vec<Value> = buffers
        .iter()
        .map(|(addr, buf)| {
            let online = buf.latest().is_some();
            let last_ts = buf.latest().map(|s| s.timestamp.to_rfc3339());
            json!({
                "address": format!("{:#04x}", addr),
                "online": online,
                "last_update": last_ts,
                "snapshots_count": buf.buffer.len(),
            })
        })
        .collect();

    Json(json!({
        "polling_active": state.polling_active.load(Ordering::Relaxed),
        "bms_count": bms_list.len(),
        "bms": bms_list,
        "version": env!("CARGO_PKG_VERSION"),
    }))
}

/// GET /api/v1/config
///
/// Retourne la configuration active (sans secrets).
pub async fn get_config(State(state): State<AppState>) -> Json<Value> {
    let cfg = &state.config;
    let addresses = cfg.bms_addresses()
        .iter()
        .map(|a| format!("{:#04x}", a))
        .collect::<Vec<_>>();

    Json(json!({
        "serial": {
            "port": cfg.serial.port,
            "baud": cfg.serial.baud,
            "poll_interval_ms": cfg.serial.poll_interval_ms,
        },
        "api": {
            "bind": cfg.api.bind,
            "auth_enabled": !cfg.api.api_key.is_empty(),
        },
        "addresses": addresses,
        "mqtt_enabled": cfg.mqtt.enabled,
        "influxdb_enabled": cfg.influxdb.enabled,
        "read_only": cfg.read_only.enabled,
    }))
}

/// GET /api/v1/system/logs?limit=N
///
/// Retourne les dernières entrées de logs capturées en mémoire (max 200).
#[derive(Deserialize)]
pub struct LogsQuery {
    pub limit: Option<usize>,
}

pub async fn get_logs(
    State(state): State<AppState>,
    Query(params): Query<LogsQuery>,
) -> Json<Value> {
    let limit = params.limit.unwrap_or(100).min(200);
    let buf = state.log_buffer.lock().unwrap();
    let logs: Vec<_> = buf.iter().rev().take(limit).collect();
    Json(json!({ "logs": logs, "total": buf.len() }))
}

/// GET /api/v1/irradiance/status
///
/// Retourne la dernière mesure du capteur d'irradiance PRALRAN.
pub async fn get_irradiance_status(State(state): State<AppState>) -> impl IntoResponse {
    match state.latest_irradiance().await {
        Some(snap) => (
            StatusCode::OK,
            Json(json!({
                "connected": true,
                "address": format!("{:#04x}", snap.address),
                "name": snap.name,
                "irradiance_wm2": snap.irradiance_wm2,
                "timestamp": snap.timestamp.to_rfc3339(),
                "total_yield_kwh": *state.mppt_yield_kwh.read().await,
                "mppt_power_w":    *state.mppt_power_w.read().await,
            })),
        ),
        None => (
            StatusCode::OK,
            Json(json!({
                "connected": false,
                "irradiance_wm2": 0.0,
                "total_yield_kwh": *state.mppt_yield_kwh.read().await,
                "mppt_power_w":    *state.mppt_power_w.read().await,
            })),
        ),
    }
}

/// Corps de la requête POST /api/v1/solar/mppt-yield
#[derive(Deserialize, Serialize)]
pub struct MpptYieldBody {
    /// Production solaire totale aujourd'hui en kWh (MPPT + ET112 delta).
    pub total_yield_kwh: Option<f32>,
    /// Rétrocompat ancien nom de champ.
    pub mppt_yield_kwh:  Option<f32>,
    /// Puissance MPPT instantanée totale en W (somme de tous les chargeurs).
    pub mppt_power_w:    Option<f32>,
}

/// POST /api/v1/solar/mppt-yield
///
/// Permet à Node-RED de pousser la production solaire totale journalière
/// et la puissance MPPT instantanée.
pub async fn set_mppt_yield(
    State(state): State<AppState>,
    Json(body): Json<MpptYieldBody>,
) -> impl IntoResponse {
    let kwh = body.total_yield_kwh.or(body.mppt_yield_kwh).unwrap_or(0.0);
    let pw  = body.mppt_power_w.unwrap_or(0.0);
    *state.mppt_yield_kwh.write().await = kwh;
    *state.mppt_power_w.write().await   = pw;
    (StatusCode::OK, Json(json!({ "ok": true, "total_yield_kwh": kwh, "mppt_power_w": pw })))
}

/// GET /api/v1/discover
///
/// Lance une découverte live sur le bus RS485.
/// ⚠️ Bloquant pendant la durée du scan (peut prendre plusieurs secondes).
pub async fn discover(State(_state): State<AppState>) -> impl IntoResponse {
    // TODO: Phase 2 — utiliser DalyBusManager::discover()
    (
        StatusCode::NOT_IMPLEMENTED,
        Json(json!({
            "error": "Découverte non encore implémentée (Phase 2)",
        })),
    )
}
