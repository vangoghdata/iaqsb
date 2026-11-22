//! MRP · Planeación de requerimientos de materiales (equivalente al módulo MRP de WinQSB).
//!
//! Maestro de artículos, lista de materiales (BOM), programa maestro (MPS),
//! inventario y capacidad. Explota el MPS por niveles (código de nivel más
//! bajo): requerimientos brutos, recepciones programadas, inventario
//! proyectado, requerimientos netos, recepciones y liberaciones planeadas, con
//! los lotes de WinQSB (LFL, FOQ, EOQ, FPR, POQ, LUC, LTC, PPB, SM y WW),
//! desperdicio, inventario de seguridad y redondeo a unidades enteras.
//! Informes: lista de acciones, BOM indentada y de dónde se usa, capacidad y costos.

use iaqsb_core::linear::Bnd;
use iaqsb_core::winqsb::{num, parse_num, Reader, Writer};
use iaqsb_core::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

fn m_inf() -> Bnd {
    Bnd::INF
}
fn lfl() -> String {
    "LFL".into()
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Item {
    pub id: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub abc: String,
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub material: String,
    /// Unidad de medida ("EACH", "EA", "UNIT"… redondea a enteros).
    #[serde(default)]
    pub unit: String,
    #[serde(default)]
    pub lead_time: u32,
    /// LFL, FOQ, EOQ, FPR, POQ, LUC, LTC, PPB, SM, WW o una cantidad fija.
    #[serde(default = "lfl")]
    pub lot_size: String,
    /// FOQ: cantidad; FPR: períodos por pedido.
    #[serde(default)]
    pub ls_multiplier: f64,
    /// Desperdicio (%).
    #[serde(default)]
    pub scrap: f64,
    #[serde(default)]
    pub annual_demand: f64,
    #[serde(default)]
    pub unit_cost: f64,
    #[serde(default)]
    pub setup_cost: f64,
    /// Costo anual de mantener una unidad.
    #[serde(default)]
    pub holding_cost: f64,
    /// Costo anual de faltante por unidad ("M" = no se usa).
    #[serde(default = "m_inf")]
    pub shortage_cost: Bnd,
    #[serde(default)]
    pub safety_stock: f64,
    #[serde(default)]
    pub on_hand: f64,
    /// Componentes directos: "B/2, C/3" (sin "/" la cantidad es 1).
    #[serde(default)]
    pub components: String,
    /// Requerimiento y recepción vencidos (anteriores al período 1).
    #[serde(default)]
    pub overdue_requirement: f64,
    #[serde(default)]
    pub overdue_receipt: f64,
    #[serde(default)]
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Named {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MrpModel {
    #[serde(default)]
    pub name: String,
    #[serde(default = "week")]
    pub time_unit: String,
    #[serde(default = "fifty_two")]
    pub periods_per_year: f64,
    pub periods: Vec<Named>,
    pub items: Vec<Item>,
    /// Programa maestro: artículo × período.
    #[serde(default)]
    pub mps: Vec<Vec<f64>>,
    /// Recepciones programadas: artículo × período.
    #[serde(default)]
    pub receipts: Vec<Vec<f64>>,
    /// Capacidad: artículo × período (vacío = sin límite).
    #[serde(default)]
    pub capacity: Vec<Vec<Option<f64>>>,
}

fn week() -> String {
    "semana".into()
}
fn fifty_two() -> f64 {
    52.0
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct MrpOptions {
    pub steps: bool,
}

impl Default for MrpOptions {
    fn default() -> Self {
        MrpOptions { steps: true }
    }
}

/// Registro MRP de un artículo (índice 0 = vencido).
#[derive(Clone, Default)]
pub struct Record {
    pub gross: Vec<f64>,
    pub sched: Vec<f64>,
    pub poh: Vec<f64>,
    pub net: Vec<f64>,
    pub receipt: Vec<f64>,
    pub release: Vec<f64>,
}

fn integral(unit: &str) -> bool {
    matches!(unit.trim().to_ascii_uppercase().as_str(), "EACH" | "EA" | "UNIT" | "SINGLE" | "E" | "U" | "UNIDAD" | "UN" | "PZA" | "PIEZA")
}

/// Agrupa los requerimientos netos en recepciones según la regla de lote.
fn lot(it: &Item, ppy: f64, nr: &[f64]) -> Vec<f64> {
    let t = nr.len();
    let mut q = vec![0.0; t];
    let rule = it.lot_size.trim().to_ascii_uppercase();
    let hp = it.holding_cost / ppy.max(1e-12);
    let eoq = || {
        if it.holding_cost <= 0.0 || it.annual_demand <= 0.0 {
            return 0.0;
        }
        let mut e = (2.0 * it.setup_cost * it.annual_demand / it.holding_cost).sqrt();
        let p = it.shortage_cost.0;
        if p > 0.0 && p.is_finite() {
            e *= ((p + it.holding_cost) / p).sqrt();
        }
        e
    };
    let foq = |qty: f64, q: &mut Vec<f64>| {
        let mut carry = 0.0;
        for k in 0..t {
            if nr[k] > carry + 1e-12 {
                let need = nr[k] - carry;
                let n = (need / qty - 1e-9).ceil().max(1.0);
                q[k] = n * qty;
                carry = q[k] - need;
            } else {
                carry -= nr[k];
            }
        }
    };
    let period_block = |n: usize, q: &mut Vec<f64>| {
        let mut k = 0;
        while k < t {
            if nr[k] > 0.0 {
                let end = (k + n.max(1)).min(t);
                q[k] = nr[k..end].iter().sum();
                k = end;
            } else {
                k += 1;
            }
        }
    };
    // bloques con una regla de parada: f(k0, k) = valor a comparar
    let greedy = |q: &mut Vec<f64>, kind: u8| {
        let epp = if hp > 0.0 { it.setup_cost / hp } else { f64::INFINITY };
        let mut k0 = 0;
        while k0 < t {
            if nr[k0] <= 0.0 {
                k0 += 1;
                continue;
            }
            let mut best = f64::INFINITY;
            let mut end = k0 + 1;
            let mut pp = 0.0;
            let mut units = 0.0;
            let mut k = k0;
            while k < t {
                units += nr[k];
                pp += nr[k] * (k - k0) as f64;
                let v = match kind {
                    0 | 3 => (pp - epp).abs(),                            // LTC y PPB
                    1 => (it.setup_cost + hp * pp) / units.max(1e-12),   // LUC
                    _ => (it.setup_cost + hp * pp) / (k - k0 + 1) as f64, // SM
                };
                if v <= best + 1e-12 {
                    best = v;
                    end = k + 1;
                } else {
                    // PPB: mira adelante como WinQSB
                    if kind == 3 && k + 1 < t && nr[k + 1] >= nr[k] * (k - k0) as f64 {
                        end = k + 1;
                    }
                    break;
                }
                k += 1;
            }
            q[k0] = nr[k0..end].iter().sum();
            k0 = end;
        }
    };
    if let Some(v) = parse_num(&rule).filter(|v| *v > 0.0 && v.is_finite()) {
        foq(v, &mut q);
        return q;
    }
    match rule.as_str() {
        "FOQ" if it.ls_multiplier > 0.0 => foq(it.ls_multiplier, &mut q),
        "EOQ" => {
            let e = eoq();
            if e > 0.0 {
                foq(e, &mut q)
            } else {
                q = nr.to_vec()
            }
        }
        "FPR" if it.ls_multiplier >= 1.0 => period_block(it.ls_multiplier as usize, &mut q),
        "POQ" => {
            let e = eoq();
            let n = if e > 0.0 { (e / it.annual_demand * ppy + 0.5) as usize } else { 0 };
            if n > 0 {
                period_block(n, &mut q)
            } else {
                q = nr.to_vec()
            }
        }
        "LTC" => greedy(&mut q, 0),
        "PPB" => greedy(&mut q, 3),
        "LUC" => {
            if it.holding_cost <= 0.0 {
                q = nr.to_vec()
            } else {
                greedy(&mut q, 1)
            }
        }
        "SM" => greedy(&mut q, 2),
        "WW" => {
            // Wagner-Whitin sin atrasos
            let mut f = vec![f64::INFINITY; t + 1];
            let mut from = vec![0usize; t + 1];
            f[0] = 0.0;
            for k in 1..=t {
                if nr[k - 1] <= 0.0 && f[k - 1] < f[k] {
                    f[k] = f[k - 1];
                    from[k] = usize::MAX;
                }
                for i in 0..k {
                    if !f[i].is_finite() || nr[i] <= 0.0 {
                        continue;
                    }
                    let carry: f64 = (i..k).map(|m| nr[m] * hp * (m - i) as f64).sum();
                    let c = f[i] + it.setup_cost + carry;
                    if c < f[k] - 1e-12 {
                        f[k] = c;
                        from[k] = i;
                    }
                }
            }
            let mut k = t;
            while k > 0 {
                if from[k] == usize::MAX {
                    k -= 1;
                    continue;
                }
                let i = from[k];
                q[i] = nr[i..k].iter().sum();
                k = i;
            }
        }
        _ => q = nr.to_vec(),
    }
    q
}

struct Explosion {
    level: Vec<usize>,
    order: Vec<usize>,
    comps: Vec<Vec<(usize, f64)>>,
    rec: Vec<Record>,
}

fn parse_components(m: &MrpModel) -> Result<Vec<Vec<(usize, f64)>>, ModError> {
    let ids: Vec<String> = m.items.iter().map(|x| x.id.trim().to_ascii_uppercase()).collect();
    let mut out = vec![];
    for it in &m.items {
        let mut v = vec![];
        for part in it.components.split([',', ';']).map(str::trim).filter(|s| !s.is_empty()) {
            let (id, u) = match part.split_once('/') {
                Some((a, b)) => (a.trim(), parse_num(b).ok_or_else(|| ModError::Invalid(format!("{}: «{b}» no es una cantidad", it.id)))?),
                None => (part, 1.0),
            };
            let j = ids.iter().position(|x| *x == id.to_ascii_uppercase()).ok_or_else(|| ModError::Invalid(format!("{}: el componente «{id}» no está en el maestro de artículos", it.id)))?;
            if u <= 0.0 {
                return invalid(format!("{}: la cantidad de {id} debe ser positiva", it.id));
            }
            v.push((j, u));
        }
        out.push(v);
    }
    Ok(out)
}

fn explode(m: &MrpModel) -> Result<Explosion, ModError> {
    let n = m.items.len();
    let t = m.periods.len() + 1;
    if n == 0 {
        return invalid("el maestro de artículos está vacío");
    }
    check_names("artículos", &m.items.iter().map(|x| &x.id).collect::<Vec<_>>())?;
    let comps = parse_components(m)?;
    // código de nivel más bajo (y ciclos)
    let mut level = vec![0usize; n];
    for _ in 0..=n {
        let mut ch = false;
        for i in 0..n {
            for &(j, _) in &comps[i] {
                if level[j] <= level[i] {
                    level[j] = level[i] + 1;
                    ch = true;
                }
            }
        }
        if !ch {
            break;
        }
        if level.iter().any(|l| *l > n) {
            return invalid("la lista de materiales tiene un ciclo (un artículo es componente de sí mismo)");
        }
    }
    if level.iter().any(|l| *l > n) {
        return invalid("la lista de materiales tiene un ciclo (un artículo es componente de sí mismo)");
    }
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by_key(|&i| (level[i], i));
    let mut rec: Vec<Record> = (0..n)
        .map(|i| {
            let it = &m.items[i];
            let mut gross = vec![0.0; t];
            let mut sched = vec![0.0; t];
            gross[0] = it.overdue_requirement;
            sched[0] = it.overdue_receipt;
            for k in 1..t {
                gross[k] = m.mps.get(i).and_then(|r| r.get(k - 1)).copied().unwrap_or(0.0);
                sched[k] = m.receipts.get(i).and_then(|r| r.get(k - 1)).copied().unwrap_or(0.0);
            }
            Record { gross, sched, poh: vec![0.0; t], net: vec![0.0; t], receipt: vec![0.0; t], release: vec![0.0; t] }
        })
        .collect();
    for &i in &order {
        let it = &m.items[i];
        if it.scrap < 0.0 || it.scrap >= 100.0 || it.safety_stock < 0.0 || it.on_hand < 0.0 {
            return invalid(format!("{}: el desperdicio debe estar entre 0 y 100 % y los inventarios no pueden ser negativos", it.id));
        }
        let r = &mut rec[i];
        // requerimientos netos: mantener el inventario de seguridad
        let mut run = it.on_hand - it.safety_stock;
        for k in 0..t {
            run += r.sched[k] - r.gross[k];
            if run < 0.0 {
                r.net[k] = -run;
                run = 0.0;
            } else {
                r.net[k] = 0.0;
            }
        }
        let mut nr = r.net.clone();
        if it.scrap > 0.0 {
            for v in nr.iter_mut() {
                *v *= 1.0 + it.scrap / 100.0;
            }
        }
        let mut pr = lot(it, m.periods_per_year, &nr);
        if integral(&it.unit) {
            for v in pr.iter_mut() {
                *v = (*v - 1e-9).ceil().max(0.0);
            }
        }
        let mut run = it.on_hand;
        for k in 0..t {
            run += r.sched[k] - r.gross[k] + pr[k];
            r.poh[k] = run;
        }
        r.receipt = pr.clone();
        for k in 0..t {
            let at = k.saturating_sub(it.lead_time as usize);
            r.release[at] += pr[k];
        }
        let rel = r.release.clone();
        for &(j, u) in &comps[i] {
            for k in 0..t {
                rec[j].gross[k] += rel[k] * u;
            }
        }
    }
    Ok(Explosion { level, order, comps, rec })
}

pub struct Mrp;

impl TypedModule for Mrp {
    type Model = MrpModel;
    type Options = MrpOptions;

    fn file_tag(&self) -> &'static str {
        "MRP"
    }

    fn spec(&self) -> ModuleSpec {
        let lots = "LFL, FOQ (cantidad = multiplicador), EOQ, FPR (períodos = multiplicador), POQ, LUC, LTC, PPB, SM, WW o una cantidad fija";
        ModuleSpec {
            id: "mrp".into(),
            winqsb: "MRP".into(),
            name: "Planeación de requerimientos de materiales".into(),
            description: "Explosión del MPS por la lista de materiales, lotes, capacidad y costos".into(),
            file_ext: "MRP".into(),
            problem_types: vec![ProblemType {
                id: "mrp".into(),
                label: "MRP".into(),
                description: "Maestro de artículos, lista de materiales, programa maestro e inventario".into(),
                dims: vec![Field::integer("items", "Número de artículos", 4), Field::integer("periods", "Número de períodos", 8), Field::text("time_unit", "Unidad de tiempo").default("semana")],
                layout: vec![
                    Widget::params(None, vec![Field::text("time_unit", "Unidad de tiempo"), Field::number("periods_per_year", "Períodos por año").default(52.0)]),
                    Widget::records(
                        Some("Maestro de artículos y lista de materiales"),
                        "items",
                        vec![
                            Field::text("id", "Artículo"),
                            Field::text("description", "Descripción"),
                            Field::list("components", "Componentes (id/cantidad)").help("P. ej. B/2, C/3"),
                            Field::integer("lead_time", "Tiempo de entrega", 1),
                            Field::text("lot_size", "Lote").default("LFL").help(lots),
                            Field::number("ls_multiplier", "Multiplicador del lote"),
                            Field::number("safety_stock", "Inventario de seguridad"),
                            Field::number("on_hand", "Inventario disponible"),
                            Field::number("scrap", "Desperdicio (%)"),
                            Field::text("unit", "Unidad de medida").help("EACH, EA o UNIT redondean a enteros"),
                            Field::number("annual_demand", "Demanda anual"),
                            Field::number("unit_cost", "Costo unitario"),
                            Field::number("setup_cost", "Costo de preparación"),
                            Field::number("holding_cost", "Costo anual de mantener"),
                            Field::bound("shortage_cost", "Costo anual de faltante", Value::from("M")),
                            Field::number("overdue_requirement", "Requerimiento vencido"),
                            Field::number("overdue_receipt", "Recepción vencida"),
                            Field::text("abc", "Clase ABC"),
                            Field::text("source", "Origen"),
                            Field::text("material", "Tipo de material"),
                            Field::text("note", "Nota"),
                        ],
                        Some("Añadir artículo"),
                    ),
                    Widget::matrix(Some("Programa maestro de producción (MPS)"), "mps", Axis::new("items", "Artículo").add("Añadir artículo"), Axis::new("periods", "Período").add("Añadir período"), Field::number("v", "Requerimiento")),
                    Widget::matrix(Some("Recepciones programadas"), "receipts", Axis::new("items", "Artículo"), Axis::new("periods", "Período"), Field::number("v", "Recepción")),
                    Widget::matrix(Some("Capacidad (opcional)"), "capacity", Axis::new("items", "Artículo"), Axis::new("periods", "Período"), Field::number("v", "Capacidad").optional()),
                ],
            }],
            options: vec![Field::boolean("steps", "Mostrar paso a paso", true)],
        }
    }

    fn template(&self, _pt: &str, dims: &Value) -> Result<MrpModel, ModError> {
        let n = dim(dims, "items", 4, 1, 1000)?;
        let t = dim(dims, "periods", 8, 1, 500)?;
        let items = (1..=n).map(|i| serde_json::from_value(json!({"id": format!("P{i}"), "lead_time": 1})).unwrap()).collect();
        Ok(MrpModel {
            name: "MRP nuevo".into(),
            time_unit: dim_str(dims, "time_unit", "semana"),
            periods_per_year: 52.0,
            periods: (1..=t).map(|k| Named { name: k.to_string() }).collect(),
            items,
            mps: vec![vec![0.0; t]; n],
            receipts: vec![vec![0.0; t]; n],
            capacity: vec![vec![None; t]; n],
        })
    }

    fn solve(&self, m: &MrpModel, o: &MrpOptions) -> Result<Report, ModError> {
        solve(m, o)
    }

    fn read(&self, bytes: &[u8]) -> Result<MrpModel, ModError> {
        read_mrp(bytes)
    }

    fn write(&self, m: &MrpModel) -> Result<Vec<u8>, ModError> {
        Ok(write_mrp(m))
    }

    fn examples(&self) -> Vec<(&'static str, MrpModel)> {
        let ex = |s: &str| serde_json::from_str::<MrpModel>(s).unwrap();
        vec![("producto_a_render", ex(include_str!("ejemplos/render.json"))), ("bicicletas_lotes", ex(include_str!("ejemplos/bicicletas.json"))), ("siete_articulos_arango", ex(include_str!("ejemplos/arango.json")))]
    }
}

fn solve(m: &MrpModel, o: &MrpOptions) -> Result<Report, ModError> {
    let ex = explode(m)?;
    let n = m.items.len();
    let t = m.periods.len() + 1;
    let u = &m.time_unit;
    let ids: Vec<&str> = m.items.iter().map(|x| x.id.as_str()).collect();
    let mut head = vec!["Concepto".to_string(), "Vencido".to_string()];
    head.extend(m.periods.iter().map(|p| format!("{} {}", u, p.name)));
    // lista de acciones
    let mut act = head.clone();
    act[0] = "Artículo".into();
    act.push("Total".into());
    let mut at = Table::new(Some("Lista de acciones: liberaciones de órdenes planeadas"), &act).left(0);
    let mut orders = vec![];
    for &i in &ex.order {
        let r = &ex.rec[i];
        let mut row = vec![Cell::from(ids[i])];
        row.extend(r.release.iter().map(|v| if *v > 1e-9 { Cell::Num(tidy(*v)) } else { Cell::Empty }));
        row.push(Cell::Num(tidy(r.release.iter().sum())));
        at.row_tone(row, if r.release[0] > 1e-9 { Tone::Accent } else { Tone::Normal });
        for k in 0..t {
            if r.release[k] > 1e-9 {
                orders.push(json!({"item": ids[i], "period": if k == 0 { "vencido".to_string() } else { m.periods[k - 1].name.clone() }, "quantity": tidy(r.release[k])}));
            }
        }
    }
    let late: Vec<&str> = ex.order.iter().filter(|&&i| ex.rec[i].release[0] > 1e-9).map(|&i| ids[i]).collect();
    // registros MRP
    let mut recs = vec![];
    for &i in &ex.order {
        let it = &m.items[i];
        let r = &ex.rec[i];
        let mut tb = Table::new(
            Some(&format!(
                "{}{} · nivel {} · entrega {} · lote {}{}",
                it.id,
                if it.description.is_empty() { String::new() } else { format!(" ({})", it.description) },
                ex.level[i],
                it.lead_time,
                it.lot_size,
                if it.safety_stock > 0.0 { format!(" · seguridad {}", fmt(it.safety_stock)) } else { String::new() }
            )),
            &head,
        )
        .left(0);
        let line = |label: &str, v: &[f64], tone: Tone, tb: &mut Table| {
            let mut row = vec![Cell::from(label)];
            let keep_zero = label.starts_with("Inventario");
            row.extend(v.iter().map(|x| if keep_zero || x.abs() > 1e-9 { Cell::Num(tidy(*x)) } else { Cell::Empty }));
            tb.row_tone(row, tone);
        };
        line("Requerimiento bruto", &r.gross, Tone::Normal, &mut tb);
        line("Recepción programada", &r.sched, Tone::Normal, &mut tb);
        line("Inventario proyectado", &r.poh, Tone::Normal, &mut tb);
        line("Requerimiento neto", &r.net, Tone::Normal, &mut tb);
        line("Recepción planeada", &r.receipt, Tone::Normal, &mut tb);
        line("Liberación planeada", &r.release, Tone::Highlight, &mut tb);
        recs.push(tb.block());
    }
    // estructura
    let parents: Vec<Vec<(usize, f64)>> = (0..n).map(|j| (0..n).flat_map(|i| ex.comps[i].iter().filter(|c| c.0 == j).map(move |c| (i, c.1))).collect()).collect();
    let ends: Vec<usize> = (0..n).filter(|&i| parents[i].is_empty()).collect();
    let mut bom = Table::new(Some("Lista de materiales indentada"), &["Nivel", "Artículo", "Cantidad por padre", "Cantidad total", "Descripción"]).left(1).left(4);
    let mut trees = vec![];
    for &e in &ends {
        if ex.comps[e].is_empty() {
            continue;
        }
        let mut nodes = vec![];
        fn walk(i: usize, depth: usize, qty: f64, per: f64, parent: Option<usize>, edge: Option<String>, m: &MrpModel, comps: &[Vec<(usize, f64)>], bom: &mut Table, nodes: &mut Vec<TreeNode>) {
            if depth > 60 {
                return;
            }
            let id = nodes.len();
            let it = &m.items[i];
            nodes.push(TreeNode { id, parent, edge, label: it.id.clone(), kind: None, value: Some(format!("entrega {}", it.lead_time)), tone: if depth == 0 { Tone::Accent } else { Tone::Normal } });
            bom.row(cells![format!("{}{}", ".".repeat(depth), depth), &it.id, if depth == 0 { Cell::Empty } else { Cell::Num(per) }, tidy(qty), &it.description]);
            for &(j, u) in &comps[i] {
                walk(j, depth + 1, qty * u, u, Some(id), Some(format!("×{}", fmt(u))), m, comps, bom, nodes);
            }
        }
        walk(e, 0, 1.0, 1.0, None, None, m, &ex.comps, &mut bom, &mut nodes);
        trees.push(Block::Tree(Tree { title: Some(format!("Estructura de {}", ids[e])), nodes }));
    }
    let mut wu = Table::new(Some("Dónde se usa cada artículo"), &["Artículo", "Se usa en (cantidad)"]).left(0).left(1);
    for j in 0..n {
        if !parents[j].is_empty() {
            wu.row(cells![ids[j], parents[j].iter().map(|(i, u)| format!("{} ({})", ids[*i], fmt(*u))).collect::<Vec<_>>().join(", ")]);
        }
    }
    let mut struct_blocks = trees;
    struct_blocks.push(bom.block());
    struct_blocks.push(wu.block());
    // capacidad
    let mut cap_blocks = vec![];
    let has_cap = m.capacity.iter().flatten().any(|c| c.is_some());
    if has_cap {
        let mut ch = vec!["Artículo".to_string()];
        ch.extend(m.periods.iter().map(|p| format!("{} {} (%)", u, p.name)));
        let mut ct = Table::new(Some("Carga / capacidad (%) de las liberaciones planeadas"), &ch).left(0);
        for i in 0..n {
            let caps: Vec<Option<f64>> = (1..t).map(|k| m.capacity.get(i).and_then(|r| r.get(k - 1)).copied().flatten()).collect();
            if caps.iter().all(|c| c.is_none()) {
                continue;
            }
            let mut row = vec![Cell::from(ids[i])];
            for k in 1..t {
                row.push(match caps[k - 1] {
                    Some(c) if c > 0.0 => Cell::Num(tidy(100.0 * ex.rec[i].release[k] / c)),
                    _ => Cell::Empty,
                });
            }
            ct.row(row);
            for k in 1..t {
                if let Some(c) = caps[k - 1] {
                    if ex.rec[i].release[k] > c + 1e-9 {
                        ct.mark(ct.rows.len() - 1, k, Tone::Bad);
                    }
                }
            }
        }
        cap_blocks.push(ct.block());
    }
    // costos
    let ppy = m.periods_per_year.max(1e-12);
    let mut cost_t = Table::new(Some("Análisis de costos del horizonte"), &["Artículo", "Preparación", "Mantener", "Faltante", "Material", "Total"]).left(0);
    let mut tot = [0.0; 5];
    for &i in &ex.order {
        let it = &m.items[i];
        let r = &ex.rec[i];
        let setup = it.setup_cost * r.receipt.iter().filter(|x| **x > 1e-9).count() as f64;
        let hold = it.holding_cost / ppy * r.poh[1..].iter().map(|x| x.max(0.0)).sum::<f64>();
        let short = if it.shortage_cost.0.is_finite() { it.shortage_cost.0 / ppy * r.poh[1..].iter().map(|x| (-x).max(0.0)).sum::<f64>() } else { 0.0 };
        let mat = it.unit_cost * r.receipt.iter().sum::<f64>();
        let row = [setup, hold, short, mat, setup + hold + short + mat];
        for k in 0..5 {
            tot[k] += row[k];
        }
        cost_t.row(cells![ids[i], tidy(row[0]), tidy(row[1]), tidy(row[2]), tidy(row[3]), tidy(row[4])]);
    }
    cost_t.row_tone(cells!["Total", tidy(tot[0]), tidy(tot[1]), tidy(tot[2]), tidy(tot[3]), tidy(tot[4])], Tone::Total);
    let mut sol = vec![metrics(vec![metric_main("Órdenes planeadas", orders.len()), metric("Artículos", n), metric("Niveles", ex.level.iter().max().copied().unwrap_or(0) + 1), metric("Costo total", tidy(tot[4]))])];
    if !late.is_empty() {
        sol.push(text_tone(format!("Hay liberaciones vencidas (debían haberse lanzado antes del período 1): {}. Revisa los tiempos de entrega o el programa maestro.", late.join(", ")), Tone::Bad));
    }
    sol.push(at.block());
    sol.push(cost_t.block());
    let mut st = vec![];
    if o.steps {
        let mut lt = Table::new(Some("Orden de cálculo (código de nivel más bajo)"), &["Nivel", "Artículo", "Componentes"]).left(1).left(2);
        for &i in &ex.order {
            lt.row(cells![ex.level[i], ids[i], ex.comps[i].iter().map(|(j, u)| format!("{}×{}", fmt(*u), ids[*j])).collect::<Vec<_>>().join(", ")]);
        }
        let first = ex.order[0];
        st.push(steps(vec![
            step("Niveles", "Cada artículo se calcula después de todos sus padres: el código de nivel más bajo es el nivel más profundo en que aparece en la lista de materiales.", vec![lt.block()]),
            step(
                "Requerimientos netos",
                "Requerimiento bruto = programa maestro + liberaciones planeadas de los padres × cantidad por padre. Neto = lo que falta para cubrir el bruto y mantener el inventario de seguridad, después del inventario disponible y las recepciones programadas.",
                vec![recs[0].clone()],
            ),
            step(
                "Lotes",
                format!("Los netos se agrupan con la regla de lote del artículo (aquí, {} usa {}). Con desperdicio, el neto se aumenta en ese porcentaje; con unidades enteras, se redondea hacia arriba.", ids[first], m.items[first].lot_size),
                vec![],
            ),
            step("Liberación", "Cada recepción planeada se libera tantos períodos antes como su tiempo de entrega; si cae antes del período 1, queda vencida.", vec![]),
        ]));
    }
    Ok(Report::new("mrp", &m.name, if late.is_empty() { Status::Solved } else { Status::Feasible }, format!("{} órdenes planeadas para {} artículos{}.", orders.len(), n, if late.is_empty() { String::new() } else { format!("; {} con liberación vencida", late.len()) }))
        .section("solucion", "Solución", sol)
        .section("registros", "Registros MRP", recs)
        .section("estructura", "Estructura", struct_blocks)
        .section("capacidad", "Capacidad", cap_blocks)
        .section("pasos", "Paso a paso", st)
        .data(json!({
            "orders": orders,
            "records": (0..n).map(|i| json!({"item": ids[i], "level": ex.level[i], "gross": ex.rec[i].gross.iter().map(|x| tidy(*x)).collect::<Vec<_>>(), "net": ex.rec[i].net.iter().map(|x| tidy(*x)).collect::<Vec<_>>(), "planned_receipts": ex.rec[i].receipt.iter().map(|x| tidy(*x)).collect::<Vec<_>>(), "planned_releases": ex.rec[i].release.iter().map(|x| tidy(*x)).collect::<Vec<_>>(), "projected_on_hand": ex.rec[i].poh.iter().map(|x| tidy(*x)).collect::<Vec<_>>()})).collect::<Vec<_>>(),
            "total_cost": tidy(tot[4]),
        })))
}

// ------------------------------------------------------------------ archivos

fn write_mrp(m: &MrpModel) -> Vec<u8> {
    let n = m.items.len();
    let t = m.periods.len();
    let comps: Vec<Vec<String>> = m.items.iter().map(|it| it.components.split([',', ';']).map(str::trim).filter(|s| !s.is_empty()).map(|s| s.replace(' ', "")).collect()).collect();
    let span = comps.iter().map(|c| c.len()).max().unwrap_or(0).max(1);
    let mut w = Writer::new();
    w.row(&["MRP".to_string(), m.name.clone(), n.to_string(), m.time_unit.clone(), t.to_string(), num(m.periods_per_year), span.to_string()]);
    w.row(&["**Item Master"]);
    for it in &m.items {
        w.row(&[
            it.id.clone(),
            it.abc.clone(),
            it.source.clone(),
            it.material.clone(),
            it.unit.clone(),
            it.lead_time.to_string(),
            it.lot_size.clone(),
            num(it.ls_multiplier),
            num(it.scrap),
            num(it.annual_demand),
            num(it.unit_cost),
            num(it.setup_cost),
            num(it.holding_cost),
            it.shortage_cost.text(),
            it.description.clone(),
            it.note.clone(),
        ]);
    }
    w.row(&["**BOM"]);
    for (i, it) in m.items.iter().enumerate() {
        if !comps[i].is_empty() {
            let mut r = vec![it.id.clone()];
            r.extend(comps[i].iter().map(|c| {
                // WinQSB omite "/1"
                match c.split_once('/') {
                    Some((a, b)) if parse_num(b) == Some(1.0) => a.to_string(),
                    _ => c.clone(),
                }
            }));
            w.row(&r);
        }
    }
    w.row(&["**MPS"]);
    for (i, it) in m.items.iter().enumerate() {
        let v: Vec<f64> = std::iter::once(it.overdue_requirement).chain((0..t).map(|k| m.mps.get(i).and_then(|r| r.get(k)).copied().unwrap_or(0.0))).collect();
        if v.iter().any(|x| *x > 0.0) {
            let mut r = vec![it.id.clone()];
            r.extend(v.iter().map(|x| num(*x)));
            w.row(&r);
        }
    }
    w.row(&["**Inventory"]);
    for (i, it) in m.items.iter().enumerate() {
        let v: Vec<f64> = std::iter::once(it.overdue_receipt).chain((0..t).map(|k| m.receipts.get(i).and_then(|r| r.get(k)).copied().unwrap_or(0.0))).collect();
        if it.safety_stock > 0.0 || it.on_hand > 0.0 || v.iter().any(|x| *x > 0.0) {
            let mut r = vec![it.id.clone(), num(it.safety_stock), num(it.on_hand)];
            r.extend(v.iter().map(|x| num(*x)));
            w.row(&r);
        }
    }
    w.row(&["**Capacity"]);
    for (i, it) in m.items.iter().enumerate() {
        let v: Vec<Option<f64>> = (0..t).map(|k| m.capacity.get(i).and_then(|r| r.get(k)).copied().flatten()).collect();
        if v.iter().any(|x| x.is_some()) {
            let mut r = vec![it.id.clone()];
            r.extend(v.iter().map(|x| x.map(num).unwrap_or_else(|| "M".into())));
            w.row(&r);
        }
    }
    w.bytes()
}

fn read_mrp(bytes: &[u8]) -> Result<MrpModel, ModError> {
    let mut r = Reader::new(bytes);
    let h = r.line("la cabecera")?;
    if !h.get(0).trim().to_ascii_uppercase().starts_with("MRP") {
        return Err(ModError::Format("no es un archivo de MRP".into()));
    }
    let name = Some(h.get(1).trim()).filter(|s| !s.is_empty()).unwrap_or("Sample Problem").to_string();
    let n = parse_num(h.get(2)).unwrap_or(0.0) as usize;
    let unit = Some(h.get(3).trim()).filter(|s| !s.is_empty()).unwrap_or("Month").to_string();
    let t = parse_num(h.get(4)).unwrap_or(0.0) as usize;
    let ppy = parse_num(h.get(5)).filter(|x| *x > 0.0).unwrap_or(52.0);
    if n == 0 || t == 0 || n > 100_000 || t > 10_000 {
        return Err(ModError::Format("el número de artículos o de períodos no es válido".into()));
    }
    let l = r.line("el maestro de artículos")?;
    if !l.get(0).trim().to_ascii_uppercase().starts_with("**IT") {
        return Err(ModError::Format("falta la sección **Item Master".into()));
    }
    let mut items = vec![];
    for i in 0..n {
        let l = r.line(&format!("el artículo {}", i + 1))?;
        let s = |k: usize| l.get(k).trim().to_string();
        let f = |k: usize| parse_num(l.get(k)).unwrap_or(0.0);
        items.push(Item {
            id: if s(0).is_empty() { format!("Item{}", i + 1) } else { s(0) },
            abc: s(1),
            source: s(2),
            material: s(3),
            unit: s(4),
            lead_time: f(5).max(0.0) as u32,
            lot_size: if s(6).is_empty() { "LFL".into() } else { s(6) },
            ls_multiplier: f(7),
            scrap: f(8),
            annual_demand: f(9),
            unit_cost: f(10),
            setup_cost: f(11),
            holding_cost: f(12),
            shortage_cost: parse_num(l.get(13)).map(Bnd).unwrap_or(Bnd::INF),
            description: s(14),
            note: s(15),
            safety_stock: 0.0,
            on_hand: 0.0,
            components: String::new(),
            overdue_requirement: 0.0,
            overdue_receipt: 0.0,
        });
    }
    let idx = |id: &str, items: &Vec<Item>| items.iter().position(|x| x.id.eq_ignore_ascii_case(id.trim()));
    let mut mps = vec![vec![0.0; t]; n];
    let mut rec = vec![vec![0.0; t]; n];
    let mut cap = vec![vec![None; t]; n];
    let mut sec = 0;
    while let Some(l) = r.try_line() {
        let first = l.get(0).trim().to_ascii_uppercase();
        if first.starts_with("**") {
            sec = match first.get(2..3).unwrap_or("") {
                "B" => 2,
                "M" => 3,
                "I" => 4,
                "C" => 5,
                _ => 0,
            };
            continue;
        }
        let Some(i) = idx(l.get(0), &items) else { continue };
        match sec {
            2 => {
                let parts: Vec<String> = (1..l.len()).map(|k| l.get(k).trim().to_string()).filter(|s| !s.is_empty()).collect();
                items[i].components = parts.join(", ");
            }
            3 => {
                items[i].overdue_requirement = parse_num(l.get(1)).unwrap_or(0.0);
                for k in 0..t {
                    mps[i][k] = parse_num(l.get(k + 2)).unwrap_or(0.0);
                }
            }
            4 => {
                items[i].safety_stock = parse_num(l.get(1)).unwrap_or(0.0);
                items[i].on_hand = parse_num(l.get(2)).unwrap_or(0.0);
                items[i].overdue_receipt = parse_num(l.get(3)).unwrap_or(0.0);
                for k in 0..t {
                    rec[i][k] = parse_num(l.get(k + 4)).unwrap_or(0.0);
                }
            }
            5 => {
                for k in 0..t {
                    cap[i][k] = parse_num(l.get(k + 1)).filter(|x| x.is_finite() && *x < 1e15);
                }
            }
            _ => {}
        }
    }
    Ok(MrpModel { name, time_unit: unit, periods_per_year: ppy, periods: (1..=t).map(|k| Named { name: k.to_string() }).collect(), items, mps, receipts: rec, capacity: cap })
}
