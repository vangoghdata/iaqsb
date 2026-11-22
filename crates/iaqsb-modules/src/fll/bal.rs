//! Balanceo de líneas: diez reglas, búsqueda del mejor (ramificación y acotamiento) y COMSOAL.

use super::{BalMethod, BalRule, FllModel, FllOptions};
use iaqsb_core::*;
use rand::{RngExt, SeedableRng};
use serde_json::json;

struct Line {
    names: Vec<String>,
    t: Vec<f64>,
    iso: Vec<bool>,
    succ: Vec<Vec<usize>>,
    pred: Vec<Vec<usize>>,
    /// Todos los seguidores (cierre transitivo).
    follow: Vec<Vec<bool>>,
    c: f64,
}

#[derive(Clone)]
struct Station {
    tasks: Vec<usize>,
    time: f64,
    ops: usize,
}

fn key(l: &Line, r: BalRule, i: usize, rnd: &[f64]) -> f64 {
    let nf = l.follow[i].iter().filter(|x| **x).count() as f64;
    let np = (0..l.t.len()).filter(|&k| l.follow[k][i]).count() as f64;
    // valor más alto = más prioridad
    match r {
        BalRule::Rpw => l.t[i] + (0..l.t.len()).filter(|&k| l.follow[i][k]).map(|k| l.t[k]).sum::<f64>(),
        BalRule::Lot => l.t[i],
        BalRule::Sot => -l.t[i],
        BalRule::MostFollowers => nf,
        BalRule::LeastFollowers => -nf,
        BalRule::MostImmediate => l.succ[i].len() as f64,
        BalRule::LeastImmediate => -(l.succ[i].len() as f64),
        BalRule::MostPredecessors => np,
        BalRule::SmallestNumber => -(i as f64),
        BalRule::Random => rnd[i],
    }
}

/// Carga estación por estación eligiendo con `pick` entre las tareas que caben.
fn load(l: &Line, pick: &mut dyn FnMut(&[usize], f64) -> usize, trace: &mut Vec<(usize, f64, String, String)>) -> Vec<Station> {
    let n = l.t.len();
    let mut done = vec![false; n];
    let mut left = n;
    let mut st: Vec<Station> = vec![];
    let mut cur = Station { tasks: vec![], time: 0.0, ops: 1 };
    while left > 0 {
        let avail: Vec<usize> = (0..n).filter(|&i| !done[i] && l.pred[i].iter().all(|&p| done[p])).collect();
        let fits: Vec<usize> = avail
            .iter()
            .copied()
            .filter(|&i| {
                if cur.tasks.is_empty() {
                    true
                } else {
                    !l.iso[i] && !cur.tasks.iter().any(|&k| l.iso[k]) && cur.time + l.t[i] <= l.c + 1e-9 && cur.ops == 1
                }
            })
            .collect();
        if fits.is_empty() {
            st.push(std::mem::replace(&mut cur, Station { tasks: vec![], time: 0.0, ops: 1 }));
            continue;
        }
        let i = pick(&fits, l.c - cur.time);
        if trace.len() < 60 {
            trace.push((st.len() + 1, l.c - cur.time, fits.iter().map(|&k| l.names[k].clone()).collect::<Vec<_>>().join(", "), l.names[i].clone()));
        }
        cur.tasks.push(i);
        cur.time += l.t[i];
        if l.t[i] > l.c + 1e-9 {
            cur.ops = (l.t[i] / l.c - 1e-9).ceil() as usize;
        }
        done[i] = true;
        left -= 1;
    }
    if !cur.tasks.is_empty() {
        st.push(cur);
    }
    st
}

fn score(st: &[Station], c: f64) -> (usize, f64) {
    let ops: usize = st.iter().map(|s| s.ops).sum();
    let smooth: f64 = st.iter().map(|s| (c * s.ops as f64 - s.time).powi(2)).sum();
    (ops, smooth)
}

fn better(a: &[Station], b: &[Station], c: f64) -> bool {
    let (oa, sa) = score(a, c);
    let (ob, sb) = score(b, c);
    oa < ob || (oa == ob && sa < sb - 1e-9)
}

/// Ramificación y acotamiento por estaciones con cargas maximales.
fn best_bud(l: &Line, incumbent: Vec<Station>, node_limit: usize) -> (Vec<Station>, bool) {
    let n = l.t.len();
    let mut best = incumbent;
    let mut nodes = 0usize;
    let mut complete = true;
    fn rec(l: &Line, done: &mut Vec<bool>, stations: &mut Vec<Station>, best: &mut Vec<Station>, nodes: &mut usize, limit: usize, complete: &mut bool) {
        let n = l.t.len();
        *nodes += 1;
        if *nodes > limit {
            *complete = false;
            return;
        }
        let rem: f64 = (0..n).filter(|&i| !done[i]).map(|i| l.t[i]).sum();
        if rem <= 1e-12 && (0..n).all(|i| done[i]) {
            if better(stations, best, l.c) {
                *best = stations.clone();
            }
            return;
        }
        let used: usize = stations.iter().map(|s| s.ops).sum();
        let lb = used + (rem / l.c - 1e-9).ceil().max(1.0) as usize;
        if lb > score(best, l.c).0 || (lb == score(best, l.c).0 && score(best, l.c).1 <= 1e-9) {
            return;
        }
        // cargas maximales de la próxima estación
        let mut loads: Vec<Vec<usize>> = vec![];
        let avail0: Vec<usize> = (0..n).filter(|&i| !done[i] && l.pred[i].iter().all(|&p| done[p])).collect();
        // tareas que van solas (aisladas o más largas que el ciclo)
        for &i in &avail0 {
            if l.iso[i] || l.t[i] > l.c + 1e-9 {
                loads.push(vec![i]);
            }
        }
        fn grow(l: &Line, done: &mut Vec<bool>, cur: &mut Vec<usize>, time: f64, min_idx: usize, out: &mut Vec<Vec<usize>>, cap: usize) {
            if out.len() >= cap {
                return;
            }
            let n = l.t.len();
            let cand: Vec<usize> = (0..n).filter(|&i| !done[i] && !l.iso[i] && l.t[i] <= l.c + 1e-9 && l.pred[i].iter().all(|&p| done[p]) && time + l.t[i] <= l.c + 1e-9).collect();
            if cand.is_empty() {
                if !cur.is_empty() {
                    out.push(cur.clone());
                }
                return;
            }
            let mut extended = false;
            let _ = min_idx;
            for &i in &cand {
                // conjuntos en orden creciente (las tareas que se liberan después se agregan igual)
                if cur.last().map(|&last| i < last && l.pred[i].iter().all(|p| !cur.contains(p))).unwrap_or(false) {
                    continue;
                }
                extended = true;
                done[i] = true;
                cur.push(i);
                grow(l, done, cur, time + l.t[i], i + 1, out, cap);
                cur.pop();
                done[i] = false;
            }
            if !extended && !cur.is_empty() {
                out.push(cur.clone());
            }
        }
        let mut cur = vec![];
        grow(l, done, &mut cur, 0.0, 0, &mut loads, 5000);
        loads.sort_by(|a, b| {
            let ta: f64 = a.iter().map(|&i| l.t[i]).sum();
            let tb: f64 = b.iter().map(|&i| l.t[i]).sum();
            tb.partial_cmp(&ta).unwrap()
        });
        let mut seen = std::collections::BTreeSet::new();
        loads.retain(|ld| {
            let mut k = ld.clone();
            k.sort();
            seen.insert(k)
        });
        for ld in loads {
            let time: f64 = ld.iter().map(|&i| l.t[i]).sum();
            let ops = if ld.len() == 1 && l.t[ld[0]] > l.c + 1e-9 { (l.t[ld[0]] / l.c - 1e-9).ceil() as usize } else { 1 };
            for &i in &ld {
                done[i] = true;
            }
            stations.push(Station { tasks: ld.clone(), time, ops });
            rec(l, done, stations, best, nodes, limit, complete);
            stations.pop();
            for &i in &ld {
                done[i] = false;
            }
            if *nodes > limit {
                return;
            }
        }
    }
    let mut done = vec![false; n];
    let mut stations = vec![];
    rec(l, &mut done, &mut stations, &mut best, &mut nodes, node_limit, &mut complete);
    (best, complete)
}

pub fn solve(m: &FllModel, o: &FllOptions) -> Result<Report, ModError> {
    let n = m.tasks.len();
    if n == 0 {
        return invalid("no hay tareas");
    }
    check_names("tareas", &m.tasks.iter().map(|t| &t.name).collect::<Vec<_>>())?;
    let names: Vec<String> = m.tasks.iter().map(|t| t.name.clone()).collect();
    let mut succ = vec![vec![]; n];
    let mut pred = vec![vec![]; n];
    for (i, t) in m.tasks.iter().enumerate() {
        if t.time < 0.0 {
            return invalid(format!("{}: el tiempo no puede ser negativo", t.name));
        }
        for s in t.successors.split([',', ';']).map(str::trim).filter(|s| !s.is_empty()) {
            let j = names
                .iter()
                .position(|x| x.eq_ignore_ascii_case(s))
                .or_else(|| s.parse::<usize>().ok().filter(|k| *k >= 1 && *k <= n).map(|k| k - 1))
                .ok_or_else(|| ModError::Invalid(format!("{}: el sucesor «{s}» no existe", t.name)))?;
            if j == i {
                return invalid(format!("{} no puede ser sucesora de sí misma", t.name));
            }
            if !succ[i].contains(&j) {
                succ[i].push(j);
                pred[j].push(i);
            }
        }
    }
    // cierre transitivo y ciclos
    let mut follow = vec![vec![false; n]; n];
    for i in 0..n {
        let mut stack = succ[i].clone();
        while let Some(k) = stack.pop() {
            if !follow[i][k] {
                follow[i][k] = true;
                stack.extend(succ[k].iter().copied());
            }
        }
        if follow[i][i] {
            return invalid(format!("las precedencias tienen un ciclo que pasa por {}", names[i]));
        }
    }
    let c = match (m.cycle_time, m.time_available, m.production) {
        (Some(c), _, _) if c > 0.0 => c,
        (_, Some(t), Some(q)) if t > 0.0 && q > 0.0 => t / q,
        _ => return invalid("indica el tiempo de ciclo, o el tiempo disponible y la producción deseada"),
    };
    let l = Line { names: names.clone(), t: m.tasks.iter().map(|t| t.time).collect(), iso: m.tasks.iter().map(|t| t.isolated).collect(), succ, pred, follow, c };
    let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(o.seed);
    let rnd: Vec<f64> = (0..n).map(|_| rng.random()).collect();
    let rnd2: Vec<f64> = (0..n).map(|_| rng.random()).collect();
    let by_rule = |r: BalRule, tie: BalRule, trace: &mut Vec<(usize, f64, String, String)>| {
        load(
            &l,
            &mut |fits: &[usize], _rem: f64| *fits.iter().max_by(|&&a, &&b| key(&l, r, a, &rnd).partial_cmp(&key(&l, r, b, &rnd)).unwrap().then(key(&l, tie, a, &rnd2).partial_cmp(&key(&l, tie, b, &rnd2)).unwrap()).then(b.cmp(&a))).unwrap(),
            trace,
        )
    };
    let mut trace = vec![];
    let mut cmp = vec![];
    for r in BalRule::ALL {
        let mut tmp = vec![];
        let s = by_rule(r, o.tie_breaker, &mut tmp);
        cmp.push((r.label().to_string(), s));
    }
    let (stations, method_txt, proven) = match o.method {
        BalMethod::Heuristic => (by_rule(o.rule, o.tie_breaker, &mut trace), format!("regla {} (desempate: {})", o.rule.label(), o.tie_breaker.label()), false),
        BalMethod::Comsoal => {
            let k = o.generations.clamp(1, 1_000_000);
            let mut best: Option<Vec<Station>> = None;
            for _ in 0..k {
                let mut tmp = vec![];
                let s = load(&l, &mut |fits: &[usize], _| fits[rng.random_range(0..fits.len())], &mut tmp);
                if best.as_ref().map(|b| better(&s, b, c)).unwrap_or(true) {
                    best = Some(s);
                }
            }
            (best.unwrap(), format!("COMSOAL ({k} soluciones aleatorias)"), false)
        }
        BalMethod::BestBud => {
            let inc = cmp.iter().map(|x| x.1.clone()).reduce(|a, b| if better(&b, &a, c) { b } else { a }).unwrap();
            let (s, complete) = best_bud(&l, inc, 2_000_000);
            (s, if complete { "búsqueda del mejor (óptimo comprobado)".to_string() } else { "búsqueda del mejor (con límite de nodos)".to_string() }, complete)
        }
    };
    let total: f64 = l.t.iter().sum();
    let (ops, _) = score(&stations, c);
    let tmin = (total / c - 1e-9).ceil() as usize;
    let eff = total / (ops as f64 * c);
    let idle = ops as f64 * c - total;
    let u = &m.time_unit;
    let mut stt = Table::new(Some("Estaciones"), &["Estación", "Tareas", "Operadores", &format!("Tiempo ({u})"), "Tiempo ocioso", "Utilización (%)"]).left(1);
    for (k, s) in stations.iter().enumerate() {
        let cap = c * s.ops as f64;
        stt.row(cells![k + 1, s.tasks.iter().map(|&i| names[i].as_str()).collect::<Vec<_>>().join(", "), s.ops, tidy(s.time), tidy(cap - s.time), tidy(100.0 * s.time / cap)]);
    }
    stt.row_tone(cells!["Total", Cell::Empty, ops, tidy(total), tidy(idle), tidy(100.0 * eff)], Tone::Total);
    let mut ct = Table::new(Some("Comparación de las reglas"), &["Regla", "Estaciones", "Operadores", "Eficiencia (%)"]).left(0);
    for (name, s) in &cmp {
        let (o2, _) = score(s, c);
        ct.row_tone(cells![name, s.len(), o2, tidy(100.0 * total / (o2 as f64 * c))], if o2 == ops { Tone::Highlight } else { Tone::Normal });
    }
    // red de precedencias coloreada por estación
    let mut lvl = vec![0usize; n];
    for _ in 0..n {
        for i in 0..n {
            for &j in &l.succ[i] {
                if lvl[j] <= lvl[i] {
                    lvl[j] = lvl[i] + 1;
                }
            }
        }
    }
    let maxl = *lvl.iter().max().unwrap();
    let mut st_of = vec![0usize; n];
    for (k, s) in stations.iter().enumerate() {
        for &i in &s.tasks {
            st_of[i] = k;
        }
    }
    let tones = [Tone::Accent, Tone::Highlight, Tone::Normal, Tone::Good, Tone::Bad];
    let mut nodes = vec![];
    for lv in 0..=maxl {
        let at: Vec<usize> = (0..n).filter(|&i| lvl[i] == lv).collect();
        for (k, &i) in at.iter().enumerate() {
            nodes.push(NetNode {
                id: format!("t{i}"),
                label: names[i].clone(),
                x: if maxl == 0 { 0.5 } else { lv as f64 / maxl as f64 },
                y: (k as f64 + 1.0) / (at.len() as f64 + 1.0),
                note: Some(format!("{} · E{}", fmt(l.t[i]), st_of[i] + 1)),
                tone: tones[st_of[i] % tones.len()],
            });
        }
    }
    let edges = (0..n).flat_map(|i| l.succ[i].iter().map(move |&j| (i, j))).map(|(i, j)| NetEdge { from: format!("t{i}"), to: format!("t{j}"), label: None, tone: Tone::Normal }).collect();
    let net = Block::Network(Network { title: Some("Precedencias (E = estación asignada)".into()), directed: true, nodes, edges });
    let mut stp = vec![];
    if o.steps {
        let mut v = vec![step(
            "Tiempo de ciclo y mínimo teórico",
            format!("Ciclo C = {} {u}. Suma de tiempos = {}; mínimo teórico de estaciones = ⌈{} / {}⌉ = {}.", fmt(tidy(c)), fmt(tidy(total)), fmt(tidy(total)), fmt(tidy(c)), tmin),
            vec![],
        )];
        if o.method == BalMethod::Heuristic {
            let mut wt = Table::new(Some("Prioridad de cada tarea"), &["Tarea", "Tiempo", "Seguidoras", "Peso posicional"]).left(0);
            for i in 0..n {
                wt.row(cells![&names[i], l.t[i], l.follow[i].iter().filter(|x| **x).count(), tidy(key(&l, BalRule::Rpw, i, &rnd))]);
            }
            let mut tt = Table::new(Some("Asignación"), &["Estación", "Tiempo libre", "Tareas que caben", "Elegida"]).left(2).left(3);
            for (s, rem, f, ch) in &trace {
                tt.row(cells![*s, tidy(*rem), f, ch]);
            }
            v.push(step("Prioridades", "La regla ordena las tareas; el peso posicional es el tiempo de la tarea más el de todas las que la siguen.", vec![wt.block()]));
            v.push(step("Carga de estaciones", "Se abre una estación y se le asigna, entre las tareas cuyas predecesoras ya están asignadas y que caben en el tiempo libre, la de mayor prioridad. Cuando no cabe ninguna, se abre otra estación.", vec![tt.block()]));
        } else if o.method == BalMethod::BestBud {
            v.push(step("Búsqueda del mejor", "Se exploran las cargas maximales de cada estación, de la más llena a la menos, y se descartan las ramas cuya cota (estaciones usadas + ⌈tiempo restante / C⌉) no mejora la mejor solución conocida.", vec![]));
        } else {
            v.push(step("COMSOAL", "Se generan muchas secuencias eligiendo al azar entre las tareas que caben, y se queda la de menos estaciones (y, a igualdad, el tiempo ocioso mejor repartido).", vec![]));
        }
        v.push(step("Medidas", format!("Eficiencia = suma de tiempos / (operadores × C) = {} %; retraso de balance = {} %.", fmt(tidy(100.0 * eff)), fmt(tidy(100.0 * (1.0 - eff)))), vec![stt.clone().block()]));
        stp.push(steps(v));
    }
    Ok(Report::new("fll", &m.name, if proven { Status::Optimal } else { Status::Feasible }, format!("{} estaciones ({} operadores) con ciclo {}; eficiencia {} % — {}.", stations.len(), ops, fmt(tidy(c)), fmt(tidy(100.0 * eff)), method_txt))
        .section(
            "solucion",
            "Solución",
            vec![
                metrics(vec![
                    metric_main("Estaciones", stations.len()),
                    metric("Mínimo teórico", tmin),
                    metric(format!("Tiempo de ciclo ({u})"), tidy(c)),
                    metric("Eficiencia (%)", tidy(100.0 * eff)),
                    metric("Retraso de balance (%)", tidy(100.0 * (1.0 - eff))),
                ]),
                stt.block(),
            ],
        )
        .section("red", "Precedencias", vec![net])
        .section("comparacion", "Comparación", vec![ct.block()])
        .section("pasos", "Paso a paso", stp)
        .data(json!({
            "cycle_time": tidy(c),
            "stations": stations.iter().map(|s| json!({"tasks": s.tasks.iter().map(|&i| names[i].clone()).collect::<Vec<_>>(), "time": tidy(s.time), "operators": s.ops})).collect::<Vec<_>>(),
            "operators": ops,
            "theoretical_minimum": tmin,
            "efficiency": tidy(eff),
        })))
}
