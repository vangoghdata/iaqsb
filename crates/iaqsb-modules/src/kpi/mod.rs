//! KPI · Indicadores logísticos (no existe en WinQSB): a partir de una lista
//! de pedidos se calculan los indicadores de servicio (a tiempo, completos,
//! documentación, pedido perfecto, OTIF, ciclo de la orden, urgentes) y de
//! costo (costo logístico sobre ventas frente a un estándar), con la
//! comparación de los pedidos urgentes y estándar (prueba t de Welch).

use iaqsb_core::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
pub struct Order {
    #[serde(default)]
    pub id: String,
    /// Fechas como "dd/mm", "dd/mm/aaaa" o número de día.
    #[serde(default)]
    pub created: String,
    #[serde(default)]
    pub promised: String,
    #[serde(default)]
    pub delivered: String,
    #[serde(default)]
    pub ordered_qty: Option<f64>,
    #[serde(default)]
    pub delivered_qty: Option<f64>,
    /// Documentación correcta.
    #[serde(default)]
    pub doc_ok: bool,
    #[serde(default)]
    pub urgent: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
pub struct CostItem {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub amount: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct KpiModel {
    #[serde(default)]
    pub name: String,
    #[serde(default = "pt")]
    pub problem_type: String,
    #[serde(default)]
    pub orders: Vec<Order>,
    #[serde(default)]
    pub costs: Vec<CostItem>,
    #[serde(default)]
    pub sales: Option<f64>,
    /// Costo logístico estándar del sector (% de las ventas).
    #[serde(default)]
    pub benchmark: Option<f64>,
}

fn pt() -> String {
    "pedidos".into()
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct KpiOptions {
    pub steps: bool,
    /// Nivel de significancia de la prueba t.
    pub alpha: f64,
}
impl Default for KpiOptions {
    fn default() -> Self {
        KpiOptions { steps: true, alpha: 0.05 }
    }
}

/// Días desde 1970 de una fecha civil (algoritmo de Howard Hinnant).
fn civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

fn day(s: &str, what: &str, id: &str) -> Result<f64, ModError> {
    let t = s.trim();
    if let Ok(x) = t.replace(',', ".").parse::<f64>() {
        return Ok(x);
    }
    let p: Vec<&str> = t.split(['/', '-', '.']).collect();
    let n = |x: &str| x.trim().parse::<i64>().ok();
    match p.as_slice() {
        [d, m] => match (n(d), n(m)) {
            (Some(d), Some(m)) if (1..=31).contains(&d) && (1..=12).contains(&m) => Ok(civil(2001, m, d) as f64),
            _ => invalid(format!("pedido {id}: la fecha {what} «{s}» no es válida")),
        },
        [d, m, y] => match (n(d), n(m), n(y)) {
            (Some(d), Some(m), Some(y)) if (1..=31).contains(&d) && (1..=12).contains(&m) => Ok(civil(if y < 100 { 2000 + y } else { y }, m, d) as f64),
            _ => invalid(format!("pedido {id}: la fecha {what} «{s}» no es válida")),
        },
        _ => invalid(format!("pedido {id}: falta la fecha {what} (dd/mm, dd/mm/aaaa o número de día)")),
    }
}

struct Row {
    id: String,
    cycle: f64,
    promise: f64,
    ot: bool,
    inf: bool,
    doc: bool,
    urgent: bool,
    q: f64,
    e: f64,
}

fn pct(a: usize, n: usize) -> f64 {
    if n == 0 {
        0.0
    } else {
        (1000.0 * a as f64 / n as f64).round() / 10.0
    }
}
fn r2(x: f64) -> f64 {
    (x * 100.0).round() / 100.0
}
fn r4(x: f64) -> f64 {
    (x * 10000.0).round() / 10000.0
}

fn mean_sd(v: &[f64]) -> (f64, f64) {
    let n = v.len() as f64;
    let m = v.iter().sum::<f64>() / n;
    let s = if v.len() > 1 { (v.iter().map(|x| (x - m).powi(2)).sum::<f64>() / (n - 1.0)).sqrt() } else { 0.0 };
    (m, s)
}

fn solve_kpi(m: &KpiModel, o: &KpiOptions) -> Result<Report, ModError> {
    if m.orders.is_empty() {
        return invalid("agrega los pedidos");
    }
    let mut rows = vec![];
    for (k, x) in m.orders.iter().enumerate() {
        let id = if x.id.trim().is_empty() { (k + 1).to_string() } else { x.id.clone() };
        let g = day(&x.created, "de generación", &id)?;
        let p = day(&x.promised, "prometida", &id)?;
        let r = day(&x.delivered, "de entrega", &id)?;
        let q = x.ordered_qty.ok_or_else(|| ModError::Invalid(format!("pedido {id}: faltan las unidades pedidas")))?;
        let e = x.delivered_qty.unwrap_or(q);
        rows.push(Row { id, cycle: r - g, promise: p - g, ot: r <= p, inf: e >= q, doc: x.doc_ok, urgent: x.urgent, q, e });
    }
    let n = rows.len();
    let c = |f: &dyn Fn(&Row) -> bool| rows.iter().filter(|r| f(r)).count();
    let ot = c(&|r| r.ot);
    let inf = c(&|r| r.inf);
    let doc = c(&|r| r.doc);
    let perf = c(&|r| r.ot && r.inf && r.doc);
    let otif = c(&|r| r.ot && r.inf);
    let urg = c(&|r| r.urgent);
    let units_q: f64 = rows.iter().map(|r| r.q).sum();
    let units_e: f64 = rows.iter().map(|r| r.e.min(r.q)).sum();
    let cyc_all: Vec<f64> = rows.iter().map(|r| r.cycle).collect();
    let cyc_std: Vec<f64> = rows.iter().filter(|r| !r.urgent).map(|r| r.cycle).collect();
    let cyc_urg: Vec<f64> = rows.iter().filter(|r| r.urgent).map(|r| r.cycle).collect();
    let (m_all, _) = mean_sd(&cyc_all);
    let mut kt = Table::new(Some("Indicadores de servicio"), &["Indicador", "Cálculo", "Resultado"]).left(0).left(1);
    kt.row(cells!["a) Pedidos entregados a tiempo", format!("{ot} / {n}"), format!("{} %", fmt(pct(ot, n)))]);
    kt.row(cells!["b) Pedidos completos", format!("{inf} / {n}"), format!("{} %", fmt(pct(inf, n)))]);
    kt.row(cells!["   Nivel de cumplimiento en unidades (fill rate)", format!("{} / {}", fmt(units_e), fmt(units_q)), format!("{} %", fmt((1000.0 * units_e / units_q).round() / 10.0))]);
    kt.row(cells!["c) Ciclo de la orden (todos)", format!("{} / {n}", fmt(cyc_all.iter().sum::<f64>())), format!("{} días", fmt(r2(m_all)))]);
    if !cyc_std.is_empty() && !cyc_urg.is_empty() {
        let (ms, _) = mean_sd(&cyc_std);
        kt.row(cells!["   Ciclo de la orden (solo estándar)", format!("{} / {}", fmt(cyc_std.iter().sum::<f64>()), cyc_std.len()), format!("{} días", fmt(r2(ms)))]);
    }
    kt.row(cells!["d) Documentación sin problemas", format!("{doc} / {n}"), format!("{} %", fmt(pct(doc, n)))]);
    kt.row_tone(cells!["e) Pedidos perfectos (a tiempo, completos y bien documentados)", format!("{perf} / {n}"), format!("{} %", fmt(pct(perf, n)))], Tone::Highlight);
    kt.row(cells!["   OTIF (a tiempo y completos)", format!("{otif} / {n}"), format!("{} %", fmt(pct(otif, n)))]);
    kt.row(cells!["f) Pedidos urgentes", format!("{urg} / {n}"), format!("{} %", fmt(pct(urg, n)))]);
    let mut blocks = vec![];
    let mut ms = vec![metric_main("Pedido perfecto (%)", pct(perf, n)), metric("A tiempo (%)", pct(ot, n)), metric("Completos (%)", pct(inf, n)), metric("Documentación (%)", pct(doc, n)), metric("Ciclo promedio (días)", r2(m_all))];
    // costos
    let total_cost: f64 = m.costs.iter().filter_map(|x| x.amount).sum();
    let mut cost_blocks = vec![];
    let mut cost_pct = None;
    if total_cost > 0.0 {
        let mut ct = Table::new(Some("Costos logísticos"), &["Rubro", "Valor", "% del costo logístico", "% de las ventas"]).left(0);
        let mut items: Vec<(String, f64)> = m.costs.iter().filter_map(|x| x.amount.map(|a| (x.name.clone(), a))).collect();
        items.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        for (nme, a) in &items {
            ct.row(cells![nme, *a, r2(100.0 * a / total_cost), m.sales.map(|s| Cell::Num(r2(100.0 * a / s))).unwrap_or(Cell::from("—"))]);
        }
        ct.row_tone(cells!["Total", total_cost, 100.0, m.sales.map(|s| Cell::Num(r2(100.0 * total_cost / s))).unwrap_or(Cell::from("—"))], Tone::Total);
        cost_blocks.push(ct.block());
        if let Some(s) = m.sales.filter(|s| *s > 0.0) {
            let p = 100.0 * total_cost / s;
            cost_pct = Some(p);
            kt.row(cells!["g) Costo logístico sobre ventas", format!("{} / {}", fmt(total_cost), fmt(s)), format!("{} %", fmt(r2(p)))]);
            ms.push(metric("Costo logístico / ventas (%)", r2(p)));
            if let Some(b) = m.benchmark {
                let target = s * b / 100.0;
                cost_blocks.push(text(format!(
                    "Frente al estándar del {} %: la empresa está {} puntos {} ({} % frente a {} %). Con el estándar gastaría {}; gasta {}, es decir {} {} ({} % {} que el estándar). El rubro más grande es «{}» ({} % del costo logístico).",
                    fmt(b),
                    fmt(r2((p - b).abs())),
                    if p > b { "por encima" } else { "por debajo" },
                    fmt(r2(p)),
                    fmt(b),
                    fmt(r2(target)),
                    fmt(total_cost),
                    fmt(r2((total_cost - target).abs())),
                    if total_cost > target { "de más" } else { "de menos" },
                    fmt(r2(100.0 * (p - b).abs() / b)),
                    if p > b { "más" } else { "menos" },
                    items[0].0,
                    fmt(r2(100.0 * items[0].1 / total_cost))
                )));
            }
            let mut cch = Chart::new(Some("Costo logístico como % de las ventas"), "Rubro", "% de las ventas");
            cch.categories = Some(items.iter().map(|x| x.0.clone()).collect());
            cch = cch.series(Series::new("% de las ventas", SeriesKind::Bar, items.iter().enumerate().map(|(k, x)| [k as f64, r2(100.0 * x.1 / s)]).collect()).tone(Tone::Highlight));
            cost_blocks.push(cch.block());
        }
    }
    blocks.push(metrics(ms));
    blocks.push(kt.clone().block());
    // por qué el pedido perfecto es menor
    let ind = (ot as f64 / n as f64) * (inf as f64 / n as f64) * (doc as f64 / n as f64) * 100.0;
    let fails = n - perf;
    let multi: Vec<String> = rows.iter().filter(|r| [!r.ot, !r.inf, !r.doc].iter().filter(|x| **x).count() >= 2).map(|r| r.id.clone()).collect();
    blocks.push(text(format!(
        "El pedido perfecto exige las tres condiciones a la vez: las fallas de cada indicador ({} tarde, {} incompletos, {} con error de documentación) caen en pedidos distintos y se acumulan: {} pedidos fallan en algo. Si las fallas fueran independientes se esperaría {} × {} × {} = {} %; el resultado real ({} %) {} porque {} pedidos fallan en más de una cosa{}.",
        n - ot,
        n - inf,
        n - doc,
        fails,
        fmt(r4(ot as f64 / n as f64)),
        fmt(r4(inf as f64 / n as f64)),
        fmt(r4(doc as f64 / n as f64)),
        fmt(r2(ind)),
        fmt(pct(perf, n)),
        if pct(perf, n) > ind { "es algo mayor" } else { "es menor o igual" },
        multi.len(),
        if multi.is_empty() { String::new() } else { format!(" ({})", multi.join(", ")) }
    )));
    // urgentes frente a estándar
    let mut cmp_blocks = vec![];
    let mut ttest = Value::Null;
    if !cyc_urg.is_empty() && !cyc_std.is_empty() {
        let grp = |u: bool| rows.iter().filter(move |r| r.urgent == u);
        let nu = cyc_urg.len();
        let ns = cyc_std.len();
        let (mu, su) = mean_sd(&cyc_urg);
        let (mst, sst) = mean_sd(&cyc_std);
        let prom = |u: bool| {
            let v: Vec<f64> = grp(u).map(|r| r.promise).collect();
            v.iter().sum::<f64>() / v.len() as f64
        };
        let cnt = |u: bool, f: &dyn Fn(&Row) -> bool| grp(u).filter(|r| f(r)).count();
        let mut ut = Table::new(Some("Urgentes frente a estándar"), &["Concepto", &format!("Urgentes ({nu})"), &format!("Estándar ({ns})")]).left(0);
        ut.row(cells!["Plazo prometido promedio (días)", r2(prom(true)), r2(prom(false))]);
        ut.row_tone(cells!["Ciclo real promedio (días)", r2(mu), r2(mst)], Tone::Highlight);
        ut.row(cells!["Desviación estándar del ciclo", r4(su), r4(sst)]);
        ut.row(cells!["A tiempo (%)", pct(cnt(true, &|r| r.ot), nu), pct(cnt(false, &|r| r.ot), ns)]);
        ut.row(cells!["Completos (%)", pct(cnt(true, &|r| r.inf), nu), pct(cnt(false, &|r| r.inf), ns)]);
        ut.row(cells!["Documentación correcta (%)", pct(cnt(true, &|r| r.doc), nu), pct(cnt(false, &|r| r.doc), ns)]);
        ut.row(cells!["Perfectos (%)", pct(cnt(true, &|r| r.ot && r.inf && r.doc), nu), pct(cnt(false, &|r| r.ot && r.inf && r.doc), ns)]);
        cmp_blocks.push(ut.block());
        if nu > 1 && ns > 1 && (su > 0.0 || sst > 0.0) {
            use statrs::distribution::{ContinuousCDF, StudentsT};
            let (vu, vs) = (su * su / nu as f64, sst * sst / ns as f64);
            let se = (vu + vs).sqrt();
            let t = (mu - mst) / se;
            let df = (vu + vs).powi(2) / (vu * vu / (nu as f64 - 1.0) + vs * vs / (ns as f64 - 1.0));
            let td = StudentsT::new(0.0, 1.0, df).unwrap();
            let p = td.cdf(t);
            let crit = td.inverse_cdf(o.alpha);
            let rej = t < crit;
            cmp_blocks.push(text(format!(
                "Prueba t de Welch (varianzas distintas), de una cola. H₀: los urgentes no son más rápidos (μu ≥ μe); H₁: μu < μe. t = ({} − {}) / √({}²/{} + {}²/{}) = {} / {} = {}; grados de libertad ≈ {}; t crítico (α = {}) = {}; valor p = {}. {} Razón de varianzas F = {}.",
                fmt(r4(mu)),
                fmt(r4(mst)),
                fmt(r4(su)),
                nu,
                fmt(r4(sst)),
                ns,
                fmt(r4(mu - mst)),
                fmt(r4(se)),
                fmt(r4(t)),
                fmt(r2(df)),
                fmt(o.alpha),
                fmt(r4(crit)),
                if p < 1e-4 { "< 0.0001".to_string() } else { fmt(r4(p)) },
                if rej { "Como t < t crítico, se rechaza H₀: los urgentes sí son significativamente más rápidos." } else { "No se rechaza H₀: no hay evidencia de que los urgentes sean más rápidos." },
                fmt(r4((sst * sst).max(su * su) / (sst * sst).min(su * su).max(1e-12)))
            )));
            ttest = json!({"t": r4(t), "df": r2(df), "critical": r4(crit), "p_value": p, "reject": rej});
        }
        let mut ch = Chart::new(Some("Ciclo de la orden por pedido"), "Pedido", "Días").stacked();
        ch.categories = Some(rows.iter().map(|r| r.id.clone()).collect());
        ch = ch
            .series(Series::new("Urgente", SeriesKind::Bar, rows.iter().enumerate().filter(|(_, r)| r.urgent).map(|(k, r)| [k as f64, r.cycle]).collect()).tone(Tone::Accent))
            .series(Series::new("Estándar", SeriesKind::Bar, rows.iter().enumerate().filter(|(_, r)| !r.urgent).map(|(k, r)| [k as f64, r.cycle]).collect()))
            .hline(mu, "urgentes", Tone::Accent)
            .hline(mst, "estándar", Tone::Muted);
        cmp_blocks.push(ch.block());
    }
    // pedidos con fallas
    let mut ft = Table::new(Some("Pedidos con alguna falla"), &["Pedido", "Ciclo (días)", "¿A tiempo?", "¿Completo?", "¿Documentación?", "Urgente"]).left(0);
    for r in rows.iter().filter(|r| !(r.ot && r.inf && r.doc)) {
        let yn = |b: bool| if b { "Sí" } else { "NO" };
        ft.row_tone(cells![&r.id, r.cycle, yn(r.ot), if r.inf { "Sí".to_string() } else { format!("NO ({} de {})", fmt(r.e), fmt(r.q)) }, yn(r.doc), if r.urgent { "Sí" } else { "No" }], Tone::Bad);
    }
    let mut kch = Chart::new(Some("Indicadores de servicio (%)"), "Indicador", "%");
    let kc = ["A tiempo", "Completos", "Documentación", "OTIF", "Perfectos"];
    kch.categories = Some(kc.iter().map(|s| s.to_string()).collect());
    kch = kch.series(Series::new("%", SeriesKind::Bar, vec![[0.0, pct(ot, n)], [1.0, pct(inf, n)], [2.0, pct(doc, n)], [3.0, pct(otif, n)], [4.0, pct(perf, n)]]).tone(Tone::Highlight));
    kch.y_zero = true;
    let head = format!(
        "Pedido perfecto {} % · a tiempo {} % · completos {} % · documentación {} % · ciclo {} días{}.",
        fmt(pct(perf, n)),
        fmt(pct(ot, n)),
        fmt(pct(inf, n)),
        fmt(pct(doc, n)),
        fmt(r2(m_all)),
        cost_pct.map(|p| format!(" · costo logístico {} % de las ventas", fmt(r2(p)))).unwrap_or_default()
    );
    let st = if o.steps {
        vec![steps(vec![
            step("Columnas auxiliares", "Para cada pedido: ciclo = entrega real − generación; a tiempo = entrega real ≤ fecha prometida; completo = entregadas ≥ pedidas; perfecto = a tiempo, completo y con documentación correcta. Los indicadores se cuentan por pedido, no por unidades (salvo el fill rate).", vec![]),
            step("Indicadores", "Cada indicador es el número de pedidos que cumplen sobre el total.", vec![kt.block()]),
            step("Pedidos con fallas", format!("{fails} pedidos fallan en al menos una condición."), vec![ft.clone().block()]),
        ])]
    } else {
        vec![]
    };
    let mut rep = Report::new("kpi", &m.name, Status::Solved, head)
        .section("solucion", "Indicadores", blocks)
        .section("graficos", "Gráficos", vec![kch.block()])
        .section("urgentes", "Urgentes", cmp_blocks)
        .section("costos", "Costos", cost_blocks)
        .section("fallas", "Fallas", vec![ft.block()])
        .section("pasos", "Paso a paso", st)
        .data(json!({
            "on_time_pct": pct(ot, n), "in_full_pct": pct(inf, n), "doc_ok_pct": pct(doc, n), "perfect_pct": pct(perf, n), "otif_pct": pct(otif, n),
            "urgent_pct": pct(urg, n), "cycle_avg": r4(m_all), "logistics_cost_pct": cost_pct.map(r4), "welch": ttest,
        }))
        .cite("mora2008", "Indicadores de gestión logística: entregas a tiempo, completas, documentación, ciclo de la orden y costo logístico sobre ventas")
        .cite("scor", "Pedido perfecto (Perfect Order Fulfillment, RL.1.1)")
        .cite("sarache2026taller1", "Planteamiento del taller de indicadores logísticos");
    if ttest != Value::Null {
        rep = rep.cite("welch1947", "Prueba t para dos muestras con varianzas distintas (grados de libertad de Welch-Satterthwaite)");
    }
    Ok(rep)
}

pub struct Kpi;

impl TypedModule for Kpi {
    type Model = KpiModel;
    type Options = KpiOptions;

    fn file_tag(&self) -> &'static str {
        "IAQSB-KPI"
    }

    fn spec(&self) -> ModuleSpec {
        ModuleSpec {
            id: "kpi".into(),
            winqsb: "Nuevo".into(),
            name: "Indicadores logísticos".into(),
            description: "Pedidos a tiempo, completos, documentación, pedido perfecto, OTIF, ciclo de la orden, urgentes y costo logístico sobre ventas".into(),
            file_ext: "KPI".into(),
            problem_types: vec![ProblemType {
                id: "pedidos".into(),
                label: "Indicadores a partir de los pedidos".into(),
                description: "Una fila por pedido con sus fechas, unidades, documentación y si es urgente".into(),
                dims: vec![Field::integer("orders", "Número de pedidos", 10)],
                layout: vec![
                    Widget::records(
                        Some("Pedidos"),
                        "orders",
                        vec![
                            Field::text("id", "Pedido"),
                            Field::text("created", "Generación").help("dd/mm, dd/mm/aaaa o número de día"),
                            Field::text("promised", "Fecha prometida"),
                            Field::text("delivered", "Entrega real"),
                            Field::number("ordered_qty", "Unidades pedidas").optional(),
                            Field::number("delivered_qty", "Unidades entregadas").optional(),
                            Field::boolean("doc_ok", "Documentación correcta", true),
                            Field::boolean("urgent", "Urgente", false),
                        ],
                        Some("Añadir pedido"),
                    ),
                    Widget::params(Some("Ventas y estándar"), vec![Field::number("sales", "Ventas del período").optional(), Field::number("benchmark", "Costo logístico estándar del sector (% de las ventas)").optional()]),
                    Widget::records(Some("Costos logísticos"), "costs", vec![Field::text("name", "Rubro"), Field::number("amount", "Valor").optional()], Some("Añadir rubro")),
                ],
            }],
            options: vec![Field::boolean("steps", "Mostrar paso a paso", true), Field::number("alpha", "Significancia de la prueba t").default(0.05)],
        }
    }

    fn template(&self, _pt: &str, dims: &Value) -> Result<KpiModel, ModError> {
        let n = dim(dims, "orders", 10, 1, 100000)?;
        Ok(KpiModel {
            name: "Indicadores logísticos".into(),
            problem_type: "pedidos".into(),
            orders: (1..=n).map(|i| Order { id: format!("P-{i:03}"), doc_ok: true, ..Default::default() }).collect(),
            costs: ["Almacenamiento e inventario", "Transporte y distribución", "Procesamiento de pedidos", "Devoluciones"].iter().map(|s| CostItem { name: s.to_string(), amount: None }).collect(),
            sales: None,
            benchmark: None,
        })
    }

    fn solve(&self, m: &KpiModel, o: &KpiOptions) -> Result<Report, ModError> {
        solve_kpi(m, o)
    }

    fn read(&self, bytes: &[u8]) -> Result<KpiModel, ModError> {
        let s = String::from_utf8_lossy(bytes);
        let s = s.trim_start_matches('\u{feff}');
        let (head, rest) = s.split_once('\n').unwrap_or((s, ""));
        if !head.trim().to_ascii_uppercase().starts_with("IAQSB-KPI") {
            return Err(ModError::Format("no es un archivo de indicadores logísticos de IAQSB".into()));
        }
        serde_json::from_str(rest).map_err(|e| ModError::Format(e.to_string()))
    }

    fn write(&self, m: &KpiModel) -> Result<Vec<u8>, ModError> {
        Ok(format!("IAQSB-KPI 1\n{}\n", serde_json::to_string_pretty(m).unwrap()).into_bytes())
    }

    fn examples(&self) -> Vec<(&'static str, KpiModel)> {
        vec![("taller1_distribuidor", serde_json::from_str(include_str!("ejemplos/taller1.json")).unwrap())]
    }
}
