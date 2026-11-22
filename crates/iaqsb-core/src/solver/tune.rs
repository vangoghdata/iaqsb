//! Ajuste automático de parámetros (como el «tuning tool» de Gurobi): prueba
//! combinaciones de parámetros del motor en el modelo, mide cada una varias veces
//! y recomienda la más rápida que da el mismo resultado.

use super::{solve, Method, Model, Params, Solution, Status};
use serde::Serialize;
use serde_json::json;
use std::time::Instant;

#[derive(Debug, Clone, Serialize)]
pub struct Trial {
    /// Qué cambia respecto a los valores por defecto.
    pub name: String,
    pub params: Params,
    /// Tiempo de cada repetición (s).
    pub times: Vec<f64>,
    /// Mediana de los tiempos (s).
    pub time: f64,
    pub status: Status,
    pub objective: Option<f64>,
    pub gap: Option<f64>,
    /// ¿Da el mismo objetivo que la configuración por defecto?
    pub same_result: bool,
    /// Tiempo por defecto / tiempo de esta configuración.
    pub speedup: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct TuneReport {
    pub solver: String,
    pub is_mip: bool,
    pub cols: usize,
    pub rows: usize,
    pub budget: f64,
    pub elapsed: f64,
    pub repeats: usize,
    /// Tiempo máximo de cada corrida.
    pub run_limit: f64,
    pub baseline: Trial,
    /// Todas las pruebas, de la más rápida a la más lenta.
    pub trials: Vec<Trial>,
    /// Los parámetros recomendados (vacío: los de por defecto ya son los mejores).
    pub best: Params,
    pub best_name: String,
    pub best_speedup: f64,
    pub summary: String,
}

fn median(v: &mut [f64]) -> f64 {
    v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    if v.is_empty() {
        0.0
    } else if v.len() % 2 == 1 {
        v[v.len() / 2]
    } else {
        (v[v.len() / 2 - 1] + v[v.len() / 2]) / 2.0
    }
}

fn with(base: &Params, f: impl FnOnce(&mut Params)) -> Params {
    let mut p = base.clone();
    f(&mut p);
    p
}

fn raw(base: &Params, k: &str, v: serde_json::Value) -> Params {
    with(base, |p| {
        p.raw.insert(k.into(), v);
    })
}

/// Las combinaciones que se prueban (HiGHS).
fn candidates(m: &Model, base: &Params) -> Vec<(String, Params)> {
    let threads = std::thread::available_parallelism().map(|n| n.get() as u32).unwrap_or(1);
    let mut c: Vec<(String, Params)> = vec![
        ("Presolve apagado".into(), with(base, |p| p.presolve = Some(false))),
        ("Símplex dual".into(), with(base, |p| p.method = Some(Method::DualSimplex))),
        ("Símplex primal".into(), with(base, |p| p.method = Some(Method::PrimalSimplex))),
        ("Punto interior (barrera)".into(), with(base, |p| p.method = Some(Method::Barrier))),
    ];
    if m.is_mip() {
        c.extend([
            ("Más heurísticas (esfuerzo 0,3)".into(), raw(base, "mip_heuristic_effort", json!(0.3))),
            ("Sin heurísticas".into(), raw(base, "mip_heuristic_effort", json!(0.0))),
            ("Sin detección de simetrías".into(), raw(base, "mip_detect_symmetry", json!(false))),
            ("Más rondas de cortes en la raíz".into(), raw(base, "mip_lp_age_limit", json!(20))),
            ("Otra semilla (7)".into(), with(base, |p| p.seed = Some(7))),
        ]);
    } else if m.cols.len() * m.rows.len() > 1_000_000 {
        c.push(("PDLP (primer orden)".into(), with(base, |p| p.method = Some(Method::Pdlp))));
    }
    if threads > 1 {
        c.push(("Un solo hilo".into(), with(base, |p| p.threads = Some(1))));
    }
    c
}

fn run(m: &Model, p: &Params, limit: f64, repeats: usize) -> (Vec<f64>, Solution) {
    let mut times = vec![];
    let mut last = None;
    for _ in 0..repeats {
        let mut q = p.clone();
        q.time_limit = Some(q.time_limit.map_or(limit, |t| t.min(limit)));
        q.log = false;
        let t0 = Instant::now();
        let s = solve(m, &q);
        times.push(t0.elapsed().as_secs_f64());
        last = Some(s);
    }
    (times, last.expect("al menos una corrida"))
}

fn same(a: Option<f64>, b: Option<f64>, is_mip: bool, gap: f64) -> bool {
    match (a, b) {
        (Some(x), Some(y)) => (x - y).abs() <= (if is_mip { gap.max(1e-6) } else { 1e-6 }) * (1.0 + x.abs().max(y.abs())),
        (None, None) => true,
        _ => false,
    }
}

/// Ajusta los parámetros del motor para este modelo con un presupuesto de
/// tiempo total (segundos). Usa siempre HiGHS (los parámetros propios de los
/// demás motores no se ajustan aquí).
pub fn tune(m: &Model, base: &Params, budget: f64, repeats: usize) -> TuneReport {
    let t0 = Instant::now();
    let base = with(base, |p| p.solver = None);
    let repeats = repeats.clamp(1, 5);
    let budget = budget.clamp(1.0, 3600.0);
    // primero, la configuración por defecto (limitada a la mitad del presupuesto)
    let first_limit = budget / 2.0;
    let (mut times, sol) = run(m, &base, first_limit, repeats);
    let gap_tol = base.mip_gap.unwrap_or(1e-4);
    let t_base = median(&mut times);
    let mk = |name: &str, p: &Params, mut times: Vec<f64>, s: &Solution, base_obj: Option<f64>, t_ref: f64| -> Trial {
        let time = median(&mut times);
        let ok = matches!(s.status, Status::Optimal | Status::Infeasible | Status::Unbounded);
        Trial {
            name: name.into(),
            params: p.clone(),
            times,
            time,
            status: s.status,
            objective: s.objective,
            gap: s.gap,
            same_result: ok && same(s.objective, base_obj, m.is_mip(), gap_tol),
            speedup: if time > 0.0 { t_ref / time } else { 1.0 },
        }
    };
    let baseline = mk("Valores por defecto", &base, times, &sol, sol.objective, t_base);
    let cands = candidates(m, &base);
    // cada prueba tiene como tope 1.5 veces el tiempo por defecto (no sirve una más lenta)
    let run_limit = (t_base * 1.5 + 0.05).min(budget);
    let mut trials = vec![];
    for (name, p) in cands {
        let left = budget - t0.elapsed().as_secs_f64();
        if left <= run_limit * repeats as f64 * 0.5 {
            break;
        }
        let (times, s) = run(m, &p, run_limit.min(left / repeats as f64), repeats);
        trials.push(mk(&name, &p, times, &s, baseline.objective, t_base));
    }
    trials.sort_by(|a, b| (!a.same_result).cmp(&!b.same_result).then(a.time.partial_cmp(&b.time).unwrap_or(std::cmp::Ordering::Equal)));
    // la mejor: la más rápida con el mismo resultado y al menos un 10 % mejor (lo demás es ruido);
    // en modelos que se resuelven en un instante no hay nada que ganar
    let quick = t_base < 0.05;
    let pick = if quick { None } else { trials.iter().find(|t| t.same_result && t.speedup >= 1.1).cloned() };
    let (best, best_name, best_speedup) = match &pick {
        Some(t) => (t.params.clone(), t.name.clone(), t.speedup),
        None => (base.clone(), "Valores por defecto".into(), 1.0),
    };
    let summary = match &pick {
        Some(t) => format!(
            "Con «{}» el modelo se resuelve {:.1} veces más rápido ({:.3} s frente a {:.3} s) y con el mismo resultado.",
            t.name, t.speedup, t.time, t_base
        ),
        None if quick => format!("El modelo se resuelve en {:.4} s con los valores por defecto: no hace falta ajustar los parámetros.", t_base),
        None => format!("Los valores por defecto ya son los mejores para este modelo ({:.3} s); ninguna otra combinación fue al menos un 10 % más rápida.", t_base),
    };
    let mut trials_all = vec![baseline.clone()];
    trials_all.extend(trials);
    TuneReport {
        solver: super::highs_version(),
        is_mip: m.is_mip(),
        cols: m.cols.len(),
        rows: m.rows.len(),
        budget,
        elapsed: t0.elapsed().as_secs_f64(),
        repeats,
        run_limit,
        baseline,
        trials: trials_all,
        best,
        best_name,
        best_speedup,
        summary,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solver::{Col, Kind, Row};

    #[test]
    fn ajusta_un_entero_pequeno() {
        // mochila con 30 objetos
        let mut m = Model { maximize: true, ..Default::default() };
        let mut coefs = vec![];
        for j in 0..30 {
            let w = (7 + (j * 13) % 29) as f64;
            m.cols.push(Col { name: format!("x{j}"), cost: w + (j % 5) as f64, lb: 0.0, ub: 1.0, kind: Kind::Binary });
            coefs.push((j, w));
        }
        m.rows.push(Row { name: "cap".into(), lo: f64::NEG_INFINITY, hi: 150.0, coefs });
        let r = tune(&m, &Params::default(), 6.0, 2);
        assert!(r.trials.len() >= 3, "{}", r.trials.len());
        assert!(r.trials.iter().filter(|t| t.same_result).count() >= 2);
        assert!(r.elapsed < 12.0);
        assert!(!r.summary.is_empty());
    }
}
