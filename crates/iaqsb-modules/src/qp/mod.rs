//! QP-IQP · Programación cuadrática (equivalente al módulo QP-IQP de WinQSB).
//!
//! Objetivo cuadrático (términos xᵢ·xⱼ y lineales) con restricciones lineales y
//! variables continuas, enteras, binarias o libres.
//!
//! - Convexa: Clarabel (punto interior) y un pulido exacto con las restricciones
//!   activas, con multiplicadores de Lagrange (precios sombra).
//! - No convexa: óptimo global por enumeración de puntos KKT si es pequeña.
//! - Enteras: ramificación y acotamiento.
//! - Paso a paso: análisis de convexidad (valores propios de la hessiana),
//!   método de Wolfe y comprobación de las condiciones KKT.
//!
//! Nota: el formato de los archivos .QPP de WinQSB se completará con los
//! archivos de ejemplo originales; por ahora sigue la estructura de LP-ILP con
//! una fila por variable para los términos cuadráticos.

mod wolfe;

use crate::qpsolve::{self, eigen_range, is_convex, Qp, QpStatus};
use iaqsb_core::expr::{quadratic, Expr, Vars};
use iaqsb_core::linear::*;
use iaqsb_core::winqsb::{num, parse_num, Reader, Writer};
use iaqsb_core::*;
use nalgebra::{DMatrix, DVector};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// Variable con su coeficiente lineal en el objetivo.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct QVar {
    #[serde(flatten)]
    pub var: Var,
    /// Coeficiente lineal en la función objetivo.
    #[serde(default)]
    pub c: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct QpModel {
    #[serde(default)]
    pub name: String,
    #[serde(default = "qp_type")]
    pub problem_type: String,
    #[serde(default)]
    pub objective: Objective,
    pub variables: Vec<QVar>,
    /// Coeficiente del término xᵢ·xⱼ en la fila i, columna j (se suman todas las celdas:
    /// 3·X1·X2 puede ir en (1, 2) o en (2, 1)).
    pub quad: Vec<Vec<f64>>,
    #[serde(default)]
    pub constraints: Vec<Constr>,
    #[serde(default)]
    pub coefs: Vec<Vec<f64>>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub normal_form: bool,
}

fn qp_type() -> String {
    "quadratic".into()
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct QpOptions {
    pub steps: bool,
}

impl Default for QpOptions {
    fn default() -> Self {
        QpOptions { steps: true }
    }
}

impl QpModel {
    fn normalized(&self) -> Result<QpModel, ModError> {
        let mut m = self.clone();
        let n = m.variables.len();
        if n == 0 {
            return invalid("el modelo no tiene variables");
        }
        fit(&mut m.quad, n, n);
        fit(&mut m.coefs, m.constraints.len(), n);
        check_names("variables", &m.variables.iter().map(|v| &v.var.name).collect::<Vec<_>>())?;
        check_names("restricciones", &m.constraints.iter().map(|c| &c.name).collect::<Vec<_>>())?;
        if m.quad.iter().chain(&m.coefs).flatten().chain(m.variables.iter().map(|v| &v.c)).any(|x| !x.is_finite()) {
            return invalid("hay coeficientes que no son números finitos");
        }
        for v in &m.variables {
            let (l, u) = v.var.bounds();
            if l > u {
                return invalid(format!("la variable {} tiene cota inferior mayor que la superior", v.var.name));
            }
        }
        Ok(m)
    }
    fn names(&self) -> Vec<String> {
        self.variables.iter().map(|v| v.var.name.clone()).collect()
    }
    /// Hessiana de f (sin cambiar el signo).
    fn hessian(&self) -> Vec<Vec<f64>> {
        let n = self.variables.len();
        (0..n).map(|i| (0..n).map(|j| self.quad[i][j] + self.quad[j][i]).collect()).collect()
    }
    fn f(&self, x: &[f64]) -> f64 {
        let n = x.len();
        let mut v: f64 = self.variables.iter().zip(x).map(|(a, b)| a.c * b).sum();
        for i in 0..n {
            for j in 0..n {
                v += self.quad[i][j] * x[i] * x[j];
            }
        }
        v
    }
    /// Problema en forma de minimizar para el motor.
    fn as_qp(&self) -> Qp {
        let s = if self.objective.is_max() { -1.0 } else { 1.0 };
        let h = self.hessian();
        Qp {
            p: h.iter().map(|r| r.iter().map(|v| s * v).collect()).collect(),
            q: self.variables.iter().map(|v| s * v.c).collect(),
            rows: self.constraints.iter().zip(&self.coefs).map(|(c, a)| (a.clone(), c.direction, c.rhs)).collect(),
            lb: self.variables.iter().map(|v| v.var.bounds().0).collect(),
            ub: self.variables.iter().map(|v| v.var.bounds().1).collect(),
            int: self.variables.iter().map(|v| v.var.integer()).collect(),
        }
    }
}

/// Pulido exacto: con las restricciones activas como igualdades, se resuelve el
/// sistema KKT. Devuelve x y multiplicadores si el resultado es válido.
fn polish(qp: &Qp, x0: &[f64]) -> Option<(Vec<f64>, Vec<f64>)> {
    let n = qp.n();
    let mut act: Vec<(Vec<f64>, f64, Option<usize>, f64)> = vec![]; // a, b, fila de origen, signo del multiplicador
    for (k, (a, d, b)) in qp.rows.iter().enumerate() {
        let lhs: f64 = a.iter().zip(x0).map(|(p, q)| p * q).sum();
        if (lhs - b).abs() <= 1e-6 * b.abs().max(1.0) || *d == Dir::Eq {
            let sign = match d {
                Dir::Le => 1.0,
                Dir::Ge => -1.0,
                Dir::Eq => 0.0,
            };
            act.push((a.clone(), *b, Some(k), sign));
        }
    }
    for j in 0..n {
        for (v, sign) in [(qp.lb[j], -1.0), (qp.ub[j], 1.0)] {
            if v.is_finite() && (x0[j] - v).abs() <= 1e-6 * v.abs().max(1.0) {
                let mut a = vec![0.0; n];
                a[j] = 1.0;
                act.push((a, v, None, sign));
            }
        }
    }
    let k = act.len();
    let mut kkt = DMatrix::zeros(n + k, n + k);
    let mut rhs = DVector::zeros(n + k);
    for i in 0..n {
        for j in 0..n {
            kkt[(i, j)] = qp.p[i][j];
        }
        rhs[i] = -qp.q[i];
    }
    for (r, (a, b, _, _)) in act.iter().enumerate() {
        for j in 0..n {
            kkt[(n + r, j)] = a[j];
            kkt[(j, n + r)] = a[j];
        }
        rhs[n + r] = *b;
    }
    let sol = kkt.clone().lu().solve(&rhs)?;
    if (&kkt * &sol - &rhs).norm() > 1e-8 * rhs.norm().max(1.0) {
        return None;
    }
    let x: Vec<f64> = (0..n).map(|i| tidy(sol[i])).collect();
    if !qp.feasible(&x, 1e-9) {
        return None;
    }
    let mut duals = vec![0.0; qp.rows.len()];
    for (r, (_, _, k, sign)) in act.iter().enumerate() {
        // P·x + q + Σ λ·a = 0, en la convención min f + λ(a·x − b): λ ≥ 0 para ≤, ≤ 0 para ≥
        let l = sol[n + r];
        if *sign != 0.0 && l * sign < -1e-7 {
            return None;
        }
        if let Some(k) = k {
            duals[*k] = tidy(l);
        }
    }
    if (qp.value(&x) - qp.value(x0)) > 1e-7 * qp.value(x0).abs().max(1.0) {
        return None;
    }
    Some((x, duals))
}

pub struct QpMod;

impl TypedModule for QpMod {
    type Model = QpModel;
    type Options = QpOptions;

    fn file_tag(&self) -> &'static str {
        "QP"
    }

    fn spec(&self) -> ModuleSpec {
        let quad_rows = Axis::new("variables", "Variable").add("Añadir variable").extra(Field::number("c", "Término lineal"));
        let quad_cols = Axis::new("variables", "Variable");
        let var_axis = Axis::new("variables", "Variable")
            .add("Añadir variable")
            .extra(Field::bound("lower", "Cota inferior", Value::from(0.0)))
            .extra(Field::bound("upper", "Cota superior", Value::from("M")))
            .extra(Field::choice("type", "Tipo", &[("continuous", "Continua"), ("integer", "Entera"), ("binary", "Binaria"), ("unrestricted", "Libre")]));
        let con_axis = Axis::new("constraints", "Restricción")
            .add("Añadir restricción")
            .extra(Field::choice("direction", "Sentido", &[("<=", "<="), (">=", ">="), ("=", "=")]))
            .extra(Field::number("rhs", "Lado derecho"));
        let mut quad = Widget::matrix(
            Some("Función objetivo: coeficiente de Xi·Xj (fila i, columna j) y término lineal"),
            "quad",
            quad_rows,
            quad_cols,
            Field::number("q", "Coeficiente"),
        );
        if let Widget::Matrix { allow_diagonal, .. } = &mut quad {
            *allow_diagonal = true;
        }
        ModuleSpec {
            id: "qp".into(),
            winqsb: "QP-IQP".into(),
            name: "Programación cuadrática".into(),
            description: "Objetivo cuadrático con restricciones lineales; variables continuas o enteras".into(),
            file_ext: "QPP".into(),
            problem_types: vec![ProblemType {
                id: "quadratic".into(),
                label: "Programa cuadrático".into(),
                description: "Maximizar o minimizar una función cuadrática con restricciones lineales".into(),
                dims: vec![
                    Field::choice("objective", "Criterio", &[("min", "Minimizar"), ("max", "Maximizar")]),
                    Field::integer("variables", "Número de variables", 2),
                    Field::integer("constraints", "Número de restricciones", 1),
                ],
                layout: vec![
                    Widget::params(None, vec![Field::choice("objective", "Criterio", &[("min", "Minimizar"), ("max", "Maximizar")])]),
                    quad,
                    Widget::matrix(Some("Restricciones, cotas y tipos"), "coefs", con_axis, var_axis, Field::number("a", "Coeficiente")),
                ],
            }],
            options: vec![Field::boolean("steps", "Mostrar paso a paso", true)],
        }
    }

    fn template(&self, _pt: &str, dims: &Value) -> Result<QpModel, ModError> {
        let n = dim(dims, "variables", 2, 1, 500)?;
        let m = dim(dims, "constraints", 1, 0, 2000)?;
        Ok(QpModel {
            name: "Cuadrático nuevo".into(),
            problem_type: qp_type(),
            objective: Objective::parse(&dim_str(dims, "objective", "min")),
            variables: (1..=n).map(|j| QVar { var: Var::new(format!("X{j}")), c: 0.0 }).collect(),
            quad: vec![vec![0.0; n]; n],
            constraints: (1..=m).map(|i| Constr { name: format!("C{i}"), direction: Dir::Le, rhs: 0.0 }).collect(),
            coefs: vec![vec![0.0; n]; m],
            normal_form: false,
        })
    }

    fn solve(&self, model: &QpModel, o: &QpOptions) -> Result<Report, ModError> {
        let m = model.normalized()?;
        let names = m.names();
        let n = names.len();
        let qp = m.as_qp();
        let convex = is_convex(&qp.p);
        let (lo, hi) = eigen_range(&m.hessian());
        let mut s = qpsolve::solve(&qp);
        let integer = qp.int.iter().any(|b| *b);
        let mut duals = s.duals.clone();
        if s.status == QpStatus::Optimal && !integer && convex {
            if let Some((x, d)) = polish(&qp, &s.x) {
                s.x = x;
                duals = Some(d);
                s.method = "Clarabel (punto interior) y pulido exacto con las restricciones activas";
            }
        } else {
            s.x = s.x.iter().map(|v| tidy(*v)).collect();
        }
        let sign = if m.objective.is_max() { -1.0 } else { 1.0 };
        let word = if m.objective.is_max() { "máximo" } else { "mínimo" };
        let curv = if lo >= -1e-9 * hi.abs().max(1.0) {
            "convexa"
        } else if hi <= 1e-9 * lo.abs().max(1.0) {
            "cóncava"
        } else {
            "ni convexa ni cóncava"
        };
        let mut conv_blocks = vec![];
        {
            let h = m.hessian();
            let mut ht = Table::new(Some("Matriz hessiana de la función objetivo"), &std::iter::once("".to_string()).chain(names.iter().cloned()).collect::<Vec<_>>()).left(0);
            for i in 0..n {
                let mut row = vec![Cell::from(&names[i])];
                row.extend(h[i].iter().map(|v| Cell::Num(*v)));
                ht.row(row);
            }
            let ok_shape = if m.objective.is_max() { curv == "cóncava" || hi.abs() < 1e-12 } else { curv == "convexa" };
            let verdict = format!(
                "Valores propios de la hessiana entre {} y {}: la función es {}. {}",
                fmt(lo),
                fmt(hi),
                curv,
                if ok_shape {
                    format!("Al buscar el {word}, cualquier óptimo local es global.")
                } else {
                    format!("Al buscar el {word} puede haber óptimos locales que no son globales: se usa una búsqueda global (o se avisa si el problema es muy grande).")
                }
            );
            conv_blocks.push(text_tone(verdict, if ok_shape { Tone::Good } else { Tone::Highlight }));
            conv_blocks.push(ht.block());
        }
        match s.status {
            QpStatus::Infeasible => {
                return Ok(Report::new("qp", &m.name, Status::Infeasible, "Las restricciones no tienen solución.")
                    .section("solucion", "Solución", vec![text_tone("No existe ningún punto que cumpla todas las restricciones y cotas.", Tone::Bad)])
                    .section("convexidad", "Convexidad", conv_blocks)
                    .data(json!({"status": "infeasible"})));
            }
            QpStatus::Unbounded => {
                return Ok(Report::new("qp", &m.name, Status::Unbounded, format!("El objetivo no está acotado: no hay {word}.")
                    )
                    .section("solucion", "Solución", vec![text_tone(format!("La función puede {} sin límite dentro de la región factible.", if m.objective.is_max() { "crecer" } else { "decrecer" }), Tone::Bad)])
                    .section("convexidad", "Convexidad", conv_blocks)
                    .data(json!({"status": "unbounded"})));
            }
            QpStatus::Error => return invalid("el optimizador no pudo resolver el problema (revisa la escala de los datos)"),
            _ => {}
        }
        let x = s.x.clone();
        let z = tidy(m.f(&x));
        let lin: f64 = m.variables.iter().zip(&x).map(|(v, b)| v.c * b).sum();
        let status = if s.status == QpStatus::Local { Status::Feasible } else { Status::Optimal };
        let headline = format!(
            "{} {} = {} en {}.",
            if s.status == QpStatus::Local { "Valor (óptimo local)" } else if m.objective.is_max() { "Máximo" } else { "Mínimo" },
            "Z",
            fmt(z),
            names.iter().zip(&x).map(|(nm, v)| format!("{nm} = {}", fmt(*v))).collect::<Vec<_>>().join(", ")
        );
        // gradiente en el óptimo
        let h = m.hessian();
        let grad: Vec<f64> = (0..n).map(|i| m.variables[i].c + (0..n).map(|j| h[i][j] * x[j]).sum::<f64>()).collect();
        let mut vt = Table::new(Some("Variables"), &["Variable", "Valor", "Término lineal", "Derivada parcial ∂Z/∂x", "Cota inferior", "Cota superior", "Estado"]).left(0).left(6);
        for j in 0..n {
            let (l, u) = m.variables[j].var.bounds();
            let st = if l.is_finite() && (x[j] - l).abs() <= 1e-9 * l.abs().max(1.0) {
                "en su cota inferior"
            } else if u.is_finite() && (x[j] - u).abs() <= 1e-9 * u.abs().max(1.0) {
                "en su cota superior"
            } else {
                "entre cotas"
            };
            vt.row(cells![&names[j], x[j], m.variables[j].c, tidy(grad[j]), Cell::Num(l), Cell::Num(u), st]);
        }
        let mut ct = Table::new(Some("Restricciones"), &["Restricción", "Lado izquierdo", "Sentido", "Lado derecho", "Holgura", "Precio sombra"]).left(0).left(2);
        for (i, c) in m.constraints.iter().enumerate() {
            let lhs = tidy(m.coefs[i].iter().zip(&x).map(|(a, b)| a * b).sum());
            let dual = match (&duals, integer) {
                (Some(d), false) => Cell::Num(tidy(-sign * d[i])),
                _ => Cell::from("—"),
            };
            let slack = tidy((c.rhs - lhs).abs());
            ct.row_tone(cells![&c.name, lhs, c.direction.symbol(), c.rhs, slack, dual], if slack <= 1e-9 * c.rhs.abs().max(1.0) { Tone::Highlight } else { Tone::Normal });
        }
        let ct = ct.note("Precio sombra: cuánto cambia Z por cada unidad más en el lado derecho (solo en problemas continuos convexos).");
        let mut sol = vec![
            metrics(vec![metric_main(if m.objective.is_max() { "Z máximo" } else { "Z mínimo" }, z), metric("Parte lineal", tidy(lin)), metric("Parte cuadrática", tidy(z - lin))]),
            vt.block(),
        ];
        if !m.constraints.is_empty() {
            sol.push(ct.block());
        }
        if s.status == QpStatus::Local {
            sol.insert(1, text_tone("La función no tiene la curvatura adecuada y el problema es demasiado grande para la búsqueda global: este es el mejor óptimo local encontrado.", Tone::Highlight));
        }
        if integer {
            sol.push(text(format!("Ramificación y acotamiento: {} nodos resueltos.", s.nodes)));
        }
        sol.push(text_tone(format!("Método: {}.", s.method), Tone::Muted));

        // pasos
        let mut st = vec![];
        if o.steps {
            let mut v = vec![step("Convexidad", "Lo primero es mirar la curvatura: si la hessiana es semidefinida positiva (minimizar) o negativa (maximizar), las condiciones KKT bastan para el óptimo global.", conv_blocks.clone())];
            // condiciones KKT en el óptimo
            if !integer {
                let mut kt = Table::new(Some("Condiciones KKT en la solución"), &["Variable", "∂Z/∂x", "Σ multiplicadores · coeficiente", "Residuo"]).left(0);
                let d = duals.clone().unwrap_or(vec![0.0; m.constraints.len()]);
                for j in 0..n {
                    let s2: f64 = (0..m.constraints.len()).map(|i| d[i] * m.coefs[i][j]).sum();
                    kt.row(cells![&names[j], tidy(grad[j]), tidy(-sign * s2), tidy(grad[j] + sign * s2)]);
                }
                v.push(step(
                    "Condiciones de Karush-Kuhn-Tucker",
                    "En el óptimo, el gradiente de Z se escribe con los gradientes de las restricciones activas; el residuo distinto de cero solo puede aparecer en variables que están en una cota.",
                    vec![kt.block()],
                ));
            }
            st.push(steps(v));
            if let Some(w) = wolfe_steps(&m, convex, &x) {
                st.push(text("Método de Wolfe (símplex modificado): resuelve las condiciones KKT con el símplex de fase uno y entrada restringida."));
                st.push(Block::Steps { title: Some("Método de Wolfe".into()), steps: w });
            }
        }
        let data = json!({
            "objective_value": z,
            "variables": names.iter().zip(&x).map(|(nm, v)| json!({"name": nm, "value": v})).collect::<Vec<_>>(),
            "convexity": curv,
            "eigenvalues": [lo, hi],
            "local_only": s.status == QpStatus::Local,
            "shadow_prices": duals.filter(|_| !integer).map(|d| d.iter().map(|v| tidy(-sign * v)).collect::<Vec<_>>()),
        });
        Ok(Report::new("qp", &m.name, status, headline)
            .section("solucion", "Solución", sol)
            .section("convexidad", "Convexidad", conv_blocks)
            .section("pasos", "Paso a paso", st)
            .data(data))
    }

    fn read(&self, bytes: &[u8]) -> Result<QpModel, ModError> {
        read_qp(bytes)
    }

    fn write(&self, m: &QpModel) -> Result<Vec<u8>, ModError> {
        Ok(write_qp(&m.normalized()?))
    }

    fn examples(&self) -> Vec<(&'static str, QpModel)> {
        vec![
            ("cartera", serde_json::from_str(include_str!("ejemplos/cartera.json")).unwrap()),
            ("wolfe", serde_json::from_str(include_str!("ejemplos/wolfe.json")).unwrap()),
        ]
    }
}

/// Método de Wolfe si el problema es convexo, continuo, con x ≥ 0 y pequeño.
fn wolfe_steps(m: &QpModel, convex: bool, x_ok: &[f64]) -> Option<Vec<Step>> {
    let n = m.variables.len();
    if !convex || n > 8 || m.constraints.len() > 8 || m.variables.iter().any(|v| v.var.integer() || v.var.bounds().0 != 0.0) {
        return None;
    }
    let qp = m.as_qp();
    let mut a = vec![];
    let mut b = vec![];
    let mut rn = vec![];
    for (c, row) in m.constraints.iter().zip(&m.coefs) {
        match c.direction {
            Dir::Le => {
                a.push(row.clone());
                b.push(c.rhs);
                rn.push(c.name.clone());
            }
            Dir::Ge => {
                a.push(row.iter().map(|v| -v).collect());
                b.push(-c.rhs);
                rn.push(c.name.clone());
            }
            Dir::Eq => {
                a.push(row.clone());
                b.push(c.rhs);
                rn.push(format!("{}+", c.name));
                a.push(row.iter().map(|v| -v).collect());
                b.push(-c.rhs);
                rn.push(format!("{}-", c.name));
            }
        }
    }
    for v in &m.variables {
        let u = v.var.bounds().1;
        if u.is_finite() {
            let mut r = vec![0.0; n];
            r[m.variables.iter().position(|w| w.var.name == v.var.name).unwrap()] = 1.0;
            a.push(r);
            b.push(u);
            rn.push(format!("cota {}", v.var.name));
        }
    }
    let out = wolfe::run(&m.names(), &rn, &qp.p, &qp.q, &a, &b);
    let x = out.x?;
    let ok = (m.f(&x) - m.f(x_ok)).abs() <= 1e-6 * m.f(x_ok).abs().max(1.0);
    ok.then_some(out.steps)
}

// ------------------------------------------------------------------- archivos

fn quad_text(m: &QpModel) -> String {
    let names = m.names();
    let n = names.len();
    let mut terms: Vec<(f64, String)> = vec![];
    for i in 0..n {
        for j in i..n {
            let c = if i == j { m.quad[i][i] } else { m.quad[i][j] + m.quad[j][i] };
            if c != 0.0 {
                terms.push((c, if i == j { format!("{}^2", names[i]) } else { format!("{}*{}", names[i], names[j]) }));
            }
        }
    }
    for (v, nm) in m.variables.iter().zip(&names) {
        if v.c != 0.0 {
            terms.push((v.c, nm.clone()));
        }
    }
    let mut s = String::new();
    for (c, t) in terms {
        let sign = if c < 0.0 {
            "-"
        } else if s.is_empty() {
            ""
        } else {
            "+"
        };
        let mag = c.abs();
        s += &format!("{sign}{}{t}", if mag == 1.0 { String::new() } else { num(mag) });
    }
    if s.is_empty() {
        s = format!("0{}", names[0]);
    }
    s
}

fn read_qp(bytes: &[u8]) -> Result<QpModel, ModError> {
    let mut r = Reader::new(bytes);
    let mut h = r.line("la cabecera")?;
    let tag = h.next().trim().to_ascii_uppercase();
    if !tag.starts_with("QP") && !tag.starts_with("LP") {
        return Err(ModError::Format("no es un archivo de QP-IQP".into()));
    }
    let normal = !h.next().trim().to_ascii_uppercase().starts_with('M');
    let name = h.next().trim().to_string();
    let n = parse_num(&h.next()).unwrap_or(0.0) as usize;
    let mc = parse_num(&h.next()).unwrap_or(0.0) as usize;
    if n == 0 || n > 100_000 || mc > 100_000 {
        return Err(ModError::Format("el número de variables o restricciones no es válido".into()));
    }
    let mut m = QpModel {
        name,
        problem_type: qp_type(),
        objective: Objective::Min,
        variables: vec![],
        quad: vec![vec![0.0; n]; n],
        constraints: vec![],
        coefs: vec![],
        normal_form: normal,
    };
    let is_lp = tag.starts_with("LP");
    if !normal {
        let names = r.line("los nombres de las variables")?;
        let obj = r.line("la función objetivo")?;
        m.objective = Objective::parse(obj.get(0));
        m.variables = (0..n)
            .map(|j| {
                let s = names.get(j + 1).trim();
                QVar { var: Var::new(if s.is_empty() { format!("X{}", j + 1) } else { s.to_string() }), c: parse_num(obj.get(j + 1)).unwrap_or(0.0) }
            })
            .collect();
        if !is_lp {
            for i in 0..n {
                let l = r.line(&format!("los términos cuadráticos de la variable {}", i + 1))?;
                m.quad[i] = (0..n).map(|j| parse_num(l.get(j + 1)).unwrap_or(0.0)).collect();
            }
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
        let mut vars: Vec<Var> = m.variables.iter().map(|v| v.var.clone()).collect();
        read_bound_rows(&mut vars, &lb, &ub, &ty);
        for (q, v) in m.variables.iter_mut().zip(vars) {
            q.var = v;
        }
    } else {
        r.line("la fila de títulos")?;
        let obj = r.line("la función objetivo")?;
        let mut cl = vec![];
        for i in 0..mc {
            cl.push(r.line(&format!("la restricción {}", i + 1))?);
        }
        let ints = r.line("la lista de enteras")?;
        let bins = r.line("la lista de binarias")?;
        let unres = r.line("la lista de libres")?;
        let mut bl = vec![];
        for j in 0..n {
            bl.push(r.line(&format!("las cotas de la variable {}", j + 1))?);
        }
        let mut vars: Vec<Var> = bl
            .iter()
            .enumerate()
            .map(|(j, l)| {
                let s = l.get(0).trim();
                Var::new(if s.is_empty() { format!("X{}", j + 1) } else { s.to_string() })
            })
            .collect();
        let names: Vec<String> = vars.iter().map(|v| v.name.clone()).collect();
        m.objective = Objective::parse(obj.get(0));
        let mut vs = Vars::fixed(&names);
        let e = Expr::parse(obj.get(1), &mut vs).map_err(|e| ModError::Format(format!("función objetivo: {e}")))?;
        let qd = quadratic(&e, n).map_err(|e| ModError::Format(format!("función objetivo: {e}")))?;
        for i in 0..n {
            m.quad[i][i] = qd.q[i][i] / 2.0;
            for j in i + 1..n {
                m.quad[i][j] = qd.q[i][j];
            }
        }
        for (i, l) in cl.iter().enumerate() {
            let at = format!("restricción {}", i + 1);
            let txt: String = l.get(1).chars().filter(|c| !c.is_whitespace()).collect();
            let (lhs, d, rhs) = iaqsb_core::expr::split_relation(&txt).ok_or_else(|| ModError::Format(format!("{at}: falta <=, >= o =")))?;
            let nm = l.get(0).trim();
            m.constraints.push(Constr {
                name: if nm.is_empty() { format!("C{}", i + 1) } else { nm.to_string() },
                direction: Dir::parse(d).unwrap_or(Dir::Le),
                rhs: parse_num(&rhs).ok_or_else(|| ModError::Format(format!("{at}: lado derecho no válido")))?,
            });
            m.coefs.push(coefs_of(&lhs, &names, &at)?);
        }
        apply_kind_list(&mut vars, &ints, VarKind::Integer)?;
        apply_kind_list(&mut vars, &bins, VarKind::Binary)?;
        apply_kind_list(&mut vars, &unres, VarKind::Unrestricted)?;
        for (j, l) in bl.iter().enumerate() {
            if !matches!(vars[j].kind, VarKind::Binary | VarKind::Unrestricted) {
                apply_bounds(&mut vars[j], l.get(1))?;
            }
        }
        m.variables = vars.into_iter().zip(qd.c).map(|(var, c)| QVar { var, c }).collect();
    }
    Ok(m)
}

fn write_qp(m: &QpModel) -> Vec<u8> {
    let mut w = Writer::new();
    let n = m.variables.len();
    let names = m.names();
    let vars: Vec<Var> = m.variables.iter().map(|v| v.var.clone()).collect();
    let word = if m.objective.is_max() { "Maximize" } else { "Minimize" };
    w.row(&["QP".to_string(), if m.normal_form { "NormalModel" } else { "MatrixFormat" }.into(), m.name.clone(), n.to_string(), m.constraints.len().to_string()]);
    if !m.normal_form {
        let mut r = vec!["Variable -->".to_string()];
        r.extend(names.iter().cloned());
        r.extend(["Direction".into(), "R. H. S.".into()]);
        w.row(&r);
        let mut r = vec![word.to_string()];
        r.extend(m.variables.iter().map(|v| num(v.c)));
        r.extend([String::new(), String::new()]);
        w.row(&r);
        for i in 0..n {
            let mut r = vec![format!("{}*", names[i])];
            r.extend(m.quad[i].iter().map(|v| num(*v)));
            r.extend([String::new(), String::new()]);
            w.row(&r);
        }
        for (c, a) in m.constraints.iter().zip(&m.coefs) {
            let mut r = vec![c.name.clone()];
            r.extend(a.iter().map(|x| num(*x)));
            r.extend([c.direction.symbol().into(), num(c.rhs)]);
            w.row(&r);
        }
        for r in bound_rows(&vars, 2) {
            w.row(&r);
        }
    } else {
        w.row(&["OBJ/Constraint/VariableType/Bound", ""]);
        w.row(&[word.to_string(), quad_text(m)]);
        for (c, a) in m.constraints.iter().zip(&m.coefs) {
            w.row(&[c.name.clone(), format!("{}{}{}", linear_text(a, &names), c.direction.symbol(), num(c.rhs))]);
        }
        for r in kind_lists(&vars) {
            w.row(&r);
        }
        for v in &vars {
            let (l, u) = v.bounds();
            w.row(&[v.name.clone(), format!(">={}, <={}", num(l), num(u))]);
        }
    }
    w.bytes()
}

