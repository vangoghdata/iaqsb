//! Sistemas de inventario con demanda aleatoria: (s, Q), (s, S), (R, S) y (R, s, S).
//!
//! Modelo analítico (aproximación clásica con demanda normal en el período de
//! protección L o R + L):
//! - ciclo T = Q/D en revisión continua; T = máx(R, Q/D + R/2) en revisión periódica;
//! - inventario disponible medio = Q/2 + s − D·(L + R/2) + n(s), donde n(s) es la
//!   falta esperada por ciclo (corrige el inventario cuando hay faltantes);
//! - costo por tiempo = K/T + costo de revisión/R + h·inventario + (p·n(s) + F·P(X > s))/T + c·D,
//!   con p = %atrasado·costo de atraso + %perdido·costo de venta perdida.
//!
//! (R, S) es (R, s, S) con s = S. La simulación (pasos de una unidad de tiempo)
//! permite comprobar cualquier política con la distribución real.

use super::{golden, money, ItsModel, ItsOptions, ItsType, SysMode};
use crate::pert::dist::{phi, Law};
use crate::stat::unit_normal_loss;
use iaqsb_core::*;
use rand::SeedableRng;
use serde_json::json;
use statrs::distribution::{ContinuousCDF, Normal};

/// Mínimo global aproximado: busca en una rejilla y refina con sección áurea
/// alrededor del mejor punto (para funciones que no son unimodales).
fn scan(f: &dyn Fn(f64) -> f64, lo: f64, hi: f64, n: usize, log: bool) -> f64 {
    let at = |i: usize| {
        let u = i as f64 / n as f64;
        if log {
            (lo.ln() + (hi.ln() - lo.ln()) * u).exp()
        } else {
            lo + (hi - lo) * u
        }
    };
    let vals: Vec<f64> = (0..=n).map(|i| f(at(i))).collect();
    let k = (0..=n).min_by(|&a, &b| vals[a].partial_cmp(&vals[b]).unwrap_or(std::cmp::Ordering::Equal)).unwrap();
    let (a, b) = (at(k.saturating_sub(1)), at((k + 1).min(n)));
    golden(f, a, b)
}

#[derive(Clone, Copy, Debug)]
pub struct Sys {
    pub d: f64,
    pub vd: f64,
    pub lm: f64,
    pub lv: f64,
    pub k: f64,
    pub kr: f64,
    pub h: f64,
    pub c: f64,
    pub beta: f64,
    pub lam: f64,
    pub pb: f64,
    pub pl: f64,
    pub f: f64,
    /// Exceso medio bajo s al pedir (pedidos de clientes de tamaño > 1).
    pub under: f64,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Eval {
    pub r: f64,
    pub s: f64,
    pub q: f64,
    pub mu: f64,
    pub sd: f64,
    pub n: f64,
    pub p_short: f64,
    pub cycle: f64,
    pub inv: f64,
    pub setup: f64,
    pub review: f64,
    pub holding: f64,
    pub shortage: f64,
    pub material: f64,
}

impl Eval {
    pub fn related(&self) -> f64 {
        self.setup + self.review + self.holding + self.shortage
    }
    pub fn total(&self) -> f64 {
        self.related() + self.material
    }
    pub fn csl(&self) -> f64 {
        1.0 - self.p_short
    }
    pub fn fill(&self, d: f64) -> f64 {
        (1.0 - self.n / (d * self.cycle)).clamp(0.0, 1.0)
    }
}

impl Sys {
    pub fn p(&self) -> f64 {
        self.beta * self.pb + self.lam * self.pl
    }
    /// Media y desviación de la demanda en R + L.
    pub fn prot(&self, r: f64) -> (f64, f64) {
        let t = r + self.lm;
        (self.d * t, (t * self.vd + self.d * self.d * self.lv).max(0.0).sqrt())
    }
    pub fn eval(&self, r: f64, s: f64, q: f64) -> Eval {
        let (mu, sd) = self.prot(r);
        let se = s - self.under;
        let qe = q.max(0.0) + self.under;
        let (n, ps) = if sd > 0.0 {
            let z = (se - mu) / sd;
            (sd * unit_normal_loss(z), 1.0 - phi(z))
        } else {
            ((mu - se).max(0.0), if se < mu { 1.0 } else { 0.0 })
        };
        let cycle = if r > 0.0 { r.max(qe / self.d + r / 2.0) } else { qe / self.d };
        let cycle = cycle.max(1e-12);
        let inv = (qe / 2.0 + se - self.d * (self.lm + r / 2.0) + n).max(0.0);
        Eval {
            r,
            s,
            q,
            mu,
            sd,
            n,
            p_short: ps,
            cycle,
            inv,
            setup: self.k / cycle,
            review: if r > 0.0 { self.kr / r } else { 0.0 },
            holding: self.h * inv,
            shortage: (self.p() * n + self.f * ps) / cycle,
            material: self.c * self.d,
        }
    }
    pub fn eoq(&self) -> f64 {
        (2.0 * self.d * self.k.max(1e-9) / self.h).sqrt()
    }
    fn best_s(&self, r: f64, q: f64) -> f64 {
        let (mu, sd) = self.prot(r);
        let w = sd.max(1e-6 * mu.abs().max(1.0));
        golden(&|s| self.eval(r, s, q).related(), mu - 6.0 * w - q, mu + 10.0 * w + self.under)
    }
    fn best_sq(&self, r: f64, q_fixed: Option<f64>) -> (f64, f64) {
        let e0 = self.eoq();
        if let Some(q) = q_fixed {
            return (self.best_s(r, q), q);
        }
        // descenso alternado desde dos puntos de partida; se queda el mejor
        let run = |q0: f64| {
            let mut q = q0;
            let mut s = self.best_s(r, q);
            for _ in 0..60 {
                let qn = scan(&|x| self.eval(r, s, x).related(), 0.0, e0 * 20.0 + self.d * r, 48, false);
                let sn = self.best_s(r, qn);
                let done = (qn - q).abs() < 1e-7 * (1.0 + q) && (sn - s).abs() < 1e-7 * (1.0 + s.abs());
                q = qn;
                s = sn;
                if done {
                    break;
                }
            }
            (s, q)
        };
        let a = run(e0);
        let b = run(0.0);
        if self.eval(r, a.0, a.1).related() <= self.eval(r, b.0, b.1).related() {
            a
        } else {
            b
        }
    }
    /// Política óptima del modelo analítico (búsqueda por coordenadas y pulido final con Nelder-Mead).
    pub fn optimal(&self, t: ItsType, r_known: Option<f64>) -> Eval {
        let e = self.optimal_coarse(t, r_known);
        // variables libres: (s, q) en SQ/SS, (r, s) en RS, (r, s, q) en RSS (r fijo si se dio)
        let free_r = r_known.is_none() && matches!(t, ItsType::RS | ItsType::RSS);
        let free_q = !matches!(t, ItsType::RS);
        let pack = |e: &Eval| {
            let mut v = vec![];
            if free_r {
                v.push(e.r.max(1e-12).ln());
            }
            v.push(e.s);
            if free_q {
                v.push(e.q);
            }
            v
        };
        let unpack = |v: &[f64]| {
            let mut i = 0;
            let r = if free_r {
                i += 1;
                v[0].exp()
            } else {
                e.r
            };
            let s = v[i];
            let q = if free_q { v[i + 1].max(0.0) } else { e.q };
            (r, s, q)
        };
        let f = |v: &[f64]| {
            let (r, s, q) = unpack(v);
            self.eval(r, s, q).related()
        };
        let x0 = pack(&e);
        let scale: Vec<f64> = x0.iter().enumerate().map(|(k, x)| if free_r && k == 0 { 0.05 } else { 0.02 * x.abs().max(self.eoq() * 0.05).max(1e-6) }).collect();
        let x = nelder_mead(&f, &x0, &scale, 400);
        if f(&x) < f(&x0) {
            let (r, s, q) = unpack(&x);
            self.eval(r, s, q)
        } else {
            e
        }
    }

    fn optimal_coarse(&self, t: ItsType, r_known: Option<f64>) -> Eval {
        match t {
            ItsType::SQ | ItsType::SS => {
                let (s, q) = self.best_sq(0.0, None);
                self.eval(0.0, s, q)
            }
            ItsType::RS => {
                let inner = |r: f64| {
                    let s = self.best_s(r, 0.0);
                    self.eval(r, s, 0.0)
                };
                let r = r_known.unwrap_or_else(|| {
                    let r0 = self.eoq() / self.d;
                    scan(&|r| inner(r).related(), r0 / 50.0, r0 * 300.0, 64, true)
                });
                inner(r)
            }
            _ => {
                let inner = |r: f64| {
                    let (s, q) = self.best_sq(r, None);
                    self.eval(r, s, q)
                };
                let r = r_known.unwrap_or_else(|| {
                    let r0 = self.eoq() / self.d;
                    scan(&|r| inner(r).related(), r0 / 50.0, r0 * 300.0, 64, true)
                });
                let a = inner(r);
                // la cuenca con q = 0 (política (R, S)) puede estar casi empatada con otra: se comparan las dos
                let b = self.optimal_coarse(ItsType::RS, r_known);
                let b = self.eval(b.r, b.s, 0.0);
                if b.related() < a.related() {
                    b
                } else {
                    a
                }
            }
        }
    }
}

/// Nelder-Mead sin restricciones (para pulir el óptimo).
fn nelder_mead(f: &dyn Fn(&[f64]) -> f64, x0: &[f64], step: &[f64], iters: usize) -> Vec<f64> {
    let n = x0.len();
    let mut pts: Vec<Vec<f64>> = vec![x0.to_vec()];
    for i in 0..n {
        let mut p = x0.to_vec();
        p[i] += step[i];
        pts.push(p);
    }
    let mut vals: Vec<f64> = pts.iter().map(|p| f(p)).collect();
    for _ in 0..iters {
        let mut idx: Vec<usize> = (0..=n).collect();
        idx.sort_by(|&a, &b| vals[a].partial_cmp(&vals[b]).unwrap_or(std::cmp::Ordering::Equal));
        pts = idx.iter().map(|&i| pts[i].clone()).collect();
        vals = idx.iter().map(|&i| vals[i]).collect();
        if (vals[n] - vals[0]).abs() <= 1e-12 * vals[0].abs().max(1e-12) {
            break;
        }
        let c: Vec<f64> = (0..n).map(|j| pts[..n].iter().map(|p| p[j]).sum::<f64>() / n as f64).collect();
        let at = |t: f64| -> Vec<f64> { (0..n).map(|j| c[j] + t * (pts[n][j] - c[j])).collect() };
        let xr = at(-1.0);
        let fr = f(&xr);
        if fr < vals[0] {
            let xe = at(-2.0);
            let fe = f(&xe);
            if fe < fr {
                pts[n] = xe;
                vals[n] = fe;
            } else {
                pts[n] = xr;
                vals[n] = fr;
            }
        } else if fr < vals[n - 1] {
            pts[n] = xr;
            vals[n] = fr;
        } else {
            let xc = if fr < vals[n] { at(-0.5) } else { at(0.5) };
            let fc = f(&xc);
            if fc < vals[n].min(fr) {
                pts[n] = xc;
                vals[n] = fc;
            } else {
                for i in 1..=n {
                    pts[i] = (0..n).map(|j| pts[0][j] + 0.5 * (pts[i][j] - pts[0][j])).collect();
                    vals[i] = f(&pts[i]);
                }
            }
        }
    }
    let b = (0..=n).min_by(|&a, &b| vals[a].partial_cmp(&vals[b]).unwrap_or(std::cmp::Ordering::Equal)).unwrap();
    pts[b].clone()
}

fn build(m: &ItsModel) -> Result<(Sys, Law, Law), ModError> {
    let dl = m.demand_dist.law("demanda")?;
    let ll = m.lead_dist.law("tiempo de entrega")?;
    let d = dl.mean();
    if !(d > 0.0) || !d.is_finite() {
        return invalid("la demanda media debe ser positiva");
    }
    if !ll.mean().is_finite() || ll.mean() < 0.0 {
        return invalid("el tiempo de entrega debe tener media finita y no negativa");
    }
    if m.holding_cost <= 0.0 {
        return invalid("el costo de mantener debe ser positivo");
    }
    if [m.setup_cost, m.unit_cost, m.backorder_cost, m.fixed_shortage_cost, m.review_cost].iter().any(|x| *x < 0.0) {
        return invalid("los costos no pueden ser negativos");
    }
    if !(0.0..=100.0).contains(&m.backorder_pct) {
        return invalid("el % de faltantes atrasados debe estar entre 0 y 100");
    }
    let beta = m.backorder_pct / 100.0;
    let lam = 1.0 - beta;
    let pl = m.lost_sale_cost.0;
    if lam > 0.0 && !pl.is_finite() {
        return invalid("hay ventas perdidas: indica su costo por unidad (no M)");
    }
    let sys = Sys {
        d,
        vd: dl.variance(),
        lm: ll.mean(),
        lv: ll.variance(),
        k: m.setup_cost,
        kr: if matches!(m.problem_type, ItsType::RS | ItsType::RSS) { m.review_cost } else { 0.0 },
        h: m.holding_cost,
        c: m.unit_cost,
        beta,
        lam,
        pb: m.backorder_cost,
        pl: if lam > 0.0 { pl } else { 0.0 },
        f: m.fixed_shortage_cost,
        under: if m.problem_type == ItsType::SS { ((m.order_size - 1.0) / 2.0).max(0.0) } else { 0.0 },
    };
    if sys.p() <= 0.0 && sys.f <= 0.0 && sys.k > 0.0 {
        return invalid("sin costo de faltante convendría no tener inventario: indica el costo de atraso o de venta perdida");
    }
    Ok((sys, dl, ll))
}

pub struct SimOut {
    pub cost: f64,
    pub setup: f64,
    pub review: f64,
    pub holding: f64,
    pub shortage: f64,
    pub material: f64,
    pub on_hand: f64,
    pub backlog: f64,
    pub fill: f64,
    pub orders: f64,
    pub stockouts: f64,
    pub profile: Vec<(f64, f64, f64)>,
}

/// Subdivisiones de cada unidad de tiempo en la simulación.
pub const SUB: u64 = 20;

/// Simula la política. La demanda de cada unidad de tiempo sale de su
/// distribución y se reparte de forma pareja en SUB subpasos; la revisión
/// continua mira el inventario en cada subpaso.
pub fn simulate(m: &ItsModel, sys: &Sys, dl: &Law, ll: &Law, e: &Eval, horizon: u32, warmup: u32, seed: u64) -> SimOut {
    let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(seed);
    let t = m.problem_type;
    let big_s = e.s + e.q;
    let r_sub = if e.r > 0.0 { ((e.r * SUB as f64).round() as u64).max(1) } else { 1 };
    let mut on = big_s.max(0.0);
    let mut back: f64 = 0.0;
    let mut pipe: Vec<(u64, f64)> = vec![];
    let mut acc = SimOut { cost: 0.0, setup: 0.0, review: 0.0, holding: 0.0, shortage: 0.0, material: 0.0, on_hand: 0.0, backlog: 0.0, fill: 0.0, orders: 0.0, stockouts: 0.0, profile: vec![] };
    let (mut dem, mut served) = (0.0, 0.0);
    let mut was_short = false;
    let total = horizon as u64 * SUB;
    let start = warmup as u64 * SUB;
    let mut x = 0.0;
    let stride = (SUB / 4).max(1);
    for now in 0..total {
        let collect = now >= start;
        if now % SUB == 0 {
            x = dl.sample(&mut rng) / SUB as f64;
        }
        // llegadas
        let mut arrived = 0.0;
        pipe.retain(|&(at, q)| {
            if at <= now {
                arrived += q;
                false
            } else {
                true
            }
        });
        on += arrived;
        let fill_back = back.min(on);
        on -= fill_back;
        back -= fill_back;
        // demanda del subpaso
        let sold = x.min(on);
        on -= sold;
        let short = x - sold;
        back += short * sys.beta;
        let lost = short * sys.lam;
        let short_now = short > 1e-12;
        if collect {
            dem += x;
            served += sold;
            acc.shortage += short * sys.beta * sys.pb + lost * sys.pl;
            if short_now && !was_short {
                acc.shortage += sys.f;
                acc.stockouts += 1.0;
            }
        }
        was_short = short_now;
        // revisión y pedido
        let ip = on - back + pipe.iter().map(|p| p.1).sum::<f64>();
        let review = match t {
            ItsType::SQ | ItsType::SS => true,
            _ => (now + 1) % r_sub == 0,
        };
        let mut ordered = 0.0;
        if review {
            if matches!(t, ItsType::RS | ItsType::RSS) && collect {
                acc.review += sys.kr;
            }
            match t {
                ItsType::SQ => {
                    if ip <= e.s && e.q > 0.0 {
                        ordered = (((e.s - ip) / e.q).floor() + 1.0) * e.q;
                    }
                }
                ItsType::RS => {
                    if ip < big_s - 1e-9 {
                        ordered = big_s - ip;
                    }
                }
                _ => {
                    if ip <= e.s {
                        ordered = big_s - ip;
                    }
                }
            }
        }
        if ordered > 0.0 {
            let lt = (ll.sample(&mut rng).max(0.0) * SUB as f64).round() as u64;
            pipe.push((now + lt + 1, ordered));
            if collect {
                acc.setup += sys.k;
                acc.material += sys.c * ordered;
                acc.orders += 1.0;
            }
        }
        if collect {
            acc.holding += sys.h * on / SUB as f64;
            acc.on_hand += on;
            acc.backlog += back;
            if acc.profile.len() < 240 && (now - start) % stride == 0 {
                acc.profile.push(((now - start) as f64 / SUB as f64, on - back, on - back + pipe.iter().map(|p| p.1).sum::<f64>()));
            }
        }
    }
    let n = (horizon.saturating_sub(warmup)).max(1) as f64;
    for v in [&mut acc.setup, &mut acc.review, &mut acc.holding, &mut acc.shortage, &mut acc.material, &mut acc.orders, &mut acc.stockouts] {
        *v /= n;
    }
    acc.on_hand /= n * SUB as f64;
    acc.backlog /= n * SUB as f64;
    acc.cost = acc.setup + acc.review + acc.holding + acc.shortage + acc.material;
    acc.fill = if dem > 0.0 { served / dem } else { 1.0 };
    acc
}

fn need(v: Option<f64>, what: &str) -> Result<f64, ModError> {
    v.ok_or_else(|| ModError::Invalid(format!("indica {what} en las opciones")))
}

pub fn solve(m: &ItsModel, o: &ItsOptions) -> Result<Report, ModError> {
    let t = m.problem_type;
    let (sys, dl, ll) = build(m)?;
    let periodic = matches!(t, ItsType::RS | ItsType::RSS);
    let r_known = if periodic { o.review_interval.filter(|r| *r > 0.0) } else { None };
    let z_of = |sl: f64| Normal::new(0.0, 1.0).unwrap().inverse_cdf((sl / 100.0).clamp(1e-9, 1.0 - 1e-9));
    let opt = sys.optimal(t, r_known);
    let e = match o.mode {
        SysMode::Optimal => opt,
        SysMode::Known => match t {
            ItsType::SQ => sys.eval(0.0, need(o.reorder_point, "el punto de reorden s")?, need(o.order_quantity, "la cantidad Q")?),
            ItsType::SS => {
                let s = need(o.reorder_point, "el punto de reorden s")?;
                sys.eval(0.0, s, need(o.order_up_to, "el nivel máximo S")? - s)
            }
            ItsType::RS => {
                let s = need(o.order_up_to, "el nivel máximo S")?;
                sys.eval(need(r_known, "el intervalo de revisión R")?, s, 0.0)
            }
            _ => {
                let s = need(o.reorder_point, "el punto de reorden s")?;
                sys.eval(need(r_known, "el intervalo de revisión R")?, s, need(o.order_up_to, "el nivel máximo S")? - s)
            }
        },
        SysMode::Service => {
            if !(0.0 < o.service_level && o.service_level < 100.0) {
                return invalid("el nivel de servicio debe estar entre 0 y 100 %");
            }
            let z = z_of(o.service_level);
            let r = if periodic { r_known.unwrap_or(sys.eoq() / sys.d) } else { 0.0 };
            let (mu, sd) = sys.prot(r);
            let s = mu + z * sd + sys.under;
            let q = if t == ItsType::RS { 0.0 } else { sys.eoq() };
            sys.eval(r, s, q)
        }
    };
    if e.q < 0.0 {
        return invalid("el nivel máximo S debe ser mayor o igual que el punto de reorden s");
    }
    let u = &m.time_unit;
    let big_s = e.s + e.q;
    let mut pol = Table::new(Some("Política y desempeño"), &["Concepto", "Valor"]).left(0);
    let mut put = |l: String, v: Cell, tone: Tone| {
        pol.row_tone(vec![Cell::from(l), v], tone);
    };
    if periodic {
        put(format!("Intervalo de revisión R ({u})"), money(e.r), Tone::Highlight);
    }
    match t {
        ItsType::SQ => {
            put("Punto de reorden s".into(), money(e.s), Tone::Highlight);
            put("Cantidad a pedir Q".into(), money(e.q), Tone::Highlight);
        }
        ItsType::RS => put("Nivel máximo S".into(), money(big_s), Tone::Highlight),
        _ => {
            put("Punto de reorden s".into(), money(e.s), Tone::Highlight);
            put("Nivel máximo S".into(), money(big_s), Tone::Highlight);
        }
    }
    let prot = if periodic { "R + L" } else { "L" };
    put(format!("Demanda media en {prot}"), money(e.mu), Tone::Normal);
    put(format!("Desviación de la demanda en {prot}"), money(e.sd), Tone::Normal);
    put("Inventario de seguridad".into(), money(e.s - sys.under - e.mu), Tone::Normal);
    put("Nivel de servicio del ciclo (%)".into(), money(100.0 * e.csl()), Tone::Normal);
    put("Tasa de cumplimiento (%)".into(), money(100.0 * e.fill(sys.d)), Tone::Normal);
    put("Faltante esperado por ciclo".into(), money(e.n), Tone::Normal);
    put(format!("Pedidos por {u}"), money(1.0 / e.cycle), Tone::Normal);
    put("Inventario promedio".into(), money(e.inv), Tone::Normal);
    put(format!("Costo de ordenar por {u}"), money(e.setup), Tone::Normal);
    if periodic {
        put(format!("Costo de revisión por {u}"), money(e.review), Tone::Normal);
    }
    put(format!("Costo de mantener por {u}"), money(e.holding), Tone::Normal);
    put(format!("Costo de faltantes por {u}"), money(e.shortage), Tone::Normal);
    put("Subtotal (costos del inventario)".into(), money(e.related()), Tone::Total);
    if sys.c > 0.0 {
        put(format!("Costo de adquisición por {u}"), money(e.material), Tone::Normal);
        put(format!("Costo total por {u}"), money(e.total()), Tone::Total);
    }
    let mode_txt = match o.mode {
        SysMode::Optimal => "Política óptima del modelo".to_string(),
        SysMode::Known => "Política con los valores indicados".to_string(),
        SysMode::Service => format!("Política con nivel de servicio del {} % en {prot}", fmt(o.service_level)),
    };
    let mut blocks = vec![
        metrics(match t {
            ItsType::SQ => vec![metric_main("Punto de reorden s", tidy(e.s)), metric_main("Cantidad Q", tidy(e.q)), metric(format!("Costo del inventario por {u}"), tidy(e.related()))],
            ItsType::SS => vec![metric_main("Punto de reorden s", tidy(e.s)), metric_main("Nivel máximo S", tidy(big_s)), metric(format!("Costo del inventario por {u}"), tidy(e.related()))],
            ItsType::RS => vec![metric_main("Intervalo R", tidy(e.r)), metric_main("Nivel máximo S", tidy(big_s)), metric(format!("Costo del inventario por {u}"), tidy(e.related()))],
            _ => vec![metric_main("Intervalo R", tidy(e.r)), metric_main("Punto de reorden s", tidy(e.s)), metric_main("Nivel máximo S", tidy(big_s)), metric(format!("Costo del inventario por {u}"), tidy(e.related()))],
        }),
        text(format!("{mode_txt}. La demanda en {prot} se aproxima con una normal de media {} y desviación {}.", fmt(tidy(e.mu)), fmt(tidy(e.sd)))),
        pol.clone().block(),
    ];
    if o.mode != SysMode::Optimal {
        blocks.push(text(format!("La política óptima del modelo cuesta {} por {u} (s = {}, S = {}{}).", fmt(tidy(opt.related())), fmt(tidy(opt.s)), fmt(tidy(opt.s + opt.q)), if periodic { format!(", R = {}", fmt(tidy(opt.r))) } else { String::new() })));
    }
    // curva de costo según s (o S)
    let (mu, sd) = (e.mu, e.sd.max(1e-6));
    let pts: Vec<[f64; 2]> = (0..=120).map(|i| {
        let x = mu - 3.0 * sd + 9.0 * sd * i as f64 / 120.0;
        [x, sys.eval(e.r, x, e.q).related()]
    }).collect();
    let xl = if t == ItsType::RS { "Nivel máximo S" } else { "Punto de reorden s" };
    let mut ch = Chart::new(Some(&format!("Costo según {}", if t == ItsType::RS { "S" } else { "s" })), xl, &format!("Costo por {u}")).series(Series::new("Costo del inventario", SeriesKind::Line, pts).tone(Tone::Highlight));
    ch = ch.vline(e.s, "elegido", Tone::Accent);
    let mut charts = vec![ch.block()];
    let mut sim = vec![];
    let mut data = json!({
        "reorder_point": tidy(e.s), "order_quantity": tidy(e.q), "order_up_to": tidy(big_s), "review_interval": tidy(e.r),
        "cycle_service_level": tidy(e.csl()), "fill_rate": tidy(e.fill(sys.d)), "expected_shortage_per_cycle": tidy(e.n),
        "average_inventory": tidy(e.inv), "related_cost": tidy(e.related()), "total_cost": tidy(e.total()),
        "protection_demand": {"mean": tidy(e.mu), "sd": tidy(e.sd)},
    });
    if o.simulation_time > 0 {
        let hz = o.simulation_time.min(2_000_000);
        if o.warmup >= hz {
            return invalid("el inicio de la recolección debe ser menor que el tiempo simulado");
        }
        let so = simulate(m, &sys, &dl, &ll, &e, hz, o.warmup, o.seed);
        let mut stb = Table::new(Some("Resultados de la simulación"), &["Concepto", "Simulado", "Modelo analítico"]).left(0);
        stb.row(cells![format!("Costo de ordenar por {u}"), tidy(so.setup), tidy(e.setup)]);
        if periodic {
            stb.row(cells![format!("Costo de revisión por {u}"), tidy(so.review), tidy(e.review)]);
        }
        stb.row(cells![format!("Costo de mantener por {u}"), tidy(so.holding), tidy(e.holding)]);
        stb.row(cells![format!("Costo de faltantes por {u}"), tidy(so.shortage), tidy(e.shortage)]);
        stb.row_tone(cells!["Subtotal", tidy(so.cost - so.material), tidy(e.related())], Tone::Total);
        stb.row(cells!["Inventario promedio", tidy(so.on_hand), tidy(e.inv)]);
        stb.row(cells!["Atraso promedio", tidy(so.backlog), Cell::Empty]);
        stb.row(cells!["Tasa de cumplimiento (%)", tidy(100.0 * so.fill), tidy(100.0 * e.fill(sys.d))]);
        stb.row(cells![format!("Pedidos por {u}"), tidy(so.orders), tidy(1.0 / e.cycle)]);
        let mut pc = Chart::new(Some("Inventario simulado"), &format!("Tiempo ({u})"), "Unidades")
            .series(Series::new("Inventario neto", SeriesKind::Line, so.profile.iter().map(|p| [p.0, p.1]).collect()).tone(Tone::Highlight))
            .series(Series::new("Posición del inventario", SeriesKind::Line, so.profile.iter().map(|p| [p.0, p.2]).collect()).dashed());
        if t != ItsType::RS {
            pc = pc.hline(e.s, "s", Tone::Accent);
        }
        pc = pc.hline(0.0, "0", Tone::Muted);
        sim.push(metrics(vec![metric_main(format!("Costo simulado del inventario por {u}"), tidy(so.cost - so.material)), metric("Tasa de cumplimiento simulada (%)", tidy(100.0 * so.fill)), metric("Períodos simulados", (hz - o.warmup) as usize)]));
        sim.push(stb.block());
        sim.push(pc.block());
        sim.push(text("La simulación toma la demanda de cada unidad de tiempo de su distribución y la reparte de forma pareja en 20 subpasos; la revisión continua mira el inventario en cada subpaso."));
        data["simulation"] = json!({"related_cost": tidy(so.cost - so.material), "fill_rate": tidy(so.fill), "average_inventory": tidy(so.on_hand), "orders_per_time": tidy(so.orders)});
    } else {
        charts.push(text("Para ver el inventario simulado, indica un tiempo de simulación en los parámetros."));
    }
    let mut st = vec![];
    if o.steps {
        let mut v = vec![step(
            "Demanda durante la protección",
            format!(
                "Con demanda media {} y varianza {} por {u}, y tiempo de entrega medio {} con varianza {}: media en {prot} = D·({}) = {}; desviación = √(t·σ²_D + D²·σ²_L) = {}.",
                fmt(tidy(sys.d)),
                fmt(tidy(sys.vd)),
                fmt(tidy(sys.lm)),
                fmt(tidy(sys.lv)),
                if periodic { "R + L" } else { "L" },
                fmt(tidy(e.mu)),
                fmt(tidy(e.sd))
            ),
            vec![],
        )];
        v.push(step(
            "Faltante esperado",
            format!(
                "Con z = (s − media)/σ = {}, el faltante esperado por ciclo es n(s) = σ·G(z) = {} y la probabilidad de faltante 1 − Φ(z) = {}.",
                fmt(tidy((e.s - sys.under - e.mu) / e.sd.max(1e-12))),
                fmt(tidy(e.n)),
                fmt(tidy(e.p_short))
            ),
            vec![],
        ));
        v.push(step(
            "Costo por unidad de tiempo",
            "K/T + revisión/R + h·(Q/2 + s − D·(L + R/2) + n(s)) + (p·n(s) + F·P(faltante))/T. En el óptimo, s y Q se ajustan alternadamente hasta que dejan de cambiar (Hadley-Whitin); en revisión periódica se busca además el mejor R.",
            vec![pol.block()],
        ));
        if o.mode == SysMode::Service {
            v.push(step("Nivel de servicio", format!("Para un nivel de servicio del ciclo de {} %, z = {} y s = media + z·σ = {}.", fmt(o.service_level), fmt(tidy(z_of(o.service_level))), fmt(tidy(e.s))), vec![]));
        }
        st.push(steps(v));
    }
    let head = match t {
        ItsType::SQ => format!("Pedir Q = {} cuando el inventario llegue a s = {}", fmt(tidy(e.q)), fmt(tidy(e.s))),
        ItsType::SS => format!("Pedir hasta S = {} cuando el inventario llegue a s = {}", fmt(tidy(big_s)), fmt(tidy(e.s))),
        ItsType::RS => format!("Cada R = {} {u}, pedir hasta S = {}", fmt(tidy(e.r)), fmt(tidy(big_s))),
        _ => format!("Cada R = {} {u}, si el inventario está en s = {} o menos, pedir hasta S = {}", fmt(tidy(e.r)), fmt(tidy(e.s)), fmt(tidy(big_s))),
    };
    Ok(Report::new("its", &m.name, if o.mode == SysMode::Optimal { Status::Optimal } else { Status::Solved }, format!("{head}; costo del inventario {} por {u}.", fmt(tidy(e.related()))))
        .section("solucion", "Solución", blocks)
        .section("graficos", "Gráficos", charts)
        .section("simulacion", "Simulación", sim)
        .section("pasos", "Paso a paso", st)
        .data(data))
}
