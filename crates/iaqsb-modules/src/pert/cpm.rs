//! Red del proyecto (actividades en los nodos), pases hacia adelante y hacia
//! atrás, holguras, rutas críticas y sus vistas (tabla, red, Gantt y pasos).

use super::{cost, crash, plural, Crashing, PertModel, PertOptions, TimeBasis};
use iaqsb_core::layout;
use iaqsb_core::report::*;
use iaqsb_core::{cells, ModError};
use serde_json::json;

/// Precedencias resueltas y orden topológico.
pub struct Net {
    pub n: usize,
    pub names: Vec<String>,
    pub preds: Vec<Vec<usize>>,
    pub succs: Vec<Vec<usize>>,
    pub order: Vec<usize>,
}

pub fn build(m: &PertModel) -> Result<Net, ModError> {
    let n = m.activities.len();
    let names: Vec<String> = m.activities.iter().map(|a| a.name.trim().to_string()).collect();
    let mut preds = vec![vec![]; n];
    for (j, a) in m.activities.iter().enumerate() {
        for raw in &a.predecessors {
            for s in raw.split([',', ';']).map(str::trim).filter(|s| !s.is_empty()) {
                let i = names
                    .iter()
                    .position(|x| x.eq_ignore_ascii_case(s))
                    .or_else(|| s.parse::<usize>().ok().filter(|k| *k >= 1 && *k <= n).map(|k| k - 1))
                    .ok_or_else(|| ModError::Invalid(format!("la predecesora «{s}» de la actividad {} no existe", names[j])))?;
                if i == j {
                    return Err(ModError::Invalid(format!("la actividad {} no puede ser predecesora de sí misma", names[j])));
                }
                if !preds[j].contains(&i) {
                    preds[j].push(i);
                }
            }
        }
    }
    let mut succs = vec![vec![]; n];
    for j in 0..n {
        for &i in &preds[j] {
            succs[i].push(j);
        }
    }
    // orden topológico (Kahn), estable según el orden de entrada
    let mut indeg: Vec<usize> = preds.iter().map(|p| p.len()).collect();
    let mut order = Vec::with_capacity(n);
    let mut ready: Vec<usize> = (0..n).filter(|&i| indeg[i] == 0).collect();
    while !ready.is_empty() {
        ready.sort_unstable_by(|a, b| b.cmp(a));
        let u = ready.pop().unwrap();
        order.push(u);
        for &v in &succs[u] {
            indeg[v] -= 1;
            if indeg[v] == 0 {
                ready.push(v);
            }
        }
    }
    if order.len() < n {
        // buscar un ciclo entre las actividades que quedaron
        let left: Vec<bool> = (0..n).map(|i| indeg[i] > 0).collect();
        let start = (0..n).find(|&i| left[i]).unwrap();
        let mut seen = vec![usize::MAX; n];
        let mut path = vec![start];
        let mut u = start;
        seen[u] = 0;
        loop {
            let p = *preds[u].iter().find(|&&p| left[p]).unwrap();
            if seen[p] != usize::MAX {
                // el camino sigue a las predecesoras: al revés queda en orden de precedencia
                let mut ids: Vec<usize> = path[seen[p]..].iter().rev().copied().collect();
                // empezar por la que va antes en la hoja
                let k = (0..ids.len()).min_by_key(|&k| ids[k]).unwrap();
                ids.rotate_left(k);
                ids.push(ids[0]);
                let cyc: Vec<&str> = ids.iter().map(|&i| names[i].as_str()).collect();
                return Err(ModError::Invalid(format!("las precedencias forman un ciclo: {}", cyc.join(" → "))));
            }
            seen[p] = path.len();
            path.push(p);
            u = p;
        }
    }
    Ok(Net { n, names, preds, succs, order })
}

#[derive(Debug, Clone)]
pub struct Sched {
    pub t: Vec<f64>,
    pub es: Vec<f64>,
    pub ef: Vec<f64>,
    pub ls: Vec<f64>,
    pub lf: Vec<f64>,
    pub slack: Vec<f64>,
    pub free: Vec<f64>,
    pub critical: Vec<bool>,
    pub total: f64,
    pub eps: f64,
}

pub fn forward(net: &Net, t: &[f64]) -> (Vec<f64>, Vec<f64>, f64) {
    let mut es = vec![0.0; net.n];
    let mut ef = vec![0.0; net.n];
    for &j in &net.order {
        es[j] = net.preds[j].iter().map(|&i| ef[i]).fold(0.0, f64::max);
        ef[j] = es[j] + t[j];
    }
    let total = ef.iter().cloned().fold(0.0, f64::max);
    (es, ef, total)
}

pub fn schedule(net: &Net, t: &[f64]) -> Sched {
    let (es, ef, total) = forward(net, t);
    let mut ls = vec![0.0; net.n];
    let mut lf = vec![0.0; net.n];
    for &i in net.order.iter().rev() {
        lf[i] = net.succs[i].iter().map(|&j| ls[j]).fold(total, f64::min);
        ls[i] = lf[i] - t[i];
    }
    let eps = 1e-9 * total.abs().max(1.0);
    let slack: Vec<f64> = (0..net.n).map(|i| clean(ls[i] - es[i]).max(0.0)).collect();
    let free: Vec<f64> = (0..net.n).map(|i| clean(net.succs[i].iter().map(|&j| es[j]).fold(total, f64::min) - ef[i]).max(0.0)).collect();
    let critical = slack.iter().map(|s| *s <= eps).collect();
    Sched { t: t.to_vec(), es, ef, ls, lf, slack, free, critical, total, eps }
}

impl Sched {
    /// Arco i→j sobre una ruta crítica (ambas críticas y sin espera entre ellas).
    pub fn tight(&self, i: usize, j: usize) -> bool {
        self.critical[i] && self.critical[j] && (self.ef[i] - self.es[j]).abs() <= self.eps
    }

    /// Todas las rutas críticas (hasta `limit`).
    pub fn critical_paths(&self, net: &Net, limit: usize) -> Vec<Vec<usize>> {
        let mut out = Vec::new();
        let starts: Vec<usize> = (0..net.n).filter(|&i| self.critical[i] && self.es[i].abs() <= self.eps).collect();
        fn dfs(s: &Sched, net: &Net, u: usize, path: &mut Vec<usize>, out: &mut Vec<Vec<usize>>, limit: usize) {
            if out.len() >= limit {
                return;
            }
            path.push(u);
            let next: Vec<usize> = net.succs[u].iter().copied().filter(|&j| s.tight(u, j)).collect();
            if (s.ef[u] - s.total).abs() <= s.eps && next.iter().all(|&j| s.t[j] > s.eps) {
                out.push(path.clone());
            }
            for j in next {
                dfs(s, net, j, path, out, limit);
            }
            path.pop();
        }
        for s in starts {
            let mut path = vec![];
            dfs(self, net, s, &mut path, &mut out, limit);
        }
        out
    }
}

pub fn path_names(net: &Net, p: &[usize]) -> String {
    p.iter().map(|&i| net.names[i].as_str()).collect::<Vec<_>>().join(" → ")
}

/// Tabla de actividades con lo calculado hasta el momento.
/// `fw` / `bw`: qué filas ya tienen pase hacia adelante / hacia atrás.
pub fn table(net: &Net, s: &Sched, title: Option<&str>, fw: &[bool], bw: &[bool], current: Option<usize>, full: bool) -> Table {
    let mut cols = vec!["Actividad", "Predecesoras", "Tiempo", "Inicio temprano", "Fin temprano", "Inicio tardío", "Fin tardío"];
    if full {
        cols.extend(["Holgura total", "Holgura libre", "¿Crítica?"]);
    }
    let mut t = Table::new(title, &cols).left(0).left(1);
    if full {
        t = t.left(9);
    }
    for i in 0..net.n {
        let pred = net.preds[i].iter().map(|&p| net.names[p].as_str()).collect::<Vec<_>>().join(", ");
        let f = |ok: bool, x: f64| if ok { Cell::Num(x) } else { Cell::Empty };
        let mut row = vec![
            Cell::from(&net.names[i]),
            Cell::from(if pred.is_empty() { "—".to_string() } else { pred }),
            Cell::Num(s.t[i]),
            f(fw[i], s.es[i]),
            f(fw[i], s.ef[i]),
            f(bw[i], s.ls[i]),
            f(bw[i], s.lf[i]),
        ];
        if full {
            row.extend([Cell::Num(s.slack[i]), Cell::Num(s.free[i]), Cell::from(if s.critical[i] { "sí" } else { "no" })]);
        }
        let tone = if full && s.critical[i] { Tone::Highlight } else { Tone::Normal };
        t.row_tone(row, tone);
        if current == Some(i) {
            for c in 0..cols.len() {
                t.mark(i, c, Tone::Accent);
            }
        }
    }
    t
}

/// Red de actividades en los nodos, con Inicio y Fin.
pub fn network(m: &PertModel, net: &Net, s: Option<&Sched>, title: &str) -> Block {
    let n = net.n;
    let with_pos = m.positions.as_ref().filter(|p| p.len() == n && p.iter().any(|x| *x != p[0]));
    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    let tone_e = |i: usize, j: usize| match s {
        Some(s) if s.tight(i, j) => Tone::Highlight,
        Some(_) => Tone::Muted,
        None => Tone::Normal,
    };
    let note = |i: usize| -> Option<String> {
        s.map(|s| format!("{} → {}\nt {} · h {}", fmt(s.es[i]), fmt(s.ef[i]), fmt(s.t[i]), fmt(s.slack[i])))
    };
    let tone_n = |i: usize| match s {
        Some(s) if s.critical[i] => Tone::Highlight,
        _ => Tone::Normal,
    };
    for i in 0..n {
        for &j in &net.succs[i] {
            edges.push(NetEdge { from: format!("a{i}"), to: format!("a{j}"), label: None, tone: tone_e(i, j) });
        }
    }
    let pos: Vec<(f64, f64)> = if let Some(p) = with_pos {
        layout::from_grid(p)
    } else {
        // n = Inicio, n+1 = Fin
        let mut e: Vec<(usize, usize)> = edges_idx(net);
        for i in 0..n {
            if net.preds[i].is_empty() {
                e.push((n, i));
            }
            if net.succs[i].is_empty() {
                e.push((i, n + 1));
            }
        }
        layout::layered(n + 2, &e, n)
    };
    for i in 0..n {
        nodes.push(NetNode { id: format!("a{i}"), label: net.names[i].clone(), x: pos[i].0, y: pos[i].1, note: note(i), tone: tone_n(i) });
    }
    if with_pos.is_none() {
        let total = s.map(|s| format!("duración {}", fmt(s.total)));
        nodes.push(NetNode { id: "ini".into(), label: "Inicio".into(), x: pos[n].0, y: pos[n].1, note: None, tone: Tone::Accent });
        nodes.push(NetNode { id: "fin".into(), label: "Fin".into(), x: pos[n + 1].0, y: pos[n + 1].1, note: total, tone: Tone::Accent });
        for i in 0..n {
            let crit = s.map(|s| s.critical[i]).unwrap_or(false);
            if net.preds[i].is_empty() {
                let t = if s.is_none() { Tone::Normal } else if crit && s.unwrap().es[i] <= s.unwrap().eps { Tone::Highlight } else { Tone::Muted };
                edges.push(NetEdge { from: "ini".into(), to: format!("a{i}"), label: None, tone: t });
            }
            if net.succs[i].is_empty() {
                let t = match s {
                    None => Tone::Normal,
                    Some(s) if crit && (s.ef[i] - s.total).abs() <= s.eps => Tone::Highlight,
                    _ => Tone::Muted,
                };
                edges.push(NetEdge { from: format!("a{i}"), to: "fin".into(), label: None, tone: t });
            }
        }
    }
    Block::Network(Network { title: Some(title.into()), directed: true, nodes, edges })
}

fn edges_idx(net: &Net) -> Vec<(usize, usize)> {
    let mut e = vec![];
    for i in 0..net.n {
        for &j in &net.succs[i] {
            e.push((i, j));
        }
    }
    e
}

pub fn gantt(net: &Net, s: &Sched, title: &str, unit: &str, markers: Vec<RefLine>) -> Block {
    let rows = (0..net.n)
        .map(|i| {
            let mut bars = vec![GanttBar {
                start: s.es[i],
                end: s.ef[i],
                label: Some(fmt(s.t[i])),
                tone: if s.critical[i] { Tone::Highlight } else { Tone::Accent },
            }];
            if s.slack[i] > s.eps {
                bars.push(GanttBar { start: s.ef[i], end: s.lf[i], label: Some(format!("holgura {}", fmt(s.slack[i]))), tone: Tone::Muted });
            }
            GanttRow { label: net.names[i].clone(), bars }
        })
        .collect();
    let mut mk = vec![RefLine { value: s.total, label: format!("fin {}", fmt(s.total)), tone: Tone::Highlight }];
    mk.extend(markers);
    let legend = vec![(Tone::Highlight, "crítica".into()), (Tone::Accent, "no crítica".into()), (Tone::Muted, "holgura".into())];
    Block::Gantt(Gantt { title: Some(title.into()), unit: unit.into(), rows, markers: mk, legend })
}

/// Pasos del método de la ruta crítica.
pub fn cpm_steps(m: &PertModel, net: &Net, s: &Sched, intro: Vec<Step>) -> Vec<Step> {
    let n = net.n;
    let mut steps = intro;
    let mut prec = Table::new(Some("Precedencias"), &["Actividad", "Predecesoras inmediatas", "Tiempo"]).left(0).left(1);
    for i in 0..n {
        let p = net.preds[i].iter().map(|&p| net.names[p].as_str()).collect::<Vec<_>>().join(", ");
        prec.row(cells![&net.names[i], if p.is_empty() { "—".to_string() } else { p }, s.t[i]]);
    }
    steps.push(step(
        "La red del proyecto",
        "Cada actividad es un nodo y cada flecha una relación de precedencia: una actividad no puede empezar hasta que terminen todas sus predecesoras inmediatas.",
        vec![prec.block(), network(m, net, None, "Red de actividades")],
    ));
    let detailed = n <= 30;
    let none = vec![false; n];
    let mut fw = vec![false; n];
    if detailed {
        for &j in &net.order {
            fw[j] = true;
            let desc = if net.preds[j].is_empty() {
                format!(
                    "{} no tiene predecesoras: empieza en 0. Fin temprano = 0 + {} = {}.",
                    net.names[j],
                    fmt(s.t[j]),
                    fmt(s.ef[j])
                )
            } else {
                let terms: Vec<String> = net.preds[j].iter().map(|&i| format!("FT({}) = {}", net.names[i], fmt(s.ef[i]))).collect();
                format!(
                    "Inicio temprano de {} = máximo de los fines tempranos de sus predecesoras: máx({}) = {}. Fin temprano = {} + {} = {}.",
                    net.names[j],
                    terms.join(", "),
                    fmt(s.es[j]),
                    fmt(s.es[j]),
                    fmt(s.t[j]),
                    fmt(s.ef[j])
                )
            };
            steps.push(step(format!("Pase hacia adelante: {}", net.names[j]), desc, vec![table(net, s, None, &fw, &none, Some(j), false).block()]));
        }
    } else {
        fw = vec![true; n];
        steps.push(step(
            "Pase hacia adelante",
            "En orden de precedencias: inicio temprano = máximo de los fines tempranos de las predecesoras; fin temprano = inicio temprano + tiempo.",
            vec![table(net, s, None, &fw, &none, None, false).block()],
        ));
    }
    let unit = plural(&m.time_unit);
    steps.push(step(
        "Duración del proyecto",
        format!("El proyecto termina cuando acaba la última actividad: máximo de los fines tempranos = {} {unit}.", fmt(s.total)),
        vec![metrics(vec![metric_main("Duración del proyecto", s.total)])],
    ));
    let mut bw = vec![false; n];
    if detailed {
        for &i in net.order.iter().rev() {
            bw[i] = true;
            let desc = if net.succs[i].is_empty() {
                format!(
                    "{} no tiene sucesoras: puede terminar tan tarde como el proyecto, en {}. Inicio tardío = {} − {} = {}.",
                    net.names[i],
                    fmt(s.total),
                    fmt(s.lf[i]),
                    fmt(s.t[i]),
                    fmt(s.ls[i])
                )
            } else {
                let terms: Vec<String> = net.succs[i].iter().map(|&j| format!("IT({}) = {}", net.names[j], fmt(s.ls[j]))).collect();
                format!(
                    "Fin tardío de {} = mínimo de los inicios tardíos de sus sucesoras: mín({}) = {}. Inicio tardío = {} − {} = {}.",
                    net.names[i],
                    terms.join(", "),
                    fmt(s.lf[i]),
                    fmt(s.lf[i]),
                    fmt(s.t[i]),
                    fmt(s.ls[i])
                )
            };
            steps.push(step(format!("Pase hacia atrás: {}", net.names[i]), desc, vec![table(net, s, None, &fw, &bw, Some(i), false).block()]));
        }
    } else {
        bw = vec![true; n];
        steps.push(step(
            "Pase hacia atrás",
            "En orden inverso: fin tardío = mínimo de los inicios tardíos de las sucesoras (o la duración del proyecto); inicio tardío = fin tardío − tiempo.",
            vec![table(net, s, None, &fw, &bw, None, false).block()],
        ));
    }
    let paths = s.critical_paths(net, 50);
    let list = paths.iter().map(|p| format!("• {}", path_names(net, p))).collect::<Vec<_>>().join("\n");
    steps.push(step(
        "Holguras y ruta crítica",
        "Holgura total = inicio tardío − inicio temprano: lo que puede retrasarse una actividad sin retrasar el proyecto. Holgura libre: lo que puede retrasarse sin retrasar a ninguna sucesora. Las actividades con holgura total 0 son críticas.",
        vec![
            table(net, s, None, &fw, &bw, None, true).block(),
            text(format!("Ruta{} crítica{}:\n{list}", if paths.len() == 1 { "" } else { "s" }, if paths.len() == 1 { "" } else { "s" })),
            network(m, net, Some(s), "Ruta crítica"),
        ],
    ));
    steps
}

pub fn times(m: &PertModel, basis: TimeBasis) -> Vec<f64> {
    m.activities
        .iter()
        .map(|a| {
            let n = a.normal_time.unwrap_or(0.0);
            match basis {
                TimeBasis::Normal => n,
                TimeBasis::Crash => a.crash_time.unwrap_or(n),
            }
        })
        .collect()
}

pub fn solve_cpm(m: &PertModel, o: &PertOptions) -> Result<Report, ModError> {
    let net = build(m)?;
    let basis = if o.crashing != Crashing::None { TimeBasis::Normal } else { o.time_basis };
    let t = times(m, basis);
    let s = schedule(&net, &t);
    let unit = m.unit_plural();
    let paths = s.critical_paths(&net, 200);
    let crit_count = s.critical.iter().filter(|c| **c).count();

    // costos según la base de tiempos
    let cost_of = |i: usize| -> Option<f64> {
        let a = &m.activities[i];
        match basis {
            TimeBasis::Normal => a.normal_cost,
            TimeBasis::Crash => a.crash_cost.or(a.normal_cost),
        }
    };
    let has_cost = (0..net.n).any(|i| cost_of(i).is_some());
    let total_cost: f64 = (0..net.n).filter_map(cost_of).sum();
    let cp_cost: f64 = (0..net.n).filter(|&i| s.critical[i]).filter_map(cost_of).sum();

    let mut ms = vec![
        metric_main("Duración del proyecto", s.total),
        metric("Rutas críticas", paths.len()),
        metric("Actividades críticas", crit_count),
    ];
    if has_cost {
        ms.push(metric("Costo total", total_cost));
        ms.push(metric("Costo en la ruta crítica", cp_cost));
    }
    let all = vec![true; net.n];
    let title = if basis == TimeBasis::Crash { "Análisis de actividades (tiempos de compresión)" } else { "Análisis de actividades" };
    let mut sol = vec![metrics(ms), table(&net, &s, Some(title), &all, &all, None, true).block()];
    let mut pt = Table::new(Some("Rutas críticas"), &["#", "Ruta", "Duración"]).left(1);
    for (k, p) in paths.iter().enumerate() {
        pt.row(cells![k + 1, path_names(&net, p), s.total]);
    }
    if paths.len() >= 200 {
        pt = pt.note("Se muestran las primeras 200 rutas críticas.");
    }
    sol.push(pt.block());
    if let (Some(d), Crashing::None) = (o.desired_time, o.crashing) {
        if s.total <= d + s.eps {
            sol.push(text_tone(format!("El proyecto cumple el plazo deseado de {} {unit} (sobran {}).", fmt(d), fmt(d - s.total)), Tone::Good));
        } else {
            sol.push(text_tone(
                format!(
                    "El proyecto no cumple el plazo deseado de {} {unit}: se pasa en {}. Prueba el análisis de compresión con «Plazo deseado».",
                    fmt(d),
                    fmt(s.total - d)
                ),
                Tone::Bad,
            ));
        }
    }

    let mut markers = vec![];
    if let Some(d) = o.desired_time {
        markers.push(RefLine { value: d, label: format!("plazo {}", fmt(d)), tone: Tone::Accent });
    }
    if let Some(c) = o.current_time {
        markers.push(RefLine { value: c, label: format!("hoy {}", fmt(c)), tone: Tone::Normal });
    }

    let headline_base = format!("El proyecto dura {} {unit}; ruta crítica: {}.", fmt(s.total), paths.first().map(|p| path_names(&net, p)).unwrap_or_default());
    let mut rep = Report::new("pert", &m.name, Status::Solved, headline_base);
    let mut data = json!({
        "time_basis": basis,
        "completion_time": clean(s.total),
        "critical_paths": paths.iter().map(|p| p.iter().map(|&i| net.names[i].clone()).collect::<Vec<_>>()).collect::<Vec<_>>(),
        "activities": (0..net.n).map(|i| json!({
            "name": net.names[i], "time": clean(s.t[i]), "es": clean(s.es[i]), "ef": clean(s.ef[i]),
            "ls": clean(s.ls[i]), "lf": clean(s.lf[i]), "slack": clean(s.slack[i]), "free_slack": clean(s.free[i]), "critical": s.critical[i]
        })).collect::<Vec<_>>(),
    });
    if has_cost {
        data["total_cost"] = json!(clean(total_cost));
        data["critical_path_cost"] = json!(clean(cp_cost));
    }

    // compresión
    let mut crash_blocks = vec![];
    if o.crashing != Crashing::None {
        let r = crash::analyze(m, &net, o)?;
        rep.status = r.status;
        rep.headline = r.headline.clone();
        crash_blocks = r.blocks;
        data["crashing"] = r.data;
    }

    let costs: Vec<f64> = (0..net.n).map(|i| cost_of(i).unwrap_or(0.0)).collect();
    let cost_blocks = if has_cost { cost::pert_cost(m, &net, &s, &costs) } else { vec![] };
    let (control_blocks, control_data) = cost::control(m, &net, &s, o.current_time);
    if let Some(cd) = control_data {
        data["control"] = cd;
    }

    let mut st = vec![];
    if o.steps {
        st.push(steps(cpm_steps(m, &net, &s, vec![])));
    }

    rep = rep
        .section("solucion", "Solución", sol)
        .section("red", "Red", vec![network(m, &net, Some(&s), "Red del proyecto y ruta crítica")])
        .section("gantt", "Gantt", vec![gantt(&net, &s, "Diagrama de Gantt (inicios tempranos)", &unit, markers)])
        .section("compresion", "Compresión", crash_blocks)
        .section("costos", "PERT/Costo", cost_blocks)
        .section("control", "Control", control_blocks)
        .section("pasos", "Paso a paso", st)
        .data(data);
    Ok(rep)
}
