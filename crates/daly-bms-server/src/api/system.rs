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
                "mppt_yield_kwh": *state.mppt_yield_kwh.read().await,
            })),
        ),
        None => (
            StatusCode::OK,
            Json(json!({
                "connected": false,
                "irradiance_wm2": 0.0,
                "mppt_yield_kwh": *state.mppt_yield_kwh.read().await,
            })),
        ),
    }
}

/// Corps de la requête POST /api/v1/solar/mppt-yield
#[derive(Deserialize, Serialize)]
pub struct MpptYieldBody {
    /// Production MPPT aujourd'hui en kWh (publiée par Node-RED).
    pub mppt_yield_kwh: f32,
}

/// POST /api/v1/solar/mppt-yield
///
/// Permet à Node-RED de pousser la production MPPT journalière.
/// Node-RED appelle ce endpoint chaque fois que `mppt_yield_today` est mis à jour.
pub async fn set_mppt_yield(
    State(state): State<AppState>,
    Json(body): Json<MpptYieldBody>,
) -> impl IntoResponse {
    *state.mppt_yield_kwh.write().await = body.mppt_yield_kwh;
    (StatusCode::OK, Json(json!({ "ok": true, "mppt_yield_kwh": body.mppt_yield_kwh })))
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
