//! Générateurs d'options ECharts côté serveur (Rust → JSON → template HTML).
//!
//! Chaque fonction retourne une `String` JSON représentant un objet `option`
//! ECharts complet, prêt à être injecté dans `echarts.setOption(...)`.
//!
//! Le rendu graphique est assuré par la bibliothèque ECharts (JS) côté navigateur ;
//! Rust ne fait que construire la configuration, sans aucune dépendance JS.

use daly_bms_core::types::BmsSnapshot;
use std::collections::BTreeMap;

// ─── Palette couleurs (light theme) ─────────────────────────────────────────
const C_BG:      &str = "transparent";
const C_MUTED:   &str = "#57606a";
const C_GRID:    &str = "#e8ecf0";
const C_AXIS:    &str = "#d0d7de";
const C_BLUE:    &str = "#0969da";
const C_GREEN:   &str = "#1a7f37";
const C_YELLOW:  &str = "#9a6700";
const C_RED:     &str = "#cf222e";

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
        surface = "#ffffff",
        text    = "#1f2328",
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
