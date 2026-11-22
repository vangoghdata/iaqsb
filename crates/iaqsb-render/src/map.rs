//! Plano de rutas: puntos en sus coordenadas y un color por vehículo (como MapView.svelte).

use crate::*;
use std::collections::HashMap;

const COLORS: [&str; 12] = [SOLVER, BINDING, "#10b981", "#ef4444", "#0ea5e9", "#d946ef", "#84cc16", "#f97316", "#14b8a6", "#a855f7", "#64748b", "#eab308"];

fn color(k: Option<usize>) -> &'static str {
    k.map(|k| COLORS[k % COLORS.len()]).unwrap_or(INK3)
}

pub fn draw(m: &Value) -> Option<Canvas> {
    let points = arr(m, "points");
    if points.is_empty() {
        return None;
    }
    let w = 820.0;
    let pad = 46.0;
    let xs: Vec<f64> = points.iter().map(|p| f(p, "x")).collect();
    let ys: Vec<f64> = points.iter().map(|p| f(p, "y")).collect();
    let mn = |v: &[f64]| v.iter().cloned().fold(f64::INFINITY, f64::min);
    let mx = |v: &[f64]| v.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let (mut x0, mut x1, mut y0, mut y1) = (mn(&xs), mx(&xs), mn(&ys), mx(&ys));
    if x1 - x0 < 1e-9 {
        x0 -= 1.0;
        x1 += 1.0;
    }
    if y1 - y0 < 1e-9 {
        y0 -= 1.0;
        y1 += 1.0;
    }
    // coordenadas geográficas (x = longitud, y = latitud): proyección
    // equirectangular, un grado de longitud mide cos(latitud) grados de latitud
    let geo = b(m, "geo");
    let kx = if geo { ((y0 + y1) / 2.0).to_radians().cos().max(0.05) } else { 1.0 };
    let (ex, ey) = ((x1 - x0) * 0.06, (y1 - y0) * 0.06);
    let (x0, x1, y0, y1) = (x0 - ex, x1 + ex, y0 - ey, y1 + ey);
    // misma escala en X y en Y para no deformar los ángulos (método del barrido)
    let scale = ((w - 2.0 * pad) / ((x1 - x0) * kx)).min(560.0 / (y1 - y0));
    let h = ((y1 - y0) * scale + 2.0 * pad).max(260.0);
    let off = (w - (x1 - x0) * kx * scale) / 2.0;
    let sx = |x: f64| off + (x - x0) * kx * scale;
    let sy = |y: f64| h - pad - (y - y0) * scale;
    let tick = |v: f64| if geo { format!("{}°", num(v, 4)) } else { num(v, 3) };
    let by: HashMap<&str, &Value> = points.iter().map(|p| (s(p, "id"), p)).collect();

    let mut cv = Canvas::new(w, h);
    let (xt, yt) = (ticks(x0, x1, 8), ticks(y0, y1, 6));
    for v in &xt {
        cv.line(sx(*v), pad * 0.4, sx(*v), h - pad * 0.6, RULE_SOFT, 1.0, "");
        cv.text(sx(*v), h - 10.0, &tick(*v), 10.5, INK3, "middle", "");
    }
    for v in &yt {
        cv.line(off - 6.0, sy(*v), w - off + 6.0, sy(*v), RULE_SOFT, 1.0, "");
        cv.text(off - 10.0, sy(*v) + 4.0, &tick(*v), 10.5, INK3, "end", "");
    }
    for r in arr(m, "rays") {
        let Some(p) = by.get(s(r, "from")) else { continue };
        let (px, py) = (f(p, "x"), f(p, "y"));
        let a = f(r, "angle").to_radians();
        // el ángulo se mide en el plano proyectado: en X se avanza dx / kx
        let (dx, dy) = (a.cos() / kx, a.sin());
        let mut t = ((x1 - x0) * kx).hypot(y1 - y0);
        if dx > 1e-9 {
            t = t.min((x1 - px) / dx);
        }
        if dx < -1e-9 {
            t = t.min((x0 - px) / dx);
        }
        if dy > 1e-9 {
            t = t.min((y1 - py) / dy);
        }
        if dy < -1e-9 {
            t = t.min((y0 - py) / dy);
        }
        let start = s(r, "label") == "inicio";
        let (ax, ay, bx, by_) = (sx(px), sy(py), sx(px + dx * t), sy(py + dy * t));
        cv.line(ax, ay, bx, by_, if start { SOLVER } else { INK3 }, 1.2, r#"stroke-dasharray="5 5" stroke-opacity="0.8""#);
        if let Some(l) = opt(r, "label") {
            cv.text(bx, by_ - 6.0, l, 11.0, SOLVER, "middle", r#"font-weight="600""#);
        }
    }
    for (k, r) in arr(m, "routes").iter().enumerate() {
        let col = color(Some(k));
        let stops: Vec<&str> = arr(r, "stops").iter().filter_map(|x| x.as_str()).collect();
        let dash = if b(r, "dashed") { r#"stroke-dasharray="6 5""# } else { "" };
        for pair in stops.windows(2) {
            let (Some(a), Some(c)) = (by.get(pair[0]), by.get(pair[1])) else { continue };
            let (ax, ay, cx, cy) = (sx(f(a, "x")), sy(f(a, "y")), sx(f(c, "x")), sy(f(c, "y")));
            cv.line(ax, ay, cx, cy, col, 2.6, &format!(r#"stroke-linecap="round" stroke-opacity="0.9" {dash}"#));
            if (cx - ax).hypot(cy - ay) > 34.0 {
                let ang = (cy - ay).atan2(cx - ax).to_degrees();
                cv.push(format!(r#"<path d="M-5,-4.5 L5,0 L-5,4.5 z" fill="{col}" transform="translate({} {}) rotate({})"/>"#, n((ax + cx) / 2.0), n((ay + cy) / 2.0), n(ang)));
            }
        }
        let name = s(r, "name");
        let label = match opt(r, "note") {
            Some(nt) => format!("{name} · {nt}"),
            None => name.to_string(),
        };
        cv.legend.push((col.to_string(), label, false));
    }
    let halo = r#"stroke="white" stroke-width="4" stroke-linejoin="round" paint-order="stroke""#;
    for p in points {
        let (x, y) = (sx(f(p, "x")), sy(f(p, "y")));
        if s(p, "kind") == "depot" {
            cv.push(format!(r#"<rect x="{}" y="{}" width="22" height="22" rx="6" fill="{SOLVER}" stroke="white" stroke-width="2"/>"#, n(x - 11.0), n(y - 11.0)));
            cv.push(format!(
                r#"<path d="M{},{} L{},{} L{},{} M{},{} V{} H{} V{}" fill="none" stroke="white" stroke-width="1.6" stroke-linejoin="round"/>"#,
                n(x - 5.0),
                n(y + 1.0),
                n(x),
                n(y - 4.0),
                n(x + 5.0),
                n(y + 1.0),
                n(x - 3.5),
                n(y),
                n(y + 5.0),
                n(x + 3.5),
                n(y)
            ));
            cv.text(x, y - 17.0, s(p, "label"), 12.0, INK, "middle", &format!(r#"font-weight="600" {halo}"#));
        } else {
            let col = color(p.get("route").and_then(|r| r.as_u64()).map(|r| r as usize));
            cv.push(format!(r#"<circle cx="{}" cy="{}" r="7" fill="white" stroke="{col}" stroke-width="3"/>"#, n(x), n(y)));
            cv.text(x, y - 12.0, s(p, "label"), 12.0, INK, "middle", &format!(r#"font-weight="600" {halo}"#));
            if let Some(nt) = opt(p, "note") {
                cv.text(x, y + 20.0, nt, 10.5, INK3, "middle", r#"stroke="white" stroke-width="3" paint-order="stroke""#);
            }
        }
    }
    Some(cv)
}
