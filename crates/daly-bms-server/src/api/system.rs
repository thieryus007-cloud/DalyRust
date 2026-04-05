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
    let kwh  = *state.mppt_yield_kwh.read().await;
    let mpw  = *state.mppt_power_w.read().await;
    let solw = *state.solar_total_w.read().await;
    let housew = *state.house_power_w.read().await;
    match state.latest_irradiance().await {
        Some(snap) => (
            StatusCode::OK,
            Json(json!({
                "connected": true,
                "address": format!("{:#04x}", snap.address),
                "name": snap.name,
                "irradiance_wm2": snap.irradiance_wm2,
                "timestamp": snap.timestamp.to_rfc3339(),
                "total_yield_kwh": kwh,
                "mppt_power_w":    mpw,
                "solar_total_w":   solw,
                "house_power_w":   housew,
            })),
        ),
        None => (
            StatusCode::OK,
            Json(json!({
                "connected": false,
                "irradiance_wm2": 0.0,
                "total_yield_kwh": kwh,
                "mppt_power_w":    mpw,
                "solar_total_w":   solw,
                "house_power_w":   housew,
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
    /// Puissance MPPT seule en W (273+289, sans ET112).
    pub mppt_power_w:    Option<f32>,
    /// Puissance solaire totale en W = MPPT + ET112 PVInverter (source VRM Node-RED).
    /// Champ canonique depuis Solar_power.json — source de vérité pour le dashboard.
    pub solar_total_w:   Option<f32>,
    /// Puissance maison en W = N/c0619ab9929a/system/0/Ac/ConsumptionOnOutput/L1/Power.
    pub house_power_w:   Option<f32>,
}

/// POST /api/v1/solar/mppt-yield
///
/// Mise à jour partielle : seuls les champs présents dans le body sont écrits.
/// Solar_power.json envoie solar_total_w + mppt_power_w.
/// meteo.json envoie total_yield_kwh + mppt_power_w (keepalive kWh).
pub async fn set_mppt_yield(
    State(state): State<AppState>,
    Json(body): Json<MpptYieldBody>,
) -> impl IntoResponse {
    if let Some(kwh) = body.total_yield_kwh.or(body.mppt_yield_kwh) {
        *state.mppt_yield_kwh.write().await = kwh;
    }
    if let Some(pw) = body.mppt_power_w {
        *state.mppt_power_w.write().await = pw;
    }
    if let Some(tw) = body.solar_total_w {
        *state.solar_total_w.write().await = tw;
    }
    if let Some(hw) = body.house_power_w {
        *state.house_power_w.write().await = hw;
    }
    let kwh  = *state.mppt_yield_kwh.read().await;
    let pw   = *state.mppt_power_w.read().await;
    let tw   = *state.solar_total_w.read().await;
    let housew = *state.house_power_w.read().await;
    (StatusCode::OK, Json(json!({
        "ok": true,
        "total_yield_kwh": kwh,
        "mppt_power_w":    pw,
        "solar_total_w":   tw,
        "house_power_w":   housew,
    })))
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

// =============================================================================
// Endpoints Venus OS — Données D-Bus via MQTT
// =============================================================================

/// GET /api/v1/venus/mppt
///
/// Retourne tous les MPPT SolarCharger actuels (depuis D-Bus Venus OS).
pub async fn get_venus_mppt(State(state): State<AppState>) -> impl IntoResponse {
    let mppts = state.venus_mppts_all().await;
    (StatusCode::OK, Json(json!({
        "count": mppts.len(),
        "mppts": mppts,
        "total_power_w": state.venus_mppt_total_power().await,
    })))
}

/// GET /api/v1/venus/smartshunt
///
/// Retourne le SmartShunt actuel (depuis D-Bus Venus OS).
pub async fn get_venus_smartshunt(State(state): State<AppState>) -> impl IntoResponse {
    match state.venus_smartshunt_get().await {
        Some(shunt) => (
            StatusCode::OK,
            Json(json!({
                "connected": true,
                "shunt": shunt,
            })),
        ),
        None => (
            StatusCode::OK,
            Json(json!({
                "connected": false,
                "shunt": Value::Null,
                "message": "SmartShunt non disponible ou non configuré",
            })),
        ),
    }
}

/// GET /api/v1/venus/temperatures
///
/// Retourne tous les capteurs de température actuels (depuis D-Bus Venus OS).
pub async fn get_venus_temperatures(State(state): State<AppState>) -> impl IntoResponse {
    let temps = state.venus_temperatures_all().await;
    (StatusCode::OK, Json(json!({
        "count": temps.len(),
        "temperatures": temps,
    })))
}

/// GET /api/v1/system/totals
///
/// Retourne les totaux agrégés du système :
/// - Puissance production (MPPT + PV Inverter)
/// - Puissance consommation (maison)
/// - Charge/décharge batteries
/// - SOC moyen batteries
#[derive(Serialize)]
pub struct SystemTotals {
    pub production_w: f32,           // MPPT + ET112 pvinverter
    pub consumption_w: f32,          // Maison
    pub batteries_power_w: f32,      // Charge (+) / Décharge (-)
    pub avg_soc_percent: f32,        // SOC moyen des batteries
    pub avg_voltage_v: f32,          // Tension moyenne batteries
    pub total_current_a: f32,        // Courant total batteries
    pub smartshunt_soc_percent: Option<f32>, // SmartShunt SOC si disponible
}

pub async fn get_system_totals(State(state): State<AppState>) -> impl IntoResponse {
    // Production : MPPT + PV Inverter (ET112 0x07)
    let mppt_power = state.venus_mppt_total_power().await;
    let pv_inverter_power = state
        .et112_latest_all()
        .await
        .iter()
        .find(|e| e.address == 0x07)
        .map(|e| e.power_w)
        .unwrap_or(0.0);
    let production_w = mppt_power + pv_inverter_power;

    // Consommation : maison
    let consumption_w = *state.house_power_w.read().await;

    // Batteries : SOC moyen et tension/courant total
    let bms_snapshots = state.latest_snapshots().await;
    let avg_soc_percent = if !bms_snapshots.is_empty() {
        bms_snapshots
            .iter()
            .map(|s| s.soc)
            .sum::<f32>() / bms_snapshots.len() as f32
    } else {
        0.0
    };

    let avg_voltage_v = if !bms_snapshots.is_empty() {
        bms_snapshots
            .iter()
            .map(|s| s.dc.voltage)
            .sum::<f32>() / bms_snapshots.len() as f32
    } else {
        0.0
    };

    let total_current_a: f32 = bms_snapshots
        .iter()
        .map(|s| s.dc.current)
        .sum();

    let batteries_power_w: f32 = bms_snapshots
        .iter()
        .map(|s| s.dc.power)
        .sum();

    let smartshunt_soc = state
        .venus_smartshunt_get()
        .await
        .and_then(|s| s.soc_percent);

    (
        StatusCode::OK,
        Json(SystemTotals {
            production_w,
            consumption_w,
            batteries_power_w,
            avg_soc_percent,
            avg_voltage_v,
            total_current_a,
            smartshunt_soc_percent: smartshunt_soc,
        }),
    )
}
