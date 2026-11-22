//! NLP · Programación no lineal (equivalente al módulo NLP de WinQSB).
//!
//! Función objetivo y restricciones escritas como expresiones (las mismas
//! funciones que WinQSB: EXP, LOG, SQR, SIN, ATN, INT, MOD...), cotas y punto
//! inicial por variable.
//!
//! El resultado lo calcula una programación cuadrática sucesiva (SQP) con
//! derivadas exactas y varios arranques. Si el problema es convexo (cuadrático
//! con restricciones convexas) el óptimo es global; si no, se avisa de que es el
//! mejor óptimo local encontrado. El paso a paso muestra el método de
//! penalización de WinQSB y las condiciones de Karush-Kuhn-Tucker; para dos
//! variables, un gráfico con curvas de nivel, restricciones y región factible.

use crate::nlpsolve::{self, default_start, Nlp};
use crate::plot2d;
use crate::qpsolve::eigen_range;
use iaqsb_core::expr::{quadratic, split_relation, Expr, Vars};
use iaqsb_core::linear::{apply_bounds, Bnd, Dir, Var};
use iaqsb_core::winqsb::{num, parse_num, Reader, Writer};
use iaqsb_core::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

fn zero() -> Bnd {
    Bnd(0.0)
}
fn inf() -> Bnd {
    Bnd::INF
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct NlpVar {
    pub name: String,
    #[serde(default = "zero")]
    pub lower: Bnd,
    #[serde(default = "inf")]
    pub upper: Bnd,
    /// Punto de partida (opcional).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub initial: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct NlpConstraint {
    pub name: String,
    /// Restricción completa, con su relación: "X1^2 + X2 <= 10".
    pub expression: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct NlpModel {
    #[serde(default)]
    pub name: String,
    #[serde(default = "nlp_type")]
    pub problem_type: String,
    #[serde(default)]
    pub objective: Objective,
    /// Función objetivo: "100*(X2 - X1^2)^2 + (1 - X1)^2".
    pub function: String,
    #[serde(default)]
    pub constraints: Vec<NlpConstraint>,
    pub variables: Vec<NlpVar>,
}

fn nlp_type() -> String {
    "nonlinear".into()
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct NlpOptions {
    pub steps: bool,
    /// Arranques (puntos iniciales distintos) para buscar el mejor óptimo.
    pub starts: u32,
    /// Punto a analizar: "1, 2" o "X1 = 1, X2 = 2".
    pub evaluate: Option<String>,
}

impl Default for NlpOptions {
    fn default() -> Self {
        NlpOptions { steps: true, starts: 8, evaluate: None }
    }
}

struct Compiled {
    names: Vec<String>,
    f: Expr,
    cons: Vec<(String, Expr, Dir, String)>,
    lb: Vec<f64>,
    ub: Vec<f64>,
}

fn compile(m: &NlpModel) -> Result<Compiled, ModError> {
    check_names("variables", &m.variables.iter().map(|v| &v.name).collect::<Vec<_>>())?;
    check_names("restricciones", &m.constraints.iter().map(|c| &c.name).collect::<Vec<_>>())?;
    if m.variables.is_empty() {
        return invalid("el modelo no tiene variables");
    }
    let names: Vec<String> = m.variables.iter().map(|v| v.name.trim().to_string()).collect();
    let mut vars = Vars::fixed(&names);
    let unknown = |e: iaqsb_core::expr::ExprError, at: &str| -> ModError {
        let msg = e.0.replace("variable desconocida", "variable desconocida (añádela a la lista de variables)");
        ModError::Invalid(format!("{at}: {msg}"))
    };
    if m.function.trim().is_empty() {
        return invalid("falta la función objetivo");
    }
    let f = Expr::parse(&m.function, &mut vars).map_err(|e| unknown(e, "función objetivo"))?;
    let mut cons = vec![];
    for c in &m.constraints {
        let txt: String = c.expression.trim().to_string();
        let (lhs, rel, rhs) = split_relation(&txt).ok_or_else(|| ModError::Invalid(format!("restricción {}: falta <=, >= o =", c.name)))?;
        let e = Expr::parse(&format!("({lhs}) - ({rhs})"), &mut vars).map_err(|e| unknown(e, &format!("restricción {}", c.name)))?;
        cons.push((c.name.clone(), e, Dir::parse(rel).unwrap_or(Dir::Le), txt));
    }
    let lb: Vec<f64> = m.variables.iter().map(|v| v.lower.0).collect();
    let ub: Vec<f64> = m.variables.iter().map(|v| v.upper.0).collect();
    for (j, v) in m.variables.iter().enumerate() {
        if lb[j] > ub[j] {
            return invalid(format!("la variable {} tiene cota inferior mayor que la superior", v.name));
        }
    }
    Ok(Compiled { names, f, cons, lb, ub })
}

/// ¿Problema convexo? (todo cuadrático, con la curvatura adecuada)
fn convexity(c: &Compiled, maximize: bool) -> Option<bool> {
    let n = c.names.len();
    let q = quadratic(&c.f, n).ok()?;
    let (lo, hi) = eigen_range(&q.q);
    let obj_ok = if maximize { hi <= 1e-9 * lo.abs().max(1.0) } else { lo >= -1e-9 * hi.abs().max(1.0) };
    if !obj_ok {
        return Some(false);
    }
    for (_, e, d, _) in &c.cons {
        let g = quadratic(e, n).ok()?;
        let (lo, hi) = eigen_range(&g.q);
        let ok = match d {
            Dir::Le => lo >= -1e-9 * hi.abs().max(1.0),
            Dir::Ge => hi <= 1e-9 * lo.abs().max(1.0),
            Dir::Eq => g.is_linear(),
        };
        if !ok {
            return Some(false);
        }
    }
    Some(true)
}

fn parse_point(s: &str, names: &[String]) -> Result<Vec<f64>, ModError> {
    let parts: Vec<&str> = s.split([',', ';']).map(str::trim).filter(|p| !p.is_empty()).collect();
    let mut x = vec![0.0; names.len()];
    if parts.iter().all(|p| p.contains('=')) {
        for p in parts {
            let (a, b) = p.split_once('=').unwrap();
            let j = names.iter().position(|n| n.eq_ignore_ascii_case(a.trim())).ok_or_else(|| ModError::Invalid(format!("punto a analizar: no existe la variable {}", a.trim())))?;
            x[j] = parse_num(b).ok_or_else(|| ModError::Invalid(format!("punto a analizar: «{b}» no es un número")))?;
        }
    } else {
        if parts.len() != names.len() {
            return invalid(format!("punto a analizar: hacen falta {} valores", names.len()));
        }
        for (j, p) in parts.iter().enumerate() {
            x[j] = parse_num(p).ok_or_else(|| ModError::Invalid(format!("punto a analizar: «{p}» no es un número")))?;
        }
    }
    Ok(x)
}

/// Tabla de restricciones evaluadas en x.
fn cons_table(c: &Compiled, x: &[f64], lambda: Option<&[f64]>, sign: f64, title: &str) -> (Table, f64) {
    let mut head = vec!["Restricción", "Expresión", "Lado izquierdo − derecho", "Holgura", "Violación", "Estado"];
    if lambda.is_some() {
        head.push("Precio sombra");
    }
    let mut t = Table::new(Some(title), &head).left(0).left(1).left(5);
    let mut maxv: f64 = 0.0;
    for (k, (name, e, d, txt)) in c.cons.iter().enumerate() {
        let v = e.eval(x);
        let (slack, viol) = match d {
            Dir::Le => ((-v).max(0.0), v.max(0.0)),
            Dir::Ge => (v.max(0.0), (-v).max(0.0)),
            Dir::Eq => (0.0, v.abs()),
        };
        maxv = maxv.max(viol);
        let tol = 1e-6;
        let st = if viol > tol {
            "violada"
        } else if slack <= tol {
            "activa"
        } else {
            "con holgura"
        };
        let mut row = cells![name, txt, tidy(v), tidy(slack), tidy(viol), st];
        if let Some(l) = lambda {
            // λ en forma c ≤ 0; para ≥ se cambió el signo: d(f*)/d(rhs)
            let lam = l[k];
            let sp = match d {
                Dir::Ge => sign * lam,
                _ => -sign * lam,
            };
            row.push(Cell::Num(tidy(sp)));
        }
        let tone = if viol > tol {
            Tone::Bad
        } else if slack <= tol {
            Tone::Highlight
        } else {
            Tone::Normal
        };
        t.row_tone(row, tone);
    }
    (t, maxv)
}

pub struct NlpMod;

impl TypedModule for NlpMod {
    type Model = NlpModel;
    type Options = NlpOptions;

    fn file_tag(&self) -> &'static str {
        "NLP"
    }

    fn spec(&self) -> ModuleSpec {
        ModuleSpec {
            id: "nlp".into(),
            winqsb: "NLP".into(),
            name: "Programación no lineal".into(),
            description: "Funciones no lineales con restricciones, cotas y punto inicial; gráfico para dos variables".into(),
            file_ext: "NLP".into(),
            problem_types: vec![ProblemType {
                id: "nonlinear".into(),
                label: "Programa no lineal".into(),
                description: "Maximizar o minimizar una función cualquiera con restricciones".into(),
                dims: vec![
                    Field::choice("objective", "Criterio", &[("min", "Minimizar"), ("max", "Maximizar")]),
                    Field::integer("variables", "Número de variables", 2),
                    Field::integer("constraints", "Número de restricciones", 1),
                ],
                layout: vec![
                    Widget::params(
                        None,
                        vec![
                            Field::choice("objective", "Criterio", &[("min", "Minimizar"), ("max", "Maximizar")]),
                            Field::expr("function", "Función objetivo").help("Ej.: 100*(X2 - X1^2)^2 + (1 - X1)^2. Funciones: EXP, LOG (ln), SQR, SIN, COS, ATN, ABS..."),
                        ],
                    ),
                    Widget::records(
                        Some("Restricciones"),
                        "constraints",
                        vec![Field::text("name", "Nombre"), Field::expr("expression", "Restricción").help("Con su relación: X1^2 + X2 <= 10")],
                        Some("Añadir restricción"),
                    ),
                    Widget::records(
                        Some("Variables"),
                        "variables",
                        vec![
                            Field::text("name", "Variable"),
                            Field::bound("lower", "Cota inferior", Value::from(0.0)),
                            Field::bound("upper", "Cota superior", Value::from("M")),
                            Field::number("initial", "Punto inicial").optional(),
                        ],
                        Some("Añadir variable"),
                    ),
                ],
            }],
            options: vec![
                Field::boolean("steps", "Mostrar paso a paso", true),
                Field::integer("starts", "Arranques", 8).help("Puntos iniciales distintos para buscar el mejor óptimo"),
                Field::text("evaluate", "Analizar el punto").optional().help("Ej.: 1, 2  o  X1 = 1, X2 = 2"),
            ],
        }
    }

    fn template(&self, _pt: &str, dims: &Value) -> Result<NlpModel, ModError> {
        let n = dim(dims, "variables", 2, 1, 200)?;
        let m = dim(dims, "constraints", 1, 0, 500)?;
        Ok(NlpModel {
            name: "No lineal nuevo".into(),
            problem_type: nlp_type(),
            objective: Objective::parse(&dim_str(dims, "objective", "min")),
            function: (1..=n).map(|j| format!("X{j}^2")).collect::<Vec<_>>().join(" + "),
            constraints: (1..=m).map(|i| NlpConstraint { name: format!("C{i}"), expression: format!("{} >= 1", (1..=n).map(|j| format!("X{j}")).collect::<Vec<_>>().join(" + ")) }).collect(),
            variables: (1..=n).map(|j| NlpVar { name: format!("X{j}"), lower: Bnd(0.0), upper: Bnd::INF, initial: None }).collect(),
        })
    }

    fn solve(&self, m: &NlpModel, o: &NlpOptions) -> Result<Report, ModError> {
        let c = compile(m)?;
        let n = c.names.len();
        let maximize = m.objective.is_max();
        let sign = if maximize { -1.0 } else { 1.0 };
        let p = Nlp { f: c.f.clone(), cons: c.cons.iter().map(|(_, e, d, _)| (e.clone(), *d)).collect(), lb: c.lb.clone(), ub: c.ub.clone(), sign };
        let mut x0 = default_start(&c.lb, &c.ub);
        for (j, v) in m.variables.iter().enumerate() {
            if let Some(i) = v.initial {
                x0[j] = i.clamp(c.lb[j], c.ub[j]);
            }
        }
        let convex = convexity(&c, maximize);
        let starts = if convex == Some(true) { 1 } else { o.starts.clamp(1, 200) as usize };
        let (s, feasible_runs) = nlpsolve::solve(&p, &x0, starts, 7);
        let word = if maximize { "máximo" } else { "mínimo" };
        if s.unbounded {
            return Ok(Report::new("nlp", &m.name, Status::Unbounded, format!("La función no tiene {word}: puede {} sin límite.", if maximize { "crecer" } else { "decrecer" }))
                .section("solucion", "Solución", vec![text_tone("Los iterados se alejan sin límite mejorando el objetivo: añade cotas o restricciones.", Tone::Bad)])
                .data(json!({"status": "unbounded"})));
        }
        // redondeo del ruido solo si no cambia la factibilidad ni el valor
        let xt: Vec<f64> = s.x.iter().map(|v| tidy(*v)).collect();
        let x: Vec<f64> = if p.violation(&xt) <= p.violation(&s.x) + 1e-12 && (c.f.eval(&xt) - c.f.eval(&s.x)).abs() <= 1e-10 * (1.0 + c.f.eval(&s.x).abs()) { xt } else { s.x.clone() };
        let fx = { let v = c.f.eval(&x); if (tidy(v) - v).abs() <= 1e-10 * (1.0 + v.abs()) { tidy(v) } else { v } };
        let (ct, maxv) = cons_table(&c, &x, Some(&s.lambda), sign, "Restricciones en la solución");
        let feasible = maxv <= 1e-6;
        let (status, verdict) = if !feasible {
            (Status::Infeasible, format!("No se encontró ningún punto que cumpla todas las restricciones (violación mínima alcanzada: {}).", fmt(maxv)))
        } else if convex == Some(true) {
            (Status::Optimal, format!("El problema es convexo: este {word} local es el {word} global."))
        } else {
            (
                Status::Feasible,
                format!(
                    "El problema no es (o no se pudo comprobar que sea) convexo: es el mejor {word} local de {starts} arranque{} ({feasible_runs} factible{}).",
                    if starts == 1 { "" } else { "s" },
                    if feasible_runs == 1 { "" } else { "s" }
                ),
            )
        };
        let headline = if feasible {
            format!(
                "{} f = {} en {}.",
                if maximize { "Máximo" } else { "Mínimo" },
                fmt(fx),
                c.names.iter().zip(&x).map(|(nm, v)| format!("{nm} = {}", fmt(*v))).collect::<Vec<_>>().join(", ")
            )
        } else {
            "No se encontró una solución factible.".to_string()
        };
        let (_, grad) = c.f.eval_grad(&x);
        let mut vt = Table::new(Some("Variables"), &["Variable", "Valor", "Cota inferior", "Cota superior", "∂f/∂x"]).left(0);
        for j in 0..n {
            vt.row(cells![&c.names[j], x[j], Cell::Num(c.lb[j]), Cell::Num(c.ub[j]), tidy(grad[j])]);
        }
        let mut sol = vec![
            metrics(vec![metric_main(format!("f {word}"), fx), metric("Violación máxima", tidy(maxv)), metric("Iteraciones (SQP)", s.iters)]),
            text_tone(verdict, if status == Status::Optimal { Tone::Good } else if feasible { Tone::Highlight } else { Tone::Bad }),
            vt.block(),
        ];
        if !c.cons.is_empty() {
            sol.push(ct.note("Precio sombra: cuánto cambia el óptimo por cada unidad más en el lado derecho (multiplicador de Lagrange).").block());
        }
        sol.push(text_tone("Método: programación cuadrática sucesiva (SQP) con hessiana BFGS, derivadas exactas y subproblemas resueltos con Clarabel.", Tone::Muted));

        // ---- gráficos
        let mut gr = vec![];
        if n == 2 {
            let xr = plot2d::range_around(x[0], c.lb[0], c.ub[0]);
            let yr = plot2d::range_around(x[1], c.lb[1], c.ub[1]);
            let f2 = |a: f64, b: f64| c.f.eval(&[a, b]);
            let cons2: Vec<(String, Box<dyn Fn(f64, f64) -> f64 + '_>)> = c.cons.iter().map(|(nm, e, _, _)| (nm.clone(), Box::new(move |a: f64, b: f64| e.eval(&[a, b])) as Box<dyn Fn(f64, f64) -> f64>)).collect();
            let feas = |a: f64, b: f64| p.violation(&[a, b]) <= 1e-9;
            gr.push(plot2d::level_chart("Curvas de nivel, restricciones y región factible", (&c.names[0], &c.names[1]), &f2, &cons2, &feas, xr, yr, Some((x[0], x[1]))));
        }
        if n <= 6 {
            for j in 0..n {
                let (a, b) = plot2d::range_around(x[j], c.lb[j], c.ub[j]);
                let pts: Vec<[f64; 2]> = (0..=120)
                    .map(|k| {
                        let t = a + (b - a) * k as f64 / 120.0;
                        let mut y = x.clone();
                        y[j] = t;
                        [t, c.f.eval(&y)]
                    })
                    .filter(|p| p[1].is_finite())
                    .collect();
                let ch = Chart::new(Some(&format!("f según {} (las demás en la solución)", c.names[j])), &c.names[j], "f")
                    .series(Series::new("f", SeriesKind::Line, pts).tone(Tone::Accent))
                    .series(Series::new("Solución", SeriesKind::Scatter, vec![[x[j], fx]]).tone(Tone::Highlight))
                    .vline(x[j], format!("{} = {}", c.names[j], fmt(x[j])), Tone::Highlight);
                gr.push(ch.block());
            }
        }

        // ---- análisis de un punto
        let mut an = vec![];
        if let Some(ev) = o.evaluate.as_ref().filter(|s| !s.trim().is_empty()) {
            let y = parse_point(ev, &c.names)?;
            let fy = c.f.eval(&y);
            let (t, mv) = cons_table(&c, &y, None, sign, "Restricciones en el punto");
            an.push(metrics(vec![metric_main("f en el punto", tidy(fy)), metric("Violación máxima", tidy(mv)), metric("Diferencia con la solución", tidy(fy - fx))]));
            an.push(text(if mv <= 1e-9 {
                "No se viola ninguna restricción.".to_string()
            } else {
                let k = c.cons.iter().filter(|(_, e, d, _)| {
                    let v = e.eval(&y);
                    match d {
                        Dir::Le => v > 1e-9,
                        Dir::Ge => v < -1e-9,
                        Dir::Eq => v.abs() > 1e-9,
                    }
                }).count();
                format!("{k} restricción{} violada{}; violación máxima {}.", if k == 1 { "" } else { "es" }, if k == 1 { "" } else { "s" }, fmt(mv))
            }));
            if !c.cons.is_empty() {
                an.push(t.block());
            }
        }

        // ---- pasos
        let mut st = vec![];
        if o.steps {
            let mut v = vec![];
            // iteraciones del SQP del mejor arranque
            let mut it = Table::new(Some("Iteraciones de la SQP"), &std::iter::once("k".to_string()).chain(c.names.iter().cloned()).chain(["f".into(), "Violación".into()]).collect::<Vec<_>>());
            for (k, (xx, ff, vv)) in s.history.iter().enumerate().take(40) {
                let mut row = vec![Cell::Num(k as f64)];
                row.extend(xx.iter().map(|q| Cell::Num(*q)));
                row.push(Cell::Num(*ff));
                row.push(Cell::Num(*vv));
                it.row(row);
            }
            v.push(step(
                "Programación cuadrática sucesiva",
                "En cada iteración se aproxima el problema por uno cuadrático (gradientes exactos y una hessiana que aprende de los pasos anteriores) y se avanza hacia su solución mientras mejore la función de mérito.",
                vec![it.block()],
            ));
            if !c.cons.is_empty() {
                let path = nlpsolve::penalty_path(&p, &x0);
                let mut pt = Table::new(Some("Método de penalización (el de WinQSB)"), &std::iter::once("r".to_string()).chain(c.names.iter().cloned()).chain(["f".into(), "Violación".into()]).collect::<Vec<_>>());
                for (r, xx, ff, vv) in &path {
                    let mut row = vec![Cell::Num(*r)];
                    row.extend(xx.iter().map(|q| Cell::Num(tidy(*q))));
                    row.push(Cell::Num(tidy(*ff)));
                    row.push(Cell::Num(tidy(*vv)));
                    pt.row(row);
                }
                v.push(step(
                    "Método de penalización",
                    "Se resuelve sin restricciones la función f + r·Σ(violación)², aumentando r (1, 10, 100...). Cada vez la solución viola menos las restricciones y se acerca al óptimo restringido: es el método que usa WinQSB.",
                    vec![pt.block()],
                ));
            }
            if feasible {
                let mut kt = Table::new(Some("Condiciones de Karush-Kuhn-Tucker en la solución"), &["Variable", "∂f/∂x", "Σ λ · ∂g/∂x", "Residuo", "Nota"]).left(0).left(4);
                for j in 0..n {
                    let mut s2 = 0.0;
                    for (k, (_, e, d, _)) in c.cons.iter().enumerate() {
                        let (_, ge) = e.eval_grad(&x);
                        let gsign = if *d == Dir::Ge { -1.0 } else { 1.0 };
                        s2 += s.lambda[k] * gsign * ge[j];
                    }
                    let r = sign * grad[j] + s2;
                    let at_bound = (c.lb[j].is_finite() && (x[j] - c.lb[j]).abs() <= 1e-7) || (c.ub[j].is_finite() && (x[j] - c.ub[j]).abs() <= 1e-7);
                    kt.row(cells![&c.names[j], tidy(sign * grad[j]), tidy(s2), tidy(r), if r.abs() <= 1e-5 * (1.0 + grad[j].abs()) { "se cumple" } else if at_bound { "variable en su cota" } else { "revisar" }]);
                }
                v.push(step(
                    "Condiciones de Karush-Kuhn-Tucker",
                    "En un óptimo, el gradiente del objetivo (en forma de minimizar) más la suma de los multiplicadores por los gradientes de las restricciones activas es cero, salvo en las variables que están en una cota.",
                    vec![kt.block()],
                ));
            }
            st.push(steps(v));
        }

        let data = json!({
            "objective_value": fx,
            "variables": c.names.iter().zip(&x).map(|(nm, v)| json!({"name": nm, "value": v})).collect::<Vec<_>>(),
            "constraints": c.cons.iter().enumerate().map(|(k, (nm, e, _, _))| json!({"name": nm, "lhs_minus_rhs": tidy(e.eval(&x)), "multiplier": tidy(s.lambda[k])})).collect::<Vec<_>>(),
            "max_violation": tidy(maxv),
            "convex": convex,
            "starts": starts,
        });
        Ok(Report::new("nlp", &m.name, status, headline)
            .section("solucion", "Solución", sol)
            .section("grafico", "Gráficos", gr)
            .section("analisis", "Análisis del punto", an)
            .section("pasos", "Paso a paso", st)
            .data(data))
    }

    fn read(&self, bytes: &[u8]) -> Result<NlpModel, ModError> {
        let mut r = Reader::new(bytes);
        let mut h = r.line("la cabecera")?;
        if !h.next().trim().to_ascii_uppercase().starts_with("NLP") {
            return Err(ModError::Format("no es un archivo de NLP".into()));
        }
        let name = h.next().trim().to_string();
        let n = parse_num(&h.next()).unwrap_or(0.0) as usize;
        let mc = parse_num(&h.next()).unwrap_or(0.0) as usize;
        if n + mc == 0 || n > 10_000 || mc > 10_000 {
            return Err(ModError::Format("el número de variables o restricciones no es válido".into()));
        }
        r.line("la fila de títulos")?;
        let mut o = r.line("la función objetivo")?;
        let objective = Objective::parse(&o.next());
        let function = o.next().trim().to_string();
        let mut constraints = vec![];
        for i in 0..mc {
            let mut l = r.line(&format!("la restricción {}", i + 1))?;
            let nm = l.next().trim().to_string();
            constraints.push(NlpConstraint { name: if nm.is_empty() { format!("C{}", i + 1) } else { nm }, expression: l.next().trim().to_string() });
        }
        let mut variables = vec![];
        for j in 0..n {
            let mut l = r.line(&format!("la variable {}", j + 1))?;
            let nm = l.next().trim().to_string();
            let mut v = Var::new(if nm.is_empty() { format!("X{}", j + 1) } else { nm });
            apply_bounds(&mut v, &l.next())?;
            variables.push(NlpVar { name: v.name, lower: v.lower, upper: v.upper, initial: None });
        }
        Ok(NlpModel { name, problem_type: nlp_type(), objective, function, constraints, variables })
    }

    fn write(&self, m: &NlpModel) -> Result<Vec<u8>, ModError> {
        let mut w = Writer::new();
        // WinQSB escribe los números con Str$ (con un espacio delante)
        w.row(&["NLP".to_string(), m.name.clone(), format!(" {}", m.variables.len()), format!(" {}", m.constraints.len())]);
        w.row(&["", "OBJ / Constraint / Variable Bound"]);
        w.row(&[if m.objective.is_max() { "Maximize" } else { "Minimize" }.to_string(), m.function.clone()]);
        for c in &m.constraints {
            w.row(&[c.name.clone(), c.expression.clone()]);
        }
        for v in &m.variables {
            w.row(&[v.name.clone(), format!(">={}, <={}", num(v.lower.0), num(v.upper.0))]);
        }
        Ok(w.bytes())
    }

    fn examples(&self) -> Vec<(&'static str, NlpModel)> {
        let ex = |s: &str| serde_json::from_str::<NlpModel>(s).unwrap();
        vec![
            ("cobb_douglas", ex(include_str!("ejemplos/cobb.json"))),
            ("rosenbrock", ex(include_str!("ejemplos/rosenbrock.json"))),
            ("hillier_nlp", ex(include_str!("ejemplos/hillier.json"))),
        ]
    }
}
