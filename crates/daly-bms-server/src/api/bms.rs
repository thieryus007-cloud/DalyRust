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
use serde::Deserialize;
use serde_json::{json, Value};

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

    let content_disposition = format!("attachment; filename=\"bms_{:#04x}.csv\"", addr);
    Ok((
        [
            (axum::http::header::CONTENT_TYPE,       "text/csv".to_string()),
            (axum::http::header::CONTENT_DISPOSITION, content_disposition),
        ],
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

/// Mot de passe Daly requis pour les commandes d'écriture (identique à l'app Daly).
const DALY_WRITE_PASSWORD: &str = "12345678";

/// Extrait le port série depuis l'état. Retourne une erreur HTTP si indisponible.
async fn require_port(state: &AppState) -> Result<std::sync::Arc<daly_bms_core::bus::DalyPort>, Response> {
    let guard = state.port.read().await;
    match guard.as_ref() {
        Some(p) => Ok(p.clone()),
        None => Err((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({"error": "Port série non disponible (mode simulateur ou port non ouvert)"})),
        ).into_response()),
    }
}

#[derive(Deserialize)]
pub struct MosCommand {
    #[allow(dead_code)]
    pub charge: Option<bool>,
    #[allow(dead_code)]
    pub discharge: Option<bool>,
    #[allow(dead_code)]
    pub password: Option<String>,
}

/// POST /api/v1/bms/:id/mos — Non implémenté (hors scope interface web)
pub async fn set_mos(
    Path(_id): Path<String>,
    State(_state): State<AppState>,
    Json(_body): Json<MosCommand>,
) -> impl IntoResponse {
    (StatusCode::NOT_IMPLEMENTED, Json(json!({"error": "Commande MOS non exposée via l'interface web"})))
}

#[derive(Deserialize)]
pub struct SocCommand {
    pub soc: f32,
    pub password: String,
}

/// POST /api/v1/bms/:id/soc
///
/// Calibre le SOC du BMS à la valeur indiquée.
/// Body JSON : `{ "soc": 80.0, "password": "12345678" }`
pub async fn set_soc(
    Path(id): Path<String>,
    State(state): State<AppState>,
    Json(body): Json<SocCommand>,
) -> impl IntoResponse {
    if body.password != DALY_WRITE_PASSWORD {
        return (StatusCode::FORBIDDEN, Json(json!({"error": "Mot de passe incorrect"}))).into_response();
    }
    if !(0.0..=100.0).contains(&body.soc) {
        return (StatusCode::BAD_REQUEST, Json(json!({"error": "SOC hors plage [0, 100]"}))).into_response();
    }
    if state.config.read_only.enabled {
        return (StatusCode::FORBIDDEN, Json(json!({"error": "Mode lecture seule actif"}))).into_response();
    }
    let addr = match parse_addr(&id) {
        Some(a) => a,
        None => return (StatusCode::BAD_REQUEST, Json(json!({"error": "Adresse BMS invalide"}))).into_response(),
    };
    let port = match require_port(&state).await {
        Ok(p)  => p,
        Err(e) => return e,
    };
    match daly_bms_core::write::set_soc(&port, addr, body.soc, false).await {
        Ok(()) => (StatusCode::OK, Json(json!({
            "ok": true,
            "bms": format!("{:#04x}", addr),
            "soc": body.soc,
        }))).into_response(),
        Err(daly_bms_core::error::DalyError::Timeout { .. }) => {
            // Certains BMS n'envoient pas d'ACK après écriture SOC — comportement normal.
            (StatusCode::OK, Json(json!({
                "ok": true,
                "bms": format!("{:#04x}", addr),
                "soc": body.soc,
                "warning": "Pas d'ACK BMS (commande probablement reçue — vérifier le SOC dans 5s)",
            }))).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": format!("{:?}", e)}))).into_response(),
    }
}

// =============================================================================
// GET — Paramètres de configuration (lecture à la demande)
// =============================================================================

/// GET /api/v1/bms/:id/settings
///
/// Lit tous les paramètres de configuration du BMS (0x50, 0x5F, 0x59, 0x5A, 0x5B, 0x5E).
/// Cette commande interroge directement le BMS sur le bus RS485.
pub async fn get_settings(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let addr = match parse_addr(&id) {
        Some(a) => a,
        None => return (StatusCode::BAD_REQUEST, Json(json!({"error": "Adresse BMS invalide"}))).into_response(),
    };
    let port = match require_port(&state).await {
        Ok(p)  => p,
        Err(e) => return e,
    };
    // Retry jusqu'à 3 fois avec 300 ms de délai entre tentatives.
    // Nécessaire sur bus RS485 partagé : la première tentative peut tomber sur
    // un moment de contention avec le polling ou un autre BMS.
    let mut last_err = None;
    for attempt in 0u8..3 {
        match daly_bms_core::commands::get_bms_settings(&port, addr).await {
            Ok(s) => return (StatusCode::OK, Json(json!({
                "bms": format!("{:#04x}", addr),
                "settings": s,
            }))).into_response(),
            Err(e) => {
                if attempt < 2 {
                    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
                }
                last_err = Some(e);
            }
        }
    }
    let e = last_err.unwrap();
    (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": format!("{:?}", e)}))).into_response()
}

// =============================================================================
// POST — Écriture des paramètres de configuration
// =============================================================================

#[derive(Deserialize)]
pub struct CellVoltAlarmsCmd {
    pub high_l1_mv: u16,
    pub high_l2_mv: u16,
    pub low_l1_mv:  u16,
    pub low_l2_mv:  u16,
    pub password:   String,
}

/// POST /api/v1/bms/:id/settings/cell-voltage-alarms
pub async fn set_cell_volt_alarms(
    Path(id): Path<String>,
    State(state): State<AppState>,
    Json(body): Json<CellVoltAlarmsCmd>,
) -> impl IntoResponse {
    if body.password != DALY_WRITE_PASSWORD {
        return (StatusCode::FORBIDDEN, Json(json!({"error": "Mot de passe incorrect"}))).into_response();
    }
    if state.config.read_only.enabled {
        return (StatusCode::FORBIDDEN, Json(json!({"error": "Mode lecture seule actif"}))).into_response();
    }
    let addr = match parse_addr(&id) {
        Some(a) => a,
        None => return (StatusCode::BAD_REQUEST, Json(json!({"error": "Adresse invalide"}))).into_response(),
    };
    let port = match require_port(&state).await { Ok(p) => p, Err(e) => return e };
    match daly_bms_core::write::set_cell_volt_alarms(
        &port, addr, body.high_l1_mv, body.high_l2_mv, body.low_l1_mv, body.low_l2_mv, false,
    ).await {
        Ok(()) => (StatusCode::OK, Json(json!({"ok": true, "bms": format!("{:#04x}", addr)}))).into_response(),
        Err(daly_bms_core::error::DalyError::Timeout { .. }) =>
            (StatusCode::OK, Json(json!({"ok": true, "warning": "Pas d'ACK BMS"}))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": format!("{:?}", e)}))).into_response(),
    }
}

#[derive(Deserialize)]
pub struct PackVoltAlarmsCmd {
    pub high_l1_dv: u16,
    pub high_l2_dv: u16,
    pub low_l1_dv:  u16,
    pub low_l2_dv:  u16,
    pub password:   String,
}

/// POST /api/v1/bms/:id/settings/pack-voltage-alarms
pub async fn set_pack_volt_alarms(
    Path(id): Path<String>,
    State(state): State<AppState>,
    Json(body): Json<PackVoltAlarmsCmd>,
) -> impl IntoResponse {
    if body.password != DALY_WRITE_PASSWORD {
        return (StatusCode::FORBIDDEN, Json(json!({"error": "Mot de passe incorrect"}))).into_response();
    }
    if state.config.read_only.enabled {
        return (StatusCode::FORBIDDEN, Json(json!({"error": "Mode lecture seule actif"}))).into_response();
    }
    let addr = match parse_addr(&id) {
        Some(a) => a,
        None => return (StatusCode::BAD_REQUEST, Json(json!({"error": "Adresse invalide"}))).into_response(),
    };
    let port = match require_port(&state).await { Ok(p) => p, Err(e) => return e };
    match daly_bms_core::write::set_pack_volt_alarms(
        &port, addr, body.high_l1_dv, body.high_l2_dv, body.low_l1_dv, body.low_l2_dv, false,
    ).await {
        Ok(()) => (StatusCode::OK, Json(json!({"ok": true, "bms": format!("{:#04x}", addr)}))).into_response(),
        Err(daly_bms_core::error::DalyError::Timeout { .. }) =>
            (StatusCode::OK, Json(json!({"ok": true, "warning": "Pas d'ACK BMS"}))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": format!("{:?}", e)}))).into_response(),
    }
}

#[derive(Deserialize)]
pub struct CurrentAlarmsCmd {
    pub chg_l1_a: f32,
    pub chg_l2_a: f32,
    pub dch_l1_a: f32,
    pub dch_l2_a: f32,
    pub password: String,
}

/// POST /api/v1/bms/:id/settings/current-alarms
pub async fn set_current_alarms(
    Path(id): Path<String>,
    State(state): State<AppState>,
    Json(body): Json<CurrentAlarmsCmd>,
) -> impl IntoResponse {
    if body.password != DALY_WRITE_PASSWORD {
        return (StatusCode::FORBIDDEN, Json(json!({"error": "Mot de passe incorrect"}))).into_response();
    }
    if state.config.read_only.enabled {
        return (StatusCode::FORBIDDEN, Json(json!({"error": "Mode lecture seule actif"}))).into_response();
    }
    let addr = match parse_addr(&id) {
        Some(a) => a,
        None => return (StatusCode::BAD_REQUEST, Json(json!({"error": "Adresse invalide"}))).into_response(),
    };
    let port = match require_port(&state).await { Ok(p) => p, Err(e) => return e };
    match daly_bms_core::write::set_current_alarms(
        &port, addr, body.chg_l1_a, body.chg_l2_a, body.dch_l1_a, body.dch_l2_a, false,
    ).await {
        Ok(()) => (StatusCode::OK, Json(json!({"ok": true, "bms": format!("{:#04x}", addr)}))).into_response(),
        Err(daly_bms_core::error::DalyError::Timeout { .. }) =>
            (StatusCode::OK, Json(json!({"ok": true, "warning": "Pas d'ACK BMS"}))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": format!("{:?}", e)}))).into_response(),
    }
}

#[derive(Deserialize)]
pub struct DeltaAlarmsCmd {
    pub cell_delta_l1_mv: u16,
    pub cell_delta_l2_mv: u16,
    pub temp_delta_l1:    u8,
    pub temp_delta_l2:    u8,
    pub password:         String,
}

/// POST /api/v1/bms/:id/settings/delta-alarms
pub async fn set_delta_alarms(
    Path(id): Path<String>,
    State(state): State<AppState>,
    Json(body): Json<DeltaAlarmsCmd>,
) -> impl IntoResponse {
    if body.password != DALY_WRITE_PASSWORD {
        return (StatusCode::FORBIDDEN, Json(json!({"error": "Mot de passe incorrect"}))).into_response();
    }
    if state.config.read_only.enabled {
        return (StatusCode::FORBIDDEN, Json(json!({"error": "Mode lecture seule actif"}))).into_response();
    }
    let addr = match parse_addr(&id) {
        Some(a) => a,
        None => return (StatusCode::BAD_REQUEST, Json(json!({"error": "Adresse invalide"}))).into_response(),
    };
    let port = match require_port(&state).await { Ok(p) => p, Err(e) => return e };
    match daly_bms_core::write::set_delta_alarms(
        &port, addr, body.cell_delta_l1_mv, body.cell_delta_l2_mv, body.temp_delta_l1, body.temp_delta_l2, false,
    ).await {
        Ok(()) => (StatusCode::OK, Json(json!({"ok": true, "bms": format!("{:#04x}", addr)}))).into_response(),
        Err(daly_bms_core::error::DalyError::Timeout { .. }) =>
            (StatusCode::OK, Json(json!({"ok": true, "warning": "Pas d'ACK BMS"}))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": format!("{:?}", e)}))).into_response(),
    }
}

#[derive(Deserialize)]
pub struct BalancingCmd {
    pub activation_mv: u16,
    pub delta_mv:      u16,
    pub password:      String,
}

/// POST /api/v1/bms/:id/settings/balancing
pub async fn set_balancing(
    Path(id): Path<String>,
    State(state): State<AppState>,
    Json(body): Json<BalancingCmd>,
) -> impl IntoResponse {
    if body.password != DALY_WRITE_PASSWORD {
        return (StatusCode::FORBIDDEN, Json(json!({"error": "Mot de passe incorrect"}))).into_response();
    }
    if state.config.read_only.enabled {
        return (StatusCode::FORBIDDEN, Json(json!({"error": "Mode lecture seule actif"}))).into_response();
    }
    let addr = match parse_addr(&id) {
        Some(a) => a,
        None => return (StatusCode::BAD_REQUEST, Json(json!({"error": "Adresse invalide"}))).into_response(),
    };
    let port = match require_port(&state).await { Ok(p) => p, Err(e) => return e };
    match daly_bms_core::write::set_balancing_thresh(
        &port, addr, body.activation_mv, body.delta_mv, false,
    ).await {
        Ok(()) => (StatusCode::OK, Json(json!({"ok": true, "bms": format!("{:#04x}", addr)}))).into_response(),
        Err(daly_bms_core::error::DalyError::Timeout { .. }) =>
            (StatusCode::OK, Json(json!({"ok": true, "warning": "Pas d'ACK BMS"}))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": format!("{:?}", e)}))).into_response(),
    }
}

/// POST /api/v1/bms/:id/soc/full — SOC → 100%
pub async fn set_soc_full(
    Path(id): Path<String>,
    State(_state): State<AppState>,
) -> impl IntoResponse {
    (StatusCode::NOT_IMPLEMENTED, Json(json!({"error": "Utiliser POST /soc avec { soc: 100, password: ... }",
        "hint": format!("POST /api/v1/bms/{}/soc", id)
    })))
}

/// POST /api/v1/bms/:id/soc/empty — SOC → 0%
pub async fn set_soc_empty(
    Path(id): Path<String>,
    State(_state): State<AppState>,
) -> impl IntoResponse {
    (StatusCode::NOT_IMPLEMENTED, Json(json!({"error": "Utiliser POST /soc avec { soc: 0, password: ... }",
        "hint": format!("POST /api/v1/bms/{}/soc", id)
    })))
}

#[derive(Deserialize)]
pub struct ResetCommand {
    pub confirm: bool,
    pub password: String,
}

/// POST /api/v1/bms/:id/reset
///
/// Réinitialise le BMS. Nécessite `confirm: true` ET le mot de passe Daly.
/// Body JSON : `{ "confirm": true, "password": "12345678" }`
pub async fn reset_bms(
    Path(id): Path<String>,
    State(state): State<AppState>,
    Json(body): Json<ResetCommand>,
) -> impl IntoResponse {
    if !body.confirm {
        return (StatusCode::BAD_REQUEST, Json(json!({"error": "confirm: true requis"}))).into_response();
    }
    if body.password != DALY_WRITE_PASSWORD {
        return (StatusCode::FORBIDDEN, Json(json!({"error": "Mot de passe incorrect"}))).into_response();
    }
    if state.config.read_only.enabled {
        return (StatusCode::FORBIDDEN, Json(json!({"error": "Mode lecture seule actif"}))).into_response();
    }
    let addr = match parse_addr(&id) {
        Some(a) => a,
        None => return (StatusCode::BAD_REQUEST, Json(json!({"error": "Adresse BMS invalide"}))).into_response(),
    };
    let port = match require_port(&state).await {
        Ok(p)  => p,
        Err(e) => return e,
    };
    match daly_bms_core::write::reset_bms(&port, addr, false).await {
        Ok(()) => (StatusCode::OK, Json(json!({
            "ok": true,
            "bms": format!("{:#04x}", addr),
            "action": "reset",
        }))).into_response(),
        Err(daly_bms_core::error::DalyError::Timeout { .. }) => {
            // Le reset ne renvoie généralement pas d'ACK.
            (StatusCode::OK, Json(json!({
                "ok": true,
                "bms": format!("{:#04x}", addr),
                "action": "reset",
                "warning": "Pas d'ACK BMS (normal après un reset)",
            }))).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": format!("{:?}", e)}))).into_response(),
    }
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
