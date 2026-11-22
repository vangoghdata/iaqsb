//! Plano a escala de la bodega (como LayoutView.svelte).

use crate::*;

pub fn draw(p: &Value) -> Option<Canvas> {
    let (pw, ph) = (f(p, "width"), f(p, "height"));
    if !(pw > 0.0 && ph > 0.0) {
        return None;
    }
    let w = 760.0;
    let pad = 46.0;
    let scale = ((w - 2.0 * pad) / pw).min(520.0 / ph);
    let h = ph * scale + 2.0 * pad;
    let x = |v: f64| pad + v * scale;
    let y = |v: f64| pad + v * scale;
    let mut cv = Canvas::new(w, h);
    cv.defs.push_str(&format!(r#"<marker id="tick" viewBox="0 0 6 6" refX="3" refY="3" markerWidth="6" markerHeight="6" orient="auto"><path d="M3 0 L3 6" stroke="{INK2}" stroke-width="1.2"/></marker>"#));
    cv.push(format!(r#"<rect x="{}" y="{}" width="{}" height="{}" fill="{RULE_SOFT}" stroke="{INK3}" stroke-width="1.5"/>"#, n(x(0.0)), n(y(0.0)), n(pw * scale), n(ph * scale)));
    for r in arr(p, "rects") {
        let t = if s(r, "tone").is_empty() { "normal" } else { s(r, "tone") };
        let soft = b(r, "soft");
        let fill = if soft { tint(INK3, 10.0) } else { tint(tone(t), if t != "normal" { 26.0 } else { 14.0 }) };
        let stroke = if soft { "none".to_string() } else { tone(t).to_string() };
        let (rw, rh) = (f(r, "w") * scale, f(r, "h") * scale);
        cv.push(format!(
            r#"<rect x="{}" y="{}" width="{}" height="{}" rx="1.5" fill="{fill}" stroke="{stroke}" stroke-width="{}"/>"#,
            n(x(f(r, "x"))),
            n(y(f(r, "y"))),
            n(rw.max(0.5)),
            n(rh.max(0.5)),
            if soft { 0 } else { 1 }
        ));
        if let Some(l) = opt(r, "label") {
            if rw > 26.0 && rh > 13.0 {
                let size = if tw(l, 11.0) > rw - 4.0 { 9.0 } else { 11.0 };
                cv.text(x(f(r, "x") + f(r, "w") / 2.0), y(f(r, "y") + f(r, "h") / 2.0) + size * 0.35, l, size, if soft { INK3 } else { INK }, "middle", "");
            }
        }
    }
    for d in arr(p, "dims") {
        let (x1, y1, x2, y2) = (f(d, "x1"), f(d, "y1"), f(d, "x2"), f(d, "y2"));
        cv.line(x(x1), y(y1), x(x2), y(y2), INK2, 1.0, r#"marker-start="url(#tick)" marker-end="url(#tick)""#);
        let l = s(d, "label");
        if (y2 - y1).abs() < 1e-9 {
            cv.text(x((x1 + x2) / 2.0), y(y1) - 6.0, l, 11.0, INK2, "middle", r#"font-weight="600""#);
        } else {
            let (cx, cy) = (x(x1) - 6.0, y((y1 + y2) / 2.0));
            cv.text(cx, cy, l, 11.0, INK2, "middle", &format!(r#"font-weight="600" transform="rotate(-90 {} {})""#, n(cx), n(cy)));
        }
    }
    cv.note = Some(format!("Escala: {} × {} {}", num(pw, 3), num(ph, 3), s(p, "unit")));
    Some(cv)
}
