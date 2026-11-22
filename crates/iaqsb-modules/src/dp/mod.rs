//! DP · Programación dinámica (equivalente al módulo DP de WinQSB).
//!
//! Tres problemas clásicos resueltos por recursión hacia atrás, con las tablas
//! de cada etapa (estado, decisiones, f*, decisión óptima) como en los libros:
//!
//! - Diligencia (ruta más corta por etapas) desde cualquier nodo al destino.
//! - Mochila con unidades limitadas y función de rendimiento de cualquier forma.
//! - Planificación de producción e inventario con costo de preparación,
//!   capacidades y función de costo variable en P (producción), H (inventario)
//!   y B (pedidos pendientes).
//!
//! Análisis «¿qué pasa si?»: se pueden fijar decisiones de algunas etapas.

use iaqsb_core::expr::{Expr, Vars};
use iaqsb_core::winqsb::{num, opt, parse_num, Reader, Writer};
use iaqsb_core::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum DpType {
    #[default]
    Stagecoach,
    Knapsack,
    Production,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DpNode {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Item {
    pub name: String,
    /// Unidades disponibles (vacío = las que quepan).
    #[serde(default)]
    pub available: Option<f64>,
    /// Capacidad que ocupa cada unidad.
    pub capacity: f64,
    /// Rendimiento según la cantidad, p. ej. "50X", "3X+100", "2.15X^2+5".
    pub function: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Period {
    pub name: String,
    pub demand: f64,
    /// Capacidad de producción (vacío = sin límite).
    #[serde(default)]
    pub production_capacity: Option<f64>,
    /// Capacidad de almacén (vacío = sin límite).
    #[serde(default)]
    pub storage_capacity: Option<f64>,
    #[serde(default)]
    pub setup_cost: f64,
    /// Costo variable en P (producción), H (inventario final) y B (pendientes), p. ej. "5P+2H+10B".
    pub cost_function: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DpModel {
    #[serde(default)]
    pub name: String,
    pub problem_type: DpType,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub nodes: Vec<DpNode>,
    /// Distancia de i a j (null = sin conexión).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub distances: Vec<Vec<Option<f64>>>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub items: Vec<Item>,
    /// Capacidad de la mochila.
    #[serde(default)]
    pub capacity: f64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub periods: Vec<Period>,
    #[serde(default)]
    pub initial_inventory: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct DpOptions {
    pub steps: bool,
    /// Diligencia: nodo de origen (vacío = el primero).
    pub origin: Option<String>,
    /// Diligencia: nodo de destino (vacío = el último).
    pub destination: Option<String>,
    /// Decisiones fijadas por etapa ("¿qué pasa si?"): "2, , 3" (vacío = libre).
    pub preset: Option<String>,
}

impl Default for DpOptions {
    fn default() -> Self {
        DpOptions { steps: true, origin: None, destination: None, preset: None }
    }
}

const MAX_STATES: usize = 200_000;

fn preset_list(o: &DpOptions, n: usize) -> Result<Vec<Option<f64>>, ModError> {
    let mut v = vec![None; n];
    if let Some(s) = o.preset.as_ref().filter(|s| !s.trim().is_empty()) {
        for (k, part) in s.split([',', ';']).enumerate() {
            if k >= n {
                break;
            }
            let p = part.trim();
            if !p.is_empty() {
                v[k] = Some(parse_num(p).ok_or_else(|| ModError::Invalid(format!("decisión fijada «{p}» no es un número")))?);
            }
        }
    }
    Ok(v)
}

/// Compila una función de una sola variable (la cantidad), sea cual sea su nombre.
fn one_var(src: &str, at: &str) -> Result<Expr, ModError> {
    let mut v = Vars::open();
    let e = Expr::parse(src, &mut v).map_err(|e| ModError::Invalid(format!("{at}: {e}")))?;
    if v.len() > 1 {
        return invalid(format!("{at}: la función debe tener una sola variable (la cantidad), y tiene {}", v.names.join(", ")));
    }
    Ok(e)
}

pub struct Dp;

impl TypedModule for Dp {
    type Model = DpModel;
    type Options = DpOptions;

    fn file_tag(&self) -> &'static str {
        "DP"
    }

    fn spec(&self) -> ModuleSpec {
        ModuleSpec {
            id: "dp".into(),
            winqsb: "DP".into(),
            name: "Programación dinámica".into(),
            description: "Diligencia (ruta más corta), mochila y planificación de producción e inventario".into(),
            file_ext: "DPP".into(),
            problem_types: vec![
                ProblemType {
                    id: "stagecoach".into(),
                    label: "Diligencia (ruta más corta)".into(),
                    description: "La ruta más corta de cualquier nodo al destino, etapa por etapa".into(),
                    dims: vec![Field::integer("nodes", "Número de nodos", 10)],
                    layout: vec![Widget::matrix(
                        Some("Distancias (vacío = sin conexión)"),
                        "distances",
                        Axis::new("nodes", "Desde").add("Añadir nodo"),
                        Axis::new("nodes", "Hasta"),
                        Field::number("d", "Distancia").optional(),
                    )],
                },
                ProblemType {
                    id: "knapsack".into(),
                    label: "Mochila".into(),
                    description: "Cuántas unidades de cada artículo llevar para obtener el mayor rendimiento".into(),
                    dims: vec![Field::integer("items", "Número de artículos", 3)],
                    layout: vec![
                        Widget::params(None, vec![Field::number("capacity", "Capacidad de la mochila")]),
                        Widget::records(
                            Some("Artículos (etapas)"),
                            "items",
                            vec![
                                Field::text("name", "Artículo"),
                                Field::number("available", "Unidades disponibles").optional().help("Vacío = las que quepan"),
                                Field::number("capacity", "Capacidad por unidad"),
                                Field::expr("function", "Rendimiento (X = unidades)").help("Ej.: 50X, 3X+100, 2.15X^2+5"),
                            ],
                            Some("Añadir artículo"),
                        ),
                    ],
                },
                ProblemType {
                    id: "production".into(),
                    label: "Producción e inventario".into(),
                    description: "Cuánto producir en cada periodo para cubrir la demanda al menor costo".into(),
                    dims: vec![Field::integer("periods", "Número de periodos", 4)],
                    layout: vec![
                        Widget::params(None, vec![Field::number("initial_inventory", "Inventario inicial")]),
                        Widget::records(
                            Some("Periodos (etapas)"),
                            "periods",
                            vec![
                                Field::text("name", "Periodo"),
                                Field::number("demand", "Demanda"),
                                Field::number("production_capacity", "Capacidad de producción").optional(),
                                Field::number("storage_capacity", "Capacidad de almacén").optional(),
                                Field::number("setup_cost", "Costo de preparación"),
                                Field::expr("cost_function", "Costo variable (P, H, B)").help("P producción, H inventario final, B pendientes. Ej.: 5P+2H+10B"),
                            ],
                            Some("Añadir periodo"),
                        ),
                    ],
                },
            ],
            options: vec![
                Field::boolean("steps", "Mostrar paso a paso", true),
                Field::text("origin", "Nodo de origen").optional().help("Vacío = el primero").show_if("problem_type", &["stagecoach"]),
                Field::text("destination", "Nodo de destino").optional().help("Vacío = el último").show_if("problem_type", &["stagecoach"]),
                Field::text("preset", "Decisiones fijadas (¿qué pasa si?)").optional().help("Una por etapa separadas por comas; vacío = libre. Ej.: 2, , 3").show_if("problem_type", &["knapsack", "production"]),
            ],
        }
    }

    fn template(&self, pt: &str, dims: &Value) -> Result<DpModel, ModError> {
        let t: DpType = serde_json::from_value(Value::from(pt)).map_err(|_| ModError::Invalid(format!("tipo de problema desconocido: {pt}")))?;
        let mut m = DpModel { name: "Programación dinámica nueva".into(), problem_type: t, nodes: vec![], distances: vec![], items: vec![], capacity: 10.0, periods: vec![], initial_inventory: 0.0 };
        match t {
            DpType::Stagecoach => {
                let n = dim(dims, "nodes", 10, 2, 500)?;
                m.nodes = (1..=n).map(|i| DpNode { name: format!("Nodo{i}") }).collect();
                m.distances = vec![vec![None; n]; n];
            }
            DpType::Knapsack => {
                let n = dim(dims, "items", 3, 1, 200)?;
                m.items = (1..=n).map(|i| Item { name: format!("Artículo{i}"), available: None, capacity: 1.0, function: "X".into() }).collect();
            }
            DpType::Production => {
                let n = dim(dims, "periods", 4, 1, 200)?;
                m.periods = (1..=n).map(|i| Period { name: format!("Periodo{i}"), demand: 1.0, production_capacity: None, storage_capacity: None, setup_cost: 0.0, cost_function: "P+H".into() }).collect();
            }
        }
        Ok(m)
    }

    fn solve(&self, m: &DpModel, o: &DpOptions) -> Result<Report, ModError> {
        let mut r = match m.problem_type {
            DpType::Stagecoach => stagecoach(m, o)?,
            DpType::Knapsack => knapsack(m, o)?,
            DpType::Production => production(m, o)?,
        };
        if r.problem.is_empty() {
            r.problem = m.name.clone();
        }
        Ok(r)
    }

    fn read(&self, bytes: &[u8]) -> Result<DpModel, ModError> {
        read_dp(bytes)
    }

    fn write(&self, m: &DpModel) -> Result<Vec<u8>, ModError> {
        Ok(write_dp(m))
    }

    fn examples(&self) -> Vec<(&'static str, DpModel)> {
        let ex = |s: &str| serde_json::from_str::<DpModel>(s).unwrap();
        vec![
            ("diligencia", ex(include_str!("ejemplos/diligencia.json"))),
            ("carga_avion", ex(include_str!("ejemplos/mochila.json"))),
            ("produccion", ex(include_str!("ejemplos/produccion.json"))),
        ]
    }
}

// ------------------------------------------------------------------ diligencia

fn stagecoach(m: &DpModel, o: &DpOptions) -> Result<Report, ModError> {
    let n = m.nodes.len();
    check_names("nodos", &m.nodes.iter().map(|x| &x.name).collect::<Vec<_>>())?;
    if n < 2 {
        return invalid("hacen falta al menos dos nodos");
    }
    let mut d = m.distances.clone();
    d.resize(n, vec![]);
    for r in d.iter_mut() {
        r.resize(n, None);
    }
    if d.iter().flatten().flatten().any(|x| !x.is_finite()) {
        return invalid("hay distancias que no son números finitos");
    }
    let names: Vec<&str> = m.nodes.iter().map(|x| x.name.as_str()).collect();
    let find = |s: &Option<String>, def: usize| -> Result<usize, ModError> {
        match s.as_ref().map(|x| x.trim()).filter(|x| !x.is_empty()) {
            None => Ok(def),
            Some(t) => names.iter().position(|x| x.eq_ignore_ascii_case(t)).ok_or_else(|| ModError::Invalid(format!("no existe el nodo «{t}»"))),
        }
    };
    let src = find(&o.origin, 0)?;
    let dst = find(&o.destination, n - 1)?;
    // f*(i): distancia más corta de i al destino (Bellman-Ford hacia atrás)
    let mut f = vec![f64::INFINITY; n];
    f[dst] = 0.0;
    for _ in 0..n {
        let mut changed = false;
        for i in 0..n {
            for j in 0..n {
                if let Some(c) = d[i][j] {
                    if i != j && f[j] + c < f[i] - 1e-12 {
                        f[i] = f[j] + c;
                        changed = true;
                    }
                }
            }
        }
        if !changed {
            break;
        }
    }
    if (0..n).any(|i| (0..n).any(|j| d[i][j].map(|c| i != j && f[j] + c < f[i] - 1e-9).unwrap_or(false))) {
        return invalid("hay un ciclo de longitud negativa: la ruta más corta no existe");
    }
    let eps = 1e-9;
    let best_next = |i: usize| -> Vec<usize> { (0..n).filter(|&j| i != j && d[i][j].map(|c| (c + f[j] - f[i]).abs() <= eps * f[i].abs().max(1.0)).unwrap_or(false)).collect() };
    // todas las rutas óptimas desde el origen
    let mut routes = vec![];
    fn walk(i: usize, dst: usize, path: &mut Vec<usize>, out: &mut Vec<Vec<usize>>, nx: &dyn Fn(usize) -> Vec<usize>) {
        path.push(i);
        if i == dst {
            out.push(path.clone());
        } else if out.len() < 50 && path.len() <= 1000 {
            for j in nx(i) {
                if !path.contains(&j) {
                    walk(j, dst, path, out, nx);
                }
            }
        }
        path.pop();
    }
    if f[src].is_finite() {
        walk(src, dst, &mut vec![], &mut routes, &best_next);
    }
    if !f[src].is_finite() {
        return Ok(Report::new("dp", &m.name, Status::Infeasible, format!("No hay ruta de {} a {}.", names[src], names[dst]))
            .section("solucion", "Solución", vec![text_tone("Con las conexiones dadas no se puede llegar al destino.", Tone::Bad)])
            .data(json!({"status": "infeasible"})));
    }
    let path_txt = |p: &[usize]| p.iter().map(|&i| names[i]).collect::<Vec<_>>().join(" → ");
    let mut rt = Table::new(Some("Rutas óptimas"), &["#", "Ruta", "Distancia"]).left(1);
    for (k, p) in routes.iter().enumerate() {
        rt.row(cells![k + 1, path_txt(p), f[src]]);
    }
    let mut all = Table::new(Some("Desde cada nodo al destino"), &["Nodo", "Distancia más corta f*", "Siguiente nodo"]).left(0).left(2);
    for i in 0..n {
        let nx: Vec<&str> = best_next(i).iter().map(|&j| names[j]).collect();
        let tone = if routes.iter().any(|r| r.contains(&i)) { Tone::Highlight } else { Tone::Normal };
        all.row_tone(
            vec![Cell::from(names[i]), if f[i].is_finite() { Cell::Num(f[i]) } else { Cell::from("sin ruta") }, Cell::from(if i == dst { "—".into() } else { nx.join(", ") })],
            tone,
        );
    }
    // etapas: capas por el camino más largo desde el origen
    let edges: Vec<(usize, usize)> = (0..n).flat_map(|i| (0..n).filter(move |&j| i != j).map(move |j| (i, j))).filter(|&(i, j)| d[i][j].is_some()).collect();
    let pos = iaqsb_core::layout::layered(n, &edges, src);
    let mut st = vec![];
    if o.steps {
        // agrupar por columna del dibujo (etapa), de la última a la primera
        let mut layers: BTreeMap<i64, Vec<usize>> = BTreeMap::new();
        for i in 0..n {
            if i != dst {
                layers.entry((pos[i].0 * 1000.0).round() as i64).or_default().push(i);
            }
        }
        let mut steps_v = vec![step(
            "Recursión hacia atrás",
            format!("f*(destino) = 0. Para cada nodo i: f*(i) = mín sobre j de [ c(i, j) + f*(j) ]. Se resuelve de la última etapa hacia la primera; al final, f*({}) es la distancia más corta.", names[src]),
            vec![],
        )];
        let total = layers.len();
        for (k, (_, nodes)) in layers.iter().rev().enumerate() {
            let succ: Vec<usize> = {
                let mut v: Vec<usize> = nodes.iter().flat_map(|&i| (0..n).filter(move |&j| j != i)).filter(|&j| nodes.iter().any(|&i| d[i][j].is_some())).collect();
                v.sort();
                v.dedup();
                v
            };
            let mut head = vec!["Estado s".to_string()];
            head.extend(succ.iter().map(|&j| format!("x = {}", names[j])));
            head.extend(["f*(s)".to_string(), "x*".to_string()]);
            let mut t = Table::new(None, &head).left(0).left(head.len() - 1);
            for (r, &i) in nodes.iter().enumerate() {
                let mut row = vec![Cell::from(names[i])];
                let best = best_next(i);
                for (c, &j) in succ.iter().enumerate() {
                    row.push(match d[i][j] {
                        Some(cst) if f[j].is_finite() => Cell::from(format!("{} + {} = {}", fmt(cst), fmt(f[j]), fmt(cst + f[j]))),
                        _ => Cell::from("—"),
                    });
                    if best.contains(&j) {
                        t.mark(r, c + 1, Tone::Highlight);
                    }
                }
                row.push(if f[i].is_finite() { Cell::Num(f[i]) } else { Cell::from("∞") });
                row.push(Cell::from(best.iter().map(|&j| names[j]).collect::<Vec<_>>().join(", ")));
                t.row(row);
            }
            steps_v.push(step(format!("Etapa {}", total - k), "Cada celda: distancia del arco + f* del nodo al que llega; f*(s) es el mínimo de la fila.", vec![t.block()]));
        }
        st.push(steps(steps_v));
    }
    // red con las rutas óptimas resaltadas
    let on_route = |i: usize, j: usize| routes.iter().any(|r| r.windows(2).any(|w| w[0] == i && w[1] == j));
    let net = Block::Network(Network {
        title: Some("Red y rutas óptimas".into()),
        directed: true,
        nodes: (0..n)
            .map(|i| NetNode { id: format!("n{i}"), label: names[i].into(), x: pos[i].0, y: pos[i].1, note: f[i].is_finite().then(|| format!("f* = {}", fmt(f[i]))), tone: if routes.iter().any(|r| r.contains(&i)) { Tone::Highlight } else { Tone::Normal } })
            .collect(),
        edges: edges.iter().map(|&(i, j)| NetEdge { from: format!("n{i}"), to: format!("n{j}"), label: d[i][j].map(fmt), tone: if on_route(i, j) { Tone::Highlight } else { Tone::Muted } }).collect(),
    });
    let headline = format!("Distancia más corta de {} a {}: {} ({}).", names[src], names[dst], fmt(f[src]), routes.iter().map(|p| path_txt(p)).collect::<Vec<_>>().join("; "));
    Ok(Report::new("dp", &m.name, Status::Optimal, headline)
        .section("solucion", "Solución", vec![metrics(vec![metric_main("Distancia más corta", f[src]), metric("Rutas óptimas", routes.len())]), rt.block(), all.block()])
        .section("red", "Red", vec![net])
        .section("pasos", "Paso a paso", st)
        .data(json!({
            "distance": f[src],
            "routes": routes.iter().map(|p| p.iter().map(|&i| names[i]).collect::<Vec<_>>()).collect::<Vec<_>>(),
            "f": (0..n).map(|i| json!({"node": names[i], "f": if f[i].is_finite() { json!(f[i]) } else { Value::Null }})).collect::<Vec<_>>(),
        })))
}

// --------------------------------------------------------------------- mochila

fn key(x: f64) -> i64 {
    (x * 1e6).round() as i64
}

fn knapsack(m: &DpModel, o: &DpOptions) -> Result<Report, ModError> {
    let n = m.items.len();
    if n == 0 {
        return invalid("no hay artículos");
    }
    check_names("artículos", &m.items.iter().map(|x| &x.name).collect::<Vec<_>>())?;
    let cap = m.capacity;
    if !(cap >= 0.0 && cap.is_finite()) {
        return invalid("la capacidad de la mochila debe ser un número no negativo");
    }
    let funcs: Vec<Expr> = m.items.iter().map(|it| one_var(&it.function, &format!("rendimiento de {}", it.name))).collect::<Result<_, _>>()?;
    let preset = preset_list(o, n)?;
    let mut maxu = vec![0usize; n];
    for (k, it) in m.items.iter().enumerate() {
        if !(it.capacity >= 0.0 && it.capacity.is_finite()) {
            return invalid(format!("{}: la capacidad por unidad debe ser no negativa", it.name));
        }
        let fit = if it.capacity > 0.0 { (cap / it.capacity + 1e-9).floor() } else { f64::INFINITY };
        let a = it.available.unwrap_or(f64::INFINITY);
        let u = fit.min(a);
        if !u.is_finite() {
            return invalid(format!("{}: no ocupa capacidad y no tiene límite de unidades", it.name));
        }
        maxu[k] = u.max(0.0) as usize;
        if maxu[k] > 100_000 {
            return invalid(format!("{}: demasiadas unidades posibles ({})", it.name, maxu[k]));
        }
    }
    let r = |k: usize, x: usize| funcs[k].eval(&[x as f64]);
    // f_k(s) = máx_x r_k(x) + f_{k+1}(s − w_k x), con s = capacidad restante
    // estados alcanzables de capacidad restante
    let mut states: Vec<Vec<f64>> = vec![vec![]; n + 1];
    states[0] = vec![cap];
    for k in 0..n {
        let mut set: BTreeMap<i64, f64> = BTreeMap::new();
        for &s in &states[k] {
            let xs: Vec<usize> = match preset[k] {
                Some(p) => vec![p as usize],
                None => (0..=maxu[k]).collect(),
            };
            for x in xs {
                let left = s - m.items[k].capacity * x as f64;
                if left >= -1e-9 {
                    set.insert(key(left.max(0.0)), left.max(0.0));
                }
            }
        }
        states[k + 1] = set.into_values().collect();
        if states[k + 1].len() > MAX_STATES {
            return invalid("el problema genera demasiados estados para la programación dinámica");
        }
        if states[k + 1].is_empty() {
            return invalid(format!("la decisión fijada para {} no cabe en la mochila", m.items[k].name));
        }
    }
    let mut fv: Vec<BTreeMap<i64, (f64, Vec<usize>)>> = vec![BTreeMap::new(); n + 1];
    for &s in &states[n] {
        fv[n].insert(key(s), (0.0, vec![]));
    }
    for k in (0..n).rev() {
        for &s in &states[k] {
            let xs: Vec<usize> = match preset[k] {
                Some(p) => vec![p as usize],
                None => (0..=maxu[k]).collect(),
            };
            let mut best = f64::NEG_INFINITY;
            let mut arg = vec![];
            for x in xs {
                let left = s - m.items[k].capacity * x as f64;
                if left < -1e-9 {
                    continue;
                }
                let Some((fn1, _)) = fv[k + 1].get(&key(left.max(0.0))) else { continue };
                let v = r(k, x) + fn1;
                if !best.is_finite() || v > best + 1e-9 * best.abs().max(1.0) {
                    best = v;
                    arg = vec![x];
                } else if (v - best).abs() <= 1e-9 * best.abs().max(1.0) {
                    arg.push(x);
                }
            }
            if best.is_finite() {
                fv[k].insert(key(s), (best, arg));
            }
        }
    }
    let (total, _) = fv[0].get(&key(cap)).cloned().ok_or_else(|| ModError::Invalid("no hay solución factible".into()))?;
    // reconstrucción
    let mut s = cap;
    let mut plan = vec![];
    for k in 0..n {
        let (_, arg) = &fv[k][&key(s)];
        let x = arg[0];
        plan.push(x);
        s = (s - m.items[k].capacity * x as f64).max(0.0);
    }
    let mut t = Table::new(Some("Solución"), &["Etapa", "Artículo", "Unidades (X)", "Rendimiento", "Capacidad usada", "Capacidad restante"]).left(1);
    let mut left = cap;
    for k in 0..n {
        let used = m.items[k].capacity * plan[k] as f64;
        left -= used;
        t.row_tone(cells![k + 1, &m.items[k].name, plan[k], tidy(r(k, plan[k])), tidy(used), tidy(left)], if plan[k] > 0 { Tone::Accent } else { Tone::Normal });
    }
    t.row_tone(cells!["Total", "", "", tidy(total), tidy(cap - left), ""], Tone::Total);
    let mut st = vec![];
    if o.steps {
        let mut v = vec![step(
            "Recursión hacia atrás",
            "Etapa k = artículo k; estado s = capacidad que queda. f_k(s) = máx sobre x de [ r_k(x) + f_{k+1}(s − w_k·x) ], con x entre 0 y las unidades que caben o hay disponibles. Se empieza por el último artículo.",
            vec![],
        )];
        for k in (0..n).rev() {
            let sts = &states[k];
            let xs_all: Vec<usize> = match preset[k] {
                Some(p) => vec![p as usize],
                None => (0..=maxu[k]).collect(),
            };
            let show_cols = xs_all.len() <= 12;
            let mut head = vec!["Capacidad s".to_string()];
            if show_cols {
                head.extend(xs_all.iter().map(|x| format!("x = {x}")));
            }
            head.extend(["f*(s)".to_string(), "x*".to_string()]);
            let hl = head.len();
            let mut tb = Table::new(None, &head).left(hl - 1);
            for (ri, &sv) in sts.iter().enumerate().take(60) {
                let Some((best, arg)) = fv[k].get(&key(sv)) else { continue };
                let mut row = vec![Cell::Num(tidy(sv))];
                if show_cols {
                    for (c, &x) in xs_all.iter().enumerate() {
                        let l = sv - m.items[k].capacity * x as f64;
                        row.push(match fv[k + 1].get(&key(l.max(0.0))) {
                            Some((f1, _)) if l >= -1e-9 => Cell::Num(tidy(r(k, x) + f1)),
                            _ => Cell::from("—"),
                        });
                        if arg.contains(&x) {
                            tb.mark(ri, c + 1, Tone::Highlight);
                        }
                    }
                }
                row.push(Cell::Num(tidy(*best)));
                row.push(Cell::from(arg.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(", ")));
                tb.row(row);
            }
            let tb = if sts.len() > 60 { tb.note(format!("Se muestran 60 de {} estados.", sts.len())) } else { tb };
            v.push(step(
                format!("Etapa {}: {}", k + 1, m.items[k].name),
                format!("Cada unidad ocupa {} y rinde r(X) = {}. Cada celda: r(x) + f* de la etapa siguiente con la capacidad que queda.", fmt(m.items[k].capacity), m.items[k].function),
                vec![tb.block()],
            ));
        }
        st.push(steps(v));
    }
    let headline = format!(
        "Rendimiento máximo {}: {}.",
        fmt(tidy(total)),
        m.items.iter().zip(&plan).filter(|(_, x)| **x > 0).map(|(it, x)| format!("{} {} de {}", x, if *x == 1 { "unidad" } else { "unidades" }, it.name)).collect::<Vec<_>>().join(", ")
    );
    Ok(Report::new("dp", &m.name, Status::Optimal, headline)
        .section("solucion", "Solución", vec![metrics(vec![metric_main("Rendimiento total", tidy(total)), metric("Capacidad usada", tidy(cap - left)), metric("Capacidad libre", tidy(left))]), t.block()])
        .section("pasos", "Paso a paso", st)
        .data(json!({"total_return": tidy(total), "decisions": m.items.iter().zip(&plan).map(|(it, x)| json!({"item": it.name, "units": x})).collect::<Vec<_>>()})))
}

// ------------------------------------------------------ producción e inventario

fn production(m: &DpModel, o: &DpOptions) -> Result<Report, ModError> {
    let n = m.periods.len();
    if n == 0 {
        return invalid("no hay periodos");
    }
    check_names("periodos", &m.periods.iter().map(|x| &x.name).collect::<Vec<_>>())?;
    let names = ["P", "H", "B"];
    let mut costs = vec![];
    let mut uses_b = false;
    for p in &m.periods {
        let mut v = Vars::fixed(&names);
        let e = Expr::parse(&p.cost_function, &mut v).map_err(|e| ModError::Invalid(format!("costo de {}: {e} (usa P, H y B)", p.name)))?;
        // ¿usa B? (se comprueba evaluando, no por el texto: ABS contiene una B)
        if [(1.0, 1.0), (3.0, 0.0), (0.0, 7.0)].iter().any(|&(p, h)| (e.eval(&[p, h, 0.0]) - e.eval(&[p, h, 5.0])).abs() > 1e-12) {
            uses_b = true;
        }
        costs.push(e);
        if p.demand < 0.0 || p.setup_cost < 0.0 {
            return invalid(format!("{}: la demanda y el costo de preparación no pueden ser negativos", p.name));
        }
    }
    let preset = preset_list(o, n)?;
    let total_demand: f64 = m.periods.iter().map(|p| p.demand).sum();
    let int_ok = m.periods.iter().all(|p| p.demand.fract() == 0.0) && m.initial_inventory.fract() == 0.0;
    if !int_ok {
        return invalid("las demandas y el inventario inicial deben ser enteros (la recursión recorre unidades)");
    }
    let maxp: Vec<i64> = m.periods.iter().map(|p| p.production_capacity.map(|c| c.floor() as i64).unwrap_or((total_demand - m.initial_inventory).max(0.0) as i64 + 1).max(0)).collect();
    let low = if uses_b { -(total_demand as i64) } else { 0 };
    let i0 = m.initial_inventory as i64;
    let stage_cost = |k: usize, i: i64, p: i64| -> Option<(f64, i64)> {
        let e = i + p - m.periods[k].demand as i64;
        if e < low {
            return None;
        }
        if let Some(sc) = m.periods[k].storage_capacity {
            if e as f64 > sc + 1e-9 {
                return None;
            }
        }
        if k == n - 1 && e < 0 {
            return None;
        }
        let h = e.max(0) as f64;
        let b = (-e).max(0) as f64;
        let c = costs[k].eval(&[p as f64, h, b]) + if p > 0 { m.periods[k].setup_cost } else { 0.0 };
        Some((c, e))
    };
    // estados: inventario al empezar cada periodo
    let mut states: Vec<Vec<i64>> = vec![vec![i0]];
    for k in 0..n {
        let mut set = std::collections::BTreeSet::new();
        for &i in &states[k] {
            let ps: Vec<i64> = match preset[k] {
                Some(p) => vec![p as i64],
                None => (0..=maxp[k]).collect(),
            };
            for p in ps {
                if let Some((_, e)) = stage_cost(k, i, p) {
                    set.insert(e);
                }
            }
        }
        if set.is_empty() {
            return Ok(Report::new("dp", &m.name, Status::Infeasible, format!("No se puede cubrir la demanda del periodo {} con las capacidades dadas.", m.periods[k].name))
                .section("solucion", "Solución", vec![text_tone("Con el inventario inicial, las capacidades de producción y de almacén (y las decisiones fijadas), algún periodo queda sin cubrir.", Tone::Bad)])
                .data(json!({"status": "infeasible"})));
        }
        if set.len() * (maxp[k] as usize + 1) > 50 * MAX_STATES {
            return invalid("el problema genera demasiados estados para la programación dinámica");
        }
        states.push(set.into_iter().collect());
    }
    let mut fv: Vec<BTreeMap<i64, (f64, Vec<i64>)>> = vec![BTreeMap::new(); n + 1];
    for &s in &states[n] {
        fv[n].insert(s, (0.0, vec![]));
    }
    for k in (0..n).rev() {
        for &i in &states[k] {
            let ps: Vec<i64> = match preset[k] {
                Some(p) => vec![p as i64],
                None => (0..=maxp[k]).collect(),
            };
            let mut best = f64::INFINITY;
            let mut arg = vec![];
            for p in ps {
                let Some((c, e)) = stage_cost(k, i, p) else { continue };
                let Some((f1, _)) = fv[k + 1].get(&e) else { continue };
                let v = c + f1;
                if !best.is_finite() || v < best - 1e-9 * best.abs().max(1.0) {
                    best = v;
                    arg = vec![p];
                } else if (v - best).abs() <= 1e-9 * best.abs().max(1.0) {
                    arg.push(p);
                }
            }
            if best.is_finite() {
                fv[k].insert(i, (best, arg));
            }
        }
    }
    let Some((total, _)) = fv[0].get(&i0).cloned() else {
        return Ok(Report::new("dp", &m.name, Status::Infeasible, "Inventario inicial no factible: no hay plan que cubra la demanda.")
            .section("solucion", "Solución", vec![text_tone("No existe ningún plan de producción factible.", Tone::Bad)])
            .data(json!({"status": "infeasible"})));
    };
    let mut t = Table::new(
        Some("Plan de producción"),
        &["Periodo", "Demanda", "Inventario inicial", "Producción", "Inventario final", "Pendientes", "Costo de preparación", "Costo variable", "Costo total"],
    )
    .left(0);
    let mut i = i0;
    let mut plan = vec![];
    for k in 0..n {
        let p = fv[k][&i].1[0];
        let (c, e) = stage_cost(k, i, p).unwrap();
        let setup = if p > 0 { m.periods[k].setup_cost } else { 0.0 };
        t.row_tone(cells![&m.periods[k].name, m.periods[k].demand, i as f64, p as f64, e.max(0) as f64, (-e).max(0) as f64, setup, tidy(c - setup), tidy(c)], if p > 0 { Tone::Accent } else { Tone::Normal });
        plan.push((i, p, e, c));
        i = e;
    }
    t.row_tone(cells!["Total", total_demand, "", plan.iter().map(|x| x.1 as f64).sum::<f64>(), "", "", "", "", tidy(total)], Tone::Total);
    let mut st = vec![];
    if o.steps {
        let mut v = vec![step(
            "Recursión hacia atrás",
            format!(
                "Etapa k = periodo k; estado I = inventario al empezar. f_k(I) = mín sobre P de [ preparación·(P > 0) + c(P, H, B) + f_(k+1)(I + P − D) ], con H el inventario final y B los pendientes{}.",
                if uses_b { "" } else { " (no se admiten pendientes porque la función de costo no usa B)" }
            ),
            vec![],
        )];
        for k in (0..n).rev() {
            let mut tb = Table::new(None, &["Inventario I", "Mejor producción P*", "Inventario final", "Costo del periodo", "f*(I)"]);
            for &s in states[k].iter().take(80) {
                let Some((best, arg)) = fv[k].get(&s) else { continue };
                let (c, e) = stage_cost(k, s, arg[0]).unwrap();
                tb.row(cells![s as f64, arg.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(", "), e as f64, tidy(c), tidy(*best)]);
            }
            v.push(step(format!("Etapa {}: {}", k + 1, m.periods[k].name), format!("Demanda {}; costo variable {}; preparación {}.", fmt(m.periods[k].demand), m.periods[k].cost_function, fmt(m.periods[k].setup_cost)), vec![tb.block()]));
        }
        st.push(steps(v));
    }
    let chart = Chart::new(Some("Producción e inventario"), "Periodo", "Unidades")
        .series(Series::new("Producción", SeriesKind::Bar, plan.iter().enumerate().map(|(k, x)| [k as f64, x.1 as f64]).collect()).tone(Tone::Accent))
        .series(Series::new("Demanda", SeriesKind::Line, m.periods.iter().enumerate().map(|(k, p)| [k as f64, p.demand]).collect()).tone(Tone::Highlight))
        .series(Series::new("Inventario final", SeriesKind::Line, plan.iter().enumerate().map(|(k, x)| [k as f64, x.2 as f64]).collect()).tone(Tone::Muted));
    let mut chart = chart;
    chart.categories = Some(m.periods.iter().map(|p| p.name.clone()).collect());
    let headline = format!(
        "Costo mínimo {}: producir {}.",
        fmt(tidy(total)),
        m.periods.iter().zip(&plan).map(|(p, x)| format!("{} en {}", x.1, p.name)).collect::<Vec<_>>().join(", ")
    );
    Ok(Report::new("dp", &m.name, Status::Optimal, headline)
        .section("solucion", "Solución", vec![metrics(vec![metric_main("Costo total", tidy(total)), metric("Producción total", plan.iter().map(|x| x.1 as f64).sum::<f64>())]), t.block(), chart.block()])
        .section("pasos", "Paso a paso", st)
        .data(json!({"total_cost": tidy(total), "plan": m.periods.iter().zip(&plan).map(|(p, x)| json!({"period": p.name, "start": x.0, "produce": x.1, "end": x.2, "cost": tidy(x.3)})).collect::<Vec<_>>()})))
}

// -------------------------------------------------------------------- archivos

fn read_dp(bytes: &[u8]) -> Result<DpModel, ModError> {
    let mut r = Reader::new(bytes);
    let mut h = r.line("la cabecera")?;
    if !h.next().trim().to_ascii_uppercase().starts_with("DP") {
        return Err(ModError::Format("no es un archivo de DP".into()));
    }
    let name = h.next().trim().to_string();
    let code = h.next().trim().to_ascii_uppercase();
    let t = match code.get(..2).unwrap_or("") {
        "KS" => DpType::Knapsack,
        "PI" => DpType::Production,
        _ => DpType::Stagecoach,
    };
    let n = parse_num(&h.next()).unwrap_or(0.0) as usize;
    if n == 0 || n > 10_000 {
        return Err(ModError::Format("el número de etapas no es válido".into()));
    }
    let mut m = DpModel { name: if name.is_empty() { "DP Example".into() } else { name }, problem_type: t, nodes: vec![], distances: vec![], items: vec![], capacity: 0.0, periods: vec![], initial_inventory: 0.0 };
    let head = r.line("la fila de títulos")?;
    match t {
        DpType::Stagecoach => {
            m.nodes = (0..n).map(|j| DpNode { name: Some(head.get(j + 1).trim()).filter(|s| !s.is_empty()).map(str::to_string).unwrap_or(format!("Node{}", j + 1)) }).collect();
            for i in 0..n {
                let l = r.line(&format!("la fila del nodo {}", i + 1))?;
                m.distances.push((0..n).map(|j| parse_num(l.get(j + 1)).filter(|x| x.is_finite())).collect());
            }
        }
        DpType::Knapsack => {
            for i in 0..n {
                let l = r.line(&format!("el artículo {}", i + 1))?;
                m.items.push(Item {
                    name: Some(l.get(1).trim()).filter(|s| !s.is_empty()).map(str::to_string).unwrap_or(format!("Item{}", i + 1)),
                    available: parse_num(l.get(2)).filter(|x| x.is_finite()),
                    capacity: parse_num(l.get(3)).unwrap_or(0.0),
                    function: l.get(4).trim().to_string(),
                });
            }
            let l = r.line("la capacidad de la mochila")?;
            m.capacity = parse_num(l.get(2)).unwrap_or(0.0);
        }
        DpType::Production => {
            for i in 0..n {
                let l = r.line(&format!("el periodo {}", i + 1))?;
                m.periods.push(Period {
                    name: Some(l.get(1).trim()).filter(|s| !s.is_empty()).map(str::to_string).unwrap_or(format!("Period{}", i + 1)),
                    demand: parse_num(l.get(2)).unwrap_or(0.0),
                    production_capacity: parse_num(l.get(3)).filter(|x| x.is_finite()),
                    storage_capacity: parse_num(l.get(4)).filter(|x| x.is_finite()),
                    setup_cost: parse_num(l.get(5)).unwrap_or(0.0),
                    cost_function: l.get(6).trim().to_string(),
                });
            }
            // Línea opcional: los ejemplos originales sin inventario inicial no la traen.
            if let Some(l) = r.try_line() {
                m.initial_inventory = parse_num(l.get(2)).unwrap_or(0.0);
            }
        }
    }
    Ok(m)
}

fn write_dp(m: &DpModel) -> Vec<u8> {
    let mut w = Writer::new();
    let (code, n) = match m.problem_type {
        DpType::Stagecoach => ("SC", m.nodes.len()),
        DpType::Knapsack => ("KS", m.items.len()),
        DpType::Production => ("PIS", m.periods.len()),
    };
    w.row(&["DP".to_string(), m.name.clone(), code.into(), n.to_string()]);
    let blank = |s: String| if s.is_empty() { " ".to_string() } else { s };
    match m.problem_type {
        DpType::Stagecoach => {
            let mut r = vec!["From \\ To".to_string()];
            r.extend(m.nodes.iter().map(|x| x.name.clone()));
            w.row(&r);
            for (i, nd) in m.nodes.iter().enumerate() {
                let mut r = vec![nd.name.clone()];
                r.extend((0..n).map(|j| blank(m.distances.get(i).and_then(|row| row.get(j)).copied().flatten().map(num).unwrap_or_default())));
                w.row(&r);
            }
        }
        DpType::Knapsack => {
            w.row(&["Stage", "Item", "Available", "Capacity Required", "Return Function"]);
            for (k, it) in m.items.iter().enumerate() {
                w.row(&[(k + 1).to_string(), it.name.clone(), blank(opt(it.available)), num(it.capacity), it.function.clone()]);
            }
            w.row(&[" ".to_string(), "Knapsack Capacity =".into(), num(m.capacity), " ".into(), " ".into()]);
        }
        DpType::Production => {
            w.row(&["Stage", "Period", "Demand", "Production Capacity", "Storage Capacity", "Production Setup Cost", "Cost Function"]);
            for (k, p) in m.periods.iter().enumerate() {
                w.row(&[(k + 1).to_string(), p.name.clone(), num(p.demand), blank(opt(p.production_capacity)), blank(opt(p.storage_capacity)), num(p.setup_cost), p.cost_function.clone()]);
            }
            if m.initial_inventory != 0.0 {
                w.row(&[" ".to_string(), "Initial Inventory =".into(), num(m.initial_inventory), " ".into(), " ".into(), " ".into(), " ".into()]);
            }
        }
    }
    w.bytes()
}

