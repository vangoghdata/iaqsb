//! PERT probabilístico: tiempos esperados y varianzas, ruta crítica con tiempos
//! medios, probabilidad de terminar en un plazo (aproximación normal) y
//! simulación de Monte Carlo del proyecto completo.

use super::cpm::{build, cpm_steps, gantt, network, path_names, schedule, Net};
use super::dist::{phi, Dist, Law};
use super::{PertModel, PertOptions};
use iaqsb_core::report::*;
use iaqsb_core::{cells, ModError};
use rand::SeedableRng;
use serde_json::{json, Value};

pub fn laws(m: &PertModel) -> Result<Vec<Law>, ModError> {
    m.activities
        .iter()
        .map(|a| {
            let d = if m.time_distribution == Dist::Mixed {
                a.distribution.filter(|d| *d != Dist::Mixed).ok_or_else(|| ModError::Invalid(format!("actividad {}: indica su distribución", a.name)))?
            } else {
                m.time_distribution
            };
            Law::new(d, [a.p1, a.p2, a.p3], a.discrete.as_deref(), &format!("actividad {}", a.name))
        })
        .collect()
}

/// z de la normal estándar para una probabilidad (búsqueda por bisección sobre Φ).
fn z_of(p: f64) -> f64 {
    let (mut lo, mut hi) = (-10.0, 10.0);
    for _ in 0..100 {
        let mid = 0.5 * (lo + hi);
        if phi(mid) < p {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    0.5 * (lo + hi)
}

struct SimResult {
    totals: Vec<f64>,
    critical_count: Vec<usize>,
    mean_time: Vec<f64>,
}

fn simulate(net: &Net, laws: &[Law], runs: usize, seed: u64) -> SimResult {
    let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(seed);
    let n = net.n;
    let mut totals = Vec::with_capacity(runs);
    let mut critical_count = vec![0usize; n];
    let mut sum_t = vec![0.0; n];
    let mut t = vec![0.0; n];
    for _ in 0..runs {
        for i in 0..n {
            t[i] = laws[i].sample(&mut rng);
            sum_t[i] += t[i];
        }
        let s = schedule(net, &t);
        for i in 0..n {
            if s.critical[i] {
                critical_count[i] += 1;
            }
        }
        totals.push(s.total);
    }
    let mean_time = sum_t.iter().map(|x| x / runs.max(1) as f64).collect();
    SimResult { totals, critical_count, mean_time }
}

fn quantile(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return f64::NAN;
    }
    let k = ((p * sorted.len() as f64).ceil() as usize).clamp(1, sorted.len());
    sorted[k - 1]
}

pub fn solve_pert(m: &PertModel, o: &PertOptions) -> Result<Report, ModError> {
    let net = build(m)?;
    let laws = laws(m)?;
    let n = net.n;
    let unit = m.unit_plural();
    let mean: Vec<f64> = laws.iter().map(|l| l.mean()).collect();
    let var: Vec<f64> = laws.iter().map(|l| l.variance()).collect();
    if let Some(i) = (0..n).find(|&i| !mean[i].is_finite()) {
        return Err(ModError::Invalid(format!("actividad {}: su distribución no tiene media finita", net.names[i])));
    }
    let s = schedule(&net, &mean);
    let paths = s.critical_paths(&net, 200);
    let pstats: Vec<(f64, f64)> = paths
        .iter()
        .map(|p| {
            let mu: f64 = p.iter().map(|&i| mean[i]).sum();
            let v: f64 = p.iter().map(|&i| var[i]).sum();
            (mu, v.sqrt())
        })
        .collect();
    // ruta de referencia: la de mayor varianza (la más incierta)
    let main = (0..paths.len()).max_by(|&a, &b| pstats[a].1.partial_cmp(&pstats[b].1).unwrap()).unwrap_or(0);
    let (mu, sigma) = pstats.get(main).copied().unwrap_or((s.total, 0.0));
    let prob = |d: f64, mu: f64, sd: f64| -> f64 {
        if sd <= 1e-12 {
            if d + 1e-9 >= mu {
                1.0
            } else {
                0.0
            }
        } else {
            phi((d - mu) / sd)
        }
    };

    // ---- solución
    let mut at = Table::new(
        Some("Análisis de actividades (tiempos esperados)"),
        &["Actividad", "¿Crítica?", "Distribución", "Tiempo medio", "Desviación", "Inicio temprano", "Fin temprano", "Inicio tardío", "Fin tardío", "Holgura"],
    )
    .left(0)
    .left(1)
    .left(2);
    for i in 0..n {
        at.row_tone(
            cells![&net.names[i], if s.critical[i] { "sí" } else { "no" }, laws[i].dist.label(), mean[i], var[i].sqrt(), s.es[i], s.ef[i], s.ls[i], s.lf[i], s.slack[i]],
            if s.critical[i] { Tone::Highlight } else { Tone::Normal },
        );
    }
    let mut pt = Table::new(Some("Rutas críticas"), &["#", "Ruta", "Duración esperada", "Desviación"]).left(1);
    for (k, p) in paths.iter().enumerate() {
        pt.row(cells![k + 1, path_names(&net, p), pstats[k].0, pstats[k].1]);
    }
    let mut ms = vec![metric_main("Duración esperada", s.total), metric("Desviación de la ruta crítica", sigma), metric("Rutas críticas", paths.len())];
    let mut data = json!({
        "expected_completion_time": clean(s.total),
        "std_dev": clean(sigma),
        "critical_paths": paths.iter().zip(&pstats).map(|(p, st)| json!({
            "path": p.iter().map(|&i| net.names[i].clone()).collect::<Vec<_>>(), "mean": clean(st.0), "std_dev": clean(st.1)
        })).collect::<Vec<_>>(),
        "activities": (0..n).map(|i| json!({
            "name": net.names[i], "distribution": laws[i].dist.id(), "mean": clean(mean[i]), "std_dev": clean(var[i].sqrt()),
            "es": clean(s.es[i]), "ef": clean(s.ef[i]), "ls": clean(s.ls[i]), "lf": clean(s.lf[i]), "slack": clean(s.slack[i]), "critical": s.critical[i]
        })).collect::<Vec<_>>(),
    });
    let mut headline = format!("Duración esperada {} {unit} (σ = {}); ruta crítica: {}.", fmt(s.total), fmt(sigma), paths.first().map(|p| path_names(&net, p)).unwrap_or_default());

    // ---- probabilidad
    let mut pb = vec![];
    if let Some(d) = o.desired_time {
        let mut t = Table::new(Some(&format!("Probabilidad de terminar en {} {unit}", fmt(d))), &["Ruta crítica", "Duración esperada", "Desviación", "z = (plazo − media)/σ", "Probabilidad"]).left(0);
        let mut overall = 1.0;
        let mut per = vec![];
        for (k, p) in paths.iter().enumerate() {
            let (pm, ps) = pstats[k];
            let z = if ps > 1e-12 { Cell::Num((d - pm) / ps) } else { Cell::from("—") };
            let pr = prob(d, pm, ps);
            overall *= pr;
            per.push(pr);
            t.row(cells![path_names(&net, p), pm, ps, z, pr]);
        }
        if paths.len() > 1 {
            t.row_tone(cells!["Proyecto (rutas independientes)", "", "", "", overall], Tone::Total);
        }
        let p0 = per.get(main).copied().unwrap_or(1.0);
        ms.push(metric(format!("P(terminar en {})", fmt(d)), if paths.len() > 1 { overall } else { p0 }));
        pb.push(text(if paths.len() > 1 {
            format!(
                "Con varias rutas críticas, el producto de sus probabilidades ({}) supone que son independientes; como comparten actividades, la probabilidad real es algo mayor. La simulación la estima mejor.",
                fmt(overall)
            )
        } else {
            format!(
                "La duración del proyecto se aproxima con una normal de media {} y desviación {} (suma de las varianzas de la ruta crítica, por el teorema central del límite): P = Φ({}) = {}.",
                fmt(mu),
                fmt(sigma),
                fmt(if sigma > 0.0 { (d - mu) / sigma } else { 0.0 }),
                fmt(p0)
            )
        }));
        pb.push(t.block());
        headline = format!(
            "Duración esperada {} {unit} (σ = {}); probabilidad de terminar en {} {unit}: {}.",
            fmt(s.total),
            fmt(sigma),
            fmt(d),
            fmt(if paths.len() > 1 { overall } else { p0 })
        );
        data["probability"] = json!({"desired_time": d, "by_path": per, "project": clean(if paths.len() > 1 { overall } else { p0 })});
    }
    if sigma > 0.0 {
        let mut q = Table::new(Some("Plazo con una probabilidad dada (ruta crítica)"), &["Probabilidad", "z", "Plazo"]);
        for p in [0.5, 0.8, 0.9, 0.95, 0.975, 0.99] {
            let z = z_of(p);
            q.row(cells![p, z, mu + z * sigma]);
        }
        let pts: Vec<[f64; 2]> = (0..=80).map(|k| {
            let x = mu - 3.5 * sigma + 7.0 * sigma * k as f64 / 80.0;
            [x, phi((x - mu) / sigma)]
        }).collect();
        let mut ch = Chart::new(Some("Probabilidad de terminar antes de cada fecha"), &format!("Tiempo ({unit})"), "Probabilidad")
            .series(Series::new("P(duración ≤ t)", SeriesKind::Line, pts).tone(Tone::Accent));
        ch.y_zero = true;
        if let Some(d) = o.desired_time {
            ch = ch.vline(d, format!("plazo {}", fmt(d)), Tone::Highlight);
        }
        pb.push(ch.block());
        pb.push(q.block());
    } else {
        pb.push(text("La ruta crítica no tiene variabilidad: el proyecto dura siempre lo mismo."));
    }

    // ---- simulación
    let mut sim_blocks = vec![];
    let runs = o.simulation_runs as usize;
    if runs > 0 {
        if runs > 1_000_000 || (runs as f64) * (n as f64) > 5e7 {
            return Err(ModError::Invalid("demasiadas observaciones para el tamaño del proyecto (máximo 50 millones de actividades simuladas)".into()));
        }
        let r = simulate(&net, &laws, runs, o.seed);
        let mut sorted = r.totals.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let avg = sorted.iter().sum::<f64>() / runs as f64;
        let sd = if runs > 1 { (sorted.iter().map(|x| (x - avg).powi(2)).sum::<f64>() / (runs - 1) as f64).sqrt() } else { 0.0 };
        let (lo, hi) = (sorted[0], sorted[runs - 1]);
        let mut sms = vec![metric_main("Duración media simulada", avg), metric("Desviación", sd), metric("Mínimo", lo), metric("Máximo", hi)];
        let mut sim_prob = Value::Null;
        if let Some(d) = o.desired_time {
            let c = sorted.iter().filter(|x| **x <= d + 1e-9).count();
            let p = c as f64 / runs as f64;
            sms.insert(1, metric(format!("P(terminar en {})", fmt(d)), p));
            sim_prob = json!(p);
            // error estándar de la proporción
            let se = (p * (1.0 - p) / runs as f64).sqrt();
            sim_blocks.push(text(format!(
                "En {} de {runs} proyectos simulados se terminó en {} {unit} o menos: probabilidad {} (± {} con 95 % de confianza).",
                c,
                fmt(d),
                fmt(p),
                fmt(1.96 * se)
            )));
        }
        sim_blocks.insert(0, metrics(sms));
        // histograma de 20 intervalos
        let bins = 20;
        let w = if hi > lo { (hi - lo) / bins as f64 } else { 1.0 };
        let mut freq = vec![0usize; bins];
        for x in &sorted {
            let k = if hi > lo { (((x - lo) / w).floor() as usize).min(bins - 1) } else { 0 };
            freq[k] += 1;
        }
        let mut ht = Table::new(Some("Distribución de la duración simulada"), &["Desde", "Hasta (incluido)", "Frecuencia", "%", "% acumulado"]);
        let mut acc = 0;
        let mut cats = vec![];
        let mut bars = vec![];
        for k in 0..bins {
            acc += freq[k];
            let a = lo + w * k as f64;
            let b = if k == bins - 1 { hi } else { lo + w * (k + 1) as f64 };
            ht.row(cells![a, b, freq[k], 100.0 * freq[k] as f64 / runs as f64, 100.0 * acc as f64 / runs as f64]);
            // etiquetas con los decimales que pide el ancho del intervalo
            let dec = (1.0 - w.log10().floor()).clamp(0.0, 4.0) as usize;
            cats.push(format!("{:.*}", dec, (a + b) / 2.0));
            bars.push([k as f64, 100.0 * freq[k] as f64 / runs as f64]);
        }
        let mut ch = Chart::new(Some("Histograma de la duración del proyecto"), &format!("Duración ({unit})"), "% de las simulaciones")
            .series(Series::new("Frecuencia", SeriesKind::Bar, bars).tone(Tone::Accent));
        ch.categories = Some(cats);
        ch.y_zero = true;
        sim_blocks.push(ch.block());
        let mut ct = Table::new(Some("Índice de criticidad"), &["Actividad", "Crítica en las simulaciones (%)", "Tiempo medio simulado", "Crítica en PERT"]).left(0).left(3);
        let mut order: Vec<usize> = (0..n).collect();
        order.sort_by(|&a, &b| r.critical_count[b].cmp(&r.critical_count[a]));
        for i in order {
            let ci = 100.0 * r.critical_count[i] as f64 / runs as f64;
            ct.row_tone(cells![&net.names[i], ci, r.mean_time[i], if s.critical[i] { "sí" } else { "no" }], if ci >= 50.0 { Tone::Highlight } else { Tone::Normal });
        }
        sim_blocks.push(
            ct.note("Porcentaje de simulaciones en que la actividad quedó en la ruta crítica: muestra actividades que el PERT clásico no ve como críticas.")
                .block(),
        );
        let mut qt = Table::new(Some("Percentiles de la duración"), &["Probabilidad", "Duración simulada", "Aproximación normal"]);
        for p in [0.5, 0.8, 0.9, 0.95, 0.99] {
            qt.row(cells![p, quantile(&sorted, p), if sigma > 0.0 { Cell::Num(mu + z_of(p) * sigma) } else { Cell::Num(mu) }]);
        }
        sim_blocks.push(qt.block());
        sim_blocks.push(ht.block());
        if avg > s.total + 1e-6 {
            sim_blocks.push(text(format!(
                "La duración media simulada ({}) supera a la de la ruta crítica con tiempos medios ({}): cuando varias rutas compiten, el máximo de duraciones aleatorias tiende a ser mayor que el máximo de las medias.",
                fmt(avg),
                fmt(s.total)
            )));
        }
        sim_blocks.push(text_tone(format!("Observaciones: {runs}. Semilla: {}.", o.seed), Tone::Muted));
        data["simulation"] = json!({
            "runs": runs, "seed": o.seed, "mean": clean(avg), "std_dev": clean(sd), "min": clean(lo), "max": clean(hi),
            "probability": sim_prob,
            "percentiles": {"p50": quantile(&sorted, 0.5), "p90": quantile(&sorted, 0.9), "p95": quantile(&sorted, 0.95)},
            "criticality": (0..n).map(|i| json!({"name": net.names[i], "index": r.critical_count[i] as f64 / runs as f64})).collect::<Vec<_>>(),
        });
    }

    // ---- pasos
    let mut st = vec![];
    if o.steps {
        let mut mt = Table::new(Some("Tiempo esperado y varianza de cada actividad"), &["Actividad", "Distribución", "Cálculo", "Tiempo esperado", "Varianza"]).left(0).left(1).left(2);
        for i in 0..n {
            mt.row(cells![&net.names[i], laws[i].dist.label(), laws[i].formula(), mean[i], var[i]]);
        }
        let intro = vec![step(
            "Tiempos esperados",
            "PERT sustituye cada tiempo aleatorio por su valor esperado y guarda su varianza. Con tres estimaciones (a, m, b), la distribución beta da t = (a + 4m + b)/6 y σ² = ((b − a)/6)².",
            vec![mt.block()],
        )];
        let mut v = cpm_steps(m, &net, &s, intro);
        if let Some(p) = paths.get(main) {
            let terms: Vec<String> = p.iter().map(|&i| fmt(var[i])).collect();
            let mut b = vec![text(format!(
                "Varianza de la ruta {} = {} = {}; desviación σ = √{} = {}.",
                path_names(&net, p),
                terms.join(" + "),
                fmt(sigma * sigma),
                fmt(sigma * sigma),
                fmt(sigma)
            ))];
            if let Some(d) = o.desired_time {
                if sigma > 0.0 {
                    let z = (d - mu) / sigma;
                    b.push(text(format!(
                        "Probabilidad de terminar en {}: z = ({} − {}) / {} = {}; P = Φ({}) = {}.",
                        fmt(d),
                        fmt(d),
                        fmt(mu),
                        fmt(sigma),
                        fmt(z),
                        fmt(z),
                        fmt(phi(z))
                    )));
                }
            }
            v.push(step(
                "Varianza de la ruta crítica",
                "Si los tiempos de las actividades son independientes, la varianza de la ruta es la suma de sus varianzas, y su duración es aproximadamente normal.",
                b,
            ));
        }
        st.push(steps(v));
    }

    let mut markers = vec![];
    if let Some(d) = o.desired_time {
        markers.push(RefLine { value: d, label: format!("plazo {}", fmt(d)), tone: Tone::Accent });
    }
    let rep = Report::new("pert", &m.name, Status::Solved, headline)
        .section("solucion", "Solución", vec![metrics(ms), at.block(), pt.block()])
        .section("red", "Red", vec![network(m, &net, Some(&s), "Red del proyecto (tiempos esperados)")])
        .section("gantt", "Gantt", vec![gantt(&net, &s, "Diagrama de Gantt (tiempos esperados)", &unit, markers)])
        .section("probabilidad", "Probabilidad", pb)
        .section("simulacion", "Simulación", sim_blocks)
        .section("pasos", "Paso a paso", st)
        .data(data);
    Ok(rep)
}
