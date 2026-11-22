//! PDF de un informe: maquetación propia en páginas A4 (cada página se arma como
//! SVG con la letra Inter incluida) y conversión con svg2pdf. El texto queda
//! seleccionable y las figuras, vectoriales.

use crate::export::{runs, svg_size, Doc, Item, Table};
use std::fmt::Write as _;

const W: f64 = 595.28;
const H: f64 = 841.89;
const ML: f64 = 64.0;
const MR: f64 = 64.0;
const MT: f64 = 64.0;
const MB: f64 = 70.0;
const AW: f64 = W - ML - MR;
const INK: &str = "#1d1d1f";
const INK2: &str = "#515154";
const RULE: &str = "#d2d2d7";
const ACCENT: &str = "#0a66d8";
const SOFT: &str = "#f2f2f7";

struct Faces {
    reg: ttf_parser::Face<'static>,
    bold: ttf_parser::Face<'static>,
    fall: ttf_parser::Face<'static>,
}

fn faces() -> &'static Faces {
    static F: std::sync::OnceLock<Faces> = std::sync::OnceLock::new();
    F.get_or_init(|| Faces {
        reg: ttf_parser::Face::parse(include_bytes!("../fuentes/Inter-Regular.otf"), 0).expect("Inter"),
        bold: ttf_parser::Face::parse(include_bytes!("../fuentes/Inter-SemiBold.otf"), 0).expect("Inter SemiBold"),
        fall: ttf_parser::Face::parse(include_bytes!("../fuentes/DejaVuSansCondensed.ttf"), 0).expect("DejaVu"),
    })
}

/// Ancho de un texto en puntos.
fn width(t: &str, size: f64, bold: bool) -> f64 {
    let f = faces();
    let face = if bold { &f.bold } else { &f.reg };
    let mut w = 0.0;
    for ch in t.chars() {
        let (fc, g) = match face.glyph_index(ch) {
            Some(g) => (face, g),
            None => match f.fall.glyph_index(ch) {
                Some(g) => (&f.fall, g),
                None => {
                    w += size * 0.5;
                    continue;
                }
            },
        };
        w += fc.glyph_hor_advance(g).unwrap_or(0) as f64 / fc.units_per_em() as f64 * size;
    }
    w
}

fn esc(t: &str) -> String {
    let mut o = String::with_capacity(t.len());
    for c in t.chars() {
        match c {
            '&' => o.push_str("&amp;"),
            '<' => o.push_str("&lt;"),
            '>' => o.push_str("&gt;"),
            '"' => o.push_str("&quot;"),
            c if (c as u32) < 0x20 => {}
            c => o.push(c),
        }
    }
    o
}

/// Un trozo de texto con estilo.
#[derive(Clone)]
struct Run {
    t: String,
    bold: bool,
    ital: bool,
}

/// Parte trozos en líneas que caben en `max` (por palabras; una palabra más larga
/// que la línea se corta por caracteres).
fn wrap(text_runs: &[Run], size: f64, max: f64) -> Vec<Vec<Run>> {
    let mut lines: Vec<Vec<Run>> = vec![];
    let mut cur: Vec<Run> = vec![];
    let mut cur_w = 0.0;
    let push_word = |cur: &mut Vec<Run>, word: &str, r: &Run| {
        if let Some(last) = cur.last_mut() {
            if last.bold == r.bold && last.ital == r.ital {
                last.t.push_str(word);
                return;
            }
        }
        cur.push(Run { t: word.to_string(), bold: r.bold, ital: r.ital });
    };
    for r in text_runs {
        for (k, para) in r.t.split('\n').enumerate() {
            if k > 0 {
                lines.push(std::mem::take(&mut cur));
                cur_w = 0.0;
            }
            // palabras con su espacio delante
            let mut words: Vec<String> = vec![];
            let mut w = String::new();
            for ch in para.chars() {
                if ch == ' ' && !w.trim().is_empty() {
                    words.push(std::mem::take(&mut w));
                }
                w.push(ch);
            }
            if !w.is_empty() {
                words.push(w);
            }
            for word in words {
                let ww = width(&word, size, r.bold);
                if cur_w + ww > max && cur_w > 0.0 {
                    lines.push(std::mem::take(&mut cur));
                    let trimmed = word.trim_start().to_string();
                    cur_w = 0.0;
                    // palabra larguísima: por caracteres
                    if width(&trimmed, size, r.bold) > max {
                        let mut piece = String::new();
                        for ch in trimmed.chars() {
                            let cw = width(&ch.to_string(), size, r.bold);
                            if cur_w + cw > max && !piece.is_empty() {
                                push_word(&mut cur, &piece, r);
                                lines.push(std::mem::take(&mut cur));
                                piece.clear();
                                cur_w = 0.0;
                            }
                            piece.push(ch);
                            cur_w += cw;
                        }
                        push_word(&mut cur, &piece, r);
                    } else {
                        cur_w = width(&trimmed, size, r.bold);
                        push_word(&mut cur, &trimmed, r);
                    }
                } else {
                    let word = if cur_w == 0.0 { word.trim_start().to_string() } else { word };
                    cur_w += width(&word, size, r.bold);
                    push_word(&mut cur, &word, r);
                }
            }
        }
    }
    lines.push(cur);
    lines
}

fn to_runs(t: &str) -> Vec<Run> {
    runs(t).into_iter().map(|(t, bold, ital)| Run { t, bold, ital }).collect()
}

struct Pages {
    pages: Vec<String>,
    cur: String,
    y: f64,
}

impl Pages {
    fn new() -> Self {
        Pages { pages: vec![], cur: String::new(), y: MT }
    }
    fn room(&self) -> f64 {
        H - MB - self.y
    }
    fn break_page(&mut self) {
        self.pages.push(std::mem::take(&mut self.cur));
        self.y = MT;
    }
    fn need(&mut self, h: f64) {
        if h > self.room() && self.y > MT + 1.0 {
            self.break_page();
        }
    }
    /// Una línea de trozos con su línea base en `y`.
    fn line(&mut self, x: f64, y: f64, line: &[Run], size: f64, color: &str) {
        let mut cx = x;
        for r in line {
            if r.t.is_empty() {
                continue;
            }
            let w = width(&r.t, size, r.bold);
            let weight = if r.bold { "600" } else { "400" };
            if r.ital {
                // cursiva simulada (Inter no trae cursiva incluida): inclinación de 12°
                let _ = write!(self.cur, r#"<text transform="matrix(1 0 -0.21 1 {:.2} {:.2})" font-family="Inter" font-size="{size}" font-weight="{weight}" fill="{color}" xml:space="preserve">{}</text>"#, cx, y, esc(&r.t));
            } else {
                let _ = write!(self.cur, r#"<text x="{cx:.2}" y="{y:.2}" font-family="Inter" font-size="{size}" font-weight="{weight}" fill="{color}" xml:space="preserve">{}</text>"#, esc(&r.t));
            }
            cx += w;
        }
    }
    /// Párrafo con sangría; devuelve el alto usado.
    fn para(&mut self, runs: &[Run], size: f64, indent: f64, hanging: f64, color: &str, after: f64) {
        let lh = size * 1.42;
        let first = wrap(runs, size, AW - indent);
        // con sangría francesa, la primera línea va más a la izquierda
        let lines = if hanging > 0.0 {
            let mut l = wrap(runs, size, AW - indent);
            if l.len() > 1 {
                let rest: Vec<Run> = l[1..].iter().flat_map(|x| x.iter().cloned().chain(std::iter::once(Run { t: " ".into(), bold: false, ital: false }))).collect();
                let mut out = vec![l.remove(0)];
                out.extend(wrap(&rest, size, AW - indent - hanging));
                out
            } else {
                l
            }
        } else {
            first
        };
        for (k, l) in lines.iter().enumerate() {
            self.need(lh);
            let x = ML + indent + if k > 0 { hanging } else { 0.0 };
            self.line(x, self.y + size, l, size, color);
            self.y += lh;
        }
        self.y += after;
    }
    fn rule(&mut self, x1: f64, x2: f64, y: f64, w: f64, color: &str) {
        let _ = write!(self.cur, r#"<line x1="{x1:.2}" y1="{y:.2}" x2="{x2:.2}" y2="{y:.2}" stroke="{color}" stroke-width="{w}"/>"#);
    }
}

/// Un SVG completo puesto en (x, y) con el tamaño (w, h).
fn nest(svg: &str, x: f64, y: f64, w: f64, h: f64) -> String {
    let start = match svg.find("<svg") {
        Some(p) => p,
        None => return String::new(),
    };
    let body = &svg[start..];
    let end = body.find('>').unwrap_or(0);
    let tag = &body[..end];
    let (ow, oh) = svg_size(svg);
    // quita width, height, x, y del elemento raíz y pone los nuevos
    let mut attrs = String::new();
    let mut rest = &tag[4..];
    let mut has_vb = false;
    while let Some(eq) = rest.find("=\"") {
        let name = rest[..eq].trim().to_string();
        let after = &rest[eq + 2..];
        let close = after.find('"').unwrap_or(after.len());
        let val = &after[..close];
        if name == "viewBox" {
            has_vb = true;
        }
        if !matches!(name.as_str(), "width" | "height" | "x" | "y") && !name.is_empty() {
            let _ = write!(attrs, r#" {name}="{val}""#);
        }
        rest = &after[(close + 1).min(after.len())..];
    }
    if !has_vb {
        let _ = write!(attrs, r#" viewBox="0 0 {ow} {oh}""#);
    }
    format!(r#"<svg x="{x:.2}" y="{y:.2}" width="{w:.2}" height="{h:.2}"{attrs}>{}"#, &body[end + 1..])
}

fn heading(p: &mut Pages, level: u8, t: &str) {
    let (size, before, after) = match level {
        1 => (15.5, 18.0, 8.0),
        2 => (12.5, 12.0, 5.0),
        _ => (11.0, 9.0, 4.0),
    };
    // el título no queda solo al final de la página
    p.need(before + size * 1.4 + 40.0);
    if p.y > MT + 1.0 {
        p.y += before;
    }
    let mut runs = to_runs(t);
    for r in &mut runs {
        r.bold = true;
        if level == 3 {
            r.ital = true;
        }
    }
    p.para(&runs, size, 0.0, 0.0, INK, 0.0);
    if level == 1 {
        let y = p.y + 1.0;
        p.rule(ML, W - MR, y, 0.6, RULE);
        p.y += 4.0;
    }
    p.y += after;
}

fn caption(p: &mut Pages, label: &str, title: &str, min_after: f64) {
    p.need(36.0 + min_after);
    p.y += 6.0;
    p.para(&[Run { t: label.into(), bold: true, ital: false }], 10.0, 0.0, 0.0, INK, 0.0);
    if !title.is_empty() {
        p.para(&[Run { t: title.into(), bold: false, ital: true }], 10.0, 0.0, 0.0, INK, 3.0);
    }
}

fn table(p: &mut Pages, t: &Table, n: usize, tr: &dyn Fn(&str) -> String) {
    let ncol = t.header.len().max(t.rows.iter().map(|r| r.len()).max().unwrap_or(0)).max(1);
    let pad = 5.0;
    // tamaño de letra: el mayor que deja la tabla en el ancho de la página
    let natural_with = |size: f64, whole: bool| -> Vec<f64> {
        (0..ncol)
            .map(|j| {
                // si no cabe, el encabezado puede ocupar varias líneas: cuenta su palabra más larga
                let hw = t.header.get(j).map(|h| if whole { width(h, size, true) } else { h.split_whitespace().map(|w| width(w, size, true)).fold(0.0, f64::max) }).unwrap_or(0.0);
                let cw = t.rows.iter().filter_map(|r| r.get(j)).map(|c| width(c, size, false)).fold(0.0, f64::max);
                hw.max(cw) + 2.0 * pad
            })
            .collect()
    };
    let natural = |size: f64| -> Vec<f64> {
        let w = natural_with(size, true);
        if w.iter().sum::<f64>() <= AW { w } else { natural_with(size, false) }
    };
    let mut size = 8.8;
    let mut widths = natural(size);
    while widths.iter().sum::<f64>() > AW && size > 6.0 {
        size -= 0.4;
        widths = natural(size);
    }
    let total: f64 = widths.iter().sum();
    if total > AW {
        // reparte: las columnas angostas conservan su ancho, las anchas se ajustan
        let fair = AW / ncol as f64;
        let small: f64 = widths.iter().filter(|w| **w <= fair).sum();
        let big: f64 = widths.iter().filter(|w| **w > fair).sum();
        let left = (AW - small).max(0.0);
        for w in &mut widths {
            if *w > fair {
                *w = (*w / big * left).max(28.0);
            }
        }
    }
    let lh = size * 1.32;
    let cell_lines = |txt: &str, j: usize, bold: bool| -> Vec<Vec<Run>> { wrap(&[Run { t: txt.to_string(), bold, ital: false }], size, (widths[j] - 2.0 * pad).max(8.0)) };
    let header_lines: Vec<Vec<Vec<Run>>> = (0..ncol).map(|j| cell_lines(t.header.get(j).map(|x| x.as_str()).unwrap_or(""), j, true)).collect();
    let hh = header_lines.iter().map(|l| l.len()).max().unwrap_or(1) as f64 * lh + 2.0 * 3.0;
    let x0 = ML;
    let tw: f64 = widths.iter().sum();
    let draw_header = |p: &mut Pages| {
        let top = p.y;
        p.rule(x0, x0 + tw, top, 1.0, INK);
        let mut x = x0;
        for j in 0..ncol {
            let num = t.numeric.get(j) == Some(&true);
            for (k, l) in header_lines[j].iter().enumerate() {
                let lw: f64 = l.iter().map(|r| width(&r.t, size, true)).sum();
                let lx = if num { x + widths[j] - pad - lw } else { x + pad };
                p.line(lx, top + 3.0 + size + k as f64 * lh, l, size, INK);
            }
            x += widths[j];
        }
        p.y = top + hh;
        p.rule(x0, x0 + tw, p.y, 0.6, INK);
    };
    let first_row_h = t.rows.first().map(|r| (0..ncol).map(|j| cell_lines(r.get(j).map(|x| x.as_str()).unwrap_or(""), j, false).len()).max().unwrap_or(1) as f64 * lh + 5.0).unwrap_or(lh);
    caption(p, &format!("{} {n}", tr("Tabla")), &t.title, hh + first_row_h);
    draw_header(p);
    for (i, r) in t.rows.iter().enumerate() {
        let strong = t.strong_rows.contains(&i);
        let lines: Vec<Vec<Vec<Run>>> = (0..ncol).map(|j| cell_lines(r.get(j).map(|x| x.as_str()).unwrap_or(""), j, strong)).collect();
        let rh = lines.iter().map(|l| l.len()).max().unwrap_or(1) as f64 * lh + 5.0;
        if rh > p.room() {
            p.rule(x0, x0 + tw, p.y, 1.0, INK);
            p.break_page();
            draw_header(p);
        }
        let top = p.y;
        let mut x = x0;
        for j in 0..ncol {
            let num = t.numeric.get(j) == Some(&true);
            for (k, l) in lines[j].iter().enumerate() {
                let lw: f64 = l.iter().map(|x| width(&x.t, size, x.bold)).sum();
                let lx = if num { x + widths[j] - pad - lw } else { x + pad };
                p.line(lx, top + 2.5 + size + k as f64 * lh, l, size, INK);
            }
            x += widths[j];
        }
        p.y = top + rh;
        if i + 1 < t.rows.len() {
            p.rule(x0, x0 + tw, p.y, 0.3, RULE);
        }
    }
    p.rule(x0, x0 + tw, p.y, 1.0, INK);
    p.y += 4.0;
    if !t.note.is_empty() {
        let mut runs = vec![Run { t: format!("{}. ", tr("Nota")), bold: false, ital: true }];
        runs.extend(to_runs(&t.note));
        p.para(&runs, 8.5, 0.0, 0.0, INK2, 2.0);
    }
    p.y += 8.0;
}

/// PDF del documento.
pub fn pdf(d: &Doc, tr: &dyn Fn(&str) -> String) -> Result<Vec<u8>, String> {
    let mut p = Pages::new();
    // portada (en la primera página, arriba)
    p.para(&[Run { t: d.title.clone(), bold: true, ital: false }], 22.0, 0.0, 0.0, INK, 2.0);
    p.para(&[Run { t: d.subtitle.clone(), bold: false, ital: false }], 13.0, 0.0, 0.0, INK2, 10.0);
    for (k, v) in &d.meta {
        p.para(&[Run { t: format!("{k}: "), bold: true, ital: false }, Run { t: v.clone(), bold: false, ital: false }], 9.5, 0.0, 0.0, INK2, 0.0);
    }
    p.y += 8.0;
    let (mut nt, mut nf) = (0, 0);
    for (bi, b) in d.blocks.iter().enumerate() {
        match b {
            Item::Heading(l, t) => {
                // el título va con el comienzo de lo que sigue (no queda solo al pie)
                let next = match d.blocks.get(bi + 1) {
                    Some(Item::Figure { svg, .. }) => {
                        let (w, h) = svg_size(svg);
                        h * (AW / w).min(H * 0.5 / h).min(1.0) + 40.0
                    }
                    Some(Item::Table(_)) => 90.0,
                    Some(Item::Heading(..)) => 70.0,
                    _ => 40.0,
                };
                p.need(next + 30.0);
                heading(&mut p, *l, t)
            }
            Item::Para(t) => p.para(&to_runs(t), 10.0, 0.0, 0.0, INK, 6.0),
            Item::Callout(t) => {
                let mut runs = to_runs(t);
                for r in &mut runs {
                    r.bold = true;
                }
                let lines = wrap(&runs, 10.5, AW - 22.0);
                let h = lines.len() as f64 * 10.5 * 1.42 + 12.0;
                p.need(h);
                let top = p.y;
                let _ = write!(p.cur, r#"<rect x="{ML}" y="{top:.2}" width="{AW}" height="{h:.2}" rx="3" fill="{SOFT}"/><rect x="{ML}" y="{top:.2}" width="3" height="{h:.2}" fill="{ACCENT}"/>"#);
                p.y += 6.0;
                for l in &lines {
                    p.line(ML + 14.0, p.y + 10.5, l, 10.5, INK);
                    p.y += 10.5 * 1.42;
                }
                p.y = top + h + 8.0;
            }
            Item::Metrics(m) => {
                let kw = m.iter().map(|(k, _)| width(k, 9.5, false)).fold(0.0, f64::max).min(AW * 0.5) + 14.0;
                for (k, v) in m {
                    let kl = wrap(&[Run { t: k.clone(), bold: false, ital: false }], 9.5, kw - 12.0);
                    let vl = wrap(&[Run { t: v.clone(), bold: true, ital: false }], 9.5, AW - kw - 12.0);
                    let n = kl.len().max(vl.len());
                    let lh = 9.5 * 1.45;
                    p.need(n as f64 * lh);
                    let top = p.y;
                    for (i, l) in kl.iter().enumerate() {
                        p.line(ML + 12.0, top + 9.5 + i as f64 * lh, l, 9.5, INK2);
                    }
                    for (i, l) in vl.iter().enumerate() {
                        p.line(ML + 12.0 + kw, top + 9.5 + i as f64 * lh, l, 9.5, INK);
                    }
                    p.y = top + n as f64 * lh;
                }
                p.y += 6.0;
            }
            Item::Formula(lines) => {
                for l in lines {
                    p.para(&[Run { t: l.clone(), bold: false, ital: false }], 9.8, 18.0, 0.0, INK, 0.0);
                }
                p.y += 6.0;
            }
            Item::Table(t) => {
                nt += 1;
                table(&mut p, t, nt, tr);
            }
            Item::Figure { title, svg } => {
                nf += 1;
                let (w, h) = svg_size(svg);
                let scale = (AW / w).min(H * 0.5 / h).min(1.0);
                let (fw, fh) = (w * scale, h * scale);
                caption(&mut p, &format!("{} {nf}", tr("Figura")), title, fh);
                p.need(fh);
                let svg_in = nest(svg, ML + (AW - fw) / 2.0, p.y, fw, fh);
                p.cur.push_str(&svg_in);
                p.y += fh + 12.0;
            }
        }
    }
    if !d.data_source.is_empty() {
        heading(&mut p, 1, &tr("Origen de los datos"));
        p.para(&to_runs(&d.data_source), 10.0, 0.0, 0.0, INK, 6.0);
    }
    if !d.references.is_empty() {
        heading(&mut p, 1, &tr("Referencias"));
        for r in &d.references {
            p.para(&to_runs(r), 9.8, 0.0, 24.0, INK, 6.0);
        }
    }
    p.pages.push(std::mem::take(&mut p.cur));
    let total = p.pages.len();
    let opt = {
        let mut o = resvg::usvg::Options::default();
        o.fontdb = crate::fonts();
        o.font_family = crate::FONT.into();
        o
    };
    use pdf_writer::{Content, Finish, Name, Pdf, Rect, Ref, TextStr};
    let mut alloc = Ref::new(1);
    let catalog = alloc.bump();
    let tree_id = alloc.bump();
    let info = alloc.bump();
    let mut pdf = Pdf::new();
    let mut page_ids = vec![];
    let mut chunks = vec![];
    for (k, body) in p.pages.iter().enumerate() {
        let foot = format!(
            r#"<line x1="{ML}" y1="{y0:.2}" x2="{x2:.2}" y2="{y0:.2}" stroke="{RULE}" stroke-width="0.5"/><text x="{ML}" y="{y:.2}" font-family="Inter" font-size="7.5" fill="{INK2}">{}</text><text x="{x2:.2}" y="{y:.2}" text-anchor="end" font-family="Inter" font-size="7.5" fill="{INK2}">{}</text>"#,
            esc(&d.footer),
            esc(&tr(&format!("Página {} de {}", k + 1, total))),
            y0 = H - MB + 22.0,
            y = H - MB + 34.0,
            x2 = W - MR
        );
        let svg = format!(r#"<svg xmlns="http://www.w3.org/2000/svg" width="{W}" height="{H}" viewBox="0 0 {W} {H}"><rect width="{W}" height="{H}" fill="white"/>{body}{foot}</svg>"#);
        let tree = resvg::usvg::Tree::from_str(&svg, &opt).map_err(|e| format!("página {} del PDF: {e}", k + 1))?;
        let (chunk, id) = svg2pdf::to_chunk(&tree, svg2pdf::ConversionOptions::default()).map_err(|e| format!("página {} del PDF: {e:?}", k + 1))?;
        let mut map = std::collections::HashMap::new();
        let chunk = chunk.renumber(|old| *map.entry(old).or_insert_with(|| alloc.bump()));
        let svg_id = *map.get(&id).ok_or("PDF: referencia perdida")?;
        let page_id = alloc.bump();
        let content_id = alloc.bump();
        page_ids.push(page_id);
        let name = Name(b"P0");
        {
            let mut page = pdf.page(page_id);
            page.media_box(Rect::new(0.0, 0.0, W as f32, H as f32));
            page.parent(tree_id);
            page.contents(content_id);
            page.resources().x_objects().pair(name, svg_id);
            page.finish();
        }
        let mut c = Content::new();
        c.transform([W as f32, 0.0, 0.0, H as f32, 0.0, 0.0]);
        c.x_object(name);
        pdf.stream(content_id, &c.finish());
        chunks.push(chunk);
    }
    for c in &chunks {
        pdf.extend(c);
    }
    pdf.pages(tree_id).kids(page_ids.iter().copied()).count(page_ids.len() as i32);
    pdf.catalog(catalog).pages(tree_id);
    {
        let mut i = pdf.document_info(info);
        i.title(TextStr(&d.title));
        i.creator(TextStr("IAQSB"));
        i.producer(TextStr("IAQSB (svg2pdf)"));
        if let Some((_, a)) = d.meta.first().filter(|m| m.0 == tr("Autor")) {
            i.author(TextStr(a));
        }
    }
    Ok(pdf.finish())
}
