//! Posiciones de los nodos para dibujar redes (coordenadas en [0, 1]).

use std::collections::VecDeque;

/// Nodos en círculo.
pub fn circle(n: usize) -> Vec<(f64, f64)> {
    (0..n)
        .map(|i| {
            let a = -std::f64::consts::FRAC_PI_2 + 2.0 * std::f64::consts::PI * i as f64 / n.max(1) as f64;
            (0.5 + 0.45 * a.cos(), 0.5 + 0.45 * a.sin())
        })
        .collect()
}

fn spread(k: usize, i: usize) -> f64 {
    if k <= 1 {
        0.5
    } else {
        0.05 + 0.9 * i as f64 / (k - 1) as f64
    }
}

/// Dos columnas: orígenes a la izquierda y destinos a la derecha.
pub fn bipartite(ns: usize, nd: usize) -> Vec<(f64, f64)> {
    let mut v: Vec<(f64, f64)> = (0..ns).map(|i| (0.08, spread(ns, i))).collect();
    v.extend((0..nd).map(|j| (0.92, spread(nd, j))));
    v
}

/// Capas de izquierda a derecha. En un grafo acíclico cada nodo va en la capa de
/// su camino más largo desde el origen; si hay ciclos, por distancia BFS desde `start`.
pub fn layered(n: usize, edges: &[(usize, usize)], start: usize) -> Vec<(f64, f64)> {
    let mut indeg = vec![0usize; n];
    let mut out = vec![vec![]; n];
    for &(a, b) in edges {
        if a != b && a < n && b < n {
            indeg[b] += 1;
            out[a].push(b);
        }
    }
    // orden topológico (Kahn)
    let mut layer = vec![0usize; n];
    let mut q: VecDeque<usize> = (0..n).filter(|&i| indeg[i] == 0).collect();
    let mut seen = 0;
    let mut indeg2 = indeg.clone();
    while let Some(u) = q.pop_front() {
        seen += 1;
        for &v in &out[u] {
            layer[v] = layer[v].max(layer[u] + 1);
            indeg2[v] -= 1;
            if indeg2[v] == 0 {
                q.push_back(v);
            }
        }
    }
    if seen < n {
        // hay ciclos: BFS desde el inicio (sin dirección para alcanzar todo)
        let mut und = vec![vec![]; n];
        for &(a, b) in edges {
            if a < n && b < n {
                und[a].push(b);
                und[b].push(a);
            }
        }
        layer = vec![usize::MAX; n];
        let mut q = VecDeque::new();
        layer[start.min(n.saturating_sub(1))] = 0;
        q.push_back(start.min(n.saturating_sub(1)));
        while let Some(u) = q.pop_front() {
            for &v in &und[u] {
                if layer[v] == usize::MAX {
                    layer[v] = layer[u] + 1;
                    q.push_back(v);
                }
            }
        }
        let maxl = layer.iter().filter(|&&l| l != usize::MAX).max().copied().unwrap_or(0);
        for l in layer.iter_mut() {
            if *l == usize::MAX {
                *l = maxl + 1;
            }
        }
    }
    let nl = layer.iter().max().copied().unwrap_or(0) + 1;
    let mut count = vec![0usize; nl];
    for &l in &layer {
        count[l] += 1;
    }
    let mut idx = vec![0usize; nl];
    let mut pos = vec![(0.0, 0.0); n];
    for i in 0..n {
        let l = layer[i];
        pos[i] = (spread(nl, l), spread(count[l], idx[l]));
        idx[l] += 1;
    }
    pos
}

/// Normaliza posiciones de una cuadrícula (fila, columna) a [0, 1].
pub fn from_grid(cells: &[(i64, i64)]) -> Vec<(f64, f64)> {
    let (rmin, rmax) = cells.iter().fold((i64::MAX, i64::MIN), |a, c| (a.0.min(c.0), a.1.max(c.0)));
    let (cmin, cmax) = cells.iter().fold((i64::MAX, i64::MIN), |a, c| (a.0.min(c.1), a.1.max(c.1)));
    let f = |v: i64, lo: i64, hi: i64| if hi == lo { 0.5 } else { 0.05 + 0.9 * (v - lo) as f64 / (hi - lo) as f64 };
    cells.iter().map(|&(r, c)| (f(c, cmin, cmax), f(r, rmin, rmax))).collect()
}
