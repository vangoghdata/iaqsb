//! PMP · Programa maestro de producción (no existe en WinQSB): desagrega la
//! demanda en lotes de producto terminado.
//!
//! - **Básico** (fabricar para inventario): proyectado disponible = disponible
//!   anterior + PMP − demanda; se programa un lote cuando la demanda no alcanza a
//!   cubrirse con lo disponible (Arango Marín, 2020, diapositivas 8 a 12).
//! - **Ensamble bajo pedido (ATO)**: dentro de la barrera de tiempo de demanda
//!   solo cuentan los pedidos (el programa está congelado); después, el mayor
//!   entre el pronóstico y los pedidos. Disponible para promesa (DPP) discreto:
//!   lo que produce cada lote menos los pedidos hasta el siguiente lote; si un
//!   DPP queda negativo se cubre con el del lote anterior (Vollmann et al., 2005).
//! - **Solicitudes de pedidos**: si se pueden prometer, cuánto como máximo, desde
//!   cuándo completas y cómo queda el PMP al aceptarlas (sin tocar lo congelado).
//! - **Óptimo con el motor**: el PMP de menor costo de preparación más
//!   inventario por programación entera, con la capacidad de cada periodo.
//! - **Capacidad aproximada (RCCP)**: horas que pide el PMP frente a las disponibles.

use iaqsb_core::lp::{solve as lp_solve, Lp, LpStatus, Sense};
use iaqsb_core::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

const EPS: f64 = 1e-9;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
pub struct Periodo {
    #[serde(default)]
    pub name: String,
    /// Pronóstico (en el programa básico, la demanda).
    #[serde(default)]
    pub forecast: f64,
    /// Pedidos de clientes ya comprometidos.
    #[serde(default)]
    pub orders: f64,
    /// Horas disponibles del recurso clave (vacío = sin límite).
    #[serde(default)]
    pub capacity: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
pub struct Solicitud {
    /// Periodo pedido (nombre o número desde 1).
    #[serde(default)]
    pub period: String,
    #[serde(default)]
    pub quantity: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PmpModel {
    #[serde(default)]
    pub name: String,
    /// "basico" (fabricar para inventario) o "ato" (ensamble bajo pedido).
    #[serde(default = "ato")]
    pub problem_type: String,
    /// Disponible inicial (periodo 0).
    #[serde(default)]
    pub on_hand: f64,
    /// Tamaño de lote (0 = lote a lote).
    #[serde(default)]
    pub lot_size: f64,
    #[serde(default)]
    pub safety_stock: f64,
    /// Barrera de tiempo de demanda (periodos congelados).
    #[serde(default)]
    pub demand_fence: u32,
    /// Barrera de tiempo de planificación (0 = todo el horizonte).
    #[serde(default)]
    pub planning_fence: u32,
    /// Horas del recurso clave por unidad (0 = sin análisis de capacidad).
    #[serde(default)]
    pub hours_per_unit: f64,
    /// Costo de preparar un lote (para el óptimo y la comparación).
    #[serde(default)]
    pub setup_cost: f64,
    /// Costo de mantener una unidad un periodo.
    #[serde(default)]
    pub holding_cost: f64,
    pub periods: Vec<Periodo>,
    #[serde(default)]
    pub requests: Vec<Solicitud>,
}

fn ato() -> String {
    "ato".into()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "lowercase")]
pub enum Metodo {
    /// Regla del curso: un lote cuando lo disponible no alcanza.
    #[default]
    Regla,
    /// Programación entera con el motor (menor costo de preparación más inventario).
    Optimo,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct PmpOptions {
    pub steps: bool,
    pub method: Metodo,
    /// En el óptimo, mantener el PMP de la regla dentro de la barrera de demanda.
    pub freeze: bool,
}
impl Default for PmpOptions {
    fn default() -> Self {
        PmpOptions { steps: true, method: Metodo::Regla, freeze: true }
    }
}

fn r2(x: f64) -> f64 {
    tidy((x * 100.0).round() / 100.0)
}

fn lista(v: &[String]) -> String {
    match v.len() {
        0 => String::new(),
        1 => v[0].clone(),
        n => format!("{} y {}", v[..n - 1].join(", "), v[n - 1]),
    }
}

/// Datos ya validados.
struct Datos<'a> {
    m: &'a PmpModel,
    ato: bool,
    n: usize,
    nombres: Vec<String>,
    dtf: usize,
}

impl<'a> Datos<'a> {
    fn new(m: &'a PmpModel) -> Result<Datos<'a>, ModError> {
        let ato = match m.problem_type.as_str() {
            "ato" => true,
            "basico" => false,
            o => return invalid(format!("tipo de problema desconocido: «{o}» (usa basico o ato)")),
        };
        let n = m.periods.len();
        if n == 0 {
            return invalid("agrega los periodos con su demanda");
        }
        if m.lot_size < 0.0 || m.on_hand.is_nan() || m.safety_stock < 0.0 {
            return invalid("el tamaño de lote y el stock de seguridad no pueden ser negativos");
        }
        for (k, p) in m.periods.iter().enumerate() {
            if p.forecast < 0.0 || p.orders < 0.0 {
                return invalid(format!("periodo {}: la demanda y los pedidos no pueden ser negativos", k + 1));
            }
        }
        let nombres: Vec<String> = m.periods.iter().enumerate().map(|(k, p)| if p.name.trim().is_empty() { (k + 1).to_string() } else { p.name.trim().to_string() }).collect();
        let dtf = if ato { (m.demand_fence as usize).min(n) } else { 0 };
        Ok(Datos { m, ato, n, nombres, dtf })
    }

    /// Demanda que usa el programa en el periodo t (0 = primer periodo).
    fn demanda(&self, t: usize, pedidos: &[f64]) -> f64 {
        let p = &self.m.periods[t];
        if !self.ato {
            p.forecast.max(pedidos[t])
        } else if t < self.dtf {
            pedidos[t]
        } else {
            p.forecast.max(pedidos[t])
        }
    }

    fn zona(&self, t: usize) -> &'static str {
        let ptf = self.m.planning_fence as usize;
        if self.ato && t < self.dtf {
            "Congelado"
        } else if ptf > 0 && t >= ptf {
            "Libre"
        } else {
            "Flexible"
        }
    }

    fn periodo(&self, s: &str) -> Option<usize> {
        let s = s.trim();
        if let Some(k) = self.nombres.iter().position(|x| x.eq_ignore_ascii_case(s)) {
            return Some(k);
        }
        let t: usize = s.trim_start_matches(|c: char| !c.is_ascii_digit()).parse().ok()?;
        (1..=self.n).contains(&t).then(|| t - 1)
    }
}

/// Resultado de la regla del curso.
#[derive(Clone)]
struct Plan {
    demanda: Vec<f64>,
    pmp: Vec<f64>,
    disponible: Vec<f64>,
    /// Explicación de cada periodo para el paso a paso.
    porque: Vec<String>,
}

/// Lote(s) que hacen falta para cubrir `faltante`.
fn lotes(faltante: f64, lote: f64) -> f64 {
    if faltante <= EPS {
        0.0
    } else if lote > EPS {
        lote * (faltante / lote - 1e-9).ceil()
    } else {
        faltante
    }
}

/// Regla del curso: PMP = lote si la demanda no alcanza a cubrirse con lo disponible.
/// `fijo[t]` deja el PMP de ese periodo como está (programa congelado).
fn regla(d: &Datos, pedidos: &[f64], fijo: &[Option<f64>]) -> Plan {
    let m = d.m;
    let mut disp = m.on_hand;
    let mut p = Plan { demanda: vec![], pmp: vec![], disponible: vec![], porque: vec![] };
    for t in 0..d.n {
        let dem = d.demanda(t, pedidos);
        let falta = m.safety_stock + dem - disp;
        let q = match fijo.get(t).copied().flatten() {
            Some(q) => q,
            None => lotes(falta, m.lot_size),
        };
        let nuevo = disp + q - dem;
        let porque = if fijo.get(t).copied().flatten().is_some() {
            format!("PMP congelado = {}; disponible = {} + {} − {} = {}", fmt(q), fmt(r2(disp)), fmt(q), fmt(dem), fmt(r2(nuevo)))
        } else if q > EPS {
            let n_lotes = if m.lot_size > EPS { (q / m.lot_size).round() } else { 1.0 };
            format!(
                "{} + {} = {} > {} disponible: se {} → PMP = {}; disponible = {} + {} − {} = {}",
                fmt(dem),
                fmt(m.safety_stock),
                fmt(r2(dem + m.safety_stock)),
                fmt(r2(disp)),
                if m.lot_size <= EPS { "produce lo que falta".to_string() } else if n_lotes > 1.0 { format!("programan {} lotes", fmt(n_lotes)) } else { "programa un lote".to_string() },
                fmt(q),
                fmt(r2(disp)),
                fmt(q),
                fmt(dem),
                fmt(r2(nuevo))
            )
        } else {
            format!("{} + {} = {} ≤ {} disponible: no se programa; disponible = {} − {} = {}", fmt(dem), fmt(m.safety_stock), fmt(r2(dem + m.safety_stock)), fmt(r2(disp)), fmt(r2(disp)), fmt(dem), fmt(r2(nuevo)))
        };
        p.demanda.push(dem);
        p.pmp.push(tidy(q));
        p.disponible.push(tidy(nuevo));
        p.porque.push(porque);
        disp = nuevo;
    }
    p
}

/// Disponible para promesa discreto (con préstamo del lote anterior si queda negativo).
struct Dpp {
    /// DPP en los periodos con lote (y en el primero).
    dpp: Vec<Option<f64>>,
    /// Acumulado: lo que se puede prometer hasta cada periodo.
    acumulado: Vec<f64>,
    /// Cálculo de cada DPP para el paso a paso.
    calculo: Vec<(usize, String)>,
    prestamos: Vec<String>,
    /// Pedidos que no alcanzan a cubrirse ni con el disponible inicial.
    sobrecomprometido: f64,
}

fn dpp(d: &Datos, pmp: &[f64], pedidos: &[f64]) -> Dpp {
    let inicios: Vec<usize> = (0..d.n).filter(|&t| t == 0 || pmp[t] > EPS).collect();
    let mut val = vec![];
    let mut calculo = vec![];
    for (i, &s) in inicios.iter().enumerate() {
        let e = inicios.get(i + 1).copied().unwrap_or(d.n);
        let base = pmp[s] + if s == 0 { d.m.on_hand } else { 0.0 };
        let comprometido: f64 = pedidos[s..e].iter().sum();
        let v = base - comprometido;
        let partes: Vec<String> = pedidos[s..e].iter().map(|x| fmt(*x)).collect();
        let base_txt = if s == 0 && pmp[0] > EPS { format!("({} + {})", fmt(d.m.on_hand), fmt(pmp[0])) } else { fmt(base) };
        calculo.push((s, format!("DPP({}) = {} − ({}) = {}", d.nombres[s], base_txt, partes.join(" + "), fmt(r2(v)))));
        val.push(v);
    }
    let mut prestamos = vec![];
    for i in (1..val.len()).rev() {
        if val[i] < -EPS {
            prestamos.push(format!("El DPP del periodo {} queda en {}: se cubre con el del periodo {}.", d.nombres[inicios[i]], fmt(r2(val[i])), d.nombres[inicios[i - 1]]));
            val[i - 1] += val[i];
            val[i] = 0.0;
        }
    }
    let sobre = if val.first().copied().unwrap_or(0.0) < -EPS { -val[0] } else { 0.0 };
    let mut out = vec![None; d.n];
    for (i, &s) in inicios.iter().enumerate() {
        out[s] = Some(tidy(val[i]));
    }
    let mut acc = 0.0;
    let acumulado = (0..d.n)
        .map(|t| {
            if let Some(v) = out[t] {
                acc += v;
            }
            tidy(acc)
        })
        .collect();
    Dpp { dpp: out, acumulado, calculo, prestamos, sobrecomprometido: tidy(sobre) }
}

/// Costo de un PMP: preparación por cada periodo con lote + mantener el disponible.
fn costo(m: &PmpModel, pmp: &[f64], disp: &[f64]) -> f64 {
    let prep = pmp.iter().filter(|q| **q > EPS).count() as f64 * m.setup_cost;
    let mant: f64 = disp.iter().map(|x| x.max(0.0)).sum::<f64>() * m.holding_cost;
    prep + mant
}

/// PMP óptimo por programación entera: menor costo de preparación más inventario,
/// con la capacidad de cada periodo y (si se pide) lo congelado de la regla.
fn optimo(d: &Datos, base: &Plan, congelar: bool) -> Result<Result<(Vec<f64>, Vec<f64>, f64), String>, ModError> {
    let m = d.m;
    let n = d.n;
    let (prep, mant) = if m.setup_cost <= EPS && m.holding_cost <= EPS { (0.0, 1.0) } else { (m.setup_cost, m.holding_cost) };
    let total: f64 = base.demanda.iter().sum::<f64>() + m.safety_stock;
    let mut lp = Lp::new(0, false);
    let mut prod = vec![];
    let mut inv = vec![];
    for t in 0..n {
        let fijo = congelar && t < d.dtf;
        // producción: lotes enteros (o cantidad libre si es lote a lote)
        let (p, coef) = if m.lot_size > EPS {
            let ub = if fijo { (base.pmp[t] / m.lot_size).round() } else { (total / m.lot_size).ceil() + 1.0 };
            let lb = if fijo { ub } else { 0.0 };
            (lp.add_var(0.0, lb, ub, true), m.lot_size)
        } else {
            let (lb, ub) = if fijo { (base.pmp[t], base.pmp[t]) } else { (0.0, total) };
            (lp.add_var(0.0, lb, ub, false), 1.0)
        };
        // preparación: binaria, obliga a pagar el costo si hay producción
        if prep > EPS || m.lot_size <= EPS {
            let z = lp.add_var(prep, 0.0, 1.0, true);
            let big = if m.lot_size > EPS { (total / m.lot_size).ceil() + 1.0 } else { total.max(1.0) };
            lp.row(vec![(p, 1.0), (z, -big)], Sense::Le, 0.0);
        }
        let i = lp.add_var(mant, m.safety_stock, f64::INFINITY, false);
        // balance: I(t) = I(t−1) + producción − demanda
        let mut fila = vec![(i, 1.0), (p, -coef)];
        let rhs = if t == 0 {
            m.on_hand - base.demanda[0]
        } else {
            fila.push((inv[t - 1], -1.0));
            -base.demanda[t]
        };
        lp.row(fila, Sense::Eq, rhs);
        if m.hours_per_unit > EPS {
            if let Some(c) = m.periods[t].capacity {
                lp.row(vec![(p, coef * m.hours_per_unit)], Sense::Le, c);
            }
        }
        prod.push((p, coef));
        inv.push(i);
    }
    lp.time_limit = Some(60.0);
    let s = lp_solve(&lp);
    match s.status {
        LpStatus::Optimal => {}
        LpStatus::Infeasible => return Ok(Err("con la capacidad de los periodos no se alcanza a cubrir la demanda (ni con el stock de seguridad)".into())),
        LpStatus::Unbounded => return invalid("el modelo no está acotado (revisa los costos)"),
        LpStatus::Error(e) => return invalid(format!("error del optimizador: {e}")),
    }
    let pmp: Vec<f64> = prod.iter().map(|(p, c)| tidy((s.x[*p] * c * 1e6).round() / 1e6)).collect();
    let disp: Vec<f64> = inv.iter().map(|i| tidy((s.x[*i] * 1e6).round() / 1e6)).collect();
    let c = costo(m, &pmp, &disp);
    Ok(Ok((pmp, disp, c)))
}

struct Respuesta {
    periodo: usize,
    cantidad: f64,
    maximo: f64,
    acepta: bool,
    completo_desde: Option<usize>,
    /// PMP con el pedido aceptado (lo congelado no se mueve).
    plan: Plan,
    dpp: Dpp,
    reprograma: bool,
    texto: String,
}

fn responder(d: &Datos, plan: &Plan, dp: &Dpp, s: &Solicitud) -> Result<Respuesta, ModError> {
    let t = d.periodo(&s.period).ok_or_else(|| ModError::Invalid(format!("solicitud: el periodo «{}» no existe", s.period)))?;
    if s.quantity <= 0.0 {
        return invalid(format!("solicitud del periodo {}: la cantidad debe ser positiva", d.nombres[t]));
    }
    let q = s.quantity;
    let maximo = dp.acumulado[t].max(0.0);
    let acepta = q <= maximo + EPS;
    let completo_desde = (t..d.n).find(|&k| dp.acumulado[k] >= q - EPS);
    let mut pedidos: Vec<f64> = d.m.periods.iter().map(|p| p.orders).collect();
    pedidos[t] += q;
    let fijo: Vec<Option<f64>> = (0..d.n).map(|k| (k < d.dtf).then(|| plan.pmp[k])).collect();
    let nuevo = regla(d, &pedidos, &fijo);
    let ndpp = dpp(d, &nuevo.pmp, &pedidos);
    let reprograma = nuevo.pmp.iter().zip(&plan.pmp).any(|(a, b)| (a - b).abs() > EPS);
    let cambios: Vec<String> = (0..d.n).filter(|&k| (nuevo.pmp[k] - plan.pmp[k]).abs() > EPS).map(|k| format!("periodo {}: {} → {}", d.nombres[k], fmt(plan.pmp[k]), fmt(nuevo.pmp[k]))).collect();
    let texto = if acepta {
        let extra = if reprograma { format!(" Al aceptarlo, la regla cambia el PMP ({}).", cambios.join("; ")) } else { " El PMP no cambia.".to_string() };
        format!("Se acepta: en el periodo {} se pueden prometer hasta {} unidades (DPP acumulado) y se piden {}.{extra}", d.nombres[t], fmt(r2(maximo)), fmt(q))
    } else if t < d.dtf {
        format!(
            "No se puede aceptar completo: el periodo {} está dentro de la barrera de demanda (programa congelado) y solo hay {} unidades disponibles para promesa. Se pueden prometer {} ahora{}.",
            d.nombres[t],
            fmt(r2(maximo)),
            fmt(r2(maximo)),
            completo_desde.map(|k| format!(" o las {} completas en el periodo {}", fmt(q), d.nombres[k])).unwrap_or_default()
        )
    } else if ndpp.sobrecomprometido <= EPS {
        format!(
            "Con el PMP actual solo se pueden prometer {} unidades en el periodo {}. Se puede aceptar si se reprograma el PMP fuera de la zona congelada ({}){}.",
            fmt(r2(maximo)),
            d.nombres[t],
            if cambios.is_empty() { "sin cambios".to_string() } else { cambios.join("; ") },
            completo_desde.map(|k| format!("; sin reprogramar, las {} unidades completas estarían en el periodo {}", fmt(q), d.nombres[k])).unwrap_or_default()
        )
    } else {
        format!("No se puede aceptar: aun reprogramando el PMP faltarían {} unidades. Se pueden prometer como máximo {} en el periodo {}.", fmt(ndpp.sobrecomprometido), fmt(r2(maximo)), d.nombres[t])
    };
    Ok(Respuesta { periodo: t, cantidad: q, maximo, acepta, completo_desde, plan: nuevo, dpp: ndpp, reprograma, texto })
}

/// Tabla del programa maestro.
fn tabla(d: &Datos, titulo: &str, plan: &Plan, dp: Option<&Dpp>, pedidos: &[f64], marcar: Option<usize>) -> Table {
    let mut cols: Vec<&str> = vec!["Periodo"];
    if d.ato {
        cols.extend(["Pronóstico", "Pedidos", "Demanda usada"]);
    } else {
        cols.push("Demanda");
    }
    cols.extend(["Proyectado disponible", "PMP"]);
    if dp.is_some() {
        cols.extend(["DPP", "DPP acumulado"]);
    }
    if d.ato {
        cols.push("Zona");
    }
    let mut tb = Table::new(Some(titulo), &cols).left(0);
    let mut fila0 = vec![Cell::from("0")];
    fila0.extend(std::iter::repeat_n(Cell::Empty, if d.ato { 3 } else { 1 }));
    fila0.push(Cell::from(d.m.on_hand));
    fila0.push(Cell::Empty);
    if dp.is_some() {
        fila0.extend([Cell::Empty, Cell::Empty]);
    }
    if d.ato {
        fila0.push(Cell::Empty);
    }
    tb.row_tone(fila0, Tone::Muted);
    for t in 0..d.n {
        let mut f = vec![Cell::from(d.nombres[t].as_str())];
        if d.ato {
            f.extend([Cell::from(d.m.periods[t].forecast), Cell::from(pedidos[t]), Cell::from(plan.demanda[t])]);
        } else {
            f.push(Cell::from(plan.demanda[t]));
        }
        f.push(Cell::from(plan.disponible[t]));
        f.push(if plan.pmp[t] > EPS { Cell::from(plan.pmp[t]) } else { Cell::from(0.0) });
        if let Some(dp) = dp {
            f.push(dp.dpp[t].map(Cell::from).unwrap_or(Cell::Empty));
            f.push(Cell::from(dp.acumulado[t]));
        }
        if d.ato {
            f.push(Cell::from(d.zona(t)));
        }
        let tono = if plan.disponible[t] < -EPS {
            Tone::Bad
        } else if marcar == Some(t) {
            Tone::Accent
        } else if plan.pmp[t] > EPS {
            Tone::Highlight
        } else {
            Tone::Normal
        };
        tb.row_tone(f, tono);
    }
    tb
}

fn grafica(d: &Datos, plan: &Plan, titulo: &str) -> Block {
    let mut ch = Chart::new(Some(titulo), "Periodo", "Unidades");
    ch.categories = Some(d.nombres.clone());
    ch = ch
        .series(Series::new("PMP", SeriesKind::Bar, plan.pmp.iter().enumerate().map(|(k, v)| [k as f64, *v]).collect()).tone(Tone::Accent))
        .series(Series::new("Demanda", SeriesKind::Line, plan.demanda.iter().enumerate().map(|(k, v)| [k as f64, *v]).collect()).tone(Tone::Highlight))
        .series(Series::new("Proyectado disponible", SeriesKind::Line, plan.disponible.iter().enumerate().map(|(k, v)| [k as f64, *v]).collect()).tone(Tone::Muted));
    if d.m.safety_stock > EPS {
        ch = ch.hline(d.m.safety_stock, "stock de seguridad", Tone::Bad);
    }
    ch.block()
}

fn solve_pmp(m: &PmpModel, o: &PmpOptions) -> Result<Report, ModError> {
    let d = Datos::new(m)?;
    let pedidos: Vec<f64> = m.periods.iter().map(|p| p.orders).collect();
    let sin_fijo = vec![None; d.n];
    let base = regla(&d, &pedidos, &sin_fijo);
    let costo_regla = costo(m, &base.pmp, &base.disponible);

    // óptimo con el motor
    let mut plan = base.clone();
    let mut nota_opt = None;
    let mut estado = Status::Solved;
    let mut costo_opt = None;
    if o.method == Metodo::Optimo {
        match optimo(&d, &base, o.freeze)? {
            Ok((pmp, disp, c)) => {
                plan = Plan { demanda: base.demanda.clone(), pmp, disponible: disp, porque: vec![] };
                costo_opt = Some(c);
                estado = Status::Optimal;
            }
            Err(e) => {
                nota_opt = Some(e);
                estado = Status::Infeasible;
            }
        }
    }
    let con_dpp = d.ato || pedidos.iter().any(|x| *x > EPS);
    let dp = con_dpp.then(|| dpp(&d, &plan.pmp, &pedidos));

    let periodos_lote: Vec<String> = (0..d.n).filter(|&t| plan.pmp[t] > EPS).map(|t| d.nombres[t].clone()).collect();
    let fin = plan.disponible.last().copied().unwrap_or(m.on_hand);
    let total_pmp: f64 = plan.pmp.iter().sum();
    let faltantes: Vec<String> = (0..d.n).filter(|&t| plan.disponible[t] < -EPS).map(|t| d.nombres[t].clone()).collect();

    let lote_txt = if m.lot_size > EPS { format!("lotes de {}", fmt(m.lot_size)) } else { "lote a lote".to_string() };
    let mut head = if let Some(e) = &nota_opt {
        format!("No hay un PMP factible con la capacidad dada: {e}.")
    } else if periodos_lote.is_empty() {
        format!("No hace falta producir: el disponible de {} cubre la demanda de los {} periodos.", fmt(m.on_hand), d.n)
    } else if let Some(c) = costo_opt {
        format!("PMP óptimo (programación entera): {} en los periodos {}; costo {} frente a {} con la regla del curso.", lote_txt, lista(&periodos_lote), fmt(r2(c)), fmt(r2(costo_regla)))
    } else {
        format!("PMP con {} en los periodos {}.", lote_txt, lista(&periodos_lote))
    };
    if let Some(dp) = &dp {
        if nota_opt.is_none() {
            head.push_str(&format!(" DPP del periodo {} = {}.", d.nombres[0], fmt(dp.dpp[0].unwrap_or(0.0))));
        }
    }

    // --- solución
    let mut ms = vec![metric_main("Lotes programados", periodos_lote.len() as f64), metric("Total programado", total_pmp), metric("Disponible al final", r2(fin))];
    if let Some(dp) = &dp {
        ms.push(metric("DPP acumulado al final", dp.acumulado.last().copied().unwrap_or(0.0)));
    }
    if m.setup_cost > EPS || m.holding_cost > EPS {
        ms.push(metric("Costo (preparación + inventario)", r2(costo_opt.unwrap_or(costo_regla))));
    }
    let titulo_tabla = if d.ato { "Programa maestro para ensamble bajo pedido (ATO)" } else { "Programa maestro" };
    let mut sol = vec![metrics(ms), tabla(&d, titulo_tabla, &plan, dp.as_ref(), &pedidos, None).block()];
    if let Some(e) = &nota_opt {
        sol.insert(0, text_tone(format!("El óptimo no tiene solución: {e}. Revisa la capacidad o el stock de seguridad."), Tone::Bad));
    }
    if !faltantes.is_empty() {
        sol.push(text_tone(format!("El proyectado disponible queda negativo en los periodos {}: el PMP congelado no alcanza a cubrir los pedidos.", lista(&faltantes)), Tone::Bad));
    }
    if d.ato {
        let ptf = m.planning_fence as usize;
        sol.push(text(format!(
            "Barrera de tiempo de demanda: {} periodo{} (programa congelado: se ignora el pronóstico y priman los pedidos). {}Después de la barrera la demanda es el mayor entre el pronóstico y los pedidos.",
            d.dtf,
            if d.dtf == 1 { "" } else { "s" },
            if ptf > 0 { format!("Barrera de planificación: {ptf} periodos (zona flexible hasta ahí; después, libre). ") } else { String::new() }
        )));
    }
    if let Some(dp) = &dp {
        if !dp.prestamos.is_empty() {
            sol.push(text_tone(dp.prestamos.join(" "), Tone::Highlight));
        }
        if dp.sobrecomprometido > EPS {
            sol.push(text_tone(format!("Hay {} unidades de pedidos que no se alcanzan a cubrir: el DPP del primer periodo queda negativo.", fmt(dp.sobrecomprometido)), Tone::Bad));
        }
    }
    if let Some(c) = costo_opt {
        let mut cmp = Table::new(Some("Regla del curso frente al óptimo"), &["Plan", "Lotes", "Inventario total", "Costo"]).left(0);
        let lotes_n = |p: &[f64]| p.iter().filter(|q| **q > EPS).count() as f64;
        cmp.row(cells!["Regla del curso", lotes_n(&base.pmp), r2(base.disponible.iter().map(|x| x.max(0.0)).sum()), r2(costo_regla)]);
        cmp.row_tone(cells!["Óptimo (programación entera)", lotes_n(&plan.pmp), r2(plan.disponible.iter().sum()), r2(c)], Tone::Highlight);
        sol.push(cmp.block());
        if m.setup_cost <= EPS && m.holding_cost <= EPS {
            sol.push(text("Sin costos de preparación ni de inventario, el óptimo minimiza el inventario total."));
        }
    }

    // --- solicitudes de pedidos
    let mut sol_bloques = vec![];
    let mut resp_data = vec![];
    if let (Some(dp), true) = (&dp, !m.requests.is_empty()) {
        let mut rt = Table::new(Some("Solicitudes de pedidos (independientes, sin efectos acumulativos)"), &["Periodo", "Cantidad", "DPP acumulado", "¿Se acepta?", "Completo desde"]).left(0);
        let mut detalles = vec![];
        for s in &m.requests {
            let r = responder(&d, &plan, dp, s)?;
            let decision = if r.acepta {
                "Sí"
            } else if r.periodo >= d.dtf && r.dpp.sobrecomprometido <= EPS {
                "Sí, reprogramando el PMP"
            } else {
                "No"
            };
            rt.row_tone(
                cells![d.nombres[r.periodo].as_str(), r.cantidad, r2(r.maximo), decision, r.completo_desde.map(|k| Cell::from(d.nombres[k].as_str())).unwrap_or(Cell::from("—"))],
                if r.acepta { Tone::Good } else if decision == "No" { Tone::Bad } else { Tone::Highlight },
            );
            let mut ped2 = pedidos.clone();
            ped2[r.periodo] += r.cantidad;
            detalles.push(text(format!("{} unidades en el periodo {}: {}", fmt(r.cantidad), d.nombres[r.periodo], r.texto)));
            detalles.push(tabla(&d, &format!("PMP si se aceptan {} unidades en el periodo {}", fmt(r.cantidad), d.nombres[r.periodo]), &r.plan, Some(&r.dpp), &ped2, Some(r.periodo)).block());
            resp_data.push(json!({"period": d.nombres[r.periodo], "quantity": r.cantidad, "available_to_promise": r2(r.maximo), "accepted": r.acepta, "decision": decision, "full_from": r.completo_desde.map(|k| d.nombres[k].clone()), "mps_changes": r.reprograma, "mps": r.plan.pmp}));
        }
        sol_bloques.push(rt.block());
        sol_bloques.push(text("Una solicitud se puede prometer si no supera el DPP acumulado del periodo: lo que queda sin comprometer de los lotes anteriores y del lote que cubre ese periodo."));
        sol_bloques.extend(detalles);
    }

    // --- capacidad aproximada (RCCP)
    let mut cap_bloques = vec![];
    let mut sobrecargas = vec![];
    if m.hours_per_unit > EPS {
        let mut ct = Table::new(Some("Plan aproximado de capacidad (RCCP)"), &["Periodo", "PMP", "Horas requeridas", "Horas disponibles", "Utilización (%)"]).left(0);
        let mut carga = vec![];
        for t in 0..d.n {
            let h = plan.pmp[t] * m.hours_per_unit;
            carga.push(h);
            let cap = m.periods[t].capacity;
            let util = cap.filter(|c| *c > EPS).map(|c| r2(100.0 * h / c));
            let sobre = cap.is_some_and(|c| h > c + EPS);
            if sobre {
                sobrecargas.push(d.nombres[t].clone());
            }
            ct.row_tone(cells![d.nombres[t].as_str(), plan.pmp[t], r2(h), cap.map(Cell::from).unwrap_or(Cell::from("—")), util.map(Cell::from).unwrap_or(Cell::from("—"))], if sobre { Tone::Bad } else { Tone::Normal });
        }
        cap_bloques.push(ct.block());
        cap_bloques.push(if sobrecargas.is_empty() {
            text_tone("El PMP cabe en la capacidad de todos los periodos.", Tone::Good)
        } else {
            text_tone(format!("Sobrecarga en los periodos {}: hay que mover lotes, trabajar horas extra o usar el PMP óptimo con capacidad.", lista(&sobrecargas)), Tone::Bad)
        });
        let mut ch = Chart::new(Some("Carga del PMP frente a la capacidad"), "Periodo", "Horas");
        ch.categories = Some(d.nombres.clone());
        ch = ch.series(Series::new("Horas requeridas", SeriesKind::Bar, carga.iter().enumerate().map(|(k, v)| [k as f64, r2(*v)]).collect()).tone(Tone::Accent));
        let caps: Vec<[f64; 2]> = (0..d.n).filter_map(|t| m.periods[t].capacity.map(|c| [t as f64, c])).collect();
        if !caps.is_empty() {
            ch = ch.series(Series::new("Horas disponibles", SeriesKind::Line, caps).tone(Tone::Bad).dashed());
        }
        cap_bloques.push(ch.block());
    }

    // --- paso a paso
    let mut pasos = vec![];
    if o.steps {
        let regla_txt = if m.lot_size > EPS {
            format!("PMP = {} si la demanda (más el stock de seguridad) es mayor que el disponible anterior; si no, 0. Proyectado disponible = disponible anterior + PMP − demanda.", fmt(m.lot_size))
        } else {
            "Lote a lote: PMP = lo que falta para cubrir la demanda y el stock de seguridad. Proyectado disponible = disponible anterior + PMP − demanda.".to_string()
        };
        if d.ato {
            pasos.push(step(
                "Demanda que usa el programa",
                format!("Hasta la barrera de demanda (periodo {}) solo cuentan los pedidos; después, Máx(pronóstico, pedidos).", d.dtf),
                vec![],
            ));
        }
        let mut pt = Table::new(Some("Cálculo periodo a periodo (regla del curso)"), &["Periodo", "Cálculo"]).left(0).left(1);
        for t in 0..d.n {
            pt.row(cells![d.nombres[t].as_str(), base.porque[t].as_str()]);
        }
        pasos.push(step("Proyectado disponible y PMP", regla_txt, vec![pt.block()]));
        if let Some(dp) = &dp {
            let mut dt = Table::new(Some("Disponible para promesa"), &["Periodo", "Cálculo"]).left(0).left(1);
            for (s, c) in &dp.calculo {
                dt.row(cells![d.nombres[*s].as_str(), c.as_str()]);
            }
            pasos.push(step(
                "Disponible para promesa (DPP)",
                "En el primer periodo: disponible inicial + PMP − pedidos hasta el siguiente lote. En cada periodo con lote: PMP − pedidos desde ese periodo hasta el siguiente lote. El DPP acumulado suma los DPP hasta cada periodo.",
                vec![dt.block()],
            ));
        }
        if costo_opt.is_some() {
            pasos.push(step(
                "PMP óptimo con el motor",
                format!(
                    "Programación entera: minimizar {} × (periodos con lote) + {} × (inventario de cada periodo), con inventario(t) = inventario(t−1) + PMP(t) − demanda(t), inventario ≥ {}{}{}.",
                    fmt(m.setup_cost),
                    fmt(if m.setup_cost <= EPS && m.holding_cost <= EPS { 1.0 } else { m.holding_cost }),
                    fmt(m.safety_stock),
                    if m.lot_size > EPS { format!(", PMP en múltiplos de {}", fmt(m.lot_size)) } else { String::new() },
                    if o.freeze && d.dtf > 0 { ", y el PMP de la zona congelada fijo" } else { "" }
                ),
                vec![],
            ));
        }
    }

    let data = json!({
        "problem_type": m.problem_type,
        "method": if costo_opt.is_some() { "optimo" } else { "regla" },
        "periods": (0..d.n).map(|t| json!({
            "period": d.nombres[t], "demand": plan.demanda[t], "projected_available": plan.disponible[t], "mps": plan.pmp[t],
            "atp": dp.as_ref().and_then(|x| x.dpp[t]), "cumulative_atp": dp.as_ref().map(|x| x.acumulado[t]), "zone": d.zona(t),
        })).collect::<Vec<_>>(),
        "lot_periods": periodos_lote,
        "total_mps": total_pmp,
        "ending_available": fin,
        "cost": r2(costo_opt.unwrap_or(costo_regla)),
        "rule_cost": r2(costo_regla),
        "requests": resp_data,
        "overloaded_periods": sobrecargas,
    });

    let mut rep = Report::new("pmp", &m.name, estado, head)
        .section("solucion", "Programa maestro", sol)
        .section("graficos", "Gráficos", vec![grafica(&d, &plan, "Programa maestro, demanda y proyectado disponible")])
        .section("solicitudes", "Solicitudes de pedidos", sol_bloques)
        .section("capacidad", "Capacidad", cap_bloques)
        .section("pasos", "Paso a paso", if pasos.is_empty() { vec![] } else { vec![steps(pasos)] })
        .data(data)
        .cite("arango2020maestro", "Programa maestro básico y de ensamble bajo pedido (ATO), barreras de tiempo y disponible para promesa")
        .cite("vollmann2005", "Programa maestro, barreras de tiempo y disponible para promesa discreto y acumulado");
    rep = rep.cite_if(costo_opt.is_some(), "huangfuhall2018", "Motor HiGHS: programación entera para el PMP óptimo");
    rep = rep.cite_if(m.hours_per_unit > EPS, "vollmann2005", "Plan aproximado de capacidad (RCCP)");
    Ok(rep)
}

pub struct Pmp;

impl TypedModule for Pmp {
    type Model = PmpModel;
    type Options = PmpOptions;

    fn file_tag(&self) -> &'static str {
        "IAQSB-PMP"
    }

    fn spec(&self) -> ModuleSpec {
        let costos = Widget::params(
            Some("Capacidad y costos (opcional)"),
            vec![
                Field::number("hours_per_unit", "Horas del recurso clave por unidad").help("Para el plan aproximado de capacidad (RCCP); 0 = sin análisis"),
                Field::number("setup_cost", "Costo de preparar un lote"),
                Field::number("holding_cost", "Costo de mantener una unidad un periodo"),
            ],
        );
        ModuleSpec {
            id: "pmp".into(),
            winqsb: "Nuevo".into(),
            name: "Programa maestro de producción".into(),
            description: "Programa maestro básico y de ensamble bajo pedido (ATO): barreras de tiempo, disponible para promesa, solicitudes de pedidos, capacidad aproximada y PMP óptimo con el motor".into(),
            file_ext: "PMP".into(),
            problem_types: vec![
                ProblemType {
                    id: "basico".into(),
                    label: "Programa maestro básico".into(),
                    description: "Fabricar para inventario: un lote cada vez que lo disponible no alcanza a cubrir la demanda".into(),
                    dims: vec![Field::integer("periods", "Número de periodos", 10)],
                    layout: vec![
                        Widget::params(
                            Some("Datos"),
                            vec![Field::number("on_hand", "Disponible inicial"), Field::number("lot_size", "Tamaño de lote").help("0 = lote a lote"), Field::number("safety_stock", "Stock de seguridad")],
                        ),
                        Widget::records(Some("Demanda por periodo"), "periods", vec![Field::text("name", "Periodo"), Field::number("forecast", "Demanda"), Field::number("capacity", "Horas disponibles").optional()], Some("Añadir periodo")),
                        costos.clone(),
                    ],
                },
                ProblemType {
                    id: "ato".into(),
                    label: "Ensamble bajo pedido (ATO)".into(),
                    description: "Pronóstico y pedidos con barrera de tiempo de demanda, disponible para promesa y respuesta a solicitudes de pedidos".into(),
                    dims: vec![Field::integer("periods", "Número de periodos", 12)],
                    layout: vec![
                        Widget::params(
                            Some("Datos"),
                            vec![
                                Field::number("on_hand", "Disponible inicial"),
                                Field::number("lot_size", "Tamaño de lote").help("0 = lote a lote"),
                                Field::number("safety_stock", "Stock de seguridad"),
                                Field::integer("demand_fence", "Barrera de tiempo de demanda (periodos)", 2),
                                Field::integer("planning_fence", "Barrera de tiempo de planificación (periodos)", 0).help("0 = todo el horizonte"),
                            ],
                        ),
                        Widget::records(
                            Some("Pronóstico y pedidos"),
                            "periods",
                            vec![Field::text("name", "Periodo"), Field::number("forecast", "Pronóstico"), Field::number("orders", "Pedidos"), Field::number("capacity", "Horas disponibles").optional()],
                            Some("Añadir periodo"),
                        ),
                        Widget::records(Some("Solicitudes de pedidos por evaluar"), "requests", vec![Field::text("period", "Periodo"), Field::number("quantity", "Cantidad")], Some("Añadir solicitud")),
                        costos,
                    ],
                },
            ],
            options: vec![
                Field::boolean("steps", "Mostrar paso a paso", true),
                Field::choice("method", "Método", &[("regla", "Regla del curso (un lote cuando no alcanza)"), ("optimo", "Óptimo con el motor (programación entera)")]).comparable(),
                Field::boolean("freeze", "Respetar la zona congelada en el óptimo", true).show_if("method", &["optimo"]),
            ],
        }
    }

    fn template(&self, pt: &str, dims: &Value) -> Result<PmpModel, ModError> {
        let ato = pt != "basico";
        let n = dim(dims, "periods", if ato { 12 } else { 10 }, 1, 520)?;
        Ok(PmpModel {
            name: if ato { "Programa maestro ATO".into() } else { "Programa maestro".into() },
            problem_type: if ato { "ato".into() } else { "basico".into() },
            on_hand: 0.0,
            lot_size: 0.0,
            safety_stock: 0.0,
            demand_fence: if ato { 2 } else { 0 },
            planning_fence: 0,
            hours_per_unit: 0.0,
            setup_cost: 0.0,
            holding_cost: 0.0,
            periods: (1..=n).map(|i| Periodo { name: i.to_string(), ..Default::default() }).collect(),
            requests: vec![],
        })
    }

    fn solve(&self, m: &PmpModel, o: &PmpOptions) -> Result<Report, ModError> {
        solve_pmp(m, o)
    }

    fn read(&self, bytes: &[u8]) -> Result<PmpModel, ModError> {
        let s = String::from_utf8_lossy(bytes);
        let s = s.trim_start_matches('\u{feff}');
        let (head, rest) = s.split_once('\n').unwrap_or((s, ""));
        if !head.trim().to_ascii_uppercase().starts_with("IAQSB-PMP") {
            return Err(ModError::Format("no es un archivo de programa maestro de IAQSB".into()));
        }
        serde_json::from_str(rest).map_err(|e| ModError::Format(e.to_string()))
    }

    fn write(&self, m: &PmpModel) -> Result<Vec<u8>, ModError> {
        Ok(format!("IAQSB-PMP 1\n{}\n", serde_json::to_string_pretty(m).unwrap()).into_bytes())
    }

    fn examples(&self) -> Vec<(&'static str, PmpModel)> {
        vec![
            ("basico_arango", serde_json::from_str(include_str!("ejemplos/basico.json")).unwrap()),
            ("ato_arango", serde_json::from_str(include_str!("ejemplos/ato.json")).unwrap()),
            ("ato_solicitudes", serde_json::from_str(include_str!("ejemplos/ato_solicitudes.json")).unwrap()),
            ("ato_capacidad", serde_json::from_str(include_str!("ejemplos/ato_capacidad.json")).unwrap()),
        ]
    }
}
