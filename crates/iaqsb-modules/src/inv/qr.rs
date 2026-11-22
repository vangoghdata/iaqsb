//! Simulación día a día de la política de revisión continua (Q, R), con la
//! estructura de tabla de la clase:
//!
//! - Bodega: inventario físico al inicio del día = bodega de ayer − lo vendido
//!   ayer + lo que llega hoy. Con pedidos pendientes puede ser negativa
//!   (inventario neto: lo que se le debe al cliente).
//! - Tránsito: pedidos colocados que todavía no llegan.
//! - Posición = bodega + tránsito. Si posición ≤ R se pide Q ese día.
//! - Un pedido colocado el día t llega al inicio del día t + L.
//! - Perdidas = máx(0, ventas − bodega); con pedidos pendientes, quedan
//!   pendientes y se atienden primero cuando llega el siguiente pedido.
//!
//! Q y R salen de la demanda diaria promedio d̄ y su desviación σ:
//! Q = k·d̄ (o un valor fijo) y R = d̄·L + Z·σ·√L.

use super::{mean_sd, values, z_value, InvModel, InvOptions, Scenario};
use iaqsb_core::*;
use serde_json::{json, Value};

#[derive(Clone, Debug)]
pub struct Day {
    pub t: usize,
    pub sales: f64,
    pub stock: f64,
    pub transit: f64,
    pub position: f64,
    pub order: f64,
    pub arrival: f64,
    pub short: f64,
}

#[derive(Clone, Debug)]
pub struct Sim {
    pub days: Vec<Day>,
    pub avg: f64,
    pub hold: f64,
    pub short: f64,
    pub short_cost: f64,
    pub total: f64,
    pub orders: usize,
    pub served_pct: f64,
}

pub fn simulate(f: &[f64], q: f64, r: f64, l: usize, back: bool, inv0: f64, h: f64, c_short: f64, c_back: f64) -> Sim {
    let n = f.len();
    let mut arr = vec![0.0; n + l + 2];
    let mut stock = inv0;
    let mut prev_out = 0.0;
    let mut days = vec![];
    let (mut sum_pos, mut short, mut orders) = (0.0, 0.0, 0);
    for t in 1..=n {
        if t > 1 {
            stock -= prev_out;
        }
        let ing = arr[t];
        stock += ing;
        arr[t] = 0.0;
        let transit: f64 = arr[t + 1..].iter().sum();
        let pos = stock + transit;
        let mut order = 0.0;
        if pos <= r + 1e-9 && q > 0.0 {
            order = q;
            arr[t + l] += q;
            orders += 1;
        }
        let dem = f[t - 1];
        let avail = stock.max(0.0);
        let miss = (dem - avail).max(0.0);
        short += miss;
        prev_out = if back { dem } else { dem.min(avail) };
        sum_pos += stock.max(0.0);
        days.push(Day { t, sales: dem, stock, transit, position: pos, order, arrival: ing, short: miss });
    }
    let avg = sum_pos / n as f64;
    let hold = avg * h;
    let short_cost = short * if back { c_back } else { c_short };
    let tot_dem: f64 = f.iter().sum();
    Sim { days, avg, hold, short, short_cost, total: hold + short_cost, orders, served_pct: if tot_dem > 0.0 { 100.0 * (tot_dem - short) / tot_dem } else { 100.0 } }
}

struct Params {
    name: String,
    ns: Option<f64>,
    l: usize,
    q: f64,
    r: f64,
    z: Option<f64>,
    back: bool,
    how_q: String,
    how_r: String,
}

fn params(s: &Scenario, k: usize, m: &InvModel, o: &InvOptions, d: f64, sd: f64) -> Result<Params, ModError> {
    let name = if s.name.trim().is_empty() { format!("Escenario {}", k + 1) } else { s.name.trim().to_string() };
    let lf = s.lead_time.or(m.lead_time).ok_or_else(|| ModError::Invalid(format!("{name}: falta el tiempo de entrega")))?;
    if lf < 0.0 || lf.fract() != 0.0 {
        return invalid(format!("{name}: el tiempo de entrega debe ser un número entero de días"));
    }
    let l = lf as usize;
    let rnd = |x: f64| if o.round { x.round() } else { x };
    let (q, how_q) = match (s.quantity, s.q_factor) {
        (Some(q), _) => (q, format!("Q = {} (dato)", fmt(tidy(q)))),
        (None, Some(k)) => (rnd(k * d), format!("Q = {}·d̄ = {}·{} = {} → {}", fmt(k), fmt(k), fmt(tidy(d)), fmt(tidy(k * d)), fmt(tidy(rnd(k * d))))),
        _ => return invalid(format!("{name}: falta Q (fijo o como múltiplo del promedio)")),
    };
    if q <= 0.0 {
        return invalid(format!("{name}: Q debe ser positivo"));
    }
    let ns = s.service_level.or(m.service_level);
    let (r, z, how_r) = match (s.reorder_point, ns) {
        (Some(r), _) => (r, None, format!("R = {} (dato)", fmt(tidy(r)))),
        (None, Some(ns)) => {
            let z = z_value(ns, &o.z_mode)?;
            let raw = d * lf + z * sd * lf.sqrt();
            (rnd(raw), Some(z), format!("Z({} %) = {}; R = d̄·L + Z·σ·√L = {}·{} + {}·{}·√{} = {} + {} = {} → {}", fmt(ns), fmt(tidy(z)), fmt(tidy(d)), fmt(lf), fmt(tidy(z)), fmt(tidy(sd)), fmt(lf), fmt(tidy(d * lf)), fmt(tidy(z * sd * lf.sqrt())), fmt(tidy(raw)), fmt(tidy(rnd(raw)))))
        }
        _ => return invalid(format!("{name}: falta el nivel de servicio (o un R fijo)")),
    };
    Ok(Params { name, ns, l, q, r, z, back: s.backorders.unwrap_or(m.backorders), how_q, how_r })
}

fn day_table(p: &Params, s: &Sim) -> Table {
    let last = if p.back { "Nuevo back order" } else { "Perdidas" };
    let mut t = Table::new(Some(&format!("{}: tabla día a día", p.name)), &["Día", "Ventas", "Bodega", "Tránsito", "Posición", "Orden", "Ingreso", last]);
    let dash = |x: f64| if x == 0.0 { Cell::from("—") } else { Cell::Num(tidy(x)) };
    for d in &s.days {
        let tone = if d.short > 0.0 { Tone::Bad } else if d.order > 0.0 { Tone::Highlight } else { Tone::Normal };
        t.row_tone(cells![d.t, tidy(d.sales), tidy(d.stock), tidy(d.transit), tidy(d.position), dash(d.order), dash(d.arrival), dash(d.short)], tone);
    }
    let sum = |f: &dyn Fn(&Day) -> f64| tidy(s.days.iter().map(f).sum());
    t.row_tone(cells!["Σ", sum(&|d| d.sales), sum(&|d| d.stock.max(0.0)), "", "", sum(&|d| d.order), sum(&|d| d.arrival), sum(&|d| d.short)], Tone::Total);
    t
}

fn chart(p: &Params, s: &Sim) -> Block {
    let mut pts = vec![];
    for d in &s.days {
        if d.arrival > 0.0 {
            pts.push([d.t as f64, d.stock - d.arrival]);
        }
        pts.push([d.t as f64, d.stock]);
    }
    let pos: Vec<[f64; 2]> = s.days.iter().map(|d| [d.t as f64, d.position]).collect();
    let mut ch = Chart::new(Some(&format!("{}: inventario en bodega (Q = {}, R = {})", p.name, fmt(tidy(p.q)), fmt(tidy(p.r)))), "Día", "Unidades")
        .series(Series::new("Inventario en bodega", SeriesKind::Line, pts).tone(Tone::Highlight))
        .series(Series::new("Posición (bodega + tránsito)", SeriesKind::Line, pos).dashed());
    ch = ch.hline(p.r, "R", Tone::Accent).hline(0.0, "0", Tone::Muted);
    for d in s.days.iter().filter(|d| d.order > 0.0) {
        ch = ch.vline(d.t as f64, format!("pedido día {}", d.t), Tone::Muted);
    }
    ch.block()
}

pub fn solve(m: &InvModel, o: &InvOptions) -> Result<Report, ModError> {
    let f = values(&m.forecast);
    if f.is_empty() {
        return invalid("falta el pronóstico de ventas");
    }
    if f.iter().any(|x| *x < 0.0) {
        return invalid("las ventas del pronóstico no pueden ser negativas");
    }
    let hist = values(&m.history);
    let (hm, hs) = if hist.len() >= 2 { mean_sd(&hist) } else { (f64::NAN, f64::NAN) };
    let d = m.mean_demand.or((!hm.is_nan()).then_some(hm)).ok_or_else(|| ModError::Invalid("falta la demanda promedio diaria (o el histórico de ventas)".into()))?;
    let sd = m.std_demand.or((!hs.is_nan()).then_some(hs)).ok_or_else(|| ModError::Invalid("falta la desviación estándar diaria (o el histórico de ventas)".into()))?;
    if m.scenarios.is_empty() {
        return invalid("agrega al menos un escenario");
    }
    let ps: Vec<Params> = m.scenarios.iter().enumerate().map(|(k, s)| params(s, k, m, o, d, sd)).collect::<Result<_, _>>()?;
    let sims: Vec<Sim> = ps.iter().map(|p| simulate(&f, p.q, p.r, p.l, p.back, m.initial_inventory, m.unit_holding_cost, m.shortage_cost, m.backorder_cost)).collect();
    let base = sims[0].total;
    let mut sum_t = Table::new(
        Some("Resumen de los escenarios"),
        &["Escenario", "NS (%)", "L", "Q", "R", "Inv. promedio", "C. almacenar", "Faltantes", "C. inexistencia", "Costo total", "Atendido el mismo día (%)", "Pedidos", "vs. el primero (%)"],
    )
    .left(0);
    // con back orders sin costo el total no es comparable: solo compiten si todos son así
    let comparable: Vec<usize> = (0..sims.len()).filter(|&k| !(ps[k].back && m.backorder_cost == 0.0)).collect();
    let pool = if comparable.is_empty() { (0..sims.len()).collect::<Vec<_>>() } else { comparable };
    let best = *pool.iter().min_by(|&&a, &&b| sims[a].total.partial_cmp(&sims[b].total).unwrap()).unwrap();
    for (p, s) in ps.iter().zip(&sims) {
        let is_best = std::ptr::eq(s, &sims[best]);
        sum_t.row_tone(
            cells![
                format!("{}{}", p.name, if p.back && !p.name.to_lowercase().contains("back") { " (back orders)" } else { "" }),
                p.ns.map(Cell::Num).unwrap_or(Cell::from("—")),
                p.l,
                tidy(p.q),
                tidy(p.r),
                tidy2(s.avg),
                tidy2(s.hold),
                tidy(s.short),
                tidy2(s.short_cost),
                tidy2(s.total),
                tidy1(s.served_pct),
                s.orders,
                if base > 0.0 { Cell::Num(tidy1(100.0 * (s.total - base) / base)) } else { Cell::from("—") }
            ],
            if is_best { Tone::Highlight } else { Tone::Normal },
        );
    }
    let cats: Vec<String> = ps.iter().map(|p| p.name.clone()).collect();
    let mut cost_ch = Chart::new(Some("Costo total por escenario"), "Escenario", "Costo").stacked();
    cost_ch.categories = Some(cats.clone());
    cost_ch = cost_ch
        .series(Series::new("Almacenar", SeriesKind::Bar, sims.iter().enumerate().map(|(k, s)| [k as f64, tidy2(s.hold)]).collect()))
        .series(Series::new("Inexistencia / pendientes", SeriesKind::Bar, sims.iter().enumerate().map(|(k, s)| [k as f64, tidy2(s.short_cost)]).collect()).tone(Tone::Bad));
    // escenarios uno por uno
    let mut sc_steps = vec![];
    for (p, s) in ps.iter().zip(&sims) {
        let ms = metrics(vec![
            metric_main("Costo total", tidy2(s.total)),
            metric("Inventario promedio", tidy2(s.avg)),
            metric("Costo de almacenar", tidy2(s.hold)),
            metric(if p.back { "Unidades en back order" } else { "Unidades perdidas" }, tidy(s.short)),
            metric(if p.back { "Costo de pendientes" } else { "Costo de inexistencia" }, tidy2(s.short_cost)),
            metric("Demanda atendida el mismo día (%)", tidy1(s.served_pct)),
        ]);
        let desc = format!(
            "{}. {}. L = {} días.{} Inventario promedio = Σ bodega / {} = {}{}; costo de almacenar = {} × {} = {}; {} = {} × {} = {}.",
            p.how_q,
            p.how_r,
            p.l,
            if p.back { " Se aceptan back orders: la bodega puede quedar negativa y la posición la usa, así que se pide antes; cuando llega un pedido se paga primero lo pendiente." } else { " No se aceptan back orders: lo que no hay se pierde." },
            s.days.len(),
            fmt(tidy2(s.avg)),
            if p.back { " (solo los valores positivos)" } else { "" },
            fmt(tidy2(s.avg)),
            fmt(m.unit_holding_cost),
            fmt(tidy2(s.hold)),
            if p.back { "costo de pendientes" } else { "costo de inexistencia" },
            fmt(tidy(s.short)),
            fmt(if p.back { m.backorder_cost } else { m.shortage_cost }),
            fmt(tidy2(s.short_cost))
        );
        sc_steps.push(step(p.name.clone(), desc, vec![ms, chart(p, s), day_table(p, s).block()]));
    }
    // búsqueda de la mejor combinación
    let mut search_blocks = vec![];
    let mut best_found = Value::Null;
    if o.search {
        let back = m.backorders;
        let lmin = o.min_lead.max(0.0).ceil() as usize;
        let lmax = ps.iter().map(|p| p.l).max().unwrap_or(5).max(lmin);
        let levels = [50.0, 60.0, 70.0, 75.0, 80.0, 85.0, 90.0, 95.0, 97.5, 98.0, 99.0, 99.5, 99.9];
        let qmax = (12.0 * d).max(ps.iter().map(|p| p.q).fold(0.0, f64::max) * 1.5);
        let step_q = (d / 20.0).max(1.0).round();
        let mut res: Vec<(f64, f64, usize, f64, f64, f64, f64)> = vec![];
        for l in lmin..=lmax {
            for ns in levels {
                let z = z_value(ns, &o.z_mode)?;
                let r = {
                    let x = d * l as f64 + z * sd * (l as f64).sqrt();
                    if o.round {
                        x.round()
                    } else {
                        x
                    }
                };
                let mut q = step_q;
                while q <= qmax {
                    let s = simulate(&f, q, r, l, back, m.initial_inventory, m.unit_holding_cost, m.shortage_cost, m.backorder_cost);
                    res.push((s.total, ns, l, q, r, s.short, s.avg));
                    q += step_q;
                }
            }
        }
        res.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap().then(a.3.partial_cmp(&b.3).unwrap()));
        let mut bt = Table::new(Some("Mejores combinaciones encontradas"), &["#", "NS (%)", "L", "Q", "R", "Inv. promedio", "Faltantes", "Costo total"]);
        let mut seen = vec![];
        for x in res.iter() {
            if seen.iter().any(|y: &(f64, f64, usize)| (y.0 - x.3).abs() < 1e-9 && (y.1 - x.4).abs() < 1e-9 && y.2 == x.2) {
                continue;
            }
            seen.push((x.3, x.4, x.2));
            bt.row_tone(cells![seen.len(), x.1, x.2, tidy(x.3), tidy(x.4), tidy2(x.6), tidy(x.5), tidy2(x.0)], if seen.len() == 1 { Tone::Highlight } else { Tone::Normal });
            if seen.len() >= 8 {
                break;
            }
        }
        if let Some(b) = res.first() {
            best_found = json!({"service_level": b.1, "lead_time": b.2, "quantity": tidy(b.3), "reorder_point": tidy(b.4), "total_cost": tidy2(b.0)});
            search_blocks.push(text(format!(
                "Se probaron {} combinaciones: tiempos de entrega de {} a {} días, 13 niveles de servicio y Q de {} en {} hasta {}. La mejor: NS {} %, L = {}, Q = {}, R = {} → costo total {}{}. Ojo: está ajustada a este pronóstico; con otra demanda puede no ser la mejor.",
                res.len(),
                lmin,
                lmax,
                fmt(step_q),
                fmt(step_q),
                fmt(tidy(qmax)),
                fmt(b.1),
                b.2,
                fmt(tidy(b.3)),
                fmt(tidy(b.4)),
                fmt(tidy2(b.0)),
                if base > 0.0 { format!(" ({} % frente al primer escenario)", fmt(tidy1(100.0 * (b.0 - base) / base))) } else { String::new() }
            )));
            search_blocks.push(bt.block());
            // costo según Q para el mejor L y NS
            let pts: Vec<[f64; 2]> = res.iter().filter(|x| x.1 == b.1 && x.2 == b.2).map(|x| [x.3, tidy2(x.0)]).collect::<Vec<_>>();
            let mut pts = pts;
            pts.sort_by(|a, b| a[0].partial_cmp(&b[0]).unwrap());
            let mut ch = Chart::new(Some(&format!("Costo total según Q (NS {} %, L = {})", fmt(b.1), b.2)), "Q", "Costo total").series(Series::new("Costo total", SeriesKind::Line, pts).tone(Tone::Highlight));
            ch = ch.vline(b.3, format!("Q = {}", fmt(tidy(b.3))), Tone::Accent);
            ch.y_zero = true;
            search_blocks.push(ch.block());
        }
    }
    let p0 = &ps[0];
    let s0 = &sims[0];
    let tot_f: f64 = f.iter().sum();
    let bo_note = ps.iter().any(|p| p.back) && m.backorder_cost == 0.0;
    let mut sol = vec![
        metrics(vec![
            metric_main(&format!("Costo total ({})", p0.name), tidy2(s0.total)),
            metric("Q", tidy(p0.q)),
            metric("R", tidy(p0.r)),
            metric("Demanda promedio d̄", tidy(d)),
            metric("Desviación σ", tidy(sd)),
            metric("Mejor escenario", tidy2(sims[best].total)),
        ]),
        text(format!(
            "Parámetros: d̄ = {}, σ = {}{}. El pronóstico suma {} en {} días (promedio {} por día, {} % {} que el histórico).",
            fmt(tidy(d)),
            fmt(tidy(sd)),
            if hist.len() >= 2 && m.mean_demand.is_none() { format!(" (con los {} datos del histórico)", hist.len()) } else { String::new() },
            fmt(tidy(tot_f)),
            f.len(),
            fmt(tidy(tot_f / f.len() as f64)),
            fmt(tidy1(((tot_f / f.len() as f64) / d - 1.0).abs() * 100.0)),
            if tot_f / f.len() as f64 >= d { "más" } else { "menos" }
        )),
        sum_t.clone().block(),
        cost_ch.block(),
    ];
    if bo_note {
        sol.push(text("Con back orders no se dio un costo por unidad pendiente: su costo total solo incluye almacenar, así que no se compara con los escenarios de ventas perdidas. El ahorro es real en ventas, pero no incluye el costo de hacer esperar al cliente."));
    }
    if let Some(z) = p0.z {
        let _ = z;
    }
    sol.insert(2, chart(p0, s0));
    let mut st = vec![];
    if o.steps {
        st.push(steps(vec![
            step("Parámetros de control", format!("{}. {}.", p0.how_q, p0.how_r), vec![]),
            step(
                "Reglas de la tabla",
                "Bodega = bodega de ayer − lo vendido ayer + lo que llega hoy. Tránsito = pedidos colocados que no han llegado. Posición = bodega + tránsito; si posición ≤ R se pide Q. El pedido del día t llega al inicio del día t + L. Perdidas = máx(0, ventas − bodega).",
                vec![],
            ),
            step("Tabla del primer escenario", "Las filas en rojo tienen faltantes y las resaltadas son días en que se coloca un pedido.", vec![day_table(p0, s0).block()]),
            step("Costos", "Inventario promedio = Σ bodega / días (con back orders solo los valores positivos); costo de almacenar = inventario promedio × costo por unidad; costo de inexistencia = unidades no vendidas × costo por unidad.", vec![sum_t.block()]),
        ]));
    }
    let head = format!(
        "{}: Q = {}, R = {} → costo total {}. Escenario de menor costo: {} ({}).",
        p0.name,
        fmt(tidy(p0.q)),
        fmt(tidy(p0.r)),
        fmt(tidy2(s0.total)),
        ps[best].name,
        fmt(tidy2(sims[best].total))
    );
    let data = json!({
        "mean_demand": tidy(d),
        "std_demand": tidy(sd),
        "scenarios": ps.iter().zip(&sims).map(|(p, s)| json!({
            "name": p.name, "service_level": p.ns, "lead_time": p.l, "quantity": tidy(p.q), "reorder_point": tidy(p.r), "z": p.z.map(tidy),
            "backorders": p.back, "average_inventory": tidy2(s.avg), "holding_cost": tidy2(s.hold), "shortage_units": tidy(s.short),
            "shortage_cost": tidy2(s.short_cost), "total_cost": tidy2(s.total), "orders": s.orders, "served_pct": tidy1(s.served_pct),
            "stock": s.days.iter().map(|d| tidy(d.stock)).collect::<Vec<_>>(),
        })).collect::<Vec<_>>(),
        "best_search": best_found,
    });
    let mut rep = Report::new("inv", &m.name, Status::Solved, head)
        .section("solucion", "Solución", sol)
        .section("escenarios", "Escenarios", vec![Block::Steps { title: Some("Escenarios".into()), steps: sc_steps }])
        .section("busqueda", "Mejor política", search_blocks)
        .section("pasos", "Paso a paso", st)
        .data(data)
        .cite("hadleywhitin1963", "Sistema de revisión continua (Q, R) con demanda aleatoria")
        .cite("silver2017", "Punto de reorden R = d̄·L + Z·σ·√L y simulación de la política con ventas perdidas o pendientes")
        .cite("sarache2026inventarios", "Modelo con demanda variable, sistema continuo (diapositivas 44–45)")
        .cite("sarache2026taller2", "Estructura de la tabla día a día, escenarios y costos del taller de inventarios")
        .cite("statrs", "Valor Z (inversa de la distribución normal estándar)");
    if o.search {
        rep = rep.cite("silver2017", "Búsqueda de la combinación (Q, R) de menor costo por simulación");
    }
    Ok(rep)
}

fn tidy2(x: f64) -> f64 {
    (x * 100.0).round() / 100.0
}
fn tidy1(x: f64) -> f64 {
    (x * 10.0).round() / 10.0
}
