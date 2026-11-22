//! Gráfico de líneas, barras, dispersión, escalera y área (como ChartView.svelte).

use crate::*;
use std::collections::HashMap;


fn pt(p: &Value) -> (f64, f64) {
    let a = p.as_array();
    let g = |i: usize| a.and_then(|a| a.get(i)).and_then(|v| v.as_f64()).unwrap_or(f64::NAN);
    (g(0), g(1))
}

pub fn draw(c: &Value) -> Option<Canvas> {
    let series = arr(c, "series");
    if series.is_empty() {
        return None;
    }
    let (w, h) = (760.0, 360.0);
    let cats: Option<Vec<String>> = c.get("categories").and_then(|v| v.as_array()).map(|a| a.iter().map(|x| x.as_str().unwrap_or("").to_string()).collect());
    let stacked = b(c, "stacked");
    let hlines = arr(c, "hlines");
    let vlines = arr(c, "vlines");
    let fin = |p: (f64, f64)| p.0.is_finite() && p.1.is_finite();
    let pts: Vec<(f64, f64)> = series.iter().flat_map(|sr| arr(sr, "points").iter().map(pt)).filter(|p| fin(*p)).collect();
    let has_bars = series.iter().any(|sr| s(sr, "kind") == "bar");

    let xr = if let Some(cs) = &cats {
        (-0.5, cs.len() as f64 - 0.5)
    } else {
        let xs: Vec<f64> = pts.iter().map(|p| p.0).chain(vlines.iter().map(|v| f(v, "value"))).filter(|x| x.is_finite()).collect();
        if xs.is_empty() {
            (0.0, 1.0)
        } else {
            nice_range(xs.iter().cloned().fold(f64::INFINITY, f64::min), xs.iter().cloned().fold(f64::NEG_INFINITY, f64::max), false)
        }
    };
    // barras apiladas: base de cada barra = suma de las anteriores en la misma x
    let key = |x: f64| (x * 1e6).round() as i64;
    let mut base: HashMap<(usize, i64), f64> = HashMap::new();
    let mut tops = vec![];
    if stacked {
        let mut acc: HashMap<i64, f64> = HashMap::new();
        for (i, sr) in series.iter().enumerate() {
            if s(sr, "kind") != "bar" {
                continue;
            }
            for p in arr(sr, "points").iter().map(pt) {
                let b0 = *acc.get(&key(p.0)).unwrap_or(&0.0);
                base.insert((i, key(p.0)), b0);
                let top = b0 + if p.1.is_finite() { p.1 } else { 0.0 };
                acc.insert(key(p.0), top);
                tops.push(top);
            }
        }
    }
    let ys: Vec<f64> = pts.iter().map(|p| p.1).chain(tops).chain(hlines.iter().map(|h| f(h, "value"))).filter(|y| y.is_finite()).collect();
    let yr = if ys.is_empty() {
        (0.0, 1.0)
    } else {
        nice_range(ys.iter().cloned().fold(f64::INFINITY, f64::min), ys.iter().cloned().fold(f64::NEG_INFINITY, f64::max), b(c, "y_zero") || has_bars)
    };
    let yt = ticks(yr.0, yr.1, 5);
    let longest = yt.iter().map(|v| num(*v, 3).chars().count()).max().unwrap_or(1).max(1) as f64;
    // categorías largas: se parten en renglones del ancho de su espacio
    let slot = |k: usize| (w - (34.0 + longest * 7.4).max(62.0) - 22.0) / k.max(1) as f64;
    let cat_lines: Vec<Vec<String>> = match &cats {
        Some(cs) => {
            let step = (cs.len() as f64 / 14.0).ceil().max(1.0) as usize;
            let room = slot(cs.len()) * step as f64 - 6.0;
            cs.iter().map(|t| wrap(t, room, 11.5)).collect()
        }
        None => vec![],
    };
    let extra = cat_lines.iter().map(|l| l.len()).max().unwrap_or(1).saturating_sub(1) as f64 * 13.0;
    let h = h + extra;
    let (pl, pr, pt_, pb) = ((34.0 + longest * 7.4).max(62.0), 22.0, 18.0, 52.0 + extra);
    let sx = |x: f64| pl + (x - xr.0) / (if xr.1 - xr.0 == 0.0 { 1.0 } else { xr.1 - xr.0 }) * (w - pl - pr);
    let sy = |y: f64| h - pb - (y - yr.0) / (if yr.1 - yr.0 == 0.0 { 1.0 } else { yr.1 - yr.0 }) * (h - pt_ - pb);

    // colores: las series sin tono toman los de la paleta que no usen las que tienen tono
    let used: Vec<&str> = series.iter().map(|sr| s(sr, "tone")).filter(|t| !t.is_empty() && *t != "normal").map(tone).collect();
    let free: Vec<&str> = PALETTE.iter().cloned().filter(|c| !used.contains(c)).collect();
    let free = if free.is_empty() { PALETTE.to_vec() } else { free };
    let mut k = 0;
    let colors: Vec<String> = series
        .iter()
        .map(|sr| {
            let t = s(sr, "tone");
            if !t.is_empty() && t != "normal" {
                tone(t).to_string()
            } else {
                k += 1;
                free[(k - 1) % free.len()].to_string()
            }
        })
        .collect();

    let mut cv = Canvas::new(w, h);
    for v in &yt {
        cv.line(pl, sy(*v), w - pr, sy(*v), "#ececf0", 1.0, "");
    }
    let path = |p: &[(f64, f64)], step: bool| {
        let mut d = String::new();
        let mut pen = false;
        for q in p {
            if !fin(*q) {
                pen = false;
                continue;
            }
            if !pen {
                d += &format!(" M{},{}", n(sx(q.0)), n(sy(q.1)));
            } else if step {
                d += &format!(" H{} V{}", n(sx(q.0)), n(sy(q.1)));
            } else {
                d += &format!(" L{},{}", n(sx(q.0)), n(sy(q.1)));
            }
            pen = true;
        }
        d.trim().to_string()
    };
    let bars: Vec<usize> = series.iter().enumerate().filter(|(_, sr)| s(sr, "kind") == "bar").map(|(i, _)| i).collect();
    let bw = if bars.is_empty() {
        0.0
    } else {
        let mut xs: Vec<f64> = bars.iter().flat_map(|&i| arr(&series[i], "points").iter().map(|p| pt(p).0)).filter(|x| x.is_finite()).collect();
        xs.sort_by(|a, b| a.partial_cmp(b).unwrap());
        xs.dedup();
        let mut gap = if xs.len() > 1 { xs.windows(2).map(|w| w[1] - w[0]).fold(f64::INFINITY, f64::min) } else { xr.1 - xr.0 };
        if cats.is_some() {
            gap = 1.0;
        }
        (sx(gap) - sx(0.0)).abs() * 0.72 / if stacked { 1.0 } else { bars.len() as f64 }
    };
    for (i, sr) in series.iter().enumerate() {
        let col = &colors[i];
        let p: Vec<(f64, f64)> = arr(sr, "points").iter().map(pt).collect();
        let dash = if b(sr, "dashed") { r#"stroke-dasharray="6 5""# } else { "" };
        match s(sr, "kind") {
            "area" => {
                let y0 = sy(yr.0.max(0.0));
                let first = p.first().map(|q| sx(q.0)).unwrap_or(0.0);
                let last = p.last().map(|q| sx(q.0)).unwrap_or(0.0);
                cv.push(format!(r#"<path d="{} L{},{} L{},{} Z" fill="{col}" fill-opacity="0.14"/>"#, path(&p, false), n(last), n(y0), n(first), n(y0)));
                cv.push(format!(r#"<path d="{}" fill="none" stroke="{col}" stroke-width="2.25" stroke-linejoin="round" stroke-linecap="round"/>"#, path(&p, false)));
            }
            "line" | "step" => {
                cv.push(format!(r#"<path d="{}" fill="none" stroke="{col}" stroke-width="2.25" stroke-linejoin="round" stroke-linecap="round" {dash}/>"#, path(&p, s(sr, "kind") == "step")));
                if p.len() <= 40 {
                    for q in p.iter().filter(|q| fin(**q)) {
                        cv.push(format!(r#"<circle cx="{}" cy="{}" r="3.2" fill="{col}" stroke="white" stroke-width="1.4"/>"#, n(sx(q.0)), n(sy(q.1))));
                    }
                }
            }
            "scatter" => {
                let many = p.len() > 60;
                for q in p.iter().filter(|q| fin(**q)) {
                    cv.push(format!(r#"<circle cx="{}" cy="{}" r="{}" fill="{col}" fill-opacity="{}"/>"#, n(sx(q.0)), n(sy(q.1)), if many { 1.7 } else { 4.0 }, if many { 0.3 } else { 0.85 }));
                }
            }
            "bar" => {
                let bi = bars.iter().position(|&j| j == i).unwrap_or(0) as f64;
                for q in p.iter().filter(|q| fin(**q)) {
                    if stacked {
                        let b0 = *base.get(&(i, key(q.0))).unwrap_or(&0.0);
                        let (ya, yb) = (sy(b0 + q.1), sy(b0));
                        cv.push(format!(r#"<rect x="{}" y="{}" width="{}" height="{}" rx="2" fill="{col}" stroke="white" stroke-width="1"/>"#, n(sx(q.0) - bw / 2.0), n(ya.min(yb)), n(bw.max(1.0)), n((ya - yb).abs())));
                    } else {
                        let x0 = sx(q.0) - bw * bars.len() as f64 / 2.0 + bi * bw;
                        let (ya, yb) = (sy(q.1), sy(yr.0.max(0.0)));
                        cv.push(format!(r#"<rect x="{}" y="{}" width="{}" height="{}" rx="4" fill="{col}"/>"#, n(x0), n(ya.min(sy(0.0))), n((bw - 2.0).max(1.0)), n((ya - yb).abs())));
                    }
                }
            }
            _ => {}
        }
    }
    for hl in hlines {
        let v = f(hl, "value");
        let col = tone(s(hl, "tone"));
        let l = s(hl, "label");
        let txt = if l.is_empty() || l == num(v, 4) { num(v, 4) } else { format!("{l} = {}", num(v, 4)) };
        cv.line(pl, sy(v), w - pr, sy(v), col, 1.3, r#"stroke-dasharray="4 4""#);
        cv.text(w - pr - 4.0, sy(v) - 6.0, &txt, 11.0, col, "end", r#"font-weight="600" stroke="white" stroke-width="3" paint-order="stroke""#);
    }
    for vl in vlines {
        let v = f(vl, "value");
        let col = tone(s(vl, "tone"));
        cv.line(sx(v), pt_, sx(v), h - pb, col, 1.3, r#"stroke-dasharray="4 4""#);
        cv.text(sx(v) + 5.0, pt_ + 12.0, s(vl, "label"), 11.0, col, "start", r#"font-weight="600" stroke="white" stroke-width="3" paint-order="stroke""#);
    }
    // ejes
    cv.line(pl, h - pb, w - pr, h - pb, "#c7c7cc", 1.0, "");
    if let Some(cs) = &cats {
        let step = (cs.len() as f64 / 14.0).ceil().max(1.0) as usize;
        for i in (0..cs.len()).step_by(step) {
            let mut t = format!(r#"<text font-size="11.5" fill="{INK3}" text-anchor="middle">"#);
            for (k, line) in cat_lines[i].iter().enumerate() {
                t += &format!(r#"<tspan x="{}" y="{}">{}</tspan>"#, n(sx(i as f64)), n(h - pb + 17.0 + 13.0 * k as f64), esc(line));
            }
            cv.push(t + "</text>");
        }
    } else {
        for v in ticks(xr.0, xr.1, 6) {
            cv.text(sx(v), h - pb + 18.0, &num(v, 3), 11.5, INK3, "middle", "");
        }
    }
    for v in &yt {
        cv.text(pl - 10.0, sy(*v) + 4.0, &num(*v, 3), 11.5, INK3, "end", "");
    }
    cv.text((pl + w - pr) / 2.0, h - 10.0, s(c, "x_label"), 12.0, INK2, "middle", r#"font-weight="600""#);
    let my = (pt_ + h - pb) / 2.0;
    cv.push(format!(r#"<text transform="translate(16 {}) rotate(-90)" font-size="12" font-weight="600" fill="{INK2}" text-anchor="middle">{}</text>"#, n(my), esc(s(c, "y_label"))));
    if series.len() > 1 {
        for (i, sr) in series.iter().enumerate() {
            cv.legend.push((colors[i].clone(), s(sr, "name").to_string(), b(sr, "dashed")));
        }
    }
    Some(cv)
}

/// Parte un texto en renglones de a lo sumo `room` de ancho (máximo 3; el último se corta con "…").
fn wrap(t: &str, room: f64, size: f64) -> Vec<String> {
    if tw(t, size) <= room {
        return vec![t.to_string()];
    }
    let mut lines: Vec<String> = vec![];
    let mut cur = String::new();
    for word in t.split_whitespace() {
        let cand = if cur.is_empty() { word.to_string() } else { format!("{cur} {word}") };
        if tw(&cand, size) <= room || cur.is_empty() {
            cur = cand;
        } else {
            lines.push(std::mem::take(&mut cur));
            cur = word.to_string();
        }
    }
    if !cur.is_empty() {
        lines.push(cur);
    }
    if lines.len() > 3 {
        let mut last = lines[2..].join(" ");
        let max = (room / (size * 0.55)).floor().max(2.0) as usize;
        if last.chars().count() > max {
            last = last.chars().take(max - 1).collect::<String>() + "…";
        }
        lines.truncate(2);
        lines.push(last);
    }
    lines
}
