//! FIN · Finanzas del inventario (no existe en WinQSB): cómo el inventario
//! afecta la rentabilidad (ROI), la liquidez (razón corriente y prueba ácida)
//! y el capital de trabajo (ciclo operativo y desfase).

use iaqsb_core::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum FinType {
    #[default]
    Roi,
    Liquidez,
    Capital,
}

/// Estado de resultados y balance resumidos de un escenario.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
pub struct RoiCase {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub sales: f64,
    #[serde(default)]
    pub cost_of_sales: f64,
    /// Gastos fijos y variables (o solo gastos totales).
    #[serde(default)]
    pub fixed_expenses: Option<f64>,
    #[serde(default)]
    pub variable_expenses: Option<f64>,
    #[serde(default)]
    pub total_expenses: Option<f64>,
    #[serde(default)]
    pub taxes: f64,
    #[serde(default)]
    pub fixed_assets: f64,
    #[serde(default)]
    pub banks: f64,
    #[serde(default)]
    pub receivables: f64,
    #[serde(default)]
    pub inventories: f64,
}

/// Situación del ciclo operativo.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
pub struct Cycle {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub days_raw: Option<f64>,
    #[serde(default)]
    pub days_wip: Option<f64>,
    #[serde(default)]
    pub days_finished: Option<f64>,
    /// Días de inventario en total (si no se dan por separado).
    #[serde(default)]
    pub days_inventory: Option<f64>,
    #[serde(default)]
    pub days_receivable: Option<f64>,
    #[serde(default)]
    pub days_payable: Option<f64>,
    /// Veces que se repite el desfase en el año (problema al revés).
    #[serde(default)]
    pub times_per_year: Option<f64>,
    /// Reducción (%) de los días de inventario frente a la primera situación.
    #[serde(default)]
    pub inventory_reduction: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FinModel {
    #[serde(default)]
    pub name: String,
    pub problem_type: FinType,
    // ROI
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cases: Vec<RoiCase>,
    /// Escenario adicional: inventario reducido a este valor.
    #[serde(default)]
    pub reduce_inventory_to: Option<f64>,
    /// Los gastos bajan en la misma proporción que el inventario.
    #[serde(default)]
    pub proportional_expenses: bool,
    /// Cambio en los impuestos (%), p. ej. −10.
    #[serde(default)]
    pub tax_change: Option<f64>,
    // liquidez
    #[serde(default)]
    pub banks: f64,
    #[serde(default)]
    pub receivables: f64,
    #[serde(default)]
    pub securities: f64,
    #[serde(default)]
    pub inventories: f64,
    #[serde(default)]
    pub current_liabilities: f64,
    /// % del inventario que resulta obsoleto y % de su valor que se recupera.
    #[serde(default)]
    pub obsolete_pct: Option<f64>,
    #[serde(default)]
    pub recovery_pct: Option<f64>,
    // capital de trabajo
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cycles: Vec<Cycle>,
    /// Presupuesto anual de egresos y de ventas.
    #[serde(default)]
    pub annual_expenses: f64,
    #[serde(default)]
    pub annual_sales: f64,
    #[serde(default = "d360")]
    pub days_per_year: f64,
}

fn d360() -> f64 {
    360.0
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct FinOptions {
    pub steps: bool,
}
impl Default for FinOptions {
    fn default() -> Self {
        FinOptions { steps: true }
    }
}

fn r2(x: f64) -> f64 {
    (x * 100.0).round() / 100.0
}
fn r4(x: f64) -> f64 {
    (x * 10000.0).round() / 10000.0
}

struct RoiOut {
    name: String,
    gross: f64,
    expenses: f64,
    taxes: f64,
    net: f64,
    current: f64,
    assets: f64,
    roi: f64,
    margin: f64,
    turnover: f64,
    inv: f64,
    sales: f64,
}

fn roi_of(c: &RoiCase) -> Result<RoiOut, ModError> {
    let expenses = match (c.total_expenses, c.fixed_expenses, c.variable_expenses) {
        (Some(t), _, _) => t,
        (None, f, v) => f.unwrap_or(0.0) + v.unwrap_or(0.0),
    };
    let gross = c.sales - c.cost_of_sales;
    let net = gross - expenses - c.taxes;
    let current = c.banks + c.receivables + c.inventories;
    let assets = c.fixed_assets + current;
    if assets <= 0.0 {
        return invalid(format!("{}: el activo total debe ser positivo", c.name));
    }
    Ok(RoiOut {
        name: c.name.clone(),
        gross,
        expenses,
        taxes: c.taxes,
        net,
        current,
        assets,
        roi: 100.0 * net / assets,
        margin: if c.sales > 0.0 { 100.0 * net / c.sales } else { 0.0 },
        turnover: c.sales / assets,
        inv: c.inventories,
        sales: c.sales,
    })
}

fn roi(m: &FinModel, o: &FinOptions) -> Result<Report, ModError> {
    if m.cases.is_empty() {
        return invalid("agrega al menos un escenario");
    }
    let mut cases = m.cases.clone();
    for (k, c) in cases.iter_mut().enumerate() {
        if c.name.trim().is_empty() {
            c.name = format!("Escenario {}", k + 1);
        }
    }
    let mut note = None;
    if let Some(to) = m.reduce_inventory_to {
        let b = cases[0].clone();
        if b.inventories <= 0.0 {
            return invalid("para reducir el inventario, el primer escenario debe tener inventario");
        }
        let red = (b.inventories - to) / b.inventories;
        let mut n = b.clone();
        n.name = format!("Inventario en {}", fmt(to));
        n.inventories = to;
        let mut txt = format!("Inventario de {} a {}: baja {} %.", fmt(b.inventories), fmt(to), fmt(r2(100.0 * red)));
        if m.proportional_expenses {
            let f = 1.0 - red;
            n.total_expenses = Some(roi_of(&b)?.expenses * f);
            n.fixed_expenses = None;
            n.variable_expenses = None;
            txt += &format!(" Supuesto: los gastos bajan en la misma proporción ({} %), de {} a {}.", fmt(r2(100.0 * red)), fmt(r2(roi_of(&b)?.expenses)), fmt(r2(n.total_expenses.unwrap())));
        }
        if let Some(tc) = m.tax_change {
            n.taxes = b.taxes * (1.0 + tc / 100.0);
            txt += &format!(" Impuestos {} {} %: de {} a {}.", if tc < 0.0 { "bajan" } else { "suben" }, fmt(tc.abs()), fmt(b.taxes), fmt(r2(n.taxes)));
        }
        note = Some(txt);
        cases.push(n);
    }
    let outs: Vec<RoiOut> = cases.iter().map(roi_of).collect::<Result<_, _>>()?;
    let mut head = vec!["Concepto".to_string()];
    head.extend(outs.iter().map(|x| x.name.clone()));
    let mut t = Table::new(Some("Rentabilidad sobre la inversión"), &head).left(0);
    let row = |t: &mut Table, l: &str, f: &dyn Fn(&RoiOut) -> f64, tone: Tone| {
        let mut r = vec![Cell::from(l)];
        r.extend(outs.iter().map(|x| Cell::Num(r4(f(x)))));
        t.row_tone(r, tone);
    };
    row(&mut t, "Ventas", &|x| x.sales, Tone::Normal);
    row(&mut t, "Utilidad bruta (ventas − costo de ventas)", &|x| x.gross, Tone::Normal);
    row(&mut t, "Gastos", &|x| x.expenses, Tone::Normal);
    row(&mut t, "Impuestos", &|x| x.taxes, Tone::Normal);
    row(&mut t, "Utilidad neta", &|x| x.net, Tone::Total);
    row(&mut t, "Inventarios", &|x| x.inv, Tone::Normal);
    row(&mut t, "Activo corriente (bancos + cartera + inventarios)", &|x| x.current, Tone::Normal);
    row(&mut t, "Activo total", &|x| x.assets, Tone::Total);
    row(&mut t, "Margen neto (%)", &|x| x.margin, Tone::Normal);
    row(&mut t, "Rotación del activo (veces)", &|x| x.turnover, Tone::Normal);
    row(&mut t, "ROI = utilidad neta / activo total (%)", &|x| x.roi, Tone::Highlight);
    let mut ch = Chart::new(Some("ROI por escenario"), "Escenario", "ROI (%)");
    ch.categories = Some(outs.iter().map(|x| x.name.clone()).collect());
    ch = ch.series(Series::new("ROI (%)", SeriesKind::Bar, outs.iter().enumerate().map(|(k, x)| [k as f64, r2(x.roi)]).collect()).tone(Tone::Highlight));
    let mut ch2 = Chart::new(Some("Composición del activo"), "Escenario", "Valor").stacked();
    ch2.categories = ch.categories.clone();
    ch2 = ch2
        .series(Series::new("Activo fijo", SeriesKind::Bar, cases.iter().enumerate().map(|(k, c)| [k as f64, c.fixed_assets]).collect()))
        .series(Series::new("Bancos y cartera", SeriesKind::Bar, cases.iter().enumerate().map(|(k, c)| [k as f64, c.banks + c.receivables]).collect()))
        .series(Series::new("Inventarios", SeriesKind::Bar, cases.iter().enumerate().map(|(k, c)| [k as f64, c.inventories]).collect()).tone(Tone::Accent));
    let first = &outs[0];
    let last = outs.last().unwrap();
    let headline = if outs.len() == 1 {
        format!("ROI = {} / {} = {} %.", fmt(r2(first.net)), fmt(r2(first.assets)), fmt(r2(first.roi)))
    } else {
        format!("ROI: {} ({} {} puntos).", outs.iter().map(|x| format!("{} {} %", x.name, fmt(r2(x.roi)))).collect::<Vec<_>>().join(" → "), if last.roi >= first.roi { "sube" } else { "baja" }, fmt(r2((last.roi - first.roi).abs())))
    };
    let mut blocks = vec![metrics(outs.iter().enumerate().map(|(k, x)| if k == 0 { metric_main(format!("ROI {}", x.name), r2(x.roi)) } else { metric(format!("ROI {}", x.name), r2(x.roi)) }).collect())];
    if let Some(n) = &note {
        blocks.push(text(n.clone()));
    }
    blocks.push(t.clone().block());
    if outs.len() > 1 {
        blocks.push(text("El inventario golpea los dos lados de la fracción: menos inventario baja el activo total (denominador) y, con menos que almacenar, bajan los gastos y sube la utilidad (numerador)."));
    }
    let mut st = vec![];
    if o.steps {
        let mut v = vec![];
        for x in &outs {
            v.push(step(
                x.name.clone(),
                format!(
                    "Utilidad neta = utilidad bruta − gastos − impuestos = {} − {} − {} = {}. Activo total = activo fijo + activo corriente = {} + {} = {}. ROI = {} / {} = {} % (margen {} % × rotación {} veces).",
                    fmt(r2(x.gross)),
                    fmt(r2(x.expenses)),
                    fmt(r2(x.taxes)),
                    fmt(r2(x.net)),
                    fmt(r2(x.assets - x.current)),
                    fmt(r2(x.current)),
                    fmt(r2(x.assets)),
                    fmt(r2(x.net)),
                    fmt(r2(x.assets)),
                    fmt(r2(x.roi)),
                    fmt(r2(x.margin)),
                    fmt(r4(x.turnover))
                ),
                vec![],
            ));
        }
        st.push(steps(v));
    }
    Ok(Report::new("fin", &m.name, Status::Solved, headline)
        .section("solucion", "Solución", blocks)
        .section("graficos", "Gráficos", vec![ch.block(), ch2.block()])
        .section("pasos", "Paso a paso", st)
        .data(json!({"cases": outs.iter().map(|x| json!({"name": x.name, "net_income": r4(x.net), "total_assets": r4(x.assets), "roi_pct": r4(x.roi)})).collect::<Vec<_>>()}))
        .cite("sarache2026inventarios", "Efecto de los inventarios en el ROI (diapositivas 7–8)")
        .cite("ballou2004", "El inventario como inversión: efecto en la rentabilidad del activo"))
}

fn liquidez(m: &FinModel, o: &FinOptions) -> Result<Report, ModError> {
    if m.current_liabilities <= 0.0 {
        return invalid("el pasivo corriente debe ser positivo");
    }
    let ac = m.banks + m.receivables + m.securities + m.inventories;
    let pc = m.current_liabilities;
    let rc = ac / pc;
    let pa = (ac - m.inventories) / pc;
    let mut t = Table::new(Some("Indicadores de liquidez"), &["Indicador", "Fórmula", "Antes", "Después del ajuste"]).left(0).left(1);
    let mut after = None;
    if let Some(ob) = m.obsolete_pct.filter(|x| *x > 0.0) {
        let rec = m.recovery_pct.unwrap_or(0.0);
        let inv2 = m.inventories * (1.0 - ob / 100.0) + m.inventories * ob / 100.0 * rec / 100.0;
        let ac2 = ac - m.inventories + inv2;
        after = Some((inv2, ac2, ac2 / pc, (ac2 - inv2) / pc, m.inventories - inv2));
    }
    let a = |f: &dyn Fn(&(f64, f64, f64, f64, f64)) -> f64| after.as_ref().map(|x| Cell::Num(r4(f(x)))).unwrap_or(Cell::from("—"));
    t.row(cells!["Inventarios", "", r4(m.inventories), a(&|x| x.0)]);
    t.row(cells!["Activo corriente", "bancos + cartera + valores + inventarios", r4(ac), a(&|x| x.1)]);
    t.row(cells!["Pasivo corriente", "", r4(pc), a(&|_| pc)]);
    t.row_tone(cells!["Razón corriente", "AC / PC", r4(rc), a(&|x| x.2)], Tone::Highlight);
    t.row_tone(cells!["Prueba ácida", "(AC − inventarios) / PC", r4(pa), a(&|x| x.3)], Tone::Highlight);
    t.row(cells!["Capital de trabajo neto", "AC − PC", r4(ac - pc), a(&|x| x.1 - pc)]);
    let mut blocks = vec![metrics(vec![metric_main("Razón corriente", r4(rc)), metric("Prueba ácida", r4(pa)), metric("Capital de trabajo neto", r4(ac - pc))])];
    let mut head = format!("Razón corriente = {} y prueba ácida = {}.", fmt(r4(rc)), fmt(r4(pa)));
    if let Some(x) = after {
        head += &format!(" Con el inventario obsoleto la razón corriente cae a {} (pérdida de {}); la prueba ácida no cambia.", fmt(r4(x.2)), fmt(r4(x.4)));
        blocks.push(text(format!(
            "El {} % del inventario es obsoleto y se recupera el {} % de su valor: inventario = {}·{} + {}·{}·{} = {}. Pérdida = {}. La prueba ácida no cambia porque ya excluía el inventario.",
            fmt(m.obsolete_pct.unwrap()),
            fmt(m.recovery_pct.unwrap_or(0.0)),
            fmt(m.inventories),
            fmt(r4(1.0 - m.obsolete_pct.unwrap() / 100.0)),
            fmt(m.inventories),
            fmt(r4(m.obsolete_pct.unwrap() / 100.0)),
            fmt(r4(m.recovery_pct.unwrap_or(0.0) / 100.0)),
            fmt(r4(x.0)),
            fmt(r4(x.4))
        )));
    }
    blocks.push(t.clone().block());
    blocks.push(text(format!(
        "Por cada peso que se debe a corto plazo hay {} en activo corriente, pero sin vender inventario solo {}. La brecha entre los dos indicadores es el peso del inventario: la liquidez depende de venderlo.",
        fmt(r2(rc)),
        fmt(r2(pa))
    )));
    let mut ch = Chart::new(Some("Liquidez"), "Indicador", "Veces");
    let cats = vec!["Razón corriente".to_string(), "Prueba ácida".to_string()];
    ch.categories = Some(cats);
    ch = ch.series(Series::new("Antes", SeriesKind::Bar, vec![[0.0, r4(rc)], [1.0, r4(pa)]]).tone(Tone::Highlight));
    if let Some(x) = after {
        ch = ch.series(Series::new("Con el ajuste", SeriesKind::Bar, vec![[0.0, r4(x.2)], [1.0, r4(x.3)]]).tone(Tone::Bad));
    }
    ch = ch.hline(1.0, "1", Tone::Muted);
    let st = if o.steps {
        vec![steps(vec![
            step("Activo y pasivo corriente", format!("AC = {} + {} + {} + {} = {}; PC = {}.", fmt(m.banks), fmt(m.receivables), fmt(m.securities), fmt(m.inventories), fmt(r4(ac)), fmt(pc)), vec![]),
            step("Razón corriente y prueba ácida", format!("RC = {} / {} = {}; PA = ({} − {}) / {} = {}.", fmt(r4(ac)), fmt(pc), fmt(r4(rc)), fmt(r4(ac)), fmt(m.inventories), fmt(pc), fmt(r4(pa))), vec![t.block()]),
        ])]
    } else {
        vec![]
    };
    Ok(Report::new("fin", &m.name, Status::Solved, head)
        .section("solucion", "Solución", blocks)
        .section("graficos", "Gráficos", vec![ch.block()])
        .section("pasos", "Paso a paso", st)
        .data(json!({"current_ratio": r4(rc), "acid_test": r4(pa), "after": after.map(|x| json!({"inventories": r4(x.0), "current_ratio": r4(x.2), "acid_test": r4(x.3), "loss": r4(x.4)}))}))
        .cite("sarache2026inventarios", "Efecto de los inventarios en el flujo de efectivo: razón corriente y prueba ácida (diapositivas 9–10)"))
}

struct CycOut {
    name: String,
    inv: f64,
    raw: Option<(f64, f64, f64)>,
    rec: f64,
    pay: f64,
    cycle: f64,
    gap: f64,
    times: f64,
    wc: f64,
    income: f64,
    profit: f64,
    how: String,
}

fn capital(m: &FinModel, o: &FinOptions) -> Result<Report, ModError> {
    if m.cycles.is_empty() {
        return invalid("agrega al menos una situación");
    }
    let dy = m.days_per_year;
    let mut outs: Vec<CycOut> = vec![];
    for (k, c) in m.cycles.iter().enumerate() {
        let name = if c.name.trim().is_empty() { format!("Situación {}", k + 1) } else { c.name.clone() };
        let prev = outs.first();
        let rec = c.days_receivable.or(prev.map(|p| p.rec)).ok_or_else(|| ModError::Invalid(format!("{name}: faltan los días de cartera")))?;
        let pay = c.days_payable.or(prev.map(|p| p.pay)).ok_or_else(|| ModError::Invalid(format!("{name}: faltan los días de pago")))?;
        let parts = match (c.days_raw, c.days_wip, c.days_finished) {
            (Some(a), Some(b), Some(cc)) => Some((a, b, cc)),
            _ => None,
        };
        let mut how = String::new();
        let inv = if let Some((a, b, cc)) = parts {
            a + b + cc
        } else if let Some(i) = c.days_inventory {
            i
        } else if let Some(t) = c.times_per_year {
            // al revés: desfase = días/veces; ciclo = desfase + pago; inventario = ciclo − cartera
            let gap = dy / t;
            let i = gap + pay - rec;
            how = format!("Al revés: desfase = {}/{} = {} días; ciclo = desfase + pago = {} + {} = {}; días de inventario = ciclo − cartera = {} − {} = {}.", fmt(dy), fmt(t), fmt(r4(gap)), fmt(r4(gap)), fmt(pay), fmt(r4(gap + pay)), fmt(r4(gap + pay)), fmt(rec), fmt(r4(i)));
            i
        } else if let (Some(red), Some(p)) = (c.inventory_reduction, prev) {
            how = format!("Inventario {} % menor que en «{}»: {} × {} = {} días (los días de cobertura bajan en la misma proporción que el nivel de inventario).", fmt(red), p.name, fmt(r4(p.inv)), fmt(r4(1.0 - red / 100.0)), fmt(r4(p.inv * (1.0 - red / 100.0))));
            p.inv * (1.0 - red / 100.0)
        } else {
            return invalid(format!("{name}: faltan los días de inventario (o las veces al año, o la reducción frente a la primera situación)"));
        };
        if inv < 0.0 {
            return invalid(format!("{name}: los días de inventario dan negativos ({})", fmt(r4(inv))));
        }
        let cycle = inv + rec;
        let gap = cycle - pay;
        if gap <= 0.0 {
            return invalid(format!("{name}: el pago a proveedores cubre todo el ciclo (desfase ≤ 0): no se necesita capital de trabajo"));
        }
        let times = dy / gap;
        let wc = m.annual_expenses / times;
        let income = m.annual_sales / times;
        outs.push(CycOut { name, inv, raw: parts, rec, pay, cycle, gap, times, wc, income, profit: income - wc, how });
    }
    let mut head = vec!["Concepto".to_string()];
    head.extend(outs.iter().map(|x| x.name.clone()));
    let mut t = Table::new(Some("Ciclo operativo y capital de trabajo"), &head).left(0);
    let row = |t: &mut Table, l: &str, f: &dyn Fn(&CycOut) -> f64, tone: Tone| {
        let mut r = vec![Cell::from(l)];
        r.extend(outs.iter().map(|x| Cell::Num(r4(f(x)))));
        t.row_tone(r, tone);
    };
    row(&mut t, "Días de inventario (MP + PP + PT)", &|x| x.inv, Tone::Normal);
    row(&mut t, "Días de cartera", &|x| x.rec, Tone::Normal);
    row(&mut t, "Ciclo operativo (días)", &|x| x.cycle, Tone::Normal);
    row(&mut t, "Días de pago a proveedores", &|x| x.pay, Tone::Normal);
    row(&mut t, "Desfase (días)", &|x| x.gap, Tone::Total);
    row(&mut t, "Desfase anual (veces)", &|x| x.times, Tone::Normal);
    row(&mut t, "Capital de trabajo = egresos / veces", &|x| x.wc, Tone::Highlight);
    row(&mut t, "Ingreso por ciclo", &|x| x.income, Tone::Normal);
    row(&mut t, "Utilidad por ciclo", &|x| x.profit, Tone::Normal);
    row(&mut t, "Utilidad anual", &|x| x.profit * x.times, Tone::Normal);
    // línea de tiempo del ciclo
    let mut rows = vec![];
    for x in &outs {
        let mut bars = vec![];
        if let Some((a, b, c)) = x.raw {
            bars.push(GanttBar { start: 0.0, end: a, label: Some(format!("MP {}", fmt(a))), tone: Tone::Normal });
            bars.push(GanttBar { start: a, end: a + b, label: Some(format!("PP {}", fmt(b))), tone: Tone::Normal });
            bars.push(GanttBar { start: a + b, end: a + b + c, label: Some(format!("PT {}", fmt(c))), tone: Tone::Normal });
        } else {
            bars.push(GanttBar { start: 0.0, end: x.inv, label: Some(format!("Inventario {}", fmt(r2(x.inv)))), tone: Tone::Normal });
        }
        bars.push(GanttBar { start: x.inv, end: x.cycle, label: Some(format!("Cartera {}", fmt(x.rec))), tone: Tone::Accent });
        rows.push(GanttRow { label: format!("{}: ciclo {}", x.name, fmt(r2(x.cycle))), bars });
        rows.push(GanttRow {
            label: format!("{}: pago y desfase", x.name),
            bars: vec![GanttBar { start: 0.0, end: x.pay, label: Some(format!("Pago {}", fmt(x.pay))), tone: Tone::Muted }, GanttBar { start: x.pay, end: x.cycle, label: Some(format!("Desfase {}", fmt(r2(x.gap)))), tone: Tone::Bad }],
        });
    }
    let gantt = Block::Gantt(Gantt { title: Some("Ciclo operativo: el desfase es lo que la empresa financia con su propio dinero".into()), unit: "días".into(), rows, markers: vec![], legend: vec![] });
    let mut ch = Chart::new(Some("Capital de trabajo necesario"), "Situación", "Capital de trabajo");
    ch.categories = Some(outs.iter().map(|x| x.name.clone()).collect());
    ch = ch.series(Series::new("Capital de trabajo", SeriesKind::Bar, outs.iter().enumerate().map(|(k, x)| [k as f64, r4(x.wc)]).collect()).tone(Tone::Highlight));
    let first = &outs[0];
    let mut headline = format!("{}: desfase de {} días, capital de trabajo {}.", first.name, fmt(r4(first.gap)), fmt(r4(first.wc)));
    let mut blocks = vec![metrics(outs.iter().enumerate().map(|(k, x)| if k == 0 { metric_main(format!("Capital de trabajo ({})", x.name), r4(x.wc)) } else { metric(format!("Capital de trabajo ({})", x.name), r4(x.wc)) }).collect())];
    for x in outs.iter().filter(|x| !x.how.is_empty()) {
        blocks.push(text(format!("{}: {}", x.name, x.how)));
    }
    if outs.len() > 1 {
        let last = outs.last().unwrap();
        headline += &format!(" {}: {} (se liberan {}).", last.name, fmt(r4(last.wc)), fmt(r4(first.wc - last.wc)));
        blocks.push(text(format!(
            "Se liberan {} − {} = {}. La utilidad anual no cambia ({}): lo que cambia es el dinero inmovilizado para obtenerla, que da {} vueltas al año en lugar de {}.",
            fmt(r4(first.wc)),
            fmt(r4(last.wc)),
            fmt(r4(first.wc - last.wc)),
            fmt(r4(m.annual_sales - m.annual_expenses)),
            fmt(r4(last.times)),
            fmt(r4(first.times))
        )));
    }
    blocks.push(t.clone().block());
    let st = if o.steps {
        let mut v = vec![];
        for x in &outs {
            v.push(step(
                x.name.clone(),
                format!(
                    "{}Ciclo = días de inventario + cartera = {} + {} = {}. Desfase = ciclo − pago = {} − {} = {}. Desfase anual = {}/{} = {} veces. Capital de trabajo = egresos / veces = {}/{} = {}. Ingreso por ciclo = {}/{} = {}; utilidad por ciclo = {}.",
                    if x.how.is_empty() { String::new() } else { format!("{} ", x.how) },
                    fmt(r4(x.inv)),
                    fmt(x.rec),
                    fmt(r4(x.cycle)),
                    fmt(r4(x.cycle)),
                    fmt(x.pay),
                    fmt(r4(x.gap)),
                    fmt(dy),
                    fmt(r4(x.gap)),
                    fmt(r4(x.times)),
                    fmt(m.annual_expenses),
                    fmt(r4(x.times)),
                    fmt(r4(x.wc)),
                    fmt(m.annual_sales),
                    fmt(r4(x.times)),
                    fmt(r4(x.income)),
                    fmt(r4(x.profit))
                ),
                vec![],
            ));
        }
        vec![steps(v)]
    } else {
        vec![]
    };
    Ok(Report::new("fin", &m.name, Status::Solved, headline)
        .section("solucion", "Solución", blocks)
        .section("graficos", "Gráficos", vec![gantt, ch.block()])
        .section("pasos", "Paso a paso", st)
        .data(json!({"cycles": outs.iter().map(|x| json!({"name": x.name, "inventory_days": r4(x.inv), "cycle_days": r4(x.cycle), "gap_days": r4(x.gap), "times_per_year": r4(x.times), "working_capital": r4(x.wc), "profit_per_cycle": r4(x.profit)})).collect::<Vec<_>>()}))
        .cite("sarache2026inventarios", "Ciclo operativo, desfase y necesidad de capital de trabajo (diapositivas 12–16)")
        .cite("richards1980", "Ciclo de conversión del efectivo: días de inventario + días de cartera − días de pago"))
}

pub struct Fin;

impl TypedModule for Fin {
    type Model = FinModel;
    type Options = FinOptions;

    fn file_tag(&self) -> &'static str {
        "IAQSB-FIN"
    }

    fn spec(&self) -> ModuleSpec {
        let pt = |id: &str, label: &str, desc: &str, layout: Vec<Widget>, dims: Vec<Field>| ProblemType { id: id.into(), label: label.into(), description: desc.into(), dims, layout };
        let opt = |k: &str, l: &str| Field::number(k, l).optional();
        ModuleSpec {
            id: "fin".into(),
            winqsb: "Nuevo".into(),
            name: "Finanzas del inventario".into(),
            description: "Efecto del inventario en el ROI, la liquidez (razón corriente y prueba ácida) y el capital de trabajo (ciclo operativo y desfase)".into(),
            file_ext: "FIN".into(),
            problem_types: vec![
                pt(
                    "roi",
                    "Rentabilidad (ROI)",
                    "ROI = utilidad neta / activo total, en uno o varios escenarios",
                    vec![
                        Widget::records(
                            Some("Escenarios"),
                            "cases",
                            vec![
                                Field::text("name", "Escenario"),
                                Field::number("sales", "Ventas"),
                                Field::number("cost_of_sales", "Costo de ventas"),
                                opt("fixed_expenses", "Gastos fijos"),
                                opt("variable_expenses", "Gastos variables"),
                                opt("total_expenses", "o gastos totales"),
                                Field::number("taxes", "Impuestos"),
                                Field::number("fixed_assets", "Activo fijo"),
                                Field::number("banks", "Bancos"),
                                Field::number("receivables", "Cuentas por cobrar"),
                                Field::number("inventories", "Inventarios"),
                            ],
                            Some("Añadir escenario"),
                        ),
                        Widget::params(
                            Some("Reducción del inventario (opcional)"),
                            vec![
                                opt("reduce_inventory_to", "Reducir el inventario a"),
                                Field::boolean("proportional_expenses", "Los gastos bajan en la misma proporción", false),
                                opt("tax_change", "Cambio en impuestos (%)").help("−10 = bajan 10 %"),
                            ],
                        ),
                    ],
                    vec![Field::integer("cases", "Escenarios", 1)],
                ),
                pt(
                    "liquidez",
                    "Liquidez",
                    "Razón corriente, prueba ácida y efecto del inventario obsoleto",
                    vec![Widget::params(
                        None,
                        vec![
                            Field::number("banks", "Bancos"),
                            Field::number("receivables", "Cuentas por cobrar"),
                            Field::number("securities", "Valores negociables"),
                            Field::number("inventories", "Inventarios"),
                            Field::number("current_liabilities", "Pasivo corriente (cuentas por pagar y deudas de corto plazo)"),
                            opt("obsolete_pct", "Inventario obsoleto (%)"),
                            opt("recovery_pct", "Valor que se recupera del obsoleto (%)"),
                        ],
                    )],
                    vec![],
                ),
                pt(
                    "capital",
                    "Capital de trabajo",
                    "Ciclo operativo, desfase, capital de trabajo y utilidad por ciclo; también al revés (con las veces que se repite el desfase)",
                    vec![
                        Widget::params(
                            Some("Presupuestos"),
                            vec![Field::number("annual_expenses", "Presupuesto anual de egresos (PAE)"), Field::number("annual_sales", "Presupuesto anual de ventas (PAV)"), Field::number("days_per_year", "Días del año").default(360.0)],
                        ),
                        Widget::records(
                            Some("Situaciones"),
                            "cycles",
                            vec![
                                Field::text("name", "Situación"),
                                opt("days_raw", "Días MP"),
                                opt("days_wip", "Días PP"),
                                opt("days_finished", "Días PT"),
                                opt("days_inventory", "o días de inventario total"),
                                opt("days_receivable", "Días de cartera"),
                                opt("days_payable", "Días de pago"),
                                opt("times_per_year", "o veces al año del desfase"),
                                opt("inventory_reduction", "o reducción del inventario (%)"),
                            ],
                            Some("Añadir situación"),
                        ),
                    ],
                    vec![],
                ),
            ],
            options: vec![Field::boolean("steps", "Mostrar paso a paso", true)],
        }
    }

    fn template(&self, pt: &str, dims: &Value) -> Result<FinModel, ModError> {
        let t: FinType = serde_json::from_value(Value::from(pt)).map_err(|_| ModError::Invalid(format!("tipo de problema desconocido: {pt}")))?;
        let mut m: FinModel = serde_json::from_value(json!({"name": "Finanzas del inventario", "problem_type": t})).unwrap();
        match t {
            FinType::Roi => {
                let n = dim(dims, "cases", 1, 1, 20)?;
                m.cases = (1..=n).map(|i| RoiCase { name: format!("Escenario {i}"), ..Default::default() }).collect();
            }
            FinType::Capital => m.cycles = vec![Cycle { name: "Actual".into(), ..Default::default() }],
            FinType::Liquidez => {}
        }
        Ok(m)
    }

    fn solve(&self, m: &FinModel, o: &FinOptions) -> Result<Report, ModError> {
        let mut r = match m.problem_type {
            FinType::Roi => roi(m, o)?,
            FinType::Liquidez => liquidez(m, o)?,
            FinType::Capital => capital(m, o)?,
        };
        if r.problem.is_empty() {
            r.problem = m.name.clone();
        }
        Ok(r)
    }

    fn read(&self, bytes: &[u8]) -> Result<FinModel, ModError> {
        let s = String::from_utf8_lossy(bytes);
        let s = s.trim_start_matches('\u{feff}');
        let (head, rest) = s.split_once('\n').unwrap_or((s, ""));
        if !head.trim().to_ascii_uppercase().starts_with("IAQSB-FIN") {
            return Err(ModError::Format("no es un archivo de finanzas del inventario de IAQSB".into()));
        }
        serde_json::from_str(rest).map_err(|e| ModError::Format(e.to_string()))
    }

    fn write(&self, m: &FinModel) -> Result<Vec<u8>, ModError> {
        Ok(format!("IAQSB-FIN 1\n{}\n", serde_json::to_string_pretty(m).unwrap()).into_bytes())
    }

    fn examples(&self) -> Vec<(&'static str, FinModel)> {
        let ex = |s: &str| serde_json::from_str::<FinModel>(s).unwrap();
        vec![
            ("roi_escenarios", ex(include_str!("ejemplos/roi_escenarios.json"))),
            ("roi_manufacturera", ex(include_str!("ejemplos/roi_manufacturera.json"))),
            ("liquidez_almacen", ex(include_str!("ejemplos/liquidez.json"))),
            ("capital_de_trabajo", ex(include_str!("ejemplos/capital.json"))),
            ("parcial_capital", ex(include_str!("ejemplos/parcial_capital.json"))),
        ]
    }
}
