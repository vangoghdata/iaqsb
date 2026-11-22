//! Tamaño de lote con demanda dinámica: diez métodos; Wagner-Whitin da el óptimo
//! (con atrasos si su costo es finito).

use super::{money, ItsModel, ItsOptions, LotMethod};
use iaqsb_core::*;
use serde_json::json;

struct Data {
    n: usize,
    d: Vec<f64>,
    k: Vec<f64>,
    c: Vec<f64>,
    h: Vec<f64>,
    b: Vec<f64>,
}

#[derive(Clone)]
pub struct Schedule {
    pub q: Vec<f64>,
    pub inv: Vec<f64>,
    pub setup: f64,
    pub variable: f64,
    pub holding: f64,
    pub backorder: f64,
    pub feasible: bool,
}

impl Schedule {
    pub fn total(&self) -> f64 {
        self.setup + self.variable + self.holding + self.backorder
    }
    pub fn orders(&self) -> usize {
        self.q.iter().filter(|x| **x > 1e-9).count()
    }
}

fn evaluate(dt: &Data, q: Vec<f64>) -> Schedule {
    let mut inv = 0.0;
    let mut s = Schedule { inv: vec![], setup: 0.0, variable: 0.0, holding: 0.0, backorder: 0.0, feasible: true, q: q.clone() };
    for t in 0..dt.n {
        inv += q[t] - dt.d[t];
        if q[t] > 1e-9 {
            s.setup += dt.k[t];
            s.variable += dt.c[t] * q[t];
        }
        if inv >= 0.0 {
            s.holding += dt.h[t] * inv;
        } else if dt.b[t].is_finite() {
            s.backorder += dt.b[t] * -inv;
        } else if inv < -1e-9 {
            s.feasible = false;
        }
        s.inv.push(inv);
    }
    s
}

/// Costo de mantener si se pide en t lo de los períodos t..t+len.
fn carry(dt: &Data, t: usize, len: usize) -> f64 {
    let mut acc = 0.0;
    let mut hsum = 0.0;
    for j in t..(t + len).min(dt.n) {
        acc += dt.d[j] * hsum;
        hsum += dt.h[j];
    }
    acc
}

fn block_rule(dt: &Data, choose: &dyn Fn(usize) -> usize) -> Vec<f64> {
    let mut q = vec![0.0; dt.n];
    let mut t = 0;
    while t < dt.n {
        if dt.d[t] <= 0.0 {
            t += 1;
            continue;
        }
        let len = choose(t).max(1).min(dt.n - t);
        q[t] = dt.d[t..t + len].iter().sum();
        t += len;
    }
    q
}

fn heuristic(dt: &Data, method: LotMethod, foq: f64, foi: usize) -> Vec<f64> {
    let maxlen = |t: usize| dt.n - t;
    match method {
        LotMethod::LotForLot => dt.d.clone(),
        LotMethod::Foq | LotMethod::Eoq => {
            let mut q = vec![0.0; dt.n];
            let mut inv = 0.0;
            for t in 0..dt.n {
                if inv < dt.d[t] - 1e-9 {
                    let need = dt.d[t] - inv;
                    let lots = (need / foq - 1e-9).ceil().max(1.0);
                    q[t] = lots * foq;
                    inv += q[t];
                }
                inv -= dt.d[t];
            }
            q
        }
        LotMethod::Foi | LotMethod::Poq => block_rule(dt, &|_| foi),
        LotMethod::Luc => block_rule(dt, &|t| {
            let (mut best, mut bl) = (f64::INFINITY, 1);
            let mut units = 0.0;
            for len in 1..=maxlen(t) {
                units += dt.d[t + len - 1];
                let u = (dt.k[t] + carry(dt, t, len)) / units;
                if u < best - 1e-12 {
                    best = u;
                    bl = len;
                } else if dt.d[t + len - 1] > 0.0 {
                    break;
                }
            }
            bl
        }),
        LotMethod::Ltc => block_rule(dt, &|t| {
            let (mut best, mut bl) = (f64::INFINITY, 1);
            for len in 1..=maxlen(t) {
                let gap = (carry(dt, t, len) - dt.k[t]).abs();
                if gap < best - 1e-12 {
                    best = gap;
                    bl = len;
                }
                if carry(dt, t, len) > dt.k[t] {
                    break;
                }
            }
            bl
        }),
        LotMethod::Ppb => block_rule(dt, &|t| {
            let mut bl = 1;
            for len in 1..=maxlen(t) {
                if carry(dt, t, len) <= dt.k[t] + 1e-12 {
                    bl = len;
                } else {
                    break;
                }
            }
            bl
        }),
        LotMethod::SilverMeal => block_rule(dt, &|t| {
            let (mut best, mut bl) = (f64::INFINITY, 1);
            for len in 1..=maxlen(t) {
                let a = (dt.k[t] + carry(dt, t, len)) / len as f64;
                if a < best - 1e-12 {
                    best = a;
                    bl = len;
                } else {
                    break;
                }
            }
            bl
        }),
        LotMethod::WagnerWhitin => unreachable!(),
    }
}

/// Wagner-Whitin con atrasos: f(k) = min f(i) + costo de producir en j (i ≤ j < k)
/// la demanda de los períodos i..k. Devuelve el plan y la tabla f.
fn wagner_whitin(dt: &Data) -> (Vec<f64>, Vec<(f64, usize, usize)>) {
    let n = dt.n;
    let mut pd = vec![0.0; n + 1];
    let mut ph = vec![0.0; n + 1];
    let mut pdh = vec![0.0; n + 1];
    let mut pb = vec![0.0; n + 1];
    let mut pdb = vec![0.0; n + 1];
    let mut last_inf = vec![-1i64; n + 1];
    for t in 0..n {
        pd[t + 1] = pd[t] + dt.d[t];
        ph[t + 1] = ph[t] + dt.h[t];
        pdh[t + 1] = pdh[t] + dt.d[t] * ph[t];
        let b = if dt.b[t].is_finite() { dt.b[t] } else { 0.0 };
        pb[t + 1] = pb[t] + b;
        pdb[t + 1] = pdb[t] + dt.d[t] * pb[t];
        last_inf[t + 1] = if dt.b[t].is_finite() { last_inf[t] } else { t as i64 };
    }
    // f[k]: costo mínimo de cubrir 0..k; (costo, i, j)
    let mut f = vec![(f64::INFINITY, 0usize, usize::MAX); n + 1];
    f[0] = (0.0, 0, usize::MAX);
    for k in 1..=n {
        // sin pedir si el período no tiene demanda
        if dt.d[k - 1] <= 0.0 && f[k - 1].0 < f[k].0 {
            f[k] = (f[k - 1].0, k - 1, usize::MAX);
        }
        for i in 0..k {
            if !f[i].0.is_finite() || pd[k] - pd[i] <= 0.0 {
                continue;
            }
            for j in i..k {
                // atrasos de i..j: no puede haber un período con atraso prohibido en [i, j)
                if j > i && last_inf[j] >= i as i64 {
                    continue;
                }
                let later = (pdh[k] - pdh[j]) - ph[j] * (pd[k] - pd[j]);
                let earlier = pb[j] * (pd[j] - pd[i]) - (pdb[j] - pdb[i]);
                let cost = f[i].0 + dt.k[j] + dt.c[j] * (pd[k] - pd[i]) + later + earlier;
                if cost < f[k].0 - 1e-9 {
                    f[k] = (cost, i, j);
                }
            }
        }
    }
    let mut q = vec![0.0; n];
    let mut k = n;
    while k > 0 {
        let (_, i, j) = f[k];
        if j != usize::MAX {
            q[j] += pd[k] - pd[i];
        }
        k = i;
    }
    (q, f)
}

fn method_label(m: LotMethod) -> &'static str {
    match m {
        LotMethod::WagnerWhitin => "Wagner-Whitin (óptimo)",
        LotMethod::LotForLot => "Lote a lote",
        LotMethod::Foq => "Cantidad fija (FOQ)",
        LotMethod::Eoq => "EOQ",
        LotMethod::Foi => "Intervalo fijo (FOI)",
        LotMethod::Poq => "Cantidad periódica (POQ)",
        LotMethod::Luc => "Mínimo costo unitario (LUC)",
        LotMethod::Ltc => "Mínimo costo total (LTC)",
        LotMethod::Ppb => "Balanceo de períodos (PPB)",
        LotMethod::SilverMeal => "Silver-Meal",
    }
}

fn method_rule(m: LotMethod) -> &'static str {
    match m {
        LotMethod::WagnerWhitin => "Programación dinámica: f(k) = mín sobre i ≤ j < k de f(i) + costo de producir en j la demanda de los períodos i a k − 1 (antes de j quedan atrasados, después se mantienen).",
        LotMethod::LotForLot => "Se produce en cada período exactamente su demanda.",
        LotMethod::Foq => "Cuando el inventario no alcanza, se pide la cantidad fija (o sus múltiplos).",
        LotMethod::Eoq => "Como la cantidad fija, con Q = EOQ calculado con la demanda y los costos promedio.",
        LotMethod::Foi => "Cada pedido cubre la demanda de un número fijo de períodos.",
        LotMethod::Poq => "Cada pedido cubre EOQ / demanda promedio períodos (redondeado).",
        LotMethod::Luc => "El pedido cubre los períodos que minimizan el costo por unidad (preparación + mantener) / unidades.",
        LotMethod::Ltc => "El pedido cubre los períodos con los que el costo de mantener queda más cerca del costo de preparación.",
        LotMethod::Ppb => "El pedido cubre todos los períodos posibles sin que el costo de mantener supere al de preparación (períodos-parte económicos).",
        LotMethod::SilverMeal => "El pedido cubre períodos mientras baje el costo promedio por período (preparación + mantener) / períodos.",
    }
}

pub fn solve(m: &ItsModel, o: &ItsOptions) -> Result<Report, ModError> {
    let n = m.periods.len();
    if n == 0 {
        return invalid("hace falta al menos un período");
    }
    if n > 20_000 {
        return invalid("como máximo 20 000 períodos");
    }
    for (t, p) in m.periods.iter().enumerate() {
        if [p.demand, p.setup_cost, p.unit_cost, p.holding_cost].iter().any(|x| *x < 0.0 || !x.is_finite()) || p.backorder_cost.0 < 0.0 {
            return invalid(format!("el período {} tiene valores negativos", if p.name.is_empty() { (t + 1).to_string() } else { p.name.clone() }));
        }
    }
    let dt = Data {
        n,
        d: m.periods.iter().map(|p| p.demand).collect(),
        k: m.periods.iter().map(|p| p.setup_cost).collect(),
        c: m.periods.iter().map(|p| p.unit_cost).collect(),
        h: m.periods.iter().map(|p| p.holding_cost).collect(),
        b: m.periods.iter().map(|p| p.backorder_cost.0).collect(),
    };
    let names: Vec<String> = m.periods.iter().enumerate().map(|(t, p)| if p.name.trim().is_empty() { (t + 1).to_string() } else { p.name.clone() }).collect();
    let total_d: f64 = dt.d.iter().sum();
    if total_d <= 0.0 {
        return invalid("no hay demanda en ningún período");
    }
    let avg = |v: &[f64]| v.iter().sum::<f64>() / n as f64;
    let (ad, ak, ah) = (total_d / n as f64, avg(&dt.k), avg(&dt.h));
    let eoq = if ah > 0.0 { (2.0 * ad * ak / ah).sqrt() } else { total_d };
    let eoq = if eoq > 0.0 { eoq } else { ad };
    let poq = ((eoq / ad).round() as usize).max(1);
    let foq = o.fixed_quantity.filter(|x| *x > 0.0).unwrap_or(eoq);
    let foi = o.fixed_interval.map(|x| x as usize).filter(|x| *x > 0).unwrap_or(poq);
    let (ww_q, ftab) = wagner_whitin(&dt);
    let all = [
        LotMethod::WagnerWhitin,
        LotMethod::LotForLot,
        LotMethod::Foq,
        LotMethod::Eoq,
        LotMethod::Foi,
        LotMethod::Poq,
        LotMethod::Luc,
        LotMethod::Ltc,
        LotMethod::Ppb,
        LotMethod::SilverMeal,
    ];
    let scheds: Vec<(LotMethod, Schedule)> = all
        .iter()
        .map(|&mt| {
            let q = match mt {
                LotMethod::WagnerWhitin => ww_q.clone(),
                LotMethod::Foq => heuristic(&dt, mt, foq, foi),
                LotMethod::Eoq => heuristic(&dt, mt, eoq, foi),
                LotMethod::Foi => heuristic(&dt, mt, foq, foi),
                LotMethod::Poq => heuristic(&dt, mt, foq, poq),
                _ => heuristic(&dt, mt, foq, foi),
            };
            (mt, evaluate(&dt, q))
        })
        .collect();
    let opt = scheds[0].1.total();
    let sel = scheds.iter().find(|s| s.0 == o.method).unwrap().1.clone();
    let mut pt = Table::new(
        Some(&format!("Plan: {}", method_label(o.method))),
        &["Período", "Demanda", "Pedido", "Inventario final", "Preparación", "Costo variable", "Mantener", "Atraso", "Costo total"],
    )
    .left(0);
    for t in 0..n {
        let inv = sel.inv[t];
        let hold = if inv > 0.0 { dt.h[t] * inv } else { 0.0 };
        let back = if inv < 0.0 && dt.b[t].is_finite() { dt.b[t] * -inv } else { 0.0 };
        let setup = if sel.q[t] > 1e-9 { dt.k[t] } else { 0.0 };
        let var = dt.c[t] * sel.q[t];
        pt.row_tone(
            vec![Cell::from(&names[t]), Cell::Num(dt.d[t]), money(sel.q[t]), money(inv), money(setup), money(var), money(hold), money(back), money(setup + var + hold + back)],
            if sel.q[t] > 1e-9 { Tone::Accent } else { Tone::Normal },
        );
    }
    pt.row_tone(
        vec![Cell::from("Total"), Cell::Num(total_d), money(sel.q.iter().sum()), Cell::Empty, money(sel.setup), money(sel.variable), money(sel.holding), money(sel.backorder), money(sel.total())],
        Tone::Total,
    );
    let mut ct = Table::new(Some("Comparación de los diez métodos"), &["Método", "Pedidos", "Costo total", "Sobre el óptimo (%)"]).left(0);
    for (mt, s) in &scheds {
        let tone = if *mt == o.method { Tone::Highlight } else if !s.feasible { Tone::Muted } else { Tone::Normal };
        ct.row_tone(cells![method_label(*mt), s.orders(), tidy(s.total()), if opt > 0.0 { Cell::Num(tidy(100.0 * (s.total() - opt) / opt)) } else { Cell::Empty }], tone);
    }
    let cats: Vec<String> = names.clone();
    let mut ch = Chart::new(Some("Demanda, pedidos e inventario"), "Período", "Unidades");
    ch.categories = Some(cats);
    ch = ch
        .series(Series::new("Demanda", SeriesKind::Bar, (0..n).map(|t| [t as f64, dt.d[t]]).collect()))
        .series(Series::new("Pedido", SeriesKind::Bar, (0..n).map(|t| [t as f64, sel.q[t]]).collect()).tone(Tone::Highlight))
        .series(Series::new("Inventario final", SeriesKind::Line, (0..n).map(|t| [t as f64, sel.inv[t]]).collect()).tone(Tone::Accent));
    let mut st = vec![];
    if o.steps {
        let mut v = vec![step(method_label(o.method), method_rule(o.method), vec![])];
        if matches!(o.method, LotMethod::Eoq | LotMethod::Poq) {
            v.push(step(
                "EOQ con promedios",
                format!("Demanda promedio {}, preparación promedio {}, mantener promedio {}: EOQ = √(2·{}·{}/{}) = {}{}.", fmt(tidy(ad)), fmt(tidy(ak)), fmt(tidy(ah)), fmt(tidy(ad)), fmt(tidy(ak)), fmt(tidy(ah)), fmt(tidy(eoq)), if o.method == LotMethod::Poq { format!("; cada pedido cubre {} períodos", poq) } else { String::new() }),
                vec![],
            ));
        }
        if o.method == LotMethod::WagnerWhitin {
            let mut ft = Table::new(Some("Tabla de la programación dinámica"), &["Hasta el período", "f (costo mínimo)", "Último pedido en", "Cubre los períodos"]).left(0).left(2).left(3);
            for k in 1..=n {
                let (c, i, j) = ftab[k];
                ft.row(cells![&names[k - 1], tidy(c), if j == usize::MAX { "—".to_string() } else { names[j].clone() }, if j == usize::MAX { "—".to_string() } else { format!("{} a {}", names[i], names[k - 1]) }]);
            }
            v.push(step("Programación dinámica", "Para cada período final k se prueba cada bloque i..k y el período j en que se produce; f(n) es el costo óptimo y el plan se reconstruye hacia atrás.", vec![ft.block()]));
        }
        v.push(step("Plan resultante", format!("Costo total {} con {} pedidos.", fmt(tidy(sel.total())), sel.orders()), vec![pt.clone().block()]));
        v.push(step("Comparar con los otros métodos", "Las reglas heurísticas son rápidas y fáciles de explicar; Wagner-Whitin garantiza el mínimo.", vec![ct.clone().block()]));
        st.push(steps(v));
    }
    let mut blocks = vec![metrics(vec![metric_main("Costo total", tidy(sel.total())), metric("Pedidos", sel.orders()), metric("Óptimo (Wagner-Whitin)", tidy(opt))]), pt.block()];
    if !sel.feasible {
        blocks.insert(0, text_tone("Este plan deja demanda atrasada en períodos donde no se permiten atrasos.", Tone::Accent));
    }
    Ok(Report::new("its", &m.name, if o.method == LotMethod::WagnerWhitin { Status::Optimal } else { Status::Feasible }, format!("{}: costo total {} con {} pedidos (óptimo {}).", method_label(o.method), fmt(tidy(sel.total())), sel.orders(), fmt(tidy(opt))))
        .section("solucion", "Solución", blocks)
        .section("comparacion", "Comparación", vec![ct.block()])
        .section("graficos", "Gráficos", vec![ch.block()])
        .section("pasos", "Paso a paso", st)
        .data(json!({
            "method": o.method,
            "total_cost": tidy(sel.total()),
            "orders": sel.q.iter().map(|x| tidy(*x)).collect::<Vec<_>>(),
            "optimal_cost": tidy(opt),
            "methods": scheds.iter().map(|(mt, s)| json!({"method": mt, "total_cost": tidy(s.total()), "orders": s.orders(), "feasible": s.feasible})).collect::<Vec<_>>(),
        })))
}
