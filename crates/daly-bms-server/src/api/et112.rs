//! Endpoints REST pour les compteurs Carlo Gavazzi ET112.
//!
//! Routes :
//! ```text
//! GET /api/v1/et112                     → liste des ET112 configurés + dernier snapshot
//! GET /api/v1/et112/:addr/status        → dernier snapshot d'un ET112
//! GET /api/v1/et112/:addr/history       → historique (ring buffer)
//! ```

use crate::state::AppState;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;

/// Parse une adresse Modbus depuis un segment de chemin ("3", "0x03").
fn parse_addr(s: &str) -> Option<u8> {
    let s = s.trim();
    if s.starts_with("0x") || s.starts_with("0X") {
        u8::from_str_radix(&s[2..], 16).ok()
    } else {
        s.parse::<u8>().ok()
    }
}

/// GET /api/v1/et112 — liste de tous les ET112 + dernier snapshot
pub async fn list_et112(State(state): State<AppState>) -> Json<serde_json::Value> {
    let devices = &state.config.et112.devices;
    let mut result = Vec::new();
    for dev in devices {
        let addr = dev.parsed_address();
        let snap = state.et112_latest_for(addr).await;
        let snap_with_connected = snap.map(|s| {
            let mut val = serde_json::to_value(&s).unwrap_or_default();
            if let Some(obj) = val.as_object_mut() {
                obj.insert("connected".to_string(), serde_json::json!(true));
            }
            val
        });
        result.push(serde_json::json!({
            "address":    addr,
            "name":       dev.name,
            "mqtt_index": dev.mqtt_index,
            "snapshot":   snap_with_connected,
        }));
    }
    Json(serde_json::json!({ "et112": result }))
}

/// GET /api/v1/et112/:addr/status — dernier snapshot
pub async fn get_et112_status(
    State(state): State<AppState>,
    Path(addr_str): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let addr = parse_addr(&addr_str).ok_or(StatusCode::BAD_REQUEST)?;
    let snap = state.et112_latest_for(addr).await.ok_or(StatusCode::NOT_FOUND)?;
    let mut val = serde_json::to_value(&snap).unwrap_or_default();
    // Ajouter le champ `connected` pour la visualization
    if let Some(obj) = val.as_object_mut() {
        obj.insert("connected".to_string(), serde_json::json!(true));
    }
    Ok(Json(val))
}

#[derive(Deserialize)]
pub struct HistoryParams {
    /// Nombre de points (défaut 360)
    pub limit: Option<usize>,
}

/// GET /api/v1/et112/:addr/history — historique complet
pub async fn get_et112_history(
    State(state): State<AppState>,
    Path(addr_str): Path<String>,
    Query(params): Query<HistoryParams>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let addr  = parse_addr(&addr_str).ok_or(StatusCode::BAD_REQUEST)?;
    let limit = params.limit.unwrap_or(360).min(1440);
    let snaps = state.et112_history_for(addr, limit).await;
    Ok(Json(serde_json::json!({ "address": addr, "count": snaps.len(), "history": snaps })))
}
