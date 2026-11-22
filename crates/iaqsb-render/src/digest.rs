//! Resumen del informe en Markdown, pensado para que lo lea un modelo de IA:
//! corto por defecto, con tablas en Markdown, las gráficas descritas en
//! palabras y números, y lo grande (el modelo lineal equivalente, tablas
//! largas, el paso a paso) disponible a pedido por partes.

use crate::{arr, f, num, opt, s};
use serde_json::{json, Map, Value};
use std::fmt::Write;

/// Cuánto detalle incluir.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Detail {
    /// Resultado, métricas y tablas principales (cortas). Sin paso a paso.
    Summary,
    /// Como Summary, pero con el paso a paso completo.
    Steps,
    /// Todo: todas las secciones y el paso a paso.
    Full,
}

#[derive(Debug, Clone)]
pub struct DigestOpts {
    pub detail: Detail,
    /// Filas máximas por tabla.
    pub max_rows: usize,
    /// Identificador para pedir más partes del informe.
    pub report_id: Option<String>,
}

impl Default for DigestOpts {
    fn default() -> Self {
        DigestOpts { detail: Detail::Summary, max_rows: 25, report_id: None }
    }
}

/// Informe completo en Markdown.
pub fn markdown(r: &Value, o: &DigestOpts) -> String {
    if r.get("variables").is_some() && r.get("constraints").is_some() && r.get("objective_value").is_some() {
        return lp_markdown(r, o);
    }
    let mut out = String::new();
    let _ = writeln!(out, "## {} · {}", s(r, "problem"), module_name(s(r, "module")));
    let _ = writeln!(out, "**{}**", s(r, "headline"));
    let _ = writeln!(out, "Estado: {}", status_es(s(r, "status")));
    let mut omitted_steps = vec![];
    let mut first = true;
    for sec in arr(r, "sections") {
        let id = s(sec, "id");
        if id == "fuentes" {
            continue;
        }
        let blocks = arr(sec, "blocks");
        let only_steps = !blocks.is_empty() && blocks.iter().all(|b| s(b, "type") == "steps");
        if only_steps && o.detail == Detail::Summary {
            let n: usize = blocks.iter().map(|b| arr(b, "steps").len()).sum();
            omitted_steps.push((id.to_string(), s(sec, "title").to_string(), n, blocks));
            continue;
        }
        let mut buf = String::new();
        let _ = writeln!(buf, "\n### {} [{}]", s(sec, "title"), id);
        for b in blocks {
            block_md(&mut buf, b, o, 0);
        }
        // en el resumen, las secciones largas (salvo la primera) quedan a pedido
        if o.detail != Detail::Full && !first && !only_steps && buf.len() > 2500 {
            let what: Vec<String> = blocks.iter().map(block_label).collect();
            let _ = writeln!(out, "\n### {} [{}]\nNo se muestra por largo: {}.", s(sec, "title"), id, what.join(" · "));
            if let Some(rid) = &o.report_id {
                let _ = writeln!(out, "Para verla: get_report_part(report_id=\"{rid}\", section=\"{id}\").");
            }
        } else {
            out.push_str(&buf);
        }
        first = false;
    }
    for (id, title, n, blocks) in omitted_steps {
        let titles: Vec<&str> = blocks.iter().flat_map(|b| arr(b, "steps").iter().map(|st| s(st, "title"))).take(30).collect();
        let _ = writeln!(out, "\n### {title} [{id}]\n{n} pasos sin mostrar: {}{}.", titles.join(" · "), if n > 30 { " …" } else { "" });
        if let Some(rid) = &o.report_id {
            let _ = writeln!(out, "Para verlos: get_report_part(report_id=\"{rid}\", section=\"{id}\") o un paso con step=número.");
        }
    }
    engine_md(&mut out, r);
    sources_md(&mut out, r);
    data_md(&mut out, r.get("data"), o);
    footer(&mut out, r, o);
    out
}

fn footer(out: &mut String, r: &Value, o: &DigestOpts) {
    if let Some(rid) = &o.report_id {
        let secs: Vec<String> = arr(r, "sections").iter().map(|x| format!("{} ({})", s(x, "id"), s(x, "title"))).collect();
        let _ = writeln!(out, "\n---\nreport_id: {rid}");
        if !secs.is_empty() {
            let _ = writeln!(out, "Secciones: {}.", secs.join(", "));
        }
        let _ = writeln!(out, "Más detalle con get_report_part: section, step, path (p. ej. data.lp_model) o images=true para las gráficas.");
    }
}

fn status_es(st: &str) -> &str {
    match st {
        "optimal" => "óptimo",
        "feasible" => "factible (no se garantiza el óptimo)",
        "infeasible" => "no factible",
        "unbounded" => "no acotado",
        "solved" => "resuelto",
        other => other,
    }
}

/// Nombre del módulo en español (para títulos).
pub fn module_name_pub(m: &str) -> &str {
    module_name(m)
}

fn module_name(m: &str) -> &str {
    match m {
        "lp" => "programación lineal y entera",
        "net" => "redes",
        "pert" => "PERT/CPM",
        "gp" => "programación por metas",
        "qp" => "programación cuadrática",
        "nlp" => "programación no lineal",
        "dp" => "programación dinámica",
        "ap" => "planificación agregada",
        "da" => "análisis de decisiones",
        "mkp" => "procesos de Markov",
        "its" => "inventarios",
        "fc" => "pronósticos",
        "st" => "series de tiempo",
        "qa" => "teoría de colas",
        "qss" => "simulación de colas",
        "job" => "programación de trabajos",
        "mrp" => "MRP",
        "pmp" => "programa maestro",
        "mrp2" => "MRP II (Gozinto)",
        "fll" => "localización y distribución",
        "qcc" => "control de calidad",
        "asa" => "muestreo de aceptación",
        "vrp" => "ruteo de vehículos",
        "modo" => "modo de transporte",
        "inv" => "gestión de inventarios",
        "fin" => "finanzas del inventario",
        "alm" => "almacenes",
        "kpi" => "indicadores logísticos",
        other => other,
    }
}

// ---------------------------------------------------------------- bloques

fn block_label(b: &Value) -> String {
    let t = opt(b, "title");
    match s(b, "type") {
        "table" => format!("tabla{} ({} filas)", t.map(|x| format!(" «{x}»")).unwrap_or_default(), arr(b, "rows").len()),
        "text" => "texto".into(),
        "metrics" => "métricas".into(),
        "steps" => format!("{} pasos", arr(b, "steps").len()),
        other => format!("gráfica {other}{}", t.map(|x| format!(" «{x}»")).unwrap_or_default()),
    }
}

fn cell(v: &Value) -> String {
    match v {
        Value::Null => String::new(),
        Value::Number(x) => num(x.as_f64().unwrap_or(f64::NAN), 4).replace('−', "-"),
        Value::String(t) => t.replace('|', "\\|").replace('\n', " "),
        Value::Bool(b) => if *b { "sí".into() } else { "no".into() },
        other => other.to_string(),
    }
}

fn indent(depth: usize) -> String {
    "  ".repeat(depth)
}

fn block_md(out: &mut String, b: &Value, o: &DigestOpts, depth: usize) {
    let ind = indent(depth);
    match s(b, "type") {
        "metrics" => {
            if let Some(t) = opt(b, "title") {
                let _ = writeln!(out, "{ind}**{t}**");
            }
            let items: Vec<String> = arr(b, "items")
                .iter()
                .map(|m| {
                    let v = cell(m.get("value").unwrap_or(&Value::Null));
                    let u = opt(m, "unit").map(|u| format!(" {u}")).unwrap_or_default();
                    if m.get("emphasis").and_then(|e| e.as_bool()) == Some(true) {
                        format!("**{}: {v}{u}**", s(m, "label"))
                    } else {
                        format!("{}: {v}{u}", s(m, "label"))
                    }
                })
                .collect();
            let _ = writeln!(out, "{ind}{}", items.join(" · "));
        }
        "text" => {
            if let Some(t) = opt(b, "title") {
                let _ = writeln!(out, "{ind}**{t}** {}", s(b, "body").replace('\n', " "));
            } else {
                let _ = writeln!(out, "{ind}{}", s(b, "body").replace('\n', &format!("\n{ind}")));
            }
        }
        "table" => table_md(out, b, o.max_rows, &ind),
        "chart" => chart_md(out, b, &ind),
        "network" => network_md(out, b, &ind),
        "map" => {
            let _ = writeln!(out, "{ind}[Gráfica: mapa de rutas «{}»]", s(b, "title"));
            for r in arr(b, "routes") {
                let stops: Vec<&str> = arr(r, "stops").iter().filter_map(|x| x.as_str()).collect();
                let _ = writeln!(out, "{ind}- {}: {}{}", s(r, "name"), stops.join(" → "), opt(r, "note").map(|n| format!(" · {n}")).unwrap_or_default());
            }
        }
        "gantt" => {
            let rows = arr(b, "rows");
            let _ = writeln!(out, "{ind}[Gráfica: diagrama de Gantt «{}», en {}]", s(b, "title"), s(b, "unit"));
            for r in rows.iter().take(40) {
                let bars: Vec<String> = arr(r, "bars")
                    .iter()
                    .map(|x| format!("{}–{}{}", num(f(x, "start"), 3), num(f(x, "end"), 3), opt(x, "label").map(|l| format!(" {l}")).unwrap_or_default()))
                    .collect();
                let _ = writeln!(out, "{ind}- {}: {}", s(r, "label"), bars.join(", "));
            }
            if rows.len() > 40 {
                let _ = writeln!(out, "{ind}- … {} filas más", rows.len() - 40);
            }
            let mk: Vec<String> = arr(b, "markers").iter().map(|m| format!("{} = {}", s(m, "label"), num(f(m, "value"), 3))).collect();
            if !mk.is_empty() {
                let _ = writeln!(out, "{ind}Marcas: {}", mk.join(", "));
            }
        }
        "layout" => {
            let rects = arr(b, "rects");
            let solid = rects.iter().filter(|r| r.get("soft").and_then(|x| x.as_bool()) != Some(true)).count();
            let _ = writeln!(
                out,
                "{ind}[Gráfica: plano «{}» de {} × {} {}, con {solid} elementos]",
                s(b, "title"),
                num(f(b, "width"), 3),
                num(f(b, "height"), 3),
                s(b, "unit")
            );
            let dims: Vec<&str> = arr(b, "dims").iter().map(|d| s(d, "label")).collect();
            if !dims.is_empty() {
                let _ = writeln!(out, "{ind}Cotas: {}", dims.join(", "));
            }
        }
        "tree" => tree_md(out, b, &ind),
        "steps" => {
            let steps = arr(b, "steps");
            for (k, st) in steps.iter().enumerate() {
                let _ = writeln!(out, "\n{ind}**[{}/{}] {}**", k + 1, steps.len(), s(st, "title"));
                step_body(out, st, o, depth);
            }
        }
        "step" => {
            let _ = writeln!(out, "{ind}**[{}/{}] {}**", b["number"], b["of"], s(b, "title"));
            step_body(out, b, o, depth);
        }
        "sources" => {}
        _ => {}
    }
}

fn step_body(out: &mut String, st: &Value, o: &DigestOpts, depth: usize) {
    if let Some(d) = opt(st, "description") {
        let _ = writeln!(out, "{}{d}", indent(depth));
    }
    for x in arr(st, "blocks") {
        block_md(out, x, &DigestOpts { max_rows: o.max_rows.min(15), ..o.clone() }, depth);
    }
}

fn table_md(out: &mut String, b: &Value, max_rows: usize, ind: &str) {
    let cols = arr(b, "columns");
    let rows = arr(b, "rows");
    if let Some(t) = opt(b, "title") {
        let _ = writeln!(out, "{ind}**{t}**");
    }
    let head: Vec<String> = cols.iter().map(|c| cell(c.get("label").unwrap_or(&Value::Null))).collect();
    let _ = writeln!(out, "{ind}| {} |", head.join(" | "));
    let _ = writeln!(out, "{ind}|{}|", cols.iter().map(|c| if s(c, "align") == "left" { " :-- " } else { " --: " }).collect::<Vec<_>>().join("|"));
    let marks: Vec<(usize, usize)> = arr(b, "marks").iter().map(|m| (m["row"].as_u64().unwrap_or(0) as usize, m["col"].as_u64().unwrap_or(0) as usize)).collect();
    for (ri, r) in rows.iter().take(max_rows).enumerate() {
        let strong = matches!(s(r, "tone"), "highlight" | "accent" | "total");
        let cells: Vec<String> = arr(r, "cells")
            .iter()
            .enumerate()
            .map(|(ci, c)| {
                let t = cell(c);
                if (strong || marks.contains(&(ri, ci))) && !t.is_empty() {
                    format!("**{t}**")
                } else {
                    t
                }
            })
            .collect();
        let _ = writeln!(out, "{ind}| {} |", cells.join(" | "));
    }
    if rows.len() > max_rows {
        let _ = writeln!(out, "{ind}… y {} filas más.", rows.len() - max_rows);
    }
    if let Some(n) = opt(b, "note") {
        let _ = writeln!(out, "{ind}{n}");
    }
}

fn chart_md(out: &mut String, c: &Value, ind: &str) {
    let cats: Option<Vec<&str>> = c.get("categories").and_then(|v| v.as_array()).map(|a| a.iter().map(|x| x.as_str().unwrap_or("")).collect());
    let _ = writeln!(
        out,
        "{ind}[Gráfica: «{}», {} según {}]",
        opt(c, "title").unwrap_or("gráfico"),
        s(c, "y_label"),
        s(c, "x_label")
    );
    for sr in arr(c, "series") {
        let pts: Vec<(f64, f64)> = arr(sr, "points")
            .iter()
            .map(|p| (p.get(0).and_then(|v| v.as_f64()).unwrap_or(f64::NAN), p.get(1).and_then(|v| v.as_f64()).unwrap_or(f64::NAN)))
            .filter(|p| p.0.is_finite() && p.1.is_finite())
            .collect();
        let kind = match s(sr, "kind") {
            "bar" => "barras",
            "scatter" => "puntos",
            "step" => "escalera",
            "area" => "área",
            _ => "línea",
        };
        let xl = |x: f64| match &cats {
            Some(c) => c.get(x.round() as usize).map(|t| t.to_string()).unwrap_or_else(|| num(x, 3)),
            None => num(x, 3),
        };
        if pts.len() <= 24 {
            let list: Vec<String> = pts.iter().map(|p| format!("{} → {}", xl(p.0), num(p.1, 4))).collect();
            let _ = writeln!(out, "{ind}- {} ({kind}): {}", s(sr, "name"), list.join("; "));
        } else {
            let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
            let (mut alo, mut ahi) = (0.0, 0.0);
            for p in &pts {
                if p.1 < lo {
                    lo = p.1;
                    alo = p.0;
                }
                if p.1 > hi {
                    hi = p.1;
                    ahi = p.0;
                }
            }
            let _ = writeln!(
                out,
                "{ind}- {} ({kind}, {} puntos): de {} a {}; mínimo {} en {}, máximo {} en {}",
                s(sr, "name"),
                pts.len(),
                xl(pts[0].0),
                xl(pts[pts.len() - 1].0),
                num(lo, 4),
                xl(alo),
                num(hi, 4),
                xl(ahi)
            );
        }
    }
    let refs: Vec<String> = arr(c, "hlines")
        .iter()
        .chain(arr(c, "vlines").iter())
        .map(|h| {
            let (l, v) = (s(h, "label"), num(f(h, "value"), 4));
            if l.contains('=') {
                l.to_string()
            } else if l.is_empty() || l == v {
                v
            } else {
                format!("{l} = {v}")
            }
        })
        .collect();
    if !refs.is_empty() {
        let _ = writeln!(out, "{ind}Líneas de referencia: {}", refs.join(", "));
    }
}

fn network_md(out: &mut String, n: &Value, ind: &str) {
    let nodes = arr(n, "nodes");
    let edges = arr(n, "edges");
    let label: std::collections::HashMap<&str, &str> = nodes.iter().map(|x| (s(x, "id"), s(x, "label"))).collect();
    let l = |id: &str| label.get(id).copied().unwrap_or(id).to_string();
    let sep = if n.get("directed").and_then(|d| d.as_bool()) == Some(true) { "→" } else { "–" };
    let hot: Vec<String> = edges
        .iter()
        .filter(|e| matches!(s(e, "tone"), "highlight" | "accent" | "good"))
        .map(|e| format!("{}{sep}{}{}", l(s(e, "from")), l(s(e, "to")), opt(e, "label").map(|x| format!(" ({x})")).unwrap_or_default()))
        .collect();
    let _ = writeln!(out, "{ind}[Gráfica: red «{}», {} nodos y {} arcos]", opt(n, "title").unwrap_or("red"), nodes.len(), edges.len());
    if !hot.is_empty() && hot.len() <= 60 {
        let _ = writeln!(out, "{ind}Arcos destacados: {}", hot.join(", "));
    }
    let notes: Vec<String> = nodes.iter().filter_map(|x| opt(x, "note").map(|t| format!("{}: {}", s(x, "label"), t.replace('\n', " · ")))).collect();
    if !notes.is_empty() && notes.len() <= 40 {
        let _ = writeln!(out, "{ind}Nodos: {}", notes.join("; "));
    }
}

fn tree_md(out: &mut String, t: &Value, ind: &str) {
    let nodes = arr(t, "nodes");
    let _ = writeln!(out, "{ind}[Gráfica: árbol «{}», {} nodos]", opt(t, "title").unwrap_or("árbol"), nodes.len());
    let id = |v: &Value| v.get("id").and_then(|x| x.as_u64()).unwrap_or(0);
    let parent = |v: &Value| v.get("parent").and_then(|x| x.as_u64());
    fn walk(out: &mut String, nodes: &[Value], p: Option<u64>, depth: usize, ind: &str, count: &mut usize, id: &dyn Fn(&Value) -> u64, parent: &dyn Fn(&Value) -> Option<u64>) {
        for nd in nodes.iter().filter(|x| parent(x) == p) {
            *count += 1;
            if *count > 80 {
                return;
            }
            let edge = opt(nd, "edge").map(|e| format!("[{e}] ")).unwrap_or_default();
            let val = opt(nd, "value").map(|v| format!(" = {v}")).unwrap_or_default();
            let mark = if matches!(s(nd, "tone"), "highlight" | "accent") { " ◀" } else { "" };
            let _ = writeln!(out, "{ind}{}- {edge}{}{val}{mark}", "  ".repeat(depth), s(nd, "label"));
            walk(out, nodes, Some(id(nd)), depth + 1, ind, count, id, parent);
        }
    }
    let mut count = 0;
    walk(out, nodes, None, 0, ind, &mut count, &id, &parent);
}

/// Motor usado, diagnóstico de infactibilidad y avisos.
fn engine_md(out: &mut String, r: &Value) {
    if let Some(e) = r.get("engine").filter(|e| e.is_object()) {
        let mut parts = vec![format!("Motor: {}", s(e, "solver"))];
        if let Some(t) = e.get("time").and_then(|v| v.as_f64()) {
            parts.push(format!("{} s", num(t, 3)));
        }
        if let Some(n) = e.get("runs").and_then(|v| v.as_u64()).filter(|n| *n > 1) {
            parts.push(format!("{n} resoluciones"));
        }
        if let (Some(c), Some(rw)) = (e.get("cols").and_then(|v| v.as_u64()), e.get("rows").and_then(|v| v.as_u64())) {
            parts.push(format!("{c} variables × {rw} restricciones"));
        }
        if e.get("is_mip").and_then(|v| v.as_bool()) == Some(true) || e.get("gap").map_or(false, |g| g.is_number()) {
            if let Some(b) = e.get("bound").and_then(|v| v.as_f64()) {
                parts.push(format!("cota {}", num(b, 6)));
            }
            if let Some(g) = e.get("gap").and_then(|v| v.as_f64()) {
                parts.push(format!("brecha {} %", num(100.0 * g, 4)));
            }
            if let Some(n) = e.get("nodes").and_then(|v| v.as_i64()).filter(|n| *n > 0) {
                parts.push(format!("{n} nodos"));
            }
        }
        let _ = writeln!(out, "\n{}.", parts.join(" · "));
    }
    if let Some(d) = r.get("diagnosis").filter(|d| d.is_object()) {
        let _ = writeln!(out, "\n### Por qué no es factible");
        let conflict = arr(d, "conflict");
        if !conflict.is_empty() {
            let _ = writeln!(out, "Subsistema irreducible infactible (basta quitar o relajar uno para romper el conflicto):");
            for c in conflict {
                let _ = writeln!(out, "- {}", s(c, "text"));
            }
        }
        let relax = arr(d, "relaxation");
        if !relax.is_empty() {
            let _ = writeln!(out, "Menor cambio que lo vuelve factible (relajación de factibilidad, total {}):", d.get("relaxation_total").map(cell).unwrap_or_default());
            for c in relax {
                let _ = writeln!(out, "- {}", s(c, "text"));
            }
        }
    }
    for n in arr(r, "notes") {
        if let Some(t) = n.as_str() {
            let _ = writeln!(out, "Nota: {t}");
        }
    }
    if let Some(ov) = r.get("objective_values").and_then(|v| v.as_array()).filter(|v| !v.is_empty()) {
        let t: Vec<String> = ov.iter().filter_map(|p| p.as_array()).map(|p| format!("{} = {}", p.first().and_then(|x| x.as_str()).unwrap_or(""), p.get(1).map(cell).unwrap_or_default())).collect();
        let _ = writeln!(out, "Objetivos: {}.", t.join(", "));
    }
}

fn sources_md(out: &mut String, r: &Value) {
    let refs = arr(r, "references");
    let ds = r.get("data_source").filter(|d| !d.is_null());
    if refs.is_empty() && ds.is_none() {
        return;
    }
    let _ = writeln!(out, "\n### Fuentes (APA 7)");
    if let Some(d) = ds {
        let _ = writeln!(out, "Datos: {} — {}{}", s(d, "origin_label"), s(d, "summary"), if d.get("verified").and_then(|v| v.as_bool()) == Some(false) { " (por verificar)" } else { "" });
    }
    for c in refs {
        let used: Vec<&str> = arr(c, "used_for").iter().filter_map(|x| x.as_str()).collect();
        let pend = if c.get("verified").and_then(|v| v.as_bool()) == Some(false) { " · por verificar" } else { "" };
        let _ = writeln!(out, "- {}: {}{pend}", s(c, "cite"), used.join("; "));
    }
    let _ = writeln!(out, "Referencias completas y BibTeX: get_bibliography.");
}

fn data_md(out: &mut String, data: Option<&Value>, o: &DigestOpts) {
    let Some(d) = data.filter(|d| !d.is_null() && d.as_object().map(|m| !m.is_empty()).unwrap_or(true)) else { return };
    let (c, omitted) = compact(d, if o.detail == Detail::Full { 4000 } else { 1200 }, "data");
    let text = serde_json::to_string(&c).unwrap_or_default();
    let _ = writeln!(out, "\n### Datos (JSON)\n```json\n{text}\n```");
    if !omitted.is_empty() {
        let _ = writeln!(out, "Omitidos por tamaño: {} (pídelos con get_report_part y path).", omitted.join(", "));
    }
}

/// Copia del valor en la que todo objeto o lista cuyo JSON pasa de `limit`
/// caracteres se cambia por un aviso. Devuelve también las rutas omitidas.
pub fn compact(v: &Value, limit: usize, path: &str) -> (Value, Vec<String>) {
    let mut omitted = vec![];
    let out = compact_in(v, limit, path, &mut omitted, true);
    (out, omitted)
}

fn compact_in(v: &Value, limit: usize, path: &str, om: &mut Vec<String>, top: bool) -> Value {
    let size = serde_json::to_string(v).map(|t| t.len()).unwrap_or(0);
    match v {
        Value::Object(m) => {
            if size <= limit {
                return v.clone();
            }
            let mut out = Map::new();
            for (k, x) in m {
                let p = format!("{path}.{k}");
                let xs = serde_json::to_string(x).map(|t| t.len()).unwrap_or(0);
                let _ = xs;
                out.insert(k.clone(), compact_in(x, limit, &p, om, false));
            }
            Value::Object(out)
        }
        Value::Array(a) if size > limit && a.len() <= 30 && a.iter().any(|x| x.is_object()) => {
            Value::Array(a.iter().enumerate().map(|(i, x)| compact_in(x, limit, &format!("{path}.{i}"), om, false)).collect())
        }
        Value::Array(a) if size > limit => {
            // lista larga de números cortos: se deja el comienzo
            if a.iter().all(|x| x.is_number() || x.is_string() && x.as_str().unwrap().len() < 24) && !top {
                let mut keep = vec![];
                let mut len = 2;
                for x in a {
                    len += x.to_string().len() + 1;
                    if len > limit / 2 {
                        break;
                    }
                    keep.push(x.clone());
                }
                om.push(path.to_string());
                keep.push(json!(format!("«… {} elementos en total, path={path}»", a.len())));
                return Value::Array(keep);
            }
            om.push(path.to_string());
            json!(format!("«omitido: lista de {} elementos, {} KB, path={path}»", a.len(), size.div_ceil(1024)))
        }
        Value::String(t) if t.len() > limit => {
            om.push(path.to_string());
            let cut: String = t.chars().take(limit / 2).collect();
            json!(format!("{cut}… «texto de {} caracteres, path={path}»", t.chars().count()))
        }
        other => other.clone(),
    }
}

// ---------------------------------------------------------------- partes

/// Una parte del informe: una sección, un paso o un valor de los datos.
pub fn part(r: &Value, section: Option<&str>, step: Option<&str>, path: Option<&str>) -> Result<Value, String> {
    if let Some(p) = path {
        let ptr = format!("/{}", p.trim_start_matches('/').replace('.', "/"));
        return r.pointer(&ptr).cloned().ok_or_else(|| format!("no hay nada en {p}"));
    }
    let secs = arr(r, "sections");
    let sec = match section {
        Some(sid) => secs
            .iter()
            .find(|x| s(x, "id") == sid || s(x, "title").eq_ignore_ascii_case(sid))
            .ok_or_else(|| format!("no hay sección «{sid}»; hay: {}", secs.iter().map(|x| s(x, "id")).collect::<Vec<_>>().join(", ")))?,
        None if step.is_some() => secs
            .iter()
            .find(|x| arr(x, "blocks").iter().any(|b| s(b, "type") == "steps"))
            .ok_or("este informe no tiene paso a paso")?,
        None => return Ok(r.clone()),
    };
    let Some(st) = step else { return Ok(sec.clone()) };
    let steps: Vec<&Value> = arr(sec, "blocks").iter().filter(|b| s(b, "type") == "steps").flat_map(|b| arr(b, "steps").iter()).collect();
    let found = match st.parse::<usize>() {
        Ok(k) if k >= 1 && k <= steps.len() => Some(steps[k - 1]),
        _ => steps.iter().copied().find(|x| s(x, "title").to_lowercase().contains(&st.to_lowercase())),
    };
    let x = found.ok_or_else(|| format!("no hay paso «{st}»; hay {} pasos", steps.len()))?;
    let k = steps.iter().position(|y| std::ptr::eq(*y, x)).unwrap_or(0);
    let mut v = x.clone();
    v["type"] = json!("step");
    v["number"] = json!(k + 1);
    v["of"] = json!(steps.len());
    Ok(v)
}

/// Markdown de una parte (sección o paso) con todo su detalle.
pub fn part_markdown(v: &Value, max_rows: usize) -> String {
    let o = DigestOpts { detail: Detail::Full, max_rows, report_id: None };
    let mut out = String::new();
    if v.get("blocks").is_some() && v.get("type").is_none() {
        let _ = writeln!(out, "### {} [{}]", s(v, "title"), s(v, "id"));
        for b in arr(v, "blocks") {
            block_md(&mut out, b, &o, 0);
        }
    } else if v.get("type").is_some() {
        block_md(&mut out, v, &o, 0);
    } else {
        let _ = writeln!(out, "```json\n{}\n```", serde_json::to_string(v).unwrap_or_default());
    }
    out
}

// ---------------------------------------------------------------- programación lineal

fn lp_markdown(r: &Value, o: &DigestOpts) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "## {} · programación lineal y entera", s(r, "problem"));
    let _ = writeln!(out, "**{}**", s(r, "headline"));
    let _ = writeln!(out, "Estado: {} · {}", status_es(s(r, "status")), s(r, "message"));
    if let Some(z) = r.get("objective_value").and_then(|v| v.as_f64()).filter(|_| !s(r, "headline").contains("Z =")) {
        let _ = writeln!(out, "**{} Z = {}**", if s(r, "objective") == "max" { "Máximo" } else { "Mínimo" }, num(z, 6));
    }
    if r.get("alternate_optima").and_then(|v| v.as_bool()) == Some(true) {
        let _ = writeln!(out, "Hay soluciones óptimas alternativas.");
    }
    if let Some(bb) = r.get("branch_and_bound").filter(|b| b.is_object()) {
        let _ = writeln!(out, "Branch and bound: {} nodos, relajación lineal = {}.", bb["nodes_explored"], num(f(bb, "relaxed_objective"), 6));
    }
    let vars = arr(r, "variables");
    let big = vars.len() > o.max_rows;
    let shown: Vec<&Value> = vars.iter().filter(|v| !big || f(v, "value").abs() > 1e-9).collect();
    let _ = writeln!(
        out,
        "\n### Variables{}",
        if big { format!(" (solo las {} distintas de cero, de {})", shown.len(), vars.len()) } else { String::new() }
    );
    let full = o.detail != Detail::Summary;
    if full {
        let _ = writeln!(out, "| Variable | Valor | c(j) | Costo reducido | c(j) mín | c(j) máx |\n| :-- | --: | --: | --: | --: | --: |");
    } else {
        let _ = writeln!(out, "| Variable | Valor | c(j) | Costo reducido |\n| :-- | --: | --: | --: |");
    }
    for v in shown.iter().take(200) {
        let c = |k: &str| v.get(k).map(cell).unwrap_or_default();
        if full {
            let _ = writeln!(out, "| {} | {} | {} | {} | {} | {} |", s(v, "name"), c("value"), c("cost"), c("reduced_cost"), c("cost_min"), c("cost_max"));
        } else {
            let _ = writeln!(out, "| {} | {} | {} | {} |", s(v, "name"), c("value"), c("cost"), c("reduced_cost"));
        }
    }
    let cons = arr(r, "constraints");
    let binding: Vec<&Value> = cons.iter().filter(|c| f(c, "slack_or_surplus").abs() < 1e-9).collect();
    let show_all = full || cons.len() <= o.max_rows;
    let list: Vec<&Value> = if show_all { cons.iter().collect() } else { binding.iter().copied().take(o.max_rows).collect() };
    let _ = writeln!(
        out,
        "\n### Restricciones{}",
        if show_all { String::new() } else { format!(" (activas: {} de {}; se muestran {})", binding.len(), cons.len(), list.len()) }
    );
    let _ = writeln!(out, "| Restricción | Lado izq. | Relación | Lado der. | Holgura | Precio sombra |\n| :-- | --: | :-: | --: | --: | --: |");
    for c in list {
        let g = |k: &str| c.get(k).map(cell).unwrap_or_default();
        let _ = writeln!(out, "| {} | {} | {} | {} | {} | {} |", s(c, "name"), g("lhs"), s(c, "direction"), g("rhs"), g("slack_or_surplus"), g("shadow_price"));
    }
    // modelo algebraico: la solución por índices
    for t in arr(r, "tables") {
        block_md(&mut out, t, o, 0);
    }
    engine_md(&mut out, r);
    sources_md(&mut out, r);
    if let Some(rid) = &o.report_id {
        let _ = writeln!(out, "\n---\nreport_id: {rid}\nMás detalle con get_report_part: path=variables, path=constraints o path=sensitivity.");
    }
    out
}
