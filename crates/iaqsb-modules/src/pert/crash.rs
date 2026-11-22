//! Análisis de compresión (crashing). El programa exacto es lineal:
//!
//!   mín  Σ kᵢ·yᵢ (+ penalización o premio)
//!   s.a. Sⱼ ≥ Sᵢ + tᵢ − yᵢ   para cada precedencia i → j
//!        T  ≥ Sᵢ + tᵢ − yᵢ   para cada actividad
//!        0 ≤ yᵢ ≤ tᵢ − cᵢ
//!
//! donde yᵢ es lo que se comprime la actividad y kᵢ su costo por unidad de
//! tiempo. Lo resuelve HiGHS. La curva tiempo-costo se obtiene resolviendo el
//! mismo programa para cada duración, y es la base del paso a paso.

use super::cpm::{gantt, path_names, schedule, Net};
use super::{Crashing, PertModel, PertOptions};
use iaqsb_core::lp::{solve, Lp, LpStatus, Sense};
use iaqsb_core::report::*;
use iaqsb_core::{cells, ModError};
use serde_json::{json, Value};

pub struct CrashResult {
    pub status: Status,
    pub headline: String,
    pub blocks: Vec<Block>,
    pub data: Value,
}

struct Data {
    t: Vec<f64>,
    /// Máxima reducción de cada actividad.
    max: Vec<f64>,
    /// Costo por unidad de tiempo comprimida.
    slope: Vec<f64>,
    base: f64,
}

fn data(m: &PertModel) -> Result<Data, ModError> {
    let mut d = Data { t: vec![], max: vec![], slope: vec![], base: 0.0 };
    for a in &m.activities {
        let t = a.normal_time.unwrap_or(0.0);
        let c = a.crash_time.unwrap_or(t);
        let r = (t - c).max(0.0);
        let mut k = 0.0;
        if r > 1e-12 {
            let (nc, cc) = match (a.normal_cost, a.crash_cost) {
                (Some(x), Some(y)) => (x, y),
                _ => return Err(ModError::Invalid(format!("actividad {}: para comprimirla hacen falta el costo normal y el de compresión", a.name))),
            };
            k = (cc - nc) / r;
            if k < -1e-12 {
                return Err(ModError::Invalid(format!("actividad {}: el costo de compresión es menor que el normal", a.name)));
            }
        }
        d.t.push(t);
        d.max.push(r);
        d.slope.push(k.max(0.0));
        d.base += a.normal_cost.unwrap_or(0.0);
    }
    Ok(d)
}

/// Modelo base. Devuelve (lp, índice de T). Variables: S (n), y (n), T.
fn model(net: &Net, d: &Data) -> (Lp, usize) {
    let n = net.n;
    let mut lp = Lp::new(0, false);
    for _ in 0..n {
        lp.add_var(0.0, 0.0, f64::INFINITY, false);
    }
    let kmax = d.slope.iter().cloned().fold(1.0, f64::max);
    for i in 0..n {
        // un poco de costo para no comprimir sin necesidad las actividades gratuitas
        lp.add_var(d.slope[i] + 1e-7 * kmax, 0.0, d.max[i], false);
    }
    let tt = lp.add_var(0.0, 0.0, f64::INFINITY, false);
    for j in 0..n {
        for &i in &net.preds[j] {
            lp.row(vec![(j, 1.0), (i, -1.0), (n + i, 1.0)], Sense::Ge, d.t[i]);
        }
    }
    for i in 0..n {
        lp.row(vec![(tt, 1.0), (i, -1.0), (n + i, 1.0)], Sense::Ge, d.t[i]);
    }
    (lp, tt)
}

fn crash_of(net: &Net, x: &[f64], d: &Data) -> Vec<f64> {
    // HiGHS trabaja con tolerancia ~1e-7: lo que está a esa distancia de un valor
    // con 6 decimales se redondea, para no mostrar ruido como 39.80000004
    (0..net.n)
        .map(|i| tidy(x[net.n + i]).clamp(0.0, d.max[i]))
        .collect()
}

/// Costo mínimo de compresión para terminar en `limit` (None si no se puede).
fn min_cost_at(net: &Net, d: &Data, limit: f64) -> Result<Option<Vec<f64>>, ModError> {
    let (mut lp, tt) = model(net, d);
    lp.ub[tt] = limit;
    let s = solve(&lp);
    match s.status {
        LpStatus::Optimal => Ok(Some(crash_of(net, &s.x, d))),
        LpStatus::Infeasible => Ok(None),
        LpStatus::Unbounded => Err(ModError::Invalid("el programa de compresión no está acotado".into())),
        LpStatus::Error(e) => Err(ModError::Invalid(format!("error del optimizador: {e}"))),
    }
}

fn direct(d: &Data, y: &[f64]) -> f64 {
    y.iter().zip(&d.slope).map(|(a, b)| a * b).sum()
}

pub fn analyze(m: &PertModel, net: &Net, o: &PertOptions) -> Result<CrashResult, ModError> {
    let d = data(m)?;
    let n = net.n;
    let unit = m.unit_plural();
    let normal = schedule(net, &d.t);
    let crash_t: Vec<f64> = (0..n).map(|i| d.t[i] - d.max[i]).collect();
    let fastest = schedule(net, &crash_t);
    let tn = normal.total;
    let tc = fastest.total;
    let eps = normal.eps;
    let crash_all: f64 = (0..n).map(|i| d.max[i] * d.slope[i]).sum();
    let ref_metrics = vec![
        metric("Con tiempos normales", format!("{} {unit} · {}", fmt(tn), fmt(d.base))),
        metric("Comprimiendo todo", format!("{} {unit} · {}", fmt(tc), fmt(d.base + crash_all))),
    ];

    let fail = |msg: String| CrashResult {
        status: Status::Infeasible,
        headline: msg.clone(),
        blocks: vec![text_tone(msg, Tone::Bad), metrics(ref_metrics.clone())],
        data: json!({"feasible": false}),
    };

    // ---- elegir la compresión según el criterio
    let (y, desc): (Vec<f64>, String) = match o.crashing {
        Crashing::None => unreachable!(),
        Crashing::Deadline => {
            let dl = o.desired_time.ok_or_else(|| ModError::Invalid("indica el plazo deseado para la compresión".into()))?;
            if dl < tc - eps {
                return Ok(fail(format!(
                    "El plazo deseado de {} {unit} es inalcanzable: aun comprimiendo todas las actividades, el proyecto dura {} {unit}.",
                    fmt(dl),
                    fmt(tc)
                )));
            }
            let y = min_cost_at(net, &d, dl)?.ok_or_else(|| ModError::Invalid("no se encontró una compresión factible".into()))?;
            (y, format!("Programa de costo mínimo para terminar en {} {unit} o antes.", fmt(dl)))
        }
        Crashing::Budget => {
            let b = o.budget.ok_or_else(|| ModError::Invalid("indica el presupuesto total para la compresión".into()))?;
            if b < d.base - 1e-9 * d.base.abs().max(1.0) {
                return Ok(fail(format!("El presupuesto de {} no alcanza ni para el costo normal del proyecto ({}).", fmt(b), fmt(d.base))));
            }
            let (mut lp, tt) = model(net, &d);
            // primero, la menor duración; el gasto se ajusta después
            for i in 0..n {
                lp.c[n + i] = 0.0;
            }
            lp.c[tt] = 1.0;
            lp.row((0..n).map(|i| (n + i, d.slope[i])).collect(), Sense::Le, b - d.base);
            let s = solve(&lp);
            if s.status != LpStatus::Optimal {
                return Err(ModError::Invalid("no se pudo resolver la compresión por presupuesto".into()));
            }
            let best_t = s.x[tt];
            // con esa duración, el gasto mínimo
            let y = min_cost_at(net, &d, best_t + 1e-9 * best_t.max(1.0))?.unwrap_or_else(|| crash_of(net, &s.x, &d));
            (y, format!("Menor duración posible con un presupuesto total de {}.", fmt(b)))
        }
        Crashing::MinCost => {
            let dl = o.desired_time.unwrap_or(tn);
            let pen = o.late_penalty.unwrap_or(0.0);
            let rew = o.early_reward.unwrap_or(0.0);
            if pen < 0.0 || rew < 0.0 {
                return Err(ModError::Invalid("la penalización y el premio no pueden ser negativos".into()));
            }
            let mut best: Option<(f64, Vec<f64>)> = None;
            // tramo 1: T ≤ plazo, premio por cada unidad de adelanto
            if dl >= tc - eps {
                let (mut lp, tt) = model(net, &d);
                lp.ub[tt] = dl;
                lp.c[tt] = rew;
                let s = solve(&lp);
                if s.status == LpStatus::Optimal {
                    let y = crash_of(net, &s.x, &d);
                    let t = schedule(net, &(0..n).map(|i| d.t[i] - y[i]).collect::<Vec<_>>()).total;
                    let cost = direct(&d, &y) - rew * (dl - t);
                    best = Some((cost, y));
                }
            }
            // tramo 2: T ≥ plazo, penalización por cada unidad de retraso
            if tn > dl + eps || best.is_none() {
                let (mut lp, tt) = model(net, &d);
                lp.lb[tt] = dl.max(0.0);
                lp.c[tt] = pen;
                let s = solve(&lp);
                if s.status == LpStatus::Optimal {
                    let y = crash_of(net, &s.x, &d);
                    let t = schedule(net, &(0..n).map(|i| d.t[i] - y[i]).collect::<Vec<_>>()).total;
                    let cost = direct(&d, &y) + pen * (t - dl).max(0.0);
                    if best.as_ref().map(|b| cost < b.0 - 1e-9 * b.0.abs().max(1.0)).unwrap_or(true) {
                        best = Some((cost, y));
                    }
                }
            }
            let (_, y) = best.ok_or_else(|| ModError::Invalid("no se encontró un programa factible".into()))?;
            let what = if o.desired_time.is_some() {
                format!(
                    "Programa de costo total mínimo con plazo {} {unit}, penalización {} y premio {} por {}.",
                    fmt(dl),
                    fmt(pen),
                    fmt(rew),
                    m.time_unit
                )
            } else {
                "Programa de costo total mínimo (sin plazo, penalización ni premio: no conviene comprimir).".into()
            };
            (y, what)
        }
    };

    let tsug: Vec<f64> = (0..n).map(|i| clean(d.t[i] - y[i])).collect();
    let s = schedule(net, &tsug);
    let extra = direct(&d, &y);
    let mut total = d.base + extra;
    let mut pr_line = None;
    if o.crashing == Crashing::MinCost {
        if let Some(dl) = o.desired_time {
            let pen = o.late_penalty.unwrap_or(0.0);
            let rew = o.early_reward.unwrap_or(0.0);
            let late = (s.total - dl).max(0.0);
            let early = (dl - s.total).max(0.0);
            let pr = pen * late - rew * early;
            total += pr;
            pr_line = Some((late, early, pr));
        }
    }

    // ---- tabla de resultados
    let mut t = Table::new(
        Some("Análisis de compresión"),
        &["Actividad", "¿Crítica?", "Tiempo normal", "Tiempo de compresión", "Tiempo sugerido", "Costo adicional", "Costo normal", "Costo sugerido"],
    )
    .left(0)
    .left(1);
    for i in 0..n {
        let a = &m.activities[i];
        let add = y[i] * d.slope[i];
        let nc = a.normal_cost.unwrap_or(0.0);
        let tone = if y[i] > 1e-9 { Tone::Accent } else if s.critical[i] { Tone::Highlight } else { Tone::Normal };
        t.row_tone(
            cells![&a.name, if s.critical[i] { "sí" } else { "no" }, d.t[i], d.t[i] - d.max[i], tsug[i], add, nc, nc + add],
            tone,
        );
    }
    t.row_tone(cells!["Total", "", "", "", "", extra, d.base, d.base + extra], Tone::Total);
    let t = t.note("En azul, las actividades que conviene comprimir; en ámbar, las críticas del programa sugerido.");

    let mut ms = vec![
        metric_main("Duración sugerida", s.total),
        metric("Costo adicional de compresión", extra),
        metric("Costo total del proyecto", total),
        metric("Duración normal", tn),
    ];
    if let Some((late, early, pr)) = pr_line {
        if late > 0.0 {
            ms.push(metric("Retraso sobre el plazo", late));
        }
        if early > 0.0 {
            ms.push(metric("Adelanto sobre el plazo", early));
        }
        ms.push(metric("Penalización (+) / premio (−)", pr));
    }
    let crashed: Vec<String> = (0..n).filter(|&i| y[i] > 1e-9).map(|i| format!("{} en {}", m.activities[i].name, fmt(y[i]))).collect();
    let paths = s.critical_paths(net, 50);
    let headline = if crashed.is_empty() {
        format!("No conviene comprimir: el proyecto dura {} {unit} con un costo de {}.", fmt(s.total), fmt(total))
    } else {
        format!("Comprimiendo {} el proyecto dura {} {unit} con un costo total de {}.", crashed.join(", "), fmt(s.total), fmt(total))
    };

    let mut blocks = vec![
        metrics(ms),
        text(format!(
            "{desc}\nRutas críticas del programa sugerido: {}.",
            paths.iter().map(|p| path_names(net, p)).collect::<Vec<_>>().join("; ")
        )),
        t.block(),
        metrics(ref_metrics),
    ];

    // ---- curva tiempo-costo y paso a paso
    let curve = curve_points(net, &d, tn, tc)?;
    if curve.len() >= 2 {
        let mut ch = Chart::new(Some("Curva tiempo-costo"), &format!("Duración ({unit})"), "Costo total").series(
            Series::new("Costo directo mínimo", SeriesKind::Line, curve.iter().map(|(tt, y)| [*tt, d.base + direct(&d, y)]).collect()).tone(Tone::Accent),
        );
        if o.crashing == Crashing::MinCost {
            if let Some(dl) = o.desired_time {
                let pen = o.late_penalty.unwrap_or(0.0);
                let rew = o.early_reward.unwrap_or(0.0);
                let pts = curve.iter().map(|(tt, y)| [*tt, d.base + direct(&d, y) + pen * (tt - dl).max(0.0) - rew * (dl - tt).max(0.0)]).collect();
                ch = ch.series(Series::new("Con penalización y premio", SeriesKind::Line, pts).tone(Tone::Highlight));
                ch = ch.vline(dl, format!("plazo {}", fmt(dl)), Tone::Muted);
            }
        }
        ch = ch.vline(s.total, format!("sugerido {}", fmt(s.total)), Tone::Highlight);
        blocks.push(ch.block());
    }
    blocks.push(gantt(net, &s, "Gantt del programa sugerido", &unit, vec![]));
    if o.steps {
        blocks.push(Block::Steps { title: Some("Compresión paso a paso".into()), steps: crash_steps(m, net, &d, &curve) });
    }

    let data = json!({
        "feasible": true,
        "mode": o.crashing,
        "normal_time": clean(tn),
        "crash_time": clean(tc),
        "suggested_time": clean(s.total),
        "additional_cost": clean(extra),
        "normal_cost": clean(d.base),
        "total_cost": clean(total),
        "activities": (0..n).map(|i| json!({
            "name": m.activities[i].name, "suggested_time": tsug[i], "reduction": clean(y[i]),
            "cost_per_unit": clean(d.slope[i]), "additional_cost": clean(y[i] * d.slope[i]), "critical": s.critical[i]
        })).collect::<Vec<_>>(),
        "time_cost_curve": curve.iter().map(|(tt, y)| json!([clean(*tt), clean(d.base + direct(&d, y))])).collect::<Vec<_>>(),
    });
    Ok(CrashResult { status: Status::Optimal, headline, blocks, data })
}

/// Duraciones de la curva: de la normal a la mínima, de unidad en unidad si es posible.
fn curve_points(net: &Net, d: &Data, tn: f64, tc: f64) -> Result<Vec<(f64, Vec<f64>)>, ModError> {
    if tn - tc <= 1e-9 || net.n > 400 {
        return Ok(vec![]);
    }
    let mut ts = vec![];
    if tn - tc <= 60.0 {
        let mut x = tn;
        while x > tc + 1e-9 {
            ts.push(x);
            x = if (x - x.floor()).abs() > 1e-9 { x.floor() } else { x - 1.0 };
        }
        ts.push(tc);
    } else {
        for k in 0..=40 {
            ts.push(tn - (tn - tc) * k as f64 / 40.0);
        }
    }
    let mut out = vec![];
    for t in ts {
        if let Some(y) = min_cost_at(net, d, t + 1e-9 * t.max(1.0))? {
            out.push((t, y));
        }
    }
    Ok(out)
}

fn crash_steps(m: &PertModel, net: &Net, d: &Data, curve: &[(f64, Vec<f64>)]) -> Vec<Step> {
    let n = net.n;
    let unit = m.unit_plural();
    let mut steps = vec![];
    let mut k = Table::new(
        Some("Costo por unidad de tiempo comprimida"),
        &["Actividad", "Tiempo normal", "Tiempo de compresión", "Reducción máxima", "Costo normal", "Costo de compresión", "Costo por unidad"],
    )
    .left(0);
    for i in 0..n {
        let a = &m.activities[i];
        let tone = if d.max[i] > 0.0 { Tone::Normal } else { Tone::Muted };
        k.row_tone(
            cells![&a.name, d.t[i], d.t[i] - d.max[i], d.max[i], a.normal_cost, a.crash_cost, if d.max[i] > 0.0 { Cell::Num(d.slope[i]) } else { Cell::from("—") }],
            tone,
        );
    }
    steps.push(step(
        "Pendiente de costo de cada actividad",
        "Costo por unidad = (costo de compresión − costo normal) / (tiempo normal − tiempo de compresión). Para acortar el proyecto hay que acortar todas sus rutas críticas, eligiendo lo más barato.",
        vec![k.block()],
    ));
    let mut prev: Option<&(f64, Vec<f64>)> = None;
    let mut acc = Table::new(Some("Curva tiempo-costo"), &["Duración", "Costo directo", "Costo marginal"]);
    for p in curve {
        let (tt, y) = p;
        let times: Vec<f64> = (0..n).map(|i| clean(d.t[i] - y[i])).collect();
        let s = schedule(net, &times);
        let cost = d.base + direct(d, y);
        let paths = s.critical_paths(net, 20);
        let mut tb = Table::new(None, &["Actividad", "Tiempo", "Comprimido", "Costo adicional", "¿Crítica?"]).left(0).left(4);
        for i in 0..n {
            let delta = prev.map(|q| y[i] - q.1[i]).unwrap_or(0.0);
            let tone = if delta.abs() > 1e-9 { Tone::Accent } else if s.critical[i] { Tone::Highlight } else { Tone::Normal };
            tb.row_tone(cells![&m.activities[i].name, times[i], y[i], y[i] * d.slope[i], if s.critical[i] { "sí" } else { "no" }], tone);
        }
        let desc = match prev {
            None => format!(
                "Con todos los tiempos normales el proyecto dura {} {unit} y cuesta {}. Rutas críticas: {}.",
                fmt(*tt),
                fmt(cost),
                paths.iter().map(|p| path_names(net, p)).collect::<Vec<_>>().join("; ")
            ),
            Some(q) => {
                let changes: Vec<String> = (0..n)
                    .filter(|&i| (y[i] - q.1[i]).abs() > 1e-9)
                    .map(|i| {
                        let dl = y[i] - q.1[i];
                        if dl > 0.0 {
                            format!("acortar {} en {}", m.activities[i].name, fmt(dl))
                        } else {
                            format!("devolver {} a {} de su tiempo", fmt(-dl), m.activities[i].name)
                        }
                    })
                    .collect();
                let dc = cost - (d.base + direct(d, &q.1));
                format!(
                    "Para bajar de {} a {} {unit} lo más barato es {} (costo {}). Rutas críticas: {}.",
                    fmt(q.0),
                    fmt(*tt),
                    if changes.is_empty() { "no cambiar nada".into() } else { changes.join(", ") },
                    fmt(dc),
                    paths.iter().map(|p| path_names(net, p)).collect::<Vec<_>>().join("; ")
                )
            }
        };
        let marginal = prev.map(|q| Cell::Num((cost - d.base - direct(d, &q.1)) / (q.0 - tt).max(1e-12))).unwrap_or(Cell::Empty);
        acc.row(vec![Cell::Num(*tt), Cell::Num(cost), marginal]);
        steps.push(step(format!("Duración {} {unit}", fmt(*tt)), desc, vec![tb.block()]));
        prev = Some(p);
    }
    if curve.len() >= 2 {
        steps.push(step(
            "Curva tiempo-costo",
            "Cada unidad de tiempo menos cuesta igual o más que la anterior: la curva es convexa. Con penalización por retraso o premio por adelanto, el mejor programa es el de menor costo total.",
            vec![acc.block()],
        ));
    }
    steps
}
