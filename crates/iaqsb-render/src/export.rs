//! Informes para entregar: cualquier informe de IAQSB (LP-ILP o los demás
//! módulos) como documento de Word (.docx), PDF o HTML, con portada, tablas y
//! figuras numeradas al estilo APA 7 («Tabla 1», «Figura 1») y la lista de
//! referencias con sangría francesa.
//!
//! Primero se arma un [`Doc`] neutro a partir del informe (JSON) y después cada
//! formato lo escribe. Los textos fijos se escriben en español y pasan por la
//! función `tr` que da quien llama (la traducción vive en iaqsb-core).

use serde_json::Value;
use std::fmt::Write as _;

/// Opciones del documento.
#[derive(Debug, Clone, Default)]
pub struct Options {
    /// Incluye el paso a paso (tablas del símplex, iteraciones…).
    pub steps: bool,
    /// Datos de la portada (opcionales).
    pub author: String,
    pub course: String,
    pub institution: String,
    /// Fecha ya escrita («21 de octubre de 2026»).
    pub date: String,
    /// Nombre del módulo («Programación lineal y entera»).
    pub module_name: String,
    /// Modelo resuelto (LP-ILP: para escribir el planteamiento).
    pub model: Option<Value>,
    /// Coma decimal (como se escribe en Colombia, Brasil y casi toda Latinoamérica).
    pub decimal_comma: bool,
    /// Código del idioma del documento (es, en, pt).
    pub lang: String,
    /// Versión de IAQSB para el pie.
    pub version: String,
}

/// Una tabla del documento (ya como texto).
#[derive(Debug, Clone, Default)]
pub struct Table {
    pub title: String,
    pub header: Vec<String>,
    pub rows: Vec<Vec<String>>,
    /// Columnas numéricas (se alinean a la derecha).
    pub numeric: Vec<bool>,
    /// Fila de totales o resaltada (índices).
    pub strong_rows: Vec<usize>,
    pub note: String,
}

#[derive(Debug, Clone)]
pub enum Item {
    /// Título de sección (1, 2 o 3).
    Heading(u8, String),
    /// Párrafo (admite **negrita** y *cursiva*).
    Para(String),
    /// Párrafo destacado (aviso, conclusión).
    Callout(String),
    /// Pares dato–valor.
    Metrics(Vec<(String, String)>),
    Table(Table),
    /// Figura: título y SVG.
    Figure { title: String, svg: String },
    /// Líneas de un modelo (fórmulas).
    Formula(Vec<String>),
}

#[derive(Debug, Clone, Default)]
pub struct Doc {
    pub title: String,
    pub subtitle: String,
    /// Portada: (dato, valor).
    pub meta: Vec<(String, String)>,
    pub blocks: Vec<Item>,
    /// Referencias en APA 7 (cursivas entre asteriscos).
    pub references: Vec<String>,
    /// Origen de los datos del ejemplo.
    pub data_source: String,
    pub lang: String,
    pub footer: String,
}

fn s<'a>(v: &'a Value, k: &str) -> &'a str {
    v.get(k).and_then(|x| x.as_str()).unwrap_or("")
}
fn arr<'a>(v: &'a Value, k: &str) -> &'a [Value] {
    v.get(k).and_then(|x| x.as_array()).map(|a| a.as_slice()).unwrap_or(&[])
}

/// Número para el documento: hasta 4 decimales, sin ceros de más, «−» como signo.
pub fn fmt_num(x: f64, comma: bool) -> String {
    if !x.is_finite() {
        return if x > 0.0 { "∞".into() } else if x < 0.0 { "−∞".into() } else { "—".into() };
    }
    if x.abs() >= 1e15 {
        return format!("{x:e}");
    }
    let r = (x * 1e4).round() / 1e4;
    let mut t = if r.fract() == 0.0 { format!("{}", r as i64) } else { format!("{r:.4}").trim_end_matches('0').trim_end_matches('.').to_string() };
    if t == "-0" {
        t = "0".into();
    }
    // miles con espacio fino (no se confunde con el separador decimal)
    let neg = t.starts_with('-');
    let body = t.trim_start_matches('-');
    let (int, frac) = body.split_once('.').map(|(a, b)| (a.to_string(), Some(b.to_string()))).unwrap_or((body.to_string(), None));
    let int = if int.len() > 4 {
        let b: Vec<char> = int.chars().collect();
        let mut o = String::new();
        for (i, c) in b.iter().enumerate() {
            if i > 0 && (b.len() - i) % 3 == 0 {
                o.push('\u{202f}');
            }
            o.push(*c);
        }
        o
    } else {
        int
    };
    let mut out = String::new();
    if neg {
        out.push('−');
    }
    out.push_str(&int);
    if let Some(f) = frac {
        out.push(if comma { ',' } else { '.' });
        out.push_str(&f);
    }
    out
}

fn cell(v: &Value, comma: bool) -> String {
    match v {
        Value::Null => String::new(),
        Value::Number(n) => fmt_num(n.as_f64().unwrap_or(0.0), comma),
        Value::String(t) => {
            if t == "M" {
                "M".into()
            } else if t == "-M" {
                "−M".into()
            } else if comma && t.parse::<f64>().is_ok() {
                fmt_num(t.parse().unwrap(), comma)
            } else {
                t.clone()
            }
        }
        Value::Bool(b) => if *b { "✓".into() } else { "—".into() },
        other => other.to_string(),
    }
}

fn is_num(v: &Value) -> bool {
    v.is_number() || v.as_str().is_some_and(|t| t == "M" || t == "-M" || t.replace('−', "-").trim().parse::<f64>().is_ok())
}

fn table_from_block(b: &Value, comma: bool) -> Table {
    let header: Vec<String> = arr(b, "columns").iter().map(|c| c.get("label").and_then(|l| l.as_str()).map(str::to_string).unwrap_or_else(|| c.as_str().unwrap_or("").to_string())).collect();
    let mut rows = vec![];
    let mut strong = vec![];
    let ncol = header.len();
    let mut num_votes = vec![(0usize, 0usize); ncol];
    for (i, r) in arr(b, "rows").iter().enumerate() {
        let cells: Vec<Value> = r.get("cells").and_then(|c| c.as_array()).cloned().or_else(|| r.as_array().cloned()).unwrap_or_default();
        for (j, c) in cells.iter().enumerate().take(ncol) {
            if !c.is_null() && !(c.as_str() == Some("")) {
                num_votes[j].1 += 1;
                if is_num(c) {
                    num_votes[j].0 += 1;
                }
            }
        }
        if matches!(s(r, "tone"), "total" | "highlight" | "accent") {
            strong.push(i);
        }
        rows.push(cells.iter().map(|c| cell(c, comma)).collect());
    }
    let numeric = num_votes.iter().enumerate().map(|(j, (n, t))| j > 0 && *t > 0 && n * 2 >= *t).collect();
    Table { title: s(b, "title").to_string(), header, rows, numeric, strong_rows: strong, note: s(b, "note").to_string() }
}

/// Arma el documento de un informe.
pub fn document(r: &Value, o: &Options, tr: &dyn Fn(&str) -> String) -> Doc {
    let mut d = Doc { title: s(r, "problem").to_string(), lang: if o.lang.is_empty() { "es".into() } else { o.lang.clone() }, ..Default::default() };
    if d.title.is_empty() {
        d.title = tr("Informe");
    }
    d.subtitle = if o.module_name.is_empty() { tr(crate::digest::module_name_pub(s(r, "module"))) } else { o.module_name.clone() };
    for (k, v) in [("Autor", &o.author), ("Curso", &o.course), ("Institución", &o.institution), ("Fecha", &o.date)] {
        if !v.trim().is_empty() {
            d.meta.push((tr(k), v.trim().to_string()));
        }
    }
    let ver = if o.version.is_empty() { String::new() } else { format!(" {}", o.version) };
    d.meta.push((tr("Calculado con"), format!("IAQSB{ver} · {}", tr("Hecho por IA Nativa"))));
    d.footer = format!("{} · IAQSB", d.title);
    let lp = r.get("variables").is_some() && r.get("constraints").is_some() && r.get("objective_value").is_some();
    if lp {
        lp_doc(&mut d, r, o, tr);
    } else {
        generic_doc(&mut d, r, o, tr);
    }
    engine_items(&mut d, r, tr, o.decimal_comma);
    // referencias: las del informe (o las del bloque de fuentes)
    let mut refs: Vec<String> = arr(r, "references").iter().map(|x| s(x, "apa").to_string()).filter(|a| !a.is_empty()).collect();
    if refs.is_empty() {
        for sec in arr(r, "sections") {
            for b in arr(sec, "blocks") {
                if s(b, "type") == "sources" {
                    refs.extend(arr(b, "refs").iter().map(|x| s(x, "apa").to_string()).filter(|a| !a.is_empty()));
                }
            }
        }
    }
    refs.sort_by_key(|a| a.to_lowercase());
    refs.dedup();
    d.references = refs;
    if let Some(ds) = r.get("data_source").filter(|x| x.is_object()) {
        let lab = s(ds, "origin_label");
        let sum = s(ds, "summary");
        if !sum.is_empty() {
            d.data_source = if lab.is_empty() { sum.to_string() } else { format!("{lab}: {sum}") };
            if ds.get("verified").and_then(|v| v.as_bool()) == Some(false) {
                d.data_source.push_str(&format!(" ({})", tr("por verificar")));
            }
        }
    }
    d
}

fn status_text(st: &str, tr: &dyn Fn(&str) -> String) -> String {
    tr(match st {
        "optimal" => "Óptimo",
        "feasible" => "Factible (sin probar que es óptima)",
        "infeasible" => "No factible",
        "unbounded" => "No acotado",
        "time_limit" => "Se acabó el tiempo",
        "solved" => "Resuelto",
        "error" => "Error del motor",
        other => other,
    })
}

fn engine_items(d: &mut Doc, r: &Value, tr: &dyn Fn(&str) -> String, comma: bool) {
    let e = &r["engine"];
    if !e.is_object() {
        return;
    }
    let mut m = vec![(tr("Motor"), s(e, "solver").to_string())];
    if let Some(t) = e["time"].as_f64() {
        m.push((tr("Tiempo de cálculo"), format!("{} s", fmt_num(t, comma))));
    }
    if let (Some(c), Some(f)) = (e["cols"].as_u64(), e["rows"].as_u64()) {
        m.push((tr("Tamaño"), tr(&format!("{c} variables × {f} restricciones"))));
    }
    if let Some(g) = e["gap"].as_f64().filter(|_| e["is_mip"].as_bool() == Some(true) || e.get("runs").is_some()) {
        m.push((tr("Brecha MIP"), format!("{} %", fmt_num(g * 100.0, comma))));
    }
    if let Some(n) = e["nodes"].as_u64().filter(|_| e["is_mip"].as_bool() == Some(true)) {
        m.push((tr("Nodos"), n.to_string()));
    }
    d.blocks.push(Item::Heading(2, tr("Detalles del cálculo")));
    d.blocks.push(Item::Metrics(m));
}

fn lp_doc(d: &mut Doc, r: &Value, o: &Options, tr: &dyn Fn(&str) -> String) {
    let c = o.decimal_comma;
    d.blocks.push(Item::Heading(1, tr("Resultado")));
    d.blocks.push(Item::Callout(s(r, "message").to_string()));
    let mut m = vec![(tr("Estado"), status_text(s(r, "status"), tr))];
    if let Some(z) = r["objective_value"].as_f64() {
        m.push((if s(r, "objective") == "min" { tr("Mínimo de la función objetivo") } else { tr("Máximo de la función objetivo") }, fmt_num(z, c)));
    }
    d.blocks.push(Item::Metrics(m));
    if let Some(model) = &o.model {
        let lines = lp_formulation(model, c, tr);
        if !lines.is_empty() {
            d.blocks.push(Item::Heading(1, tr("Planteamiento del modelo")));
            d.blocks.push(Item::Formula(lines));
        }
    }
    if let Some(dg) = r.get("diagnosis").filter(|x| x.is_object()) {
        d.blocks.push(Item::Heading(1, tr("Por qué no es factible")));
        d.blocks.push(Item::Para(s(dg, "summary").to_string()));
        let conf: Vec<String> = arr(dg, "conflict").iter().map(|x| s(x, "text").to_string()).collect();
        if !conf.is_empty() {
            d.blocks.push(Item::Formula(conf));
        }
    }
    let ok = !matches!(s(r, "status"), "infeasible" | "unbounded" | "error" | "time_limit");
    let sens = r["sensitivity"].as_bool().unwrap_or(false);
    if ok && !arr(r, "variables").is_empty() {
        let mut header = vec![tr("Variable"), tr("Valor"), "c(j)".into(), tr("Contribución"), tr("Costo reducido"), tr("Estado")];
        if sens {
            header.extend([tr("c(j) mínimo"), tr("c(j) máximo")]);
        }
        let vars = arr(r, "variables");
        // en modelos grandes, solo las distintas de cero
        let big = vars.len() > 60;
        let rows: Vec<Vec<String>> = vars
            .iter()
            .filter(|v| !big || v["value"].as_f64().unwrap_or(0.0).abs() > 1e-9)
            .take(500)
            .map(|v| {
                let mut row = vec![s(v, "name").to_string(), cell(&v["value"], c), cell(&v["cost"], c), cell(&v["contribution"], c), cell(&v["reduced_cost"], c), if s(v, "basis_status") == "basic" { tr("Básica") } else { tr("En su cota") }];
                if sens {
                    row.extend([cell(&v["cost_min"], c), cell(&v["cost_max"], c)]);
                }
                row
            })
            .collect();
        let n = header.len();
        let mut t = Table { title: tr("Variables de decisión"), header, rows, numeric: (0..n).map(|j| (1..5).contains(&j) || j >= 6).collect(), ..Default::default() };
        if big {
            t.note = tr(&format!("Solo las variables distintas de cero ({} de {}).", t.rows.len(), vars.len()));
        }
        d.blocks.push(Item::Heading(1, tr("Solución")));
        d.blocks.push(Item::Table(t));
        let mut header = vec![tr("Restricción"), tr("Lado izquierdo"), tr("Sentido"), tr("Lado derecho"), tr("Holgura o exceso")];
        if sens {
            header.extend([tr("Precio sombra"), tr("Lado der. mínimo"), tr("Lado der. máximo")]);
        }
        let cons = arr(r, "constraints");
        let rows: Vec<Vec<String>> = cons
            .iter()
            .take(500)
            .map(|x| {
                let mut row = vec![s(x, "name").to_string(), cell(&x["lhs"], c), s(x, "direction").replace("<=", "≤").replace(">=", "≥"), cell(&x["rhs"], c), cell(&x["slack_or_surplus"], c)];
                if sens {
                    row.extend([cell(&x["shadow_price"], c), cell(&x["rhs_min"], c), cell(&x["rhs_max"], c)]);
                }
                row
            })
            .collect();
        let n = header.len();
        d.blocks.push(Item::Table(Table { title: tr("Restricciones"), header, rows, numeric: (0..n).map(|j| j != 0 && j != 2).collect(), ..Default::default() }));
        // modelo algebraico: la solución por índices
        let idx = arr(r, "tables");
        if !idx.is_empty() {
            d.blocks.push(Item::Heading(2, tr("Solución por índices")));
            for t in idx {
                d.blocks.push(Item::Table(table_from_block(t, c)));
            }
        }
        if r["alternate_optima"].as_bool() == Some(true) {
            d.blocks.push(Item::Para(tr("Hay soluciones óptimas alternativas: otras combinaciones de variables alcanzan el mismo valor del objetivo.")));
        }
        if sens {
            d.blocks.push(Item::Para(tr("El precio sombra es lo que cambia el objetivo por cada unidad más de lado derecho, válido dentro de su rango. Los rangos de c(j) indican cuánto puede cambiar cada coeficiente sin que cambie la solución.")));
        }
    }
    for n in arr(r, "notes") {
        if let Some(t) = n.as_str() {
            d.blocks.push(Item::Para(t.to_string()));
        }
    }
    // paso a paso
    if o.steps {
        let trace = arr(r, "simplex_trace");
        if !trace.is_empty() {
            d.blocks.push(Item::Heading(1, tr("Tablas del símplex")));
            for t in trace {
                let mv = |v: &Value| -> String {
                    let (m, k) = (v["m"].as_f64().unwrap_or(0.0), v["c"].as_f64().unwrap_or(0.0));
                    if m == 0.0 {
                        fmt_num(k, c)
                    } else if k == 0.0 {
                        format!("{}M", fmt_num(m, c))
                    } else {
                        format!("{}M {} {}", fmt_num(m, c), if k < 0.0 { "−" } else { "+" }, fmt_num(k.abs(), c))
                    }
                };
                let cols: Vec<String> = arr(t, "columns").iter().map(|x| x.as_str().unwrap_or("").to_string()).collect();
                let mut header = vec![tr("Base")];
                header.extend(cols.clone());
                header.push("b".into());
                let mut rows = vec![];
                for (i, row) in arr(t, "rows").iter().enumerate() {
                    let mut cells = vec![t["basis"][i].as_str().unwrap_or("").to_string()];
                    cells.extend(row.as_array().into_iter().flatten().map(|x| cell(x, c)));
                    cells.push(cell(&t["rhs"][i], c));
                    rows.push(cells);
                }
                let mut cz = vec!["C(j)−Z(j)".to_string()];
                cz.extend(arr(t, "cj_minus_zj").iter().map(mv));
                cz.push(mv(&t["objective"]));
                let last = rows.len();
                rows.push(cz);
                let n = header.len();
                let mut title = tr(&format!("Iteración {}", t["iteration"]));
                if let (Some(e), Some(l)) = (t["entering"].as_str(), t["leaving"].as_str()) {
                    title = tr(&format!("Iteración {}: entra {e}, sale {l}", t["iteration"]));
                }
                d.blocks.push(Item::Table(Table { title, header, rows, numeric: (0..n).map(|j| j > 0).collect(), strong_rows: vec![last], ..Default::default() }));
            }
        }
        if let Some(b) = r.get("branch_and_bound").filter(|x| x.is_object()) {
            let tr_rows: Vec<Vec<String>> = arr(b, "trace")
                .iter()
                .map(|n| vec![n["node"].to_string(), arr(n, "branches").iter().filter_map(|x| x.as_str()).collect::<Vec<_>>().join(", "), cell(&n["objective_value"], c), s(n, "action").to_string()])
                .collect();
            if !tr_rows.is_empty() {
                d.blocks.push(Item::Heading(1, tr("Branch and bound")));
                d.blocks.push(Item::Table(Table {
                    title: tr("Nodos explorados"),
                    header: vec![tr("Nodo"), tr("Ramas"), "z".into(), tr("Acción")],
                    rows: tr_rows,
                    numeric: vec![true, false, true, false],
                    ..Default::default()
                }));
            }
        }
    }
}

/// «Max Z = 3X1 + 5X2», «sujeto a», cada restricción y las cotas.
fn lp_formulation(m: &Value, comma: bool, tr: &dyn Fn(&str) -> String) -> Vec<String> {
    let vars = arr(m, "variables");
    let names: Vec<String> = vars.iter().map(|v| s(v, "name").to_string()).collect();
    let expr = |coefs: &[Value]| -> String {
        let mut out = String::new();
        for (j, c) in coefs.iter().enumerate() {
            let a = c.as_f64().unwrap_or(0.0);
            if a == 0.0 || j >= names.len() {
                continue;
            }
            let sign = if a < 0.0 { "−" } else { "+" };
            // «3X1», pero «3 Puertas» (nombres que son palabras)
            let word = names[j].chars().filter(|c| c.is_alphabetic()).count() > 2;
            let mag = if (a.abs() - 1.0).abs() < 1e-12 { String::new() } else if word { format!("{} ", fmt_num(a.abs(), comma)) } else { fmt_num(a.abs(), comma) };
            if out.is_empty() {
                out = format!("{}{mag}{}", if a < 0.0 { "−" } else { "" }, names[j]);
            } else {
                let _ = write!(out, " {sign} {mag}{}", names[j]);
            }
        }
        if out.is_empty() { "0".into() } else { out }
    };
    if vars.is_empty() {
        return vec![];
    }
    let costs: Vec<Value> = vars.iter().map(|v| v["cost"].clone()).collect();
    let mut lines = vec![format!("{} Z = {}", if s(m, "objective") == "min" { tr("Minimizar") } else { tr("Maximizar") }, expr(&costs))];
    lines.push(tr("sujeto a"));
    for c in arr(m, "constraints").iter().take(80) {
        let coefs = c["coefs"].as_array().cloned().unwrap_or_default();
        let dir = s(c, "direction").replace("<=", "≤").replace(">=", "≥");
        lines.push(format!("{}: {} {dir} {}", s(c, "name"), expr(&coefs), cell(&c["rhs"], comma)));
    }
    if arr(m, "constraints").len() > 80 {
        lines.push(format!("… ({} {})", arr(m, "constraints").len() - 80, tr("restricciones más")));
    }
    let mut ints = vec![];
    let mut bins = vec![];
    let mut free = vec![];
    for v in vars {
        match s(v, "type") {
            "integer" | "semiinteger" => ints.push(s(v, "name")),
            "binary" => bins.push(s(v, "name")),
            "unrestricted" => free.push(s(v, "name")),
            _ => {}
        }
    }
    let nonneg: Vec<&str> = vars.iter().filter(|v| !matches!(s(v, "type"), "unrestricted" | "binary")).map(|v| s(v, "name")).collect();
    if !nonneg.is_empty() && nonneg.len() <= 30 {
        lines.push(format!("{} ≥ 0", nonneg.join(", ")));
    }
    if !ints.is_empty() {
        lines.push(format!("{}: {}", tr("Enteras"), ints.join(", ")));
    }
    if !bins.is_empty() {
        lines.push(format!("{}: {}", tr("Binarias"), bins.join(", ")));
    }
    if !free.is_empty() {
        lines.push(format!("{}: {}", tr("Libres"), free.join(", ")));
    }
    lines
}

fn generic_doc(d: &mut Doc, r: &Value, o: &Options, tr: &dyn Fn(&str) -> String) {
    d.blocks.push(Item::Heading(1, tr("Resultado")));
    d.blocks.push(Item::Callout(s(r, "headline").to_string()));
    d.blocks.push(Item::Metrics(vec![(tr("Estado"), status_text(s(r, "status"), tr))]));
    for sec in arr(r, "sections") {
        let id = s(sec, "id");
        if id == "fuentes" {
            continue;
        }
        let blocks = arr(sec, "blocks");
        let only_steps = !blocks.is_empty() && blocks.iter().all(|b| s(b, "type") == "steps");
        if only_steps && !o.steps {
            continue;
        }
        d.blocks.push(Item::Heading(1, s(sec, "title").to_string()));
        for b in blocks {
            block_items(&mut d.blocks, b, o, 2);
        }
    }
}

fn block_items(out: &mut Vec<Item>, b: &Value, o: &Options, level: u8) {
    let c = o.decimal_comma;
    match s(b, "type") {
        "metrics" => {
            if !s(b, "title").is_empty() {
                out.push(Item::Heading(level, s(b, "title").to_string()));
            }
            let items = arr(b, "items")
                .iter()
                .map(|m| {
                    let mut v = cell(&m["value"], c);
                    if !s(m, "unit").is_empty() {
                        v = format!("{v} {}", s(m, "unit"));
                    }
                    (s(m, "label").to_string(), v)
                })
                .collect();
            out.push(Item::Metrics(items));
        }
        "table" => out.push(Item::Table(table_from_block(b, c))),
        "text" => {
            if !s(b, "title").is_empty() {
                out.push(Item::Heading(level, s(b, "title").to_string()));
            }
            let body = s(b, "body");
            if matches!(s(b, "tone"), "accent" | "highlight" | "good" | "bad") {
                out.push(Item::Callout(body.to_string()));
            } else {
                for p in body.split("\n\n") {
                    if !p.trim().is_empty() {
                        out.push(Item::Para(p.trim().to_string()));
                    }
                }
            }
        }
        "steps" => {
            if !o.steps {
                return;
            }
            for st in arr(b, "steps") {
                out.push(Item::Heading(level.min(3), s(st, "title").to_string()));
                if !s(st, "description").is_empty() {
                    out.push(Item::Para(s(st, "description").to_string()));
                }
                for x in arr(st, "blocks") {
                    block_items(out, x, o, 3);
                }
            }
        }
        "sources" => {}
        _ => {
            // el título va arriba de la figura (APA), no dentro de la imagen
            let mut nb = b.clone();
            if let Some(o) = nb.as_object_mut() {
                o.remove("title");
            }
            if let Some(svg) = crate::svg(&nb) {
                out.push(Item::Figure { title: s(b, "title").to_string(), svg });
            }
        }
    }
}

// ================================================================ Markdown en línea

/// Trozos de un texto con **negrita** y *cursiva*.
pub fn runs(t: &str) -> Vec<(String, bool, bool)> {
    let mut out: Vec<(String, bool, bool)> = vec![];
    let (mut bold, mut ital) = (false, false);
    let mut cur = String::new();
    let ch: Vec<char> = t.chars().collect();
    let mut i = 0;
    while i < ch.len() {
        if ch[i] == '*' && i + 1 < ch.len() && ch[i + 1] == '*' {
            if !cur.is_empty() {
                out.push((std::mem::take(&mut cur), bold, ital));
            }
            bold = !bold;
            i += 2;
            continue;
        }
        // una sola marca de cursiva, si tiene pareja más adelante
        if ch[i] == '*' && (ital || ch[i + 1..].contains(&'*')) {
            if !cur.is_empty() {
                out.push((std::mem::take(&mut cur), bold, ital));
            }
            ital = !ital;
            i += 1;
            continue;
        }
        cur.push(ch[i]);
        i += 1;
    }
    if !cur.is_empty() {
        out.push((cur, bold, ital));
    }
    out
}

fn xml(t: &str) -> String {
    let mut o = String::with_capacity(t.len());
    for c in t.chars() {
        match c {
            '&' => o.push_str("&amp;"),
            '<' => o.push_str("&lt;"),
            '>' => o.push_str("&gt;"),
            '"' => o.push_str("&quot;"),
            c if (c as u32) < 0x20 && c != '\n' && c != '\t' => {}
            c => o.push(c),
        }
    }
    o
}

/// Tamaño de un SVG (width y height del elemento raíz, o su viewBox).
pub fn svg_size(svg: &str) -> (f64, f64) {
    let head = &svg[..svg.find('>').unwrap_or(svg.len()).min(svg.len())];
    let attr = |k: &str| -> Option<f64> {
        let p = head.find(&format!(" {k}=\""))? + k.len() + 3;
        let q = head[p..].find('"')? + p;
        head[p..q].trim_end_matches("px").parse().ok()
    };
    match (attr("width"), attr("height")) {
        (Some(w), Some(h)) => (w, h),
        _ => {
            if let Some(p) = head.find("viewBox=\"") {
                let v: Vec<f64> = head[p + 9..].split('"').next().unwrap_or("").split([' ', ',']).filter_map(|x| x.parse().ok()).collect();
                if v.len() == 4 {
                    return (v[2], v[3]);
                }
            }
            (640.0, 400.0)
        }
    }
}

// ================================================================ HTML

/// Documento HTML autónomo (las figuras van en SVG dentro del archivo).
pub fn html(d: &Doc, tr: &dyn Fn(&str) -> String) -> String {
    let inline = |t: &str| -> String {
        runs(t)
            .into_iter()
            .map(|(x, b, i)| {
                let mut x = xml(&x);
                if i {
                    x = format!("<em>{x}</em>");
                }
                if b {
                    x = format!("<strong>{x}</strong>");
                }
                x
            })
            .collect()
    };
    let mut h = String::new();
    let _ = write!(
        h,
        r#"<!doctype html><html lang="{}"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><meta name="generator" content="IAQSB"><title>{}</title><style>
:root{{--ink:#1d1d1f;--ink2:#515154;--rule:#d2d2d7;--soft:#f5f5f7;--accent:#0a66d8;--bg:#fff}}
@media (prefers-color-scheme:dark){{:root{{--ink:#f5f5f7;--ink2:#a1a1a6;--rule:#3a3a3c;--soft:#1c1c1e;--accent:#4c9bff;--bg:#000}}}}
*{{box-sizing:border-box}}body{{margin:0;background:var(--bg);color:var(--ink);font:15px/1.55 -apple-system,BlinkMacSystemFont,"Segoe UI",Inter,Roboto,"Helvetica Neue",Arial,sans-serif}}
main{{max-width:900px;margin:0 auto;padding:48px 24px 64px}}h1.t{{font-size:30px;margin:0 0 4px;letter-spacing:-.02em}}.sub{{color:var(--ink2);margin:0 0 20px;font-size:17px}}
.meta{{display:grid;grid-template-columns:max-content 1fr;gap:2px 14px;margin:0 0 28px;color:var(--ink2);font-size:14px}}.meta b{{font-weight:600;color:var(--ink)}}
h1{{font-size:21px;margin:34px 0 10px;padding-bottom:6px;border-bottom:1px solid var(--rule)}}h2{{font-size:17px;margin:24px 0 8px}}h3{{font-size:15px;margin:18px 0 6px}}
.call{{border-left:3px solid var(--accent);background:var(--soft);padding:10px 14px;border-radius:0 8px 8px 0;font-weight:500}}
dl{{display:grid;grid-template-columns:max-content 1fr;gap:4px 18px;margin:10px 0}}dt{{color:var(--ink2)}}dd{{margin:0;font-variant-numeric:tabular-nums;font-weight:600}}
.cap{{margin:22px 0 6px}}.cap b{{display:block}}.cap i{{display:block}}.tw{{overflow-x:auto}}
table{{border-collapse:collapse;font-variant-numeric:tabular-nums;font-size:13.5px;min-width:40%}}th,td{{padding:5px 10px;border-bottom:1px solid var(--rule);text-align:left;vertical-align:top}}
thead th{{border-top:1.5px solid var(--ink);border-bottom:1px solid var(--ink);font-weight:600}}tbody tr:last-child td{{border-bottom:1.5px solid var(--ink)}}td.n,th.n{{text-align:right}}tr.s td{{font-weight:600}}
.note{{font-size:13px;color:var(--ink2);margin:4px 0 0}}figure{{margin:0 0 8px}}figure svg{{max-width:100%;height:auto;background:#fff;border-radius:8px}}
pre{{background:var(--soft);padding:12px 14px;border-radius:8px;overflow-x:auto;font:13.5px/1.6 ui-monospace,"SF Mono",Menlo,Consolas,monospace}}
.refs p{{padding-left:2em;text-indent:-2em;margin:0 0 8px}}footer{{margin-top:40px;color:var(--ink2);font-size:12.5px}}
@media print{{main{{padding:0}}h1{{break-after:avoid}}figure,table{{break-inside:avoid}}}}
</style></head><body><main>"#,
        xml(&d.lang),
        xml(&d.title)
    );
    let _ = write!(h, r#"<h1 class="t">{}</h1><p class="sub">{}</p><div class="meta">"#, xml(&d.title), xml(&d.subtitle));
    for (k, v) in &d.meta {
        let _ = write!(h, "<b>{}</b><span>{}</span>", xml(k), xml(v));
    }
    h.push_str("</div>");
    let (mut nt, mut nf) = (0, 0);
    for b in &d.blocks {
        match b {
            Item::Heading(l, t) => {
                let _ = write!(h, "<h{l}>{}</h{l}>", inline(t));
            }
            Item::Para(t) => {
                let _ = write!(h, "<p>{}</p>", inline(t));
            }
            Item::Callout(t) => {
                let _ = write!(h, r#"<p class="call">{}</p>"#, inline(t));
            }
            Item::Metrics(m) => {
                h.push_str("<dl>");
                for (k, v) in m {
                    let _ = write!(h, "<dt>{}</dt><dd>{}</dd>", xml(k), xml(v));
                }
                h.push_str("</dl>");
            }
            Item::Formula(lines) => {
                let _ = write!(h, "<pre>{}</pre>", xml(&lines.join("\n")));
            }
            Item::Table(t) => {
                nt += 1;
                let _ = write!(h, r#"<div class="cap"><b>{} {nt}</b><i>{}</i></div><div class="tw"><table><thead><tr>"#, xml(&tr("Tabla")), xml(&t.title));
                for (j, c) in t.header.iter().enumerate() {
                    let _ = write!(h, r#"<th{}>{}</th>"#, if t.numeric.get(j) == Some(&true) { r#" class="n""# } else { "" }, xml(c));
                }
                h.push_str("</tr></thead><tbody>");
                for (i, r) in t.rows.iter().enumerate() {
                    let _ = write!(h, "<tr{}>", if t.strong_rows.contains(&i) { r#" class="s""# } else { "" });
                    for (j, c) in r.iter().enumerate() {
                        let _ = write!(h, r#"<td{}>{}</td>"#, if t.numeric.get(j) == Some(&true) { r#" class="n""# } else { "" }, xml(c));
                    }
                    h.push_str("</tr>");
                }
                h.push_str("</tbody></table></div>");
                if !t.note.is_empty() {
                    let _ = write!(h, r#"<p class="note"><i>{}.</i> {}</p>"#, xml(&tr("Nota")), xml(&t.note));
                }
            }
            Item::Figure { title, svg } => {
                nf += 1;
                let body = svg.find("<svg").map(|p| &svg[p..]).unwrap_or(svg);
                let _ = write!(h, r#"<div class="cap"><b>{} {nf}</b><i>{}</i></div><figure>{body}</figure>"#, xml(&tr("Figura")), xml(title));
            }
        }
    }
    if !d.data_source.is_empty() {
        let _ = write!(h, "<h1>{}</h1><p>{}</p>", xml(&tr("Origen de los datos")), inline(&d.data_source));
    }
    if !d.references.is_empty() {
        let _ = write!(h, r#"<h1>{}</h1><div class="refs">"#, xml(&tr("Referencias")));
        for r in &d.references {
            let _ = write!(h, "<p>{}</p>", inline(r));
        }
        h.push_str("</div>");
    }
    let _ = write!(h, "<footer>{}</footer></main></body></html>", xml(&d.footer));
    h
}

// ================================================================ Word (.docx)

const EMU_PER_PT: f64 = 12_700.0;

/// Documento de Word: estilos propios (Título, Encabezado 1–3, Leyenda), tablas al
/// estilo APA, figuras en PNG, pie con número de página.
pub fn docx(d: &Doc, tr: &dyn Fn(&str) -> String) -> Result<Vec<u8>, String> {
    let mut body = String::new();
    let mut media: Vec<Vec<u8>> = vec![];
    let lang_tag = match d.lang.as_str() {
        "en" => "en-US",
        "pt" => "pt-BR",
        _ => "es-CO",
    };
    let run = |t: &str, b: bool, i: bool| -> String {
        let mut pr = String::new();
        if b {
            pr.push_str("<w:b/>");
        }
        if i {
            pr.push_str("<w:i/>");
        }
        let pr = if pr.is_empty() { String::new() } else { format!("<w:rPr>{pr}</w:rPr>") };
        // saltos de línea dentro del texto
        let parts: Vec<String> = t.split('\n').map(|p| format!(r#"<w:t xml:space="preserve">{}</w:t>"#, xml(p))).collect();
        format!("<w:r>{pr}{}</w:r>", parts.join("<w:br/>"))
    };
    let para = |style: &str, t: &str, extra: &str| -> String {
        let runs: String = runs(t).into_iter().map(|(x, b, i)| run(&x, b, i)).collect();
        let ps = if style.is_empty() { String::new() } else { format!(r#"<w:pStyle w:val="{style}"/>"#) };
        format!("<w:p><w:pPr>{ps}{extra}</w:pPr>{runs}</w:p>")
    };
    body.push_str(&para("Title", &d.title, ""));
    body.push_str(&para("Subtitle", &d.subtitle, ""));
    for (k, v) in &d.meta {
        body.push_str(&para("Meta", &format!("**{k}:** {v}"), ""));
    }
    let (mut nt, mut nf) = (0, 0);
    let width_pt = 451.0; // ancho útil de A4 con márgenes de 2,54 cm
    for b in &d.blocks {
        match b {
            Item::Heading(l, t) => body.push_str(&para(&format!("Heading{l}"), t, "")),
            Item::Para(t) => body.push_str(&para("", t, "")),
            Item::Callout(t) => body.push_str(&para("Callout", t, "")),
            Item::Metrics(m) => {
                for (k, v) in m {
                    body.push_str(&para("Metric", &format!("{k}: **{v}**"), ""));
                }
            }
            Item::Formula(lines) => {
                for l in lines {
                    body.push_str(&para("Formula", l, ""));
                }
            }
            Item::Table(t) => {
                nt += 1;
                body.push_str(&para("CaptionLabel", &format!("{} {nt}", tr("Tabla")), "<w:keepNext/>"));
                if !t.title.is_empty() {
                    body.push_str(&para("CaptionTitle", &t.title, "<w:keepNext/>"));
                }
                let ncol = t.header.len().max(t.rows.iter().map(|r| r.len()).max().unwrap_or(0)).max(1);
                let small = ncol > 7;
                let sz = if ncol > 12 { 14 } else if small { 16 } else { 19 };
                body.push_str(&format!(r#"<w:tbl><w:tblPr><w:tblStyle w:val="IAQSBTabla"/><w:tblW w:w="0" w:type="auto"/><w:tblLook w:val="04A0" w:firstRow="1" w:lastRow="0" w:firstColumn="0" w:lastColumn="0" w:noHBand="1" w:noVBand="1"/></w:tblPr><w:tblGrid>"#));
                // ancho de cada columna según su texto más largo (en caracteres)
                let lens: Vec<f64> = (0..ncol)
                    .map(|j| {
                        let h = t.header.get(j).map(|x| x.split_whitespace().map(|w| w.chars().count()).max().unwrap_or(0)).unwrap_or(0);
                        let c = t.rows.iter().filter_map(|r| r.get(j)).map(|x| x.chars().count()).max().unwrap_or(0);
                        (h.max(c).max(3) as f64) * if small { 105.0 } else { 125.0 } + 220.0
                    })
                    .collect();
                let total: f64 = lens.iter().sum();
                let k = if total > 9026.0 { 9026.0 / total } else { 1.0 };
                let grid: Vec<i64> = lens.iter().map(|l| (l * k) as i64).collect();
                body = body.replacen(r#"<w:tblW w:w="0" w:type="auto"/>"#, &format!(r#"<w:tblW w:w="{}" w:type="dxa"/><w:tblLayout w:type="fixed"/>"#, grid.iter().sum::<i64>()), 1);
                for g in &grid {
                    body.push_str(&format!(r#"<w:gridCol w:w="{g}"/>"#));
                }
                body.push_str("</w:tblGrid>");
                let tc = |txt: &str, num: bool, head: bool, strong: bool| -> String {
                    let jc = if num { r#"<w:jc w:val="right"/>"# } else { "" };
                    let b = if head || strong { "<w:b/>" } else { "" };
                    let border = if head { r#"<w:tcPr><w:tcBorders><w:top w:val="single" w:sz="8" w:color="1D1D1F"/><w:bottom w:val="single" w:sz="6" w:color="1D1D1F"/></w:tcBorders></w:tcPr>"# } else { "" };
                    format!(r#"<w:tc>{border}<w:p><w:pPr><w:spacing w:before="20" w:after="20"/>{jc}</w:pPr><w:r><w:rPr>{b}<w:sz w:val="{sz}"/></w:rPr><w:t xml:space="preserve">{}</w:t></w:r></w:p></w:tc>"#, xml(txt))
                };
                body.push_str(r#"<w:tr><w:trPr><w:tblHeader/></w:trPr>"#);
                for j in 0..ncol {
                    body.push_str(&tc(t.header.get(j).map(|x| x.as_str()).unwrap_or(""), t.numeric.get(j) == Some(&true), true, false));
                }
                body.push_str("</w:tr>");
                for (i, r) in t.rows.iter().enumerate() {
                    let last = i + 1 == t.rows.len();
                    let rowpr = if last { r#"<w:trPr><w:cantSplit/></w:trPr>"# } else { r#"<w:trPr><w:cantSplit/></w:trPr>"# };
                    body.push_str(&format!("<w:tr>{rowpr}"));
                    for j in 0..ncol {
                        let mut c = tc(r.get(j).map(|x| x.as_str()).unwrap_or(""), t.numeric.get(j) == Some(&true), false, t.strong_rows.contains(&i));
                        if last {
                            c = c.replacen("<w:tc>", r#"<w:tc><w:tcPr><w:tcBorders><w:bottom w:val="single" w:sz="8" w:color="1D1D1F"/></w:tcBorders></w:tcPr>"#, 1);
                        }
                        body.push_str(&c);
                    }
                    body.push_str("</w:tr>");
                }
                body.push_str("</w:tbl>");
                if !t.note.is_empty() {
                    body.push_str(&para("TableNote", &format!("*{}.* {}", tr("Nota"), t.note), ""));
                } else {
                    body.push_str(&para("", "", r#"<w:spacing w:after="60"/>"#));
                }
            }
            Item::Figure { title, svg } => {
                nf += 1;
                let png = crate::png(svg, 2.0)?;
                media.push(png);
                let (w, h) = svg_size(svg);
                let scale = (width_pt / w).min(380.0 / h).min(1.0);
                let (cx, cy) = ((w * scale * EMU_PER_PT) as i64, (h * scale * EMU_PER_PT) as i64);
                let id = media.len();
                body.push_str(&para("CaptionLabel", &format!("{} {nf}", tr("Figura")), "<w:keepNext/>"));
                if !title.is_empty() {
                    body.push_str(&para("CaptionTitle", title, "<w:keepNext/>"));
                }
                let _ = write!(
                    body,
                    r#"<w:p><w:pPr><w:spacing w:after="160"/></w:pPr><w:r><w:drawing><wp:inline distT="0" distB="0" distL="0" distR="0"><wp:extent cx="{cx}" cy="{cy}"/><wp:docPr id="{id}" name="Figura {id}" descr="{alt}"/><wp:cNvGraphicFramePr><a:graphicFrameLocks noChangeAspect="1"/></wp:cNvGraphicFramePr><a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/picture"><pic:pic><pic:nvPicPr><pic:cNvPr id="{id}" name="figura{id}.png"/><pic:cNvPicPr/></pic:nvPicPr><pic:blipFill><a:blip r:embed="rImg{id}"/><a:stretch><a:fillRect/></a:stretch></pic:blipFill><pic:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="{cx}" cy="{cy}"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom></pic:spPr></pic:pic></a:graphicData></a:graphic></wp:inline></w:drawing></w:r></w:p>"#,
                    alt = xml(title)
                );
            }
        }
    }
    if !d.data_source.is_empty() {
        body.push_str(&para("Heading1", &tr("Origen de los datos"), ""));
        body.push_str(&para("", &d.data_source, ""));
    }
    if !d.references.is_empty() {
        body.push_str(&para("Heading1", &tr("Referencias"), ""));
        for r in &d.references {
            body.push_str(&para("Reference", r, ""));
        }
    }
    let sect = r#"<w:sectPr><w:footerReference w:type="default" r:id="rFooter"/><w:pgSz w:w="11906" w:h="16838"/><w:pgMar w:top="1440" w:right="1440" w:bottom="1440" w:left="1440" w:header="708" w:footer="708" w:gutter="0"/></w:sectPr>"#;
    let document = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:pic="http://schemas.openxmlformats.org/drawingml/2006/picture"><w:body>{body}{sect}</w:body></w:document>"#
    );
    let footer = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:ftr xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:p><w:pPr><w:pStyle w:val="Footer"/><w:jc w:val="center"/></w:pPr><w:r><w:t xml:space="preserve">{} · </w:t></w:r><w:r><w:fldChar w:fldCharType="begin"/></w:r><w:r><w:instrText xml:space="preserve"> PAGE </w:instrText></w:r><w:r><w:fldChar w:fldCharType="separate"/></w:r><w:r><w:t>1</w:t></w:r><w:r><w:fldChar w:fldCharType="end"/></w:r></w:p></w:ftr>"#,
        xml(&d.footer)
    );
    let styles = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:docDefaults><w:rPrDefault><w:rPr><w:rFonts w:ascii="Calibri" w:hAnsi="Calibri" w:eastAsia="Calibri" w:cs="Calibri"/><w:sz w:val="22"/><w:szCs w:val="22"/><w:lang w:val="{lang_tag}"/><w:color w:val="1D1D1F"/></w:rPr></w:rPrDefault><w:pPrDefault><w:pPr><w:spacing w:after="120" w:line="276" w:lineRule="auto"/></w:pPr></w:pPrDefault></w:docDefaults>
<w:style w:type="paragraph" w:default="1" w:styleId="Normal"><w:name w:val="Normal"/><w:qFormat/></w:style>
<w:style w:type="paragraph" w:styleId="Title"><w:name w:val="Title"/><w:basedOn w:val="Normal"/><w:next w:val="Normal"/><w:qFormat/><w:pPr><w:spacing w:before="0" w:after="60"/></w:pPr><w:rPr><w:b/><w:sz w:val="44"/></w:rPr></w:style>
<w:style w:type="paragraph" w:styleId="Subtitle"><w:name w:val="Subtitle"/><w:basedOn w:val="Normal"/><w:qFormat/><w:pPr><w:spacing w:after="240"/></w:pPr><w:rPr><w:color w:val="515154"/><w:sz w:val="28"/></w:rPr></w:style>
<w:style w:type="paragraph" w:styleId="Meta"><w:name w:val="IAQSB Portada"/><w:basedOn w:val="Normal"/><w:pPr><w:spacing w:after="20"/></w:pPr><w:rPr><w:color w:val="515154"/><w:sz w:val="20"/></w:rPr></w:style>
<w:style w:type="paragraph" w:styleId="Heading1"><w:name w:val="heading 1"/><w:basedOn w:val="Normal"/><w:next w:val="Normal"/><w:qFormat/><w:pPr><w:keepNext/><w:spacing w:before="360" w:after="120"/><w:pBdr><w:bottom w:val="single" w:sz="4" w:space="2" w:color="D2D2D7"/></w:pBdr><w:outlineLvl w:val="0"/></w:pPr><w:rPr><w:b/><w:sz w:val="30"/></w:rPr></w:style>
<w:style w:type="paragraph" w:styleId="Heading2"><w:name w:val="heading 2"/><w:basedOn w:val="Normal"/><w:next w:val="Normal"/><w:qFormat/><w:pPr><w:keepNext/><w:spacing w:before="240" w:after="80"/><w:outlineLvl w:val="1"/></w:pPr><w:rPr><w:b/><w:sz w:val="25"/></w:rPr></w:style>
<w:style w:type="paragraph" w:styleId="Heading3"><w:name w:val="heading 3"/><w:basedOn w:val="Normal"/><w:next w:val="Normal"/><w:qFormat/><w:pPr><w:keepNext/><w:spacing w:before="180" w:after="60"/><w:outlineLvl w:val="2"/></w:pPr><w:rPr><w:b/><w:i/><w:sz w:val="22"/></w:rPr></w:style>
<w:style w:type="paragraph" w:styleId="Callout"><w:name w:val="IAQSB Destacado"/><w:basedOn w:val="Normal"/><w:pPr><w:pBdr><w:left w:val="single" w:sz="18" w:space="8" w:color="0A66D8"/></w:pBdr><w:shd w:val="clear" w:color="auto" w:fill="F2F2F7"/><w:ind w:left="200"/><w:spacing w:before="80" w:after="160"/></w:pPr><w:rPr><w:b/></w:rPr></w:style>
<w:style w:type="paragraph" w:styleId="Metric"><w:name w:val="IAQSB Dato"/><w:basedOn w:val="Normal"/><w:pPr><w:spacing w:after="20"/><w:ind w:left="200"/></w:pPr></w:style>
<w:style w:type="paragraph" w:styleId="Formula"><w:name w:val="IAQSB Fórmula"/><w:basedOn w:val="Normal"/><w:pPr><w:spacing w:after="20"/><w:ind w:left="360"/></w:pPr><w:rPr><w:rFonts w:ascii="Cambria Math" w:hAnsi="Cambria Math"/></w:rPr></w:style>
<w:style w:type="paragraph" w:styleId="CaptionLabel"><w:name w:val="IAQSB Número de tabla o figura"/><w:basedOn w:val="Normal"/><w:pPr><w:keepNext/><w:spacing w:before="240" w:after="0"/></w:pPr><w:rPr><w:b/></w:rPr></w:style>
<w:style w:type="paragraph" w:styleId="CaptionTitle"><w:name w:val="IAQSB Título de tabla o figura"/><w:basedOn w:val="Normal"/><w:pPr><w:keepNext/><w:spacing w:after="100"/></w:pPr><w:rPr><w:i/></w:rPr></w:style>
<w:style w:type="paragraph" w:styleId="TableNote"><w:name w:val="IAQSB Nota de tabla"/><w:basedOn w:val="Normal"/><w:pPr><w:spacing w:before="60" w:after="200"/></w:pPr><w:rPr><w:sz w:val="19"/></w:rPr></w:style>
<w:style w:type="paragraph" w:styleId="Reference"><w:name w:val="IAQSB Referencia"/><w:basedOn w:val="Normal"/><w:pPr><w:ind w:left="720" w:hanging="720"/><w:spacing w:after="120" w:line="360" w:lineRule="auto"/></w:pPr></w:style>
<w:style w:type="paragraph" w:styleId="Footer"><w:name w:val="footer"/><w:basedOn w:val="Normal"/><w:rPr><w:color w:val="8E8E93"/><w:sz w:val="18"/></w:rPr></w:style>
<w:style w:type="table" w:styleId="IAQSBTabla"><w:name w:val="IAQSB Tabla"/><w:tblPr><w:tblCellMar><w:left w:w="90" w:type="dxa"/><w:right w:w="90" w:type="dxa"/></w:tblCellMar><w:tblBorders><w:insideH w:val="single" w:sz="2" w:color="D2D2D7"/></w:tblBorders></w:tblPr></w:style>
</w:styles>"#
    );
    let mut ct = String::from(r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Default Extension="png" ContentType="image/png"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/><Override PartName="/word/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml"/><Override PartName="/word/footer1.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.footer+xml"/><Override PartName="/docProps/core.xml" ContentType="application/vnd.openxmlformats-package.core-properties+xml"/></Types>"#);
    ct.shrink_to_fit();
    let mut rels = String::from(r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rStyles" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/><Relationship Id="rFooter" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/footer" Target="footer1.xml"/>"#);
    for i in 1..=media.len() {
        let _ = write!(rels, r#"<Relationship Id="rImg{i}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="media/figura{i}.png"/>"#);
    }
    rels.push_str("</Relationships>");
    let core = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><cp:coreProperties xmlns:cp="http://schemas.openxmlformats.org/package/2006/metadata/core-properties" xmlns:dc="http://purl.org/dc/elements/1.1/" xmlns:dcterms="http://purl.org/dc/terms/" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"><dc:title>{}</dc:title><dc:creator>{}</dc:creator><dc:language>{lang_tag}</dc:language></cp:coreProperties>"#,
        xml(&d.title),
        xml(&d.meta.iter().find(|m| m.0 == tr("Autor")).map(|m| m.1.clone()).unwrap_or_else(|| "IAQSB".into()))
    );
    let mut z = iaqsb_core::zip::ZipWriter::new();
    z.add("[Content_Types].xml", ct.as_bytes());
    z.add("_rels/.rels", br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/><Relationship Id="rId2" Type="http://schemas.openxmlformats.org/package/2006/relationships/metadata/core-properties" Target="docProps/core.xml"/></Relationships>"#);
    z.add("docProps/core.xml", core.as_bytes());
    z.add("word/document.xml", document.as_bytes());
    z.add("word/styles.xml", styles.as_bytes());
    z.add("word/footer1.xml", footer.as_bytes());
    z.add("word/_rels/document.xml.rels", rels.as_bytes());
    for (i, m) in media.iter().enumerate() {
        z.add(&format!("word/media/figura{}.png", i + 1), m);
    }
    Ok(z.finish())
}

// ================================================================ PDF

pub use crate::pdf::pdf;
