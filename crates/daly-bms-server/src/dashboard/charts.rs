//! Générateurs d'options ECharts côté serveur (Rust → JSON → template HTML).
//!
//! Chaque fonction retourne une `String` JSON représentant un objet `option`
//! ECharts complet, prêt à être injecté dans `echarts.setOption(...)`.
//!
//! Le rendu graphique est assuré par la bibliothèque ECharts (JS) côté navigateur ;
//! Rust ne fait que construire la configuration, sans aucune dépendance JS.

use daly_bms_core::types::BmsSnapshot;
use std::collections::BTreeMap;

// ─── Palette couleurs (dark theme GitHub-like) ───────────────────────────────
const C_BG:      &str = "transparent";
const C_MUTED:   &str = "#8b949e";
const C_GRID:    &str = "#21262d";
const C_AXIS:    &str = "#30363d";
const C_BLUE:    &str = "#58a6ff";
const C_GREEN:   &str = "#3fb950";
const C_YELLOW:  &str = "#d29922";
const C_RED:     &str = "#f85149";
const C_ORANGE:  &str = "#fb8500";

// =============================================================================
// Jauge SOC (page d'accueil — mini, et détail — grande)
// =============================================================================

/// Génère l'option ECharts pour une jauge SOC.
/// `size` indique le style : "mini" pour les cartes, "full" pour le détail.
pub fn soc_gauge(soc: f32, size: &str) -> String {
    let font_size  = if size == "full" { 36 } else { 22 };
    let title_size = if size == "full" { 13  } else { 10 };
    let radius     = if size == "full" { "88%" } else { "85%" };
    let line_width = if size == "full" { 16 } else { 10 };

    let color = match soc as u32 {
        0..=14  => C_RED,
        15..=24 => C_ORANGE,
        25..=39 => C_YELLOW,
        _       => C_GREEN,
    };

    format!(r#"{{
  "backgroundColor": "{bg}",
  "series": [{{
    "type": "gauge",
    "startAngle": 205,
    "endAngle": -25,
    "min": 0,
    "max": 100,
    "splitNumber": 5,
    "radius": "{radius}",
    "center": ["50%", "55%"],
    "axisLine": {{
      "lineStyle": {{
        "width": {lw},
        "color": [
          [0.15, "{c_red}"],
          [0.25, "{c_orange}"],
          [0.40, "{c_yellow}"],
          [1.00, "{c_green}"]
        ]
      }}
    }},
    "pointer": {{
      "show": true,
      "length": "58%",
      "width": 4,
      "itemStyle": {{ "color": "auto" }}
    }},
    "axisTick":  {{ "show": false }},
    "splitLine": {{ "show": false }},
    "axisLabel": {{ "show": false }},
    "detail": {{
      "valueAnimation": true,
      "formatter": "{{value}}%",
      "color":     "{c_val}",
      "fontSize":  {fs},
      "fontWeight": "bold",
      "offsetCenter": [0, "20%"]
    }},
    "title": {{
      "color":        "{c_title}",
      "fontSize":     {ts},
      "offsetCenter": [0, "50%"]
    }},
    "data": [{{"value": {soc:.1}, "name": "SOC"}}]
  }}]
}}"#,
        bg      = C_BG,
        radius  = radius,
        lw      = line_width,
        c_red   = C_RED,
        c_orange= C_ORANGE,
        c_yellow= C_YELLOW,
        c_green = C_GREEN,
        c_val   = color,
        c_title = C_MUTED,
        fs      = font_size,
        ts      = title_size,
        soc     = soc,
    )
}

// =============================================================================
// Barres — tensions des cellules (amélioré)
// =============================================================================

/// Génère l'option ECharts pour le graphe de tensions des cellules.
///
/// - Cellule MIN : barre rouge + label "MIN"
/// - Cellule MAX : barre verte + label "MAX"
/// - Autres      : barre bleue
/// - Axe Y dynamique zoomé sur la plage réelle (±30 mV de marge)
/// - Ligne de moyenne (tirets jaunes)
/// - Titre "Δ = X mV" coloré selon sévérité
pub fn cell_voltages_bar(
    voltages: &BTreeMap<String, f32>,
    min_cell_id: &str,
    max_cell_id: &str,
) -> String {
    if voltages.is_empty() {
        return "{}".to_string();
    }

    let avg     = voltages.values().sum::<f32>() / voltages.len() as f32;
    let min_v   = voltages.values().cloned().fold(f32::INFINITY,     f32::min);
    let max_v   = voltages.values().cloned().fold(f32::NEG_INFINITY, f32::max);
    let delta_mv = (max_v - min_v) * 1000.0;

    // Zoom sur la plage réelle ±30 mV, clampé à [2.5 V, 4.3 V]
    let y_min = (min_v - 0.030).max(2.5);
    let y_max = (max_v + 0.030).min(4.3);

    let delta_color = if delta_mv > 100.0 { C_RED }
                      else if delta_mv > 50.0 { C_YELLOW }
                      else { C_GREEN };

    // Tri numérique (Cell1 < Cell2 < … < Cell16), pas alphabétique
    let mut sorted: Vec<(&String, &f32)> = voltages.iter().collect();
    sorted.sort_by_key(|(k, _)| k.trim_start_matches("Cell").parse::<u16>().unwrap_or(0));

    let labels: Vec<String> = sorted.iter()
        .map(|(k, _)| format!("\"C{}\"", k.trim_start_matches("Cell")))
        .collect();

    let values: Vec<String> = sorted.iter()
        .map(|(k, &v)| {
            // voltages keys are "Cell4", min_cell_id is "C4" — normalize before compare
            let short     = format!("C{}", k.trim_start_matches("Cell"));
            let is_min    = short == min_cell_id;
            let is_max    = short == max_cell_id;
            let color     = if is_min { C_RED } else if is_max { C_GREEN } else { C_BLUE };
            let label     = if is_min { "MIN" } else if is_max { "MAX" } else { "" };
            let show_lbl  = !label.is_empty();
            format!(
                r#"{{"value":{v:.4},"itemStyle":{{"color":"{c}","borderRadius":[3,3,0,0]}},"label":{{"show":{sl},"formatter":"{lbl}","position":"top","fontSize":8,"fontWeight":"bold","color":"{c}"}}}}"#,
                v   = v,
                c   = color,
                sl  = show_lbl,
                lbl = label,
            )
        })
        .collect();

    format!(r#"{{
  "backgroundColor": "{bg}",
  "animation": false,
  "title": {{
    "text": "\u0394 = {delta:.0} mV",
    "right": "1%",
    "top": "2%",
    "textStyle": {{ "color": "{dcol}", "fontSize": 11, "fontWeight": "bold" }}
  }},
  "tooltip": {{
    "trigger": "axis",
    "formatter": "{{b}}: {{c}} V",
    "borderColor": "{axis}",
    "textStyle": {{ "color": "{muted}", "fontSize": 11 }}
  }},
  "grid": {{ "left": "1%", "right": "5%", "top": "14%", "bottom": "12%", "containLabel": true }},
  "xAxis": {{
    "type":      "category",
    "data":      [{labels}],
    "axisLabel": {{ "color": "{muted}", "fontSize": 9 }},
    "axisLine":  {{ "lineStyle": {{ "color": "{axis}" }} }}
  }},
  "yAxis": {{
    "type":      "value",
    "min":       {y_min:.3},
    "max":       {y_max:.3},
    "splitNumber": 4,
    "axisLabel": {{ "color": "{muted}", "formatter": "{{value}} V", "fontSize": 9 }},
    "splitLine": {{ "lineStyle": {{ "color": "{grid}", "type": "dashed" }} }}
  }},
  "series": [{{
    "type": "bar",
    "data": [{values}],
    "barMaxWidth": 24,
    "markLine": {{
      "silent": true,
      "symbol": "none",
      "data": [{{
        "yAxis": {avg:.4},
        "lineStyle": {{ "color": "{yellow}", "type": "dashed", "width": 1.5 }},
        "label": {{
          "show": true,
          "formatter": "moy {avg:.3} V",
          "color": "{yellow}",
          "fontSize": 9,
          "position": "insideEndTop"
        }}
      }}]
    }}
  }}]
}}"#,
        bg     = C_BG,
        delta  = delta_mv,
        dcol   = delta_color,
        labels = labels.join(", "),
        values = values.join(", "),
        y_min  = y_min,
        y_max  = y_max,
        avg    = avg,
        muted  = C_MUTED,
        axis   = C_AXIS,
        grid   = C_GRID,
        yellow = C_YELLOW,
    )
}

// =============================================================================
// Aire — historique spread min/max des cellules
// =============================================================================

/// Génère l'option ECharts pour l'évolution du spread cellules dans le temps.
///
/// Deux courbes (min et max) avec aires superposées, permettant de voir
/// comment l'équilibrage évolue sur la session.
pub fn cell_spread_history(data: &HistoryData) -> String {
    if data.timestamps.is_empty() {
        return "{}".to_string();
    }

    let ts_json  = json_str_array(&data.timestamps);
    let min_json = json_f32_array_prec(&data.min_cell_v, 4);
    let max_json = json_f32_array_prec(&data.max_cell_v, 4);

    format!(r#"{{
  "backgroundColor": "{bg}",
  "animation": false,
  "legend": {{
    "data": ["Max cellule", "Min cellule"],
    "textStyle": {{ "color": "{muted}", "fontSize": 10 }},
    "top": 0,
    "right": 0
  }},
  "grid": {{ "left": "3%", "right": "2%", "top": "18%", "bottom": "18%", "containLabel": true }},
  "xAxis": {{
    "type":      "category",
    "data":      {ts},
    "axisLabel": {{ "color": "{muted}", "fontSize": 8, "rotate": 30, "interval": "auto" }},
    "axisLine":  {{ "lineStyle": {{ "color": "{axis}" }} }}
  }},
  "yAxis": {{
    "type":      "value",
    "scale":     true,
    "axisLabel": {{ "color": "{muted}", "formatter": "{{value}}V", "fontSize": 9 }},
    "splitLine": {{ "lineStyle": {{ "color": "{grid}", "type": "dashed" }} }}
  }},
  "series": [
    {{
      "name":   "Max cellule",
      "type":   "line",
      "data":   {max_v},
      "smooth": true,
      "symbol": "none",
      "lineStyle": {{ "color": "{green}", "width": 2 }},
      "areaStyle": {{
        "color": {{
          "type": "linear", "x": 0, "y": 0, "x2": 0, "y2": 1,
          "colorStops": [
            {{ "offset": 0, "color": "rgba(63,185,80,0.28)" }},
            {{ "offset": 1, "color": "rgba(63,185,80,0.05)" }}
          ]
        }}
      }}
    }},
    {{
      "name":   "Min cellule",
      "type":   "line",
      "data":   {min_v},
      "smooth": true,
      "symbol": "none",
      "lineStyle": {{ "color": "{red}", "width": 2 }},
      "areaStyle": {{
        "color": {{
          "type": "linear", "x": 0, "y": 0, "x2": 0, "y2": 1,
          "colorStops": [
            {{ "offset": 0, "color": "rgba(248,81,73,0.05)" }},
            {{ "offset": 1, "color": "rgba(248,81,73,0.28)" }}
          ]
        }}
      }}
    }}
  ],
  "dataZoom": [{{ "type": "inside" }}, {{ "type": "slider", "height": 16, "bottom": 0 }}]
}}"#,
        bg    = C_BG,
        ts    = ts_json,
        max_v = max_json,
        min_v = min_json,
        muted = C_MUTED,
        axis  = C_AXIS,
        grid  = C_GRID,
        green = C_GREEN,
        red   = C_RED,
    )
}

// =============================================================================
// Boxplot — distribution historique par cellule
// =============================================================================

/// Génère l'option ECharts boxplot montrant la distribution [min, Q1, médiane, Q3, max]
/// de la tension de chaque cellule sur l'ensemble des snapshots historiques.
///
/// - `min_cell_id` / `max_cell_id` : cellules MIN/MAX courantes (format "C4")
/// - `balances`                    : carte "CellN" → 0/1 indiquant l'équilibrage actif
///
/// Nécessite au moins 4 snapshots. Les cellules sont triées numériquement.
pub fn cell_boxplot(
    history:     &[BmsSnapshot],
    min_cell_id: &str,
    max_cell_id: &str,
    balances:    &BTreeMap<String, u8>,
) -> String {
    if history.len() < 4 {
        return "{}".to_string();
    }

    // Regrouper les tensions par cellule à travers tous les snapshots
    let mut per_cell: BTreeMap<String, Vec<f32>> = BTreeMap::new();
    for snap in history {
        for (k, &v) in &snap.voltages {
            per_cell.entry(k.clone()).or_default().push(v);
        }
    }
    if per_cell.is_empty() {
        return "{}".to_string();
    }

    // Tri numérique
    let mut cells: Vec<(String, Vec<f32>)> = per_cell.into_iter().collect();
    cells.sort_by_key(|(k, _)| k.trim_start_matches("Cell").parse::<u16>().unwrap_or(0));

    let mut labels          = Vec::new();
    let mut box_data        = Vec::new();
    let mut bal_scatter     = Vec::new();

    for (k, mut vals) in cells {
        if vals.is_empty() { continue; }
        vals.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let n  = vals.len();
        let mn = vals[0];
        let mx = vals[n - 1];
        let q1 = vals[n / 4];
        let md = vals[n / 2];
        let q3 = vals[(3 * n) / 4];

        // "Cell4" → "C4" pour comparer avec min_cell_id / max_cell_id
        let short  = format!("C{}", k.trim_start_matches("Cell"));
        let is_min = short == min_cell_id;
        let is_max = short == max_cell_id;
        let is_bal = balances.get(&k).copied().unwrap_or(0) != 0;

        let (fill, border) = if is_min {
            ("rgba(248,81,73,0.25)", C_RED)
        } else if is_max {
            ("rgba(63,185,80,0.25)", C_GREEN)
        } else {
            ("rgba(88,166,255,0.15)", C_BLUE)
        };

        // box data avec itemStyle individuel (format! regular — pas de raw string ici)
        box_data.push(format!(
            "{{\"value\":[{mn:.4},{q1:.4},{md:.4},{q3:.4},{mx:.4}],\"itemStyle\":{{\"color\":\"{fill}\",\"borderColor\":\"{border}\",\"borderWidth\":2}}}}",
            mn=mn, q1=q1, md=md, q3=q3, mx=mx, fill=fill, border=border
        ));

        // Point scatter pour les cellules en cours d'équilibrage
        if is_bal {
            let y = mx + (mx - mn) * 0.08 + 0.001;
            bal_scatter.push(format!("[\"{lbl}\",{y:.4}]", lbl=short, y=y));
        }

        labels.push(format!("\"{}\"", short));
    }

    // Série scatter (balance indicator) — toujours présente pour permettre setOption JS
    let scatter_json = format!(
        "{{\"type\":\"scatter\",\"data\":[{data}],\"symbol\":\"diamond\",\"symbolSize\":10,\"itemStyle\":{{\"color\":\"{yellow}\"}},\"z\":10,\"label\":{{\"show\":true,\"formatter\":\"\u{26a1}\",\"position\":\"top\",\"fontSize\":10,\"color\":\"{yellow}\"}}}}",
        data   = bal_scatter.join(","),
        yellow = C_YELLOW,
    );

    format!(r#"{{
  "backgroundColor": "{bg}",
  "animation": false,
  "title": {{
    "text": "Distribution par cellule ({n} snapshots)",
    "textStyle": {{ "color": "{muted}", "fontSize": 11, "fontWeight": "normal" }},
    "top": "2%"
  }},
  "tooltip": {{
    "trigger": "item",
    "backgroundColor": "{surface}",
    "borderColor": "{axis}",
    "textStyle": {{ "color": "{text}", "fontSize": 11 }}
  }},
  "grid": {{ "left": "1%", "right": "2%", "top": "14%", "bottom": "12%", "containLabel": true }},
  "xAxis": {{
    "type":      "category",
    "data":      [{labels}],
    "axisLabel": {{ "color": "{muted}", "fontSize": 9 }},
    "axisLine":  {{ "lineStyle": {{ "color": "{axis}" }} }}
  }},
  "yAxis": {{
    "type":      "value",
    "scale":     true,
    "axisLabel": {{ "color": "{muted}", "formatter": "{{value}} V", "fontSize": 9 }},
    "splitLine": {{ "lineStyle": {{ "color": "{grid}", "type": "dashed" }} }}
  }},
  "series": [
    {{
      "type":     "boxplot",
      "data":     [{data}],
      "boxWidth": ["20%", "45%"],
      "itemStyle": {{
        "color":       "rgba(88,166,255,0.15)",
        "borderColor": "{blue}",
        "borderWidth": 1.5
      }},
      "emphasis": {{
        "itemStyle": {{
          "color":       "rgba(88,166,255,0.30)",
          "borderColor": "{blue}",
          "borderWidth": 2
        }}
      }}
    }},
    {scatter}
  ]
}}"#,
        bg      = C_BG,
        surface = "#161b22",
        text    = "#e6edf3",
        n       = history.len(),
        labels  = labels.join(", "),
        data    = box_data.join(", "),
        scatter = scatter_json,
        muted   = C_MUTED,
        axis    = C_AXIS,
        grid    = C_GRID,
        blue    = C_BLUE,
    )
}

// =============================================================================
// Lignes — historique SOC + courant
// =============================================================================

/// Données d'historique extraites de la série de snapshots.
pub struct HistoryData {
    pub timestamps: Vec<String>,
    pub soc:        Vec<f32>,
    pub current:    Vec<f32>,
    pub voltage:    Vec<f32>,
    pub temp_max:   Vec<f32>,
    /// Tension de la cellule la plus faible à chaque instant
    pub min_cell_v: Vec<f32>,
    /// Tension de la cellule la plus élevée à chaque instant
    pub max_cell_v: Vec<f32>,
}

impl HistoryData {
    /// Construit depuis une liste de snapshots (ordre chronologique, du plus ancien au plus récent).
    pub fn from_snapshots(snaps: &[BmsSnapshot]) -> Self {
        let cap = snaps.len();
        let mut timestamps = Vec::with_capacity(cap);
        let mut soc        = Vec::with_capacity(cap);
        let mut current    = Vec::with_capacity(cap);
        let mut voltage    = Vec::with_capacity(cap);
        let mut temp_max   = Vec::with_capacity(cap);
        let mut min_cell_v = Vec::with_capacity(cap);
        let mut max_cell_v = Vec::with_capacity(cap);

        for s in snaps {
            timestamps.push(s.timestamp.format("%H:%M:%S").to_string());
            soc.push(s.soc);
            current.push(s.dc.current);
            voltage.push(s.dc.voltage);
            temp_max.push(s.system.max_cell_temperature);
            min_cell_v.push(s.system.min_cell_voltage);
            max_cell_v.push(s.system.max_cell_voltage);
        }
        Self { timestamps, soc, current, voltage, temp_max, min_cell_v, max_cell_v }
    }
}

/// Génère l'option ECharts pour l'historique SOC (line chart avec aire).
pub fn soc_history_line(data: &HistoryData) -> String {
    let ts_json  = json_str_array(&data.timestamps);
    let soc_json = json_f32_array(&data.soc);

    format!(r#"{{
  "backgroundColor": "{bg}",
  "animation": false,
  "grid": {{ "left": "3%", "right": "2%", "top": "8%", "bottom": "18%", "containLabel": true }},
  "xAxis": {{
    "type":      "category",
    "data":      {ts},
    "axisLabel": {{ "color": "{muted}", "fontSize": 8, "rotate": 30, "interval": "auto" }},
    "axisLine":  {{ "lineStyle": {{ "color": "{axis}" }} }}
  }},
  "yAxis": {{
    "type":      "value",
    "min":       0,
    "max":       100,
    "axisLabel": {{ "color": "{muted}", "formatter": "{{value}}%", "fontSize": 9 }},
    "splitLine": {{ "lineStyle": {{ "color": "{grid}", "type": "dashed" }} }}
  }},
  "series": [{{
    "type":   "line",
    "data":   {soc},
    "smooth": true,
    "symbol": "none",
    "lineStyle": {{ "color": "{green}", "width": 2 }},
    "areaStyle": {{
      "color": {{
        "type": "linear", "x": 0, "y": 0, "x2": 0, "y2": 1,
        "colorStops": [
          {{ "offset": 0, "color": "rgba(63,185,80,0.35)" }},
          {{ "offset": 1, "color": "rgba(63,185,80,0.02)" }}
        ]
      }}
    }}
  }}],
  "dataZoom": [{{ "type": "inside" }}, {{ "type": "slider", "height": 16, "bottom": 0 }}]
}}"#,
        bg    = C_BG,
        ts    = ts_json,
        soc   = soc_json,
        muted = C_MUTED,
        axis  = C_AXIS,
        grid  = C_GRID,
        green = C_GREEN,
    )
}

/// Génère l'option ECharts pour l'historique courant (+ charge, - décharge).
pub fn current_history_line(data: &HistoryData) -> String {
    let ts_json      = json_str_array(&data.timestamps);
    let current_json = json_f32_array(&data.current);

    format!(r#"{{
  "backgroundColor": "{bg}",
  "animation": false,
  "grid": {{ "left": "3%", "right": "2%", "top": "8%", "bottom": "18%", "containLabel": true }},
  "xAxis": {{
    "type":      "category",
    "data":      {ts},
    "axisLabel": {{ "color": "{muted}", "fontSize": 8, "rotate": 30, "interval": "auto" }},
    "axisLine":  {{ "lineStyle": {{ "color": "{axis}" }} }}
  }},
  "yAxis": {{
    "type":      "value",
    "axisLabel": {{ "color": "{muted}", "formatter": "{{value}}A", "fontSize": 9 }},
    "splitLine": {{ "lineStyle": {{ "color": "{grid}", "type": "dashed" }} }}
  }},
  "series": [{{
    "type":   "line",
    "data":   {cur},
    "smooth": true,
    "symbol": "none",
    "lineStyle": {{ "color": "{blue}", "width": 2 }},
    "markLine": {{
      "silent": true,
      "symbol": "none",
      "data": [{{ "yAxis": 0, "lineStyle": {{ "color": "{muted}", "type": "dashed" }} }}]
    }}
  }}],
  "dataZoom": [{{ "type": "inside" }}, {{ "type": "slider", "height": 16, "bottom": 0 }}]
}}"#,
        bg    = C_BG,
        ts    = ts_json,
        cur   = current_json,
        muted = C_MUTED,
        axis  = C_AXIS,
        grid  = C_GRID,
        blue  = C_BLUE,
    )
}

/// Génère l'option ECharts pour l'historique tension + température (double axe Y).
pub fn voltage_temp_line(data: &HistoryData) -> String {
    let ts_json   = json_str_array(&data.timestamps);
    let volt_json = json_f32_array(&data.voltage);
    let temp_json = json_f32_array(&data.temp_max);

    format!(r#"{{
  "backgroundColor": "{bg}",
  "animation": false,
  "legend": {{
    "data": ["Tension (V)", "Temp max (°C)"],
    "textStyle": {{ "color": "{muted}", "fontSize": 10 }},
    "top": 0
  }},
  "grid": {{ "left": "3%", "right": "5%", "top": "18%", "bottom": "18%", "containLabel": true }},
  "xAxis": {{
    "type":      "category",
    "data":      {ts},
    "axisLabel": {{ "color": "{muted}", "fontSize": 8, "rotate": 30, "interval": "auto" }},
    "axisLine":  {{ "lineStyle": {{ "color": "{axis}" }} }}
  }},
  "yAxis": [
    {{
      "type":      "value",
      "name":      "V",
      "nameTextStyle": {{ "color": "{muted}", "fontSize": 9 }},
      "axisLabel": {{ "color": "{muted}", "formatter": "{{value}}V", "fontSize": 9 }},
      "splitLine": {{ "lineStyle": {{ "color": "{grid}", "type": "dashed" }} }}
    }},
    {{
      "type":      "value",
      "name":      "°C",
      "nameTextStyle": {{ "color": "{muted}", "fontSize": 9 }},
      "axisLabel": {{ "color": "{muted}", "formatter": "{{value}}°C", "fontSize": 9 }},
      "splitLine": {{ "show": false }}
    }}
  ],
  "series": [
    {{
      "name":   "Tension (V)",
      "type":   "line",
      "yAxisIndex": 0,
      "data":   {volt},
      "smooth": true,
      "symbol": "none",
      "lineStyle": {{ "color": "{blue}", "width": 2 }}
    }},
    {{
      "name":   "Temp max (°C)",
      "type":   "line",
      "yAxisIndex": 1,
      "data":   {temp},
      "smooth": true,
      "symbol": "none",
      "lineStyle": {{ "color": "{orange}", "width": 2 }}
    }}
  ],
  "dataZoom": [{{ "type": "inside" }}, {{ "type": "slider", "height": 16, "bottom": 0 }}]
}}"#,
        bg     = C_BG,
        ts     = ts_json,
        volt   = volt_json,
        temp   = temp_json,
        muted  = C_MUTED,
        axis   = C_AXIS,
        grid   = C_GRID,
        blue   = C_BLUE,
        orange = C_ORANGE,
    )
}

// =============================================================================
// Utilitaires de sérialisation JSON
// =============================================================================

fn json_str_array(v: &[String]) -> String {
    let inner: Vec<String> = v.iter()
        .map(|s| format!("\"{}\"", s.replace('"', "\\\"")))
        .collect();
    format!("[{}]", inner.join(","))
}

fn json_f32_array(v: &[f32]) -> String {
    json_f32_array_prec(v, 3)
}

fn json_f32_array_prec(v: &[f32], prec: usize) -> String {
    let inner: Vec<String> = v.iter()
        .map(|f| format!("{:.prec$}", f, prec = prec))
        .collect();
    format!("[{}]", inner.join(","))
}
