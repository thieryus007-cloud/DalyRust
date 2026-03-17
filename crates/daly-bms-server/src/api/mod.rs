//! Router Axum — API REST + WebSocket
//!
//! Toutes les routes sont définies ici et réparties dans les sous-modules.

pub mod system;
pub mod bms;

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

        // ── BMS — Écriture ────────────────────────────────────────────────────
        .route("/api/v1/bms/:id/mos",         post(bms::set_mos))
        .route("/api/v1/bms/:id/soc",         post(bms::set_soc))
        .route("/api/v1/bms/:id/soc/full",    post(bms::set_soc_full))
        .route("/api/v1/bms/:id/soc/empty",   post(bms::set_soc_empty))
        .route("/api/v1/bms/:id/reset",       post(bms::reset_bms))

        // ── WebSocket ─────────────────────────────────────────────────────────
        .route("/ws/bms/stream",         get(bms::ws_all))
        .route("/ws/bms/:id/stream",     get(bms::ws_single))

        // ── Middlewares ───────────────────────────────────────────────────────
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
