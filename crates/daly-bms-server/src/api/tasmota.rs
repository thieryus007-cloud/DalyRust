//! Endpoints REST pour les prises connectées Tasmota.
//!
//! Routes :
//! ```text
//! GET /api/v1/tasmota                  → liste des prises configurées + dernier snapshot
//! GET /api/v1/tasmota/:id/status       → dernier snapshot d'une prise
//! GET /api/v1/tasmota/:id/history      → historique (ring buffer)
//! ```

use crate::state::AppState;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;

/// GET /api/v1/tasmota — liste de toutes les prises + dernier snapshot
pub async fn list_tasmota(State(state): State<AppState>) -> Json<serde_json::Value> {
    let devices = &state.config.tasmota.devices;
    let mut result = Vec::new();
    for dev in devices {
        let snap = state.tasmota_latest_for(dev.id).await;
        result.push(serde_json::json!({
            "id":           dev.id,
            "name":         dev.name,
            "tasmota_id":   dev.tasmota_id,
            "mqtt_index":   dev.mqtt_index,
            "service_type": dev.service_type,
            "snapshot":     snap,
        }));
    }
    Json(serde_json::json!({ "tasmota": result }))
}

/// GET /api/v1/tasmota/:id/status — dernier snapshot
pub async fn get_tasmota_status(
    State(state): State<AppState>,
    Path(id_str): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let id   = id_str.trim().parse::<u8>().map_err(|_| StatusCode::BAD_REQUEST)?;
    let snap = state.tasmota_latest_for(id).await.ok_or(StatusCode::NOT_FOUND)?;
    Ok(Json(serde_json::to_value(&snap).unwrap_or_default()))
}

#[derive(Deserialize)]
pub struct HistoryParams {
    pub limit: Option<usize>,
}

/// GET /api/v1/tasmota/:id/history — historique complet
pub async fn get_tasmota_history(
    State(state): State<AppState>,
    Path(id_str): Path<String>,
    Query(params): Query<HistoryParams>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let id    = id_str.trim().parse::<u8>().map_err(|_| StatusCode::BAD_REQUEST)?;
    let limit = params.limit.unwrap_or(360).min(1440);
    let snaps = state.tasmota_history_for(id, limit).await;
    Ok(Json(serde_json::json!({ "id": id, "count": snaps.len(), "history": snaps })))
}
