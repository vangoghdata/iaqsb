//! Árbol de izquierda a derecha (decisiones, azar, resultados), como TreeView.svelte.

use crate::*;
use std::collections::HashMap;

pub fn draw(t: &Value) -> Option<Canvas> {
    let nodes = arr(t, "nodes");
    if nodes.is_empty() {
        return None;
    }
    let id = |v: &Value| v.get("id").and_then(|x| x.as_u64()).unwrap_or(0);
    let parent = |v: &Value| v.get("parent").and_then(|x| x.as_u64());
    let mut kids: HashMap<u64, Vec<u64>> = HashMap::new();
    let mut roots = vec![];
    for nd in nodes {
        match parent(nd) {
            Some(p) => kids.entry(p).or_default().push(id(nd)),
            None => roots.push(id(nd)),
        }
    }
    let mut depth: HashMap<u64, usize> = HashMap::new();
    let mut ypos: HashMap<u64, f64> = HashMap::new();
    let mut leaf = 0usize;
    fn visit(i: u64, d: usize, kids: &HashMap<u64, Vec<u64>>, depth: &mut HashMap<u64, usize>, ypos: &mut HashMap<u64, f64>, leaf: &mut usize) {
        if depth.contains_key(&i) {
            return;
        }
        depth.insert(i, d);
        let ch = kids.get(&i).cloned().unwrap_or_default();
        if ch.is_empty() {
            ypos.insert(i, *leaf as f64);
            *leaf += 1;
            return;
        }
        for c in &ch {
            visit(*c, d + 1, kids, depth, ypos, leaf);
        }
        let a = ypos.get(&ch[0]).cloned().unwrap_or(0.0);
        let z = ypos.get(ch.last().unwrap()).cloned().unwrap_or(0.0);
        ypos.insert(i, (a + z) / 2.0);
    }
    for r in &roots {
        visit(*r, 0, &kids, &mut depth, &mut ypos, &mut leaf);
    }
    let max_d = depth.values().cloned().max().unwrap_or(0);
    let col_w = (760.0 / (max_d + 1) as f64).clamp(150.0, 220.0);
    let w = 60.0 + (max_d + 1) as f64 * col_w + 160.0;
    let row_h = 46.0;
    let h = 40.0 + leaf.max(1) as f64 * row_h;
    let px = |i: u64| 40.0 + *depth.get(&i).unwrap_or(&0) as f64 * col_w;
    let py = |i: u64| 30.0 + ypos.get(&i).cloned().unwrap_or(0.0) * row_h;
    let mut cv = Canvas::new(w, h);
    let hot = |t: &str| t == "highlight" || t == "accent";
    for nd in nodes {
        let Some(p) = parent(nd) else { continue };
        let (x1, y1, x2, y2) = (px(p) + 12.0, py(p), px(id(nd)) - 12.0, py(id(nd)));
        let tn = s(nd, "tone");
        let (col, sw) = if hot(tn) { (BINDING, 2.4) } else if tn == "muted" { ("#e5e5ea", 1.3) } else { (INK3, 1.3) };
        cv.push(format!(r#"<path d="M{},{} L{},{} L{},{}" fill="none" stroke="{col}" stroke-width="{sw}"/>"#, n(x1), n(y1), n(x1 + 24.0), n(y2), n(x2), n(y2)));
        if let Some(e) = opt(nd, "edge") {
            cv.text(x1 + 30.0, y2 - 6.0, e, 11.5, INK2, "start", "");
        }
    }
    let mut kinds = false;
    for nd in nodes {
        let (x, y) = (px(id(nd)), py(id(nd)));
        let tn = s(nd, "tone");
        let (fill, stroke, sw) = if hot(tn) { (BINDING_TINT, BINDING, 2.2) } else { ("white", INK2, 1.5) };
        let st = format!(r#"fill="{fill}" stroke="{stroke}" stroke-width="{sw}""#);
        let kind = s(nd, "kind");
        match kind {
            "decision" => cv.push(format!(r#"<rect x="{}" y="{}" width="22" height="22" rx="3" {st}/>"#, n(x - 11.0), n(y - 11.0))),
            "chance" => cv.push(format!(r#"<circle cx="{}" cy="{}" r="11" {st}/>"#, n(x), n(y))),
            "end" => cv.push(format!(r#"<path d="M{},{} L{},{} L{},{} Z" {st}/>"#, n(x - 10.0), n(y), n(x + 9.0), n(y - 10.0), n(x + 9.0), n(y + 10.0))),
            _ => cv.push(format!(r#"<circle cx="{}" cy="{}" r="7" {st}/>"#, n(x), n(y))),
        }
        kinds |= matches!(kind, "decision" | "chance" | "end");
        cv.text(x, y - 16.0, s(nd, "label"), 12.0, INK, "middle", r#"font-weight="600""#);
        if let Some(v) = opt(nd, "value") {
            if kind == "end" {
                cv.text(x + 16.0, y + 4.0, v, 12.0, SOLVER, "start", r#"font-weight="600""#);
            } else {
                cv.text(x, y + 26.0, v, 12.0, SOLVER, "middle", r#"font-weight="600""#);
            }
        }
    }
    if kinds {
        cv.note = Some("■ decisión   ● azar   ◀ resultado".into());
    }
    Some(cv)
}
