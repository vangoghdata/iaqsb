//! Modelos de programación lineal entera de las redes, como se escriben en
//! clase: se muestran (función objetivo, restricciones) y se pueden abrir en el
//! módulo de programación lineal o guardar como .LPP para WinQSB.

use super::NetModel;
use iaqsb_core::cells;
use iaqsb_core::report::*;
use serde_json::{json, Value};

#[allow(dead_code)]
#[derive(Clone, Copy, PartialEq)]
pub enum Rel {
    Le,
    Ge,
    Eq,
}

pub struct LpText {
    pub name: String,
    pub max: bool,
    pub vars: Vec<(String, f64, f64, Option<f64>, &'static str, String)>, // nombre, costo, mín, máx, tipo, significado
    pub rows: Vec<(String, Vec<(usize, f64)>, Rel, f64)>,
}

fn term(c: f64, name: &str, first: bool) -> String {
    let (sign, a) = if c < 0.0 { ("−", -c) } else { ("+", c) };
    let coef = if (a - 1.0).abs() < 1e-12 { String::new() } else { fmt(a) };
    if first {
        format!("{}{coef}{name}", if c < 0.0 { "−" } else { "" })
    } else {
        format!(" {sign} {coef}{name}")
    }
}

impl LpText {
    pub fn new(name: &str, max: bool) -> LpText {
        LpText { name: name.into(), max, vars: vec![], rows: vec![] }
    }
    pub fn var(&mut self, name: String, cost: f64, lo: f64, up: Option<f64>, kind: &'static str, meaning: String) -> usize {
        self.vars.push((name, cost, lo, up, kind, meaning));
        self.vars.len() - 1
    }
    pub fn row(&mut self, name: impl Into<String>, t: Vec<(usize, f64)>, r: Rel, rhs: f64) {
        self.rows.push((name.into(), t, r, rhs));
    }
    fn expr(&self, t: &[(usize, f64)]) -> String {
        if t.is_empty() {
            return "0".into();
        }
        t.iter().enumerate().map(|(k, (v, c))| term(*c, &self.vars[*v].0, k == 0)).collect()
    }
    pub fn objective(&self) -> String {
        let t: Vec<(usize, f64)> = self.vars.iter().enumerate().filter(|(_, v)| v.1 != 0.0).map(|(k, v)| (k, v.1)).collect();
        format!("{} Z = {}", if self.max { "Max" } else { "Min" }, self.expr(&t))
    }
    /// Bloques para mostrar el modelo; con `full = false` solo los conteos.
    pub fn blocks(&self, intro: &str, full: bool) -> Vec<Block> {
        let nb = self.vars.iter().filter(|v| v.4 == "binary").count();
        let mut out = vec![text(format!(
            "{intro} El modelo tiene {} variables ({} binarias) y {} restricciones.",
            self.vars.len(),
            nb,
            self.rows.len()
        ))];
        if !full {
            out.push(text("La red es grande: el detalle de las restricciones no se muestra, pero puedes abrir el modelo completo en Programación lineal."));
            return out;
        }
        out.push(text(self.objective()));
        let mut vt = Table::new(Some("Variables"), &["Variable", "Tipo", "Significado"]).left(0).left(1).left(2);
        for v in &self.vars {
            vt.row(cells![
                &v.0,
                match v.4 {
                    "binary" => "Binaria (0 o 1)",
                    "integer" => "Entera",
                    _ => "Continua",
                },
                &v.5
            ]);
        }
        let mut rt = Table::new(Some("Restricciones"), &["#", "Restricción", "Expresión", "Relación", "Lado derecho"]).left(1).left(2).left(3);
        for (k, (name, t, r, rhs)) in self.rows.iter().enumerate() {
            rt.row(cells![
                k + 1,
                name,
                self.expr(t),
                match r {
                    Rel::Le => "≤",
                    Rel::Ge => "≥",
                    Rel::Eq => "=",
                },
                *rhs
            ]);
        }
        out.push(vt.block());
        out.push(rt.block());
        out
    }
    /// Modelo en el formato del módulo LP-ILP.
    pub fn json(&self) -> Value {
        let nv = self.vars.len();
        let variables: Vec<Value> = self
            .vars
            .iter()
            .map(|v| json!({"name": v.0, "cost": v.1, "lower": v.2, "upper": v.3.map(Value::from).unwrap_or(Value::from("M")), "type": v.4}))
            .collect();
        let constraints: Vec<Value> = self
            .rows
            .iter()
            .enumerate()
            .map(|(k, (_, t, r, rhs))| {
                let mut coefs = vec![0.0; nv];
                for (v, c) in t {
                    coefs[*v] += c;
                }
                json!({"name": format!("C{}", k + 1), "coefs": coefs, "direction": match r { Rel::Le => "<=", Rel::Ge => ">=", Rel::Eq => "=" }, "rhs": rhs})
            })
            .collect();
        json!({"name": self.name, "objective": if self.max { "max" } else { "min" }, "variables": variables, "constraints": constraints})
    }
}

/// Nombres cortos de los nodos para las variables (XAB, X13...).
fn short(m: &NetModel) -> Vec<String> {
    m.nodes.iter().map(|n| n.name.chars().filter(|c| c.is_alphanumeric()).collect()).collect()
}

fn xname(sh: &[String], i: usize, j: usize, used: &mut std::collections::HashSet<String>) -> String {
    // con nombres de más de un carácter se separa con _ (X1_10 y no X110, que se confunde)
    let mut s = if sh.iter().any(|x| x.chars().count() > 1) { format!("X{}_{}", sh[i], sh[j]) } else { format!("X{}{}", sh[i], sh[j]) };
    if s.chars().count() > 12 || !used.insert(s.to_uppercase()) {
        s = format!("X{}_{}", i + 1, j + 1);
        used.insert(s.to_uppercase());
    }
    s
}

/// Ruta más corta como programa entero binario (como en la clase).
pub fn shortest_path(m: &NetModel, o: usize, d: usize) -> LpText {
    let n = m.nodes.len();
    let sh = short(m);
    let mut lp = LpText::new(&format!("Ruta más corta {} a {}", m.nodes[o].name, m.nodes[d].name), m.objective.is_max());
    let mut x = vec![vec![None; n]; n];
    let mut used = std::collections::HashSet::new();
    for i in 0..n {
        for j in 0..n {
            // no se usan arcos que entran al origen ni que salen del destino
            if i == j || j == o || i == d {
                continue;
            }
            if let Some(c) = m.arc(i, j) {
                let name = xname(&sh, i, j, &mut used);
                x[i][j] = Some(lp.var(name, c, 0.0, Some(1.0), "binary", format!("1 si la ruta usa el arco {} → {}", m.nodes[i].name, m.nodes[j].name)));
            }
        }
    }
    let outs = |v: usize| (0..n).filter_map(|j| x[v][j].map(|k| (k, 1.0))).collect::<Vec<_>>();
    let ins = |v: usize| (0..n).filter_map(|i| x[i][v].map(|k| (k, 1.0))).collect::<Vec<_>>();
    lp.row(format!("Del origen {} sale un solo vehículo", m.nodes[o].name), outs(o), Rel::Eq, 1.0);
    lp.row(format!("Al destino {} llega un solo vehículo", m.nodes[d].name), ins(d), Rel::Eq, 1.0);
    for v in 0..n {
        if v == o || v == d {
            continue;
        }
        let mut t = ins(v);
        t.extend(outs(v).into_iter().map(|(k, c)| (k, -c)));
        if !t.is_empty() {
            lp.row(format!("Continuidad en {} (lo que entra sale)", m.nodes[v].name), t, Rel::Eq, 0.0);
        }
    }
    lp
}

/// Agente viajero con la "relajación" de subtours de Miller-Tucker-Zemlin.
pub fn tsp(m: &NetModel, libro: bool) -> LpText {
    let n = m.nodes.len();
    let sh = short(m);
    let mut lp = LpText::new(&if m.name.is_empty() { "Agente viajero".to_string() } else { m.name.clone() }, m.objective.is_max());
    let mut x = vec![vec![None; n]; n];
    let mut used = std::collections::HashSet::new();
    for i in 0..n {
        for j in 0..n {
            if i == j {
                continue;
            }
            if let Some(c) = m.arc(i, j) {
                let name = xname(&sh, i, j, &mut used);
                x[i][j] = Some(lp.var(name, c, 0.0, Some(1.0), "binary", format!("1 si el recorrido va de {} a {}", m.nodes[i].name, m.nodes[j].name)));
            }
        }
    }
    let u: Vec<Option<usize>> = (0..n)
        .map(|i| {
            (i > 0).then(|| {
                let (lo, up, txt) = if libro { (1.0, (n - 1) as f64, "1 ≤ U ≤ n − 1") } else { (2.0, n as f64, "2 ≤ U ≤ n") };
                lp.var(format!("U{}", sh[i]), 0.0, lo, Some(up), "continuous", format!("Orden en que se visita {} ({txt})", m.nodes[i].name))
            })
        })
        .collect();
    for i in 0..n {
        lp.row(format!("De {} sale un solo vehículo", m.nodes[i].name), (0..n).filter_map(|j| x[i][j].map(|k| (k, 1.0))).collect(), Rel::Eq, 1.0);
    }
    for j in 0..n {
        lp.row(format!("A {} llega un solo vehículo", m.nodes[j].name), (0..n).filter_map(|i| x[i][j].map(|k| (k, 1.0))).collect(), Rel::Eq, 1.0);
    }
    // forma de la clase (Sarache): U_i − U_j + 1 ≤ (n − 1)(1 − X_ij)  ⇔  U_i − U_j + (n − 1) X_ij ≤ n − 2,
    // con 2 ≤ U ≤ n; es la de Miller, Tucker y Zemlin (1960) con n = nodos sin contar el inicio.
    // forma de libro: U_i − U_j + n X_ij ≤ n − 1, con 1 ≤ U ≤ n − 1 (n = todos los nodos), un poco más floja.
    // En las dos, i y j son distintos del nodo de inicio (sin la fila ni la columna 1).
    let nn = if libro { n as f64 + 1.0 } else { n as f64 };
    for i in 1..n {
        for j in 1..n {
            if i == j {
                continue;
            }
            if let Some(k) = x[i][j] {
                lp.row(
                    format!("Relajación para {}", lp.vars[k].0),
                    vec![(u[i].unwrap(), 1.0), (u[j].unwrap(), -1.0), (k, nn - 1.0)],
                    Rel::Le,
                    nn - 2.0,
                );
            }
        }
    }
    lp
}

/// Ecuaciones de rompimiento de subtour sin relajación desde un nodo, como en
/// la clase: cada subtour de tres nodos que pasa por el nodo, en los dos
/// sentidos (X_ka + X_ab + X_bk ≤ 2 y X_kb + X_ba + X_ak ≤ 2).
pub fn subtours_from(m: &NetModel, k: usize) -> (Block, usize) {
    let n = m.nodes.len();
    let sh = short(m);
    let mut used = std::collections::HashSet::new();
    let mut names = vec![vec![String::new(); n]; n];
    for i in 0..n {
        for j in 0..n {
            if i != j && m.arc(i, j).is_some() {
                names[i][j] = xname(&sh, i, j, &mut used);
            }
        }
    }
    let nm = |i: usize| m.nodes[i].name.clone();
    let mut t = Table::new(
        Some(&format!("Ecuaciones de rompimiento de subtour sin relajación desde el nodo {}", nm(k))),
        &["#", "Subtour", "Ecuación"],
    )
    .left(1)
    .left(2);
    let others: Vec<usize> = (0..n).filter(|&v| v != k).collect();
    let mut count = 0;
    for (p, &a) in others.iter().enumerate() {
        for &b in &others[p + 1..] {
            for (x, y) in [(a, b), (b, a)] {
                let arcs = [(k, x), (x, y), (y, k)];
                if arcs.iter().any(|&(i, j)| names[i][j].is_empty()) {
                    continue;
                }
                count += 1;
                let eq = format!("{} ≤ 2", arcs.iter().map(|&(i, j)| names[i][j].clone()).collect::<Vec<_>>().join(" + "));
                t.row(cells![count, format!("{}-{}-{}-{}", nm(k), nm(x), nm(y), nm(k)), eq]);
            }
        }
    }
    (t.block(), count)
}
