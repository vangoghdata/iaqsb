//! AP · Planificación agregada (equivalente al módulo AP de WinQSB).
//!
//! Tres formas del problema, como en WinQSB:
//!
//! - Modelo simple: diez estrategias (nivelada, por bloques, plantilla
//!   constante, persecución de la demanda, con horas extra, sin contratar,
//!   producción propuesta por el usuario y el óptimo por programación lineal),
//!   con su comparación de costos.
//! - Modelo de transporte (Bowman): tabla de transporte resuelta por el método
//!   de transporte de NET, con sus pasos (Vogel, MODI).
//! - Modelo general de programación lineal: tiempo completo y medio tiempo,
//!   contratación y despido, horas extra, subcontratación, pedidos pendientes y
//!   ventas perdidas; resuelto con HiGHS.

use crate::net::{InitialMethod, Net, NetModel, NetNode, NetOptions, NetType, TspMethod};
use iaqsb_core::lp::{solve as lp_solve, Lp, LpStatus, Sense};
use iaqsb_core::winqsb::{num, parse_num, Reader, Writer};
use iaqsb_core::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum ApType {
    #[default]
    Simple,
    Transportation,
    Lp,
}

impl ApType {
    fn code(self) -> u8 {
        match self {
            ApType::Simple => 0,
            ApType::Transportation => 1,
            ApType::Lp => 2,
        }
    }
}

/// Datos de un periodo. Los vacíos son 0 (costos) o «sin límite» (capacidades máximas).
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct ApPeriod {
    pub name: String,
    pub demand: f64,
    /// Capacidad de tiempo normal por trabajador (horas) o, en transporte, en unidades.
    pub regular_capacity: Option<f64>,
    /// Costo por hora (o por unidad en transporte) de tiempo normal.
    pub regular_cost: f64,
    /// Costo por hora de tiempo normal no usado.
    pub undertime_cost: f64,
    pub overtime_capacity: Option<f64>,
    pub overtime_cost: f64,
    pub hiring_cost: f64,
    pub dismissal_cost: f64,
    pub max_workers: Option<f64>,
    pub min_workers: Option<f64>,
    // medio tiempo (modelo de programación lineal; en transporte, capacidad y costo por unidad)
    pub pt_regular_capacity: Option<f64>,
    pub pt_regular_cost: f64,
    pub pt_undertime_cost: f64,
    pub pt_overtime_capacity: Option<f64>,
    pub pt_overtime_cost: f64,
    pub pt_hiring_cost: f64,
    pub pt_dismissal_cost: f64,
    pub pt_max_workers: Option<f64>,
    pub pt_min_workers: Option<f64>,
    pub max_inventory: Option<f64>,
    /// Inventario final mínimo (de seguridad).
    pub min_inventory: f64,
    pub holding_cost: f64,
    pub max_subcontract: Option<f64>,
    pub subcontract_cost: f64,
    pub max_backorder: Option<f64>,
    pub backorder_cost: f64,
    pub max_lost_sales: Option<f64>,
    pub lost_sales_cost: f64,
    /// Otro costo por unidad producida (materiales...).
    pub other_cost: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ApModel {
    #[serde(default)]
    pub name: String,
    pub problem_type: ApType,
    /// Nombre del recurso (trabajador, máquina...).
    #[serde(default = "def_resource")]
    pub resource: String,
    /// Unidad de capacidad (hora...).
    #[serde(default = "def_unit")]
    pub capacity_unit: String,
    /// Horas (unidad de capacidad) que requiere cada unidad producida.
    #[serde(default = "one")]
    pub capacity_per_unit: f64,
    /// Inventario inicial (+) o pedidos pendientes (−).
    #[serde(default)]
    pub initial_inventory: f64,
    #[serde(default)]
    pub initial_workers: f64,
    #[serde(default)]
    pub initial_part_time: f64,
    #[serde(default)]
    pub overtime: bool,
    #[serde(default)]
    pub part_time: bool,
    #[serde(default)]
    pub hiring: bool,
    #[serde(default)]
    pub subcontracting: bool,
    #[serde(default)]
    pub backorder: bool,
    #[serde(default)]
    pub lost_sales: bool,
    pub periods: Vec<ApPeriod>,
}

fn def_resource() -> String {
    "trabajador".into()
}
fn def_unit() -> String {
    "hora".into()
}
fn one() -> f64 {
    1.0
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum Strategy {
    /// Óptimo por programación lineal.
    #[default]
    Lp,
    /// Producción constante promedio (estrategia nivelada).
    Level,
    /// Promedio por bloques de periodos.
    PeriodicAverage,
    /// Plantilla constante propuesta, a plena capacidad normal.
    ConstantRegular,
    /// Plantilla inicial constante, a plena capacidad normal.
    ConstantInitial,
    /// Plantilla mínima constante, a plena capacidad normal.
    ConstantMinimum,
    /// Producir lo que se demanda en tiempo normal, contratando y despidiendo (persecución).
    Chase,
    /// Hasta la demanda con tiempo normal y horas extra, sin cambiar la plantilla.
    ChaseOvertime,
    /// Hasta la demanda en tiempo normal sin contratar ni despedir.
    NoHiring,
    /// Producción propuesta por el usuario.
    User,
}

const STRATS: [(Strategy, &str); 10] = [
    (Strategy::Lp, "Óptimo (programación lineal)"),
    (Strategy::Level, "Producción constante promedio (nivelada)"),
    (Strategy::PeriodicAverage, "Promedio por bloques (nivelada)"),
    (Strategy::ConstantRegular, "Plantilla constante propuesta"),
    (Strategy::ConstantInitial, "Plantilla inicial constante"),
    (Strategy::ConstantMinimum, "Plantilla mínima constante"),
    (Strategy::Chase, "Hasta la demanda, tiempo normal (persecución)"),
    (Strategy::ChaseOvertime, "Hasta la demanda, normal y extra"),
    (Strategy::NoHiring, "Hasta la demanda sin contratar ni despedir"),
    (Strategy::User, "Producción propuesta por el usuario"),
];

fn strat_label(s: Strategy) -> &'static str {
    STRATS.iter().find(|x| x.0 == s).unwrap().1
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct ApOptions {
    pub steps: bool,
    /// Modelo simple: estrategia.
    pub strategy: Strategy,
    /// Plantilla propuesta (estrategia de plantilla constante).
    pub workers: Option<f64>,
    /// Periodos por bloque (promedio por bloques).
    pub block: u32,
    /// Producción propuesta por periodo: "100, 120, 90".
    pub production: Option<String>,
    /// Cantidades y plantilla enteras.
    pub whole: bool,
}

impl Default for ApOptions {
    fn default() -> Self {
        ApOptions { steps: true, strategy: Strategy::Lp, workers: None, block: 2, production: None, whole: true }
    }
}

/// Un plan: plantilla, producción y su efecto en inventario.
#[derive(Debug, Clone, Default)]
struct Plan {
    /// Trabajadores [tiempo completo, medio tiempo] por periodo.
    w: Vec<[f64; 2]>,
    hire: Vec<[f64; 2]>,
    fire: Vec<[f64; 2]>,
    /// Horas normales usadas y horas extra por clase.
    reg_h: Vec<[f64; 2]>,
    ot_h: Vec<[f64; 2]>,
    /// Producción total (unidades) por periodo.
    prod: Vec<f64>,
    sub: Vec<f64>,
    inv: Vec<f64>,
    back: Vec<f64>,
    lost: Vec<f64>,
    feasible: bool,
    issue: Option<String>,
}

#[derive(Debug, Clone, Default)]
struct Costs {
    regular: f64,
    undertime: f64,
    overtime: f64,
    hiring: f64,
    dismissal: f64,
    holding: f64,
    subcontract: f64,
    backorder: f64,
    lost: f64,
    other: f64,
}

impl Costs {
    fn total(&self) -> f64 {
        self.regular + self.undertime + self.overtime + self.hiring + self.dismissal + self.holding + self.subcontract + self.backorder + self.lost + self.other
    }
    fn rows(&self) -> Vec<(&'static str, f64)> {
        vec![
            ("Tiempo normal", self.regular),
            ("Tiempo ocioso", self.undertime),
            ("Horas extra", self.overtime),
            ("Contratación", self.hiring),
            ("Despido", self.dismissal),
            ("Inventario", self.holding),
            ("Subcontratación", self.subcontract),
            ("Pedidos pendientes", self.backorder),
            ("Ventas perdidas", self.lost),
            ("Otros costos de producción", self.other),
        ]
    }
}

fn opt0(x: Option<f64>) -> f64 {
    x.unwrap_or(f64::INFINITY)
}

fn costs(m: &ApModel, p: &Plan) -> Costs {
    let mut c = Costs::default();
    for (t, q) in m.periods.iter().enumerate() {
        let classes = [
            (q.regular_capacity.unwrap_or(0.0), q.regular_cost, q.undertime_cost, q.overtime_cost, q.hiring_cost, q.dismissal_cost),
            (q.pt_regular_capacity.unwrap_or(0.0), q.pt_regular_cost, q.pt_undertime_cost, q.pt_overtime_cost, q.pt_hiring_cost, q.pt_dismissal_cost),
        ];
        for (k, (cap, rc, uc, oc, hc, dc)) in classes.iter().enumerate() {
            c.regular += rc * p.reg_h[t][k];
            c.undertime += uc * (cap * p.w[t][k] - p.reg_h[t][k]).max(0.0);
            c.overtime += oc * p.ot_h[t][k];
            c.hiring += hc * p.hire[t][k];
            c.dismissal += dc * p.fire[t][k];
        }
        c.holding += q.holding_cost * p.inv[t];
        c.subcontract += q.subcontract_cost * p.sub[t];
        c.backorder += q.backorder_cost * p.back[t];
        c.lost += q.lost_sales_cost * p.lost[t];
        c.other += q.other_cost * p.prod[t];
    }
    c
}

// ------------------------------------------------------------- modelo de PL

fn lp_plan(m: &ApModel, whole: bool, part_time: bool) -> Result<Plan, ModError> {
    let n = m.periods.len();
    let r = m.capacity_per_unit.max(1e-12);
    let mut lp = Lp::new(0, false);
    let classes: Vec<usize> = if part_time { vec![0, 1] } else { vec![0] };
    // índices por periodo
    struct Ix {
        w: [usize; 2],
        h: [usize; 2],
        f: [usize; 2],
        rg: [usize; 2],
        ot: [usize; 2],
        s: usize,
        i: usize,
        b: usize,
        l: usize,
    }
    let mut ix = vec![];
    for (t, q) in m.periods.iter().enumerate() {
        let mut e = Ix { w: [usize::MAX; 2], h: [usize::MAX; 2], f: [usize::MAX; 2], rg: [usize::MAX; 2], ot: [usize::MAX; 2], s: 0, i: 0, b: 0, l: 0 };
        for &k in &classes {
            let (cap, rc, uc, otcap, oc, hc, dc, mx, mn) = if k == 0 {
                (q.regular_capacity.unwrap_or(0.0), q.regular_cost, q.undertime_cost, q.overtime_capacity.unwrap_or(0.0), q.overtime_cost, q.hiring_cost, q.dismissal_cost, q.max_workers, q.min_workers)
            } else {
                (q.pt_regular_capacity.unwrap_or(0.0), q.pt_regular_cost, q.pt_undertime_cost, q.pt_overtime_capacity.unwrap_or(0.0), q.pt_overtime_cost, q.pt_hiring_cost, q.pt_dismissal_cost, q.pt_max_workers, q.pt_min_workers)
            };
            let w0 = if k == 0 { m.initial_workers } else { m.initial_part_time };
            // costo de la plantilla: tiempo ocioso = cap·W − horas usadas → uc·cap·W − uc·horas
            let (lb, ub) = if m.hiring { (mn.unwrap_or(0.0), mx.unwrap_or(f64::INFINITY)) } else { (w0, w0) };
            e.w[k] = lp.add_var(uc * cap, lb, ub.max(lb), whole);
            e.h[k] = lp.add_var(hc, 0.0, if m.hiring { f64::INFINITY } else { 0.0 }, whole);
            e.f[k] = lp.add_var(dc, 0.0, if m.hiring { f64::INFINITY } else { 0.0 }, whole);
            e.rg[k] = lp.add_var(rc - uc + q.other_cost / r, 0.0, f64::INFINITY, false);
            e.ot[k] = lp.add_var(oc + q.other_cost / r, 0.0, if m.overtime { f64::INFINITY } else { 0.0 }, false);
            // horas normales ≤ cap·W ; extra ≤ otcap·W
            lp.row(vec![(e.rg[k], 1.0), (e.w[k], -cap)], Sense::Le, 0.0);
            lp.row(vec![(e.ot[k], 1.0), (e.w[k], -otcap)], Sense::Le, 0.0);
            // W_t = W_{t−1} + H − F
            if t == 0 {
                lp.row(vec![(e.w[k], 1.0), (e.h[k], -1.0), (e.f[k], 1.0)], Sense::Eq, w0);
            } else {
                let prev: &Ix = &ix[t - 1];
                lp.row(vec![(e.w[k], 1.0), (prev.w[k], -1.0), (e.h[k], -1.0), (e.f[k], 1.0)], Sense::Eq, 0.0);
            }
        }
        e.s = lp.add_var(q.subcontract_cost, 0.0, if m.subcontracting { opt0(q.max_subcontract) } else { 0.0 }, whole);
        e.i = lp.add_var(q.holding_cost, q.min_inventory, opt0(q.max_inventory).max(q.min_inventory), whole);
        let last = t + 1 == n;
        e.b = lp.add_var(q.backorder_cost, 0.0, if m.backorder && !last { opt0(q.max_backorder) } else { 0.0 }, whole);
        e.l = lp.add_var(q.lost_sales_cost, 0.0, if m.lost_sales { opt0(q.max_lost_sales) } else { 0.0 }, whole);
        // balance: I_t − B_t − (I_{t−1} − B_{t−1}) − producción − S − L = −D
        let mut row = vec![(e.i, 1.0), (e.b, -1.0), (e.s, -1.0), (e.l, -1.0)];
        for &k in &classes {
            row.push((e.rg[k], -1.0 / r));
            row.push((e.ot[k], -1.0 / r));
        }
        let rhs = if t == 0 {
            m.initial_inventory - q.demand
        } else {
            row.push((ix[t - 1].i, -1.0));
            row.push((ix[t - 1].b, 1.0));
            -q.demand
        };
        lp.row(row, Sense::Eq, rhs);
        ix.push(e);
    }
    if whole {
        lp.time_limit = Some(30.0);
    }
    let s = lp_solve(&lp);
    match s.status {
        LpStatus::Optimal => {}
        LpStatus::Infeasible => {
            return Ok(Plan { feasible: false, issue: Some("con las capacidades y límites dados no hay forma de cubrir la demanda".into()), ..Default::default() });
        }
        LpStatus::Unbounded => return invalid("el modelo no está acotado (revisa costos negativos)"),
        LpStatus::Error(e) => return invalid(format!("error del optimizador: {e}")),
    }
    let x = |i: usize| if i == usize::MAX { 0.0 } else { tidy(s.x[i]) };
    let mut p = Plan { feasible: true, ..Default::default() };
    for (t, e) in ix.iter().enumerate() {
        p.w.push([x(e.w[0]), x(e.w[1])]);
        p.hire.push([x(e.h[0]), x(e.h[1])]);
        p.fire.push([x(e.f[0]), x(e.f[1])]);
        p.reg_h.push([x(e.rg[0]), x(e.rg[1])]);
        p.ot_h.push([x(e.ot[0]), x(e.ot[1])]);
        p.prod.push(tidy((x(e.rg[0]) + x(e.rg[1]) + x(e.ot[0]) + x(e.ot[1])) / r));
        p.sub.push(x(e.s));
        p.inv.push(x(e.i));
        p.back.push(x(e.b));
        p.lost.push(x(e.l));
        let _ = t;
    }
    Ok(p)
}

// ------------------------------------------------------- estrategias simples

/// Simula un plan dados la plantilla y la producción normal y extra objetivo
/// (en unidades); los faltantes se cubren con subcontratación, pendientes y
/// ventas perdidas (en ese orden, si se permiten).
fn simulate(m: &ApModel, w: &[f64], reg_units: &[f64], ot_units: &[f64]) -> Plan {
    let n = m.periods.len();
    let r = m.capacity_per_unit.max(1e-12);
    let mut p = Plan { feasible: true, ..Default::default() };
    let mut pos = m.initial_inventory; // inventario (+) o pendientes (−)
    let mut prev_w = m.initial_workers;
    for t in 0..n {
        let q = &m.periods[t];
        let cap = q.regular_capacity.unwrap_or(0.0);
        let otcap = if m.overtime { q.overtime_capacity.unwrap_or(0.0) } else { 0.0 };
        let wt = w[t];
        let mut reg = reg_units[t].clamp(0.0, wt * cap / r);
        let ot = ot_units[t].clamp(0.0, wt * otcap / r);
        let last = t + 1 == n;
        // no pasar del inventario máximo: se produce menos en tiempo normal
        let mut end = pos + reg + ot - q.demand;
        if let Some(mx) = q.max_inventory {
            if end > mx {
                reg = (reg - (end - mx)).max(0.0);
                end = pos + reg + ot - q.demand;
            }
        }
        let mut sub = 0.0;
        let mut lost = 0.0;
        let target = q.min_inventory;
        if end < target {
            let mut short = target - end;
            if m.subcontracting {
                sub = short.min(opt0(q.max_subcontract));
                short -= sub;
                end += sub;
            }
            if short > 1e-9 {
                // pendientes (no en el último periodo)
                let back_ok = m.backorder && !last;
                let floor = if back_ok { -opt0(q.max_backorder) } else { 0.0 };
                if end < floor {
                    let need = floor - end;
                    if m.lost_sales {
                        lost = need.min(opt0(q.max_lost_sales));
                        end += lost;
                    }
                }
                if end < floor - 1e-9 {
                    p.feasible = false;
                    p.issue.get_or_insert(format!("en {} faltan {} unidades que no se pueden cubrir", q.name, fmt(floor - end)));
                }
            }
        }
        if end < 0.0 && !(m.backorder && !last) {
            p.feasible = false;
            p.issue.get_or_insert(format!("en {} faltan {} unidades", q.name, fmt(-end)));
        }
        let (h, f) = if wt >= prev_w { (wt - prev_w, 0.0) } else { (0.0, prev_w - wt) };
        if !m.hiring && (h > 1e-9 || f > 1e-9) {
            p.feasible = false;
            p.issue.get_or_insert("la estrategia cambia la plantilla y no se permite contratar ni despedir".into());
        }
        if let Some(mx) = q.max_workers {
            if wt > mx + 1e-9 {
                p.feasible = false;
                p.issue.get_or_insert(format!("en {} la plantilla supera el máximo", q.name));
            }
        }
        p.w.push([wt, 0.0]);
        p.hire.push([h, 0.0]);
        p.fire.push([f, 0.0]);
        p.reg_h.push([reg * r, 0.0]);
        p.ot_h.push([ot * r, 0.0]);
        p.prod.push(reg + ot);
        p.sub.push(sub);
        p.inv.push(end.max(0.0));
        p.back.push((-end).max(0.0));
        p.lost.push(lost);
        pos = end;
        prev_w = wt;
    }
    p
}

fn workers_for(m: &ApModel, t: usize, units: f64, whole: bool) -> f64 {
    let cap = m.periods[t].regular_capacity.unwrap_or(0.0);
    if cap <= 0.0 {
        return 0.0;
    }
    let w = units * m.capacity_per_unit / cap;
    let w = if whole { (w - 1e-9).ceil() } else { w };
    let w = w.max(m.periods[t].min_workers.unwrap_or(0.0));
    w.max(0.0)
}

fn heuristic(m: &ApModel, s: Strategy, o: &ApOptions) -> Result<Plan, ModError> {
    let n = m.periods.len();
    let whole = o.whole;
    let rnd = |x: f64| if whole { (x - 1e-9).ceil().max(0.0) } else { x.max(0.0) };
    let net_total: f64 = m.periods.iter().map(|p| p.demand).sum::<f64>() + m.periods.last().map(|p| p.min_inventory).unwrap_or(0.0) - m.initial_inventory;
    let full = |w: &[f64]| -> Vec<f64> { (0..n).map(|t| w[t] * m.periods[t].regular_capacity.unwrap_or(0.0) / m.capacity_per_unit.max(1e-12)).collect() };
    let zeros = vec![0.0; n];
    Ok(match s {
        Strategy::Lp => unreachable!(),
        Strategy::Level | Strategy::PeriodicAverage => {
            let k = if s == Strategy::Level { n } else { (o.block.max(1) as usize).min(n) };
            let mut prod = vec![0.0; n];
            let mut carried = m.initial_inventory;
            let mut t = 0;
            while t < n {
                let e = (t + k).min(n);
                let need: f64 = m.periods[t..e].iter().map(|p| p.demand).sum::<f64>() + if e == n { m.periods[n - 1].min_inventory } else { 0.0 } - carried;
                let avg = rnd(need.max(0.0) / (e - t) as f64);
                for x in prod.iter_mut().take(e).skip(t) {
                    *x = avg;
                }
                carried += avg * (e - t) as f64 - m.periods[t..e].iter().map(|p| p.demand).sum::<f64>();
                t = e;
            }
            let _ = net_total;
            let w: Vec<f64> = if m.hiring { (0..n).map(|t| workers_for(m, t, prod[t], whole)).collect() } else { vec![m.initial_workers; n] };
            let reg: Vec<f64> = (0..n).map(|t| prod[t].min(full(&w)[t])).collect();
            let ot: Vec<f64> = (0..n).map(|t| (prod[t] - reg[t]).max(0.0)).collect();
            simulate(m, &w, &reg, &ot)
        }
        Strategy::ConstantRegular | Strategy::ConstantInitial | Strategy::ConstantMinimum => {
            let wc = match s {
                Strategy::ConstantRegular => o.workers.ok_or_else(|| ModError::Invalid("indica la plantilla propuesta".into()))?,
                Strategy::ConstantInitial => m.initial_workers,
                _ => m.periods.iter().filter_map(|p| p.min_workers).fold(f64::NAN, f64::max).max(0.0),
            };
            let w = vec![if whole { wc.ceil() } else { wc }; n];
            simulate(m, &w, &full(&w), &zeros)
        }
        Strategy::Chase | Strategy::ChaseOvertime | Strategy::NoHiring => {
            // producción para llegar justo a la demanda (más el inventario de seguridad)
            let mut w = vec![0.0; n];
            let mut reg = vec![0.0; n];
            let mut ot = vec![0.0; n];
            let mut pos = m.initial_inventory;
            for t in 0..n {
                let need = rnd((m.periods[t].demand + m.periods[t].min_inventory - pos).max(0.0));
                let cap_u = |wt: f64| wt * m.periods[t].regular_capacity.unwrap_or(0.0) / m.capacity_per_unit.max(1e-12);
                if s == Strategy::Chase && m.hiring {
                    w[t] = workers_for(m, t, need, whole);
                    reg[t] = need.min(cap_u(w[t]));
                } else {
                    w[t] = m.initial_workers;
                    reg[t] = need.min(cap_u(w[t]));
                    if s == Strategy::ChaseOvertime && m.overtime {
                        let otcap = w[t] * m.periods[t].overtime_capacity.unwrap_or(0.0) / m.capacity_per_unit.max(1e-12);
                        ot[t] = (need - reg[t]).max(0.0).min(otcap);
                    }
                }
                pos += reg[t] + ot[t] - m.periods[t].demand;
                pos = pos.max(if m.backorder { f64::NEG_INFINITY } else { 0.0 });
            }
            simulate(m, &w, &reg, &ot)
        }
        Strategy::User => {
            let txt = o.production.as_ref().filter(|s| !s.trim().is_empty()).ok_or_else(|| ModError::Invalid("indica la producción propuesta por periodo".into()))?;
            let mut prod = vec![0.0; n];
            for (t, part) in txt.split([',', ';']).enumerate().take(n) {
                let p = part.trim();
                if !p.is_empty() {
                    prod[t] = parse_num(p).ok_or_else(|| ModError::Invalid(format!("producción «{p}» no válida")))?;
                }
            }
            let w: Vec<f64> = if m.hiring { (0..n).map(|t| workers_for(m, t, prod[t], whole)).collect() } else { vec![m.initial_workers; n] };
            let reg: Vec<f64> = (0..n).map(|t| prod[t].min(full(&w)[t])).collect();
            let ot: Vec<f64> = (0..n).map(|t| (prod[t] - reg[t]).max(0.0)).collect();
            simulate(m, &w, &reg, &ot)
        }
    })
}

// ------------------------------------------------------------ informe del plan

fn plan_blocks(m: &ApModel, p: &Plan, title: &str) -> (Vec<Block>, Costs) {
    let c = costs(m, p);
    let pt = m.part_time && m.problem_type == ApType::Lp;
    let r = m.capacity_per_unit;
    let mut head = vec!["Periodo", "Demanda", "Plantilla", "Contrata", "Despide"];
    if pt {
        head.extend(["Medio tiempo", "Contrata (m.t.)", "Despide (m.t.)"]);
    }
    head.extend(["Producción normal", "Producción extra", "Subcontrato", "Inventario final", "Pendientes", "Ventas perdidas"]);
    let mut t = Table::new(Some(title), &head).left(0);
    for (k, q) in m.periods.iter().enumerate() {
        let mut row = cells![&q.name, q.demand, p.w[k][0], p.hire[k][0], p.fire[k][0]];
        if pt {
            row.extend(cells![p.w[k][1], p.hire[k][1], p.fire[k][1]]);
        }
        row.extend(cells![tidy((p.reg_h[k][0] + p.reg_h[k][1]) / r), tidy((p.ot_h[k][0] + p.ot_h[k][1]) / r), p.sub[k], p.inv[k], p.back[k], p.lost[k]]);
        t.row(row);
    }
    let mut ct = Table::new(Some("Análisis de costos"), &["Concepto", "Costo"]).left(0);
    for (lab, v) in c.rows() {
        if v.abs() > 1e-12 {
            ct.row(cells![lab, tidy(v)]);
        }
    }
    ct.row_tone(cells!["Total", tidy(c.total())], Tone::Total);
    let mut ch = Chart::new(Some("Demanda, producción e inventario"), "Periodo", "Unidades")
        .series(Series::new("Producción", SeriesKind::Bar, p.prod.iter().enumerate().map(|(k, v)| [k as f64, *v]).collect()).tone(Tone::Accent))
        .series(Series::new("Demanda", SeriesKind::Line, m.periods.iter().enumerate().map(|(k, q)| [k as f64, q.demand]).collect()).tone(Tone::Highlight))
        .series(Series::new("Inventario final", SeriesKind::Line, p.inv.iter().enumerate().map(|(k, v)| [k as f64, *v]).collect()).tone(Tone::Muted));
    if p.back.iter().any(|v| *v > 0.0) {
        ch = ch.series(Series::new("Pendientes", SeriesKind::Line, p.back.iter().enumerate().map(|(k, v)| [k as f64, *v]).collect()).tone(Tone::Bad).dashed());
    }
    ch.categories = Some(m.periods.iter().map(|q| q.name.clone()).collect());
    (vec![metrics(vec![metric_main("Costo total", tidy(c.total())), metric("Producción total", tidy(p.prod.iter().sum::<f64>()))]), t.block(), ct.block(), ch.block()], c)
}

fn plan_data(m: &ApModel, p: &Plan, c: &Costs) -> Value {
    json!({
        "total_cost": tidy(c.total()),
        "costs": c.rows().iter().map(|(k, v)| json!({"item": k, "cost": tidy(*v)})).collect::<Vec<_>>(),
        "periods": m.periods.iter().enumerate().map(|(k, q)| json!({
            "period": q.name, "workers": p.w[k][0], "part_time": p.w[k][1], "hire": p.hire[k][0], "fire": p.fire[k][0],
            "production": p.prod[k], "subcontract": p.sub[k], "inventory": p.inv[k], "backorder": p.back[k], "lost_sales": p.lost[k]
        })).collect::<Vec<_>>(),
    })
}

// ---------------------------------------------------- modelo de transporte

fn transport(m: &ApModel, o: &ApOptions) -> Result<Report, ModError> {
    let n = m.periods.len();
    let mut sources: Vec<(String, f64, usize, f64)> = vec![]; // nombre, capacidad, periodo, costo unitario
    if m.initial_inventory > 0.0 {
        sources.push(("Inventario inicial".into(), m.initial_inventory, 0, 0.0));
    }
    for (t, q) in m.periods.iter().enumerate() {
        sources.push((format!("Normal {}", q.name), q.regular_capacity.unwrap_or(0.0), t, q.regular_cost + q.other_cost));
        if m.overtime {
            sources.push((format!("Extra {}", q.name), q.overtime_capacity.unwrap_or(0.0), t, q.overtime_cost + q.other_cost));
        }
        if m.part_time {
            sources.push((format!("Medio tiempo {}", q.name), q.pt_regular_capacity.unwrap_or(0.0), t, q.pt_regular_cost + q.other_cost));
        }
        if m.subcontracting {
            let total: f64 = m.periods.iter().map(|p| p.demand).sum::<f64>() + m.periods.iter().map(|p| p.min_inventory).fold(0.0, f64::max);
            sources.push((format!("Subcontrato {}", q.name), q.max_subcontract.unwrap_or(total), t, q.subcontract_cost));
        }
    }
    if m.lost_sales {
        let total: f64 = m.periods.iter().map(|p| p.demand).sum();
        let cap = m.periods.iter().map(|p| p.max_lost_sales.unwrap_or(total)).fold(0.0, f64::max).min(total);
        sources.push(("Ventas perdidas".into(), cap, usize::MAX, m.periods.iter().map(|p| p.lost_sales_cost).fold(0.0, f64::max)));
    }
    let hold_sum = |a: usize, b: usize| -> f64 { m.periods[a..b].iter().map(|p| p.holding_cost).sum() };
    let back_sum = |a: usize, b: usize| -> f64 { m.periods[a..b].iter().map(|p| p.backorder_cost).sum() };
    let mut costs = vec![];
    for (name, _, ps, c) in &sources {
        let mut row = vec![];
        for t in 0..n {
            row.push(if *ps == usize::MAX {
                Some(*c)
            } else if name == "Inventario inicial" {
                Some(hold_sum(0, t))
            } else if *ps <= t {
                Some(c + hold_sum(*ps, t))
            } else if m.backorder {
                Some(c + back_sum(t, *ps))
            } else {
                None
            });
        }
        costs.push(row);
    }
    let mut dem: Vec<f64> = m.periods.iter().map(|p| p.demand).collect();
    if m.initial_inventory < 0.0 {
        dem[0] -= m.initial_inventory;
    }
    dem[n - 1] += m.periods[n - 1].min_inventory;
    let nm = NetModel {
        name: m.name.clone(),
        problem_type: NetType::Transportation,
        objective: Objective::Min,
        symmetric: false,
        sources: sources.iter().map(|(nm, cap, _, _)| NetNode { name: nm.clone(), supply: *cap, demand: 0.0, ..Default::default() }).collect(),
        destinations: m.periods.iter().zip(&dem).map(|(p, d)| NetNode { name: p.name.clone(), supply: 0.0, demand: *d, ..Default::default() }).collect(),
        costs,
        nodes: vec![],
        arcs: vec![],
        bounds: vec![],
        start: None,
        end: None,
        positions: None,
    };
    let supply: f64 = sources.iter().map(|s| s.1).sum();
    if supply + 1e-9 < dem.iter().sum::<f64>() {
        return Ok(Report::new("ap", &m.name, Status::Infeasible, "La capacidad total no alcanza para cubrir la demanda.")
            .section("solucion", "Solución", vec![text_tone(format!("Capacidad total {} y demanda total {}: añade horas extra, subcontratación o ventas perdidas.", fmt(supply), fmt(dem.iter().sum::<f64>())), Tone::Bad)])
            .data(json!({"status": "infeasible"})));
    }
    let rep = TypedModule::solve(&Net, &nm, &NetOptions { steps: o.steps, initial: InitialMethod::Vogel, tsp_method: TspMethod::Exact, mst_method: "kruskal".into(), subtour_node: String::new(), mtz_form: "clase".into() })?;
    // plan desde los envíos
    let mut prod = vec![0.0; n];
    let mut sub = vec![0.0; n];
    let mut lost = vec![0.0; n];
    let mut by_src = vec![0.0; sources.len()];
    let mut src_to = vec![vec![0.0; n]; sources.len()];
    if let Some(sh) = rep.data["shipments"].as_array() {
        for s in sh {
            let (from, to, q) = (s["from"].as_str().unwrap_or(""), s["to"].as_str().unwrap_or(""), s["quantity"].as_f64().unwrap_or(0.0));
            let Some(i) = sources.iter().position(|x| x.0 == from) else { continue };
            let Some(t) = m.periods.iter().position(|p| p.name == to) else { continue };
            by_src[i] += q;
            src_to[i][t] += q;
        }
    }
    for (i, (name, _, ps, _)) in sources.iter().enumerate() {
        if *ps == usize::MAX {
            for t in 0..n {
                lost[t] += src_to[i][t];
            }
        } else if name.starts_with("Subcontrato") {
            sub[*ps] += by_src[i];
        } else if name != "Inventario inicial" {
            prod[*ps] += by_src[i];
        }
    }
    let mut t = Table::new(Some("Plan de producción"), &["Periodo", "Demanda", "Producción normal", "Extra", "Medio tiempo", "Subcontrato", "Ventas perdidas", "Inventario final", "Pendientes"]).left(0);
    let mut pos = m.initial_inventory;
    let mut inv = vec![];
    let mut back = vec![];
    for (k, q) in m.periods.iter().enumerate() {
        let get = |prefix: &str| sources.iter().enumerate().filter(|(_, s)| s.2 == k && s.0.starts_with(prefix)).map(|(i, _)| by_src[i]).sum::<f64>();
        let (rg, ot, ptm) = (get("Normal"), get("Extra"), get("Medio tiempo"));
        pos += prod[k] + sub[k] + lost[k] - q.demand;
        inv.push(pos.max(0.0));
        back.push((-pos).max(0.0));
        t.row(cells![&q.name, q.demand, tidy(rg), tidy(ot), tidy(ptm), tidy(sub[k]), tidy(lost[k]), tidy(pos.max(0.0)), tidy((-pos).max(0.0))]);
    }
    let total = rep.data["objective_value"].as_f64().unwrap_or(f64::NAN);
    let mut ch = Chart::new(Some("Demanda, producción e inventario"), "Periodo", "Unidades")
        .series(Series::new("Producción", SeriesKind::Bar, prod.iter().enumerate().map(|(k, v)| [k as f64, *v]).collect()).tone(Tone::Accent))
        .series(Series::new("Demanda", SeriesKind::Line, m.periods.iter().enumerate().map(|(k, q)| [k as f64, q.demand]).collect()).tone(Tone::Highlight))
        .series(Series::new("Inventario final", SeriesKind::Line, inv.iter().enumerate().map(|(k, v)| [k as f64, *v]).collect()).tone(Tone::Muted));
    ch.categories = Some(m.periods.iter().map(|q| q.name.clone()).collect());
    let mut out = Report::new("ap", &m.name, rep.status, format!("Costo mínimo del plan: {} (método de transporte).", fmt(total)))
        .section(
            "solucion",
            "Solución",
            vec![
                metrics(vec![metric_main("Costo total", total), metric("Producción total", tidy(prod.iter().sum::<f64>()))]),
                text("Modelo de transporte de Bowman: cada fuente es una forma de producir en un periodo (inventario inicial, tiempo normal, extra, subcontrato) y cada destino la demanda de un periodo. El costo de cada celda suma el costo de producir y el de guardar (o el de servir con retraso) hasta el periodo de destino; las celdas imposibles quedan prohibidas."),
                t.block(),
                ch.block(),
            ],
        )
        .data(json!({"total_cost": total, "production": prod, "subcontract": sub, "inventory": inv, "backorder": back, "lost_sales": lost, "shipments": rep.data["shipments"].clone()}));
    for s in rep.sections {
        if s.id == "solucion" {
            out = out.section("tabla", "Tabla de transporte", s.blocks);
        } else if s.id == "pasos" || s.id == "analisis" {
            out = out.section(&s.id, &s.title, s.blocks);
        }
    }
    Ok(out)
}

// ------------------------------------------------------------------- módulo

pub struct Ap;

impl ApModel {
    fn validate(&self) -> Result<(), ModError> {
        if self.periods.is_empty() {
            return invalid("no hay periodos");
        }
        check_names("periodos", &self.periods.iter().map(|p| &p.name).collect::<Vec<_>>())?;
        if self.capacity_per_unit <= 0.0 && self.problem_type != ApType::Transportation {
            return invalid("la capacidad que requiere cada unidad debe ser positiva");
        }
        for p in &self.periods {
            if p.demand < 0.0 || !p.demand.is_finite() {
                return invalid(format!("{}: la demanda debe ser un número no negativo", p.name));
            }
        }
        Ok(())
    }
}

fn period_fields() -> Vec<Field> {
    let lp = &["lp"];
    let simple_lp = &["simple", "lp"];
    let num = Field::number;
    let lim = |k: &str, l: &str| Field::number(k, l).optional().help("Vacío = sin límite");
    vec![
        Field::text("name", "Periodo"),
        num("demand", "Demanda"),
        lim("regular_capacity", "Capacidad normal").help("Por trabajador, en la unidad de capacidad (en transporte, unidades)"),
        num("regular_cost", "Costo normal").help("Por unidad de capacidad (en transporte, por unidad)"),
        num("undertime_cost", "Costo de tiempo ocioso").show_if("problem_type", simple_lp),
        lim("overtime_capacity", "Capacidad extra").show_if("overtime", &["true"]),
        num("overtime_cost", "Costo extra").show_if("overtime", &["true"]),
        num("hiring_cost", "Costo de contratar").show_if("hiring", &["true"]).show_if("problem_type", simple_lp),
        num("dismissal_cost", "Costo de despedir").show_if("hiring", &["true"]).show_if("problem_type", simple_lp),
        lim("max_workers", "Plantilla máxima").show_if("hiring", &["true"]).show_if("problem_type", simple_lp),
        lim("min_workers", "Plantilla mínima").show_if("hiring", &["true"]).show_if("problem_type", simple_lp),
        lim("pt_regular_capacity", "Capacidad medio tiempo").show_if("part_time", &["true"]).show_if("problem_type", &["lp", "transportation"]),
        num("pt_regular_cost", "Costo medio tiempo").show_if("part_time", &["true"]).show_if("problem_type", &["lp", "transportation"]),
        num("pt_undertime_cost", "Ocioso medio tiempo").show_if("part_time", &["true"]).show_if("problem_type", lp),
        lim("pt_overtime_capacity", "Extra medio tiempo").show_if("part_time", &["true"]).show_if("overtime", &["true"]).show_if("problem_type", lp),
        num("pt_overtime_cost", "Costo extra medio tiempo").show_if("part_time", &["true"]).show_if("overtime", &["true"]).show_if("problem_type", lp),
        num("pt_hiring_cost", "Contratar medio tiempo").show_if("part_time", &["true"]).show_if("hiring", &["true"]).show_if("problem_type", lp),
        num("pt_dismissal_cost", "Despedir medio tiempo").show_if("part_time", &["true"]).show_if("hiring", &["true"]).show_if("problem_type", lp),
        lim("pt_max_workers", "Máx. medio tiempo").show_if("part_time", &["true"]).show_if("hiring", &["true"]).show_if("problem_type", lp),
        lim("pt_min_workers", "Mín. medio tiempo").show_if("part_time", &["true"]).show_if("hiring", &["true"]).show_if("problem_type", lp),
        lim("max_inventory", "Inventario máximo").show_if("problem_type", simple_lp),
        num("min_inventory", "Inventario de seguridad"),
        num("holding_cost", "Costo de inventario"),
        lim("max_subcontract", "Subcontrato máximo").show_if("subcontracting", &["true"]),
        num("subcontract_cost", "Costo de subcontratar").show_if("subcontracting", &["true"]),
        lim("max_backorder", "Pendientes máximos").show_if("backorder", &["true"]).show_if("problem_type", simple_lp),
        num("backorder_cost", "Costo de pendiente").show_if("backorder", &["true"]),
        lim("max_lost_sales", "Ventas perdidas máximas").show_if("lost_sales", &["true"]),
        num("lost_sales_cost", "Costo de venta perdida").show_if("lost_sales", &["true"]),
        num("other_cost", "Otro costo por unidad"),
    ]
}

impl TypedModule for Ap {
    type Model = ApModel;
    type Options = ApOptions;

    fn file_tag(&self) -> &'static str {
        "AP"
    }

    fn spec(&self) -> ModuleSpec {
        let flags = vec![
            Field::boolean("overtime", "Horas extra", false),
            Field::boolean("part_time", "Medio tiempo", false).show_if("problem_type", &["lp", "transportation"]),
            Field::boolean("hiring", "Contratar y despedir", false).show_if("problem_type", &["simple", "lp"]),
            Field::boolean("subcontracting", "Subcontratación", false),
            Field::boolean("backorder", "Pedidos pendientes", false),
            Field::boolean("lost_sales", "Ventas perdidas", false),
        ];
        let general = vec![
            Field::text("resource", "Recurso").default("trabajador").show_if("problem_type", &["simple", "lp"]),
            Field::text("capacity_unit", "Unidad de capacidad").default("hora").show_if("problem_type", &["simple", "lp"]),
            Field::number("capacity_per_unit", "Capacidad por unidad producida").default(1.0).show_if("problem_type", &["simple", "lp"]),
            Field::number("initial_inventory", "Inventario inicial (+) o pendientes (−)"),
            Field::number("initial_workers", "Plantilla inicial").show_if("problem_type", &["simple", "lp"]),
            Field::number("initial_part_time", "Plantilla inicial de medio tiempo").show_if("problem_type", &["lp"]).show_if("part_time", &["true"]),
        ];
        let layout = vec![
            Widget::params(None, general),
            Widget::params(Some("Opciones del problema"), flags),
            Widget::records(Some("Periodos"), "periods", period_fields(), Some("Añadir periodo")),
        ];
        let pt = |id: &str, label: &str, desc: &str| ProblemType {
            id: id.into(),
            label: label.into(),
            description: desc.into(),
            dims: vec![Field::integer("periods", "Número de periodos", 6)],
            layout: layout.clone(),
        };
        let strategies: Vec<(&str, &str)> = STRATS
            .iter()
            .map(|(s, l)| (serde_json::to_value(s).unwrap().as_str().unwrap().to_string().leak() as &str, *l))
            .collect();
        ModuleSpec {
            id: "ap".into(),
            winqsb: "AP".into(),
            name: "Planificación agregada".into(),
            description: "Producción, plantilla e inventario por periodos: estrategias, modelo de transporte y programación lineal".into(),
            file_ext: "APP".into(),
            problem_types: vec![
                pt("simple", "Modelo simple", "Diez estrategias (nivelada, persecución, plantilla constante...) y el óptimo"),
                pt("transportation", "Modelo de transporte", "Tabla de transporte de Bowman: producir en un periodo para otro"),
                pt("lp", "Programación lineal", "Modelo completo con medio tiempo, contrataciones, extras, subcontratos y pendientes"),
            ],
            options: vec![
                Field::boolean("steps", "Mostrar paso a paso", true),
                Field::choice("strategy", "Estrategia", &strategies).comparable().show_if("problem_type", &["simple"]),
                Field::boolean("whole", "Cantidades enteras", true).show_if("problem_type", &["simple", "lp"]),
                Field::number("workers", "Plantilla propuesta").optional().show_if("problem_type", &["simple"]).show_if("strategy", &["constant_regular"]),
                Field::integer("block", "Periodos por bloque", 2).show_if("problem_type", &["simple"]).show_if("strategy", &["periodic_average"]),
                Field::text("production", "Producción propuesta").optional().help("Una cantidad por periodo, separadas por comas").show_if("problem_type", &["simple"]).show_if("strategy", &["user"]),
            ],
        }
    }

    fn template(&self, pt: &str, dims: &Value) -> Result<ApModel, ModError> {
        let t: ApType = serde_json::from_value(Value::from(pt)).map_err(|_| ModError::Invalid(format!("tipo de problema desconocido: {pt}")))?;
        let n = dim(dims, "periods", 6, 1, 120)?;
        Ok(ApModel {
            name: "Planificación agregada nueva".into(),
            problem_type: t,
            resource: def_resource(),
            capacity_unit: def_unit(),
            capacity_per_unit: 1.0,
            initial_inventory: 0.0,
            initial_workers: 0.0,
            initial_part_time: 0.0,
            overtime: false,
            part_time: false,
            hiring: t != ApType::Transportation,
            subcontracting: false,
            backorder: false,
            lost_sales: false,
            periods: (1..=n).map(|k| ApPeriod { name: format!("Periodo{k}"), ..Default::default() }).collect(),
        })
    }

    fn solve(&self, m: &ApModel, o: &ApOptions) -> Result<Report, ModError> {
        m.validate()?;
        match m.problem_type {
            ApType::Transportation => transport(m, o),
            ApType::Lp => {
                let p = lp_plan(m, o.whole, m.part_time)?;
                if !p.feasible {
                    return Ok(infeasible(m, p.issue.as_deref().unwrap_or("")));
                }
                let (mut blocks, c) = plan_blocks(m, &p, "Plan óptimo (programación lineal)");
                blocks.insert(1, text("Variables por periodo: plantilla, contrataciones, despidos, horas normales y extra (por clase), subcontrato, inventario, pendientes y ventas perdidas. Restricciones: balance de plantilla, capacidad de horas, balance de inventario y límites. Lo resuelve HiGHS (con enteros si se piden cantidades enteras)."));
                Ok(Report::new("ap", &m.name, Status::Optimal, format!("Costo mínimo del plan: {}.", fmt(tidy(c.total())))).section("solucion", "Solución", blocks).data(plan_data(m, &p, &c)))
            }
            ApType::Simple => {
                let p = if o.strategy == Strategy::Lp { lp_plan(m, o.whole, false)? } else { heuristic(m, o.strategy, o)? };
                // comparación de todas las estrategias automáticas
                let mut cmp = Table::new(Some("Comparación de estrategias"), &["Estrategia", "Costo total", "Factible", "Observación"]).left(0).left(2).left(3);
                let mut best: Option<(f64, Strategy)> = None;
                for (s, label) in STRATS {
                    let q = match s {
                        Strategy::Lp => lp_plan(m, o.whole, false)?,
                        Strategy::User => continue,
                        Strategy::ConstantRegular if o.workers.is_none() => continue,
                        _ => heuristic(m, s, o)?,
                    };
                    let tc = if q.feasible { tidy(costs(m, &q).total()) } else { f64::NAN };
                    if q.feasible && best.map(|b| tc < b.0).unwrap_or(true) {
                        best = Some((tc, s));
                    }
                    let tone = if s == o.strategy { Tone::Accent } else if !q.feasible { Tone::Muted } else { Tone::Normal };
                    cmp.row_tone(cells![label, if q.feasible { Cell::Num(tc) } else { Cell::from("—") }, if q.feasible { "sí" } else { "no" }, q.issue.clone().unwrap_or_default()], tone);
                }
                if !p.feasible {
                    let mut r = infeasible(m, p.issue.as_deref().unwrap_or(""));
                    r = r.section("comparacion", "Comparación", vec![cmp.block()]);
                    return Ok(r);
                }
                let (blocks, c) = plan_blocks(m, &p, &format!("Plan: {}", strat_label(o.strategy)));
                let mut cmp_blocks = vec![cmp.block()];
                if let Some((v, s)) = best {
                    cmp_blocks.push(text(format!("La estrategia más barata es «{}», con un costo de {}.", strat_label(s), fmt(v))));
                }
                let mut st = vec![];
                if o.steps {
                    st.push(steps(strategy_steps(m, o, &p)));
                }
                Ok(Report::new("ap", &m.name, Status::Solved, format!("{}: costo total {}.", strat_label(o.strategy), fmt(tidy(c.total()))))
                    .section("solucion", "Solución", blocks)
                    .section("comparacion", "Comparación", cmp_blocks)
                    .section("pasos", "Paso a paso", st)
                    .data(plan_data(m, &p, &c)))
            }
        }
    }

    fn read(&self, bytes: &[u8]) -> Result<ApModel, ModError> {
        read_ap(bytes)
    }

    fn write(&self, m: &ApModel) -> Result<Vec<u8>, ModError> {
        Ok(write_ap(m))
    }

    fn examples(&self) -> Vec<(&'static str, ApModel)> {
        let ex = |s: &str| serde_json::from_str::<ApModel>(s).unwrap();
        vec![
            ("planificacion_simple", ex(include_str!("ejemplos/simple.json"))),
            ("planificacion_transporte", ex(include_str!("ejemplos/transporte.json"))),
            ("planificacion_pl", ex(include_str!("ejemplos/pl.json"))),
        ]
    }
}

fn infeasible(m: &ApModel, why: &str) -> Report {
    Report::new("ap", &m.name, Status::Infeasible, format!("El plan no es factible: {why}."))
        .section("solucion", "Solución", vec![text_tone(format!("No se puede cumplir la demanda con las opciones y límites dados: {why}. Permite horas extra, subcontratación, pendientes o ventas perdidas, o amplía las capacidades."), Tone::Bad)])
        .data(json!({"status": "infeasible", "reason": why}))
}

fn strategy_steps(m: &ApModel, o: &ApOptions, p: &Plan) -> Vec<Step> {
    let r = m.capacity_per_unit;
    let total: f64 = m.periods.iter().map(|q| q.demand).sum();
    let mut v = vec![];
    let desc = match o.strategy {
        Strategy::Lp => "Se plantea un programa lineal con el costo total como objetivo; el resultado es el plan más barato posible con estas opciones.".to_string(),
        Strategy::Level => format!(
            "Estrategia nivelada: producir lo mismo cada periodo. Demanda total {} − inventario inicial {} (+ inventario de seguridad final) repartido entre {} periodos; el inventario absorbe las diferencias.",
            fmt(total),
            fmt(m.initial_inventory),
            m.periods.len()
        ),
        Strategy::PeriodicAverage => format!("Nivelada por bloques de {} periodos: en cada bloque se produce su demanda promedio.", o.block),
        Strategy::ConstantRegular | Strategy::ConstantInitial | Strategy::ConstantMinimum => "Plantilla constante trabajando todo su tiempo normal; los excedentes van a inventario y los faltantes se cubren con subcontratación, pendientes o ventas perdidas.".to_string(),
        Strategy::Chase => "Persecución de la demanda: cada periodo se produce exactamente lo que falta (demanda − inventario disponible), contratando o despidiendo para tener la plantilla justa.".to_string(),
        Strategy::ChaseOvertime => "Hasta la demanda con la plantilla inicial: primero tiempo normal y, si no alcanza, horas extra.".to_string(),
        Strategy::NoHiring => "Hasta la demanda con la plantilla inicial y solo tiempo normal; sin contratar ni despedir.".to_string(),
        Strategy::User => "Producción propuesta por el usuario; la plantilla se ajusta a lo necesario (si se permite contratar).".to_string(),
    };
    let mut t = Table::new(None, &["Periodo", "Inventario inicial", "Demanda", "Plantilla", "Horas normales", "Producción", "Inventario final", "Pendientes"]).left(0);
    let mut pos = m.initial_inventory;
    for (k, q) in m.periods.iter().enumerate() {
        t.row(cells![&q.name, tidy(pos), q.demand, p.w[k][0], tidy(p.reg_h[k][0]), tidy(p.prod[k]), p.inv[k], p.back[k]]);
        pos = p.inv[k] - p.back[k];
    }
    v.push(step(strat_label(o.strategy), desc, vec![t.block()]));
    v.push(step(
        "Plantilla y horas",
        format!(
            "Cada unidad requiere {} {}; un {} aporta su capacidad normal por periodo. Plantilla necesaria = producción × {} / capacidad (redondeada hacia arriba si se piden enteros). Contratar o despedir sale de comparar con el periodo anterior.",
            fmt(r),
            m.capacity_unit,
            m.resource,
            fmt(r)
        ),
        vec![],
    ));
    v
}

// ------------------------------------------------------------------ archivos

fn flag(s: &str) -> bool {
    parse_num(s).map(|x| x != 0.0).unwrap_or_else(|| s.trim().eq_ignore_ascii_case("true"))
}

fn b(x: bool) -> &'static str {
    if x {
        "-1"
    } else {
        "0"
    }
}

/// Filas de la hoja de WinQSB según el tipo y las opciones (en su orden).
fn rows(m: &ApModel) -> Vec<(String, &'static str)> {
    let res = &m.resource;
    let u = if m.problem_type == ApType::Transportation { "Unit".to_string() } else { m.capacity_unit.clone() };
    let mut v: Vec<(String, &'static str)> = vec![("Forecast Demand".into(), "demand")];
    match m.problem_type {
        ApType::Transportation => {
            v.push((format!("Regular Time Capacity in {u}"), "regular_capacity"));
            v.push((format!("Regular Time Cost per {u}"), "regular_cost"));
            if m.overtime {
                v.push((format!("Overtime Capacity in {u}"), "overtime_capacity"));
                v.push((format!("Overtime Cost per {u}"), "overtime_cost"));
            }
            if m.part_time {
                v.push((format!("  Part Time Capacity in {u}"), "pt_regular_capacity"));
                v.push((format!("  Part Time Cost per {u}"), "pt_regular_cost"));
            }
            v.push(("Initial Inventory (+) or Backorder (-)".into(), "@initial_inventory"));
            v.push(("Minimum Ending Inventory (Safety Stock)".into(), "min_inventory"));
            v.push(("Unit Inventory Holding Cost".into(), "holding_cost"));
            if m.subcontracting {
                v.push((format!("Subcontracting Capacity in {u}"), "max_subcontract"));
                v.push(("Unit Subcontracting Cost".into(), "subcontract_cost"));
            }
            if m.backorder {
                v.push(("Unit Backorder Cost".into(), "backorder_cost"));
            }
            if m.lost_sales {
                v.push(("Maximum Lost-Sales in Unit Allowed".into(), "max_lost_sales"));
                v.push(("Unit Lost-Sales Cost".into(), "lost_sales_cost"));
            }
            return v;
        }
        ApType::Simple | ApType::Lp => {
            let lp = m.problem_type == ApType::Lp;
            let pre = if lp && m.part_time { "Full Time - " } else { "" };
            v.push((format!("{pre}Initial Number of {res}"), "@initial_workers"));
            v.push((format!("{pre}Regular Time Capacity in {u} per {res}"), "regular_capacity"));
            v.push((format!("{pre}Regular Time Cost per {u}"), "regular_cost"));
            v.push((format!("{pre}Undertime Cost per {u}"), "undertime_cost"));
            if m.overtime {
                v.push((format!("{pre}Overtime Capacity in {u} per {res}"), "overtime_capacity"));
                v.push((format!("{pre}Overtime Cost per {u}"), "overtime_cost"));
            }
            if m.hiring {
                v.push((format!("{pre}Hiring Cost per {res}"), "hiring_cost"));
                v.push((format!("{pre}Dismissal Cost per {res}"), "dismissal_cost"));
                if lp {
                    v.push((format!("{pre}Maximum Number of {res} Allowed"), "max_workers"));
                    v.push((format!("{pre}Minimum Number of {res} Allowed"), "min_workers"));
                }
            }
            if lp && m.part_time {
                let pre = "  Part Time - ";
                v.push((format!("{pre}Initial Number of {res}"), "@initial_part_time"));
                v.push((format!("{pre}Regular Time Capacity in {u} per {res}"), "pt_regular_capacity"));
                v.push((format!("{pre}Regular Time Cost per {u}"), "pt_regular_cost"));
                v.push((format!("{pre}Undertime Cost per {u}"), "pt_undertime_cost"));
                if m.overtime {
                    v.push((format!("{pre}Overtime Capacity in {u} per {res}"), "pt_overtime_capacity"));
                    v.push((format!("{pre}Overtime Cost per {u}"), "pt_overtime_cost"));
                }
                if m.hiring {
                    v.push((format!("{pre}Hiring Cost per {res}"), "pt_hiring_cost"));
                    v.push((format!("{pre}Dismissal Cost per {res}"), "pt_dismissal_cost"));
                    v.push((format!("{pre}Maximum Number of {res} Allowed"), "pt_max_workers"));
                    v.push((format!("{pre}Minimum Number of {res} Allowed"), "pt_min_workers"));
                }
            }
            v.push(("Initial Inventory (+) or Backorder (-)".into(), "@initial_inventory"));
            v.push((if lp { "Maximum Ending Inventory Allowed" } else { "Maximum Inventory Allowed" }.into(), "max_inventory"));
            v.push(("Minimum Ending Inventory (Safety Stock)".into(), "min_inventory"));
            v.push(("Unit Inventory Holding Cost".into(), "holding_cost"));
            if m.subcontracting {
                v.push((if lp { "Maximum Subcontracting Allowed" } else { "Maximum Subcontracting Allowed (Capacity)" }.into(), "max_subcontract"));
                v.push(("Unit Subcontracting Cost".into(), "subcontract_cost"));
            }
            if m.backorder {
                v.push(("Maximum Backorder Allowed".into(), "max_backorder"));
                v.push(("Unit Backorder Cost".into(), "backorder_cost"));
            }
            if m.lost_sales {
                v.push(("Maximum Lost-Sales Allowed".into(), "max_lost_sales"));
                v.push(("Unit Lost-Sales Cost".into(), "lost_sales_cost"));
            }
            v.push(("Other Unit Production Cost".into(), "other_cost"));
            v.push((format!("Capacity Requirement in {u} per Unit"), "@capacity_per_unit"));
        }
    }
    v
}

/// Clave del campo a partir de la etiqueta de la fila (para leer cualquier variante).
fn key_of(label: &str, part: bool) -> Option<&'static str> {
    let l = label.trim().to_ascii_lowercase();
    let l = l.trim_start_matches("full time - ").trim_start_matches("part time - ").trim();
    let k: &'static str = if l.starts_with("forecast demand") {
        "demand"
    } else if l.starts_with("initial number of") {
        if part { "@initial_part_time" } else { "@initial_workers" }
    } else if l.starts_with("regular time capacity") {
        if part { "pt_regular_capacity" } else { "regular_capacity" }
    } else if l.starts_with("regular time cost") {
        if part { "pt_regular_cost" } else { "regular_cost" }
    } else if l.starts_with("undertime cost") {
        if part { "pt_undertime_cost" } else { "undertime_cost" }
    } else if l.starts_with("overtime capacity") {
        if part { "pt_overtime_capacity" } else { "overtime_capacity" }
    } else if l.starts_with("overtime cost") {
        if part { "pt_overtime_cost" } else { "overtime_cost" }
    } else if l.starts_with("hiring cost") {
        if part { "pt_hiring_cost" } else { "hiring_cost" }
    } else if l.starts_with("dismissal cost") {
        if part { "pt_dismissal_cost" } else { "dismissal_cost" }
    } else if l.starts_with("maximum number of") {
        if part { "pt_max_workers" } else { "max_workers" }
    } else if l.starts_with("minimum number of") {
        if part { "pt_min_workers" } else { "min_workers" }
    } else if l.starts_with("part time capacity") {
        "pt_regular_capacity"
    } else if l.starts_with("part time cost") {
        "pt_regular_cost"
    } else if l.starts_with("initial inventory") {
        "@initial_inventory"
    } else if l.starts_with("maximum ending inventory") || l.starts_with("maximum inventory") {
        "max_inventory"
    } else if l.starts_with("minimum ending inventory") {
        "min_inventory"
    } else if l.starts_with("unit inventory holding") {
        "holding_cost"
    } else if l.starts_with("maximum subcontracting") || l.starts_with("subcontracting capacity") {
        "max_subcontract"
    } else if l.starts_with("unit subcontracting") {
        "subcontract_cost"
    } else if l.starts_with("maximum backorder") {
        "max_backorder"
    } else if l.starts_with("unit backorder") {
        "backorder_cost"
    } else if l.starts_with("maximum lost-sales") {
        "max_lost_sales"
    } else if l.starts_with("unit lost-sales") {
        "lost_sales_cost"
    } else if l.starts_with("other unit production") {
        "other_cost"
    } else if l.starts_with("capacity requirement") {
        "@capacity_per_unit"
    } else {
        return None;
    };
    Some(k)
}

fn set_field(p: &mut ApPeriod, key: &str, v: Option<f64>) {
    let z = v.unwrap_or(0.0);
    match key {
        "demand" => p.demand = z,
        "regular_capacity" => p.regular_capacity = v,
        "regular_cost" => p.regular_cost = z,
        "undertime_cost" => p.undertime_cost = z,
        "overtime_capacity" => p.overtime_capacity = v,
        "overtime_cost" => p.overtime_cost = z,
        "hiring_cost" => p.hiring_cost = z,
        "dismissal_cost" => p.dismissal_cost = z,
        "max_workers" => p.max_workers = v,
        "min_workers" => p.min_workers = v,
        "pt_regular_capacity" => p.pt_regular_capacity = v,
        "pt_regular_cost" => p.pt_regular_cost = z,
        "pt_undertime_cost" => p.pt_undertime_cost = z,
        "pt_overtime_capacity" => p.pt_overtime_capacity = v,
        "pt_overtime_cost" => p.pt_overtime_cost = z,
        "pt_hiring_cost" => p.pt_hiring_cost = z,
        "pt_dismissal_cost" => p.pt_dismissal_cost = z,
        "pt_max_workers" => p.pt_max_workers = v,
        "pt_min_workers" => p.pt_min_workers = v,
        "max_inventory" => p.max_inventory = v,
        "min_inventory" => p.min_inventory = z,
        "holding_cost" => p.holding_cost = z,
        "max_subcontract" => p.max_subcontract = v,
        "subcontract_cost" => p.subcontract_cost = z,
        "max_backorder" => p.max_backorder = v,
        "backorder_cost" => p.backorder_cost = z,
        "max_lost_sales" => p.max_lost_sales = v,
        "lost_sales_cost" => p.lost_sales_cost = z,
        "other_cost" => p.other_cost = z,
        _ => {}
    }
}

fn get_field(p: &ApPeriod, key: &str) -> Option<f64> {
    Some(match key {
        "demand" => p.demand,
        "regular_capacity" => return p.regular_capacity,
        "regular_cost" => p.regular_cost,
        "undertime_cost" => p.undertime_cost,
        "overtime_capacity" => return p.overtime_capacity,
        "overtime_cost" => p.overtime_cost,
        "hiring_cost" => p.hiring_cost,
        "dismissal_cost" => p.dismissal_cost,
        "max_workers" => return p.max_workers,
        "min_workers" => return p.min_workers,
        "pt_regular_capacity" => return p.pt_regular_capacity,
        "pt_regular_cost" => p.pt_regular_cost,
        "pt_undertime_cost" => p.pt_undertime_cost,
        "pt_overtime_capacity" => return p.pt_overtime_capacity,
        "pt_overtime_cost" => p.pt_overtime_cost,
        "pt_hiring_cost" => p.pt_hiring_cost,
        "pt_dismissal_cost" => p.pt_dismissal_cost,
        "pt_max_workers" => return p.pt_max_workers,
        "pt_min_workers" => return p.pt_min_workers,
        "max_inventory" => return p.max_inventory,
        "min_inventory" => p.min_inventory,
        "holding_cost" => p.holding_cost,
        "max_subcontract" => return p.max_subcontract,
        "subcontract_cost" => p.subcontract_cost,
        "max_backorder" => return p.max_backorder,
        "backorder_cost" => p.backorder_cost,
        "max_lost_sales" => return p.max_lost_sales,
        "lost_sales_cost" => p.lost_sales_cost,
        "other_cost" => p.other_cost,
        _ => return None,
    })
}

fn read_ap(bytes: &[u8]) -> Result<ApModel, ModError> {
    let mut r = Reader::new(bytes);
    let mut h = r.line("la cabecera")?;
    if !h.next().trim().to_ascii_uppercase().starts_with("AP") {
        return Err(ModError::Format("no es un archivo de planificación agregada".into()));
    }
    let name = h.next().trim().to_string();
    let problem_type = match parse_num(&h.next()).unwrap_or(0.0) as i64 {
        1 => ApType::Transportation,
        2 => ApType::Lp,
        _ => ApType::Simple,
    };
    let mut l2 = r.line("la segunda línea")?;
    let n = parse_num(&l2.next()).unwrap_or(0.0) as usize;
    if n == 0 || n > 10_000 {
        return Err(ModError::Format("el número de periodos no es válido".into()));
    }
    let resource = Some(l2.next().trim().to_string()).filter(|s| !s.is_empty()).unwrap_or("Employee".into());
    let unit = Some(l2.next().trim().to_string()).filter(|s| !s.is_empty()).unwrap_or("Hour".into());
    let req = parse_num(&l2.next()).filter(|x| *x > 0.0).unwrap_or(1.0);
    let mut l3 = r.line("las opciones")?;
    let f: Vec<bool> = (0..6).map(|_| flag(&l3.next())).collect();
    let head = r.line("los nombres de los periodos")?;
    let mut m = ApModel {
        name: if name.is_empty() { "Aggregate Planning Problem".into() } else { name },
        problem_type,
        resource,
        capacity_unit: unit,
        capacity_per_unit: req,
        initial_inventory: 0.0,
        initial_workers: 0.0,
        initial_part_time: 0.0,
        part_time: f[0],
        overtime: f[1],
        hiring: f[2],
        subcontracting: f[3],
        backorder: f[4],
        lost_sales: f[5],
        periods: (0..n).map(|t| ApPeriod { name: Some(head.get(t + 1).trim()).filter(|s| !s.is_empty()).map(str::to_string).unwrap_or(format!("Period{}", t + 1)), ..Default::default() }).collect(),
    };
    while let Some(l) = r.try_line() {
        let label = l.get(0);
        let part = label.trim_start().to_ascii_lowercase().starts_with("part time -");
        let Some(key) = key_of(label, part) else { continue };
        if let Some(k) = key.strip_prefix('@') {
            // valor único: el primero que haya en la fila
            let v = (1..=n).find_map(|t| parse_num(l.get(t))).unwrap_or(0.0);
            match k {
                "initial_inventory" => m.initial_inventory = v,
                "initial_workers" => m.initial_workers = v,
                "initial_part_time" => m.initial_part_time = v,
                "capacity_per_unit" => m.capacity_per_unit = if v > 0.0 { v } else { 1.0 },
                _ => {}
            }
        } else {
            for t in 0..n {
                let v = parse_num(l.get(t + 1)).filter(|x| x.is_finite() && x.abs() < 1e29);
                set_field(&mut m.periods[t], key, v);
            }
        }
    }
    Ok(m)
}

fn write_ap(m: &ApModel) -> Vec<u8> {
    let mut w = Writer::new();
    let n = m.periods.len();
    w.row(&["AP".to_string(), m.name.clone(), m.problem_type.code().to_string()]);
    w.row(&[n.to_string(), m.resource.clone(), m.capacity_unit.clone(), num(m.capacity_per_unit)]);
    w.row(&[m.part_time, m.overtime, m.hiring, m.subcontracting, m.backorder, m.lost_sales].map(b));
    let mut head = vec!["          DATA ITEM".to_string()];
    head.extend(m.periods.iter().map(|p| p.name.clone()));
    w.row(&head);
    for (label, key) in rows(m) {
        let mut r = vec![label];
        if let Some(k) = key.strip_prefix('@') {
            let v = match k {
                "initial_inventory" => m.initial_inventory,
                "initial_workers" => m.initial_workers,
                "initial_part_time" => m.initial_part_time,
                _ => m.capacity_per_unit,
            };
            r.push(num(v));
            r.extend(std::iter::repeat_n(String::new(), n - 1));
        } else {
            r.extend(m.periods.iter().map(|p| get_field(p, key).map(num).unwrap_or_default()));
        }
        w.row(&r);
    }
    w.bytes()
}
