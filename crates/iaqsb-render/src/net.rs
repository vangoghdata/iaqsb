//! Red de nodos y arcos (como NetworkView.svelte).

use crate::*;
use std::collections::{HashMap, HashSet};

#[derive(Clone, Copy)]
struct P {
    x: f64,
    y: f64,
    w: f64,
    h: f64,
}

/// Punto del borde del rectángulo del nodo en la dirección (dx, dy).
fn border(p: P, dx: f64, dy: f64) -> (f64, f64) {
    let len = dx.hypot(dy).max(1e-9);
    let (ux, uy) = (dx / len, dy / len);
    let tx = if ux.abs() > 1e-9 { p.w / 2.0 / ux.abs() } else { f64::INFINITY };
    let ty = if uy.abs() > 1e-9 { p.h / 2.0 / uy.abs() } else { f64::INFINITY };
    let t = tx.min(ty) + 3.0;
    (p.x + ux * t, p.y + uy * t)
}

struct Geom {
    d: String,
    s: (f64, f64),
    c: Option<(f64, f64)>,
    t: (f64, f64),
}

impl Geom {
    fn at(&self, u: f64) -> (f64, f64) {
        match self.c {
            Some(c) => (
                (1.0 - u).powi(2) * self.s.0 + 2.0 * (1.0 - u) * u * c.0 + u * u * self.t.0,
                (1.0 - u).powi(2) * self.s.1 + 2.0 * (1.0 - u) * u * c.1 + u * u * self.t.1,
            ),
            None => (self.s.0 + (self.t.0 - self.s.0) * u, self.s.1 + (self.t.1 - self.s.1) * u),
        }
    }
}

pub fn draw(nw: &Value) -> Option<Canvas> {
    let nodes = arr(nw, "nodes");
    if nodes.is_empty() {
        return None;
    }
    let edges = arr(nw, "edges");
    let directed = b(nw, "directed");
    let w = 820.0;
    let mut cols: HashMap<i64, usize> = HashMap::new();
    for nd in nodes {
        *cols.entry((f(nd, "x") * 20.0).round() as i64).or_default() += 1;
    }
    let per_col = cols.values().cloned().max().unwrap_or(0).max(4) as f64;
    let h = (140.0 + 52.0 * per_col).clamp(300.0, 620.0);
    let pad = 60.0;
    let pos: HashMap<&str, P> = nodes
        .iter()
        .map(|nd| {
            let lw = (tw(s(nd, "label"), 12.5) + 22.0).max(44.0);
            (s(nd, "id"), P { x: pad + f(nd, "x") * (w - 2.0 * pad), y: pad * 0.7 + f(nd, "y") * (h - 1.4 * pad), w: lw, h: 30.0 })
        })
        .collect();
    let pairs: HashSet<(&str, &str)> = edges.iter().map(|e| (s(e, "from"), s(e, "to"))).collect();

    let geom = |e: &Value| -> Option<Geom> {
        let a = *pos.get(s(e, "from"))?;
        let bb = *pos.get(s(e, "to"))?;
        if s(e, "from") == s(e, "to") {
            // lazo hacia fuera del dibujo
            let (mut ox, mut oy) = (a.x - w / 2.0, a.y - h / 2.0);
            let ol = ox.hypot(oy);
            if ol < 1.0 {
                ox = 0.0;
                oy = -1.0;
            } else {
                ox /= ol;
                oy /= ol;
            }
            let (px, py) = (-oy, ox);
            let base = border(a, ox, oy);
            let s0 = (base.0 + px * 9.0, base.1 + py * 9.0);
            let t0 = (base.0 - px * 9.0, base.1 - py * 9.0);
            let c1 = (s0.0 + ox * 46.0 + px * 22.0, s0.1 + oy * 46.0 + py * 22.0);
            let c2 = (t0.0 + ox * 46.0 - px * 22.0, t0.1 + oy * 46.0 - py * 22.0);
            let top = (base.0 + ox * 38.0, base.1 + oy * 38.0);
            return Some(Geom {
                d: format!("M{},{} C{},{} {},{} {},{}", n(s0.0), n(s0.1), n(c1.0), n(c1.1), n(c2.0), n(c2.1), n(t0.0), n(t0.1)),
                s: top,
                c: None,
                t: top,
            });
        }
        let both = directed && pairs.contains(&(s(e, "to"), s(e, "from")));
        let (dx, dy) = (bb.x - a.x, bb.y - a.y);
        let len = dx.hypot(dy).max(1e-9);
        let back = directed && !both && dx < -5.0;
        let curved = both || back;
        let off = if both { 18.0 } else if back { (22.0 + 0.22 * len).min(140.0) } else { 0.0 };
        let (nx, ny) = (-dy / len * off, dx / len * off);
        let (cx, cy) = ((a.x + bb.x) / 2.0 + nx, (a.y + bb.y) / 2.0 + ny);
        let s0 = border(a, cx - a.x, cy - a.y);
        let t0 = border(bb, cx - bb.x, cy - bb.y);
        let d = if curved {
            format!("M{},{} Q{},{} {},{}", n(s0.0), n(s0.1), n(cx), n(cy), n(t0.0), n(t0.1))
        } else {
            format!("M{},{} L{},{}", n(s0.0), n(s0.1), n(t0.0), n(t0.1))
        };
        Some(Geom { d, s: s0, c: if curved { Some((cx, cy)) } else { None }, t: t0 })
    };

    let mut cv = Canvas::new(w, h);
    for (tn, col) in [("normal", "#aeaeb2"), ("muted", RULE), ("highlight", BINDING), ("accent", SOLVER), ("good", GOOD), ("bad", BAD)] {
        cv.defs.push_str(&format!(
            r#"<marker id="ar-{tn}" viewBox="0 0 10 10" refX="9" refY="5" markerUnits="userSpaceOnUse" markerWidth="10" markerHeight="10" orient="auto"><path d="M0 0L10 5L0 10z" fill="{col}"/></marker>"#
        ));
    }
    let style = |t: &str| -> (&'static str, f64) {
        match t {
            "muted" => ("#e5e5ea", 1.3),
            "highlight" => (BINDING, 3.0),
            "accent" => (SOLVER, 2.2),
            "good" => (GOOD, 2.2),
            "bad" => (BAD, 2.2),
            _ => ("#c7c7cc", 1.4),
        }
    };
    // primero los arcos normales y después los destacados, encima
    for hl in [false, true] {
        for e in edges.iter().filter(|e| (s(e, "tone") == "highlight") == hl) {
            if let Some(g) = geom(e) {
                let t = if s(e, "tone").is_empty() { "normal" } else { s(e, "tone") };
                let (col, sw) = style(t);
                let mk = if directed { format!(r#" marker-end="url(#ar-{t})""#) } else { String::new() };
                cv.push(format!(r#"<path d="{}" fill="none" stroke="{col}" stroke-width="{sw}" stroke-linecap="round"{mk}/>"#, g.d));
            }
        }
    }
    // etiquetas de los arcos sin solaparse
    let mut placed: Vec<(f64, f64, f64)> = vec![];
    for e in edges {
        let Some(label) = opt(e, "label") else { continue };
        let Some(g) = geom(e) else { continue };
        let lw = tw(label, 11.5) + 10.0;
        let mut best = g.at(0.5);
        for u in [0.5, 0.36, 0.64, 0.26, 0.74, 0.44, 0.56, 0.3, 0.7] {
            let p = g.at(u);
            if !placed.iter().any(|q| (q.0 - p.0).abs() < (q.2 + lw) / 2.0 + 2.0 && (q.1 - p.1).abs() < 20.0) {
                best = p;
                break;
            }
        }
        placed.push((best.0, best.1, lw));
        let t = s(e, "tone");
        let (stroke, fill, bold) = match t {
            "highlight" => (BINDING, INK, true),
            "muted" => (RULE_SOFT, INK3, false),
            _ => ("#e5e5ea", INK2, false),
        };
        cv.push(format!(r#"<rect x="{}" y="{}" width="{}" height="18" rx="9" fill="white" stroke="{stroke}"/>"#, n(best.0 - lw / 2.0), n(best.1 - 9.0), n(lw)));
        cv.text(best.0, best.1 + 4.0, label, 11.5, fill, "middle", if bold { r#"font-weight="600""# } else { "" });
    }
    for nd in nodes {
        let p = pos[s(nd, "id")];
        let (fill, stroke, sw) = match s(nd, "tone") {
            "accent" => (SOLVER_TINT, SOLVER, 2.0),
            "highlight" => (BINDING_TINT, BINDING, 2.0),
            "good" => ("#e6f2ee", GOOD, 2.0),
            "bad" => ("#fceeee", BAD, 2.0),
            "muted" => ("white", "#d1d1d6", 1.2),
            _ => ("white", "#c7c7cc", 1.3),
        };
        cv.push(format!(
            r#"<rect x="{}" y="{}" width="{}" height="{}" rx="15" fill="{fill}" stroke="{stroke}" stroke-width="{sw}"/>"#,
            n(p.x - p.w / 2.0),
            n(p.y - p.h / 2.0),
            n(p.w),
            n(p.h)
        ));
        cv.text(p.x, p.y + 4.5, s(nd, "label"), 12.5, if s(nd, "tone") == "muted" { INK3 } else { INK }, "middle", r#"font-weight="600""#);
        if let Some(note) = opt(nd, "note") {
            let mut t = format!(r#"<text font-size="11" fill="{INK2}" text-anchor="middle" stroke="white" stroke-width="4" stroke-linejoin="round" paint-order="stroke">"#);
            for (k, line) in note.split('\n').enumerate() {
                t += &format!(r#"<tspan x="{}" y="{}">{}</tspan>"#, n(p.x), n(p.y + p.h / 2.0 + 14.0 + 13.0 * k as f64), esc(line));
            }
            t += "</text>";
            cv.push(t);
        }
    }
    Some(cv)
}
