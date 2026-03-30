//! Router Axum — API REST + WebSocket
//!
//! Toutes les routes sont définies ici et réparties dans les sous-modules.

pub mod system;
pub mod bms;
pub mod et112;
pub mod tasmota;
pub mod chart;

use crate::dashboard;
use crate::state::AppState;
use axum::{
    Router,
    routing::{get, post},
};
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

/// Construit le router principal de l'application.
pub fn build_router(state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        // ── Dashboard HTML ────────────────────────────────────────────────────
        .merge(dashboard::build_dashboard_router())

        // ── Système ─────────────────────────────────────────────────────────
        .route("/api/v1/system/status",  get(system::get_status))
        .route("/api/v1/system/logs",    get(system::get_logs))
        .route("/api/v1/config",         get(system::get_config))
        .route("/api/v1/discover",       get(system::discover))
        .route("/api/v1/irradiance/status", get(system::get_irradiance_status))
        .route("/api/v1/solar/mppt-yield",  post(system::set_mppt_yield))

        // ── BMS — Lecture ────────────────────────────────────────────────────
        .route("/api/v1/bms/:id/status",      get(bms::get_bms_status))
        .route("/api/v1/bms/:id/cells",       get(bms::get_cells))
        .route("/api/v1/bms/:id/temperatures",get(bms::get_temperatures))
        .route("/api/v1/bms/:id/alarms",      get(bms::get_alarms))
        .route("/api/v1/bms/:id/mos",         get(bms::get_mos))
        .route("/api/v1/bms/:id/history",     get(bms::get_history))
        .route("/api/v1/bms/:id/history/summary", get(bms::get_history_summary))
        .route("/api/v1/bms/:id/export/csv",  get(bms::export_csv))
        .route("/api/v1/bms/compare",         get(bms::compare_all))

        // ── BMS — Paramètres (lecture à la demande) ───────────────────────────
        .route("/api/v1/bms/:id/settings",                         get(bms::get_settings))

        // ── BMS — Écriture ────────────────────────────────────────────────────
        .route("/api/v1/bms/:id/mos",                              post(bms::set_mos))
        .route("/api/v1/bms/:id/soc",                              post(bms::set_soc))
        .route("/api/v1/bms/:id/soc/full",                         post(bms::set_soc_full))
        .route("/api/v1/bms/:id/soc/empty",                        post(bms::set_soc_empty))
        .route("/api/v1/bms/:id/reset",                            post(bms::reset_bms))
        .route("/api/v1/bms/:id/settings/cell-voltage-alarms",     post(bms::set_cell_volt_alarms))
        .route("/api/v1/bms/:id/settings/pack-voltage-alarms",     post(bms::set_pack_volt_alarms))
        .route("/api/v1/bms/:id/settings/current-alarms",          post(bms::set_current_alarms))
        .route("/api/v1/bms/:id/settings/delta-alarms",            post(bms::set_delta_alarms))
        .route("/api/v1/bms/:id/settings/balancing",               post(bms::set_balancing))

        // ── ET112 ────────────────────────────────────────────────────────────
        .route("/api/v1/et112",                   get(et112::list_et112))
        .route("/api/v1/et112/:addr/status",      get(et112::get_et112_status))
        .route("/api/v1/et112/:addr/history",     get(et112::get_et112_history))

        // ── Chart historique InfluxDB ─────────────────────────────────────────
        .route("/api/v1/chart/history",           get(chart::get_chart_history))

        // ── Tasmota ──────────────────────────────────────────────────────────
        .route("/api/v1/tasmota",                 get(tasmota::list_tasmota))
        .route("/api/v1/tasmota/:id/status",      get(tasmota::get_tasmota_status))
        .route("/api/v1/tasmota/:id/history",     get(tasmota::get_tasmota_history))

        // ── WebSocket ─────────────────────────────────────────────────────────
        .route("/ws/bms/stream",         get(bms::ws_all))
        .route("/ws/bms/:id/stream",     get(bms::ws_single))

        // ── Middlewares ───────────────────────────────────────────────────────
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
