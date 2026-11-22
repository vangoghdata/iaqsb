//! EOQ determinístico y descuentos por cantidad.

use super::{golden, money, ItsModel, ItsOptions};
use iaqsb_core::*;
use serde_json::json;

/// Parámetros de costo de un modelo EOQ.
#[derive(Clone, Copy, Debug)]
pub struct Eoq {
    pub d: f64,
    pub k: f64,
    pub h: f64,
    /// Faltante por unidad y unidad de tiempo (∞ = sin faltantes).
    pub p1: f64,
    /// Faltante por unidad.
    pub p2: f64,
    /// 1 − D/P.
    pub rho: f64,
    pub c: f64,
}

#[derive(Clone, Copy, Debug)]
pub struct Plan {
    pub q: f64,
    pub b: f64,
    pub setup: f64,
    pub holding: f64,
    pub shortage: f64,
    pub material: f64,
}

impl Plan {
    pub fn related(&self) -> f64 {
        self.setup + self.holding + self.shortage
    }
    pub fn total(&self) -> f64 {
        self.related() + self.material
    }
}

impl Eoq {
    pub fn backorder(&self, q: f64) -> f64 {
        if !self.p1.is_finite() {
            return 0.0;
        }
        let den = self.h + self.p1;
        if den <= 0.0 {
            return q * self.rho;
        }
        (self.rho * (self.h * q - self.p2 * self.d) / den).clamp(0.0, q * self.rho)
    }
    pub fn plan_b(&self, q: f64, b: f64) -> Plan {
        let qr = q * self.rho;
        let (holding, shortage) = if qr > 0.0 {
            (self.h * (qr - b).powi(2) / (2.0 * qr), if self.p1.is_finite() { self.p1 * b * b / (2.0 * qr) } else { 0.0 })
        } else {
            (0.0, 0.0)
        };
        Plan { q, b, setup: self.k * self.d / q, holding, shortage: shortage + self.p2 * self.d * b / q, material: self.c * self.d }
    }
    pub fn plan(&self, q: f64) -> Plan {
        self.plan_b(q, self.backorder(q))
    }
    /// Q sin faltantes ni descuentos: √(2DK / (hρ)).
    pub fn basic(&self) -> f64 {
        (2.0 * self.d * self.k / (self.h * self.rho)).sqrt()
    }
    /// Cantidad óptima (fórmula cerrada si no hay costo de faltante por unidad).
    pub fn optimal(&self) -> f64 {
        if self.p2 == 0.0 {
            if self.p1.is_finite() {
                self.basic() * ((self.h + self.p1) / self.p1).sqrt()
            } else {
                self.basic()
            }
        } else {
            let q0 = self.basic();
            let f = |lq: f64| self.plan(lq.exp()).related();
            golden(&f, (q0 / 1e3).ln(), (q0 * 1e3).ln()).exp()
        }
    }
    pub fn optimal_in(&self, lo: f64, hi: f64) -> f64 {
        let q = self.optimal();
        if self.p2 == 0.0 {
            return q.clamp(lo, hi);
        }
        let f = |x: f64| self.plan(x).related();
        let hi2 = if hi.is_finite() { hi } else { lo.max(q) * 10.0 + 1.0 };
        golden(&f, lo.max(1e-12), hi2)
    }
}

fn base(m: &ItsModel) -> Result<Eoq, ModError> {
    if m.demand <= 0.0 {
        return invalid("la demanda debe ser positiva");
    }
    if m.setup_cost <= 0.0 {
        return invalid("el costo de ordenar debe ser positivo (con costo 0 convendría pedir de forma continua)");
    }
    if m.holding_cost <= 0.0 {
        return invalid("el costo de mantener debe ser positivo");
    }
    let p = m.production_rate.0;
    if p.is_finite() && p <= m.demand {
        return invalid(format!("la tasa de producción ({}) debe ser mayor que la demanda ({})", fmt(p), fmt(m.demand)));
    }
    let p1 = m.shortage_cost.0;
    if p1 < 0.0 || m.shortage_cost_fixed < 0.0 || m.lead_time < 0.0 || m.unit_cost < 0.0 {
        return invalid("los costos y el tiempo de entrega no pueden ser negativos");
    }
    if p1 == 0.0 && m.shortage_cost_fixed == 0.0 {
        return invalid("con costo de faltante 0 convendría no tener nunca inventario: indica un costo de faltante o usa M para no permitirlos");
    }
    Ok(Eoq { d: m.demand, k: m.setup_cost, h: m.holding_cost, p1, p2: if p1.is_finite() { m.shortage_cost_fixed } else { 0.0 }, rho: if p.is_finite() { 1.0 - m.demand / p } else { 1.0 }, c: m.unit_cost })
}

fn result_table(e: &Eoq, plans: &[(&str, Plan)], m: &ItsModel) -> Table {
    let u = &m.time_unit;
    let mut head = vec!["Concepto".to_string()];
    head.extend(plans.iter().map(|p| p.0.to_string()));
    let mut t = Table::new(Some("Análisis del pedido"), &head).left(0);
    let row = |t: &mut Table, label: String, f: &dyn Fn(&Plan) -> f64, tone: Tone| {
        let mut r = vec![Cell::from(label)];
        r.extend(plans.iter().map(|p| money(f(&p.1))));
        t.row_tone(r, tone);
    };
    row(&mut t, "Cantidad a pedir (Q)".into(), &|p| p.q, Tone::Highlight);
    row(&mut t, "Inventario máximo".into(), &|p| p.q * e.rho - p.b, Tone::Normal);
    row(&mut t, "Inventario promedio".into(), &|p| p.holding / e.h, Tone::Normal);
    if e.p1.is_finite() {
        row(&mut t, "Faltante máximo".into(), &|p| p.b, Tone::Normal);
    }
    row(&mut t, format!("Intervalo entre pedidos ({u})"), &|p| p.q / e.d, Tone::Normal);
    if let Some(dpy) = m.days_per_year {
        row(&mut t, "Intervalo entre pedidos (días)".into(), &|p| p.q / e.d * dpy, Tone::Normal);
    }
    if e.rho < 1.0 {
        let prate = e.d / (1.0 - e.rho);
        row(&mut t, format!("Tiempo produciendo Tp ({u})"), &|p| p.q / prate, Tone::Normal);
        if let Some(dpy) = m.days_per_year {
            row(&mut t, "Tiempo produciendo Tp (días)".into(), &|p| p.q / prate * dpy, Tone::Normal);
            row(&mut t, "Tiempo solo consumiendo Td (días)".into(), &|p| (p.q / e.d - p.q / prate) * dpy, Tone::Normal);
        }
    }
    row(&mut t, format!("Pedidos por {u}"), &|p| e.d / p.q, Tone::Normal);
    row(&mut t, "Punto de reorden".into(), &|p| e.d * m.lead_time - p.b, Tone::Normal);
    row(&mut t, format!("Costo de ordenar por {u}"), &|p| p.setup, Tone::Normal);
    row(&mut t, format!("Costo de mantener por {u}"), &|p| p.holding, Tone::Normal);
    if e.p1.is_finite() {
        row(&mut t, format!("Costo de faltantes por {u}"), &|p| p.shortage, Tone::Normal);
    }
    row(&mut t, "Subtotal (costos del inventario)".into(), &|p| p.related(), Tone::Total);
    if e.c > 0.0 {
        row(&mut t, format!("Costo de adquisición por {u}"), &|p| p.material, Tone::Normal);
        row(&mut t, format!("Costo total por {u}"), &|p| p.total(), Tone::Total);
    }
    t
}

fn cost_chart(e: &Eoq, qstar: f64, title: &str) -> Block {
    let pts = 120;
    let (lo, hi) = (qstar * 0.25, qstar * 3.0);
    let xs: Vec<f64> = (0..=pts).map(|i| lo + (hi - lo) * i as f64 / pts as f64).collect();
    let s = |name: &str, f: &dyn Fn(&Plan) -> f64, tone: Tone| Series::new(name, SeriesKind::Line, xs.iter().map(|q| [*q, f(&e.plan(*q))]).collect()).tone(tone);
    let mut ch = Chart::new(Some(title), "Cantidad a pedir (Q)", "Costo por unidad de tiempo")
        .series(s("Ordenar", &|p| p.setup, Tone::Normal))
        .series(s("Mantener + faltantes", &|p| p.holding + p.shortage, Tone::Normal))
        .series(s("Total", &|p| p.related(), Tone::Highlight));
    ch = ch.vline(qstar, format!("Q* = {}", fmt(tidy(qstar))), Tone::Accent);
    ch.y_zero = true;
    ch.block()
}

fn profile_chart(e: &Eoq, p: &Plan, unit: &str, dpy: Option<f64>) -> Block {
    let sc = dpy.unwrap_or(1.0);
    let t = p.q / e.d * sc;
    let imax = p.q * e.rho - p.b;
    let mut pts = vec![];
    for k in 0..3 {
        let t0 = k as f64 * t;
        if e.rho < 1.0 {
            let t1 = p.q / (e.d / (1.0 - e.rho)) * sc;
            pts.push([t0, -p.b]);
            pts.push([t0 + t1, imax]);
            pts.push([t0 + t, -p.b]);
        } else {
            pts.push([t0, -p.b]);
            pts.push([t0, imax]);
            pts.push([t0 + t, -p.b]);
        }
    }
    let mut ch = Chart::new(Some("Perfil del inventario (3 ciclos)"), &if dpy.is_some() { "Tiempo (días)".to_string() } else { format!("Tiempo ({unit})") }, "Inventario neto").series(Series::new("Inventario", SeriesKind::Line, pts).tone(Tone::Highlight));
    ch = ch.hline(0.0, "0", Tone::Muted);
    if p.b > 0.0 {
        ch = ch.hline(-p.b, format!("faltante máximo {}", fmt(tidy(p.b))), Tone::Accent);
    }
    ch.block()
}

pub fn eoq(m: &ItsModel, o: &ItsOptions) -> Result<Report, ModError> {
    let e = base(m)?;
    let q = e.optimal();
    let best = e.plan(q);
    let mut plans = vec![("Óptimo", best)];
    if let Some(kq) = m.known_quantity.filter(|x| *x > 0.0) {
        plans.push(("Cantidad conocida", e.plan(kq)));
    }
    let u = &m.time_unit;
    let t = result_table(&e, &plans, m);
    let mut blocks = vec![
        metrics({
            let mut v = vec![
                metric_main(if e.rho < 1.0 { "Lote económico de producción" } else { "Cantidad óptima a pedir" }, tidy(q)),
                metric(format!("Costo del inventario por {u}"), tidy(best.related())),
                metric("Punto de reorden", tidy(e.d * m.lead_time - best.b)),
                metric(format!("Pedidos por {u}"), tidy(e.d / q)),
            ];
            match m.days_per_year {
                Some(dpy) => {
                    v.push(metric("Demanda diaria (d)", tidy(e.d / dpy)));
                    v.push(metric("Intervalo entre pedidos (días)", tidy(q / e.d * dpy)));
                }
                None => v.push(metric(format!("Intervalo entre pedidos ({u})"), tidy(q / e.d))),
            }
            if e.rho < 1.0 {
                v.push(metric("Inventario máximo", tidy(q * e.rho - best.b)));
            }
            v
        }),
        t.clone().block(),
    ];
    if m.lead_time > q / e.d {
        blocks.push(text(format!(
            "El tiempo de entrega ({}) es mayor que el intervalo entre pedidos ({}): el punto de reorden se refiere a la posición del inventario (lo disponible más lo pedido).",
            fmt(m.lead_time),
            fmt(tidy(q / e.d))
        )));
    }
    let charts = vec![cost_chart(&e, q, "Costos según la cantidad pedida"), profile_chart(&e, &best, u, m.days_per_year)];
    let mut st = vec![];
    if o.steps {
        let mut v = vec![];
        let rho_txt = if e.rho < 1.0 { format!(" con ρ = 1 − D/P = {}", fmt(tidy(e.rho))) } else { String::new() };
        let basic = e.basic();
        v.push(step(
            "Datos",
            format!(
                "D = {} por {u}, K = {}, h = {}, faltante p = {}{}{}.",
                fmt(e.d),
                fmt(e.k),
                fmt(e.h),
                if e.p1.is_finite() { fmt(e.p1) } else { "M (no se permiten)".into() },
                if e.p2 > 0.0 { format!(", faltante por unidad π = {}", fmt(e.p2)) } else { String::new() },
                rho_txt
            ),
            vec![],
        ));
        if e.p2 == 0.0 {
            let mut txt = format!("Q₀ = √(2DK / (h·ρ)) = √(2·{}·{} / ({}·{})) = {}.", fmt(e.d), fmt(e.k), fmt(e.h), fmt(tidy(e.rho)), fmt(tidy(basic)));
            if e.p1.is_finite() {
                txt += &format!(
                    " Con faltantes: Q* = Q₀·√((h + p)/p) = {}·√(({} + {})/{}) = {}; faltante máximo B* = Q*·ρ·h/(h + p) = {}.",
                    fmt(tidy(basic)),
                    fmt(e.h),
                    fmt(e.p1),
                    fmt(e.p1),
                    fmt(tidy(q)),
                    fmt(tidy(best.b))
                );
            }
            v.push(step("Cantidad óptima", txt, vec![]));
        } else {
            v.push(step(
                "Cantidad óptima",
                format!(
                    "Con costo de faltante por unidad no hay fórmula cerrada simple. Para cada Q el faltante óptimo es B = ρ(hQ − πD)/(h + p) (entre 0 y Qρ), y se minimiza el costo total sobre Q: Q* = {}, B* = {}.",
                    fmt(tidy(q)),
                    fmt(tidy(best.b))
                ),
                vec![],
            ));
        }
        v.push(step(
            "Costos",
            format!(
                "Ordenar: K·D/Q = {}. Mantener: h·(Qρ − B)²/(2Qρ) = {} (sin faltantes es h·Imax/2, con Imax = Q en el EOQ y Imax = Q(1 − d/p) en el POQ).{} En el óptimo sin faltantes, ordenar y mantener cuestan lo mismo.",
                fmt(tidy(best.setup)),
                fmt(tidy(best.holding)),
                if e.p1.is_finite() { format!(" Faltantes: p·B²/(2Qρ) + π·D·B/Q = {}.", fmt(tidy(best.shortage))) } else { String::new() }
            ),
            vec![t.block()],
        ));
        let rtxt = match m.days_per_year {
            Some(dpy) if best.b == 0.0 => format!(
                "Demanda diaria d = D / días = {} / {} = {}. Se pide cuando el inventario baja a R = d·L = {}·{} = {} (Harris, 1913; sin inventario de seguridad porque la demanda es constante).",
                fmt(e.d),
                fmt(dpy),
                fmt(tidy(e.d / dpy)),
                fmt(tidy(e.d / dpy)),
                fmt(tidy(m.lead_time * dpy)),
                fmt(tidy(e.d * m.lead_time))
            ),
            _ => format!("Se pide cuando el inventario baja a D·L − B = {}·{} − {} = {}.", fmt(e.d), fmt(m.lead_time), fmt(tidy(best.b)), fmt(tidy(e.d * m.lead_time - best.b))),
        };
        v.push(step("Punto de reorden", rtxt, vec![]));
        st.push(steps(v));
    }
    Ok(Report::new("its", &m.name, Status::Optimal, match m.days_per_year {
            Some(dpy) => format!("{} {} unidades cada {} días; costo del inventario {} por {u}.", if e.rho < 1.0 { "Producir lotes de" } else { "Pedir" }, fmt(tidy(q)), fmt(tidy(q / e.d * dpy)), fmt(tidy(best.related()))),
            None => format!("{} {} unidades cada {} {u}; costo del inventario {} por {u}.", if e.rho < 1.0 { "Producir lotes de" } else { "Pedir" }, fmt(tidy(q)), fmt(tidy(q / e.d)), fmt(tidy(best.related()))),
        })
        .section("solucion", "Solución", blocks)
        .section("graficos", "Gráficos", charts)
        .section("pasos", "Paso a paso", st)
        .data(json!({
            "order_quantity": tidy(q),
            "max_backorder": tidy(best.b),
            "max_inventory": tidy(q * e.rho - best.b),
            "reorder_point": tidy(e.d * m.lead_time - best.b),
            "cycle_time": tidy(q / e.d),
            "related_cost": tidy(best.related()),
            "total_cost": tidy(best.total()),
        })))
}

struct Interval {
    lo: f64,
    hi: f64,
    disc: f64,
    e: Eoq,
}

pub fn discount(m: &ItsModel, o: &ItsOptions) -> Result<Report, ModError> {
    let e0 = base(m)?;
    if m.unit_cost <= 0.0 {
        return invalid("indica el costo unitario sin descuento");
    }
    let mut br: Vec<(f64, f64)> = m.breaks.iter().map(|b| (b.quantity, b.discount)).collect();
    br.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
    for w in br.windows(2) {
        if w[0].0 == w[1].0 {
            return invalid(format!("hay dos descuentos para la misma cantidad ({})", fmt(w[0].0)));
        }
    }
    if br.iter().any(|b| b.0 < 0.0 || b.1 < 0.0 || b.1 >= 100.0) {
        return invalid("las cantidades no pueden ser negativas y los descuentos deben estar entre 0 y 100 %");
    }
    if br.first().map(|b| b.0 > 0.0).unwrap_or(true) {
        br.insert(0, (0.0, 0.0));
    }
    let mut iv = vec![];
    let mut cum = 0.0; // costo de comprar hasta el inicio del intervalo (incremental)
    for (k, &(lo, d)) in br.iter().enumerate() {
        let hi = br.get(k + 1).map(|b| b.0).unwrap_or(f64::INFINITY);
        let f = 1.0 - d / 100.0;
        let ck = m.unit_cost * f;
        let mut e = e0;
        e.c = ck;
        if m.discount_holding {
            e.h *= f;
        }
        if m.discount_shortage && e.p1.is_finite() {
            e.p1 *= f;
        }
        if m.discount_shortage_fixed {
            e.p2 *= f;
        }
        if m.incremental {
            // C(Q) = A + ck·Q en el intervalo: A se suma al costo por pedido
            let a = cum - ck * lo;
            e.k += a;
            if hi.is_finite() {
                cum += ck * (hi - lo);
            }
        }
        iv.push(Interval { lo, hi, disc: d, e });
    }
    let interval_of = |q: f64| iv.iter().rposition(|x| q >= x.lo - 1e-12).unwrap_or(0);
    let mut rows = vec![];
    for x in &iv {
        let free = x.e.optimal();
        let lo = x.lo.max(1e-9);
        let hi = if x.hi.is_finite() { x.hi } else { f64::INFINITY };
        let q = if m.incremental { x.e.optimal_in(lo, hi) } else { x.e.optimal_in(lo, hi) };
        rows.push((free, q, x.e.plan(q)));
    }
    let bi = (0..rows.len()).min_by(|&a, &b| rows[a].2.total().partial_cmp(&rows[b].2.total()).unwrap()).unwrap();
    let best = rows[bi].2;
    let mut t = Table::new(Some("Análisis por intervalo de descuento"), &["Desde", "Hasta", "Descuento (%)", "Costo unitario", "Q sin restricción", "Q factible", "Costo total"]);
    for (k, x) in iv.iter().enumerate() {
        t.row_tone(
            cells![x.lo, if x.hi.is_finite() { Cell::Num(x.hi) } else { Cell::from("∞") }, x.disc, tidy(m.unit_cost * (1.0 - x.disc / 100.0)), tidy(rows[k].0), tidy(rows[k].1), tidy(rows[k].2.total())],
            if k == bi { Tone::Highlight } else { Tone::Normal },
        );
    }
    let mut plans = vec![("Óptimo", best)];
    if let Some(kq) = m.known_quantity.filter(|x| *x > 0.0) {
        plans.push(("Cantidad conocida", iv[interval_of(kq)].e.plan(kq)));
    }
    let rt = result_table(&iv[bi].e, &plans, m);
    let u = &m.time_unit;
    // curva del costo total (con saltos en los quiebres)
    let qmax = br.last().unwrap().0.max(best.q) * 1.6 + 1.0;
    let qmin = (rows.iter().map(|r| r.1).fold(f64::INFINITY, f64::min) * 0.3).max(qmax / 400.0);
    let mut pts = vec![];
    let n = 400;
    let mut prev = usize::MAX;
    for i in 0..=n {
        let q = qmin + (qmax - qmin) * i as f64 / n as f64;
        let k = interval_of(q);
        if prev != usize::MAX && k != prev && !m.incremental {
            pts.push([q, f64::NAN]);
        }
        prev = k;
        pts.push([q, iv[k].e.plan(q).total()]);
    }
    let mut ch = Chart::new(Some("Costo total según la cantidad pedida"), "Cantidad a pedir (Q)", &format!("Costo total por {u}")).series(Series::new("Costo total", SeriesKind::Line, pts).tone(Tone::Highlight));
    for x in iv.iter().skip(1) {
        ch = ch.vline(x.lo, format!("{} %", fmt(x.disc)), Tone::Muted);
    }
    ch = ch.vline(best.q, format!("Q* = {}", fmt(tidy(best.q))), Tone::Accent);
    let mut st = vec![];
    if o.steps {
        st.push(steps(vec![
            step(
                "Un EOQ por intervalo",
                if m.incremental {
                    "Con descuento incremental, el costo de comprar Q unidades en un intervalo es A + c·Q: la parte fija A se suma al costo de ordenar y se calcula el EOQ de cada intervalo."
                } else {
                    "Con descuento sobre todas las unidades, cada intervalo tiene su propio costo unitario (y, si se indicó, su propio costo de mantener): se calcula el EOQ de cada uno."
                },
                vec![],
            ),
            step(
                "Ajustar al intervalo",
                "Si el EOQ cae fuera de su intervalo, la mejor cantidad factible es el extremo más cercano (el costo es convexo en Q). Para el intervalo de mayor descuento suele ser la cantidad mínima que da derecho al descuento.",
                vec![t.clone().block()],
            ),
            step("Comparar costos totales", format!("Se elige el intervalo con menor costo total, que incluye la compra: Q* = {} con costo {} por {u}.", fmt(tidy(best.q)), fmt(tidy(best.total()))), vec![]),
        ]));
    }
    Ok(Report::new("its", &m.name, Status::Optimal, format!("Pedir {} unidades ({} % de descuento); costo total {} por {u}.", fmt(tidy(best.q)), fmt(iv[bi].disc), fmt(tidy(best.total()))))
        .section("solucion", "Solución", vec![metrics(vec![metric_main("Cantidad óptima a pedir", tidy(best.q)), metric(format!("Costo total por {u}"), tidy(best.total())), metric("Descuento aplicado (%)", iv[bi].disc)]), t.block(), rt.block()])
        .section("graficos", "Gráficos", vec![ch.block()])
        .section("pasos", "Paso a paso", st)
        .data(json!({"order_quantity": tidy(best.q), "total_cost": tidy(best.total()), "discount": iv[bi].disc, "candidates": rows.iter().map(|r| json!({"quantity": tidy(r.1), "total_cost": tidy(r.2.total())})).collect::<Vec<_>>()})))
}
