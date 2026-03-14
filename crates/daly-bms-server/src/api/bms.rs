//! Endpoints BMS : lecture, écriture, WebSocket, export CSV.

use crate::state::AppState;
use axum::{
    Json,
    extract::{Path, State, WebSocketUpgrade},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use axum::extract::ws::{Message, WebSocket};
use daly_bms_core::types::BmsSnapshot;
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;

// =============================================================================
// Helpers
// =============================================================================

/// Convertit une adresse hex ("0x01", "1", "01") en u8.
fn parse_addr(s: &str) -> Option<u8> {
    let s = s.trim();
    if s.starts_with("0x") || s.starts_with("0X") {
        u8::from_str_radix(&s[2..], 16).ok()
    } else {
        s.parse().ok()
    }
}

async fn require_bms(state: &AppState, id: &str) -> Result<BmsSnapshot, Response> {
    let addr = parse_addr(id).ok_or_else(|| {
        (StatusCode::BAD_REQUEST, Json(json!({"error": "Adresse BMS invalide"}))).into_response()
    })?;
    state.latest_for(addr).await.ok_or_else(|| {
        (StatusCode::NOT_FOUND, Json(json!({"error": "BMS non trouvé ou pas encore de données"}))).into_response()
    })
}

// =============================================================================
// GET — Lecture
// =============================================================================

/// GET /api/v1/bms/:id/status
pub async fn get_bms_status(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<BmsSnapshot>, Response> {
    let snap = require_bms(&state, &id).await?;
    Ok(Json(snap))
}

/// GET /api/v1/bms/:id/cells
pub async fn get_cells(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<Value>, Response> {
    let snap = require_bms(&state, &id).await?;
    let delta_mv = snap.system.cell_delta_mv();
    Ok(Json(json!({
        "bms": format!("{:#04x}", snap.address),
        "voltages": snap.voltages,
        "balances": snap.balances,
        "min": { "voltage": snap.system.min_cell_voltage, "cell": snap.system.min_voltage_cell_id },
        "max": { "voltage": snap.system.max_cell_voltage, "cell": snap.system.max_voltage_cell_id },
        "avg": snap.voltages.values().sum::<f32>() / snap.voltages.len().max(1) as f32,
        "delta_mv": delta_mv,
    })))
}

/// GET /api/v1/bms/:id/temperatures
pub async fn get_temperatures(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<Value>, Response> {
    let snap = require_bms(&state, &id).await?;
    Ok(Json(json!({
        "bms": format!("{:#04x}", snap.address),
        "min": { "temp": snap.system.min_cell_temperature, "sensor": snap.system.min_temperature_cell_id },
        "max": { "temp": snap.system.max_cell_temperature, "sensor": snap.system.max_temperature_cell_id },
        "mos_temperature": snap.system.mos_temperature,
    })))
}

/// GET /api/v1/bms/:id/alarms
pub async fn get_alarms(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<Value>, Response> {
    let snap = require_bms(&state, &id).await?;
    Ok(Json(json!({
        "bms": format!("{:#04x}", snap.address),
        "alarms": snap.alarms,
        "any_alarm": snap.alarms.any_active(),
    })))
}

/// GET /api/v1/bms/:id/mos
pub async fn get_mos(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<Value>, Response> {
    let snap = require_bms(&state, &id).await?;
    Ok(Json(json!({
        "bms": format!("{:#04x}", snap.address),
        "charge_mos": snap.io.allow_to_charge,
        "discharge_mos": snap.io.allow_to_discharge,
        "cycles": snap.history.charge_cycles,
        "soc": snap.soc,
    })))
}

/// GET /api/v1/bms/:id/history
pub async fn get_history(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<Value>, Response> {
    let addr = parse_addr(&id).ok_or_else(|| {
        (StatusCode::BAD_REQUEST, Json(json!({"error": "Adresse invalide"}))).into_response()
    })?;
    let history = state.history_for(addr, 3600).await;
    Ok(Json(json!({
        "bms": format!("{:#04x}", addr),
        "count": history.len(),
        "snapshots": history,
    })))
}

/// GET /api/v1/bms/:id/history/summary
pub async fn get_history_summary(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<Value>, Response> {
    let addr = parse_addr(&id).ok_or_else(|| {
        (StatusCode::BAD_REQUEST, Json(json!({"error": "Adresse invalide"}))).into_response()
    })?;
    let history = state.history_for(addr, 3600).await;

    if history.is_empty() {
        return Ok(Json(json!({"error": "Pas de données"})));
    }

    let soc_vals: Vec<f32> = history.iter().map(|s| s.soc).collect();
    let volt_vals: Vec<f32> = history.iter().map(|s| s.dc.voltage).collect();
    let curr_vals: Vec<f32> = history.iter().map(|s| s.dc.current).collect();
    let delta_vals: Vec<f32> = history.iter().map(|s| s.system.cell_delta_mv()).collect();

    Ok(Json(json!({
        "bms": format!("{:#04x}", addr),
        "count": history.len(),
        "soc":   { "min": min_f32(&soc_vals),  "max": max_f32(&soc_vals),  "avg": avg_f32(&soc_vals) },
        "voltage": { "min": min_f32(&volt_vals), "max": max_f32(&volt_vals), "avg": avg_f32(&volt_vals) },
        "current": { "min": min_f32(&curr_vals), "max": max_f32(&curr_vals), "avg": avg_f32(&curr_vals) },
        "cell_delta_mv": { "min": min_f32(&delta_vals), "max": max_f32(&delta_vals), "avg": avg_f32(&delta_vals) },
    })))
}

/// GET /api/v1/bms/:id/export/csv
pub async fn export_csv(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, Response> {
    let addr = parse_addr(&id).ok_or_else(|| {
        (StatusCode::BAD_REQUEST, Json(json!({"error": "Adresse invalide"}))).into_response()
    })?;
    let history = state.history_for(addr, 3600).await;

    let mut csv = String::from("timestamp,soc,voltage,current,power,min_cell_v,max_cell_v,delta_mv,temp_max\n");
    for snap in &history {
        csv.push_str(&format!(
            "{},{:.1},{:.2},{:.1},{:.1},{:.3},{:.3},{:.1},{:.1}\n",
            snap.timestamp.to_rfc3339(),
            snap.soc,
            snap.dc.voltage,
            snap.dc.current,
            snap.dc.power,
            snap.system.min_cell_voltage,
            snap.system.max_cell_voltage,
            snap.system.cell_delta_mv(),
            snap.system.max_cell_temperature,
        ));
    }

    Ok((
        [(axum::http::header::CONTENT_TYPE, "text/csv"),
         (axum::http::header::CONTENT_DISPOSITION,
          &format!("attachment; filename=\"bms_{:#04x}.csv\"", addr) as &str)],
        csv,
    ))
}

/// GET /api/v1/bms/compare
pub async fn compare_all(State(state): State<AppState>) -> Json<Value> {
    let latest = state.latest_snapshots().await;
    let comparison: Vec<Value> = latest.iter().map(|s| json!({
        "address": format!("{:#04x}", s.address),
        "soc": s.soc,
        "voltage": s.dc.voltage,
        "current": s.dc.current,
        "min_cell_v": s.system.min_cell_voltage,
        "max_cell_v": s.system.max_cell_voltage,
        "delta_mv": s.system.cell_delta_mv(),
        "temp_max": s.system.max_cell_temperature,
        "any_alarm": s.alarms.any_active(),
    })).collect();
    Json(json!({ "bms": comparison }))
}

// =============================================================================
// POST — Écriture
// =============================================================================

#[derive(Deserialize)]
pub struct MosCommand {
    pub charge: Option<bool>,
    pub discharge: Option<bool>,
}

/// POST /api/v1/bms/:id/mos
pub async fn set_mos(
    Path(_id): Path<String>,
    State(_state): State<AppState>,
    Json(_body): Json<MosCommand>,
) -> impl IntoResponse {
    // TODO: Phase 2 — appeler daly_bms_core::write::set_charge_mos / set_discharge_mos
    (StatusCode::NOT_IMPLEMENTED, Json(json!({"error": "Phase 2"})))
}

#[derive(Deserialize)]
pub struct SocCommand {
    pub soc: f32,
}

/// POST /api/v1/bms/:id/soc
pub async fn set_soc(
    Path(_id): Path<String>,
    State(_state): State<AppState>,
    Json(_body): Json<SocCommand>,
) -> impl IntoResponse {
    (StatusCode::NOT_IMPLEMENTED, Json(json!({"error": "Phase 2"})))
}

/// POST /api/v1/bms/:id/soc/full
pub async fn set_soc_full(
    Path(_id): Path<String>,
    State(_state): State<AppState>,
) -> impl IntoResponse {
    (StatusCode::NOT_IMPLEMENTED, Json(json!({"error": "Phase 2"})))
}

/// POST /api/v1/bms/:id/soc/empty
pub async fn set_soc_empty(
    Path(_id): Path<String>,
    State(_state): State<AppState>,
) -> impl IntoResponse {
    (StatusCode::NOT_IMPLEMENTED, Json(json!({"error": "Phase 2"})))
}

#[derive(Deserialize)]
pub struct ResetCommand {
    pub confirm: bool,
}

/// POST /api/v1/bms/:id/reset
pub async fn reset_bms(
    Path(_id): Path<String>,
    State(_state): State<AppState>,
    Json(body): Json<ResetCommand>,
) -> impl IntoResponse {
    if !body.confirm {
        return (StatusCode::BAD_REQUEST, Json(json!({"error": "confirm: true requis"}))).into_response();
    }
    (StatusCode::NOT_IMPLEMENTED, Json(json!({"error": "Phase 2"}))).into_response()
}

// =============================================================================
// WebSocket
// =============================================================================

/// GET /ws/bms/stream — stream de tous les BMS
pub async fn ws_all(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_ws_all(socket, state))
}

async fn handle_ws_all(socket: WebSocket, state: AppState) {
    let mut rx = state.subscribe_ws();
    let (mut sender, mut receiver) = socket.split();

    // Envoyer l'état actuel immédiatement
    let initial = state.latest_snapshots().await;
    if !initial.is_empty() {
        if let Ok(json) = serde_json::to_string(&initial) {
            let _ = sender.send(Message::Text(json)).await;
        }
    }

    loop {
        tokio::select! {
            Ok(snaps) = rx.recv() => {
                if let Ok(json) = serde_json::to_string(&*snaps) {
                    if sender.send(Message::Text(json)).await.is_err() {
                        break;
                    }
                }
            }
            Some(msg) = receiver.next() => {
                // Fermeture propre sur Close ou erreur
                match msg {
                    Ok(Message::Close(_)) | Err(_) => break,
                    _ => {}
                }
            }
        }
    }
}

/// GET /ws/bms/:id/stream — stream d'un seul BMS
pub async fn ws_single(
    Path(id): Path<String>,
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_ws_single(socket, state, id))
}

async fn handle_ws_single(socket: WebSocket, state: AppState, id: String) {
    let addr = match parse_addr(&id) {
        Some(a) => a,
        None => return,
    };

    let mut rx = state.subscribe_ws();
    let (mut sender, mut receiver) = socket.split();

    loop {
        tokio::select! {
            Ok(snaps) = rx.recv() => {
                // Filtrer pour ce BMS uniquement
                if let Some(snap) = snaps.iter().find(|s| s.address == addr) {
                    if let Ok(json) = serde_json::to_string(snap) {
                        if sender.send(Message::Text(json)).await.is_err() {
                            break;
                        }
                    }
                }
            }
            Some(msg) = receiver.next() => {
                match msg {
                    Ok(Message::Close(_)) | Err(_) => break,
                    _ => {}
                }
            }
        }
    }
}

// =============================================================================
// Utilitaires statistiques
// =============================================================================

fn min_f32(vals: &[f32]) -> f32 { vals.iter().cloned().fold(f32::INFINITY, f32::min) }
fn max_f32(vals: &[f32]) -> f32 { vals.iter().cloned().fold(f32::NEG_INFINITY, f32::max) }
fn avg_f32(vals: &[f32]) -> f32 {
    if vals.is_empty() { 0.0 } else { vals.iter().sum::<f32>() / vals.len() as f32 }
}
