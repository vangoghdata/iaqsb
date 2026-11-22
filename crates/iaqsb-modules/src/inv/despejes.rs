//! Despejes del EOQ y del POQ: con los datos que se conocen se aplican, en
//! cadena, las relaciones del modelo hasta tener todo. Cada despeje queda
//! como un paso, y al final se comprueban todas las relaciones con los valores
//! encontrados.
//!
//! Relaciones (d = demanda diaria, p = tasa de entrega diaria, días = días del año):
//! d = D/días · N = D/Q · T = días/N · Q = d·T · H = i·C
//! POQ: p = (p − d) + d · Tp = Q/p · Imax = (p − d)·Tp · Imax = d·Td · T = Tp + Td
//! EOQ: Imax = Q
//! Lote óptimo: S·N = H·Imax/2 (costo de ordenar = costo de mantener).

use super::{InvModel, InvOptions};
use iaqsb_core::*;
use serde_json::json;
use std::collections::BTreeMap;

struct Rule {
    out: &'static str,
    ins: &'static [&'static str],
    f: fn(&dyn Fn(&str) -> f64) -> f64,
    txt: &'static str,
    poq_only: bool,
    eoq_only: bool,
    optimal_only: bool,
}

macro_rules! rule {
    ($out:expr, [$($i:expr),*], $txt:expr, $f:expr) => {
        Rule { out: $out, ins: &[$($i),*], f: $f, txt: $txt, poq_only: false, eoq_only: false, optimal_only: false }
    };
}

fn rules() -> Vec<Rule> {
    let mut v = vec![
        rule!("d", ["D", "días"], "d = D / días", |g| g("D") / g("días")),
        rule!("D", ["d", "días"], "D = d · días", |g| g("d") * g("días")),
        rule!("N", ["D", "Q"], "N = D / Q", |g| g("D") / g("Q")),
        rule!("Q", ["D", "N"], "Q = D / N", |g| g("D") / g("N")),
        rule!("D", ["N", "Q"], "D = N · Q", |g| g("N") * g("Q")),
        rule!("N", ["días", "T"], "N = días / T", |g| g("días") / g("T")),
        rule!("T", ["días", "N"], "T = días / N", |g| g("días") / g("N")),
        rule!("Q", ["d", "T"], "Q = d · T (en un ciclo se consume todo el lote)", |g| g("d") * g("T")),
        rule!("T", ["Q", "d"], "T = Q / d", |g| g("Q") / g("d")),
        rule!("d", ["Q", "T"], "d = Q / T", |g| g("Q") / g("T")),
        rule!("H", ["i", "C"], "H = i · C", |g| g("i") / 100.0 * g("C")),
    ];
    let mut poq = vec![
        rule!("p", ["g", "d"], "p = (p − d) + d", |g| g("g") + g("d")),
        rule!("g", ["p", "d"], "p − d = p − d", |g| g("p") - g("d")),
        rule!("d", ["p", "g"], "d = p − (p − d)", |g| g("p") - g("g")),
        rule!("Tp", ["Q", "p"], "Tp = Q / p", |g| g("Q") / g("p")),
        rule!("Q", ["p", "Tp"], "Q = p · Tp", |g| g("p") * g("Tp")),
        rule!("p", ["Q", "Tp"], "p = Q / Tp", |g| g("Q") / g("Tp")),
        rule!("Imax", ["g", "Tp"], "Imax = (p − d) · Tp", |g| g("g") * g("Tp")),
        rule!("g", ["Imax", "Tp"], "p − d = Imax / Tp", |g| g("Imax") / g("Tp")),
        rule!("Tp", ["Imax", "g"], "Tp = Imax / (p − d)", |g| g("Imax") / g("g")),
        rule!("Imax", ["d", "Td"], "Imax = d · Td (en la bajada se consume a ritmo d)", |g| g("d") * g("Td")),
        rule!("d", ["Imax", "Td"], "d = Imax / Td", |g| g("Imax") / g("Td")),
        rule!("Td", ["Imax", "d"], "Td = Imax / d", |g| g("Imax") / g("d")),
        rule!("T", ["Tp", "Td"], "T = Tp + Td", |g| g("Tp") + g("Td")),
        rule!("Tp", ["T", "Td"], "Tp = T − Td", |g| g("T") - g("Td")),
        rule!("Td", ["T", "Tp"], "Td = T − Tp", |g| g("T") - g("Tp")),
        rule!("Imax", ["Q", "d", "p"], "Imax = Q · (1 − d/p)", |g| g("Q") * (1.0 - g("d") / g("p"))),
        rule!("Q", ["Imax", "d", "p"], "Q = Imax / (1 − d/p)", |g| g("Imax") / (1.0 - g("d") / g("p"))),
    ];
    for r in poq.iter_mut() {
        r.poq_only = true;
    }
    v.extend(poq);
    let mut eoq = vec![rule!("Imax", ["Q"], "Imax = Q (el lote llega completo)", |g| g("Q")), rule!("Q", ["Imax"], "Q = Imax", |g| g("Imax"))];
    for r in eoq.iter_mut() {
        r.eoq_only = true;
    }
    v.extend(eoq);
    let mut opt = vec![
        rule!("Imax", ["S", "N", "H"], "Lote óptimo: S·N = H·Imax/2 → Imax = 2·S·N / H", |g| 2.0 * g("S") * g("N") / g("H")),
        rule!("N", ["S", "H", "Imax"], "Lote óptimo: S·N = H·Imax/2 → N = H·Imax / (2·S)", |g| g("H") * g("Imax") / (2.0 * g("S"))),
        rule!("S", ["N", "H", "Imax"], "Lote óptimo: S = H·Imax / (2·N)", |g| g("H") * g("Imax") / (2.0 * g("N"))),
        rule!("H", ["S", "N", "Imax"], "Lote óptimo: H = 2·S·N / Imax", |g| 2.0 * g("S") * g("N") / g("Imax")),
    ];
    for r in opt.iter_mut() {
        r.optimal_only = true;
    }
    v.extend(opt);
    v
}

const NAMES: &[(&str, &str)] = &[
    ("D", "Demanda anual (D)"),
    ("d", "Demanda diaria (d)"),
    ("p", "Tasa de entrega por día (p)"),
    ("g", "Crecimiento del inventario por día (p − d)"),
    ("Q", "Tamaño del lote (Q)"),
    ("Imax", "Inventario máximo (Imax)"),
    ("Tp", "Días de entregas (Tp)"),
    ("Td", "Días de agotamiento (Td)"),
    ("T", "Duración del ciclo en días (T)"),
    ("N", "Pedidos al año (N)"),
    ("S", "Costo de ordenar (S)"),
    ("H", "Costo de mantener por unidad-año (H)"),
    ("i", "Tasa de mantener (% anual)"),
    ("C", "Costo unitario (C)"),
    ("días", "Días del año"),
];

pub fn solve(m: &InvModel, o: &InvOptions) -> Result<Report, ModError> {
    let poq = m.model_kind.as_deref().unwrap_or("poq") != "eoq";
    let mut known: BTreeMap<&'static str, (f64, String)> = BTreeMap::new();
    let mut give = |k: &'static str, v: Option<f64>| {
        if let Some(x) = v {
            known.insert(k, (x, "dato".into()));
        }
    };
    give("días", Some(m.days_per_year));
    give("D", m.annual_demand);
    give("d", m.daily_demand);
    give("Q", m.quantity);
    give("T", m.cycle_days);
    give("N", m.orders_per_year);
    give("S", m.setup_cost);
    give("H", m.holding_cost);
    give("i", m.holding_rate);
    give("C", m.unit_cost);
    if poq {
        give("p", m.delivery_rate);
        give("g", m.growth_rate);
        give("Imax", m.max_inventory);
        give("Tp", m.delivery_days);
        give("Td", m.depletion_days);
    }
    if known.values().any(|(v, _)| !v.is_finite() || *v < 0.0) {
        return invalid("los datos no pueden ser negativos");
    }
    if m.days_per_year <= 0.0 {
        return invalid("los días del año deben ser positivos");
    }
    let rs = rules();
    let applicable = |r: &Rule| (!r.poq_only || poq) && (!r.eoq_only || !poq) && (!r.optimal_only || m.is_optimal);
    let mut steps_v = vec![];
    loop {
        let mut changed = false;
        for r in rs.iter().filter(|r| applicable(r)) {
            if known.contains_key(r.out) || !r.ins.iter().all(|k| known.contains_key(k)) {
                continue;
            }
            let get = |k: &str| known.get(k).map(|x| x.0).unwrap_or(f64::NAN);
            let x = (r.f)(&get);
            if !x.is_finite() {
                continue;
            }
            if x <= 0.0 {
                return invalid(format!("con estos datos {} da {} (no positivo): revisa los datos", r.txt, fmt(tidy(x))));
            }
            let subs: Vec<String> = r.ins.iter().map(|k| format!("{} = {}", k, fmt(tidy(get(k))))).collect();
            let desc = format!("{} con {} → {} = {}", r.txt, subs.join(", "), r.out, fmt(tidy(x)));
            steps_v.push((r.out, desc.clone()));
            known.insert(r.out, (x, r.txt.to_string()));
            changed = true;
            break;
        }
        if !changed {
            break;
        }
    }
    let g = |k: &str| known.get(k).map(|x| x.0);
    // comprobación: todas las relaciones cuyos datos se conocen
    let mut checks = Table::new(Some("Comprobación con todas las relaciones del modelo"), &["Relación", "Lado izquierdo", "Lado derecho", "¿Cumple?"]).left(0);
    let mut bad = 0;
    let mut seen = std::collections::HashSet::new();
    for r in rs.iter().filter(|r| applicable(r)) {
        let mut key: Vec<&str> = r.ins.to_vec();
        key.push(r.out);
        key.sort();
        if !seen.insert(key) {
            continue;
        }
        if let (Some(lhs), true) = (g(r.out), r.ins.iter().all(|k| known.contains_key(k))) {
            let get = |k: &str| known.get(k).map(|x| x.0).unwrap_or(f64::NAN);
            let rhs = (r.f)(&get);
            let ok = (lhs - rhs).abs() <= 1e-6 * (1.0 + lhs.abs());
            if !ok {
                bad += 1;
            }
            // cada relación se muestra una vez (las variantes despejadas son la misma)
            checks.row_tone(cells![r.txt, tidy(lhs), tidy(rhs), if ok { "Sí" } else { "No" }], if ok { Tone::Normal } else { Tone::Bad });
        }
    }
    // costos
    let imax = g("Imax");
    let mut costs = vec![];
    let mut ct = None;
    if let (Some(s), Some(n), Some(h), Some(im)) = (g("S"), g("N"), g("H"), imax) {
        let ord = s * n;
        let hold = h * im / 2.0;
        ct = Some(ord + hold);
        costs.push(format!(
            "Costo total anual: CT = S·N + H·Imax/2 = {}·{} + {}·{}/2 = {} + {} = {} (el inventario promedio es Imax/2 = {}{}).",
            fmt(tidy(s)),
            fmt(tidy(n)),
            fmt(tidy(h)),
            fmt(tidy(im)),
            fmt(tidy(ord)),
            fmt(tidy(hold)),
            fmt(tidy(ord + hold)),
            fmt(tidy(im / 2.0)),
            if poq { "; en el POQ no es Q/2 porque el inventario nunca llega a Q" } else { "" }
        ));
    }
    let mut qopt = None;
    if let (Some(s), Some(d_), Some(h)) = (g("S"), g("D"), g("H")) {
        let rho = if poq { g("d").zip(g("p")).map(|(d, p)| 1.0 - d / p) } else { Some(1.0) };
        if let Some(rho) = rho.filter(|r| *r > 0.0) {
            let q = (2.0 * s * d_ / (h * rho)).sqrt();
            let cto = s * d_ / q + h * q * rho / 2.0;
            qopt = Some((q, cto));
            costs.push(format!(
                "Para comparar: el lote óptimo con estos datos es Q* = √(2·S·D / (H{})) = {} con CT = {}{}.",
                if poq { "·(1 − d/p)" } else { "" },
                fmt(tidy(q)),
                fmt(tidy(cto)),
                match (g("Q"), ct) {
                    (Some(q0), Some(c0)) if (q0 - q).abs() > 1e-6 * q => format!("; el lote actual ({}) cuesta {} más al año", fmt(tidy(q0)), fmt(tidy(c0 - cto))),
                    _ => String::new(),
                }
            ));
        }
    }
    let mut vt = Table::new(Some("Datos y despejes"), &["Variable", "Valor", "De dónde sale"]).left(0).left(2);
    for (k, label) in NAMES {
        if !poq && matches!(*k, "p" | "g" | "Tp" | "Td") {
            continue;
        }
        if let Some((v, how)) = known.get(k) {
            vt.row_tone(cells![*label, tidy(*v), how], if how == "dato" { Tone::Normal } else { Tone::Highlight });
        } else {
            vt.row(cells![*label, Cell::from("—"), "No se puede despejar con estos datos"]);
        }
    }
    let missing: Vec<&str> = ["Q", "D", "T"].into_iter().filter(|k| !known.contains_key(k)).collect();
    let headline = {
        let mut parts = vec![];
        if let Some(q) = g("Q") {
            parts.push(format!("Q = {}", fmt(tidy(q))));
        }
        if poq {
            if let Some(i) = imax {
                parts.push(format!("Imax = {}", fmt(tidy(i))));
            }
        }
        if let Some(c) = ct {
            parts.push(format!("CT = {} por año", fmt(tidy(c))));
        }
        if parts.is_empty() {
            "No hay datos suficientes para despejar el lote.".to_string()
        } else {
            format!("{}.", parts.join(" · "))
        }
    };
    let status = if bad > 0 {
        Status::Infeasible
    } else if missing.is_empty() {
        Status::Solved
    } else {
        Status::Feasible
    };
    let mut blocks = vec![];
    let mut ms = vec![];
    if let Some(q) = g("Q") {
        ms.push(metric_main("Tamaño del lote (Q)", tidy(q)));
    }
    if let Some(i) = imax {
        ms.push(metric("Inventario máximo", tidy(i)));
    }
    if let Some(c) = ct {
        ms.push(metric("Costo total anual", tidy(c)));
    }
    if let Some(n) = g("N") {
        ms.push(metric("Pedidos al año", tidy(n)));
    }
    if let Some(t) = g("T") {
        ms.push(metric("Ciclo (días)", tidy(t)));
    }
    if !ms.is_empty() {
        blocks.push(metrics(ms));
    }
    if bad > 0 {
        blocks.push(text_tone(format!("Los datos no son coherentes: {bad} relación(es) no se cumplen (ver la comprobación). Revisa si algún dato se leyó mal."), Tone::Bad));
    } else if !missing.is_empty() {
        blocks.push(text_tone(format!("Faltan datos para despejar: {}. Agrega otro dato o marca si el lote es el óptimo.", missing.join(", ")), Tone::Highlight));
    }
    blocks.push(vt.block());
    for c in &costs {
        blocks.push(text(c.clone()));
    }
    blocks.push(checks.clone().block());
    // gráfica del perfil
    let mut charts = vec![];
    if let (Some(q), Some(d_), Some(t)) = (g("Q"), g("d"), g("T")) {
        let mut pts = vec![];
        let (tp, im) = if poq { (g("Tp").unwrap_or(0.0), imax.unwrap_or(q)) } else { (0.0, q) };
        for k in 0..3 {
            let t0 = k as f64 * t;
            pts.push([t0, 0.0]);
            pts.push([t0 + tp, im]);
            pts.push([t0 + t, 0.0]);
        }
        let mut ch = Chart::new(Some("Perfil del inventario (3 ciclos)"), "Días", "Unidades en inventario").series(Series::new("Inventario", SeriesKind::Line, pts).tone(Tone::Highlight));
        ch = ch.hline(im, "Imax", Tone::Accent);
        if poq {
            ch = ch.vline(tp, format!("Tp = {}", fmt(tidy(tp))), Tone::Muted);
        }
        ch = ch.vline(t, format!("T = {}", fmt(tidy(t))), Tone::Muted);
        ch.y_zero = true;
        charts.push(ch.block());
        let _ = d_;
    }
    if let (Some(s), Some(d_), Some(h)) = (g("S"), g("D"), g("H")) {
        let rho = if poq { g("d").zip(g("p")).map(|(d, p)| 1.0 - d / p).unwrap_or(1.0) } else { 1.0 };
        if rho > 0.0 {
            let q0 = g("Q").unwrap_or((2.0 * s * d_ / (h * rho)).sqrt());
            let qs = (2.0 * s * d_ / (h * rho)).sqrt();
            let hi = q0.max(qs) * 1.6;
            let xs: Vec<f64> = (1..=120).map(|k| hi * k as f64 / 120.0).collect();
            let mk = |name: &str, f: &dyn Fn(f64) -> f64, tone: Tone| Series::new(name, SeriesKind::Line, xs.iter().map(|&q| [q, f(q)]).collect()).tone(tone);
            let mut ch = Chart::new(Some("Costos según el tamaño del lote"), "Q", "Costo por año")
                .series(mk("Ordenar S·D/Q", &|q| s * d_ / q, Tone::Normal))
                .series(mk("Mantener H·Imax/2", &|q| h * q * rho / 2.0, Tone::Normal))
                .series(mk("Total", &|q| s * d_ / q + h * q * rho / 2.0, Tone::Highlight));
            ch = ch.vline(qs, format!("Q* = {}", fmt(tidy(qs))), Tone::Accent);
            if let Some(q) = g("Q").filter(|q| (q - qs).abs() > 1e-6 * qs) {
                ch = ch.vline(q, format!("Q actual = {}", fmt(tidy(q))), Tone::Muted);
            }
            ch.y_zero = true;
            charts.push(ch.block());
        }
    }
    let mut st = vec![];
    if o.steps {
        let mut v = vec![step(
            "Datos",
            format!(
                "Modelo {}: {}. Se aplican las relaciones del modelo una por una; cada una despeja la única incógnita que le queda.",
                if poq { "POQ (el lote entra poco a poco mientras se consume: el inventario sube a ritmo p − d durante Tp y baja a ritmo d durante Td)" } else { "EOQ (el lote llega completo)" },
                known.iter().filter(|(_, v)| v.1 == "dato").map(|(k, v)| format!("{k} = {}", fmt(tidy(v.0)))).collect::<Vec<_>>().join(", ")
            ),
            vec![],
        )];
        for (k, (_, d)) in steps_v.iter().enumerate() {
            v.push(step(format!("Despeje {}", k + 1), d.clone(), vec![]));
        }
        if !costs.is_empty() {
            v.push(step("Costos", costs.join(" "), vec![]));
        }
        v.push(step("Comprobación", "Con todos los valores se verifican todas las relaciones del modelo (y no solo las que se usaron).", vec![checks.block()]));
        st.push(steps(v));
    }
    let data = json!({
        "values": known.iter().map(|(k, v)| (k.to_string(), json!(tidy(v.0)))).collect::<serde_json::Map<_, _>>(),
        "total_cost": ct.map(tidy),
        "optimal_quantity": qopt.map(|x| tidy(x.0)),
        "optimal_cost": qopt.map(|x| tidy(x.1)),
        "consistent": bad == 0,
    });
    let mut rep = Report::new("inv", &m.name, status, headline)
        .section("solucion", "Solución", blocks)
        .section("graficos", "Gráficos", charts)
        .section("pasos", "Paso a paso", st)
        .data(data)
        .cite("harris1913", "Modelo del lote económico: costo total S·D/Q + H·Q/2 y lote óptimo cuando ordenar y mantener cuestan lo mismo");
    if poq {
        rep = rep.cite("taft1918", "Lote económico de producción (entregas parciales): Imax = Q(1 − d/p) y Q* = √(2SD / (H(1 − d/p)))");
    }
    rep = rep.cite("sarache2026inventarios", "Modelo básico de aprovisionamiento y modelo POQ (diapositivas 24–31)");
    Ok(rep)
}
