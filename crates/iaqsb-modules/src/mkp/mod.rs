//! MKP · Procesos de Markov (equivalente al módulo MKP de WinQSB).
//!
//! Cadenas de Markov de tiempo discreto: verificación de probabilidades,
//! clasificación de estados (clases, recurrentes, transitorios, absorbentes y
//! periodo), probabilidades de estado estable con tiempos de recurrencia,
//! evolución período a período con su costo esperado, tiempos de primer paso y,
//! si hay estados transitorios, matriz fundamental y probabilidades de absorción.

use iaqsb_core::winqsb::{num, parse_num, Reader, Writer};
use iaqsb_core::*;
use nalgebra::{DMatrix, DVector};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MkState {
    pub name: String,
    /// Probabilidad de empezar en el estado.
    #[serde(default)]
    pub initial: f64,
    /// Costo (o rendimiento) por período en el estado.
    #[serde(default)]
    pub cost: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MkpModel {
    #[serde(default)]
    pub name: String,
    pub states: Vec<MkState>,
    /// Probabilidades de transición: desde (fila) × hacia (columna).
    pub transitions: Vec<Vec<f64>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct MkpOptions {
    pub steps: bool,
    /// Períodos a mostrar en la evolución.
    pub periods: u32,
}

impl Default for MkpOptions {
    fn default() -> Self {
        MkpOptions { steps: true, periods: 10 }
    }
}

pub struct Mkp;

impl TypedModule for Mkp {
    type Model = MkpModel;
    type Options = MkpOptions;

    fn file_tag(&self) -> &'static str {
        "MKP"
    }

    fn spec(&self) -> ModuleSpec {
        let mut w = Widget::matrix(
            Some("Probabilidades de transición (desde × hacia)"),
            "transitions",
            Axis::new("states", "Desde").add("Añadir estado"),
            Axis::new("states", "Hacia").extra(Field::number("initial", "Probabilidad inicial")).extra(Field::number("cost", "Costo del estado")),
            Field::number("p", "Probabilidad"),
        );
        if let Widget::Matrix { allow_diagonal, .. } = &mut w {
            *allow_diagonal = true;
        }
        ModuleSpec {
            id: "mkp".into(),
            winqsb: "MKP".into(),
            name: "Procesos de Markov".into(),
            description: "Estado estable, evolución por períodos, tiempos de primer paso, clasificación de estados y absorción".into(),
            file_ext: "MKP".into(),
            problem_types: vec![ProblemType {
                id: "markov".into(),
                label: "Cadena de Markov".into(),
                description: "Estados, probabilidades de transición, probabilidades iniciales y costos por estado".into(),
                dims: vec![Field::integer("states", "Número de estados", 3)],
                layout: vec![w],
            }],
            options: vec![Field::boolean("steps", "Mostrar paso a paso", true), Field::integer("periods", "Períodos de la evolución", 10)],
        }
    }

    fn template(&self, _pt: &str, dims: &Value) -> Result<MkpModel, ModError> {
        let n = dim(dims, "states", 3, 1, 300)?;
        Ok(MkpModel {
            name: "Proceso de Markov".into(),
            states: (1..=n).map(|i| MkState { name: format!("Estado{i}"), initial: if i == 1 { 1.0 } else { 0.0 }, cost: 0.0 }).collect(),
            transitions: (0..n).map(|i| (0..n).map(|j| if i == j { 1.0 } else { 0.0 }).collect()).collect(),
        })
    }

    fn solve(&self, m: &MkpModel, o: &MkpOptions) -> Result<Report, ModError> {
        solve(m, o)
    }

    fn read(&self, bytes: &[u8]) -> Result<MkpModel, ModError> {
        read_mkp(bytes)
    }

    fn write(&self, m: &MkpModel) -> Result<Vec<u8>, ModError> {
        Ok(write_mkp(m))
    }

    fn examples(&self) -> Vec<(&'static str, MkpModel)> {
        let ex = |s: &str| serde_json::from_str::<MkpModel>(s).unwrap();
        vec![
            ("inventario_camaras", ex(include_str!("ejemplos/camaras.json"))),
            ("cambio_de_marca", ex(include_str!("ejemplos/marcas.json"))),
            ("cuentas_por_cobrar", ex(include_str!("ejemplos/cuentas.json"))),
        ]
    }
}

// ---------------------------------------------------------------- análisis

/// Clasificación de los estados en clases comunicantes.
pub struct Classes {
    /// Clase de cada estado.
    pub of: Vec<usize>,
    /// Estados de cada clase, en orden.
    pub members: Vec<Vec<usize>>,
    /// La clase es cerrada (recurrente).
    pub closed: Vec<bool>,
    /// Periodo de cada clase (0 si un estado transitorio no vuelve a sí mismo).
    pub period: Vec<usize>,
}

fn gcd(a: usize, b: usize) -> usize {
    if b == 0 {
        a
    } else {
        gcd(b, a % b)
    }
}

pub fn classify(p: &[Vec<f64>]) -> Classes {
    let n = p.len();
    let mut r: Vec<Vec<bool>> = (0..n).map(|i| (0..n).map(|j| i == j || p[i][j] > 0.0).collect()).collect();
    for k in 0..n {
        for i in 0..n {
            if r[i][k] {
                for j in 0..n {
                    if r[k][j] {
                        r[i][j] = true;
                    }
                }
            }
        }
    }
    let mut of = vec![usize::MAX; n];
    let mut members: Vec<Vec<usize>> = vec![];
    for i in 0..n {
        if of[i] != usize::MAX {
            continue;
        }
        let c: Vec<usize> = (0..n).filter(|&j| r[i][j] && r[j][i]).collect();
        for &j in &c {
            of[j] = members.len();
        }
        members.push(c);
    }
    let closed: Vec<bool> = members.iter().map(|c| c.iter().all(|&i| (0..n).all(|j| p[i][j] <= 0.0 || c.contains(&j)))).collect();
    let period = members
        .iter()
        .map(|c| {
            let mut level = vec![usize::MAX; n];
            level[c[0]] = 0;
            let mut q = std::collections::VecDeque::from([c[0]]);
            while let Some(u) = q.pop_front() {
                for &v in c {
                    if p[u][v] > 0.0 && level[v] == usize::MAX {
                        level[v] = level[u] + 1;
                        q.push_back(v);
                    }
                }
            }
            let mut g = 0;
            for &u in c {
                for &v in c {
                    if p[u][v] > 0.0 {
                        g = gcd(g, (level[u] as i64 + 1 - level[v] as i64).unsigned_abs() as usize);
                    }
                }
            }
            g
        })
        .collect();
    Classes { of, members, closed, period }
}

/// Distribución estacionaria de una clase cerrada.
fn stationary(p: &[Vec<f64>], c: &[usize]) -> Option<Vec<f64>> {
    let k = c.len();
    let mut a = DMatrix::from_fn(k, k, |i, j| p[c[j]][c[i]] - if i == j { 1.0 } else { 0.0 });
    for j in 0..k {
        a[(k - 1, j)] = 1.0;
    }
    let mut b = DVector::zeros(k);
    b[k - 1] = 1.0;
    let x = a.lu().solve(&b)?;
    Some(x.iter().map(|v| v.max(0.0)).collect())
}

/// Tiempos medios de primer paso m[i][j] (j = i: tiempo de recurrencia). ∞ si
/// la probabilidad de llegar a j es menor que 1.
pub fn first_passage(p: &[Vec<f64>]) -> Vec<Vec<f64>> {
    let n = p.len();
    let mut m = vec![vec![f64::INFINITY; n]; n];
    for j in 0..n {
        // estados distintos de j que pueden llegar a j
        let mut reach = vec![false; n];
        reach[j] = true;
        loop {
            let mut ch = false;
            for i in 0..n {
                if !reach[i] && (0..n).any(|k| p[i][k] > 0.0 && reach[k]) {
                    reach[i] = true;
                    ch = true;
                }
            }
            if !ch {
                break;
            }
        }
        // probabilidad de llegar alguna vez a j
        let r: Vec<usize> = (0..n).filter(|&i| i != j && reach[i]).collect();
        let mut f = vec![0.0; n];
        if !r.is_empty() {
            let a = DMatrix::from_fn(r.len(), r.len(), |a, b| if a == b { 1.0 } else { 0.0 } - p[r[a]][r[b]]);
            let rhs = DVector::from_fn(r.len(), |a, _| p[r[a]][j]);
            if let Some(x) = a.lu().solve(&rhs) {
                for (a, &i) in r.iter().enumerate() {
                    f[i] = x[a];
                }
            }
        }
        // tiempos donde se llega con certeza
        let r1: Vec<usize> = r.iter().copied().filter(|&i| f[i] > 1.0 - 1e-9).collect();
        if !r1.is_empty() {
            let a = DMatrix::from_fn(r1.len(), r1.len(), |a, b| if a == b { 1.0 } else { 0.0 } - p[r1[a]][r1[b]]);
            if let Some(x) = a.lu().solve(&DVector::from_element(r1.len(), 1.0)) {
                for (a, &i) in r1.iter().enumerate() {
                    m[i][j] = x[a];
                }
            }
        }
        let fjj = p[j][j] + (0..n).filter(|&k| k != j).map(|k| p[j][k] * f[k]).sum::<f64>();
        if fjj > 1.0 - 1e-9 {
            m[j][j] = 1.0 + (0..n).filter(|&k| k != j && p[j][k] > 0.0).map(|k| p[j][k] * m[k][j]).sum::<f64>();
        }
    }
    m
}

fn matmul_vec(q: &[f64], p: &[Vec<f64>]) -> Vec<f64> {
    let n = q.len();
    (0..n).map(|j| (0..n).map(|i| q[i] * p[i][j]).sum()).collect()
}

fn matmul(a: &[Vec<f64>], b: &[Vec<f64>]) -> Vec<Vec<f64>> {
    a.iter().map(|r| matmul_vec(r, b)).collect()
}

fn prob_cells(v: f64) -> Cell {
    Cell::Num(tidy(v))
}

fn matrix_table(title: Option<&str>, names: &[&str], m: &[Vec<f64>], corner: &str) -> Table {
    let mut head = vec![corner.to_string()];
    head.extend(names.iter().map(|s| s.to_string()));
    let mut t = Table::new(title, &head).left(0);
    for (i, r) in m.iter().enumerate() {
        let mut row = vec![Cell::from(names[i])];
        row.extend(r.iter().map(|v| prob_cells(*v)));
        t.row(row);
    }
    t
}

fn solve(m: &MkpModel, o: &MkpOptions) -> Result<Report, ModError> {
    let n = m.states.len();
    if n == 0 {
        return invalid("hace falta al menos un estado");
    }
    check_names("estados", &m.states.iter().map(|s| &s.name).collect::<Vec<_>>())?;
    let names: Vec<&str> = m.states.iter().map(|s| s.name.as_str()).collect();
    let mut p = m.transitions.clone();
    p.resize(n, vec![]);
    for r in p.iter_mut() {
        r.resize(n, 0.0);
    }
    for i in 0..n {
        if p[i].iter().any(|v| *v < 0.0 || *v > 1.0 + 1e-9 || !v.is_finite()) {
            return invalid(format!("las probabilidades de transición desde {} deben estar entre 0 y 1", names[i]));
        }
        let s: f64 = p[i].iter().sum();
        if (s - 1.0).abs() > 1e-6 {
            return invalid(format!("las probabilidades de transición desde {} suman {}, no 1", names[i], fmt(s)));
        }
    }
    let cost: Vec<f64> = m.states.iter().map(|s| s.cost).collect();
    let mut q0: Vec<f64> = m.states.iter().map(|s| s.initial).collect();
    let s0: f64 = q0.iter().sum();
    let mut notes = vec![];
    if q0.iter().any(|v| *v < 0.0) {
        return invalid("las probabilidades iniciales no pueden ser negativas");
    }
    if s0 == 0.0 {
        q0 = vec![1.0 / n as f64; n];
        notes.push("No se dieron probabilidades iniciales: se supone la misma para todos los estados.".to_string());
    } else if (s0 - 1.0).abs() > 1e-6 {
        return invalid(format!("las probabilidades iniciales suman {}, no 1", fmt(s0)));
    }

    let cl = classify(&p);
    let nc = cl.members.len();
    let closed_ids: Vec<usize> = (0..nc).filter(|&c| cl.closed[c]).collect();
    let transient: Vec<usize> = (0..n).filter(|&i| !cl.closed[cl.of[i]]).collect();
    let pis: Vec<Vec<f64>> = closed_ids.iter().map(|&c| stationary(&p, &cl.members[c]).unwrap_or_else(|| vec![0.0; cl.members[c].len()])).collect();
    // absorción en cada clase cerrada desde los transitorios
    let tn = transient.len();
    let mut nmat = DMatrix::<f64>::zeros(tn, tn);
    let mut absorb = vec![vec![0.0; closed_ids.len()]; tn];
    if tn > 0 {
        let a = DMatrix::from_fn(tn, tn, |a, b| if a == b { 1.0 } else { 0.0 } - p[transient[a]][transient[b]]);
        nmat = a.try_inverse().ok_or_else(|| ModError::Invalid("no se pudo invertir I − Q".into()))?;
        for (k, &c) in closed_ids.iter().enumerate() {
            let rk = DVector::from_fn(tn, |a, _| cl.members[c].iter().map(|&j| p[transient[a]][j]).sum::<f64>());
            let b = &nmat * rk;
            for a in 0..tn {
                absorb[a][k] = b[a];
            }
        }
    }
    // matriz límite (promedio de largo plazo) L
    let mut lim = vec![vec![0.0; n]; n];
    for i in 0..n {
        let ci = cl.of[i];
        if cl.closed[ci] {
            let k = closed_ids.iter().position(|&c| c == ci).unwrap();
            for (a, &j) in cl.members[ci].iter().enumerate() {
                lim[i][j] = pis[k][a];
            }
        } else {
            let t = transient.iter().position(|&x| x == i).unwrap();
            for (k, &c) in closed_ids.iter().enumerate() {
                for (a, &j) in cl.members[c].iter().enumerate() {
                    lim[i][j] += absorb[t][k] * pis[k][a];
                }
            }
        }
    }
    let unique = closed_ids.len() == 1;
    let long_run = matmul_vec(&q0, &lim);
    let lr_cost: f64 = long_run.iter().zip(&cost).map(|(a, b)| a * b).sum();
    let fp = first_passage(&p);
    let has_cost = cost.iter().any(|c| *c != 0.0);

    // estado estable
    let ss_title = if unique { "Probabilidades de estado estable" } else { "Probabilidades de largo plazo (según el estado inicial)" };
    let mut ss = if has_cost {
        Table::new(Some(ss_title), &["Estado", "Probabilidad", "Tiempo de recurrencia", "Costo del estado", "Costo esperado"]).left(0)
    } else {
        Table::new(Some(ss_title), &["Estado", "Probabilidad", "Tiempo de recurrencia"]).left(0)
    };
    for i in 0..n {
        let mut row = cells![names[i], tidy(long_run[i]), tidy(fp[i][i])];
        if has_cost {
            row.extend(cells![cost[i], tidy(long_run[i] * cost[i])]);
        }
        ss.row_tone(row, if long_run[i] > 1e-12 { Tone::Normal } else { Tone::Muted });
    }
    let mut tot = cells!["Total", tidy(long_run.iter().sum::<f64>()), Cell::Empty];
    if has_cost {
        tot.extend(cells![Cell::Empty, tidy(lr_cost)]);
    }
    ss.row_tone(tot, Tone::Total);
    let aperiodic = closed_ids.iter().all(|&c| cl.period[c] == 1);
    let mut kind = vec![];
    if unique {
        kind.push(if aperiodic {
            "La cadena tiene una sola clase recurrente y es aperiódica: las probabilidades de estado estable no dependen del estado inicial y P^n converge a ellas.".to_string()
        } else {
            format!("La cadena tiene una sola clase recurrente con periodo {}: las probabilidades de estado estable existen (fracción del tiempo en cada estado), pero P^n oscila y no converge.", cl.period[closed_ids[0]])
        });
    } else {
        kind.push(format!("Hay {} clases recurrentes: no existe un estado estable único. Las probabilidades de largo plazo dependen de dónde empieza la cadena; se muestran las que corresponden a las probabilidades iniciales.", closed_ids.len()));
    }
    kind.extend(notes);

    // clasificación
    let mut ct = Table::new(Some("Clasificación de los estados"), &["Estado", "Clase", "Tipo", "Periodo"]).left(0).left(2);
    for i in 0..n {
        let c = cl.of[i];
        let tipo = if cl.closed[c] && cl.members[c].len() == 1 && p[i][i] >= 1.0 - 1e-12 {
            "absorbente"
        } else if cl.closed[c] {
            "recurrente"
        } else {
            "transitorio"
        };
        let per = if cl.period[c] == 0 { Cell::from("—") } else { Cell::Num(cl.period[c] as f64) };
        ct.row_tone(cells![names[i], c + 1, tipo, per], match tipo {
            "transitorio" => Tone::Muted,
            "absorbente" => Tone::Accent,
            _ => Tone::Normal,
        });
    }

    // diagrama de transición
    let net = {
        let r = if n == 1 { 0.0 } else { 0.38 };
        let nodes = (0..n)
            .map(|i| {
                let a = std::f64::consts::PI * (-0.5 + 2.0 * i as f64 / n as f64);
                NetNode {
                    id: format!("s{i}"),
                    label: names[i].into(),
                    x: 0.5 + r * a.cos() * 0.9,
                    y: 0.5 + r * a.sin() * 0.85,
                    note: None,
                    tone: if cl.closed[cl.of[i]] && cl.members[cl.of[i]].len() == 1 { Tone::Accent } else { Tone::Normal },
                }
            })
            .collect();
        let mut edges = vec![];
        for i in 0..n {
            for j in 0..n {
                if p[i][j] > 0.0 {
                    edges.push(NetEdge { from: format!("s{i}"), to: format!("s{j}"), label: Some(fmt(p[i][j])), tone: Tone::Normal });
                }
            }
        }
        Block::Network(Network { title: Some("Diagrama de transición".into()), directed: true, nodes, edges })
    };

    // evolución
    let per = o.periods.min(1000) as usize;
    let mut q = q0.clone();
    let mut evo = vec![q.clone()];
    for _ in 0..per {
        q = matmul_vec(&q, &p);
        evo.push(q.clone());
    }
    let mut head = vec!["Período".to_string()];
    head.extend(names.iter().map(|s| s.to_string()));
    if has_cost {
        head.push("Costo esperado".into());
    }
    let mut et = Table::new(Some("Probabilidad de cada estado por período"), &head);
    let mut total_cost = 0.0;
    for (k, v) in evo.iter().enumerate() {
        let mut row = vec![Cell::Num(k as f64)];
        row.extend(v.iter().map(|x| prob_cells(*x)));
        if has_cost {
            let c: f64 = v.iter().zip(&cost).map(|(a, b)| a * b).sum();
            if k > 0 {
                total_cost += c;
            }
            row.push(Cell::Num(tidy(c)));
        }
        et.row(row);
    }
    let mut ch = Chart::new(Some("Evolución de las probabilidades"), "Período", "Probabilidad");
    for j in 0..n {
        ch = ch.series(Series::new(names[j], SeriesKind::Line, evo.iter().enumerate().map(|(k, v)| [k as f64, v[j]]).collect()));
    }
    let mut evo_blocks = vec![];
    if has_cost {
        evo_blocks.push(metrics(vec![metric_main(format!("Costo esperado total de los períodos 1 a {per}"), tidy(total_cost)), metric("Costo esperado por período a largo plazo", tidy(lr_cost))]));
    }
    evo_blocks.push(ch.block());
    evo_blocks.push(et.clone().block());

    // primer paso
    let fpt = matrix_table(Some("Tiempos medios de primer paso (desde × hacia; diagonal = recurrencia)"), &names, &fp, "Desde \\ Hacia");
    let mut fp_blocks = vec![fpt.clone().block()];
    if fp.iter().flatten().any(|v| v.is_infinite()) {
        fp_blocks.push(text("∞: desde ese estado no se llega con certeza al estado destino."));
    }

    // absorción
    let mut ab_blocks = vec![];
    if tn > 0 && !closed_ids.is_empty() {
        let tnames: Vec<&str> = transient.iter().map(|&i| names[i]).collect();
        let cname = |c: usize| cl.members[c].iter().map(|&i| names[i]).collect::<Vec<_>>().join(", ");
        let mut h = vec!["Desde".to_string()];
        h.extend(closed_ids.iter().map(|&c| format!("Absorción en {}", cname(c))));
        h.push("Períodos esperados antes de la absorción".into());
        let mut t = Table::new(Some("Probabilidades de absorción"), &h).left(0);
        for a in 0..tn {
            let mut row = vec![Cell::from(tnames[a])];
            row.extend(absorb[a].iter().map(|v| prob_cells(*v)));
            row.push(Cell::Num(tidy(nmat.row(a).sum())));
            t.row(row);
        }
        let nm: Vec<Vec<f64>> = (0..tn).map(|a| (0..tn).map(|b| nmat[(a, b)]).collect()).collect();
        ab_blocks.push(t.block());
        ab_blocks.push(matrix_table(Some("Matriz fundamental N = (I − Q)⁻¹: visitas esperadas a cada estado transitorio"), &tnames, &nm, "Desde \\ Hacia").block());
    }

    // paso a paso
    let mut st = vec![];
    if o.steps {
        let mut v = vec![];
        let mut sums = Table::new(None, &["Estado", "Suma de la fila"]).left(0);
        for i in 0..n {
            sums.row(cells![names[i], tidy(p[i].iter().sum::<f64>())]);
        }
        v.push(step("Verificar las probabilidades", "Cada fila de la matriz de transición es una distribución: sus probabilidades suman 1.", vec![sums.block()]));
        v.push(step(
            "Clasificar los estados",
            "Dos estados se comunican si cada uno puede llegar al otro. Una clase es recurrente si de ella no se puede salir; si se puede salir, sus estados son transitorios. Un estado absorbente no se abandona nunca. El periodo es el máximo común divisor de las longitudes de los ciclos de la clase.",
            vec![ct.clone().block()],
        ));
        if n <= 10 {
            let x = |j: usize| format!("π({})", names[j]);
            let eqs: Vec<String> = (0..n)
                .map(|j| {
                    let rhs: Vec<String> = (0..n).filter(|&i| p[i][j] > 0.0).map(|i| format!("{}·{}", fmt(p[i][j]), x(i))).collect();
                    format!("{} = {}", x(j), if rhs.is_empty() { "0".into() } else { rhs.join(" + ") })
                })
                .collect();
            v.push(step(
                "Ecuaciones de estado estable",
                format!(
                    "π = π·P y la suma de las π es 1. Una de las ecuaciones de balance sobra y se reemplaza por la de la suma:\n{}\n{} = 1",
                    eqs.join("\n"),
                    (0..n).map(x).collect::<Vec<_>>().join(" + ")
                ),
                vec![ss.clone().block()],
            ));
            let mut pw = p.clone();
            let mut k = 1;
            let mut blocks = vec![];
            while k <= 16 {
                blocks.push(matrix_table(Some(&format!("P^{k}")), &names, &pw, "Desde \\ Hacia").block());
                pw = matmul(&pw, &pw);
                k *= 2;
            }
            v.push(step(
                "Potencias de P",
                if unique && aperiodic { "Al elevar P a potencias mayores, todas las filas se acercan a las probabilidades de estado estable." } else { "Las potencias de P muestran a dónde va la cadena; aquí no todas las filas convergen a la misma distribución." },
                blocks,
            ));
        } else {
            v.push(step("Estado estable", "Se resuelve π = π·P con la suma de las π igual a 1 (factorización LU).", vec![ss.clone().block()]));
        }
        v.push(step("Evolución período a período", "La distribución del período siguiente es la actual multiplicada por P: q(n+1) = q(n)·P.", vec![et.block()]));
        v.push(step(
            "Tiempos de primer paso",
            "m(i, j) = 1 + Σ p(i, k)·m(k, j) sobre los k distintos de j. En la diagonal queda el tiempo de recurrencia, que en una clase recurrente es 1/π(j).",
            vec![fpt.block()],
        ));
        if !ab_blocks.is_empty() {
            v.push(step("Absorción", "Con Q (transiciones entre transitorios) y R (de transitorios a recurrentes): N = (I − Q)⁻¹ y las probabilidades de absorción son N·R. La suma de cada fila de N es el número esperado de períodos antes de la absorción.", ab_blocks.clone()));
        }
        st.push(steps(v));
    }

    let headline = if unique {
        let mut parts: Vec<String> = (0..n).filter(|&i| long_run[i] > 1e-12).map(|i| format!("{} {}", names[i], fmt(tidy(long_run[i])))).collect();
        if parts.len() > 6 {
            parts.truncate(6);
            parts.push("…".into());
        }
        format!("Estado estable: {}{}.", parts.join(", "), if has_cost { format!("; costo esperado {} por período", fmt(tidy(lr_cost))) } else { String::new() })
    } else {
        format!("No hay estado estable único ({} clases recurrentes); se muestran las probabilidades de largo plazo desde el estado inicial.", closed_ids.len())
    };
    let mut sol = vec![];
    if has_cost {
        sol.push(metrics(vec![metric_main("Costo esperado por período (largo plazo)", tidy(lr_cost)), metric("Clases recurrentes", closed_ids.len()), metric("Estados transitorios", tn)]));
    } else {
        sol.push(metrics(vec![metric("Clases recurrentes", closed_ids.len()), metric("Estados transitorios", tn)]));
    }
    for k in kind {
        sol.push(text(k));
    }
    sol.push(ss.block());
    sol.push(ct.block());
    let mut r = Report::new("mkp", &m.name, Status::Solved, headline)
        .section("solucion", "Solución", sol)
        .section("diagrama", "Diagrama", vec![net])
        .section("evolucion", "Evolución", evo_blocks)
        .section("primer_paso", "Primer paso", fp_blocks);
    if !ab_blocks.is_empty() {
        r = r.section("absorcion", "Absorción", ab_blocks);
    }
    let finite = |v: f64| if v.is_finite() { json!(tidy(v)) } else { Value::Null };
    Ok(r.section("pasos", "Paso a paso", st).data(json!({
        "unique_steady_state": unique,
        "steady_state": long_run.iter().map(|v| tidy(*v)).collect::<Vec<_>>(),
        "long_run_cost": tidy(lr_cost),
        "first_passage": fp.iter().map(|r| r.iter().map(|v| finite(*v)).collect::<Vec<_>>()).collect::<Vec<_>>(),
        "classes": cl.members.iter().enumerate().map(|(c, ms)| json!({"states": ms.iter().map(|&i| names[i]).collect::<Vec<_>>(), "recurrent": cl.closed[c], "period": cl.period[c]})).collect::<Vec<_>>(),
        "absorption": transient.iter().enumerate().map(|(a, &i)| json!({"from": names[i], "probabilities": absorb[a].iter().map(|v| tidy(*v)).collect::<Vec<_>>(), "expected_steps": tidy(nmat.row(a).sum())})).collect::<Vec<_>>(),
        "evolution": evo.iter().map(|v| v.iter().map(|x| tidy(*x)).collect::<Vec<_>>()).collect::<Vec<_>>(),
    })))
}

// ------------------------------------------------------------------ archivos

fn read_mkp(bytes: &[u8]) -> Result<MkpModel, ModError> {
    let mut r = Reader::new(bytes);
    let h = r.line("la cabecera")?;
    if !h.get(0).trim().to_ascii_uppercase().starts_with("MKP") {
        return Err(ModError::Format("no es un archivo de MKP".into()));
    }
    let name = Some(h.get(1).trim()).filter(|s| !s.is_empty()).unwrap_or("MKP Example").to_string();
    let n = parse_num(h.get(2)).unwrap_or(0.0) as usize;
    if n == 0 || n > 10_000 {
        return Err(ModError::Format("el número de estados no es válido".into()));
    }
    let head = r.line("los nombres de los estados")?;
    let mut states: Vec<MkState> = (0..n)
        .map(|j| MkState { name: Some(head.get(j + 1).trim()).filter(|s| !s.is_empty()).map(str::to_string).unwrap_or(format!("State{}", j + 1)), initial: 0.0, cost: 0.0 })
        .collect();
    let mut transitions = vec![];
    for i in 0..n {
        let l = r.line(&format!("las transiciones desde el estado {}", i + 1))?;
        transitions.push((0..n).map(|j| parse_num(l.get(j + 1)).unwrap_or(0.0)).collect());
    }
    if let Some(l) = r.try_line() {
        for (j, s) in states.iter_mut().enumerate() {
            s.initial = parse_num(l.get(j + 1)).unwrap_or(0.0);
        }
    }
    if let Some(l) = r.try_line() {
        for (j, s) in states.iter_mut().enumerate() {
            s.cost = parse_num(l.get(j + 1)).unwrap_or(0.0);
        }
    }
    Ok(MkpModel { name, states, transitions })
}

fn write_mkp(m: &MkpModel) -> Vec<u8> {
    let n = m.states.len();
    let mut w = Writer::new();
    w.row(&["MKP".to_string(), m.name.clone(), n.to_string()]);
    let mut r = vec!["From \\ To".to_string()];
    r.extend(m.states.iter().map(|s| s.name.clone()));
    w.row(&r);
    for (i, s) in m.states.iter().enumerate() {
        let mut r = vec![s.name.clone()];
        r.extend((0..n).map(|j| num(m.transitions.get(i).and_then(|x| x.get(j)).copied().unwrap_or(0.0))));
        w.row(&r);
    }
    let mut r = vec!["Initial Prob.".to_string()];
    r.extend(m.states.iter().map(|s| num(s.initial)));
    w.row(&r);
    let mut r = vec!["State Cost".to_string()];
    r.extend(m.states.iter().map(|s| num(s.cost)));
    w.row(&r);
    w.bytes()
}
