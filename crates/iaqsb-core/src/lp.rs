//! Programación lineal y entera para los módulos que se reducen a un programa
//! lineal (transporte, flujo en redes, metas, juegos, compresión de proyectos,
//! planificación agregada, ruteo…). Es una capa sencilla sobre el motor de
//! [`crate::solver`]: usa sus parámetros por defecto del hilo (tiempo, brecha,
//! hilos, motor), así que todos los módulos los heredan.

use crate::solver::{self, Kind, Model, Params, Status};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sense {
    Le,
    Ge,
    Eq,
}

#[derive(Debug, Clone)]
pub struct LpRow {
    pub coefs: Vec<(usize, f64)>,
    pub sense: Sense,
    pub rhs: f64,
}

/// Programa lineal: optimizar c·x sujeto a filas y cotas.
#[derive(Debug, Clone, Default)]
pub struct Lp {
    pub maximize: bool,
    pub c: Vec<f64>,
    pub rows: Vec<LpRow>,
    pub lb: Vec<f64>,
    pub ub: Vec<f64>,
    pub integer: Vec<bool>,
    /// Límite de tiempo en segundos (si no, el de los parámetros del hilo).
    pub time_limit: Option<f64>,
    /// Solución inicial factible (arranque del branch and bound).
    pub initial: Option<Vec<f64>>,
}

impl Lp {
    /// n variables continuas en [0, ∞).
    pub fn new(n: usize, maximize: bool) -> Lp {
        Lp { maximize, c: vec![0.0; n], rows: vec![], lb: vec![0.0; n], ub: vec![f64::INFINITY; n], integer: vec![false; n], time_limit: None, initial: None }
    }
    pub fn add_var(&mut self, cost: f64, lb: f64, ub: f64, integer: bool) -> usize {
        self.c.push(cost);
        self.lb.push(lb);
        self.ub.push(ub);
        self.integer.push(integer);
        self.c.len() - 1
    }
    pub fn row(&mut self, coefs: Vec<(usize, f64)>, sense: Sense, rhs: f64) -> usize {
        self.rows.push(LpRow { coefs, sense, rhs });
        self.rows.len() - 1
    }
    pub fn n(&self) -> usize {
        self.c.len()
    }

    /// El mismo problema como modelo del motor.
    pub fn to_model(&self) -> Model {
        let mut m = Model { maximize: self.maximize, ..Default::default() };
        for j in 0..self.n() {
            m.add_col(format!("x{}", j + 1), self.c[j], self.lb[j], self.ub[j], if self.integer[j] { Kind::Integer } else { Kind::Continuous });
        }
        for (i, r) in self.rows.iter().enumerate() {
            let (lo, hi) = match r.sense {
                Sense::Le => (f64::NEG_INFINITY, r.rhs),
                Sense::Ge => (r.rhs, f64::INFINITY),
                Sense::Eq => (r.rhs, r.rhs),
            };
            m.add_row(format!("r{}", i + 1), r.coefs.clone(), lo, hi);
        }
        m
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum LpStatus {
    Optimal,
    Infeasible,
    Unbounded,
    Error(String),
}

#[derive(Debug, Clone)]
pub struct LpSolution {
    pub status: LpStatus,
    pub x: Vec<f64>,
    pub objective: f64,
    /// Precios sombra de las filas (problemas continuos).
    pub duals: Option<Vec<f64>>,
    /// Se detuvo por un límite (tiempo, nodos): la solución es la mejor encontrada.
    pub limit_reached: bool,
    /// Detalle del motor (tiempo, brecha, cota, nodos).
    pub detail: Option<solver::Solution>,
}

impl LpSolution {
    pub fn is_optimal(&self) -> bool {
        self.status == LpStatus::Optimal
    }
}

/// Resuelve el programa lineal o entero.
pub fn solve(lp: &Lp) -> LpSolution {
    let mut m = lp.to_model();
    let n = lp.n();
    let mut p = Params { time_limit: lp.time_limit, ..Default::default() };
    if let Some(init) = lp.initial.as_ref().filter(|v| v.len() == n) {
        p.start = Some(m.cols.iter().zip(init).map(|(c, v)| (c.name.clone(), *v)).collect());
    }
    // el multiobjetivo no aplica aquí
    m.objectives.clear();
    let s = solver::solve(&m, &p);
    let limit = matches!(s.status, Status::TimeLimit | Status::NodeLimit | Status::SolutionLimit | Status::Interrupted | Status::IterationLimit);
    let status = match s.status {
        Status::Optimal => LpStatus::Optimal,
        Status::Infeasible => LpStatus::Infeasible,
        Status::Unbounded | Status::InfeasibleOrUnbounded => LpStatus::Unbounded,
        _ if limit && s.has_solution => LpStatus::Optimal,
        Status::TimeLimit => LpStatus::Error("se acabó el tiempo sin encontrar una solución".into()),
        _ => LpStatus::Error(if s.message.is_empty() { s.status.label().to_string() } else { s.message.clone() }),
    };
    let objective = s.objective.unwrap_or(f64::NAN);
    LpSolution { status, x: s.x.clone(), objective, duals: s.row_dual.clone(), limit_reached: limit && s.has_solution, detail: Some(s) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wyndor_con_duales() {
        let mut lp = Lp::new(2, true);
        lp.c = vec![3.0, 5.0];
        lp.row(vec![(0, 1.0)], Sense::Le, 4.0);
        lp.row(vec![(1, 2.0)], Sense::Le, 12.0);
        lp.row(vec![(0, 3.0), (1, 2.0)], Sense::Le, 18.0);
        let s = solve(&lp);
        assert!(s.is_optimal());
        assert!((s.objective - 36.0).abs() < 1e-7);
        let d = s.duals.unwrap();
        assert!((d[1].abs() - 1.5).abs() < 1e-7 && (d[2].abs() - 1.0).abs() < 1e-7, "{d:?}");
    }

    #[test]
    fn entero_y_estados() {
        let mut lp = Lp::new(2, true);
        lp.c = vec![5.0, 8.0];
        lp.integer = vec![true, true];
        lp.row(vec![(0, 1.0), (1, 1.0)], Sense::Le, 6.0);
        lp.row(vec![(0, 5.0), (1, 9.0)], Sense::Le, 45.0);
        assert!((solve(&lp).objective - 40.0).abs() < 1e-7);

        let mut inf = Lp::new(1, true);
        inf.c = vec![1.0];
        inf.row(vec![(0, 1.0)], Sense::Ge, 3.0);
        inf.row(vec![(0, 1.0)], Sense::Le, 2.0);
        assert_eq!(solve(&inf).status, LpStatus::Infeasible);

        let mut unb = Lp::new(1, true);
        unb.c = vec![1.0];
        unb.row(vec![(0, 1.0)], Sense::Ge, 1.0);
        assert!(matches!(solve(&unb).status, LpStatus::Unbounded | LpStatus::Infeasible));
    }
}
