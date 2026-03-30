//! Endpoint historique graphique — proxy InfluxDB pour le dashboard overview.
//!
//! GET /api/v1/chart/history?minutes=60
//! Retourne { solar:[{t,v}], soc:[{t,v}], load:[{t,v}] }

use axum::{
    extract::{Query, State},
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use crate::state::AppState;

#[derive(Deserialize)]
pub struct HistoryParams {
    pub minutes: Option<u32>,
}

/// GET /api/v1/chart/history?minutes=X
pub async fn get_chart_history(
    State(state): State<AppState>,
    Query(q): Query<HistoryParams>,
) -> impl IntoResponse {
    let minutes = q.minutes.unwrap_or(60).clamp(1, 720);
    let cfg = &state.config.influxdb;

    if !cfg.enabled || cfg.token.is_empty() {
        return Json(json!({"solar": [], "soc": [], "load": [], "ok": false}));
    }

    let window = if minutes <= 60 { "1m" } else if minutes <= 360 { "5m" } else { "10m" };
    let b = &cfg.bucket;

    let solar_q = format!(
        "from(bucket: \"{b}\") |> range(start: -{minutes}m) \
         |> filter(fn: (r) => r._measurement == \"solar_power\" and r._field == \"solar_total\") \
         |> aggregateWindow(every: {window}, fn: mean, createEmpty: false)"
    );

    let soc_q = format!(
        "from(bucket: \"{b}\") |> range(start: -{minutes}m) \
         |> filter(fn: (r) => r._measurement == \"bms_status\" and r._field == \"soc\") \
         |> aggregateWindow(every: {window}, fn: mean, createEmpty: false)"
    );

    let load_q = format!(
        "from(bucket: \"{b}\") |> range(start: -{minutes}m) \
         |> filter(fn: (r) => r._measurement == \"et112_status\" and r._field == \"power_w\") \
         |> aggregateWindow(every: {window}, fn: mean, createEmpty: false)"
    );

    let url  = format!("{}/api/v2/query?org={}", cfg.url, cfg.org);
    let auth = format!("Token {}", cfg.token);
    let client = reqwest::Client::new();

    let (solar_r, soc_r, load_r) = tokio::join!(
        influx_query(&client, &url, &auth, &solar_q),
        influx_query(&client, &url, &auth, &soc_q),
        influx_query(&client, &url, &auth, &load_q),
    );

    Json(json!({
        "ok":    true,
        "solar": solar_r.unwrap_or_default(),
        "soc":   soc_r.map(average_by_time).unwrap_or_default(),
        "load":  load_r.map(sum_by_time).unwrap_or_default(),
    }))
}

async fn influx_query(
    client: &reqwest::Client,
    url: &str,
    auth: &str,
    flux: &str,
) -> Option<Vec<(String, f64)>> {
    let resp = client
        .post(url)
        .header("Authorization", auth)
        .header("Content-Type", "application/vnd.flux")
        .header("Accept", "application/csv")
        .body(flux.to_string())
        .send()
        .await
        .ok()?;

    if !resp.status().is_success() {
        return None;
    }

    let csv = resp.text().await.ok()?;
    Some(parse_influx_csv(&csv))
}

fn parse_influx_csv(csv: &str) -> Vec<(String, f64)> {
    let mut result = Vec::new();
    let mut time_idx:  Option<usize> = None;
    let mut value_idx: Option<usize> = None;
    let mut in_header = false;

    for raw_line in csv.lines() {
        let line = raw_line.trim_end_matches('\r');

        if line.is_empty() || line.starts_with('#') {
            in_header = false;
            time_idx  = None;
            value_idx = None;
            continue;
        }

        let fields: Vec<&str> = line.split(',').collect();

        if !in_header {
            for (i, f) in fields.iter().enumerate() {
                match *f {
                    "_time"  => time_idx  = Some(i),
                    "_value" => value_idx = Some(i),
                    _ => {}
                }
            }
            in_header = true;
            continue;
        }

        if let (Some(ti), Some(vi)) = (time_idx, value_idx) {
            if let (Some(t), Some(v_str)) = (fields.get(ti), fields.get(vi)) {
                if let Ok(v) = v_str.parse::<f64>() {
                    // ISO 8601 → "HH:MM" (chars 11..16)
                    let t_fmt = if t.len() >= 16 { &t[11..16] } else { t };
                    result.push((t_fmt.to_string(), v));
                }
            }
        }
    }

    result
}

/// Moyenne des valeurs par timestamp (SOC multi-BMS → une seule série).
fn average_by_time(rows: Vec<(String, f64)>) -> Vec<Value> {
    let mut map: BTreeMap<String, (f64, usize)> = BTreeMap::new();
    for (t, v) in rows {
        let e = map.entry(t).or_insert((0.0, 0));
        e.0 += v;
        e.1 += 1;
    }
    map.into_iter()
        .map(|(t, (sum, n))| json!({"t": t, "v": (sum / n as f64).round()}))
        .collect()
}

/// Somme des valeurs par timestamp (charges ET112 multi-appareils).
fn sum_by_time(rows: Vec<(String, f64)>) -> Vec<Value> {
    let mut map: BTreeMap<String, f64> = BTreeMap::new();
    for (t, v) in rows {
        *map.entry(t).or_insert(0.0) += v;
    }
    map.into_iter()
        .map(|(t, v)| json!({"t": t, "v": v.round()}))
        .collect()
}
