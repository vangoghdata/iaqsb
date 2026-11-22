//! Método gráfico para problemas de dos variables ("Graphic Method" de WinQSB):
//! región factible, rectas de las restricciones y punto óptimo.

use crate::model::{Direction, LpModel};
use crate::{solve, LpError, SolveOptions, Status};
use schemars::JsonSchema;
use serde::Serialize;

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct Line {
    pub name: String,
    /// a*x + b*y (direction) rhs
    pub a: f64,
    pub b: f64,
    pub direction: Direction,
    pub rhs: f64,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct GraphicData {
    pub x_name: String,
    pub y_name: String,
    pub x_range: [f64; 2],
    pub y_range: [f64; 2],
    /// Vértices de la región factible (recortada a la ventana si no está acotada).
    pub feasible_region: Vec<[f64; 2]>,
    /// La región sigue más allá de la ventana dibujada.
    pub region_unbounded: bool,
    pub lines: Vec<Line>,
    pub optimum: Option<[f64; 2]>,
    pub objective_value: Option<f64>,
    pub objective_coefs: [f64; 2],
    pub status: Status,
}

pub fn graphic(model: &LpModel, x: usize, y: usize) -> Result<GraphicData, LpError> {
    model.validate()?;
    let n = model.variables.len();
    if x >= n || y >= n || x == y {
        return Err(LpError::Invalid("elige dos variables distintas".into()));
    }
    if n > 2 {
        return Err(LpError::Invalid(
            "el método gráfico necesita exactamente dos variables (WinQSB pide fijar las demás)"
                .into(),
        ));
    }
    let rep = solve(model, &SolveOptions::default())?;
    let optimum = if rep.status == Status::Optimal {
        Some([rep.variables[x].value, rep.variables[y].value])
    } else {
        None
    };

    // semiplanos: a*x + b*y <= c
    let mut half: Vec<(f64, f64, f64)> = Vec::new();
    let mut lines = Vec::new();
    for c in &model.constraints {
        let (a, b) = (c.coefs[x], c.coefs[y]);
        lines.push(Line {
            name: c.name.clone(),
            a,
            b,
            direction: c.direction,
            rhs: c.rhs,
        });
        match c.direction {
            Direction::Le => half.push((a, b, c.rhs)),
            Direction::Ge => half.push((-a, -b, -c.rhs)),
            Direction::Eq => {
                half.push((a, b, c.rhs));
                half.push((-a, -b, -c.rhs));
            }
        }
    }
    let (lx, ux) = model.effective_bounds(x);
    let (ly, uy) = model.effective_bounds(y);
    for (lo, hi, ax, ay) in [(lx, ux, 1.0, 0.0), (ly, uy, 0.0, 1.0)] {
        if lo.is_finite() {
            half.push((-ax, -ay, -lo));
        }
        if hi.is_finite() {
            half.push((ax, ay, hi));
        }
    }

    // ventana: abarca intersecciones con los ejes, cotas y el óptimo
    let mut ext: f64 = 1.0;
    for &(a, b, c) in &half {
        if a.abs() > 1e-12 {
            ext = ext.max((c / a).abs());
        }
        if b.abs() > 1e-12 {
            ext = ext.max((c / b).abs());
        }
    }
    if let Some(p) = optimum {
        ext = ext.max(p[0].abs()).max(p[1].abs());
    }
    let xmin = if lx.is_finite() {
        lx.min(0.0)
    } else {
        -ext * 1.2
    };
    let ymin = if ly.is_finite() {
        ly.min(0.0)
    } else {
        -ext * 1.2
    };
    let xmax = ext * 1.2;
    let ymax = ext * 1.2;
    let window = [
        (1.0, 0.0, xmax),
        (0.0, 1.0, ymax),
        (-1.0, 0.0, -xmin),
        (0.0, -1.0, -ymin),
    ];
    let mut all = half.clone();
    all.extend_from_slice(&window);

    let feasible = |p: [f64; 2]| {
        all.iter()
            .all(|&(a, b, c)| a * p[0] + b * p[1] <= c + 1e-7 * (1.0 + c.abs()))
    };
    let mut pts: Vec<[f64; 2]> = Vec::new();
    for i in 0..all.len() {
        for j in i + 1..all.len() {
            let (a1, b1, c1) = all[i];
            let (a2, b2, c2) = all[j];
            let det = a1 * b2 - a2 * b1;
            if det.abs() < 1e-12 {
                continue;
            }
            let p = [(c1 * b2 - c2 * b1) / det, (a1 * c2 - a2 * c1) / det];
            if feasible(p)
                && !pts
                    .iter()
                    .any(|q| (q[0] - p[0]).abs() < 1e-9 && (q[1] - p[1]).abs() < 1e-9)
            {
                pts.push(p);
            }
        }
    }
    // ordenar por ángulo alrededor del centro
    if !pts.is_empty() {
        let cx = pts.iter().map(|p| p[0]).sum::<f64>() / pts.len() as f64;
        let cy = pts.iter().map(|p| p[1]).sum::<f64>() / pts.len() as f64;
        pts.sort_by(|p, q| {
            (p[1] - cy)
                .atan2(p[0] - cx)
                .partial_cmp(&(q[1] - cy).atan2(q[0] - cx))
                .unwrap()
        });
    }
    let on_window = pts.iter().any(|p| {
        window
            .iter()
            .any(|&(a, b, c)| (a * p[0] + b * p[1] - c).abs() < 1e-7 * (1.0 + c.abs()))
            && !half
                .iter()
                .any(|&(a, b, c)| (a * p[0] + b * p[1] - c).abs() < 1e-7 * (1.0 + c.abs()))
    });

    Ok(GraphicData {
        x_name: model.variables[x].name.clone(),
        y_name: model.variables[y].name.clone(),
        x_range: [xmin, xmax],
        y_range: [ymin, ymax],
        feasible_region: pts,
        region_unbounded: on_window,
        lines,
        optimum,
        objective_value: rep.objective_value,
        objective_coefs: [model.variables[x].cost, model.variables[y].cost],
        status: rep.status,
    })
}
