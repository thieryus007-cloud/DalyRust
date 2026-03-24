//! Module Tasmota — prises connectées WiFi avec mesure d'énergie.
//!
//! Reçoit les payloads MQTT natifs Tasmota (tele/.../SENSOR, stat/.../POWER)
//! et les expose via l'API REST, le dashboard et les bridges InfluxDB/MQTT Venus.

pub mod types;
pub mod mqtt;

pub use types::TasmotaSnapshot;
pub use mqtt::run_tasmota_mqtt_loop;
