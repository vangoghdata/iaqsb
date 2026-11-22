//! DA · Análisis de decisiones (equivalente al módulo DA de WinQSB).
//!
//! - Análisis bayesiano: probabilidades conjuntas, marginales y a posteriori.
//! - Tabla de pagos: los siete criterios de WinQSB (maximin, maximax, Hurwicz,
//!   mínimo arrepentimiento, igual verosimilitud, valor esperado y
//!   arrepentimiento esperado), valor de la información perfecta y, con
//!   información muestral, la mejor decisión por indicador, VEIM y eficiencia.
//!   Árbol de decisión de la tabla.
//! - Juego de suma cero: punto de silla, dominancia, estrategias mixtas óptimas
//!   (programación lineal con HiGHS) y simulación de partidas.
//! - Árbol de decisión: valores esperados hacia atrás y decisiones óptimas.

use iaqsb_core::lp::{solve as lp_solve, Lp, LpStatus, Sense};
use iaqsb_core::winqsb::{num, parse_num, Reader, Writer};
use iaqsb_core::*;
use rand::{RngExt, SeedableRng};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum DaType {
    Bayesian,
    #[default]
    Payoff,
    ZeroSum,
    Tree,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct State {
    pub name: String,
    /// Probabilidad a priori.
    #[serde(default)]
    pub prior: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Named {
    pub name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum NodeKind {
    #[default]
    Decision,
    Chance,
    End,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DtNode {
    pub name: String,
    #[serde(default)]
    pub kind: NodeKind,
    /// Nodos siguientes (nombres o números).
    #[serde(default)]
    pub next: Vec<String>,
    /// Pago al llegar al nodo (+ ganancia, − costo).
    #[serde(default)]
    pub payoff: f64,
    /// Probabilidad del nodo si sale de un nodo de azar.
    #[serde(default)]
    pub probability: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DaModel {
    #[serde(default)]
    pub name: String,
    pub problem_type: DaType,
    /// Estados de la naturaleza con su probabilidad a priori.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub states: Vec<State>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub decisions: Vec<Named>,
    /// Pagos: decisión × estado.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub payoffs: Vec<Vec<f64>>,
    /// Indicadores de la información muestral.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub indicators: Vec<Named>,
    /// P(indicador | estado): indicador × estado.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub likelihoods: Vec<Vec<f64>>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub player1: Vec<Named>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub player2: Vec<Named>,
    /// Ganancia del jugador 1: estrategia 1 × estrategia 2.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub game: Vec<Vec<f64>>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub nodes: Vec<DtNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct DaOptions {
    pub steps: bool,
    /// Coeficiente de optimismo del criterio de Hurwicz (0 a 1).
    pub hurwicz: f64,
    /// Juego: partidas simuladas (0 = no simular).
    pub games: u32,
    pub seed: u64,
}

impl Default for DaOptions {
    fn default() -> Self {
        DaOptions { steps: true, hurwicz: 0.5, games: 0, seed: 1 }
    }
}

fn fit(mat: &mut Vec<Vec<f64>>, r: usize, c: usize) {
    mat.resize(r, vec![]);
    for row in mat.iter_mut() {
        row.resize(c, 0.0);
    }
}

pub struct Da;

impl TypedModule for Da {
    type Model = DaModel;
    type Options = DaOptions;

    fn file_tag(&self) -> &'static str {
        "DA"
    }

    fn spec(&self) -> ModuleSpec {
        let states_axis = || Axis::new("states", "Estado").add("Añadir estado").extra(Field::number("prior", "Probabilidad a priori"));
        let lik = Widget::matrix(
            Some("Información muestral: P(indicador | estado) (opcional)"),
            "likelihoods",
            Axis::new("indicators", "Indicador").add("Añadir indicador"),
            Axis::new("states", "Estado"),
            Field::number("p", "Probabilidad"),
        );
        ModuleSpec {
            id: "da".into(),
            winqsb: "DA".into(),
            name: "Análisis de decisiones".into(),
            description: "Tabla de pagos (siete criterios), análisis bayesiano, juegos de suma cero y árboles de decisión".into(),
            file_ext: "DAA".into(),
            problem_types: vec![
                ProblemType {
                    id: "payoff".into(),
                    label: "Tabla de pagos".into(),
                    description: "Elegir una decisión ante estados de la naturaleza, con o sin información muestral".into(),
                    dims: vec![Field::integer("decisions", "Número de decisiones", 3), Field::integer("states", "Número de estados", 3), Field::integer("indicators", "Indicadores muestrales (0 = ninguno)", 0)],
                    layout: vec![
                        Widget::matrix(Some("Pagos (decisión × estado)"), "payoffs", Axis::new("decisions", "Decisión").add("Añadir decisión"), states_axis(), Field::number("v", "Pago")),
                        lik.clone(),
                    ],
                },
                ProblemType {
                    id: "bayesian".into(),
                    label: "Análisis bayesiano".into(),
                    description: "Probabilidades a posteriori de los estados según el indicador observado".into(),
                    dims: vec![Field::integer("states", "Número de estados", 2), Field::integer("indicators", "Número de indicadores", 2)],
                    layout: vec![Widget::matrix(
                        Some("P(indicador | estado) y probabilidad a priori"),
                        "likelihoods",
                        Axis::new("indicators", "Indicador").add("Añadir indicador"),
                        states_axis(),
                        Field::number("p", "Probabilidad"),
                    )],
                },
                ProblemType {
                    id: "zero_sum".into(),
                    label: "Juego de suma cero".into(),
                    description: "Estrategias óptimas de dos jugadores; los valores son lo que gana el jugador 1".into(),
                    dims: vec![Field::integer("player1", "Estrategias del jugador 1", 3), Field::integer("player2", "Estrategias del jugador 2", 3)],
                    layout: vec![Widget::matrix(
                        Some("Ganancia del jugador 1"),
                        "game",
                        Axis::new("player1", "Jugador 1").add("Añadir estrategia"),
                        Axis::new("player2", "Jugador 2").add("Añadir estrategia"),
                        Field::number("v", "Ganancia"),
                    )],
                },
                ProblemType {
                    id: "tree".into(),
                    label: "Árbol de decisión".into(),
                    description: "Nodos de decisión y de azar con pagos y probabilidades".into(),
                    dims: vec![Field::integer("nodes", "Número de nodos", 7)],
                    layout: vec![Widget::records(
                        Some("Nodos (el primero sin predecesor es la raíz)"),
                        "nodes",
                        vec![
                            Field::text("name", "Nodo"),
                            Field::choice("kind", "Tipo", &[("decision", "Decisión"), ("chance", "Azar"), ("end", "Final")]),
                            Field::list("next", "Nodos siguientes").help("Nombres o números separados por comas"),
                            Field::number("payoff", "Pago (+ ganancia, − costo)"),
                            Field::number("probability", "Probabilidad").optional().help("Si sale de un nodo de azar"),
                        ],
                        Some("Añadir nodo"),
                    )],
                },
            ],
            options: vec![
                Field::boolean("steps", "Mostrar paso a paso", true),
                Field::number("hurwicz", "Coeficiente de Hurwicz").default(0.5).help("Optimismo, de 0 a 1").show_if("problem_type", &["payoff"]),
                Field::integer("games", "Partidas simuladas", 0).help("0 = no simular").show_if("problem_type", &["zero_sum"]),
                Field::integer("seed", "Semilla", 1).show_if("problem_type", &["zero_sum"]),
            ],
        }
    }

    fn template(&self, pt: &str, dims: &Value) -> Result<DaModel, ModError> {
        let t: DaType = serde_json::from_value(Value::from(pt)).map_err(|_| ModError::Invalid(format!("tipo de problema desconocido: {pt}")))?;
        let mut m = DaModel { name: "Decisión nueva".into(), problem_type: t, states: vec![], decisions: vec![], payoffs: vec![], indicators: vec![], likelihoods: vec![], player1: vec![], player2: vec![], game: vec![], nodes: vec![] };
        let names = |p: &str, n: usize| (1..=n).map(|i| Named { name: format!("{p}{i}") }).collect::<Vec<_>>();
        match t {
            DaType::Payoff | DaType::Bayesian => {
                let s = dim(dims, "states", 3, 1, 200)?;
                m.states = (1..=s).map(|i| State { name: format!("Estado{i}"), prior: 1.0 / s as f64 }).collect();
                let k = dim(dims, "indicators", if t == DaType::Bayesian { 2 } else { 0 }, 0, 200)?;
                m.indicators = names("Indicador", k);
                m.likelihoods = vec![vec![0.0; s]; k];
                if t == DaType::Payoff {
                    let d = dim(dims, "decisions", 3, 1, 200)?;
                    m.decisions = names("Decisión", d);
                    m.payoffs = vec![vec![0.0; s]; d];
                }
            }
            DaType::ZeroSum => {
                let a = dim(dims, "player1", 3, 1, 200)?;
                let b = dim(dims, "player2", 3, 1, 200)?;
                m.player1 = names("A", a);
                m.player2 = names("B", b);
                m.game = vec![vec![0.0; b]; a];
            }
            DaType::Tree => {
                let n = dim(dims, "nodes", 7, 1, 500)?;
                m.nodes = (1..=n).map(|i| DtNode { name: format!("Nodo{i}"), kind: if i == 1 { NodeKind::Decision } else { NodeKind::End }, next: vec![], payoff: 0.0, probability: None }).collect();
            }
        }
        Ok(m)
    }

    fn solve(&self, m: &DaModel, o: &DaOptions) -> Result<Report, ModError> {
        let mut m = m.clone();
        let mut r = match m.problem_type {
            DaType::Payoff => payoff(&mut m, o)?,
            DaType::Bayesian => bayes_report(&mut m, o)?,
            DaType::ZeroSum => zero_sum(&mut m, o)?,
            DaType::Tree => tree(&m, o)?,
        };
        if r.problem.is_empty() {
            r.problem = m.name.clone();
        }
        Ok(r)
    }

    fn read(&self, bytes: &[u8]) -> Result<DaModel, ModError> {
        read_da(bytes)
    }

    fn write(&self, m: &DaModel) -> Result<Vec<u8>, ModError> {
        Ok(write_da(m))
    }

    fn examples(&self) -> Vec<(&'static str, DaModel)> {
        let ex = |s: &str| serde_json::from_str::<DaModel>(s).unwrap();
        vec![
            ("goferbroke", ex(include_str!("ejemplos/goferbroke.json"))),
            ("bayes_sismico", ex(include_str!("ejemplos/bayes.json"))),
            ("juego_campana", ex(include_str!("ejemplos/juego.json"))),
            ("arbol_decision", ex(include_str!("ejemplos/arbol.json"))),
        ]
    }
}

// ------------------------------------------------------------- bayes y pagos

struct Bayes {
    joint: Vec<Vec<f64>>,
    marginal: Vec<f64>,
    posterior: Vec<Vec<f64>>,
}

fn check_probs(m: &DaModel) -> Result<(), ModError> {
    check_names("estados", &m.states.iter().map(|s| &s.name).collect::<Vec<_>>())?;
    if m.states.is_empty() {
        return invalid("hace falta al menos un estado");
    }
    let sum: f64 = m.states.iter().map(|s| s.prior).sum();
    if m.states.iter().any(|s| s.prior < 0.0) || (sum - 1.0).abs() > 1e-6 {
        return invalid(format!("las probabilidades a priori deben ser no negativas y sumar 1 (suman {})", fmt(sum)));
    }
    Ok(())
}

fn bayes(m: &DaModel) -> Result<Option<Bayes>, ModError> {
    let s = m.states.len();
    let k = m.indicators.len();
    if k == 0 {
        return Ok(None);
    }
    for j in 0..s {
        let col: f64 = (0..k).map(|i| m.likelihoods[i][j]).sum();
        if (0..k).any(|i| m.likelihoods[i][j] < 0.0) || (col - 1.0).abs() > 1e-6 {
            return invalid(format!("las probabilidades condicionales del estado {} deben sumar 1 (suman {})", m.states[j].name, fmt(col)));
        }
    }
    let joint: Vec<Vec<f64>> = (0..k).map(|i| (0..s).map(|j| m.likelihoods[i][j] * m.states[j].prior).collect()).collect();
    let marginal: Vec<f64> = joint.iter().map(|r| r.iter().sum()).collect();
    let posterior = (0..k).map(|i| (0..s).map(|j| if marginal[i] > 0.0 { joint[i][j] / marginal[i] } else { 0.0 }).collect()).collect();
    Ok(Some(Bayes { joint, marginal, posterior }))
}

fn prob_tables(m: &DaModel, b: &Bayes) -> Vec<Block> {
    let mut head = vec!["Indicador".to_string()];
    head.extend(m.states.iter().map(|s| s.name.clone()));
    let mut hj = head.clone();
    hj.push("Marginal P(indicador)".into());
    let mut j = Table::new(Some("Probabilidades conjuntas P(indicador y estado)"), &hj).left(0);
    for (i, ind) in m.indicators.iter().enumerate() {
        let mut row = vec![Cell::from(&ind.name)];
        row.extend(b.joint[i].iter().map(|v| Cell::Num(tidy(*v))));
        row.push(Cell::Num(tidy(b.marginal[i])));
        j.row(row);
    }
    let mut p = Table::new(Some("Probabilidades a posteriori P(estado | indicador)"), &head).left(0);
    for (i, ind) in m.indicators.iter().enumerate() {
        let mut row = vec![Cell::from(&ind.name)];
        row.extend(b.posterior[i].iter().map(|v| Cell::Num(tidy(*v))));
        p.row(row);
    }
    vec![j.block(), p.block()]
}

fn bayes_report(m: &mut DaModel, o: &DaOptions) -> Result<Report, ModError> {
    let s = m.states.len();
    fit(&mut m.likelihoods, m.indicators.len(), s);
    check_probs(m)?;
    let b = bayes(m)?.ok_or_else(|| ModError::Invalid("hace falta al menos un indicador".into()))?;
    let mut st = vec![];
    if o.steps {
        st.push(steps(vec![
            step("Probabilidad conjunta", "P(indicador y estado) = P(indicador | estado) × P(estado).", vec![prob_tables(m, &b)[0].clone()]),
            step("Probabilidad marginal", "P(indicador) = suma de las conjuntas de su fila.", vec![]),
            step("Teorema de Bayes", "P(estado | indicador) = P(indicador y estado) / P(indicador).", vec![prob_tables(m, &b)[1].clone()]),
        ]));
    }
    let best: Vec<String> = m
        .indicators
        .iter()
        .enumerate()
        .map(|(i, ind)| {
            let top = b.posterior[i].iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            let js: Vec<&str> = (0..s).filter(|&j| (b.posterior[i][j] - top).abs() <= 1e-9).map(|j| m.states[j].name.as_str()).collect();
            format!("si sale {}, lo más probable es {} ({})", ind.name, js.join(" o "), fmt(tidy(top)))
        })
        .collect();
    Ok(Report::new("da", &m.name, Status::Solved, format!("Probabilidades a posteriori: {}.", best.join("; ")))
        .section("solucion", "Solución", prob_tables(m, &b))
        .section("pasos", "Paso a paso", st)
        .data(json!({"marginal": b.marginal.iter().map(|v| tidy(*v)).collect::<Vec<_>>(), "posterior": b.posterior.iter().map(|r| r.iter().map(|v| tidy(*v)).collect::<Vec<_>>()).collect::<Vec<_>>()})))
}

fn payoff(m: &mut DaModel, o: &DaOptions) -> Result<Report, ModError> {
    let s = m.states.len();
    let d = m.decisions.len();
    fit(&mut m.payoffs, d, s);
    fit(&mut m.likelihoods, m.indicators.len(), s);
    check_probs(m)?;
    check_names("decisiones", &m.decisions.iter().map(|x| &x.name).collect::<Vec<_>>())?;
    if d == 0 {
        return invalid("hace falta al menos una decisión");
    }
    let a = o.hurwicz.clamp(0.0, 1.0);
    let p: Vec<f64> = m.states.iter().map(|x| x.prior).collect();
    let v = &m.payoffs;
    let mins: Vec<f64> = v.iter().map(|r| r.iter().cloned().fold(f64::INFINITY, f64::min)).collect();
    let maxs: Vec<f64> = v.iter().map(|r| r.iter().cloned().fold(f64::NEG_INFINITY, f64::max)).collect();
    let colmax: Vec<f64> = (0..s).map(|j| (0..d).map(|i| v[i][j]).fold(f64::NEG_INFINITY, f64::max)).collect();
    let regret: Vec<Vec<f64>> = (0..d).map(|i| (0..s).map(|j| colmax[j] - v[i][j]).collect()).collect();
    let maxreg: Vec<f64> = regret.iter().map(|r| r.iter().cloned().fold(0.0, f64::max)).collect();
    let hur: Vec<f64> = (0..d).map(|i| a * maxs[i] + (1.0 - a) * mins[i]).collect();
    let avg: Vec<f64> = v.iter().map(|r| r.iter().sum::<f64>() / s as f64).collect();
    let ev: Vec<f64> = v.iter().map(|r| r.iter().zip(&p).map(|(x, q)| x * q).sum()).collect();
    let eol: Vec<f64> = regret.iter().map(|r| r.iter().zip(&p).map(|(x, q)| x * q).sum()).collect();
    let argbest = |vals: &[f64], max: bool| -> Vec<usize> {
        let best = if max { vals.iter().cloned().fold(f64::NEG_INFINITY, f64::max) } else { vals.iter().cloned().fold(f64::INFINITY, f64::min) };
        (0..vals.len()).filter(|&i| (vals[i] - best).abs() <= 1e-9 * best.abs().max(1.0)).collect()
    };
    let names = |ix: &[usize]| ix.iter().map(|&i| m.decisions[i].name.clone()).collect::<Vec<_>>().join(", ");
    let crit: Vec<(&str, &Vec<f64>, bool, String)> = vec![
        ("Maximin (pesimista)", &mins, true, "el peor pago de cada decisión; se elige el mayor".into()),
        ("Maximax (optimista)", &maxs, true, "el mejor pago de cada decisión; se elige el mayor".into()),
        ("Hurwicz", &hur, true, format!("{}·máximo + {}·mínimo", fmt(a), fmt(1.0 - a))),
        ("Mínimo arrepentimiento (Savage)", &maxreg, false, "el mayor arrepentimiento de cada decisión; se elige el menor".into()),
        ("Igual verosimilitud (Laplace)", &avg, true, "promedio de los pagos".into()),
        ("Valor esperado", &ev, true, "pagos ponderados por las probabilidades a priori".into()),
        ("Arrepentimiento esperado", &eol, false, "arrepentimientos ponderados por las probabilidades".into()),
    ];
    let mut head = vec!["Criterio".to_string()];
    head.extend(m.decisions.iter().map(|x| x.name.clone()));
    head.extend(["Decisión elegida".to_string(), "Valor".to_string()]);
    let hl = head.len();
    let mut ct = Table::new(Some("Criterios de decisión"), &head).left(0).left(hl - 2);
    let mut data_crit = vec![];
    for (r, (name, vals, max, _)) in crit.iter().enumerate() {
        let best = argbest(vals, *max);
        let mut row = vec![Cell::from(if *name == "Hurwicz" { format!("Hurwicz (p = {})", fmt(a)) } else { name.to_string() })];
        row.extend(vals.iter().map(|x| Cell::Num(tidy(*x))));
        row.push(Cell::from(names(&best)));
        row.push(Cell::Num(tidy(vals[best[0]])));
        ct.row(row);
        for &b in &best {
            ct.mark(r, b + 1, Tone::Highlight);
        }
        data_crit.push(json!({"criterion": name, "decision": names(&best), "value": tidy(vals[best[0]])}));
    }
    let evstar = ev.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let evwpi: f64 = colmax.iter().zip(&p).map(|(x, q)| x * q).sum();
    let evpi = evwpi - evstar;
    let mut blocks = vec![
        metrics(vec![
            metric_main("Mejor valor esperado", tidy(evstar)),
            metric("Valor esperado con información perfecta", tidy(evwpi)),
            metric("Valor de la información perfecta (VEIP)", tidy(evpi)),
        ]),
        ct.block(),
    ];
    let mut rh = vec!["Decisión".to_string()];
    rh.extend(m.states.iter().map(|x| x.name.clone()));
    let mut rt = Table::new(Some("Tabla de arrepentimiento"), &rh).left(0);
    for i in 0..d {
        let mut row = vec![Cell::from(&m.decisions[i].name)];
        row.extend(regret[i].iter().map(|x| Cell::Num(tidy(*x))));
        rt.row(row);
    }
    blocks.push(rt.block());
    let mut data = json!({"criteria": data_crit, "expected_values": ev, "evpi": tidy(evpi), "ev_with_perfect_information": tidy(evwpi)});

    // información muestral
    let mut sample = vec![];
    let bz = bayes(m)?;
    let mut ev_sample = None;
    if let Some(b) = &bz {
        let mut t = Table::new(Some("Decisión según el indicador"), &["Indicador", "P(indicador)", "Mejor decisión", "Valor esperado a posteriori"]).left(0).left(2);
        let mut evs = 0.0;
        let mut per = vec![];
        for (k, ind) in m.indicators.iter().enumerate() {
            let evk: Vec<f64> = (0..d).map(|i| (0..s).map(|j| b.posterior[k][j] * v[i][j]).sum()).collect();
            let best = argbest(&evk, true);
            evs += b.marginal[k] * evk[best[0]];
            t.row(cells![&ind.name, tidy(b.marginal[k]), names(&best), tidy(evk[best[0]])]);
            per.push(json!({"indicator": ind.name, "decision": names(&best), "expected_value": tidy(evk[best[0]])}));
        }
        let evsi = evs - evstar;
        sample.push(metrics(vec![
            metric_main("Valor esperado con información muestral", tidy(evs)),
            metric("Valor de la información muestral (VEIM)", tidy(evsi)),
            metric("Eficiencia (VEIM / VEIP)", if evpi > 1e-12 { Cell::from(format!("{} %", fmt(tidy(100.0 * evsi / evpi)))) } else { Cell::from("—") }),
        ]));
        sample.push(t.block());
        sample.extend(prob_tables(m, b));
        ev_sample = Some(evs);
        data["sample"] = json!({"ev_with_sample_information": tidy(evs), "evsi": tidy(evsi), "by_indicator": per});
    }

    // árbol de la tabla de pagos
    let mut tn = vec![TreeNode { id: 0, parent: None, edge: None, label: "Decidir".into(), kind: Some("decision".into()), value: Some(fmt(tidy(evstar))), tone: Tone::Normal }];
    let best_ev = argbest(&ev, true);
    for i in 0..d {
        let id = tn.len();
        tn.push(TreeNode { id, parent: Some(0), edge: Some(m.decisions[i].name.clone()), label: format!("VE = {}", fmt(tidy(ev[i]))), kind: Some("chance".into()), value: Some(fmt(tidy(ev[i]))), tone: if best_ev.contains(&i) { Tone::Highlight } else { Tone::Normal } });
        for j in 0..s {
            let k = tn.len();
            tn.push(TreeNode { id: k, parent: Some(id), edge: Some(format!("{} ({})", m.states[j].name, fmt(p[j]))), label: fmt(v[i][j]), kind: Some("end".into()), value: Some(fmt(v[i][j])), tone: Tone::Normal });
        }
    }
    let tree_block = Block::Tree(Tree { title: Some("Árbol de decisión (valor esperado)".into()), nodes: tn });

    let mut st = vec![];
    if o.steps {
        let mut v_steps = vec![];
        for (name, vals, max, how) in &crit {
            let best = argbest(vals, *max);
            let mut t = Table::new(None, &["Decisión", "Valor"]).left(0);
            for i in 0..d {
                t.row_tone(cells![&m.decisions[i].name, tidy(vals[i])], if best.contains(&i) { Tone::Highlight } else { Tone::Normal });
            }
            v_steps.push(step(name.to_string(), format!("Se calcula {how}. Elegida: {}.", names(&best)), vec![t.block()]));
        }
        v_steps.push(step(
            "Valor de la información perfecta",
            format!(
                "Con información perfecta se elegiría la mejor decisión en cada estado: Σ P(estado) × mejor pago = {}. VEIP = {} − {} = {} (coincide con el menor arrepentimiento esperado).",
                fmt(tidy(evwpi)),
                fmt(tidy(evwpi)),
                fmt(tidy(evstar)),
                fmt(tidy(evpi))
            ),
            vec![],
        ));
        if let Some(evs) = ev_sample {
            v_steps.push(step(
                "Valor de la información muestral",
                format!("Para cada indicador se usa la probabilidad a posteriori y se toma la mejor decisión; ponderando por P(indicador) da {}. VEIM = {} − {} = {}.", fmt(tidy(evs)), fmt(tidy(evs)), fmt(tidy(evstar)), fmt(tidy(evs - evstar))),
                vec![],
            ));
        }
        st.push(steps(v_steps));
    }
    let headline = format!("Por valor esperado conviene {} ({}); VEIP = {}.", names(&argbest(&ev, true)), fmt(tidy(evstar)), fmt(tidy(evpi)));
    Ok(Report::new("da", &m.name, Status::Optimal, headline)
        .section("solucion", "Solución", blocks)
        .section("muestral", "Información muestral", sample)
        .section("arbol", "Árbol", vec![tree_block])
        .section("pasos", "Paso a paso", st)
        .data(data))
}

// -------------------------------------------------------- juego de suma cero

/// Estrategia óptima del jugador de las filas: máx v con Σ xᵢ aᵢⱼ ≥ v.
fn game_lp(a: &[Vec<f64>]) -> Option<(Vec<f64>, f64)> {
    let r = a.len();
    let c = a[0].len();
    let mut lp = Lp::new(0, true);
    for _ in 0..r {
        lp.add_var(0.0, 0.0, 1.0, false);
    }
    let v = lp.add_var(1.0, f64::NEG_INFINITY, f64::INFINITY, false);
    for j in 0..c {
        let mut row: Vec<(usize, f64)> = (0..r).map(|i| (i, a[i][j])).collect();
        row.push((v, -1.0));
        lp.row(row, Sense::Ge, 0.0);
    }
    lp.row((0..r).map(|i| (i, 1.0)).collect(), Sense::Eq, 1.0);
    let s = lp_solve(&lp);
    (s.status == LpStatus::Optimal).then(|| ((0..r).map(|i| tidy(s.x[i])).collect(), tidy(s.x[v])))
}

fn zero_sum(m: &mut DaModel, o: &DaOptions) -> Result<Report, ModError> {
    let r = m.player1.len();
    let c = m.player2.len();
    if r == 0 || c == 0 {
        return invalid("cada jugador necesita al menos una estrategia");
    }
    fit(&mut m.game, r, c);
    let a = &m.game;
    let rowmin: Vec<f64> = a.iter().map(|x| x.iter().cloned().fold(f64::INFINITY, f64::min)).collect();
    let colmax: Vec<f64> = (0..c).map(|j| (0..r).map(|i| a[i][j]).fold(f64::NEG_INFINITY, f64::max)).collect();
    let maximin = rowmin.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let minimax = colmax.iter().cloned().fold(f64::INFINITY, f64::min);
    let saddle = (maximin - minimax).abs() <= 1e-9 * maximin.abs().max(1.0);
    let (x, val) = game_lp(a).ok_or_else(|| ModError::Invalid("no se pudo resolver el juego".into()))?;
    let at: Vec<Vec<f64>> = (0..c).map(|j| (0..r).map(|i| -a[i][j]).collect()).collect();
    let (y, _) = game_lp(&at).ok_or_else(|| ModError::Invalid("no se pudo resolver el juego".into()))?;
    let mut head = vec!["Jugador 1 \\ Jugador 2".to_string()];
    head.extend(m.player2.iter().map(|x| x.name.clone()));
    head.push("Mínimo de la fila".into());
    let mut t = Table::new(Some("Matriz de pagos"), &head).left(0);
    for i in 0..r {
        let mut row = vec![Cell::from(&m.player1[i].name)];
        row.extend(a[i].iter().map(|v| Cell::Num(*v)));
        row.push(Cell::Num(rowmin[i]));
        t.row(row);
        if saddle {
            for j in 0..c {
                if (a[i][j] - maximin).abs() <= 1e-9 && (rowmin[i] - maximin).abs() <= 1e-9 && (colmax[j] - minimax).abs() <= 1e-9 {
                    t.mark(i, j + 1, Tone::Highlight);
                }
            }
        }
    }
    let mut last = vec![Cell::from("Máximo de la columna")];
    last.extend(colmax.iter().map(|v| Cell::Num(*v)));
    last.push(Cell::Empty);
    t.row_tone(last, Tone::Total);
    let mut pt = Table::new(Some("Estrategias óptimas"), &["Jugador", "Estrategia", "Probabilidad"]).left(0).left(1);
    for i in 0..r {
        pt.row_tone(cells!["Jugador 1", &m.player1[i].name, x[i]], if x[i] > 1e-9 { Tone::Accent } else { Tone::Normal });
    }
    for j in 0..c {
        pt.row_tone(cells!["Jugador 2", &m.player2[j].name, y[j]], if y[j] > 1e-9 { Tone::Accent } else { Tone::Normal });
    }
    let kind = if saddle {
        format!("Hay punto de silla: maximin = minimax = {}. Las estrategias óptimas son puras.", fmt(maximin))
    } else {
        format!("No hay punto de silla (maximin {} < minimax {}): las estrategias óptimas son mixtas.", fmt(maximin), fmt(minimax))
    };
    let mut blocks = vec![metrics(vec![metric_main("Valor del juego", val), metric("Maximin", maximin), metric("Minimax", minimax)]), text(kind), t.clone().block(), pt.clone().block()];
    if r == 2 && !saddle {
        // método gráfico: ganancia esperada del jugador 1 según la probabilidad de su primera estrategia
        let mut ch = Chart::new(Some("Método gráfico (jugador 1)"), &format!("Probabilidad de {}", m.player1[0].name), "Ganancia esperada");
        for j in 0..c {
            ch = ch.series(Series::new(m.player2[j].name.clone(), SeriesKind::Line, vec![[0.0, a[1][j]], [1.0, a[0][j]]]));
        }
        let env: Vec<[f64; 2]> = (0..=100).map(|k| {
            let q = k as f64 / 100.0;
            [q, (0..c).map(|j| q * a[0][j] + (1.0 - q) * a[1][j]).fold(f64::INFINITY, f64::min)]
        }).collect();
        ch = ch.series(Series::new("Envolvente inferior", SeriesKind::Line, env).tone(Tone::Highlight).dashed());
        ch = ch.vline(x[0], format!("óptimo {}", fmt(x[0])), Tone::Accent);
        blocks.push(ch.block());
    }
    let mut data = json!({"value": val, "saddle_point": saddle, "player1": x, "player2": y});
    let mut sim = vec![];
    if o.games > 0 {
        let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(o.seed);
        let pick = |rng: &mut rand_chacha::ChaCha8Rng, p: &[f64]| -> usize {
            let u: f64 = rng.random();
            let mut acc = 0.0;
            for (k, q) in p.iter().enumerate() {
                acc += q;
                if u < acc {
                    return k;
                }
            }
            p.len() - 1
        };
        let n = o.games.min(1_000_000) as usize;
        let mut total = 0.0;
        let mut count = vec![vec![0usize; c]; r];
        for _ in 0..n {
            let i = pick(&mut rng, &x);
            let j = pick(&mut rng, &y);
            total += a[i][j];
            count[i][j] += 1;
        }
        let avg = total / n as f64;
        let mut ft = Table::new(Some("Frecuencia de cada jugada"), &std::iter::once("Jugador 1 \\ Jugador 2".to_string()).chain(m.player2.iter().map(|x| x.name.clone())).collect::<Vec<_>>()).left(0);
        for i in 0..r {
            let mut row = vec![Cell::from(&m.player1[i].name)];
            row.extend(count[i].iter().map(|v| Cell::Num(*v as f64)));
            ft.row(row);
        }
        sim.push(metrics(vec![metric_main("Ganancia media del jugador 1", avg), metric("Valor teórico", val), metric("Partidas", n)]));
        sim.push(ft.block());
        data["simulation"] = json!({"games": n, "average": avg});
    }
    let mut st = vec![];
    if o.steps {
        let mut v = vec![step(
            "Punto de silla",
            format!("Mínimo de cada fila (lo peor para el jugador 1) y máximo de cada columna (lo peor para el jugador 2). Maximin = {}, minimax = {}. {}", fmt(maximin), fmt(minimax), if saddle { "Coinciden: punto de silla." } else { "No coinciden: hay que mezclar estrategias." }),
            vec![t.block()],
        )];
        // dominancia
        let mut alive_r: Vec<bool> = vec![true; r];
        let mut alive_c: Vec<bool> = vec![true; c];
        let mut log = vec![];
        loop {
            let mut changed = false;
            for i in 0..r {
                if !alive_r[i] {
                    continue;
                }
                if let Some(k) = (0..r).find(|&k| k != i && alive_r[k] && (0..c).filter(|&j| alive_c[j]).all(|j| a[k][j] >= a[i][j]) && (0..c).filter(|&j| alive_c[j]).any(|j| a[k][j] > a[i][j])) {
                    alive_r[i] = false;
                    log.push(format!("{} domina a {} (jugador 1)", m.player1[k].name, m.player1[i].name));
                    changed = true;
                }
            }
            for j in 0..c {
                if !alive_c[j] {
                    continue;
                }
                if let Some(k) = (0..c).find(|&k| k != j && alive_c[k] && (0..r).filter(|&i| alive_r[i]).all(|i| a[i][k] <= a[i][j]) && (0..r).filter(|&i| alive_r[i]).any(|i| a[i][k] < a[i][j])) {
                    alive_c[j] = false;
                    log.push(format!("{} domina a {} (jugador 2)", m.player2[k].name, m.player2[j].name));
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }
        v.push(step(
            "Dominancia",
            if log.is_empty() { "Ninguna estrategia está dominada.".to_string() } else { format!("Se eliminan estrategias dominadas: {}.", log.join("; ")) },
            vec![],
        ));
        v.push(step(
            "Programa lineal",
            "Jugador 1: máx v sujeto a Σᵢ xᵢ·aᵢⱼ ≥ v para cada estrategia j del jugador 2, Σ xᵢ = 1, x ≥ 0. El jugador 2 resuelve el dual. Lo resuelve HiGHS.",
            vec![pt.block()],
        ));
        st.push(steps(v));
    }
    Ok(Report::new("da", &m.name, Status::Optimal, format!("Valor del juego {}; jugador 1: {}.", fmt(val), m.player1.iter().zip(&x).filter(|(_, p)| **p > 1e-9).map(|(s, p)| format!("{} {}", s.name, fmt(*p))).collect::<Vec<_>>().join(", ")))
        .section("solucion", "Solución", blocks)
        .section("simulacion", "Simulación", sim)
        .section("pasos", "Paso a paso", st)
        .data(data))
}

// ------------------------------------------------------------ árbol de decisión

fn tree(m: &DaModel, o: &DaOptions) -> Result<Report, ModError> {
    let n = m.nodes.len();
    if n == 0 {
        return invalid("el árbol no tiene nodos");
    }
    // En WinQSB los nombres pueden repetirse (p. ej. «High» bajo varias ramas): los sucesores se dan por número.
    let raw_names: Vec<&str> = m.nodes.iter().map(|x| x.name.as_str()).collect();
    let dup = |nm: &str| raw_names.iter().filter(|x| x.eq_ignore_ascii_case(nm)).count() > 1;
    let shown: Vec<String> = (0..n).map(|i| if dup(raw_names[i]) { format!("{} (#{})", raw_names[i], i + 1) } else { raw_names[i].to_string() }).collect();
    let names: Vec<&str> = shown.iter().map(String::as_str).collect();
    let mut notes = vec![];
    let mut kids = vec![vec![]; n];
    let mut has_parent = vec![false; n];
    for (i, nd) in m.nodes.iter().enumerate() {
        for raw in &nd.next {
            for s in raw.split([',', ';']).map(str::trim).filter(|s| !s.is_empty()) {
                let hits: Vec<usize> = (0..n).filter(|&k| raw_names[k].eq_ignore_ascii_case(s)).collect();
                let by_num = s.parse::<usize>().ok().filter(|k| *k >= 1 && *k <= n).map(|k| k - 1);
                let j = match (hits.len(), by_num) {
                    (1, _) => hits[0],
                    (0, Some(k)) => k,
                    (0, None) => return invalid(format!("el nodo «{s}» (siguiente de {}) no existe", names[i])),
                    (_, Some(k)) => k,
                    _ => return invalid(format!("el nombre «{s}» (siguiente de {}) se repite: indica el nodo por su número", names[i])),
                };
                if has_parent[j] {
                    return invalid(format!("el nodo {} tiene más de un predecesor: en un árbol cada nodo sale de uno solo", names[j]));
                }
                has_parent[j] = true;
                kids[i].push(j);
            }
        }
    }
    let roots: Vec<usize> = (0..n).filter(|&i| !has_parent[i]).collect();
    if roots.is_empty() {
        return invalid("el árbol no tiene nodo inicial (uno sin predecesor)");
    }
    let root = roots[0];
    for (i, nd) in m.nodes.iter().enumerate() {
        if nd.kind == NodeKind::Chance && !kids[i].is_empty() {
            let s: f64 = kids[i].iter().map(|&k| m.nodes[k].probability.unwrap_or(0.0)).sum();
            if (s - 1.0).abs() > 0.05 {
                return invalid(format!("las probabilidades que salen del nodo de azar {} suman {}, no 1", names[i], fmt(s)));
            } else if (s - 1.0).abs() > 1e-6 {
                notes.push(format!("Las probabilidades del nodo de azar {} suman {} (no 1); se usan tal como están, igual que WinQSB.", names[i], fmt(tidy(s))));
            }
        }
    }
    // valores hacia atrás
    let mut val = vec![f64::NAN; n];
    let mut best = vec![vec![]; n];
    fn eval(i: usize, m: &DaModel, kids: &[Vec<usize>], val: &mut Vec<f64>, best: &mut Vec<Vec<usize>>, depth: usize) -> f64 {
        if depth > 10_000 {
            return f64::NAN;
        }
        let nd = &m.nodes[i];
        let v = if kids[i].is_empty() {
            0.0
        } else {
            let cv: Vec<f64> = kids[i].iter().map(|&k| eval(k, m, kids, val, best, depth + 1)).collect();
            match nd.kind {
                NodeKind::Chance => kids[i].iter().zip(&cv).map(|(&k, v)| m.nodes[k].probability.unwrap_or(0.0) * v).sum(),
                _ => {
                    let b = cv.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
                    best[i] = kids[i].iter().zip(&cv).filter(|(_, v)| (**v - b).abs() <= 1e-9 * b.abs().max(1.0)).map(|(k, _)| *k).collect();
                    b
                }
            }
        };
        val[i] = nd.payoff + v;
        val[i]
    }
    let total = eval(root, m, &kids, &mut val, &mut best, 0);
    // nodos del dibujo y camino óptimo
    let mut on_path = vec![false; n];
    fn mark(i: usize, m: &DaModel, kids: &[Vec<usize>], best: &[Vec<usize>], on: &mut Vec<bool>) {
        on[i] = true;
        let next: Vec<usize> = if m.nodes[i].kind == NodeKind::Chance { kids[i].clone() } else { best[i].first().copied().into_iter().collect() };
        for k in next {
            mark(k, m, kids, best, on);
        }
    }
    mark(root, m, &kids, &best, &mut on_path);
    let mut tn = vec![];
    let mut order = vec![(root, None::<usize>)];
    let mut map = vec![usize::MAX; n];
    while let Some((i, parent)) = order.pop() {
        let id = tn.len();
        map[i] = id;
        let nd = &m.nodes[i];
        let edge = parent.map(|p| {
            if m.nodes[p].kind == NodeKind::Chance {
                format!("{} ({})", nd.name, fmt(nd.probability.unwrap_or(0.0)))
            } else {
                nd.name.clone()
            }
        });
        tn.push(TreeNode {
            id,
            parent: parent.map(|p| map[p]),
            edge,
            label: if nd.payoff != 0.0 { format!("{} [{}]", names[i], fmt(nd.payoff)) } else { names[i].to_string() },
            kind: Some(if kids[i].is_empty() { "end" } else if nd.kind == NodeKind::Chance { "chance" } else { "decision" }.into()),
            value: Some(fmt(tidy(val[i]))),
            tone: if on_path[i] && parent.map(|p| m.nodes[p].kind != NodeKind::Chance).unwrap_or(false) { Tone::Highlight } else { Tone::Normal },
        });
        for &k in kids[i].iter().rev() {
            order.push((k, Some(i)));
        }
    }
    let mut t = Table::new(Some("Análisis del árbol"), &["Nodo", "Tipo", "Pago del nodo", "Valor esperado", "Decisión óptima"]).left(0).left(1).left(4);
    for i in 0..n {
        let kind = if kids[i].is_empty() { "final" } else if m.nodes[i].kind == NodeKind::Chance { "azar" } else { "decisión" };
        let dec = best[i].iter().map(|&k| names[k]).collect::<Vec<_>>().join(", ");
        t.row_tone(cells![names[i], kind, m.nodes[i].payoff, tidy(val[i]), dec], if on_path[i] { Tone::Highlight } else { Tone::Normal });
    }
    let mut st = vec![];
    if o.steps {
        st.push(steps(vec![step(
            "Análisis hacia atrás",
            "Desde las hojas hacia la raíz: un nodo de azar vale la suma de probabilidad × valor de sus ramas; uno de decisión, la mejor de sus ramas; a cada nodo se le suma su propio pago (costo negativo).",
            vec![t.clone().block()],
        )]));
    }
    let first = best[root].iter().map(|&k| names[k]).collect::<Vec<_>>().join(", ");
    let mut sol = vec![metrics(vec![metric_main("Valor esperado", tidy(total))]), t.block()];
    for nt in notes.into_iter().rev() {
        sol.insert(0, text_tone(nt, Tone::Accent));
    }
    Ok(Report::new("da", &m.name, Status::Optimal, format!("Valor esperado {}{}.", fmt(tidy(total)), if first.is_empty() { String::new() } else { format!("; en {} conviene {}", names[root], first) }))
        .section("solucion", "Solución", sol)
        .section("arbol", "Árbol", vec![Block::Tree(Tree { title: Some("Árbol de decisión".into()), nodes: tn })])
        .section("pasos", "Paso a paso", st)
        .data(json!({"expected_value": tidy(total), "values": (0..n).map(|i| json!({"node": names[i], "value": tidy(val[i]), "best": best[i].iter().map(|&k| names[k]).collect::<Vec<_>>()})).collect::<Vec<_>>()})))
}

// ------------------------------------------------------------------ archivos

fn read_da(bytes: &[u8]) -> Result<DaModel, ModError> {
    let mut r = Reader::new(bytes);
    let mut h = r.line("la cabecera")?;
    if !h.next().trim().to_ascii_uppercase().starts_with("DA") {
        return Err(ModError::Format("no es un archivo de DA".into()));
    }
    let name = h.next().trim().to_string();
    let code = h.next().trim().to_ascii_uppercase();
    let t = match code.get(..2).unwrap_or("") {
        "BA" | "0" => DaType::Bayesian,
        "ZS" | "2" => DaType::ZeroSum,
        "DT" | "3" => DaType::Tree,
        _ => DaType::Payoff,
    };
    let mut m = DaModel { name: if name.is_empty() { "Decision Problem".into() } else { name }, problem_type: t, states: vec![], decisions: vec![], payoffs: vec![], indicators: vec![], likelihoods: vec![], player1: vec![], player2: vec![], game: vec![], nodes: vec![] };
    let rest: Vec<usize> = (0..3).map(|_| parse_num(&h.next()).unwrap_or(0.0) as usize).collect();
    let nm = |s: &str, d: String| Some(s.trim()).filter(|x| !x.is_empty()).map(str::to_string).unwrap_or(d);
    match t {
        DaType::Bayesian | DaType::Payoff => {
            let (s, dec, ind) = if t == DaType::Bayesian { (rest[0], 0, rest[1]) } else { (rest[0], rest[1], rest[2]) };
            if s == 0 || s > 10_000 {
                return Err(ModError::Format("el número de estados no es válido".into()));
            }
            let head = r.line("los nombres de los estados")?;
            let pri = r.line("las probabilidades a priori")?;
            m.states = (0..s).map(|j| State { name: nm(head.get(j + 1), format!("State{}", j + 1)), prior: parse_num(pri.get(j + 1)).unwrap_or(0.0) }).collect();
            let k = if t == DaType::Payoff && ind <= 1 { 0 } else { ind };
            for i in 0..k {
                let l = r.line(&format!("el indicador {}", i + 1))?;
                m.indicators.push(Named { name: nm(l.get(0), format!("Indicator{}", i + 1)) });
                m.likelihoods.push((0..s).map(|j| parse_num(l.get(j + 1)).unwrap_or(0.0)).collect());
            }
            for i in 0..dec {
                let l = r.line(&format!("la decisión {}", i + 1))?;
                m.decisions.push(Named { name: nm(l.get(0), format!("Decision{}", i + 1)) });
                m.payoffs.push((0..s).map(|j| parse_num(l.get(j + 1)).unwrap_or(0.0)).collect());
            }
        }
        DaType::ZeroSum => {
            let (a, b) = (rest[0], rest[1]);
            if a == 0 || b == 0 || a > 10_000 || b > 10_000 {
                return Err(ModError::Format("el número de estrategias no es válido".into()));
            }
            let head = r.line("las estrategias del jugador 2")?;
            m.player2 = (0..b).map(|j| Named { name: nm(head.get(j + 1), format!("Strategy2-{}", j + 1)) }).collect();
            for i in 0..a {
                let l = r.line(&format!("la estrategia {} del jugador 1", i + 1))?;
                m.player1.push(Named { name: nm(l.get(0), format!("Strategy1-{}", i + 1)) });
                m.game.push((0..b).map(|j| parse_num(l.get(j + 1)).unwrap_or(0.0)).collect());
            }
        }
        DaType::Tree => {
            let n = rest[0];
            if n == 0 || n > 100_000 {
                return Err(ModError::Format("el número de nodos no es válido".into()));
            }
            r.line("la fila de títulos")?;
            for i in 0..n {
                let l = r.line(&format!("el nodo {}", i + 1))?;
                let kind = match l.get(2).trim().chars().next().map(|c| c.to_ascii_uppercase()) {
                    Some('D') => NodeKind::Decision,
                    Some('C') => NodeKind::Chance,
                    _ => NodeKind::End,
                };
                m.nodes.push(DtNode {
                    name: nm(l.get(1), format!("Node{}", i + 1)),
                    kind,
                    next: l.get(3).split(',').map(str::trim).filter(|s| !s.is_empty()).map(str::to_string).collect(),
                    payoff: parse_num(l.get(4)).unwrap_or(0.0),
                    probability: parse_num(l.get(5)),
                });
            }
        }
    }
    Ok(m)
}

fn write_da(m: &DaModel) -> Vec<u8> {
    let mut w = Writer::new();
    match m.problem_type {
        DaType::Bayesian | DaType::Payoff => {
            let s = m.states.len();
            let bay = m.problem_type == DaType::Bayesian;
            let mut h = vec!["DA".to_string(), m.name.clone(), if bay { "BA" } else { "PT" }.into(), s.to_string()];
            if !bay {
                h.push(m.decisions.len().to_string());
            }
            h.push(m.indicators.len().to_string());
            w.row(&h);
            let mut r = vec![if bay { "Indicator \\ State" } else { "Decision \\ State" }.to_string()];
            r.extend(m.states.iter().map(|x| x.name.clone()));
            w.row(&r);
            let mut r = vec!["Prior Probability".to_string()];
            r.extend(m.states.iter().map(|x| num(x.prior)));
            w.row(&r);
            for (i, ind) in m.indicators.iter().enumerate() {
                let mut r = vec![ind.name.clone()];
                r.extend((0..s).map(|j| num(m.likelihoods.get(i).and_then(|x| x.get(j)).copied().unwrap_or(0.0))));
                w.row(&r);
            }
            for (i, d) in m.decisions.iter().enumerate() {
                let mut r = vec![d.name.clone()];
                r.extend((0..s).map(|j| num(m.payoffs.get(i).and_then(|x| x.get(j)).copied().unwrap_or(0.0))));
                w.row(&r);
            }
        }
        DaType::ZeroSum => {
            w.row(&["DA".to_string(), m.name.clone(), "ZS".into(), m.player1.len().to_string(), m.player2.len().to_string()]);
            let mut r = vec!["Player1 \\ Player2".to_string()];
            r.extend(m.player2.iter().map(|x| x.name.clone()));
            w.row(&r);
            for (i, p) in m.player1.iter().enumerate() {
                let mut r = vec![p.name.clone()];
                r.extend((0..m.player2.len()).map(|j| num(m.game.get(i).and_then(|x| x.get(j)).copied().unwrap_or(0.0))));
                w.row(&r);
            }
        }
        DaType::Tree => {
            w.row(&["DA".to_string(), m.name.clone(), "DT".into(), m.nodes.len().to_string()]);
            w.row(&["Node/Event Number", "Node Name", "Node Type", "Immediate Following Node", "Node Payoff", "Probability"]);
            for (i, nd) in m.nodes.iter().enumerate() {
                let kind = match nd.kind {
                    NodeKind::Decision => "D",
                    NodeKind::Chance => "C",
                    NodeKind::End => " ",
                };
                // WinQSB espera números de nodo en «Immediate Following Node».
                let nexts: Vec<String> = nd
                    .next
                    .iter()
                    .flat_map(|raw| raw.split([',', ';']).map(str::trim).filter(|s| !s.is_empty()).map(str::to_string).collect::<Vec<_>>())
                    .map(|s| {
                        let hits: Vec<usize> = (0..m.nodes.len()).filter(|&k| m.nodes[k].name.eq_ignore_ascii_case(&s)).collect();
                        if hits.len() == 1 { (hits[0] + 1).to_string() } else { s }
                    })
                    .collect();
                w.row(&[(i + 1).to_string(), nd.name.clone(), kind.into(), nexts.join(", "), num(nd.payoff), nd.probability.map(num).unwrap_or_else(|| " ".into())]);
            }
        }
    }
    w.bytes()
}
