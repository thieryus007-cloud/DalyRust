//! # daly-bms-core
//!
//! Bibliothèque principale pour la communication avec les BMS Daly via UART/RS485.
//!
//! ## Modules
//! - [`error`]    — Types d'erreurs
//! - [`types`]    — Structures de données (snapshot, alarms, cells…)
//! - [`protocol`] — Format des trames, checksum, Data IDs
//! - [`bus`]      — Gestion du port série et du bus partagé (multi-BMS)
//! - [`commands`] — Commandes de lecture (0x90–0x98)
//! - [`write`]    — Commandes d'écriture (MOS, SOC, reset, config)
//! - [`poll`]     — Boucle de polling asynchrone avec backoff

pub mod error;
pub mod types;
pub mod protocol;
pub mod bus;
pub mod commands;
pub mod write;
pub mod poll;

// Re-exports pratiques pour les crates consommateurs
pub use error::DalyError;
pub use types::{
    BmsSnapshot, DcData, Alarms, InfoData, HistoryData, SystemData,
    IoData, CellVoltages, CellTemperatures, BmsAddress,
};
pub use protocol::{DataId, RequestFrame, ResponseFrame, FRAME_LEN};
pub use bus::{DalyPort, DalyBusManager};
pub use poll::PollConfig;
