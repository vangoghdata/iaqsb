//! Archivos .CPM de WinQSB (PERT-CPM), según las rutinas de carga y guardado
//! del programa original:
//!
//! ```text
//! CPM|PERT  nombre  unidad  nActividades  Matrix|Graphic
//! CPM:  -1  fCompresión  fCostoNormal  fCostoCompresión  fCostoReal  f%Terminado
//! PERT: nombre de la distribución ("3-Time estimate", "Normal", ..., "Mixed")
//! cabecera de la hoja (se ignora al leer)
//! nº  nombre  predecesoras  [columnas según el tipo]        × nActividades
//! 0 | 1 y después "i  fila  columna" × nActividades   (posiciones del gráfico)
//! ```
//!
//! CPM guarda el tiempo normal y, si su indicador es verdadero, cada columna
//! opcional en ese orden. PERT guarda los parámetros de la distribución; con
//! «Mixed», cada fila lleva su distribución y tres parámetros; la discreta
//! guarda el número de valores y la lista "v/p,v/p,...".

use super::dist::Dist;
use super::{Activity, PertModel, PertType};
use iaqsb_core::winqsb::{num, opt, parse_num, Reader, Writer};
use iaqsb_core::ModError;

fn flag(s: &str) -> bool {
    let t = s.trim().to_ascii_uppercase();
    t == "TRUE" || t == "VERDADERO" || parse_num(&t).map(|x| x != 0.0).unwrap_or(false)
}

fn preds(s: &str) -> Vec<String> {
    s.split([',', ';']).map(str::trim).filter(|x| !x.is_empty()).map(str::to_string).collect()
}

fn val(s: &str) -> Option<f64> {
    parse_num(s).filter(|x| x.is_finite())
}

fn default_name(i: usize, n: usize) -> String {
    if n <= 26 {
        ((b'A' + i as u8) as char).to_string()
    } else {
        format!("A{}", i + 1)
    }
}

pub fn read(bytes: &[u8]) -> Result<PertModel, ModError> {
    let mut r = Reader::new(bytes);
    let mut h = r.line("la cabecera")?;
    let tag = h.next().trim().to_ascii_uppercase();
    let problem_type = if tag.starts_with("CPM") {
        PertType::Deterministic
    } else if tag.starts_with("PER") {
        PertType::Probabilistic
    } else {
        return Err(ModError::Format("no es un archivo de PERT-CPM (la primera línea debe empezar por CPM o PERT)".into()));
    };
    let name = h.next().trim().to_string();
    let mut unit = h.next().trim().to_string();
    if unit.is_empty() {
        unit = "week".into();
    }
    if unit.len() > 1 && unit.to_ascii_uppercase().ends_with('S') {
        unit.pop();
    }
    let n = parse_num(&h.next()).unwrap_or(0.0);
    if !(1.0..=100_000.0).contains(&n) {
        return Err(ModError::Format("el número de actividades no es válido".into()));
    }
    let n = n as usize;
    let graphic = h.next().trim().to_ascii_uppercase().starts_with('G');

    let mut l2 = r.line("la segunda línea")?;
    let mut flags = [true, false, false, false, false, false];
    let mut td = Dist::ThreeTime;
    match problem_type {
        PertType::Deterministic => {
            for f in flags.iter_mut() {
                *f = flag(&l2.next());
            }
            flags[0] = true;
        }
        PertType::Probabilistic => {
            let s = l2.next();
            td = Dist::parse(&s).ok_or_else(|| ModError::Format(format!("distribución desconocida: {s}")))?;
        }
    }
    r.line("la cabecera de la hoja")?;
    let mut activities = Vec::with_capacity(n);
    for i in 0..n {
        let mut l = r.line(&format!("la actividad {}", i + 1))?;
        l.next();
        let nm = l.next().trim().to_string();
        let mut a = Activity {
            name: if nm.is_empty() { default_name(i, n) } else { nm },
            predecessors: preds(&l.next()),
            ..Default::default()
        };
        match problem_type {
            PertType::Deterministic => {
                a.normal_time = Some(val(&l.next()).unwrap_or(0.0));
                let slots: [&mut Option<f64>; 5] = [&mut a.crash_time, &mut a.normal_cost, &mut a.crash_cost, &mut a.actual_cost, &mut a.percent_complete];
                for (k, slot) in slots.into_iter().enumerate() {
                    if flags[k + 1] {
                        *slot = val(&l.next());
                    }
                }
            }
            PertType::Probabilistic => {
                let d = if td == Dist::Mixed {
                    let s = l.next();
                    let d = Dist::parse(&s).filter(|d| *d != Dist::Mixed).unwrap_or(Dist::ThreeTime);
                    a.distribution = Some(d);
                    d
                } else {
                    td
                };
                if d == Dist::Discrete {
                    a.p1 = val(&l.next());
                    a.discrete = Some(l.next().trim().to_string()).filter(|s| !s.is_empty());
                    if td == Dist::Mixed {
                        a.p3 = val(&l.next());
                    }
                } else {
                    let k = if td == Dist::Mixed { 3 } else { d.params().len() };
                    let mut p = [None; 3];
                    for slot in p.iter_mut().take(k) {
                        *slot = val(&l.next());
                    }
                    // solo se guardan los parámetros que usa la distribución
                    for (j, slot) in p.iter_mut().enumerate() {
                        if j >= d.params().len() {
                            *slot = None;
                        }
                    }
                    [a.p1, a.p2, a.p3] = p;
                }
            }
        }
        activities.push(a);
    }
    let mut positions = None;
    if let Some(mut l) = r.try_line() {
        if flag(&l.next()) {
            let mut p = vec![(0i64, 0i64); n];
            for _ in 0..n {
                if let Some(mut q) = r.try_line() {
                    let i = parse_num(&q.next()).unwrap_or(0.0) as usize;
                    let row = parse_num(&q.next()).unwrap_or(0.0) as i64;
                    let col = parse_num(&q.next()).unwrap_or(0.0) as i64;
                    if i >= 1 && i <= n {
                        p[i - 1] = (row, col);
                    }
                }
            }
            positions = Some(p);
        }
    }
    Ok(PertModel {
        name: if name.is_empty() { "Project Problem".into() } else { name },
        problem_type,
        time_unit: unit,
        time_distribution: if problem_type == PertType::Probabilistic { td } else { Dist::ThreeTime },
        activities,
        positions,
        graphic,
    })
}

/// Nombres de los parámetros en inglés, como en la hoja de WinQSB.
fn param_header(d: Dist) -> &'static [&'static str] {
    match d {
        Dist::ThreeTime => &["Optimistic time (a)", "Most likely time (m)", "Pessimistic time (b)"],
        Dist::Normal | Dist::Arbitrary => &["Activity mean time (u)", "Standard deviation (s>0)"],
        Dist::Uniform => &["Lower limit (a)", "Upper limit (b)"],
        Dist::Triangular => &["Lower limit (a)", "Shape parameter (mode) (b)", "Upper limit (c)"],
        Dist::Exponential => &["Location parameter (a)", "Scale parameter (mean) (b>0)"],
        Dist::Beta => &["Scale parameter (b>0)", "Shape parameter (v>0)", "Shape parameter (w>0)"],
        Dist::Gamma | Dist::Weibull | Dist::PowerFunction => &["Location parameter (a)", "Scale parameter (b>0)", "Shape parameter (c>0)"],
        Dist::Erlang => &["Location parameter (a)", "Scale parameter (b>0)", "Shape parameter (c>=1, integer)"],
        Dist::Lognormal => &["Mean of log(x) (u)", "Std. dev. of log(x) (s>0)"],
        Dist::Laplace => &["Location parameter (mean) (a)", "Scale parameter (b>0)"],
        Dist::Pareto => &["Location parameter (a>0)", "Shape parameter (c>0)"],
        Dist::Binomial => &["Sample size (n>0)", "Probability of success (p>0)"],
        Dist::Poisson => &["Activity mean time (u)"],
        Dist::Geometric => &["Probability of success (p>0)"],
        Dist::Hypergeometric => &["Population size (N>0)", "Number of target items (K>0,K<=N)", "Sample size (n>0)"],
        Dist::Constant => &["Constant time"],
        Dist::Discrete => &["Number of discrete values", "Discrete values (v/p,v/p,...)"],
        Dist::Mixed => &[],
    }
}

fn b(x: bool) -> &'static str {
    if x {
        "-1"
    } else {
        "0"
    }
}

pub fn write(m: &PertModel) -> Vec<u8> {
    let mut w = Writer::new();
    let n = m.activities.len();
    let tag = if m.problem_type == PertType::Deterministic { "CPM" } else { "PERT" };
    let n_s = n.to_string();
    w.row(&[tag, &m.name, &m.time_unit, &n_s, if m.graphic { "Graphic" } else { "Matrix" }]);
    let pred = |a: &Activity| a.predecessors.iter().map(|s| s.trim()).filter(|s| !s.is_empty()).collect::<Vec<_>>().join(",");
    match m.problem_type {
        PertType::Deterministic => {
            let f = [
                true,
                m.has(|a| a.crash_time),
                m.has(|a| a.normal_cost),
                m.has(|a| a.crash_cost),
                m.has(|a| a.actual_cost),
                m.has(|a| a.percent_complete),
            ];
            w.row(&f.map(b));
            let names = ["Normal Time", "Crash Time", "Normal Cost", "Crash Cost", "Actual Cost", "Percent Complete"];
            let mut head = vec!["Activity No.", "Activity Name", "Immediate Predecessor"];
            head.extend((0..6).filter(|&k| f[k]).map(|k| names[k]));
            w.row(&head);
            for (i, a) in m.activities.iter().enumerate() {
                let mut row = vec![(i + 1).to_string(), a.name.clone(), pred(a), num(a.normal_time.unwrap_or(0.0))];
                let vals = [a.crash_time, a.normal_cost, a.crash_cost, a.actual_cost, a.percent_complete];
                for k in 0..5 {
                    if f[k + 1] {
                        row.push(opt(vals[k]));
                    }
                }
                w.row(&row);
            }
        }
        PertType::Probabilistic => {
            let td = m.time_distribution;
            w.row(&[td.winqsb()]);
            if td == Dist::Mixed {
                w.row(&[
                    "Activity No.",
                    "Activity Name",
                    "Immediate Predecessor",
                    "Activity Time Distribution",
                    "Parameter 1",
                    "Parameter 2",
                    "Parameter 3",
                ]);
                for (i, a) in m.activities.iter().enumerate() {
                    let d = a.distribution.filter(|d| *d != Dist::Mixed).unwrap_or(Dist::ThreeTime);
                    let mut row = vec![(i + 1).to_string(), a.name.clone(), pred(a), d.winqsb().to_string(), opt(a.p1)];
                    if d == Dist::Discrete {
                        row.push(a.discrete.clone().unwrap_or_default());
                    } else {
                        row.push(opt(a.p2));
                    }
                    row.push(opt(a.p3));
                    w.row(&row);
                }
            } else {
                let mut head = vec!["Activity No.", "Activity Name", "Immediate Predecessor"];
                head.extend(param_header(td));
                w.row(&head);
                for (i, a) in m.activities.iter().enumerate() {
                    let mut row = vec![(i + 1).to_string(), a.name.clone(), pred(a)];
                    if td == Dist::Discrete {
                        row.push(opt(a.p1));
                        row.push(a.discrete.clone().unwrap_or_default());
                    } else {
                        let p = [a.p1, a.p2, a.p3];
                        row.extend((0..td.params().len()).map(|k| opt(p[k])));
                    }
                    w.row(&row);
                }
            }
        }
    }
    match &m.positions {
        Some(p) if p.len() == n => {
            w.row(&["1"]);
            for (i, (r, c)) in p.iter().enumerate() {
                w.row(&[(i + 1).to_string(), r.to_string(), c.to_string()]);
            }
        }
        _ => {
            w.row(&["0"]);
        }
    }
    w.bytes()
}
