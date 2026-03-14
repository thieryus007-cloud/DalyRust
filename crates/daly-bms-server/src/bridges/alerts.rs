//! Moteur d'alertes avec hysteresis, journal SQLite, notifications Telegram/SMTP.
//!
//! ## Architecture
//!
//! - [`AlertEngine`] évalue chaque snapshot contre les règles configurées.
//! - Chaque règle a un seuil de déclenchement et un seuil d'effacement (hysteresis).
//! - Les événements sont journalisés dans SQLite.
//! - Les notifications sont envoyées par Telegram et/ou SMTP avec cooldown.

use crate::config::AlertsConfig;
use crate::state::AppState;
use daly_bms_core::types::BmsSnapshot;
use rusqlite::{Connection, params};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tracing::{error, info, warn};

// =============================================================================
// Règles d'alerte
// =============================================================================

/// Sévérité d'une alerte.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Warning,
    Critical,
}

impl Severity {
    fn icon(&self) -> &'static str {
        match self {
            Self::Warning  => "⚠️",
            Self::Critical => "🔴",
        }
    }
}

/// État d'une règle pour un BMS donné.
#[derive(Debug, Clone)]
struct RuleState {
    active: bool,
    last_notified: Option<Instant>,
}

impl Default for RuleState {
    fn default() -> Self { Self { active: false, last_notified: None } }
}

/// Contexte d'évaluation d'une règle.
pub struct AlertContext<'a> {
    pub snap: &'a BmsSnapshot,
    pub cfg:  &'a AlertsConfig,
}

/// Définition d'une règle d'alerte.
pub struct AlertRule {
    pub id:          &'static str,
    pub description: &'static str,
    pub severity:    Severity,
    pub cooldown:    Duration,
    pub trigger:     Box<dyn Fn(&AlertContext) -> Option<f32> + Send + Sync>,
    pub clear:       Box<dyn Fn(&AlertContext) -> bool + Send + Sync>,
}

// =============================================================================
// AlertEngine
// =============================================================================

pub struct AlertEngine {
    rules:  Vec<AlertRule>,
    states: Mutex<HashMap<(u8, &'static str), RuleState>>,
    db:     Mutex<Connection>,
    cfg:    AlertsConfig,
}

impl AlertEngine {
    /// Crée le moteur d'alertes et initialise la base SQLite.
    pub fn new(cfg: AlertsConfig) -> anyhow::Result<Arc<Self>> {
        let db = Connection::open(&cfg.db_path)?;
        init_db(&db)?;

        let engine = Arc::new(Self {
            rules:  build_rules(),
            states: Mutex::new(HashMap::new()),
            db:     Mutex::new(db),
            cfg,
        });
        Ok(engine)
    }

    /// Évalue toutes les règles sur un snapshot et envoie les notifications.
    pub async fn evaluate(&self, snap: &BmsSnapshot) {
        let ctx = AlertContext { snap, cfg: &self.cfg };

        for rule in &self.rules {
            let key = (snap.address, rule.id);
            let mut states = self.states.lock().unwrap();
            let state = states.entry(key).or_default();

            if !state.active {
                // Vérifier déclenchement
                if let Some(value) = (rule.trigger)(&ctx) {
                    state.active = true;
                    let should_notify = state.last_notified
                        .map(|t| t.elapsed() >= rule.cooldown)
                        .unwrap_or(true);

                    if should_notify {
                        state.last_notified = Some(Instant::now());
                        drop(states);
                        self.log_event(snap.address, rule.id, "triggered", value);
                        self.send_notification(snap.address, rule, value, true).await;
                    }
                }
            } else {
                // Vérifier effacement
                if (rule.clear)(&ctx) {
                    state.active = false;
                    drop(states);
                    self.log_event(snap.address, rule.id, "cleared", 0.0);
                    self.send_notification(snap.address, rule, 0.0, false).await;
                }
            }
        }
    }

    fn log_event(&self, addr: u8, rule_id: &str, event: &str, value: f32) {
        if let Ok(db) = self.db.lock() {
            let _ = db.execute(
                "INSERT INTO alert_events (bms_address, rule_id, event, value, timestamp) VALUES (?1, ?2, ?3, ?4, datetime('now'))",
                params![addr, rule_id, event, value],
            );
        }
    }

    async fn send_notification(&self, addr: u8, rule: &AlertRule, value: f32, triggered: bool) {
        let action = if triggered { "DÉCLENCHÉE" } else { "EFFACÉE" };
        let msg = format!(
            "{} Alerte {} — BMS {:#04x}\nRègle : {}\nValeur : {:.2}\nÉtat : {}",
            rule.severity.icon(),
            action,
            addr,
            rule.description,
            value,
            action,
        );

        info!("{}", msg);

        // Telegram
        if !self.cfg.telegram_token.is_empty() && !self.cfg.telegram_chat_id.is_empty() {
            if let Err(e) = send_telegram(&self.cfg.telegram_token, &self.cfg.telegram_chat_id, &msg).await {
                error!("Telegram erreur : {:?}", e);
            }
        }
    }
}

// =============================================================================
// Règles par défaut
// =============================================================================

fn build_rules() -> Vec<AlertRule> {
    vec![
        AlertRule {
            id: "cell_ovp",
            description: "Sur-tension cellule",
            severity: Severity::Critical,
            cooldown: Duration::from_secs(300),
            trigger: Box::new(|ctx| {
                let v = ctx.snap.system.max_cell_voltage;
                if v > ctx.cfg.thresholds.cell_ovp_v { Some(v) } else { None }
            }),
            clear: Box::new(|ctx| {
                ctx.snap.system.max_cell_voltage < ctx.cfg.thresholds.cell_ovp_v - 0.05
            }),
        },
        AlertRule {
            id: "cell_uvp",
            description: "Sous-tension cellule",
            severity: Severity::Critical,
            cooldown: Duration::from_secs(300),
            trigger: Box::new(|ctx| {
                let v = ctx.snap.system.min_cell_voltage;
                if v < ctx.cfg.thresholds.cell_uvp_v { Some(v) } else { None }
            }),
            clear: Box::new(|ctx| {
                ctx.snap.system.min_cell_voltage > ctx.cfg.thresholds.cell_uvp_v + 0.05
            }),
        },
        AlertRule {
            id: "cell_imbalance",
            description: "Déséquilibre cellules",
            severity: Severity::Warning,
            cooldown: Duration::from_secs(600),
            trigger: Box::new(|ctx| {
                let d = ctx.snap.system.cell_delta_mv();
                if d > ctx.cfg.thresholds.cell_delta_mv { Some(d) } else { None }
            }),
            clear: Box::new(|ctx| {
                ctx.snap.system.cell_delta_mv() < ctx.cfg.thresholds.cell_delta_mv - 10.0
            }),
        },
        AlertRule {
            id: "soc_low",
            description: "SOC bas",
            severity: Severity::Warning,
            cooldown: Duration::from_secs(900),
            trigger: Box::new(|ctx| {
                let s = ctx.snap.soc;
                if s < ctx.cfg.thresholds.soc_low_percent { Some(s) } else { None }
            }),
            clear: Box::new(|ctx| {
                ctx.snap.soc > ctx.cfg.thresholds.soc_low_percent + 5.0
            }),
        },
        AlertRule {
            id: "soc_critical",
            description: "SOC critique",
            severity: Severity::Critical,
            cooldown: Duration::from_secs(300),
            trigger: Box::new(|ctx| {
                let s = ctx.snap.soc;
                if s < ctx.cfg.thresholds.soc_critical_percent { Some(s) } else { None }
            }),
            clear: Box::new(|ctx| {
                ctx.snap.soc > ctx.cfg.thresholds.soc_critical_percent + 2.0
            }),
        },
        AlertRule {
            id: "temp_high",
            description: "Sur-température",
            severity: Severity::Critical,
            cooldown: Duration::from_secs(300),
            trigger: Box::new(|ctx| {
                let t = ctx.snap.system.max_cell_temperature;
                if t > ctx.cfg.thresholds.temp_high_c { Some(t) } else { None }
            }),
            clear: Box::new(|ctx| {
                ctx.snap.system.max_cell_temperature < ctx.cfg.thresholds.temp_high_c - 2.0
            }),
        },
        AlertRule {
            id: "high_current",
            description: "Sur-courant",
            severity: Severity::Warning,
            cooldown: Duration::from_secs(60),
            trigger: Box::new(|ctx| {
                let c = ctx.snap.dc.current.abs();
                if c > ctx.cfg.thresholds.current_high_a { Some(c) } else { None }
            }),
            clear: Box::new(|ctx| {
                ctx.snap.dc.current.abs() < ctx.cfg.thresholds.current_high_a - 5.0
            }),
        },
    ]
}

// =============================================================================
// Notifications
// =============================================================================

async fn send_telegram(token: &str, chat_id: &str, message: &str) -> anyhow::Result<()> {
    let url = format!("https://api.telegram.org/bot{}/sendMessage", token);
    let client = reqwest::Client::new();
    client
        .post(&url)
        .json(&serde_json::json!({
            "chat_id": chat_id,
            "text":    message,
            "parse_mode": "HTML",
        }))
        .send()
        .await?
        .error_for_status()?;
    Ok(())
}

// =============================================================================
// Initialisation SQLite
// =============================================================================

fn init_db(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch("
        CREATE TABLE IF NOT EXISTS alert_events (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            bms_address INTEGER NOT NULL,
            rule_id     TEXT NOT NULL,
            event       TEXT NOT NULL,
            value       REAL NOT NULL,
            timestamp   TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_alert_ts
            ON alert_events(timestamp);
        CREATE INDEX IF NOT EXISTS idx_alert_bms_rule
            ON alert_events(bms_address, rule_id);
    ")
}

// =============================================================================
// Tâche principale
// =============================================================================

/// Démarre le moteur d'alertes en arrière-plan.
pub async fn run_alert_engine(state: AppState, cfg: AlertsConfig) {
    if cfg.db_path.is_empty() {
        info!("AlertEngine : db_path vide, alertes désactivées");
        return;
    }

    let engine = match AlertEngine::new(cfg.clone()) {
        Ok(e) => e,
        Err(e) => {
            error!("AlertEngine init erreur : {:?}", e);
            return;
        }
    };

    info!(db = %cfg.db_path, "AlertEngine démarré");

    let mut rx = state.subscribe_ws();
    loop {
        match rx.recv().await {
            Ok(snaps) => {
                for snap in snaps.iter() {
                    engine.evaluate(snap).await;
                }
            }
            Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                warn!("AlertEngine : {} snapshots manqués", n);
            }
            Err(_) => break,
        }
    }
}
