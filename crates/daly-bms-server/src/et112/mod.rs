//! Module ET112 — Carlo Gavazzi ET112 compteur monophasé RS485/Modbus RTU.

pub mod poll;
pub mod types;

pub use poll::run_et112_poll_loop;
pub use types::Et112Snapshot;
