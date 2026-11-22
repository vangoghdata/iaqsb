//! Métodos de ruteo y armado del informe.

use super::{haversine_km, Direction, Method, Metric, VrpModel, VrpOptions, VrpType, EARTH_RADIUS_KM};
use iaqsb_core::lp::{self, Lp, Sense};
use iaqsb_core::report::*;
use iaqsb_core::{cells, ModError};
use serde_json::{json, Value};
use std::time::Instant;

const EPS: f64 = 1e-9;

/// Datos listos para calcular: distancias completas, demandas y coordenadas.
pub struct Inst {
    pub n: usize,
    pub names: Vec<String>,
    pub q: Vec<f64>,
    pub cap: f64,
    pub d: Vec<Vec<f64>>,
    /// Siguiente punto en la ruta más corta (red de calles).
    pub next: Option<Vec<Vec<usize>>>,
    /// Coordenadas para los ángulos del barrido. Con la distancia geográfica, km
    /// al este y al norte del centro (proyección equirectangular).
    pub xy: Vec<(f64, f64)>,
    pub real_xy: bool,
    /// Puntos en latitud y longitud: (longitud, latitud) en grados, para el mapa.
    pub geo: Option<Vec<(f64, f64)>>,
    pub lmax: Option<f64>,
    pub symmetric: bool,
    /// Pares sin calle directa que se completaron con la ruta más corta.
    pub filled: usize,
}

pub fn build(m: &VrpModel) -> Result<Inst, ModError> {
    let n = m.points.len();
    let names: Vec<String> = m.points.iter().map(|p| p.name.clone()).collect();
    let q: Vec<f64> = m.points.iter().enumerate().map(|(i, p)| if i == 0 { 0.0 } else { p.demand }).collect();
    let real_xy = m.has_coords();
    let mut d = vec![vec![0.0; n]; n];
    let mut next = None;
    let mut filled = 0;
    match m.problem_type {
        VrpType::Coordinates => {
            for i in 0..n {
                for j in 0..n {
                    let (a, b) = (&m.points[i], &m.points[j]);
                    let dx = a.x.unwrap() - b.x.unwrap();
                    let dy = a.y.unwrap() - b.y.unwrap();
                    d[i][j] = match m.metric {
                        Metric::Euclidean => dx.hypot(dy),
                        Metric::Rectilinear => dx.abs() + dy.abs(),
                        // en km, redondeada al metro para que la tabla se pueda revisar a mano
                        Metric::Haversine => (haversine_km(a.y.unwrap(), a.x.unwrap(), b.y.unwrap(), b.x.unwrap()) * 1000.0).round() / 1000.0,
                    };
                }
            }
        }
        VrpType::Network => {
            let inf = f64::INFINITY;
            let mut nx = vec![vec![usize::MAX; n]; n];
            for i in 0..n {
                for j in 0..n {
                    let direct = m.distances[i][j].or(if m.symmetric { m.distances[j][i] } else { None });
                    d[i][j] = if i == j { 0.0 } else { direct.unwrap_or(inf) };
                    if i != j && direct.is_none() {
                        filled += 1;
                    }
                    if d[i][j].is_finite() {
                        nx[i][j] = j;
                    }
                }
            }
            for k in 0..n {
                for i in 0..n {
                    if !d[i][k].is_finite() {
                        continue;
                    }
                    for j in 0..n {
                        let v = d[i][k] + d[k][j];
                        if v < d[i][j] - 1e-12 {
                            d[i][j] = v;
                            nx[i][j] = nx[i][k];
                        }
                    }
                }
            }
            for i in 0..n {
                for j in 0..n {
                    if !d[i][j].is_finite() {
                        return Err(ModError::Invalid(format!("no hay ningún camino de {} a {} en la red de calles", names[i], names[j])));
                    }
                }
            }
            next = Some(nx);
        }
    }
    let geo = (m.geographic() && real_xy).then(|| m.points.iter().map(|p| (p.x.unwrap(), p.y.unwrap())).collect::<Vec<_>>());
    let xy: Vec<(f64, f64)> = if let Some(ll) = &geo {
        // los ángulos se miden en el plano: km al este y al norte del centro de los puntos
        // (la misma proyección del mapa, con cos de la latitud media)
        let (lo, hi) = ll.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |a, p| (a.0.min(p.1), a.1.max(p.1)));
        let latc = (lo + hi) / 2.0;
        let k = EARTH_RADIUS_KM * std::f64::consts::PI / 180.0;
        let (lon0, lat0) = ll[0];
        ll.iter().map(|&(lon, lat)| ((lon - lon0) * k * latc.to_radians().cos(), (lat - lat0) * k)).collect()
    } else if real_xy {
        m.points.iter().map(|p| (p.x.unwrap(), p.y.unwrap())).collect()
    } else {
        // sin coordenadas: el depósito al centro y los clientes en círculo
        (0..n)
            .map(|i| {
                if i == 0 {
                    (0.0, 0.0)
                } else {
                    let a = std::f64::consts::FRAC_PI_2 - std::f64::consts::TAU * ((i - 1) as f64) / ((n - 1) as f64);
                    (10.0 * a.cos(), 10.0 * a.sin())
                }
            })
            .collect()
    };
    let symmetric = (0..n).all(|i| (0..n).all(|j| (d[i][j] - d[j][i]).abs() <= 1e-9 * (1.0 + d[i][j].abs())));
    Ok(Inst { n, names, q, cap: m.capacity, d, next, xy, real_xy, geo, lmax: m.max_distance.filter(|l| *l > 0.0), symmetric, filled })
}

impl Inst {
    pub fn rlen(&self, r: &[usize]) -> f64 {
        if r.is_empty() {
            return 0.0;
        }
        let mut s = self.d[0][r[0]] + self.d[r[r.len() - 1]][0];
        for w in r.windows(2) {
            s += self.d[w[0]][w[1]];
        }
        s
    }
    pub fn load(&self, r: &[usize]) -> f64 {
        r.iter().map(|&i| self.q[i]).sum()
    }
    fn fits_len(&self, r: &[usize]) -> bool {
        self.lmax.is_none_or(|l| self.rlen(r) <= l + 1e-9)
    }
    fn customers(&self) -> Vec<usize> {
        (1..self.n).collect()
    }
    /// Distancia con su unidad (km con coordenadas geográficas).
    pub fn dist_txt(&self, v: f64) -> String {
        if self.geo.is_some() {
            format!("{} km", fmt(clean(v)))
        } else {
            fmt(clean(v))
        }
    }
    /// Ángulo del cliente visto desde el depósito, en grados [0, 360).
    pub fn angle(&self, i: usize) -> f64 {
        let (x0, y0) = self.xy[0];
        let (x, y) = self.xy[i];
        (y - y0).atan2(x - x0).to_degrees().rem_euclid(360.0)
    }
    fn route_names(&self, r: &[usize]) -> String {
        let mut v = vec![self.names[0].clone()];
        v.extend(r.iter().map(|&i| self.names[i].clone()));
        v.push(self.names[0].clone());
        v.join(" → ")
    }
    /// Recorrido por las calles (con los puntos intermedios de las rutas más cortas).
    fn street_path(&self, r: &[usize]) -> Option<String> {
        let nx = self.next.as_ref()?;
        let mut seq = vec![0];
        seq.extend_from_slice(r);
        seq.push(0);
        let mut out = vec![0usize];
        let mut extra = false;
        for w in seq.windows(2) {
            let (mut a, b) = (w[0], w[1]);
            while a != b {
                a = nx[a][b];
                if a != b {
                    extra = true;
                }
                out.push(a);
            }
        }
        extra.then(|| out.iter().map(|&i| self.names[i].clone()).collect::<Vec<_>>().join(" → "))
    }
}

// ------------------------------------------------------------- recorridos

/// Mejor orden de un grupo de clientes: exacto (Held-Karp) hasta 12, si no vecino más cercano + 2-opt.
pub fn best_order(inst: &Inst, set: &[usize]) -> Vec<usize> {
    let k = set.len();
    if k <= 2 {
        let mut v = set.to_vec();
        if k == 2 && inst.rlen(&[v[1], v[0]]) < inst.rlen(&v) - EPS {
            v.swap(0, 1);
        }
        return v;
    }
    if k <= 12 {
        let full = (1usize << k) - 1;
        let inf = f64::INFINITY;
        let mut dp = vec![vec![inf; k]; 1 << k];
        let mut par = vec![vec![usize::MAX; k]; 1 << k];
        for j in 0..k {
            dp[1 << j][j] = inst.d[0][set[j]];
        }
        for mask in 1..=full {
            for j in 0..k {
                let cur = dp[mask][j];
                if !cur.is_finite() || mask & (1 << j) == 0 {
                    continue;
                }
                for t in 0..k {
                    if mask & (1 << t) != 0 {
                        continue;
                    }
                    let nm = mask | (1 << t);
                    let v = cur + inst.d[set[j]][set[t]];
                    if v < dp[nm][t] - 1e-12 {
                        dp[nm][t] = v;
                        par[nm][t] = j;
                    }
                }
            }
        }
        let mut best = (inf, 0);
        for j in 0..k {
            let v = dp[full][j] + inst.d[set[j]][0];
            if v < best.0 - 1e-12 {
                best = (v, j);
            }
        }
        let mut order = vec![];
        let (mut mask, mut j) = (full, best.1);
        while j != usize::MAX {
            order.push(set[j]);
            let pj = par[mask][j];
            mask &= !(1 << j);
            j = pj;
        }
        order.reverse();
        return order;
    }
    let mut left: Vec<usize> = set.to_vec();
    let mut r = vec![];
    let mut cur = 0;
    while !left.is_empty() {
        let (pos, _) = left.iter().enumerate().min_by(|a, b| inst.d[cur][*a.1].total_cmp(&inst.d[cur][*b.1])).unwrap();
        cur = left.remove(pos);
        r.push(cur);
    }
    two_opt(inst, &r).0
}

/// Intercambios 2-opt dentro de una ruta: devuelve la ruta mejorada y las mejoras hechas.
pub fn two_opt(inst: &Inst, r: &[usize]) -> (Vec<usize>, Vec<(String, f64)>) {
    let mut best = r.to_vec();
    let mut log = vec![];
    if best.len() < 3 {
        return (best, log);
    }
    let mut cur = inst.rlen(&best);
    loop {
        let mut improved = false;
        'outer: for i in 0..best.len() - 1 {
            for j in i + 1..best.len() {
                let mut cand = best.clone();
                cand[i..=j].reverse();
                let v = inst.rlen(&cand);
                if v < cur - 1e-9 {
                    log.push((format!("Se invierte el tramo de {} a {}", inst.names[best[i]], inst.names[best[j]]), cur - v));
                    best = cand;
                    cur = v;
                    improved = true;
                    break 'outer;
                }
            }
        }
        if !improved {
            break;
        }
    }
    (best, log)
}

// ------------------------------------------------------------- soluciones

pub struct Sol {
    pub method: Method,
    pub label: &'static str,
    pub routes: Vec<Vec<usize>>,
    pub steps: Vec<Step>,
    /// Ángulos donde se cierra cada zona del barrido.
    pub rays: Vec<f64>,
    pub start_angle: Option<f64>,
    pub note: Option<String>,
    pub proven: bool,
}

impl Sol {
    fn total(&self, inst: &Inst) -> f64 {
        self.routes.iter().map(|r| inst.rlen(r)).sum()
    }
}

fn improve_all(inst: &Inst, routes: &mut [Vec<usize>], steps: &mut Vec<Step>, show: bool) {
    let mut t = Table::new(Some("Mejoras con 2-opt"), &["Ruta", "Cambio", "Ahorro"]).left(1);
    let mut any = false;
    for (k, r) in routes.iter_mut().enumerate() {
        let (nr, log) = two_opt(inst, r);
        for (msg, gain) in log {
            any = true;
            t.row(cells![k + 1, msg, gain]);
        }
        *r = nr;
    }
    if show {
        steps.push(step(
            "Mejora con 2-opt",
            if any {
                "En cada ruta se prueba invertir un tramo del recorrido; si la ruta queda más corta, se deja el cambio."
            } else {
                "En cada ruta se prueba invertir un tramo del recorrido. Ningún cambio acorta las rutas: ya no hay cruces que eliminar."
            },
            if any { vec![t.block()] } else { vec![] },
        ));
    }
}

fn routes_table(inst: &Inst, routes: &[Vec<usize>], title: &str) -> Block {
    let mut t = Table::new(Some(title), &["Vehículo", "Recorrido", "Carga", "Distancia"]).left(1);
    for (k, r) in routes.iter().enumerate() {
        t.row(cells![k + 1, inst.route_names(r), inst.load(r), inst.rlen(r)]);
    }
    t.block()
}

pub fn sweep(inst: &Inst, start: f64, dir: Direction, improve: bool, show: bool) -> Result<Sol, String> {
    if !inst.real_xy {
        return Err("el barrido necesita las coordenadas de los puntos".into());
    }
    let rel = |i: usize| {
        let a = inst.angle(i);
        match dir {
            Direction::Counterclockwise => (a - start).rem_euclid(360.0),
            Direction::Clockwise => (start - a).rem_euclid(360.0),
        }
    };
    let mut order = inst.customers();
    order.sort_by(|&a, &b| rel(a).total_cmp(&rel(b)).then(inst.d[0][a].total_cmp(&inst.d[0][b])));
    let mut clusters: Vec<Vec<usize>> = vec![vec![]];
    let mut loads = vec![0.0];
    let mut t = Table::new(Some("Clientes en el orden del barrido"), &["Cliente", "Ángulo", "Demanda", "Carga del vehículo", "Vehículo", "Nota"]).left(0).left(5);
    let mut rays = vec![];
    let mut prev: Option<usize> = None;
    for &i in &order {
        let k = clusters.len() - 1;
        let mut note = String::new();
        let fits_q = loads[k] + inst.q[i] <= inst.cap + 1e-9;
        let fits_l = fits_q && {
            let mut c = clusters[k].clone();
            c.push(i);
            inst.fits_len(&best_order(inst, &c))
        };
        if !clusters[k].is_empty() && !(fits_q && fits_l) {
            note = if !fits_q {
                format!("No cabe ({} + {} > {}): sale otro vehículo", fmt(loads[k]), fmt(inst.q[i]), fmt(inst.cap))
            } else {
                "La ruta pasaría la distancia máxima: sale otro vehículo".into()
            };
            // el corte va a mitad de camino entre el último cliente de la zona y este
            let a = inst.angle(i);
            let p = prev.map(|p| inst.angle(p)).unwrap_or(a);
            rays.push(match dir {
                Direction::Counterclockwise => p + (a - p).rem_euclid(360.0) / 2.0,
                Direction::Clockwise => p - (p - a).rem_euclid(360.0) / 2.0,
            });
            clusters.push(vec![]);
            loads.push(0.0);
        }
        let k = clusters.len() - 1;
        clusters[k].push(i);
        loads[k] += inst.q[i];
        t.row_tone(cells![&inst.names[i], (inst.angle(i) * 10.0).round() / 10.0, inst.q[i], loads[k], k + 1, note], if k % 2 == 0 { Tone::Normal } else { Tone::Accent });
        prev = Some(i);
    }
    let mut routes: Vec<Vec<usize>> = clusters.iter().map(|c| best_order(inst, c)).collect();
    let mut steps = vec![];
    if show {
        steps.push(step(
            "1. Ordenar por ángulo",
            format!(
                "Se mide el ángulo de cada cliente visto desde {} ({}) y se barre {} desde {}°. Cada vehículo se llena en ese orden hasta que el siguiente cliente no cabe en su capacidad de {}.",
                inst.names[0],
                if inst.geo.is_some() { "0° = al este, 90° = al norte" } else { "0° = a la derecha, 90° = arriba" },
                match dir {
                    Direction::Counterclockwise => "en sentido contrario al reloj",
                    Direction::Clockwise => "en el sentido del reloj",
                },
                fmt(start),
                fmt(inst.cap)
            ),
            vec![t.block()],
        ));
        steps.push(step(
            "2. Recorrer cada zona",
            "Con los clientes de cada vehículo ya decididos, se busca el orden de visita más corto de cada zona (exacto con programación dinámica cuando la zona tiene hasta 12 clientes).",
            vec![routes_table(inst, &routes, "Rutas de cada zona")],
        ));
    }
    if improve {
        improve_all(inst, &mut routes, &mut steps, show);
    }
    Ok(Sol { method: Method::Sweep, label: "Barrido", routes, steps, rays, start_angle: Some(start), note: None, proven: false })
}

pub fn sweep_best(inst: &Inst, dir: Direction, improve: bool, show: bool) -> Result<Sol, String> {
    let mut best: Option<(usize, f64, f64)> = None;
    let mut tried = Table::new(Some("Inicios probados"), &["Primer cliente", "Ángulo inicial", "Vehículos", "Distancia total"]).left(0);
    for c in inst.customers() {
        let a = inst.angle(c);
        let st = match dir {
            Direction::Counterclockwise => a - 1e-7,
            Direction::Clockwise => a + 1e-7,
        };
        let s = sweep(inst, st, dir, improve, false)?;
        let (nv, tot) = (s.routes.len(), s.total(inst));
        tried.row(cells![&inst.names[c], (a * 10.0).round() / 10.0, nv, tot]);
        if best.is_none_or(|b| (nv, tot) < (b.0, b.1 - 1e-9)) {
            best = Some((nv, tot, st));
        }
    }
    let (_, _, st) = best.unwrap();
    let mut s = sweep(inst, st, dir, improve, show)?;
    if show {
        s.steps.insert(0, step("0. Elegir dónde empezar", "Se prueba empezar el barrido en cada cliente y se elige el inicio con menos vehículos y, entre esos, la menor distancia.", vec![tried.block()]));
    }
    Ok(s)
}

pub fn savings(inst: &Inst, improve: bool, show: bool) -> Sol {
    let n = inst.n;
    let mut routes: Vec<Vec<usize>> = (1..n).map(|i| vec![i]).collect();
    let mut of: Vec<usize> = (0..n).map(|i| i.saturating_sub(1)).collect();
    let mut alive = vec![true; routes.len()];
    let mut list = vec![];
    for i in 1..n {
        for j in 1..n {
            if i == j || (inst.symmetric && j < i) {
                continue;
            }
            let s = inst.d[i][0] + inst.d[0][j] - inst.d[i][j];
            list.push((s, i, j));
        }
    }
    list.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)).then(a.2.cmp(&b.2)));
    let mut t = Table::new(Some("Ahorros en orden"), &["Par", "Ahorro s(i,j)", "Decisión"]).left(0).left(2);
    let mut shown = 0;
    for &(s, i, j) in &list {
        if s <= EPS {
            break;
        }
        let (a, b) = (of[i], of[j]);
        let pair = format!("{} – {}", inst.names[i], inst.names[j]);
        let decision: String;
        let mut tone = Tone::Muted;
        if a == b {
            decision = "Ya están en la misma ruta".into();
        } else {
            let ra = &routes[a];
            let rb = &routes[b];
            let i_end = *ra.last().unwrap() == i;
            let i_start = ra[0] == i;
            let j_start = rb[0] == j;
            let j_end = *rb.last().unwrap() == j;
            let ok_shape = if inst.symmetric { (i_end || i_start) && (j_start || j_end) } else { i_end && j_start };
            if !ok_shape {
                decision = "Uno de los dos ya está en el medio de una ruta".into();
            } else if inst.load(ra) + inst.load(rb) > inst.cap + 1e-9 {
                decision = format!("No caben juntos ({} + {} > {})", fmt(inst.load(ra)), fmt(inst.load(rb)), fmt(inst.cap));
            } else {
                let mut x = ra.clone();
                if *x.last().unwrap() != i {
                    x.reverse();
                }
                let mut y = rb.clone();
                if y[0] != j {
                    y.reverse();
                }
                x.extend(y);
                if !inst.fits_len(&x) {
                    decision = "La ruta unida pasaría la distancia máxima".into();
                } else {
                    decision = format!("Se unen: {}", inst.route_names(&x));
                    tone = Tone::Accent;
                    for &c in &x {
                        of[c] = a;
                    }
                    routes[a] = x;
                    routes[b] = vec![];
                    alive[b] = false;
                }
            }
        }
        if shown < 80 {
            t.row_tone(cells![pair, s, decision], tone);
            shown += 1;
        }
    }
    let mut out: Vec<Vec<usize>> = routes.into_iter().zip(alive).filter(|(r, a)| *a && !r.is_empty()).map(|(r, _)| r).collect();
    let mut steps = vec![];
    if show {
        let mut st = Table::new(Some("Matriz de ahorros s(i,j) = d(i,0) + d(0,j) − d(i,j)"), &[""]);
        let small = n <= 16;
        if small {
            let mut cols = vec![String::new()];
            cols.extend((1..n).map(|j| inst.names[j].clone()));
            st = Table::new(Some("Matriz de ahorros s(i,j) = d(i,0) + d(0,j) − d(i,j)"), &cols).left(0);
            for i in 1..n {
                let mut row = vec![Cell::from(&inst.names[i])];
                for j in 1..n {
                    row.push(if i == j { Cell::from("") } else { Cell::from(clean(inst.d[i][0] + inst.d[0][j] - inst.d[i][j])) });
                }
                st.row(row);
            }
        }
        let mut b = vec![];
        if small {
            b.push(st.block());
        }
        steps.push(step(
            "1. Calcular los ahorros",
            format!(
                "Se empieza con un vehículo por cliente ({} → cliente → {}). Unir los clientes i y j en la misma ruta ahorra s(i,j) = d(i,{0}) + d({0},j) − d(i,j): ya no hay que volver al depósito entre ellos.",
                inst.names[0], inst.names[0]
            ),
            b,
        ));
        steps.push(step(
            "2. Unir rutas por el mayor ahorro",
            "Se recorren los ahorros de mayor a menor. Dos rutas se unen si i y j están en rutas distintas, los dos son extremos de su ruta y la carga total cabe en un vehículo.",
            vec![t.block()],
        ));
        steps.push(step("3. Rutas que quedan", "", vec![routes_table(inst, &out, "Rutas de los ahorros")]));
    }
    if improve {
        improve_all(inst, &mut out, &mut steps, show);
    }
    Sol { method: Method::Savings, label: "Ahorros (Clarke y Wright)", routes: out, steps, rays: vec![], start_angle: None, note: None, proven: false }
}

pub fn nearest(inst: &Inst, improve: bool, show: bool) -> Sol {
    let mut left = inst.customers();
    let mut routes = vec![];
    let mut t = Table::new(Some("Recorrido del vecino más cercano"), &["Vehículo", "Desde", "Va a", "Distancia", "Carga"]).left(1).left(2);
    while !left.is_empty() {
        let mut r: Vec<usize> = vec![];
        let mut cur = 0;
        let mut load = 0.0;
        loop {
            let cand = left
                .iter()
                .enumerate()
                .filter(|(_, &j)| load + inst.q[j] <= inst.cap + 1e-9 && {
                    let mut x = r.clone();
                    x.push(j);
                    inst.fits_len(&x)
                })
                .min_by(|a, b| inst.d[cur][*a.1].total_cmp(&inst.d[cur][*b.1]).then(a.1.cmp(b.1)));
            let Some((pos, &j)) = cand else { break };
            t.row(cells![routes.len() + 1, &inst.names[cur], &inst.names[j], inst.d[cur][j], load + inst.q[j]]);
            left.remove(pos);
            load += inst.q[j];
            r.push(j);
            cur = j;
        }
        if r.is_empty() {
            break;
        }
        t.row_tone(cells![routes.len() + 1, &inst.names[cur], &inst.names[0], inst.d[cur][0], load], Tone::Muted);
        routes.push(r);
    }
    let mut steps = vec![];
    if show {
        steps.push(step(
            "1. Ir siempre al cliente más cercano",
            format!(
                "Cada vehículo sale de {} y va al cliente más cercano que todavía cabe en su carga; cuando ya no cabe ninguno, vuelve al depósito y sale el siguiente vehículo.",
                inst.names[0]
            ),
            vec![t.block()],
        ));
    }
    let mut routes = routes;
    if improve {
        improve_all(inst, &mut routes, &mut steps, show);
    }
    Sol { method: Method::NearestNeighbor, label: "Vecino más cercano", routes, steps, rays: vec![], start_angle: None, note: None, proven: false }
}

/// Óptimo con programación entera (dos índices, cargas de Miller-Tucker-Zemlin).
pub fn exact(inst: &Inst, vehicles: Option<usize>, time_limit: f64, show: bool, warm: Option<&[Vec<usize>]>) -> Result<Sol, String> {
    let n = inst.n;
    let nc = n - 1;
    // sin límite de clientes: el motor arranca con la mejor heurística y, si se
    // acaba el tiempo, devuelve la mejor solución encontrada con su brecha
    let total_q: f64 = inst.q.iter().sum();
    let kmin = ((total_q / inst.cap) - 1e-9).ceil().max(1.0);
    let kmax = vehicles.map(|v| v as f64).unwrap_or(nc as f64);
    if kmin > kmax + 1e-9 {
        return Err(format!("con {} vehículos no alcanza la capacidad: la demanda total es {}", fmt(kmax), fmt(total_q)));
    }
    let mut lpm = Lp::new(0, false);
    lpm.time_limit = Some(time_limit.max(1.0));
    let mut x = vec![vec![usize::MAX; n]; n];
    for i in 0..n {
        for j in 0..n {
            if i != j {
                x[i][j] = lpm.add_var(inst.d[i][j], 0.0, 1.0, true);
            }
        }
    }
    let u: Vec<usize> = (0..n).map(|i| if i == 0 { usize::MAX } else { lpm.add_var(0.0, inst.q[i], inst.cap, false) }).collect();
    for j in 1..n {
        lpm.row((0..n).filter(|&i| i != j).map(|i| (x[i][j], 1.0)).collect(), Sense::Eq, 1.0);
        lpm.row((0..n).filter(|&k| k != j).map(|k| (x[j][k], 1.0)).collect(), Sense::Eq, 1.0);
    }
    let out0: Vec<(usize, f64)> = (1..n).map(|j| (x[0][j], 1.0)).collect();
    lpm.row(out0.clone(), Sense::Le, kmax);
    lpm.row(out0, Sense::Ge, kmin);
    for i in 1..n {
        for j in 1..n {
            if i != j {
                // u_j ≥ u_i + q_j − Q (1 − x_ij)
                lpm.row(vec![(u[i], 1.0), (u[j], -1.0), (x[i][j], inst.cap)], Sense::Le, inst.cap - inst.q[j]);
            }
        }
    }
    let mut t_vars: Option<Vec<usize>> = None;
    if let Some(l) = inst.lmax {
        let big = l + inst.d.iter().flatten().fold(0.0f64, |a, b| a.max(*b));
        let t: Vec<usize> = (0..n).map(|i| if i == 0 { usize::MAX } else { lpm.add_var(0.0, inst.d[0][i], l, false) }).collect();
        t_vars = Some(t.clone());
        for i in 1..n {
            for j in 1..n {
                if i != j {
                    lpm.row(vec![(t[i], 1.0), (t[j], -1.0), (x[i][j], big)], Sense::Le, big - inst.d[i][j]);
                }
            }
            lpm.row(vec![(t[i], 1.0), (x[i][0], big)], Sense::Le, l - inst.d[i][0] + big);
        }
    }
    // arranque en caliente con la mejor heurística: HiGHS empieza con una solución
    // factible y solo puede mejorarla (así nunca se queda sin respuesta por el tiempo)
    let mut warm_ok = false;
    if let Some(w) = warm.filter(|w| !w.is_empty() && (w.len() as f64) >= kmin - 1e-9 && (w.len() as f64) <= kmax + 1e-9) {
        let mut init = vec![0.0; lpm.n()];
        for r in w {
            let mut prev = 0;
            let mut load = 0.0;
            let mut dist = 0.0;
            for &j in r {
                init[x[prev][j]] = 1.0;
                load += inst.q[j];
                dist += inst.d[prev][j];
                init[u[j]] = load;
                if let Some(tv) = &t_vars {
                    init[tv[j]] = dist;
                }
                prev = j;
            }
            init[x[prev][0]] = 1.0;
        }
        lpm.initial = Some(init);
        warm_ok = true;
    }
    let t0 = Instant::now();
    let s = lp::solve(&lpm);
    let secs = t0.elapsed().as_secs_f64();
    if !s.is_optimal() {
        return Err("el programa entero no encontró una solución en el tiempo dado".into());
    }
    let mut routes = vec![];
    for j in 1..n {
        if s.x[x[0][j]] > 0.5 {
            let mut r = vec![j];
            let mut cur = j;
            for _ in 0..n {
                let nx = (0..n).find(|&k| k != cur && s.x[x[cur][k]] > 0.5).unwrap_or(0);
                if nx == 0 {
                    break;
                }
                r.push(nx);
                cur = nx;
            }
            routes.push(r);
        }
    }
    let proven = !s.limit_reached;
    let _ = warm_ok;
    let mut steps = vec![];
    if show {
        let nvars = n * (n - 1) + nc;
        steps.push(step(
            "Modelo de programación entera",
            format!(
                "x(i,j) = 1 si un vehículo va de i a j. Se minimiza la distancia total Σ d(i,j)·x(i,j). A cada cliente llega un vehículo y sale uno; del depósito salen entre {} y {} vehículos. La carga acumulada u(j) ≥ u(i) + q(j) − Q·(1 − x(i,j)), con q(j) ≤ u(j) ≤ Q, impide los subrecorridos y que un vehículo lleve más de Q (formulación de Miller-Tucker-Zemlin). Son {} variables ({} binarias) y {} restricciones. {} lo resolvió en {} s{}.",
                fmt(kmin),
                fmt(kmax),
                nvars,
                n * (n - 1),
                lpm.rows.len(),
                s.detail.as_ref().map(|d| d.solver.clone()).unwrap_or_else(|| "El motor".into()),
                fmt((secs * 100.0).round() / 100.0),
                if proven { "" } else { " (se alcanzó el tiempo máximo: es la mejor solución encontrada, sin prueba de optimalidad)" }
            ),
            vec![routes_table(inst, &routes, "Rutas óptimas")],
        ));
    }
    Ok(Sol {
        method: Method::Exact,
        label: if proven { "Óptimo (programación entera)" } else { "Programación entera (mejor encontrada)" },
        routes,
        steps,
        rays: vec![],
        start_angle: None,
        note: (!proven).then(|| "Se alcanzó el tiempo máximo: es la mejor solución encontrada, sin prueba de que sea óptima.".to_string()),
        proven,
    })
}

// ------------------------------------------------------------- informe

fn map_block(inst: &Inst, sol: &Sol, title: &str, rays: bool) -> Block {
    let mut points = vec![];
    for i in 0..inst.n {
        let route = sol.routes.iter().position(|r| r.contains(&i));
        // con coordenadas geográficas el mapa recibe longitud y latitud
        let (x, y) = inst.geo.as_ref().map(|g| g[i]).unwrap_or(inst.xy[i]);
        points.push(MapPoint {
            id: format!("p{i}"),
            label: inst.names[i].clone(),
            x,
            y,
            kind: if i == 0 { "depot".into() } else { "stop".into() },
            note: (i > 0 && inst.q[i] != 0.0).then(|| fmt(inst.q[i])),
            route,
        });
    }
    let routes = sol
        .routes
        .iter()
        .enumerate()
        .map(|(k, r)| {
            let mut stops = vec!["p0".to_string()];
            stops.extend(r.iter().map(|i| format!("p{i}")));
            stops.push("p0".into());
            MapRoute { name: format!("Vehículo {}", k + 1), stops, note: Some(format!("carga {} · {}", fmt(inst.load(r)), inst.dist_txt(inst.rlen(r)))), dashed: false }
        })
        .collect();
    let mut rs = vec![];
    if rays && inst.real_xy {
        if let Some(a) = sol.start_angle {
            rs.push(MapRay { from: "p0".into(), angle: a, label: Some("inicio".into()) });
        }
        // el corte entre zonas va entre el último cliente de una zona y el primero de la siguiente
        for &a in &sol.rays {
            rs.push(MapRay { from: "p0".into(), angle: a, label: None });
        }
    }
    RouteMap { title: Some(title.into()), points, routes, rays: rs, arrows: !inst.symmetric, geo: inst.geo.is_some() }.block()
}

fn loads_chart(inst: &Inst, sol: &Sol) -> Block {
    let cats: Vec<String> = (1..=sol.routes.len()).map(|k| format!("Vehículo {k}")).collect();
    let mut c = Chart::new(Some("Carga de cada vehículo"), "Vehículo", "Carga");
    c.categories = Some(cats);
    c.series.push(Series::new("Carga", SeriesKind::Bar, sol.routes.iter().enumerate().map(|(k, r)| [k as f64, inst.load(r)]).collect()).tone(Tone::Accent));
    c = c.hline(inst.cap, "Capacidad", Tone::Highlight);
    c.y_zero = true;
    c.block()
}

pub fn solve(m: &VrpModel, o: &VrpOptions) -> Result<Report, ModError> {
    let inst = build(m)?;
    let show = o.steps;
    let mut sols: Vec<Sol> = vec![];
    let mut skipped: Vec<(String, String)> = vec![];
    let run_sweep = |show: bool| if o.best_start { sweep_best(&inst, o.direction, o.improve, show) } else { sweep(&inst, o.start_angle, o.direction, o.improve, show) };
    match o.method {
        Method::Sweep => sols.push(run_sweep(show).map_err(ModError::Invalid)?),
        Method::Savings => sols.push(savings(&inst, o.improve, show)),
        Method::NearestNeighbor => sols.push(nearest(&inst, o.improve, show)),
        Method::Exact => {
            let a = savings(&inst, o.improve, false);
            let b = nearest(&inst, o.improve, false);
            let w = if a.total(&inst) <= b.total(&inst) { a.routes } else { b.routes };
            sols.push(exact(&inst, m.fleet(), o.time_limit, show, Some(&w)).map_err(ModError::Invalid)?)
        }
        Method::Compare => {
            match run_sweep(show) {
                Ok(s) => sols.push(s),
                Err(e) => skipped.push(("Barrido".into(), e)),
            }
            sols.push(savings(&inst, o.improve, show));
            sols.push(nearest(&inst, o.improve, show));
            if inst.n - 1 <= 15 {
                let fleet = m.fleet();
                let warm = sols.iter().filter(|s| fleet.is_none_or(|v| s.routes.len() <= v)).min_by(|a, b| a.total(&inst).total_cmp(&b.total(&inst))).map(|s| s.routes.clone());
                match exact(&inst, fleet, o.time_limit.min(15.0), show, warm.as_deref()) {
                    Ok(s) => sols.push(s),
                    Err(e) => skipped.push(("Óptimo".into(), e)),
                }
            } else {
                skipped.push(("Óptimo".into(), format!("con {} clientes tarda demasiado; elige el método Óptimo para intentarlo", inst.n - 1)));
            }
        }
    }
    let fleet = m.fleet();
    let within = |s: &Sol| fleet.is_none_or(|v| s.routes.len() <= v);
    let money = |dist: f64, veh: usize| -> Option<f64> {
        if m.cost_per_distance.is_none() && m.cost_per_vehicle.is_none() {
            return None;
        }
        Some(m.cost_per_distance.unwrap_or(0.0) * dist + m.cost_per_vehicle.unwrap_or(0.0) * veh as f64)
    };
    // con costos, gana el más barato; si no, la menor distancia
    let key = |s: &Sol| money(s.total(&inst), s.routes.len()).unwrap_or(s.total(&inst));
    let best = (0..sols.len())
        .min_by(|&a, &b| {
            let (sa, sb) = (&sols[a], &sols[b]);
            within(sb).cmp(&within(sa)).then(key(sa).total_cmp(&key(sb))).then(sa.routes.len().cmp(&sb.routes.len()))
        })
        .unwrap();
    let main = &sols[best];
    let total = main.total(&inst);
    let nveh = main.routes.len();
    let total_q: f64 = inst.q.iter().sum();
    let util = if nveh > 0 { total_q / (nveh as f64 * inst.cap) * 100.0 } else { 0.0 };

    let mut dist_metric = metric_main("Distancia total", clean(total));
    if inst.geo.is_some() {
        dist_metric.unit = Some("km".into());
    }
    let mut mets = vec![dist_metric, metric("Vehículos", nveh), metric("Uso de la capacidad", format!("{} %", fmt((util * 10.0).round() / 10.0)))];
    if let Some(c) = money(total, nveh) {
        mets.push(metric("Costo total", clean(c)));
    }
    mets.push(metric("Método", main.label));
    let mut sol_blocks = vec![metrics(mets)];
    if !within(main) {
        sol_blocks.push(text_tone(
            format!("Hacen falta {} vehículos y solo hay {}: ningún método encontró rutas con la flota disponible.", nveh, fleet.unwrap()),
            Tone::Bad,
        ));
    }
    if let Some(nt) = &main.note {
        sol_blocks.push(text_tone(nt.clone(), Tone::Highlight));
    }
    let dist_col = if inst.geo.is_some() { "Distancia (km)" } else { "Distancia" };
    let mut rt = Table::new(Some("Rutas"), &["Vehículo", "Recorrido", "Clientes", "Carga", "Uso", dist_col]).left(1);
    for (k, r) in main.routes.iter().enumerate() {
        rt.row(cells![k + 1, inst.route_names(r), r.len(), inst.load(r), format!("{} %", fmt((inst.load(r) / inst.cap * 1000.0).round() / 10.0)), clean(inst.rlen(r))]);
    }
    rt.row_tone(cells!["Total", "", inst.n - 1, total_q, "", clean(total)], Tone::Total);
    sol_blocks.push(rt.block());
    let streets: Vec<(usize, String)> = main.routes.iter().enumerate().filter_map(|(k, r)| inst.street_path(r).map(|p| (k, p))).collect();
    if !streets.is_empty() {
        let mut st = Table::new(Some("Recorrido por las calles"), &["Vehículo", "Pasa por"]).left(1);
        for (k, p) in streets {
            st.row(cells![k + 1, p]);
        }
        sol_blocks.push(st.note("Entre dos clientes sin calle directa, el vehículo va por la ruta más corta de la red.").block());
    }

    let mut rep = Report::new(
        "vrp",
        &m.name,
        if main.proven { Status::Optimal } else { Status::Feasible },
        format!(
            "{} vehículo{} {} {} en total{}. Método: {}.",
            nveh,
            if nveh == 1 { "" } else { "s" },
            if nveh == 1 { "recorre" } else { "recorren" },
            inst.dist_txt(total),
            money(total, nveh).map(|c| format!(", con un costo de {}", fmt((c * 100.0).round() / 100.0))).unwrap_or_default(),
            main.label.to_lowercase()
        ),
    )
    .section("solucion", "Solución", sol_blocks);

    let mut map = vec![map_block(&inst, main, &format!("Rutas · {}", main.label), main.method == Method::Sweep)];
    if !inst.real_xy {
        map.push(text("Los puntos no tienen coordenadas: el mapa los ubica en círculo alrededor del depósito. Escribe X y Y para verlos en su lugar."));
    }
    map.push(loads_chart(&inst, main));
    rep = rep.section("mapa", "Mapa", map);

    if sols.len() > 1 || !skipped.is_empty() {
        let with_cost = money(0.0, 0).is_some();
        let mut heads = vec!["Método", "Vehículos", if inst.geo.is_some() { "Distancia total (km)" } else { "Distancia total" }, "Diferencia con el mejor"];
        if with_cost {
            heads.push("Costo total");
        }
        let mut ct = Table::new(Some("Comparación de métodos"), &heads).left(0);
        let best_d = total;
        for (k, s) in sols.iter().enumerate() {
            let d = s.total(&inst);
            let mut row = cells![s.label, s.routes.len(), clean(d), if best_d > 0.0 { format!("+{} %", fmt(((d - best_d) / best_d * 1000.0).round() / 10.0)) } else { "0 %".into() }];
            if let Some(c) = money(d, s.routes.len()) {
                row.push(Cell::from(clean(c)));
            }
            ct.row_tone(row, if k == best { Tone::Accent } else { Tone::Normal });
        }
        for (l, why) in &skipped {
            ct.row_tone(cells![l.clone(), "", "", why.clone()], Tone::Muted);
        }
        let mut ch = Chart::new(Some("Distancia total por método"), "Método", dist_col);
        ch.categories = Some(sols.iter().map(|s| s.label.to_string()).collect());
        ch.series.push(Series::new("Distancia", SeriesKind::Bar, sols.iter().enumerate().map(|(k, s)| [k as f64, clean(s.total(&inst))]).collect()).tone(Tone::Accent));
        ch.y_zero = true;
        let mut blocks = vec![ct.block(), ch.block()];
        for s in &sols {
            if s.method != main.method {
                blocks.push(map_block(&inst, s, &format!("Rutas · {}", s.label), s.method == Method::Sweep));
            }
        }
        rep = rep.section("comparacion", "Comparación", blocks);
    }

    if show {
        let mut b = vec![];
        for s in &sols {
            if s.steps.is_empty() {
                continue;
            }
            b.push(Block::Steps { title: Some(s.label.into()), steps: s.steps.clone() });
        }
        rep = rep.section("pasos", "Paso a paso", b);
    }

    // matriz de distancias usada
    let mut dt_cols = vec![String::new()];
    dt_cols.extend(inst.names.iter().cloned());
    let mut dt = Table::new(Some(if inst.geo.is_some() { "Distancias entre puntos (km)" } else { "Distancias entre puntos" }), &dt_cols).left(0);
    for i in 0..inst.n {
        let mut row = vec![Cell::from(&inst.names[i])];
        row.extend((0..inst.n).map(|j| if i == j { Cell::from("") } else { Cell::from(clean(inst.d[i][j])) }));
        dt.row(row);
    }
    let note = match m.problem_type {
        VrpType::Coordinates => match m.metric {
            Metric::Euclidean => "Distancia en línea recta: √((x₁ − x₂)² + (y₁ − y₂)²).".to_string(),
            Metric::Rectilinear => "Distancia rectilínea (por cuadras): |x₁ − x₂| + |y₁ − y₂|.".to_string(),
            Metric::Haversine => "Distancia del gran círculo sobre la Tierra (fórmula del haversine), en km y redondeada al metro: d = 2R·asen(√(sen²(Δφ/2) + cos φ₁·cos φ₂·sen²(Δλ/2))), con φ la latitud, λ la longitud y R = 6371 km. Es la distancia en línea recta sobre el mapa; por las calles el recorrido real es más largo.".to_string(),
        },
        VrpType::Network => format!("Los {} pares sin calle directa se completaron con la ruta más corta de la red (Floyd-Warshall).", inst.filled),
    };
    let ang_cols: &[&str] =
        if inst.geo.is_some() { &["Cliente", "Latitud", "Longitud", "Ángulo", "Distancia al depósito (km)", "Demanda"] } else { &["Cliente", "X", "Y", "Ángulo", "Distancia al depósito", "Demanda"] };
    let mut ang = Table::new(Some("Ángulo de cada cliente desde el depósito"), ang_cols).left(0);
    if inst.real_xy {
        for i in 1..inst.n {
            let (a, b) = match &inst.geo {
                Some(g) => (g[i].1, g[i].0),
                None => inst.xy[i],
            };
            ang.row(cells![&inst.names[i], a, b, (inst.angle(i) * 10.0).round() / 10.0, (inst.d[0][i] * 100.0).round() / 100.0, inst.q[i]]);
        }
        if inst.geo.is_some() {
            ang = ang.note("Ángulo medido en el plano del mapa: 0° = al este, 90° = al norte (la longitud se escala por el coseno de la latitud media).");
        }
    }
    let mut datos = vec![dt.note(note).block()];
    if inst.real_xy {
        datos.push(ang.block());
    }
    rep = rep.section("distancias", "Distancias", datos);

    let routes_json: Vec<Value> = main
        .routes
        .iter()
        .map(|r| json!({"stops": r.iter().map(|&i| inst.names[i].clone()).collect::<Vec<_>>(), "load": inst.load(r), "distance": clean(inst.rlen(r))}))
        .collect();
    let comparison: Vec<Value> = sols.iter().map(|s| json!({"method": s.label, "vehicles": s.routes.len(), "distance": clean(s.total(&inst))})).collect();
    let mut data = json!({"method": main.label, "total_distance": clean(total), "vehicles": nveh, "routes": routes_json, "comparison": comparison});
    if inst.geo.is_some() {
        data["distance_unit"] = json!("km");
    }
    Ok(rep.data(data))
}
