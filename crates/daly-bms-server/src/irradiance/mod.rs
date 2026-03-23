//! Module irradiance — capteur PRALRAN RS485 (remplace irradiance_reader.py).

mod poll;
mod types;

pub use poll::run_irradiance_poll_loop;
pub use types::IrradianceSnapshot;
