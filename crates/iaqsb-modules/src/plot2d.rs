//! Gráficos de funciones de dos variables: curvas de nivel (cuadrados
//! marchantes), fronteras de restricciones y región factible como puntos.

use iaqsb_core::report::*;

/// Segmentos de la curva f(x, y) = nivel, separados por puntos vacíos.
pub fn contour(f: &dyn Fn(f64, f64) -> f64, xr: (f64, f64), yr: (f64, f64), nx: usize, ny: usize, level: f64) -> Vec<[f64; 2]> {
    let dx = (xr.1 - xr.0) / nx as f64;
    let dy = (yr.1 - yr.0) / ny as f64;
    let mut v = vec![vec![0.0; ny + 1]; nx + 1];
    for (i, col) in v.iter_mut().enumerate() {
        for (j, c) in col.iter_mut().enumerate() {
            *c = f(xr.0 + i as f64 * dx, yr.0 + j as f64 * dy) - level;
        }
    }
    let mut out = vec![];
    let lerp = |a: f64, b: f64| if (a - b).abs() < 1e-300 { 0.5 } else { a / (a - b) };
    for i in 0..nx {
        for j in 0..ny {
            let (x0, y0) = (xr.0 + i as f64 * dx, yr.0 + j as f64 * dy);
            let c = [v[i][j], v[i + 1][j], v[i + 1][j + 1], v[i][j + 1]];
            if c.iter().any(|t| !t.is_finite()) {
                continue;
            }
            // cortes en las cuatro aristas
            let mut pts: Vec<[f64; 2]> = vec![];
            let edges = [((0, 1), (x0, y0), (dx, 0.0)), ((1, 2), (x0 + dx, y0), (0.0, dy)), ((3, 2), (x0, y0 + dy), (dx, 0.0)), ((0, 3), (x0, y0), (0.0, dy))];
            for ((a, b), (px, py), (ex, ey)) in edges {
                if (c[a] > 0.0) != (c[b] > 0.0) {
                    let t = lerp(c[a], c[b]);
                    pts.push([px + t * ex, py + t * ey]);
                }
            }
            if pts.len() == 2 {
                out.extend([pts[0], pts[1], [f64::NAN, f64::NAN]]);
            } else if pts.len() == 4 {
                out.extend([pts[0], pts[1], [f64::NAN, f64::NAN], pts[2], pts[3], [f64::NAN, f64::NAN]]);
            }
        }
    }
    out
}

/// Gráfico con curvas de nivel de f, fronteras de las restricciones, la región
/// factible (puntos tenues) y el óptimo.
#[allow(clippy::too_many_arguments)]
pub fn level_chart(
    title: &str,
    names: (&str, &str),
    f: &dyn Fn(f64, f64) -> f64,
    cons: &[(String, Box<dyn Fn(f64, f64) -> f64 + '_>)],
    feasible: &dyn Fn(f64, f64) -> bool,
    xr: (f64, f64),
    yr: (f64, f64),
    opt: Option<(f64, f64)>,
) -> Block {
    let n = 48;
    // niveles: cuantiles de los valores en la malla (y el del óptimo)
    let mut vals = vec![];
    for i in 0..=n {
        for j in 0..=n {
            let v = f(xr.0 + (xr.1 - xr.0) * i as f64 / n as f64, yr.0 + (yr.1 - yr.0) * j as f64 / n as f64);
            if v.is_finite() {
                vals.push(v);
            }
        }
    }
    vals.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let mut ch = Chart::new(Some(title), names.0, names.1);
    if !vals.is_empty() {
        // niveles más juntos cerca del mínimo (se ven los valles estrechos)
        let lo = vals[0].min(opt.map(|(x, y)| f(x, y)).unwrap_or(vals[0]));
        let hi = vals[((vals.len() - 1) as f64 * 0.85) as usize];
        let mut levels: Vec<f64> = [0.004, 0.015, 0.04, 0.09, 0.17, 0.3, 0.5, 0.75].iter().map(|t| lo + (hi - lo) * t).collect();
        if let Some((x, y)) = opt {
            levels.push(f(x, y));
        }
        levels.sort_by(|a, b| a.partial_cmp(b).unwrap());
        levels.dedup_by(|a, b| (*a - *b).abs() <= 1e-9 * a.abs().max(1.0));
        let mut pts = vec![];
        let mut at_opt = vec![];
        for l in &levels {
            let seg = contour(f, xr, yr, n, n, *l);
            let is_opt = opt.map(|(x, y)| (f(x, y) - l).abs() <= 1e-9 * l.abs().max(1.0)).unwrap_or(false);
            if is_opt {
                at_opt = seg;
            } else {
                pts.extend(seg);
            }
        }
        ch = ch.series(Series::new("Curvas de nivel", SeriesKind::Line, pts).tone(Tone::Muted));
        if !at_opt.is_empty() {
            ch = ch.series(Series::new("Nivel del óptimo", SeriesKind::Line, at_opt).tone(Tone::Accent));
        }
    }
    for (name, g) in cons {
        let seg = contour(&|x, y| g(x, y), xr, yr, n, n, 0.0);
        if !seg.is_empty() {
            ch = ch.series(Series::new(name.clone(), SeriesKind::Line, seg).tone(Tone::Highlight).dashed());
        }
    }
    if !cons.is_empty() {
        let m = 34;
        let mut dots = vec![];
        for i in 0..=m {
            for j in 0..=m {
                let (x, y) = (xr.0 + (xr.1 - xr.0) * i as f64 / m as f64, yr.0 + (yr.1 - yr.0) * j as f64 / m as f64);
                if feasible(x, y) {
                    dots.push([x, y]);
                }
            }
        }
        if !dots.is_empty() {
            ch = ch.series(Series::new("Región factible", SeriesKind::Scatter, dots).tone(Tone::Good));
        }
    }
    if let Some((x, y)) = opt {
        ch = ch.series(Series::new("Óptimo", SeriesKind::Scatter, vec![[x, y]]).tone(Tone::Accent));
    }
    ch.block()
}

/// Rango de dibujo alrededor de un punto, respetando cotas finitas.
pub fn range_around(c: f64, lo: f64, hi: f64) -> (f64, f64) {
    let w = 2.0 * (1.0 + c.abs());
    let mut a = c - w;
    let mut b = c + w;
    if lo.is_finite() {
        a = a.max(lo - 0.05 * w);
    }
    if hi.is_finite() {
        b = b.min(hi + 0.05 * w);
    }
    if lo.is_finite() && hi.is_finite() && hi - lo < 4.0 * w {
        let pad = 0.08 * (hi - lo).max(1e-9);
        a = lo - pad;
        b = hi + pad;
    }
    (a, b)
}
