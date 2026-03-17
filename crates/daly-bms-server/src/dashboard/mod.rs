//! Dashboard web — serveur de pages HTML + assets.
//!
//! Stack : Axum (SSR) + Askama (templates compilés) + Apache ECharts (JS côté navigateur).
//! Aucune dépendance npm / React / Node.js — binaire unique auto-suffisant.
//!
//! Routes exposées :
//! - `GET /`                   → redirect vers /dashboard
//! - `GET /dashboard`          → vue d'ensemble de tous les BMS
//! - `GET /dashboard/bms/:id`  → détail complet d'un BMS

pub mod charts;

use crate::state::AppState;
use askama::Template;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{Html, IntoResponse, Redirect, Response},
    routing::get,
    Router,
};
use daly_bms_core::types::BmsSnapshot;
use std::sync::atomic::Ordering;
use tracing::error;

// =============================================================================
// Filtres Askama pour le formatage des nombres
// =============================================================================

mod filters {
    /// Formate un f32 avec 1 décimale : 52.1
    pub fn f1(v: &f32) -> ::askama::Result<String> {
        Ok(format!("{:.1}", v))
    }
    /// Formate un f32 sans décimale : 1234
    pub fn f0(v: &f32) -> ::askama::Result<String> {
        Ok(format!("{:.0}", v))
    }
    /// Formate un f32 avec 3 décimales : 3.405
    pub fn f3(v: &f32) -> ::askama::Result<String> {
        Ok(format!("{:.3}", v))
    }
    /// Formate un courant avec signe : +12.3 ou -8.5
    pub fn sign(v: &f32) -> ::askama::Result<String> {
        if *v >= 0.0 {
            Ok(format!("+{:.1}", v))
        } else {
            Ok(format!("{:.1}", v))
        }
    }
    /// Formate un f32 en millivolts (×1000, 0 décimales) : "23 mV"
    pub fn mv(v: &f32) -> ::askama::Result<String> {
        Ok(format!("{:.0}", v))
    }
    /// "s" si n ≠ 1, "" sinon
    pub fn pluralize(v: &usize) -> ::askama::Result<String> {
        Ok(if *v == 1 { String::new() } else { "s".to_string() })
    }
}

// =============================================================================
// Helpers de rendu
// =============================================================================

/// Rend un template Askama en réponse HTTP, ou 500 en cas d'erreur.
fn render<T: Template>(t: T) -> Response {
    match t.render() {
        Ok(html) => Html(html).into_response(),
        Err(e) => {
            error!("Template render error: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

/// Parse une adresse BMS depuis un segment de chemin ("1", "0x01", "01").
fn parse_addr(s: &str) -> Option<u8> {
    let s = s.trim();
    if s.starts_with("0x") || s.starts_with("0X") {
        u8::from_str_radix(&s[2..], 16).ok()
    } else {
        s.parse::<u8>().ok()
    }
}

// =============================================================================
// Structures de données pour les templates
// =============================================================================

/// Résumé d'une cellule individuelle pour la page d'accueil.
#[derive(Debug, Clone)]
pub struct CellInfo {
    pub num:    u16,
    pub v:      f32,
    pub is_min: bool,
    pub is_max: bool,
    pub is_bal: bool,
}

/// Résumé d'un BMS pour la carte de la page d'accueil.
#[derive(Debug, Clone)]
pub struct BmsSummary {
    pub address:               u8,
    pub address_hex:           String,   // "0x01"
    pub soc:                   f32,
    pub voltage:               f32,
    pub current:               f32,
    pub power:                 f32,
    pub temp_max:              f32,
    pub temp_min:              f32,
    pub cell_delta_mv:         f32,
    pub capacity_ah:           f32,
    pub bms_capacity_ah:       f32,
    pub installed_capacity:    f32,
    pub any_alarm:             bool,
    pub charge_ok:             bool,
    pub discharge_ok:          bool,
    pub last_update:           String,   // "HH:MM:SS"
    // Carte enrichie Node-RED style
    pub name:                  String,   // "BMS 320Ah"
    pub can_id:                String,   // "CAN02"
    pub cell_count:            u8,
    pub cells:                 Vec<CellInfo>,
    pub min_cell_id:           String,   // "C3"
    pub max_cell_id:           String,   // "C12"
    pub min_cell_v:            f32,
    pub max_cell_v:            f32,
    pub cycles:                u32,
    pub max_charge_current:    f32,
    pub max_discharge_current: f32,
    // Alarmes individuelles
    pub alarm_high_voltage:    bool,
    pub alarm_low_voltage:     bool,
    pub alarm_high_temp:       bool,
    pub alarm_imbalance:       bool,
    pub alarm_low_soc:         bool,
}

impl BmsSummary {
    fn from_snapshot(snap: &BmsSnapshot) -> Self {
        let delta = (snap.system.max_cell_voltage - snap.system.min_cell_voltage) * 1000.0;

        // Nom de la carte : depuis BmsConfig (via snap.name), sinon fallback capacité
        let name = if !snap.name.is_empty() {
            snap.name.clone()
        } else if snap.installed_capacity > 0.1 {
            format!("BMS-{:.0}Ah", snap.installed_capacity)
        } else {
            format!("BMS-{:#04x}", snap.address)
        };

        // Cellules triées numériquement avec flags MIN/MAX/balance
        let mut sorted: Vec<(&String, &f32)> = snap.voltages.iter().collect();
        sorted.sort_by_key(|(k, _)| k.trim_start_matches("Cell").parse::<u16>().unwrap_or(0));
        let cells: Vec<CellInfo> = sorted.iter().map(|&(k, &v)| {
            let num   = k.trim_start_matches("Cell").parse::<u16>().unwrap_or(0);
            let short = format!("C{}", num);
            CellInfo {
                num,
                v,
                is_min: short == snap.system.min_voltage_cell_id,
                is_max: short == snap.system.max_voltage_cell_id,
                is_bal: snap.balances.get(k).copied().unwrap_or(0) != 0,
            }
        }).collect();

        Self {
            address:               snap.address,
            address_hex:           format!("{:#04x}", snap.address),
            soc:                   snap.soc,
            voltage:               snap.dc.voltage,
            current:               snap.dc.current,
            power:                 snap.dc.power,
            temp_max:              snap.system.max_cell_temperature,
            temp_min:              snap.system.min_cell_temperature,
            cell_delta_mv:         delta,
            capacity_ah:           snap.capacity,
            bms_capacity_ah:       snap.bms_reported_capacity_ah,
            installed_capacity:    snap.installed_capacity,
            any_alarm:             snap.alarms.any_active(),
            charge_ok:             snap.io.allow_to_charge     != 0,
            discharge_ok:          snap.io.allow_to_discharge  != 0,
            last_update:           snap.timestamp.format("%H:%M:%S").to_string(),
            name,
            can_id:                format!("RS485-{}", snap.address),
            cell_count:            snap.system.nr_of_cells_per_battery,
            cells,
            min_cell_id:           snap.system.min_voltage_cell_id.clone(),
            max_cell_id:           snap.system.max_voltage_cell_id.clone(),
            min_cell_v:            snap.system.min_cell_voltage,
            max_cell_v:            snap.system.max_cell_voltage,
            cycles:                snap.history.charge_cycles,
            max_charge_current:    snap.info.max_charge_current,
            max_discharge_current: snap.info.max_discharge_current,
            alarm_high_voltage:    snap.alarms.high_voltage    != 0,
            alarm_low_voltage:     snap.alarms.low_voltage     != 0,
            alarm_high_temp:       snap.alarms.high_temperature != 0,
            alarm_imbalance:       snap.alarms.cell_imbalance  != 0,
            alarm_low_soc:         snap.alarms.low_soc         != 0,
        }
    }
}

/// Ligne d'alarme pour le tableau de la page détail.
#[derive(Debug, Clone)]
pub struct AlarmRow {
    pub name:   &'static str,
    pub active: bool,
}

fn build_alarms(snap: &BmsSnapshot) -> Vec<AlarmRow> {
    let a = &snap.alarms;
    vec![
        AlarmRow { name: "Sur-tension pack",          active: a.high_voltage != 0 },
        AlarmRow { name: "Sous-tension pack",          active: a.low_voltage  != 0 },
        AlarmRow { name: "Cellule sous-tension",       active: a.low_cell_voltage != 0 },
        AlarmRow { name: "SOC bas",                    active: a.low_soc != 0 },
        AlarmRow { name: "Sur-temp. charge",           active: a.high_charge_temperature != 0 },
        AlarmRow { name: "Sous-temp. charge",          active: a.low_charge_temperature  != 0 },
        AlarmRow { name: "Sur-température",            active: a.high_temperature != 0 },
        AlarmRow { name: "Sous-température",           active: a.low_temperature  != 0 },
        AlarmRow { name: "Sur-courant charge",         active: a.high_charge_current    != 0 },
        AlarmRow { name: "Sur-courant décharge",       active: a.high_discharge_current != 0 },
        AlarmRow { name: "Déséquilibre cellules",      active: a.cell_imbalance != 0 },
        AlarmRow { name: "Fusible grillé",             active: a.fuse_blown != 0 },
    ]
}

/// Détails complets pour la page d'un BMS.
pub struct BmsDetail {
    pub summary:            BmsSummary,
    // Infos cellules
    pub cell_count:         u8,
    pub min_cell_v:         f32,
    pub max_cell_v:         f32,
    pub min_cell_id:        String,
    pub max_cell_id:        String,
    // Infos état batterie
    pub soh:                f32,
    pub cycles:             u32,
    pub time_to_go_h:       f32,
    // Alarmes
    pub alarms:             Vec<AlarmRow>,
    // Options ECharts (JSON brut, injectés dans <script>)
    pub cells_bar_json:     String,
    pub cells_boxplot_json: String,
}

// =============================================================================
// Templates Askama
// =============================================================================

#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate {
    polling:   bool,
    bms_count: usize,
    bms_list:  Vec<BmsSummary>,
}

#[derive(Template)]
#[template(path = "bms_detail.html")]
struct DetailTemplate {
    detail: BmsDetail,
}

#[derive(Template)]
#[template(path = "logs.html")]
struct LogsTemplate {}

// =============================================================================
// Handlers Axum
// =============================================================================

/// Redirige `/` → `/dashboard`.
pub async fn redirect_root() -> Redirect {
    Redirect::temporary("/dashboard")
}

/// Page d'accueil — vue d'ensemble de tous les BMS.
pub async fn dashboard_index(State(state): State<AppState>) -> Response {
    let polling  = state.polling_active.load(Ordering::Relaxed);
    let snaps    = state.latest_snapshots().await;
    let bms_list = snaps.iter().map(BmsSummary::from_snapshot).collect();

    render(IndexTemplate { polling, bms_count: snaps.len(), bms_list })
}

/// Page de détail d'un BMS.
pub async fn dashboard_bms(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Response {
    let addr = match parse_addr(&id) {
        Some(a) => a,
        None    => return StatusCode::BAD_REQUEST.into_response(),
    };
    let snap = match state.latest_for(addr).await {
        Some(s) => s,
        None    => return (StatusCode::NOT_FOUND, "BMS non trouvé").into_response(),
    };

    // Historique : 300 derniers snapshots (≈ 5 min à 1 Hz), remis en ordre chronologique
    let mut history = state.history_for(addr, 300).await;
    history.reverse();

    let time_to_go_h = if snap.dc.current < -0.5 {
        snap.time_to_go as f32 / 3600.0
    } else { 0.0 };

    let detail = BmsDetail {
        summary:            BmsSummary::from_snapshot(&snap),
        cell_count:         snap.system.nr_of_cells_per_battery,
        min_cell_v:         snap.system.min_cell_voltage,
        max_cell_v:         snap.system.max_cell_voltage,
        min_cell_id:        snap.system.min_voltage_cell_id.clone(),
        max_cell_id:        snap.system.max_voltage_cell_id.clone(),
        soh:                snap.soh,
        cycles:             snap.history.charge_cycles,
        time_to_go_h,
        alarms:             build_alarms(&snap),
        cells_bar_json:     charts::cell_voltages_bar(
                                &snap.voltages,
                                &snap.system.min_voltage_cell_id,
                                &snap.system.max_voltage_cell_id,
                            ),
        cells_boxplot_json: charts::cell_boxplot(
                                &history,
                                &snap.system.min_voltage_cell_id,
                                &snap.system.max_voltage_cell_id,
                                &snap.balances,
                            ),
    };

    render(DetailTemplate { detail })
}

// =============================================================================
// Routeur du dashboard
// =============================================================================

/// Page des logs serveur.
pub async fn dashboard_logs() -> Response {
    render(LogsTemplate {})
}

/// Construit le routeur du dashboard (à fusionner dans le routeur principal).
pub fn build_dashboard_router() -> Router<AppState> {
    Router::new()
        .route("/",                  get(redirect_root))
        .route("/dashboard",         get(dashboard_index))
        .route("/dashboard/bms/:id", get(dashboard_bms))
        .route("/dashboard/logs",    get(dashboard_logs))
}
