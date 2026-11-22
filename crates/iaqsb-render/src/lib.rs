//! Dibuja las gráficas de un informe de IAQSB (gráficos, redes, planos de
//! rutas, diagramas de Gantt, planos de bodega y árboles) como SVG y PNG.
//!
//! Trabaja sobre el informe en JSON (el mismo que devuelve `iaqsb_api::solve`),
//! con el diseño de la aplicación de escritorio en su tema claro, para que el
//! servidor MCP pueda devolver las gráficas como imágenes y la línea de comandos
//! guardarlas en archivos. La letra (Inter, con DejaVu Sans para los símbolos
//! que Inter no trae) va incluida en el binario: el dibujo es igual en Windows, macOS y Linux.

mod chart;
pub mod digest;
pub mod export;
mod pdf;
mod gantt;
mod map;
mod net;
mod plan;
mod tree;

use serde_json::Value;
use std::fmt::Write;
use std::sync::{Arc, OnceLock};

/// Una gráfica del informe.
#[derive(Debug, Clone)]
pub struct Figure {
    /// Título de la gráfica (o de dónde sale, si no tiene).
    pub title: String,
    /// Dónde está en el informe: "Sección" o "Sección › Paso".
    pub place: String,
    /// Tipo de bloque: chart, network, map, gantt, layout o tree.
    pub kind: String,
    pub svg: String,
}

/// Qué gráficas tomar del informe.
#[derive(Debug, Clone, Copy)]
pub struct Pick {
    /// Incluye las gráficas del paso a paso (p. ej. el recorrido parcial en cada iteración).
    pub steps: bool,
    /// Máximo de gráficas.
    pub max: usize,
}

impl Default for Pick {
    fn default() -> Self {
        Pick { steps: true, max: 24 }
    }
}

/// Las gráficas del informe en orden: primero las de las secciones y después
/// las del paso a paso. Devuelve también cuántas quedaron fuera por el máximo.
pub fn figures(report: &Value, pick: Pick) -> (Vec<Figure>, usize) {
    let mut main = vec![];
    let mut in_steps = vec![];
    for sec in arr(report, "sections") {
        let sname = s(sec, "title").to_string();
        if s(sec, "id") == "fuentes" {
            continue;
        }
        for b in arr(sec, "blocks") {
            walk(b, &sname, &mut main, &mut in_steps, false);
        }
    }
    let mut all = main;
    if pick.steps {
        all.extend(in_steps);
    }
    let left = all.len().saturating_sub(pick.max);
    all.truncate(pick.max);
    (all, left)
}

fn walk(b: &Value, place: &str, main: &mut Vec<Figure>, in_steps: &mut Vec<Figure>, nested: bool) {
    let kind = s(b, "type");
    if kind == "steps" {
        for st in arr(b, "steps") {
            let p = format!("{place} › {}", s(st, "title"));
            for x in arr(st, "blocks") {
                walk(x, &p, main, in_steps, true);
            }
        }
        return;
    }
    if let Some(svg) = svg(b) {
        let title = b.get("title").and_then(|t| t.as_str()).filter(|t| !t.is_empty()).unwrap_or(place).to_string();
        let f = Figure { title, place: place.to_string(), kind: kind.to_string(), svg };
        if nested {
            in_steps.push(f);
        } else {
            main.push(f);
        }
    }
}

/// SVG de un bloque gráfico del informe; None si el bloque no es una gráfica.
pub fn svg(block: &Value) -> Option<String> {
    let title = block.get("title").and_then(|t| t.as_str()).filter(|t| !t.is_empty());
    let fig = match s(block, "type") {
        "chart" => chart::draw(block),
        "network" => net::draw(block),
        "map" => map::draw(block),
        "gantt" => gantt::draw(block),
        "layout" => plan::draw(block),
        "tree" => tree::draw(block),
        _ => return None,
    }?;
    Some(fig.finish(title))
}

/// Convierte un SVG en PNG; `scale` = 2 da buena nitidez.
pub fn png(svg: &str, scale: f32) -> Result<Vec<u8>, String> {
    use resvg::{tiny_skia, usvg};
    let mut opt = usvg::Options::default();
    opt.fontdb = fonts();
    opt.font_family = FONT.into();
    let tree = usvg::Tree::from_str(svg, &opt).map_err(|e| format!("SVG no válido: {e}"))?;
    let size = tree.size().to_int_size().scale_by(scale).ok_or("tamaño no válido")?;
    let mut pix = tiny_skia::Pixmap::new(size.width(), size.height()).ok_or("imagen demasiado grande")?;
    pix.fill(tiny_skia::Color::WHITE);
    resvg::render(&tree, tiny_skia::Transform::from_scale(scale, scale), &mut pix.as_mut());
    indexed_png(pix.width(), pix.height(), pix.data())
}

/// PNG con paleta de 256 colores (NeuQuant): unas tres veces más liviano que en
/// RGBA y sin diferencia visible en estas gráficas de colores planos.
fn indexed_png(w: u32, h: u32, rgba: &[u8]) -> Result<Vec<u8>, String> {
    let nq = color_quant::NeuQuant::new(10, 256, rgba);
    let idx: Vec<u8> = rgba.chunks_exact(4).map(|p| nq.index_of(p) as u8).collect();
    let mut out = vec![];
    {
        let mut enc = png::Encoder::new(&mut out, w, h);
        enc.set_color(png::ColorType::Indexed);
        enc.set_depth(png::BitDepth::Eight);
        enc.set_palette(nq.color_map_rgb());
        enc.set_compression(png::Compression::Best);
        let mut wr = enc.write_header().map_err(|e| e.to_string())?;
        wr.write_image_data(&idx).map_err(|e| e.to_string())?;
    }
    Ok(out)
}

/// Comprueba que el SVG se puede leer (para las pruebas).
pub fn validate(svg: &str) -> Result<(), String> {
    let mut opt = resvg::usvg::Options::default();
    opt.fontdb = fonts();
    resvg::usvg::Tree::from_str(svg, &opt).map(|_| ()).map_err(|e| e.to_string())
}

/// PNG en base64 (para devolverlo por MCP).
pub fn base64(bytes: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for ch in bytes.chunks(3) {
        let n = (ch[0] as u32) << 16 | (*ch.get(1).unwrap_or(&0) as u32) << 8 | *ch.get(2).unwrap_or(&0) as u32;
        out.push(T[(n >> 18) as usize & 63] as char);
        out.push(T[(n >> 12) as usize & 63] as char);
        out.push(if ch.len() > 1 { T[(n >> 6) as usize & 63] as char } else { '=' });
        out.push(if ch.len() > 2 { T[n as usize & 63] as char } else { '=' });
    }
    out
}

pub(crate) const FONT: &str = "Inter";

pub(crate) fn fonts() -> Arc<resvg::usvg::fontdb::Database> {
    static DB: OnceLock<Arc<resvg::usvg::fontdb::Database>> = OnceLock::new();
    DB.get_or_init(|| {
        let mut db = resvg::usvg::fontdb::Database::new();
        db.load_font_data(include_bytes!("../fuentes/Inter-Regular.otf").to_vec());
        db.load_font_data(include_bytes!("../fuentes/Inter-SemiBold.otf").to_vec());
        db.load_font_data(include_bytes!("../fuentes/DejaVuSansCondensed.ttf").to_vec());
        db.set_sans_serif_family(FONT);
        Arc::new(db)
    })
    .clone()
}

// ---------------------------------------------------------------- colores (tema claro)

// colores del sistema de Apple (modo claro)
pub(crate) const INK: &str = "#1d1d1f";
pub(crate) const INK2: &str = "#6e6e73";
pub(crate) const INK3: &str = "#8e8e93";
pub(crate) const RULE: &str = "#e5e5ea";
pub(crate) const RULE_SOFT: &str = "#f2f2f7";
pub(crate) const SOLVER: &str = "#007aff";
pub(crate) const SOLVER_TINT: &str = "#e8f2ff";
pub(crate) const BINDING: &str = "#ff9500";
pub(crate) const BINDING_TINT: &str = "#fff3e0";
pub(crate) const GOOD: &str = "#34c759";
pub(crate) const BAD: &str = "#ff3b30";
/// Paleta para series sin tono propio.
pub(crate) const PALETTE: [&str; 8] = ["#007aff", "#ff9500", "#34c759", "#af52de", "#30b0c7", "#ff2d55", "#5856d6", "#a2845e"];

pub(crate) fn tone(t: &str) -> &'static str {
    match t {
        "accent" => SOLVER,
        "highlight" => BINDING,
        "good" => GOOD,
        "bad" => BAD,
        "muted" => "#aeaeb2",
        "total" => INK,
        _ => INK2,
    }
}

/// Mezcla un color "#rrggbb" con blanco: `pct` = cuánto del color queda (0–100).
pub(crate) fn tint(hex: &str, pct: f64) -> String {
    let h = hex.trim_start_matches('#');
    let c = |i: usize| u8::from_str_radix(&h[i..i + 2], 16).unwrap_or(0) as f64;
    let m = |v: f64| (255.0 - (255.0 - v) * pct / 100.0).round() as u8;
    format!("#{:02x}{:02x}{:02x}", m(c(0)), m(c(2)), m(c(4)))
}

// ---------------------------------------------------------------- lienzo

/// Dibujo de ancho `w` y alto `h`, con una leyenda opcional debajo.
pub(crate) struct Canvas {
    pub w: f64,
    pub h: f64,
    pub body: String,
    pub defs: String,
    /// (color, texto, discontinua)
    pub legend: Vec<(String, String, bool)>,
    /// Nota bajo el dibujo (escala del plano, por ejemplo).
    pub note: Option<String>,
}

impl Canvas {
    pub fn new(w: f64, h: f64) -> Canvas {
        Canvas { w, h, body: String::new(), defs: String::new(), legend: vec![], note: None }
    }
    pub fn push(&mut self, s: impl AsRef<str>) {
        self.body.push_str(s.as_ref());
        self.body.push('\n');
    }
    pub fn line(&mut self, x1: f64, y1: f64, x2: f64, y2: f64, stroke: &str, width: f64, extra: &str) {
        self.push(format!(r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{stroke}" stroke-width="{}" {extra}/>"#, n(x1), n(y1), n(x2), n(y2), n(width)));
    }
    /// Texto; `anchor` = start, middle o end.
    pub fn text(&mut self, x: f64, y: f64, t: &str, size: f64, fill: &str, anchor: &str, extra: &str) {
        self.push(format!(r#"<text x="{}" y="{}" font-size="{}" fill="{fill}" text-anchor="{anchor}" {extra}>{}</text>"#, n(x), n(y), n(size), esc(t)));
    }

    /// SVG completo: título arriba, dibujo, leyenda y nota abajo, sobre fondo blanco.
    pub fn finish(self, title: Option<&str>) -> String {
        let pad = 14.0;
        let top = if title.is_some() { 34.0 } else { 6.0 };
        // leyenda en filas
        let mut items = vec![];
        let (mut x, mut row) = (pad, 0usize);
        for (c, t, d) in &self.legend {
            let w = 26.0 + tw(t, 12.0);
            if x + w > self.w - pad && x > pad {
                row += 1;
                x = pad;
            }
            items.push((x, row, c, t, *d));
            x += w + 14.0;
        }
        let leg_h = if items.is_empty() { 0.0 } else { (row + 1) as f64 * 20.0 + 8.0 };
        let note_h = if self.note.is_some() { 20.0 } else { 0.0 };
        let total = top + self.h + leg_h + note_h + 6.0;
        let mut out = String::new();
        let _ = writeln!(
            out,
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="{}" height="{}" viewBox="0 0 {} {}" font-family="{FONT}, DejaVu Sans, sans-serif">"#,
            n(self.w),
            n(total),
            n(self.w),
            n(total)
        );
        if !self.defs.is_empty() {
            let _ = writeln!(out, "<defs>{}</defs>", self.defs);
        }
        let _ = writeln!(out, r#"<rect width="100%" height="100%" fill="white"/>"#);
        if let Some(t) = title {
            let _ = writeln!(out, r#"<text x="{pad}" y="23" font-size="15.5" font-weight="600" fill="{INK}" letter-spacing="-0.2">{}</text>"#, esc(t));
        }
        let _ = writeln!(out, r#"<g transform="translate(0 {})">"#, n(top));
        out.push_str(&self.body);
        out.push_str("</g>\n");
        let ly = top + self.h + 8.0;
        for (x, r, c, t, d) in items {
            let y = ly + r as f64 * 20.0 + 10.0;
            if d {
                let _ = writeln!(out, r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{c}" stroke-width="3" stroke-dasharray="5 3"/>"#, n(x), n(y), n(x + 18.0), n(y));
            } else {
                let _ = writeln!(out, r#"<circle cx="{}" cy="{}" r="5" fill="{c}"/>"#, n(x + 9.0), n(y));
            }
            let _ = writeln!(out, r#"<text x="{}" y="{}" font-size="12" fill="{INK2}">{}</text>"#, n(x + 24.0), n(y + 4.0), esc(t));
        }
        if let Some(nt) = &self.note {
            let _ = writeln!(out, r#"<text x="{pad}" y="{}" font-size="11.5" fill="{INK3}">{}</text>"#, n(top + self.h + leg_h + 14.0), esc(nt));
        }
        out.push_str("</svg>\n");
        out
    }
}

// ---------------------------------------------------------------- utilidades

pub(crate) fn s<'a>(v: &'a Value, k: &str) -> &'a str {
    v.get(k).and_then(|x| x.as_str()).unwrap_or("")
}
pub(crate) fn f(v: &Value, k: &str) -> f64 {
    v.get(k).and_then(|x| x.as_f64()).unwrap_or(f64::NAN)
}
pub(crate) fn arr<'a>(v: &'a Value, k: &str) -> &'a [Value] {
    v.get(k).and_then(|x| x.as_array()).map(|a| a.as_slice()).unwrap_or(&[])
}
pub(crate) fn b(v: &Value, k: &str) -> bool {
    v.get(k).and_then(|x| x.as_bool()).unwrap_or(false)
}
pub(crate) fn opt<'a>(v: &'a Value, k: &str) -> Option<&'a str> {
    v.get(k).and_then(|x| x.as_str()).filter(|x| !x.is_empty())
}

/// Coordenada con un decimal.
pub(crate) fn n(x: f64) -> String {
    if !x.is_finite() {
        return "0".into();
    }
    let r = (x * 10.0).round() / 10.0;
    if r == r.trunc() {
        format!("{}", r as i64)
    } else {
        format!("{r:.1}")
    }
}

/// Número para mostrar, como `num` de la interfaz: hasta `digits` decimales, sin ceros de más.
pub(crate) fn num(x: f64, digits: usize) -> String {
    if x.is_nan() {
        return "–".into();
    }
    if x.is_infinite() {
        return if x > 0.0 { "∞".into() } else { "−∞".into() };
    }
    let r = if x.abs() < 1e-10 { 0.0 } else { x };
    let mut t = format!("{r:.digits$}");
    if t.contains('.') {
        t = t.trim_end_matches('0').trim_end_matches('.').to_string();
    }
    if t == "-0" {
        t = "0".into();
    }
    t.replace('-', "−")
}

pub(crate) fn esc(t: &str) -> String {
    t.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

/// Ancho aproximado de un texto (Inter).
pub(crate) fn tw(t: &str, size: f64) -> f64 {
    t.chars().count() as f64 * size * 0.57
}

/// Marcas de eje "bonitas" entre lo y hi.
pub(crate) fn ticks(lo: f64, mut hi: f64, k: usize) -> Vec<f64> {
    if !lo.is_finite() || !hi.is_finite() {
        return vec![];
    }
    if hi == lo {
        hi = lo + 1.0;
    }
    let step0 = (hi - lo) / k as f64;
    let mag = 10f64.powf(step0.log10().floor());
    let step = [1.0, 2.0, 2.5, 5.0, 10.0].iter().map(|s| s * mag).find(|s| *s >= step0).unwrap_or(step0);
    let mut out = vec![];
    let mut v = (lo / step - 1e-9).ceil() * step;
    while v <= hi + step * 1e-9 && out.len() < 200 {
        out.push(if v.abs() < step * 1e-9 { 0.0 } else { v });
        v += step;
    }
    out
}

/// Rango con extremos redondeados a las marcas.
pub(crate) fn nice_range(mut lo: f64, mut hi: f64, zero: bool) -> (f64, f64) {
    if zero {
        lo = lo.min(0.0);
        hi = hi.max(0.0);
    }
    if lo == hi {
        lo -= 1.0;
        hi += 1.0;
    }
    let t = ticks(lo, hi, 6);
    let step = if t.len() > 1 { t[1] - t[0] } else { 1.0 };
    ((lo / step + 1e-9).floor() * step, (hi / step - 1e-9).ceil() * step)
}
