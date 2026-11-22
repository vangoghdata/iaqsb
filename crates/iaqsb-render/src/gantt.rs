//! Diagrama de Gantt (como GanttView.svelte).

use crate::*;

fn bar_color(t: &str) -> &'static str {
    match t {
        "muted" => "#e5e5ea",
        "" => SOLVER,
        t => tone(t),
    }
}

pub fn draw(g: &Value) -> Option<Canvas> {
    let rows = arr(g, "rows");
    if rows.is_empty() {
        return None;
    }
    let w = 820.0;
    let row_h = 30.0;
    let label_w = (rows.iter().map(|r| tw(s(r, "label"), 12.5)).fold(0.0, f64::max) + 24.0).clamp(90.0, 220.0);
    let markers = arr(g, "markers");
    let bars = || rows.iter().flat_map(|r| arr(r, "bars").iter());
    let tmax = bars().map(|b| f(b, "end")).chain(markers.iter().map(|m| f(m, "value"))).filter(|x| x.is_finite()).fold(1.0, f64::max);
    let tmin = bars().map(|b| f(b, "start")).filter(|x| x.is_finite()).fold(0.0, f64::min);
    let sx = |t: f64| label_w + (t - tmin) / (if tmax - tmin == 0.0 { 1.0 } else { tmax - tmin }) * (w - label_w - 24.0);
    // etiquetas de los marcadores en niveles para que no se pisen
    let mut placed: Vec<(f64, f64, usize)> = vec![];
    let mut mk = vec![];
    for m in markers {
        let lw = tw(s(m, "label"), 11.5) + 6.0;
        let x1 = sx(f(m, "value")) - 4.0;
        let mut lv = 0;
        while placed.iter().any(|p| p.2 == lv && !(x1 < p.0 || x1 - lw > p.1)) {
            lv += 1;
        }
        placed.push((x1 - lw, x1, lv));
        mk.push((m, x1, lv));
    }
    let levels = mk.iter().map(|m| m.2 + 1).max().unwrap_or(1).max(1) as f64;
    let top = 30.0 + rows.len() as f64 * row_h;
    let h = top + 10.0 + levels * 15.0 + 6.0;
    let mut cv = Canvas::new(w, h);
    for v in ticks(tmin, tmax, 10) {
        cv.line(sx(v), 26.0, sx(v), h - 24.0, RULE_SOFT, 1.0, "");
        cv.text(sx(v), 18.0, &num(v, 2), 11.0, INK3, "middle", "");
    }
    for (i, r) in rows.iter().enumerate() {
        let y = 30.0 + i as f64 * row_h;
        cv.text(label_w - 10.0, y + row_h / 2.0 + 4.0, s(r, "label"), 12.5, INK, "end", "");
        cv.line(label_w, y + row_h, w - 24.0, y + row_h, RULE_SOFT, 1.0, "");
        for b in arr(r, "bars") {
            let (a, e) = (sx(f(b, "start")), sx(f(b, "end")));
            let t = s(b, "tone");
            cv.push(format!(
                r#"<rect x="{}" y="{}" width="{}" height="{}" rx="4" fill="{}" fill-opacity="{}"/>"#,
                n(a),
                n(y + 6.0),
                n((e - a).max(2.0)),
                n(row_h - 12.0),
                bar_color(t),
                if t == "muted" { 0.9 } else { 0.85 }
            ));
            if let Some(l) = opt(b, "label") {
                if e - a > tw(l, 11.5) + 8.0 {
                    cv.text((a + e) / 2.0, y + row_h / 2.0 + 4.0, l, 11.5, if t == "muted" { INK2 } else { "white" }, "middle", r#"font-weight="600""#);
                }
            }
        }
    }
    for (m, x, lv) in &mk {
        let col = tone(if s(m, "tone").is_empty() { "highlight" } else { s(m, "tone") });
        cv.line(sx(f(m, "value")), 24.0, sx(f(m, "value")), top + 4.0, col, 1.4, r#"stroke-dasharray="4 3""#);
        cv.text(*x, top + 18.0 + *lv as f64 * 15.0, s(m, "label"), 11.5, col, "end", r#"font-weight="600""#);
    }
    cv.text(label_w - 10.0, 18.0, s(g, "unit"), 11.0, INK3, "end", "");
    for l in arr(g, "legend") {
        let t = l.get(0).and_then(|x| x.as_str()).unwrap_or("accent");
        let txt = l.get(1).and_then(|x| x.as_str()).unwrap_or("");
        cv.legend.push((bar_color(t).to_string(), txt.to_string(), false));
    }
    Some(cv)
}
