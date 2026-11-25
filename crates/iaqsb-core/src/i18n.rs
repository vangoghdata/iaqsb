//! Idiomas: IAQSB se escribe en español y se traduce al inglés o al portugués
//! (de Brasil) al final.
//!
//! El catálogo de cada idioma (`i18n/en.json`, `i18n/pt.json`) lleva cada texto del núcleo (literales del código
//! y de los datos) con su traducción. Los textos con valores son plantillas:
//! `"Pedir {0} unidades cada {1} {2}"` → `"Order {0} units every {1} {2}"`.
//! Para traducir un texto se busca primero tal cual y después la plantilla más
//! específica que encaje; lo que llena cada marcador se traduce también (así se
//! traducen textos armados a partir de varios trozos).
//!
//! La API traduce lo que devuelve con [`translate_value`]: informes, ejemplos,
//! especificaciones y fuentes, sin tocar ids, claves ni códigos.

use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Lang {
    #[default]
    Es,
    En,
    /// Portugués de Brasil.
    Pt,
}

impl Lang {
    /// "en", "en-US", "english" → inglés; "pt", "pt-BR", "português" → portugués;
    /// cualquier otra cosa → español.
    pub fn parse(s: Option<&str>) -> Lang {
        match s.map(|x| x.trim().to_lowercase()) {
            Some(x) if x.starts_with("en") || x.starts_with("in") => Lang::En,
            Some(x) if x.starts_with("pt") || x.starts_with("por") || x == "br" => Lang::Pt,
            _ => Lang::Es,
        }
    }
    pub fn code(self) -> &'static str {
        match self {
            Lang::Es => "es",
            Lang::En => "en",
            Lang::Pt => "pt",
        }
    }
    /// Los idiomas de IAQSB.
    pub const ALL: [Lang; 3] = [Lang::Es, Lang::En, Lang::Pt];
}

thread_local! {
    /// Idioma de la traducción en curso (cada hilo traduce a un idioma a la vez).
    static CUR: std::cell::Cell<Lang> = const { std::cell::Cell::new(Lang::En) };
}

fn cur() -> Lang {
    CUR.with(|c| c.get())
}

/// Carga el catálogo de un idioma (en otro hilo, para que la primera traducción no espere).
pub fn warm_up() {
    for l in [Lang::En, Lang::Pt] {
        let _ = catalog_for(l);
    }
}

struct Tpl {
    /// Trozos fijos: n marcadores ⇒ n + 1 trozos (pueden estar vacíos).
    segs: Vec<String>,
    en: String,
    /// Largo de lo fijo (para preferir la plantilla más específica).
    score: usize,
}

struct Catalog {
    exact: HashMap<String, String>,
    /// Textos fijos de 4 o más letras, del más largo al más corto (para el último recurso).
    pieces: Vec<String>,
    /// Índice de `pieces` por su primera palabra (para no recorrerlas todas con cada texto).
    piece_idx: HashMap<String, Vec<usize>>,
    /// Palabras sueltas en español del catálogo (en minúsculas).
    vocab: std::collections::HashSet<String>,
    tpls: Vec<Tpl>,
    by_prefix: HashMap<String, Vec<usize>>,
    by_suffix: HashMap<String, Vec<usize>>,
    rest: Vec<usize>,
}

const K: usize = 3;
/// Puntuación y espacios de los bordes que no cambian la traducción.
const EDGE: &[char] = &[' ', ';', ',', ':', '.', '·', '—', '–', '-', '\n', '(', ')'];

fn head(s: &str, n: usize) -> String {
    s.chars().take(n).collect()
}
fn tail(s: &str, n: usize) -> String {
    let c: Vec<char> = s.chars().collect();
    c[c.len().saturating_sub(n)..].iter().collect()
}

fn split_tpl(es: &str) -> Option<Vec<String>> {
    let mut segs = vec![];
    let mut cur = String::new();
    let mut k = 0;
    let b = es.as_bytes();
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'{' {
            let j = es[i..].find('}')? + i;
            if es[i + 1..j] == k.to_string() {
                segs.push(std::mem::take(&mut cur));
                k += 1;
                i = j + 1;
                continue;
            }
        }
        let ch = es[i..].chars().next().unwrap();
        cur.push(ch);
        i += ch.len_utf8();
    }
    segs.push(cur);
    if k == 0 {
        None
    } else {
        Some(segs)
    }
}

fn catalog() -> &'static Catalog {
    catalog_for(cur())
}

fn catalog_for(lang: Lang) -> &'static Catalog {
    static EN: OnceLock<Catalog> = OnceLock::new();
    static PT: OnceLock<Catalog> = OnceLock::new();
    match lang {
        Lang::Pt => PT.get_or_init(|| build(include_str!("../i18n/pt.json"), include_str!("../i18n/pt_extra.json"))),
        _ => EN.get_or_init(|| build(include_str!("../i18n/en.json"), include_str!("../i18n/en_extra.json"))),
    }
}

fn build(main: &str, extra: &str) -> Catalog {
    {
        let raw: HashMap<String, String> = serde_json::from_str(main).expect("catálogo de traducción mal formado");
        let extra: HashMap<String, String> = serde_json::from_str(extra).expect("catálogo de traducción (extra) mal formado");
        let mut c = Catalog { exact: HashMap::new(), pieces: vec![], piece_idx: HashMap::new(), vocab: Default::default(), tpls: vec![], by_prefix: HashMap::new(), by_suffix: HashMap::new(), rest: vec![] };
        // variantes sin la puntuación de los bordes («; costo total {0}» → «costo total {0}»)
        let mut all: Vec<(String, String)> = raw.into_iter().chain(extra).collect();
        let keys: std::collections::HashSet<String> = all.iter().map(|x| x.0.clone()).collect();
        let mut alts = vec![];
        for (es, en) in &all {
            let (a, b) = (es.trim_matches(EDGE), en.trim_matches(EDGE));
            if a.len() != es.len() && !a.is_empty() && !keys.contains(a) && has_letters(a) {
                alts.push((a.to_string(), b.to_string()));
            }
        }
        all.extend(alts);
        // textos de varias líneas: también cada línea por separado
        let renum = |t: &str, order: &[usize]| -> String {
            let mut out = t.to_string();
            for (new, old) in order.iter().enumerate() {
                out = out.replace(&format!("{{{old}}}"), &format!("{{\u{1}{new}}}"));
            }
            out.replace('\u{1}', "")
        };
        let mut lines = vec![];
        for (es, en) in &all {
            if !es.contains('\n') {
                continue;
            }
            let (a, b): (Vec<&str>, Vec<&str>) = (es.split('\n').collect(), en.split('\n').collect());
            if a.len() != b.len() {
                continue;
            }
            for (x, y) in a.iter().zip(&b) {
                let x = x.trim_end_matches(':').trim();
                let y = y.trim_end_matches(':').trim();
                if x.is_empty() || !has_letters(x) {
                    continue;
                }
                // marcadores de la línea, renumerados desde 0
                let order: Vec<usize> = {
                    let mut v = vec![];
                    let mut rest = x;
                    while let Some(p) = rest.find('{') {
                        let Some(q) = rest[p..].find('}').map(|q| p + q) else { break };
                        if let Ok(k) = rest[p + 1..q].parse::<usize>() {
                            v.push(k);
                        }
                        rest = &rest[q + 1..];
                    }
                    v
                };
                lines.push((renum(x, &order), renum(y, &order)));
            }
        }
        all.extend(lines);
        // la misma frase con la primera letra en mayúscula o en minúscula
        let keys: std::collections::HashSet<String> = all.iter().map(|x| x.0.clone()).collect();
        let mut cases = vec![];
        let flip = |s: &str| -> Option<String> {
            let mut ch = s.chars();
            let f = ch.next()?;
            if !f.is_alphabetic() {
                return None;
            }
            let g: String = if f.is_uppercase() { f.to_lowercase().collect() } else { f.to_uppercase().collect() };
            Some(format!("{g}{}", ch.as_str()))
        };
        for (es, en) in &all {
            if let (Some(a), Some(b)) = (flip(es), flip(en)) {
                // no se cambian siglas («EOQ», «PERT»)
                if es.chars().nth(1).map_or(false, |c| c.is_lowercase()) && !keys.contains(&a) {
                    cases.push((a, b));
                }
            }
        }
        all.extend(cases);
        for (es, en) in all {
            match split_tpl(&es) {
                None => {
                    c.exact.insert(es, en);
                }
                Some(segs) => {
                    let score = segs.iter().map(|s| s.chars().filter(|c| !c.is_whitespace()).count()).sum::<usize>();
                    // plantillas sin letras fijas no traducen nada
                    if !segs.iter().any(|s| s.chars().filter(|c| c.is_alphabetic()).count() >= 2) {
                        continue;
                    }
                    let i = c.tpls.len();
                    let first = segs[0].clone();
                    let last = segs[segs.len() - 1].clone();
                    if !first.is_empty() {
                        c.by_prefix.entry(head(&first, K)).or_default().push(i);
                    } else if !last.is_empty() {
                        c.by_suffix.entry(tail(&last, K)).or_default().push(i);
                    } else {
                        c.rest.push(i);
                    }
                    c.tpls.push(Tpl { segs, en, score });
                }
            }
        }
        // solo frases de dos o más palabras: una palabra suelta puede ser parte de un nombre
        let accent = |k: &str| k.chars().any(|c| matches!(c, 'á' | 'é' | 'í' | 'ó' | 'ú' | 'ñ'));
        let mut pieces: Vec<String> = c
            .exact
            .iter()
            .filter(|(k, v)| k != v && (k.trim().contains(' ') && k.chars().filter(|c| c.is_alphabetic()).count() >= 6 || accent(k) && k.chars().next().map_or(false, |c| c.is_lowercase())) && !k.contains('\n'))
            .map(|(k, _)| k.clone())
            .collect();
        pieces.sort_by(|a, b| b.chars().count().cmp(&a.chars().count()).then(a.cmp(b)));
        for (i, k) in pieces.iter().enumerate() {
            if let Some(w) = tokens(k).next() {
                c.piece_idx.entry(w.to_string()).or_default().push(i);
            }
        }
        c.pieces = pieces;
        // palabras que solo aparecen en el lado español del catálogo
        let words = |t: &str| -> Vec<String> { t.split(|ch: char| !ch.is_alphabetic()).filter(|w| w.chars().count() >= 3).map(|w| w.to_lowercase()).collect() };
        let mut es_w: std::collections::HashSet<String> = Default::default();
        let mut en_w: std::collections::HashSet<String> = Default::default();
        for (k, v) in c.exact.iter().chain(c.tpls.iter().map(|t| (&t.en, &t.en)).filter(|_| false)) {
            es_w.extend(words(k));
            en_w.extend(words(v));
        }
        for t in &c.tpls {
            es_w.extend(t.segs.iter().flat_map(|x| words(x)));
            en_w.extend(words(&t.en));
        }
        c.vocab = es_w.difference(&en_w).cloned().collect();
        // las más específicas primero
        let tp = &c.tpls;
        for v in c.by_prefix.values_mut().chain(c.by_suffix.values_mut()) {
            v.sort_by(|a, b| tp[*b].score.cmp(&tp[*a].score));
        }
        c.rest.sort_by(|a, b| tp[*b].score.cmp(&tp[*a].score));
        c
    }
}

/// Todas las formas (hasta un límite) de encajar `s` en los trozos fijos;
/// cada una da lo que llena cada marcador.
fn fits<'a>(s: &'a str, segs: &[String]) -> Vec<Vec<&'a str>> {
    let first = &segs[0];
    if !s.starts_with(first.as_str()) {
        return vec![];
    }
    let last = &segs[segs.len() - 1];
    if !s[first.len()..].ends_with(last.as_str()) {
        return vec![];
    }
    let mut out = vec![];
    let mut caps = Vec::with_capacity(segs.len() - 1);
    let mut budget = 400;
    go(s, segs, 1, first.len(), &mut caps, &mut out, &mut budget);
    out
}

fn go<'a>(s: &'a str, segs: &[String], k: usize, pos: usize, caps: &mut Vec<&'a str>, out: &mut Vec<Vec<&'a str>>, budget: &mut i32) {
    *budget -= 1;
    if *budget < 0 || out.len() >= 12 {
        return;
    }
    let seg = &segs[k];
    if k == segs.len() - 1 {
        let end = s.len() - seg.len();
        if end >= pos {
            caps.push(&s[pos..end]);
            out.push(caps.clone());
            caps.pop();
        }
        return;
    }
    if seg.is_empty() {
        // marcadores seguidos: se prueba cortar en cada borde de palabra
        let ends = s.len() - segs[segs.len() - 1].len();
        let mut cuts = vec![pos];
        for (i, ch) in s[pos..ends.max(pos)].char_indices() {
            let at = pos + i;
            if at > pos && (!ch.is_alphanumeric() || !s[..at].chars().last().map_or(false, |c| c.is_alphanumeric())) {
                cuts.push(at);
                if cuts.len() >= 24 {
                    break;
                }
            }
        }
        for at in cuts {
            caps.push(&s[pos..at]);
            go(s, segs, k + 1, at, caps, out, budget);
            caps.pop();
        }
        return;
    }
    let mut from = pos;
    while let Some(off) = s[from..].find(seg.as_str()) {
        let at = from + off;
        caps.push(&s[pos..at]);
        go(s, segs, k + 1, at + seg.len(), caps, out, budget);
        caps.pop();
        from = at + s[at..].chars().next().map(|c| c.len_utf8()).unwrap_or(1);
        if from > s.len() {
            break;
        }
    }
}

/// Identificadores como «barberia_capacidad» o «wagner_whitin».
fn is_code(s: &str) -> bool {
    s.contains('_') && !s.contains(' ') || s.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-') && s.contains('-')
}

fn has_letters(s: &str) -> bool {
    s.chars().filter(|c| c.is_alphabetic()).take(2).count() >= 2 || s.chars().any(|c| matches!(c, 'á' | 'é' | 'í' | 'ó' | 'ú' | 'ñ'))
}

fn tr(s: &str, depth: usize) -> String {
    if !has_letters(s) {
        return s.to_string();
    }
    thread_local! {
        // texto → (profundidad a la que se tradujo, traducción); sirve para
        // cualquier pedido igual o más profundo (que tendría menos margen)
        static RAW: std::cell::RefCell<HashMap<String, (usize, String)>> = std::cell::RefCell::new(HashMap::new());
    }
    let key = format!("{}\u{0}{s}", cur().code());
    if let Some(x) = RAW.with(|m| m.borrow().get(&key).filter(|(d, _)| *d <= depth).map(|x| x.1.clone())) {
        return x;
    }
    let out = tr_raw(s, depth);
    // si se agotó el margen, el resultado puede estar a medias: no se guarda
    if spent() {
        return out;
    }
    RAW.with(|m| {
        let mut m = m.borrow_mut();
        if m.len() > 100_000 {
            m.clear();
        }
        m.insert(key, (depth, out.clone()));
    });
    out
}

fn tr_raw(s: &str, depth: usize) -> String {
    let c = catalog();
    if let Some(e) = c.exact.get(s) {
        return e.clone();
    }
    if depth >= 12 || spent() {
        return s.to_string();
    }
    // varias líneas: línea por línea
    if s.contains('\n') {
        return s.split('\n').map(|l| tr(l, depth + 1)).collect::<Vec<_>>().join("\n");
    }
    // se prueban varias formas y queda la que deja menos español
    let mut best: Option<(usize, String)> = None;
    // puntuación y espacios alrededor
    let t = s.trim_matches(EDGE);
    if t.len() != s.len() && !t.is_empty() && has_letters(t) {
        let a = &s[..s.find(t).unwrap_or(0)];
        let b = &s[a.len() + t.len()..];
        if let Some(e) = c.exact.get(t) {
            return format!("{a}{e}{b}");
        }
        if let Some(x) = by_template(t, depth) {
            keep(&mut best, format!("{a}{x}{b}"));
        }
    }
    if let Some(x) = by_template(s, depth) {
        keep(&mut best, x);
    }
    if best.as_ref().map_or(false, |b| b.0 == 0) {
        return best.unwrap().1;
    }
    // un texto del catálogo seguido de más frases («… restricciones. El modelo tiene …»)
    let cuts: Vec<usize> = s.match_indices(". ").map(|(i, _)| i).collect();
    if depth == 0 {
        WORK.with(|w| w.set(0)); // cada estrategia tiene su propio margen
    }
    if cuts.len() > 1 {
        for &k in cuts.iter().rev().take(8) {
            let (left, right) = (&s[..=k], &s[k + 2..]);
            let l = c.exact.get(left).cloned().or_else(|| by_template(left, depth + 1));
            if let Some(l) = l {
                let clean = spanish_count(&l) == 0;
                if depth == 0 {
                    WORK.with(|w| w.set(0));
                }
                keep(&mut best, format!("{l} {}", tr(right, depth + 1)));
                if clean {
                    break;
                }
            }
        }
        if best.as_ref().map_or(false, |b| b.0 == 0) {
            return best.unwrap().1;
        }
    }
    if depth == 0 {
        WORK.with(|w| w.set(0));
    }
    // varias frases sueltas
    {
        for sep in ["; ", " · ", " — ", ". ", ": ", ", ", " → "] {
            if s.contains(sep) {
                // «Título: texto» se parte solo en el primer «: »
                let parts: Vec<&str> = if sep == ": " { s.splitn(2, sep).collect() } else { s.split(sep).collect() };
                let out: Vec<String> = parts
                    .iter()
                    .enumerate()
                    .map(|(i, p)| {
                        // al partir por ". " se pierde el punto final de cada frase
                        if sep == ". " && i + 1 < parts.len() {
                            let x = tr(&format!("{p}."), depth + 1);
                            x.strip_suffix('.').map(|y| y.to_string()).unwrap_or(x)
                        } else {
                            tr(p, depth + 1)
                        }
                    })
                    .collect();
                keep(&mut best, out.join(sep));
                if best.as_ref().map_or(false, |b| b.0 == 0) {
                    break;
                }
            }
        }
    }
    // «Texto:» o «Texto.» con el final distinto
    for p in [":", ".", ")"] {
        if let Some(core) = s.strip_suffix(p) {
            if let Some(e) = c.exact.get(core) {
                keep(&mut best, format!("{e}{p}"));
            }
        }
    }
    // último recurso: reemplazar los trozos conocidos que aparezcan dentro
    if best.as_ref().map_or(true, |b| b.0 > 0) {
        if let Some(x) = by_pieces(best.as_ref().map(|b| b.1.as_str()).unwrap_or(s)) {
            keep(&mut best, x);
        }
    }
    match best {
        Some((r, x)) if r < spanish_count(s) || x != s && r == 0 => x,
        Some((_, x)) if x != s => x,
        _ => s.to_string(),
    }
}

fn keep(best: &mut Option<(usize, String)>, x: String) {
    let r = spanish_count(&x);
    if best.as_ref().map_or(true, |b| r < b.0) {
        *best = Some((r, x));
    }
}

fn boundary(s: &str, at: usize, len: usize) -> bool {
    let w = |c: char| c.is_alphanumeric() || c == '_';
    let before = s[..at].chars().last();
    let after = s[at + len..].chars().next();
    !before.map_or(false, w) && !after.map_or(false, w)
}

/// Palabras de un texto (letras y dígitos seguidos).
fn tokens(s: &str) -> impl Iterator<Item = &str> {
    s.split(|ch: char| !ch.is_alphanumeric()).filter(|w| !w.is_empty())
}

fn by_pieces(s: &str) -> Option<String> {
    let c = catalog();
    let n = s.chars().count();
    let mut out = s.to_string();
    let mut changed = false;
    // solo las frases cuya primera palabra está en el texto (en el orden de `pieces`: las largas primero)
    let mut cand: Vec<usize> = tokens(s).filter_map(|w| c.piece_idx.get(w)).flatten().copied().collect();
    cand.sort_unstable();
    cand.dedup();
    // se marcan con \u{1} los trozos ya traducidos para no volver a tocarlos
    for k in cand.iter().map(|&i| &c.pieces[i]) {
        if k.chars().count() > n {
            continue;
        }
        let mut from = 0;
        while let Some(off) = out[from..].find(k.as_str()) {
            let at = from + off;
            if boundary(&out, at, k.len()) && !out[..at].ends_with('\u{1}') {
                let e = format!("\u{1}{}\u{2}", c.exact[k]);
                out.replace_range(at..at + k.len(), &e);
                changed = true;
                from = at + e.len();
            } else {
                from = at + k.len();
            }
            if from >= out.len() {
                break;
            }
        }
    }
    if changed {
        Some(out.replace(['\u{1}', '\u{2}'], ""))
    } else {
        None
    }
}

thread_local! {
    static WORK: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
}

/// Límite de trabajo por texto: si se agota, lo que falta queda sin traducir.
fn spent() -> bool {
    WORK.with(|w| w.get() > 6_000)
}

fn by_template(s: &str, depth: usize) -> Option<String> {
    let c = catalog();
    let mut cands: Vec<usize> = vec![];
    for n in 1..=K {
        let h = head(s, n);
        if let Some(v) = c.by_prefix.get(&h) {
            cands.extend(v);
        }
        let t = tail(s, n);
        if let Some(v) = c.by_suffix.get(&t) {
            cands.extend(v);
        }
    }
    cands.extend(&c.rest);
    cands.sort_by(|a, b| c.tpls[*b].score.cmp(&c.tpls[*a].score));
    cands.dedup();
    // en textos largos solo sirven plantillas que cubran buena parte del texto
    // (lo demás lo resuelve partir en frases)
    let solid = s.chars().filter(|c| !c.is_whitespace()).count();
    if solid > 60 {
        cands.retain(|i| c.tpls[*i].score * 5 >= solid || c.tpls[*i].score >= 20);
    }
    // la mejor: la que deja menos español sin traducir y, a igualdad, la más específica
    let mut best: Option<(usize, usize, String)> = None; // (resto en español, -, texto)
    let mut best_score = 0;
    for i in cands {
        let tp = &c.tpls[i];
        if let Some((rest, _, _)) = &best {
            if *rest == 0 && best_score >= tp.score {
                break;
            }
        }
        for caps in fits(s, &tp.segs) {
            WORK.with(|w| w.set(w.get() + 1));
            if spent() {
                break;
            }
            let tcaps: Vec<String> = caps.iter().map(|x| tr(x, depth + 1)).collect();
            // con poco texto fijo («{0} en {1}») los valores no pueden quedar en español
            if tp.score <= 6 && tcaps.iter().any(|x| spanish_count(x) > 0) {
                continue;
            }
            let rest = tcaps.iter().map(|x| spanish_count(x)).sum::<usize>();
            let better = match &best {
                None => true,
                Some((r, _, _)) => rest < *r || (rest == *r && tp.score > best_score),
            };
            if better {
                let mut out = tp.en.clone();
                for (k, cap) in tcaps.iter().enumerate().rev() {
                    out = out.replace(&format!("{{{k}}}"), cap);
                }
                best = Some((rest, i, out));
                best_score = tp.score;
                if rest == 0 {
                    break;
                }
            }
        }
    }
    best.map(|b| b.2)
}

/// Cuántas palabras en español quedan (palabras de enlace, tildes y palabras
/// sueltas del catálogo que tienen otra traducción).
fn spanish_count(s: &str) -> usize {
    const W: &[&str] = &["de", "del", "el", "la", "los", "las", "que", "en", "y", "con", "por", "para", "se", "es", "al", "un", "una", "sin", "más", "según"];
    let vocab = &catalog().vocab;
    s.split(|c: char| !c.is_alphabetic())
        .filter(|w| !w.is_empty())
        .filter(|w| W.contains(w) || w.chars().count() >= 3 && vocab.contains(&w.to_lowercase()))
        .count()
        + s.chars().filter(|c| matches!(c, 'á' | 'é' | 'í' | 'ó' | 'ú' | 'ñ')).count()
}

fn memo() -> &'static Mutex<HashMap<(Lang, String), String>> {
    static M: OnceLock<Mutex<HashMap<(Lang, String), String>>> = OnceLock::new();
    M.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Ajustes finales del portugués: ediciones («10.ª ed.» → «10. ed.»), citas
/// «A y B» («A e B (2015)», «(A & B, 1964)») y «s. f.» → «s.d.».
fn polish_pt(s: String) -> String {
    let mut s = s.replace(".ª ed.", ". ed.").replace("s. f.", "s.d.");
    if s.contains(" y ") {
        s = cites_y(&s, "e", "&");
    }
    s
}

/// «A y B (2015a)» → «A {narr} B (2015a)»; «A y B, 1964» → «A {paren} B, 1964».
fn cites_y(s: &str, narr: &str, paren_word: &str) -> String {
    let words: Vec<&str> = s.split(' ').collect();
    let mut out: Vec<String> = Vec::with_capacity(words.len());
    for (i, w) in words.iter().enumerate() {
        let cap = |x: Option<&&str>| x.and_then(|w| w.trim_start_matches(['(', '«', '"']).chars().next()).map_or(false, |c| c.is_uppercase());
        let next = words.get(i + 1).copied().unwrap_or("");
        let after = words.get(i + 2).copied().unwrap_or("");
        let yearish = |x: &str| x.trim_start_matches('(').chars().next().map_or(false, |c| c.is_ascii_digit()) || x.trim_start_matches('(').starts_with("s.") || x.trim_start_matches('(').starts_with("n.d.");
        let narrative = after.starts_with('(') && yearish(after);
        let paren = next.ends_with(',') && yearish(after);
        if *w == "y" && i > 0 && cap(words.get(i - 1)) && cap(words.get(i + 1)) && (narrative || paren) {
            out.push(if narrative { narr.into() } else { paren_word.into() });
        } else {
            out.push(w.to_string());
        }
    }
    out.join(" ")
}

/// Ajustes finales del inglés: ordinales de las ediciones y citas «A y B».
fn polish(s: String) -> String {
    let mut s = s;
    if s.contains("th ed") {
        for (a, b) in [("1th ed", "1st ed"), ("2th ed", "2nd ed"), ("3th ed", "3rd ed")] {
            // 11.ª, 12.ª y 13.ª siguen con «th»
            let mut out = String::with_capacity(s.len());
            let mut rest = s.as_str();
            while let Some(p) = rest.find(a) {
                let prev = rest[..p].chars().last();
                out.push_str(&rest[..p]);
                out.push_str(if prev == Some('1') { a } else { b });
                rest = &rest[p + a.len()..];
            }
            out.push_str(rest);
            s = out;
        }
    }
    // «Hillier y Lieberman (2015a)» y «Clarke y Wright, 1964»
    if s.contains(" y ") {
        // solo en citas: «A y B (2015a)» o «A y B, 1964»
        s = cites_y(&s, "and", "&");
    }
    s.replace("s. f.", "n.d.")
}

/// Traduce un texto (o lo deja igual si está en español o no hay traducción).
/// Nombre del programa que ve el usuario (provisional: cuando sea definitivo, se cambia aquí y en
/// `apps/desktop/src/lib/marca.ts`). Los nombres internos (comando `iaqsb`, servidor MCP, formatos
/// de archivo como «IAQSB-ST») no cambian.
pub const MARCA: &str = "Nativo";

/// Cambia «IAQSB» por el nombre del programa (no en «IAQSB-ST», «IAQSB_…» ni dentro de palabras).
pub fn marca(s: &str) -> String {
    if MARCA == "IAQSB" || !s.contains("IAQSB") {
        return s.to_string();
    }
    let mut out = String::with_capacity(s.len());
    let mut resto = s;
    while let Some(i) = resto.find("IAQSB") {
        let antes = resto[..i].chars().next_back();
        let despues = resto[i + 5..].chars().next();
        let palabra = |c: Option<char>| c.is_some_and(|c| c.is_alphanumeric() || c == '_' || c == '-');
        out.push_str(&resto[..i]);
        out.push_str(if palabra(antes) || palabra(despues) { "IAQSB" } else { MARCA });
        resto = &resto[i + 5..];
    }
    out.push_str(resto);
    out
}

pub fn translate(s: &str, lang: Lang) -> String {
    if !has_letters(s) || is_code(s) {
        return s.to_string();
    }
    if lang == Lang::Es {
        return marca(s);
    }
    let key = (lang, s.to_string());
    if let Some(x) = memo().lock().ok().and_then(|m| m.get(&key).cloned()) {
        return x;
    }
    let prev = CUR.with(|c| c.replace(lang));
    WORK.with(|w| w.set(0));
    let raw = tr(s, 0);
    let out = marca(&if lang == Lang::Pt { polish_pt(raw) } else { polish(raw) });
    CUR.with(|c| c.set(prev));
    if let Ok(mut m) = memo().lock() {
        if m.len() > 200_000 {
            m.clear();
        }
        m.insert(key, out.clone());
    }
    out
}

/// Claves cuyo valor es un código o un dato que no se traduce.
const PROTECTED: &[&str] = &[
    "id", "key", "type", "tone", "kind", "module", "problem_type", "status", "align", "url", "doi", "link", "bibtex", "isbn", "file_ext",
    "icon", "direction", "var_type", "origin", "method", "chart", "format", "expression", "function", "content", "path", "lang", "estado",
    "basis_status", "node", "objective", "show_if", "expected",
    // códigos y textos con formato dentro de los modelos
    "space_mode", "review", "discipline", "distribution", "input_rule", "metric", "model_kind", "lot_size", "side", "resource",
    "time_distribution", "service", "interarrival", "values", "created", "delivered", "promised", "date", "time", "cost_function",
    "bom_kind", "rate_unit",
    // series de tiempo: códigos de modelos y de motores, y nombres de periodos
    "recommended_smoothing", "best_training_rmse", "best_test_rmse", "family", "engine", "seasonal_type", "train_end", "test_start", "forecast_engine",
];

/// Campos de los modelos que nombran otros elementos («Cola torno/0.1, Desechos»):
/// se traduce cada nombre por separado, igual que el nombre al que apuntan.
const REFS: &[&str] = &["followers", "components", "operations", "next", "predecessors", "successors", "start", "end", "from", "to"];

fn translate_refs(s: &str, lang: Lang) -> String {
    s.split(", ")
        .map(|tok| {
            tok.split('/')
                .map(|p| if p.trim().parse::<f64>().is_ok() { p.to_string() } else { translate(p, lang) })
                .collect::<Vec<_>>()
                .join("/")
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// Traduce en el sitio todos los textos de un valor JSON (informes, ejemplos,
/// especificaciones, bibliografía), sin tocar las claves protegidas.
pub fn translate_value(v: &mut Value, lang: Lang) {
    walk(v, lang, false);
}

fn walk(v: &mut Value, lang: Lang, protected: bool) {
    match v {
        Value::String(s) if !protected => {
            let t = translate(s, lang);
            if t != *s {
                *s = t;
            }
        }
        Value::Array(a) => {
            for x in a {
                walk(x, lang, protected);
            }
        }
        Value::Object(o) => {
            // referencias: APA y cita en inglés desde la bibliografía
            if let (Some(id), true) = (o.get("id").and_then(|x| x.as_str()).map(String::from), o.contains_key("apa")) {
                if let Some(r) = crate::biblio::get(&id) {
                    o.insert("apa".into(), Value::String(crate::biblio::apa_l(r, lang)));
                    if o.contains_key("cite") {
                        o.insert("cite".into(), Value::String(crate::biblio::cite_l(r, lang)));
                    }
                }
            }
            // origen de los datos de un ejemplo: se arma de nuevo en inglés
            if let (Some(key), true) = (o.get("key").and_then(|x| x.as_str()).map(String::from), o.contains_key("summary") && o.contains_key("origin")) {
                if let Some((m, n)) = key.split_once('/') {
                    if let Some(d) = crate::biblio::data_source_l(m, n, lang) {
                        if let Ok(Value::Object(nd)) = serde_json::to_value(d) {
                            *o = nd;
                        }
                    }
                }
            }
            // lista de opciones [[valor, etiqueta], …]: solo se traduce la etiqueta;
            // el valor por defecto de una lista es un código
            let choice = o.get("type").and_then(|t| t.as_str()) == Some("choice");
            let choice_field = o.get("kind").and_then(|k| k.get("type")).and_then(|t| t.as_str()) == Some("choice");
            for (k, x) in o.iter_mut() {
                if k == "apa" || k == "cite" || (choice_field && k == "default") {
                    continue;
                }
                if choice && k == "options" {
                    for pair in x.as_array_mut().into_iter().flatten() {
                        if let Some(label) = pair.get_mut(1) {
                            walk(label, lang, false);
                        }
                    }
                    continue;
                }
                if REFS.contains(&k.as_str()) {
                    match x {
                        Value::String(s) => *s = translate_refs(s, lang),
                        Value::Array(a) => {
                            for y in a {
                                if let Value::String(s) = y {
                                    *s = translate_refs(s, lang);
                                }
                            }
                        }
                        _ => {}
                    }
                    continue;
                }
                walk(x, lang, PROTECTED.contains(&k.as_str()));
            }
        }
        _ => {}
    }
}

/// Markdown (resumen para la IA): se traduce línea por línea y, en las tablas,
/// celda por celda; los bloques de código quedan igual.
pub fn translate_markdown(md: &str, lang: Lang) -> String {
    if lang == Lang::Es {
        return marca(md);
    }
    let mut code = false;
    let mut out = Vec::new();
    for line in md.split('\n') {
        if line.trim_start().starts_with("```") {
            code = !code;
            out.push(line.to_string());
            continue;
        }
        if code {
            out.push(line.to_string());
            continue;
        }
        let (pre, body) = split_md_prefix(line);
        let t = if body.starts_with('|') && body.ends_with('|') {
            body.split('|').map(|c| if c.trim().is_empty() { c.to_string() } else { pad(c, &translate_md_inline(c.trim(), lang)) }).collect::<Vec<_>>().join("|")
        } else {
            translate_md_inline(body, lang)
        };
        out.push(format!("{pre}{t}"));
    }
    marca(&out.join("\n"))
}

fn pad(orig: &str, t: &str) -> String {
    let l = &orig[..orig.len() - orig.trim_start().len()];
    let r = &orig[orig.trim_end().len()..];
    format!("{l}{t}{r}")
}

fn split_md_prefix(line: &str) -> (&str, &str) {
    let mut i = 0;
    let b = line.as_bytes();
    while i < b.len() && (b[i] == b' ' || b[i] == b'#' || b[i] == b'>') {
        i += 1;
    }
    if line[i..].starts_with("- ") || line[i..].starts_with("* ") {
        i += 2;
    }
    (&line[..i], &line[i..])
}

/// Quita las marcas ** y _ alrededor antes de buscar, y las vuelve a poner.
fn translate_md_inline(s: &str, lang: Lang) -> String {
    for m in ["**", "_", "`"] {
        if s.len() > 2 * m.len() && s.starts_with(m) && s.ends_with(m) {
            let inner = &s[m.len()..s.len() - m.len()];
            if !inner.contains(m) {
                return format!("{m}{}{m}", translate(inner, lang));
            }
        }
    }
    let t = translate(s, lang);
    if t != s {
        return t;
    }
    // «**Título:** texto»
    if let Some(rest) = s.strip_prefix("**") {
        if let Some(p) = rest.find("**") {
            let (a, b) = (&rest[..p], &rest[p + 2..]);
            return format!("**{}**{}", translate(a, lang), if b.trim().is_empty() { b.to_string() } else { pad(b, &translate(b.trim(), lang)) });
        }
    }
    t
}

/// Traduce los textos visibles de un SVG (contenido de <text> y <tspan>, y <title>).
pub fn translate_svg(svg: &str, lang: Lang) -> String {
    if lang == Lang::Es {
        return svg.to_string();
    }
    let mut out = String::with_capacity(svg.len());
    let mut rest = svg;
    while let Some(p) = rest.find('>') {
        out.push_str(&rest[..=p]);
        rest = &rest[p + 1..];
        let end = rest.find('<').unwrap_or(rest.len());
        let txt = &rest[..end];
        if has_letters(txt) {
            let raw = txt.replace("&amp;", "&").replace("&lt;", "<").replace("&gt;", ">").replace("&quot;", "\"");
            let t = translate(&raw, lang);
            out.push_str(&t.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;"));
        } else {
            out.push_str(txt);
        }
        rest = &rest[end..];
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plantillas_y_exactos() {
        assert_eq!(Lang::parse(Some("en-US")), Lang::En);
        assert_eq!(translate("Resolver", Lang::Es), "Resolver");
        let x = translate("Pedir 25298.2213 unidades cada 3.1623 mes; costo del inventario 7589.4664 por mes.", Lang::En);
        assert!(!x.contains("Pedir") && x.contains("25298.2213") && x.contains("month"), "{x}");
        assert_eq!(polish("10th ed. y 2th ed. y 11th ed.".into()), "10th ed. y 2nd ed. y 11th ed.");
        assert_eq!(polish("Hillier y Lieberman (2015a), cap. 3".into()), "Hillier and Lieberman (2015a), cap. 3");
        assert_eq!(polish("(Clarke y Wright, 1964)".into()), "(Clarke & Wright, 1964)");
        let s = translate_svg("<text x=\"1\">Costo total</text>", Lang::En);
        assert!(!s.contains("Costo"), "{s}");
    }

    #[test]
    fn portugues() {
        assert_eq!(Lang::parse(Some("pt-BR")), Lang::Pt);
        assert_eq!(Lang::parse(Some("português")), Lang::Pt);
        let x = translate("Pedir 25298.2213 unidades cada 3.1623 mes; costo del inventario 7589.4664 por mes.", Lang::Pt);
        assert!(x.contains("25298.2213") && x.contains("custo do estoque") && !x.contains("costo"), "{x}");
        assert_eq!(translate("Resolver", Lang::En), "Solve");
        assert_ne!(translate("Restricciones", Lang::Pt), translate("Restricciones", Lang::En));
        assert_eq!(polish_pt("Hillier y Lieberman (2015a), 10.ª ed.".into()), "Hillier e Lieberman (2015a), 10. ed.");
        assert_eq!(polish_pt("(Clarke y Wright, 1964)".into()), "(Clarke & Wright, 1964)");
    }
}
