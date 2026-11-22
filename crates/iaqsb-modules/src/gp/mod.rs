//! GP-IGP · Programación por metas (equivalente al módulo GP-IGP de WinQSB).
//!
//! Metas preferentes: cada meta es una función objetivo (minimizar o
//! maximizar) y se optimizan en orden de prioridad sin empeorar las anteriores.
//! Las variables de desviación se escriben como variables normales, igual que
//! en WinQSB. Variables continuas, enteras, binarias o libres.
//!
//! El resultado es exacto: HiGHS resuelve un programa lineal (o entero) por
//! prioridad, fijando lo logrado en las anteriores. El paso a paso muestra
//! esas etapas y, en modelos continuos, el símplex de metas con una fila
//! Cj − Zj por prioridad, comprobado contra HiGHS.

mod simplex;

use iaqsb_core::linear::*;
use iaqsb_core::lp::{solve as lp_solve, Lp, LpStatus, Sense};
use iaqsb_core::winqsb::{num, parse_num, Reader, Writer};
use iaqsb_core::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// Una meta (función objetivo con su prioridad, que es su posición en la lista).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Goal {
    pub name: String,
    #[serde(default)]
    pub objective: Objective,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GpModel {
    #[serde(default)]
    pub name: String,
    /// Siempre "goal": un solo tipo de problema.
    #[serde(default = "gp_type")]
    pub problem_type: String,
    pub variables: Vec<Var>,
    /// Metas en orden de prioridad (la primera es la más importante).
    pub goals: Vec<Goal>,
    /// Coeficientes de cada meta: una fila por meta, una columna por variable.
    pub goal_coefs: Vec<Vec<f64>>,
    #[serde(default)]
    pub constraints: Vec<Constr>,
    /// Coeficientes de las restricciones.
    #[serde(default)]
    pub coefs: Vec<Vec<f64>>,
    /// Guardar en la forma de modelo normal de WinQSB (si no, de hoja).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub normal_form: bool,
}

fn gp_type() -> String {
    "goal".into()
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct GpOptions {
    pub steps: bool,
}

impl Default for GpOptions {
    fn default() -> Self {
        GpOptions { steps: true }
    }
}

impl GpModel {
    fn normalized(&self) -> Result<GpModel, ModError> {
        let mut m = self.clone();
        let n = m.variables.len();
        fit(&mut m.goal_coefs, m.goals.len(), n);
        fit(&mut m.coefs, m.constraints.len(), n);
        if n == 0 {
            return invalid("el modelo no tiene variables");
        }
        if m.goals.is_empty() {
            return invalid("hace falta al menos una meta");
        }
        check_names("variables", &m.variables.iter().map(|v| &v.name).collect::<Vec<_>>())?;
        check_names("metas", &m.goals.iter().map(|g| &g.name).collect::<Vec<_>>())?;
        check_names("restricciones", &m.constraints.iter().map(|c| &c.name).collect::<Vec<_>>())?;
        for v in &m.variables {
            let (l, u) = v.bounds();
            if l > u {
                return invalid(format!("la variable {} tiene cota inferior mayor que la superior", v.name));
            }
        }
        if m.goal_coefs.iter().chain(&m.coefs).flatten().any(|x| !x.is_finite()) || m.constraints.iter().any(|c| !c.rhs.is_finite()) {
            return invalid("hay coeficientes que no son números finitos");
        }
        Ok(m)
    }
    fn names(&self) -> Vec<String> {
        self.variables.iter().map(|v| v.name.clone()).collect()
    }
    fn has_integers(&self) -> bool {
        self.variables.iter().any(|v| v.integer())
    }
}

pub struct Gp;

fn base_lp(m: &GpModel) -> Lp {
    let mut lp = Lp::new(0, false);
    for v in &m.variables {
        let (l, u) = v.bounds();
        lp.add_var(0.0, l, u, v.integer());
    }
    for (c, r) in m.constraints.iter().zip(&m.coefs) {
        lp.row(r.iter().enumerate().filter(|(_, a)| **a != 0.0).map(|(j, a)| (j, *a)).collect(), c.direction.sense(), c.rhs);
    }
    lp
}

fn dot(a: &[f64], x: &[f64]) -> f64 {
    a.iter().zip(x).map(|(p, q)| p * q).sum()
}

struct Stage {
    status: LpStatus,
    x: Vec<f64>,
    value: f64,
    duals: Option<Vec<f64>>,
}

/// Resuelve la meta k con las anteriores fijadas en `fixed`. Si el optimizador
/// la declara no factible por redondeo (no puede serlo: la solución anterior
/// cumple todo), se repite con más tolerancia.
fn stage(m: &GpModel, k: usize, fixed: &[f64]) -> Stage {
    let mut s = stage_tol(m, k, fixed, 1e-9, None);
    for t in [1e-7, 1e-6, 1e-5, 1e-4] {
        if k == 0 || fixed.is_empty() || s.status != LpStatus::Infeasible {
            break;
        }
        s = stage_tol(m, k, fixed, t, None);
    }
    s
}

/// Como `stage`, con tolerancia dada y, si se indica, enteras fijadas en esos valores.
fn stage_tol(m: &GpModel, k: usize, fixed: &[f64], rel: f64, ints: Option<&[f64]>) -> Stage {
    let mut lp = base_lp(m);
    if let Some(xi) = ints {
        for (j, v) in m.variables.iter().enumerate() {
            if v.integer() {
                lp.lb[j] = xi[j];
                lp.ub[j] = xi[j];
                lp.integer[j] = false;
            }
        }
    }
    for (l, &z) in fixed.iter().enumerate() {
        let tol = rel * z.abs().max(1.0);
        let row = m.goal_coefs[l].iter().enumerate().filter(|(_, a)| **a != 0.0).map(|(j, a)| (j, *a)).collect();
        if m.goals[l].objective.is_max() {
            lp.row(row, Sense::Ge, z - tol);
        } else {
            lp.row(row, Sense::Le, z + tol);
        }
    }
    lp.maximize = m.goals[k].objective.is_max();
    lp.c = m.goal_coefs[k].clone();
    let s = lp_solve(&lp);
    let value = if s.status == LpStatus::Optimal { dot(&m.goal_coefs[k], &s.x) } else { f64::NAN };
    Stage { status: s.status, x: s.x, value, duals: s.duals }
}

impl TypedModule for Gp {
    type Model = GpModel;
    type Options = GpOptions;

    fn file_tag(&self) -> &'static str {
        "GP"
    }

    fn spec(&self) -> ModuleSpec {
        let var_axis = Axis::new("variables", "Variable")
            .add("Añadir variable")
            .extra(Field::bound("lower", "Cota inferior", Value::from(0.0)))
            .extra(Field::bound("upper", "Cota superior", Value::from("M")))
            .extra(Field::choice(
                "type",
                "Tipo",
                &[("continuous", "Continua"), ("integer", "Entera"), ("binary", "Binaria"), ("unrestricted", "Libre")],
            ));
        let goal_axis = Axis::new("goals", "Meta").add("Añadir meta").extra(Field::choice("objective", "Criterio", &[("min", "Minimizar"), ("max", "Maximizar")]));
        let con_axis = Axis::new("constraints", "Restricción")
            .add("Añadir restricción")
            .extra(Field::choice("direction", "Sentido", &[("<=", "<="), (">=", ">="), ("=", "=")]))
            .extra(Field::number("rhs", "Lado derecho"));
        let mut vars_plain = Axis::new("variables", "Variable").add("Añadir variable");
        vars_plain.extra = vec![];
        ModuleSpec {
            id: "gp".into(),
            winqsb: "GP-IGP".into(),
            name: "Programación por metas".into(),
            description: "Metas con prioridades (preferentes), variables continuas, enteras o binarias".into(),
            file_ext: "GPP".into(),
            problem_types: vec![ProblemType {
                id: "goal".into(),
                label: "Metas preferentes".into(),
                description: "Optimiza las metas en orden de prioridad; las desviaciones se escriben como variables".into(),
                dims: vec![
                    Field::integer("goals", "Número de metas", 2),
                    Field::integer("variables", "Número de variables", 4),
                    Field::integer("constraints", "Número de restricciones", 2),
                ],
                layout: vec![
                    Widget::matrix(Some("Metas (en orden de prioridad)"), "goal_coefs", goal_axis, vars_plain, Field::number("c", "Coeficiente")),
                    Widget::matrix(Some("Restricciones, cotas y tipos"), "coefs", con_axis, var_axis, Field::number("a", "Coeficiente")),
                ],
            }],
            options: vec![Field::boolean("steps", "Mostrar paso a paso", true)],
        }
    }

    fn template(&self, _problem_type: &str, dims: &Value) -> Result<GpModel, ModError> {
        let k = dim(dims, "goals", 2, 1, 50)?;
        let n = dim(dims, "variables", 4, 1, 2000)?;
        let m = dim(dims, "constraints", 2, 0, 2000)?;
        Ok(GpModel {
            name: "Metas nuevo".into(),
            problem_type: gp_type(),
            variables: (1..=n).map(|j| Var::new(format!("X{j}"))).collect(),
            goals: (1..=k).map(|i| Goal { name: format!("G{i}"), objective: Objective::Min }).collect(),
            goal_coefs: vec![vec![0.0; n]; k],
            constraints: (1..=m).map(|i| Constr { name: format!("C{i}"), direction: Dir::Le, rhs: 0.0 }).collect(),
            coefs: vec![vec![0.0; n]; m],
            normal_form: false,
        })
    }

    fn solve(&self, model: &GpModel, o: &GpOptions) -> Result<Report, ModError> {
        let m = model.normalized()?;
        let names = m.names();
        let n = names.len();
        let k = m.goals.len();
        let mut fixed: Vec<f64> = vec![];
        let mut stages: Vec<Stage> = vec![];
        let mut ideal = vec![];
        for g in 0..k {
            let s = stage(&m, g, &fixed);
            match s.status {
                LpStatus::Optimal => {}
                LpStatus::Infeasible => {
                    let rep = Report::new("gp", &m.name, Status::Infeasible, "Las restricciones no tienen solución: no hay ningún punto que las cumpla todas.")
                        .section("solucion", "Solución", vec![text_tone("El conjunto de restricciones (y cotas) es incompatible. Revisa los sentidos y los lados derechos, o convierte alguna restricción rígida en meta con variables de desviación.", Tone::Bad)])
                        .data(json!({"status": "infeasible"}));
                    return Ok(rep);
                }
                LpStatus::Unbounded => {
                    let rep = Report::new("gp", &m.name, Status::Unbounded, format!("La meta {} no está acotada: puede mejorar sin límite.", m.goals[g].name))
                        .section("solucion", "Solución", vec![text_tone(format!("Con las metas de mayor prioridad cumplidas, la meta {} puede mejorar indefinidamente. Falta alguna restricción o cota.", m.goals[g].name), Tone::Bad)])
                        .data(json!({"status": "unbounded", "goal": m.goals[g].name}));
                    return Ok(rep);
                }
                LpStatus::Error(e) => return invalid(format!("error del optimizador: {e}")),
            }
            fixed.push(tidy(s.value));
            // valor si la meta se optimizara sola (sin las anteriores)
            let alone = if g == 0 { s.value } else { stage(&m, g, &[]).value };
            ideal.push(alone);
            stages.push(s);
        }
        let mut x: Vec<f64> = stages.last().unwrap().x.clone();
        if m.has_integers() {
            // enteras redondeadas y la parte continua repulida con programas lineales
            for (j, v) in m.variables.iter().enumerate() {
                if v.integer() {
                    x[j] = x[j].round();
                }
            }
            let mut fx: Vec<f64> = vec![];
            let mut polished = None;
            for g in 0..k {
                let s = stage_tol(&m, g, &fx, 1e-9, Some(&x));
                if s.status != LpStatus::Optimal {
                    polished = None;
                    break;
                }
                fx.push(tidy(s.value));
                polished = Some(s.x);
            }
            if let Some(p) = polished {
                if (0..k).all(|g| (dot(&m.goal_coefs[g], &p) - fixed[g]).abs() <= 1e-5 * fixed[g].abs().max(1.0)) {
                    x = p;
                }
            }
        }
        // redondeo del ruido solo si se siguen cumpliendo las restricciones
        let xt: Vec<f64> = x.iter().map(|v| tidy(*v)).collect();
        let holds = |y: &[f64]| m.constraints.iter().zip(&m.coefs).all(|(c, a)| c.direction.holds(dot(a, y), c.rhs, 1e-9 * c.rhs.abs().max(1.0)));
        let mut x: Vec<f64> = if holds(&xt) || !holds(&x) { xt } else { x };
        let mut gv: Vec<f64> = (0..k).map(|g| tidy(dot(&m.goal_coefs[g], &x))).collect();
        let tab = tableau_steps(&m, &gv);
        if let Some((_, _, tx)) = &tab {
            x = tx.iter().map(|v| tidy(*v)).collect();
            gv = (0..k).map(|g| tidy(dot(&m.goal_coefs[g], &x))).collect();
        }

        // ---- solución
        let mut ms: Vec<Metric> = (0..k)
            .map(|g| {
                let mut mm = metric(format!("P{} {}", g + 1, m.goals[g].name), gv[g]);
                if g == 0 {
                    mm.emphasis = true;
                }
                mm
            })
            .collect();
        if k > 6 {
            ms.truncate(6);
        }
        let mut head = vec!["Variable".to_string(), "Valor".to_string()];
        head.extend(m.goals.iter().map(|g| format!("Aporte a {}", g.name)));
        head.push("Tipo".into());
        let mut vt = Table::new(Some("Variables de decisión (informe combinado)"), &head).left(0).left(k + 2);
        for j in 0..n {
            let mut row = vec![Cell::from(&names[j]), Cell::Num(x[j])];
            row.extend((0..k).map(|g| Cell::Num(m.goal_coefs[g][j] * x[j])));
            row.push(Cell::from(match m.variables[j].kind {
                VarKind::Continuous => "continua",
                VarKind::Integer => "entera",
                VarKind::Binary => "binaria",
                VarKind::Unrestricted => "libre",
            }));
            vt.row_tone(row, if x[j].abs() > 1e-9 { Tone::Accent } else { Tone::Normal });
        }
        let mut gt = Table::new(Some("Metas"), &["Prioridad", "Meta", "Criterio", "Valor logrado", "Mejor valor posible sola", "Diferencia"]).left(1).left(2);
        for g in 0..k {
            let d = (gv[g] - ideal[g]).abs();
            gt.row_tone(
                cells![g + 1, &m.goals[g].name, if m.goals[g].objective.is_max() { "maximizar" } else { "minimizar" }, gv[g], tidy(ideal[g]), tidy(d)],
                if d > 1e-7 * ideal[g].abs().max(1.0) { Tone::Highlight } else { Tone::Good },
            );
        }
        let gt = gt.note("En verde, las metas que logran lo mismo que si fueran la única; en ámbar, las que ceden por respetar metas más importantes.");
        let mut ct = Table::new(Some("Restricciones"), &["Restricción", "Lado izquierdo", "Sentido", "Lado derecho", "Holgura o exceso", "Estado"]).left(0).left(2).left(5);
        for (i, c) in m.constraints.iter().enumerate() {
            let lhs = clean(dot(&m.coefs[i], &x));
            let sl = clean((c.rhs - lhs).abs());
            ct.row_tone(
                cells![&c.name, lhs, c.direction.symbol(), c.rhs, sl, if sl <= 1e-7 * c.rhs.abs().max(1.0) { "activa" } else { "con holgura" }],
                if sl <= 1e-7 * c.rhs.abs().max(1.0) { Tone::Highlight } else { Tone::Normal },
            );
        }
        let mut sol = vec![metrics(ms), gt.block(), vt.block()];
        if !m.constraints.is_empty() {
            sol.push(ct.block());
        }
        // el plan (sin las variables de desviación d1m, d1p, …) y qué metas logran su mejor valor
        let is_dev = |name: &str| {
            let t = name.trim().to_lowercase();
            let b = t.as_bytes();
            b.len() >= 3 && b[0] == b'd' && matches!(b[b.len() - 1], b'm' | b'p' | b'+' | b'-') && t[1..t.len() - 1].chars().all(|c| c.is_ascii_digit() || c == '_')
        };
        let plan: Vec<String> = m.variables.iter().zip(x.iter()).filter(|(v, _)| !is_dev(&v.name)).take(8).map(|(v, val)| format!("{} = {}", v.name, fmt(clean(*val)))).collect();
        let met: Vec<&str> = (0..k).filter(|&g| (gv[g] - ideal[g]).abs() <= 1e-7 * ideal[g].abs().max(1.0)).map(|g| m.goals[g].name.as_str()).collect();
        let short: Vec<String> = (0..k)
            .filter(|&g| (gv[g] - ideal[g]).abs() > 1e-7 * ideal[g].abs().max(1.0))
            .map(|g| format!("{} = {} (sola lograría {})", m.goals[g].name, fmt(gv[g]), fmt(tidy(ideal[g]))))
            .collect();
        let headline = format!(
            "{}{}{}",
            if plan.is_empty() { String::new() } else { format!("Plan: {}. ", plan.join(", ")) },
            if met.is_empty() { String::new() } else { format!("Logran su mejor valor: {}. ", met.join(", ")) },
            if short.is_empty() { String::new() } else { format!("Ceden: {}.", short.join("; ")) }
        )
        .trim()
        .to_string();

        // ---- análisis por etapas (precios sombra), solo continuo
        let mut an = vec![];
        if !m.has_integers() && !m.constraints.is_empty() {
            let mut head = vec!["Restricción".to_string()];
            head.extend(m.goals.iter().enumerate().map(|(g, gg)| format!("P{} {}", g + 1, gg.name)));
            let mut dt = Table::new(Some("Precio sombra de cada restricción en cada prioridad"), &head).left(0);
            let mut any = false;
            for (i, c) in m.constraints.iter().enumerate() {
                let mut row = vec![Cell::from(&c.name)];
                for s in &stages {
                    match &s.duals {
                        Some(d) => {
                            any = true;
                            row.push(Cell::Num(tidy(d[i])));
                        }
                        None => row.push(Cell::Empty),
                    }
                }
                dt.row(row);
            }
            if any {
                an.push(text("Cuánto mejora o empeora cada meta por cada unidad más en el lado derecho de una restricción, manteniendo lo logrado en las metas de mayor prioridad."));
                an.push(dt.block());
            }
        }

        // ---- pasos
        let mut st = vec![];
        if o.steps {
            let mut v = vec![];
            for g in 0..k {
                let mut t = Table::new(None, &["Variable", "Valor en esta etapa"]).left(0);
                for j in 0..n {
                    if stages[g].x[j].abs() > 1e-9 {
                        t.row(cells![&names[j], tidy(stages[g].x[j])]);
                    }
                }
                let prev = if g == 0 {
                    "Se optimiza la meta más importante con las restricciones del modelo.".to_string()
                } else {
                    format!(
                        "Se añaden como restricciones las metas ya logradas ({}) y se optimiza la siguiente.",
                        (0..g).map(|l| format!("{} {} {}", m.goals[l].name, if m.goals[l].objective.is_max() { "≥" } else { "≤" }, fmt(fixed[l]))).collect::<Vec<_>>().join(", ")
                    )
                };
                v.push(step(
                    format!("Prioridad {}: {} {}", g + 1, if m.goals[g].objective.is_max() { "maximizar" } else { "minimizar" }, m.goals[g].name),
                    format!("{prev} Resultado: {} = {}.", m.goals[g].name, fmt(fixed[g])),
                    vec![
                        text(format!("{} {} = {}", if m.goals[g].objective.is_max() { "Máx" } else { "Mín" }, m.goals[g].name, linear_text(&m.goal_coefs[g], &names))),
                        t.block(),
                    ],
                ));
            }
            st.push(Block::Steps { title: Some("Una etapa por prioridad (método secuencial)".into()), steps: v });
            if let Some((tab, note, _)) = tab {
                st.push(text(note));
                st.push(Block::Steps { title: Some("Símplex de metas".into()), steps: tab });
            }
        }

        let data = json!({
            "goal_values": (0..k).map(|g| json!({"goal": m.goals[g].name, "priority": g + 1, "value": gv[g], "best_alone": tidy(ideal[g])})).collect::<Vec<_>>(),
            "variables": (0..n).map(|j| json!({"name": names[j], "value": x[j]})).collect::<Vec<_>>(),
            "constraints": m.constraints.iter().enumerate().map(|(i, c)| json!({"name": c.name, "lhs": clean(dot(&m.coefs[i], &x)), "rhs": c.rhs})).collect::<Vec<_>>(),
        });
        Ok(Report::new("gp", &m.name, Status::Optimal, headline)
            .section("solucion", "Solución", sol)
            .section("analisis", "Análisis", an)
            .section("pasos", "Paso a paso", st)
            .data(data))
    }

    fn read(&self, bytes: &[u8]) -> Result<GpModel, ModError> {
        read_gp(bytes)
    }

    fn write(&self, m: &GpModel) -> Result<Vec<u8>, ModError> {
        let m = m.normalized()?;
        Ok(write_gp(&m))
    }

    fn examples(&self) -> Vec<(&'static str, GpModel)> {
        vec![("harrison", serde_json::from_str(include_str!("ejemplos/harrison.json")).unwrap())]
    }
}

/// Símplex de metas para el paso a paso, si el modelo lo permite.
fn tableau_steps(m: &GpModel, gv: &[f64]) -> Option<(Vec<Step>, String, Vec<f64>)> {
    let n = m.variables.len();
    if m.has_integers() || n > 14 || m.constraints.len() > 12 {
        return None;
    }
    if m.variables.iter().any(|v| v.bounds().0 != 0.0) {
        return None;
    }
    let mut rows: Vec<(String, Vec<f64>, Dir, f64)> = m.constraints.iter().zip(&m.coefs).map(|(c, a)| (c.name.clone(), a.clone(), c.direction, c.rhs)).collect();
    for (j, v) in m.variables.iter().enumerate() {
        let u = v.bounds().1;
        if u.is_finite() {
            let mut a = vec![0.0; n];
            a[j] = 1.0;
            rows.push((format!("Cota {}", v.name), a, Dir::Le, u));
        }
    }
    let goals: Vec<(String, Vec<f64>)> = m
        .goals
        .iter()
        .zip(&m.goal_coefs)
        .map(|(g, c)| (g.name.clone(), if g.objective.is_max() { c.iter().map(|x| -x).collect() } else { c.clone() }))
        .collect();
    let labels: Vec<String> = m.goals.iter().map(|g| g.name.clone()).collect();
    let p = simplex::Problem { names: m.names(), rows, goals };
    let out = simplex::run(&p, &labels);
    if out.status != "optimal" {
        if std::env::var("IAQSB_DEBUG").is_ok() {
            eprintln!("simplex de metas: {}", out.status);
        }
        return None;
    }
    let x = out.x?;
    // el símplex en aritmética exacta de la tabla es más preciso que HiGHS (tolerancia 1e-7)
    let ok = (0..m.goals.len()).all(|g| (dot(&m.goal_coefs[g], &x) - gv[g]).abs() <= 1e-4 * gv[g].abs().max(1.0));
    if !ok {
        if std::env::var("IAQSB_DEBUG").is_ok() {
            eprintln!("simplex de metas distinto: x={x:?} metas={:?} esperado={gv:?}", (0..m.goals.len()).map(|g| dot(&m.goal_coefs[g], &x)).collect::<Vec<_>>());
        }
        return None;
    }
    Some((
        out.steps,
        "Método del libro: una fila Cj − Zj por prioridad (las maximizaciones se escriben como minimizar el negativo). Entra la variable con Cj − Zj más negativo en la prioridad más alta que aún puede mejorar, siempre que no empeore ninguna superior (su Cj − Zj allí debe ser 0).".into(),
        x,
    ))
}

// ------------------------------------------------------------------- archivos

fn goal_label(g: &Goal) -> String {
    format!("{}:{}", if g.objective.is_max() { "Max" } else { "Min" }, g.name)
}

fn parse_goal_label(s: &str, i: usize) -> Goal {
    let t = s.trim();
    let name = t.split_once(':').map(|(_, b)| b.trim().to_string()).filter(|s| !s.is_empty()).unwrap_or(format!("G{}", i + 1));
    let objective = if t.to_ascii_uppercase().starts_with("MAX") { Objective::Max } else { Objective::Min };
    Goal { name, objective }
}

fn read_gp(bytes: &[u8]) -> Result<GpModel, ModError> {
    let mut r = Reader::new(bytes);
    let mut h = r.line("la cabecera")?;
    let tag = h.next().trim().to_ascii_uppercase();
    let (k, n, mc, normal, name);
    if tag.starts_with("LP") {
        // WinQSB abre también un archivo de LP-ILP como una sola meta
        normal = !h.next().trim().to_ascii_uppercase().starts_with('M');
        name = h.next();
        k = 1;
        n = parse_num(&h.next()).unwrap_or(0.0) as usize;
        mc = parse_num(&h.next()).unwrap_or(0.0) as usize;
    } else if tag.starts_with("GP") {
        normal = !h.next().trim().to_ascii_uppercase().starts_with('M');
        name = h.next();
        k = parse_num(&h.next()).unwrap_or(0.0) as usize;
        n = parse_num(&h.next()).unwrap_or(0.0) as usize;
        mc = parse_num(&h.next()).unwrap_or(0.0) as usize;
    } else {
        return Err(ModError::Format("no es un archivo de GP-IGP".into()));
    }
    if k == 0 || n == 0 || k > 1000 || n > 100_000 || mc > 100_000 {
        return Err(ModError::Format("el número de metas, variables o restricciones no es válido".into()));
    }
    let mut m = GpModel {
        name: name.trim().to_string(),
        problem_type: gp_type(),
        variables: vec![],
        goals: vec![],
        goal_coefs: vec![],
        constraints: vec![],
        coefs: vec![],
        normal_form: normal,
    };
    if !normal {
        let names = r.line("los nombres de las variables")?;
        m.variables = (0..n)
            .map(|j| {
                let s = names.get(j + 1).trim();
                Var::new(if s.is_empty() { format!("X{}", j + 1) } else { s.to_string() })
            })
            .collect();
        for i in 0..k {
            let l = r.line(&format!("la meta {}", i + 1))?;
            m.goals.push(parse_goal_label(l.get(0), i));
            m.goal_coefs.push((0..n).map(|j| parse_num(l.get(j + 1)).unwrap_or(0.0)).collect());
        }
        for i in 0..mc {
            let l = r.line(&format!("la restricción {}", i + 1))?;
            let nm = l.get(0).trim();
            m.constraints.push(Constr {
                name: if nm.is_empty() { format!("C{}", i + 1) } else { nm.to_string() },
                direction: Dir::parse(l.get(n + 1)).unwrap_or(Dir::Le),
                rhs: parse_num(l.get(n + 2)).unwrap_or(0.0),
            });
            m.coefs.push((0..n).map(|j| parse_num(l.get(j + 1)).unwrap_or(0.0)).collect());
        }
        let lb = r.line("las cotas inferiores")?;
        let ub = r.line("las cotas superiores")?;
        let ty = r.line("los tipos de variable")?;
        read_bound_rows(&mut m.variables, &lb, &ub, &ty);
    } else {
        r.line("la fila de títulos")?;
        let mut goal_lines = vec![];
        for i in 0..k {
            goal_lines.push(r.line(&format!("la meta {}", i + 1))?);
        }
        let mut con_lines = vec![];
        for i in 0..mc {
            con_lines.push(r.line(&format!("la restricción {}", i + 1))?);
        }
        let ints = r.line("la lista de enteras")?;
        let bins = r.line("la lista de binarias")?;
        let unres = r.line("la lista de libres")?;
        let mut bl = vec![];
        for j in 0..n {
            bl.push(r.line(&format!("las cotas de la variable {}", j + 1))?);
        }
        m.variables = bl
            .iter()
            .enumerate()
            .map(|(j, l)| {
                let s = l.get(0).trim();
                Var::new(if s.is_empty() { format!("X{}", j + 1) } else { s.to_string() })
            })
            .collect();
        let names = m.names();
        for (i, l) in goal_lines.iter().enumerate() {
            m.goals.push(parse_goal_label(l.get(0), i));
            m.goal_coefs.push(coefs_of(l.get(1), &names, &format!("meta {}", i + 1))?);
        }
        for (i, l) in con_lines.iter().enumerate() {
            let at = format!("restricción {}", i + 1);
            let txt: String = l.get(1).chars().filter(|c| !c.is_whitespace()).collect();
            let (lhs, d, rhs) = iaqsb_core::expr::split_relation(&txt).ok_or_else(|| ModError::Format(format!("{at}: falta <=, >= o =")))?;
            let nm = l.get(0).trim();
            m.constraints.push(Constr {
                name: if nm.is_empty() { format!("C{}", i + 1) } else { nm.to_string() },
                direction: Dir::parse(d).unwrap_or(Dir::Le),
                rhs: parse_num(&rhs).ok_or_else(|| ModError::Format(format!("{at}: lado derecho «{rhs}» no válido")))?,
            });
            m.coefs.push(coefs_of(&lhs, &names, &at)?);
        }
        apply_kind_list(&mut m.variables, &ints, VarKind::Integer)?;
        apply_kind_list(&mut m.variables, &bins, VarKind::Binary)?;
        apply_kind_list(&mut m.variables, &unres, VarKind::Unrestricted)?;
        for (j, l) in bl.iter().enumerate() {
            if matches!(m.variables[j].kind, VarKind::Binary | VarKind::Unrestricted) {
                continue;
            }
            apply_bounds(&mut m.variables[j], l.get(1))?;
        }
    }
    Ok(m)
}

fn write_gp(m: &GpModel) -> Vec<u8> {
    let mut w = Writer::new();
    let n = m.variables.len();
    let names = m.names();
    w.row(&[
        "GP".to_string(),
        if m.normal_form { "NormalModel" } else { "MatrixFormat" }.to_string(),
        m.name.clone(),
        m.goals.len().to_string(),
        n.to_string(),
        m.constraints.len().to_string(),
    ]);
    if !m.normal_form {
        let mut r = vec!["Variable -->".to_string()];
        r.extend(names.iter().cloned());
        r.push("Direction".into());
        r.push("R. H. S.".into());
        w.row(&r);
        for (g, c) in m.goals.iter().zip(&m.goal_coefs) {
            let mut r = vec![goal_label(g)];
            r.extend(c.iter().map(|x| num(*x)));
            r.extend([String::new(), String::new()]);
            w.row(&r);
        }
        for (c, a) in m.constraints.iter().zip(&m.coefs) {
            let mut r = vec![c.name.clone()];
            r.extend(a.iter().map(|x| num(*x)));
            r.push(c.direction.symbol().into());
            r.push(num(c.rhs));
            w.row(&r);
        }
        for r in bound_rows(&m.variables, 2) {
            w.row(&r);
        }
    } else {
        w.row(&["Goal/Constraint/VariableType/Bound", ""]);
        for (g, c) in m.goals.iter().zip(&m.goal_coefs) {
            w.row(&[goal_label(g), linear_text(c, &names)]);
        }
        for (c, a) in m.constraints.iter().zip(&m.coefs) {
            w.row(&[c.name.clone(), format!("{}{}{}", linear_text(a, &names), c.direction.symbol(), num(c.rhs))]);
        }
        for r in kind_lists(&m.variables) {
            w.row(&r);
        }
        for v in &m.variables {
            let (l, u) = v.bounds();
            w.row(&[v.name.clone(), format!(">={}, <={}", num(l), num(u))]);
        }
    }
    w.bytes()
}
