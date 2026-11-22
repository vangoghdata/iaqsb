//! Banco de pruebas: símplex de tablas de IAQSB (el de WinQSB) frente al motor (HiGHS).
//!
//!   cargo run --profile pruebas -p iaqsb-lp --example banco
use iaqsb_lp::model::LpModel;
use serde_json::json;
use iaqsb_lp::{report, SolveOptions};
use std::time::Instant;

/// Generador congruencial (reproducible, sin dependencias).
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> f64 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        ((self.0 >> 33) as f64) / ((1u64 << 31) as f64)
    }
}

/// Transporte de `o` orígenes y `d` destinos.
fn transporte(o: usize, d: usize, seed: u64) -> LpModel {
    let mut r = Rng(seed);
    let n = o * d;
    let variables: Vec<_> = (0..n).map(|k| json!({"name": format!("x{}_{}", k / d, k % d), "cost": (1.0 + 99.0 * r.next()).round()})).collect();
    let oferta: Vec<f64> = (0..o).map(|_| (50.0 + 100.0 * r.next()).round()).collect();
    let total: f64 = oferta.iter().sum();
    let mut constraints = vec![];
    for i in 0..o {
        let mut coefs = vec![0.0; n];
        for j in 0..d {
            coefs[i * d + j] = 1.0;
        }
        constraints.push(json!({"name": format!("O{i}"), "coefs": coefs, "direction": "<=", "rhs": oferta[i]}));
    }
    for j in 0..d {
        let mut coefs = vec![0.0; n];
        for i in 0..o {
            coefs[i * d + j] = 1.0;
        }
        constraints.push(json!({"name": format!("D{j}"), "coefs": coefs, "direction": ">=", "rhs": (0.9 * total / d as f64).floor()}));
    }
    serde_json::from_value(json!({"name": format!("transporte {o}x{d}"), "objective": "min", "variables": variables, "constraints": constraints})).unwrap()
}

fn main() {
    println!("| Modelo | Variables × restricciones | Símplex de tablas | Motor (HiGHS) | Mismo óptimo |");
    println!("| :-- | --: | --: | --: | :-: |");
    for (o, d) in [(10, 15), (20, 30), (30, 50), (40, 60), (60, 90)] {
        let m = transporte(o, d, 7 + o as u64);
        let opts = SolveOptions { trace: false, ..SolveOptions::default() };
        let bounds: Vec<(f64, f64)> = (0..m.variables.len()).map(|j| m.effective_bounds(j)).collect();
        let t = Instant::now();
        let (a, _) = report::solve_lp(&m, &bounds, &opts, false);
        let ta = t.elapsed().as_secs_f64();
        let em = iaqsb_lp::engine::to_engine(&m, false);
        let t = Instant::now();
        let b = iaqsb_core::solver::solve(&em, &Default::default());
        let tb = t.elapsed().as_secs_f64();
        let same = match (a.objective_value, b.objective) {
            (Some(x), Some(y)) => (x - y).abs() < 1e-6 * (1.0 + y.abs()),
            _ => false,
        };
        println!(
            "| {} | {} × {} | {:.3} s | {:.3} s | {} |",
            m.name,
            m.variables.len(),
            m.constraints.len(),
            ta,
            tb,
            if same { "sí" } else { "no" }
        );
    }
}
