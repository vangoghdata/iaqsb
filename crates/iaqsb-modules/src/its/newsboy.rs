//! Período único con demanda aleatoria (problema del vendedor de periódicos).

use super::{money, ItsModel, ItsOptions};
use crate::stat::{cdf, loss, quantile};
use iaqsb_core::*;
use serde_json::json;

pub fn solve(m: &ItsModel, o: &ItsOptions) -> Result<Report, ModError> {
    let law = m.demand_dist.law("demanda")?;
    let mu = law.mean();
    if !mu.is_finite() {
        return invalid("la demanda debe tener media finita");
    }
    let (r, c, s, v, k, i0) = (m.selling_price, m.unit_cost, m.opportunity_cost, m.salvage_value, m.setup_cost, m.initial_inventory);
    if [r, c, s, k, i0].iter().any(|x| *x < 0.0) {
        return invalid("los costos, el precio y el inventario inicial no pueden ser negativos");
    }
    if v >= c {
        return invalid(format!("el valor de rescate ({}) debe ser menor que el costo unitario ({}); si no, convendría pedir sin límite", fmt(v), fmt(c)));
    }
    let cu = r - c + s;
    let co = c - v;
    let cr = (cu / (cu + co)).clamp(0.0, 1.0);
    // G(y): ganancia esperada sin contar la compra del inventario inicial ni el costo fijo
    let g = |y: f64| {
        let n = loss(&law, y);
        r * (mu - n) + v * (y - mu + n) - s * n - c * y
    };
    let ystar = if cu <= 0.0 { f64::NEG_INFINITY } else { quantile(&law, cr) };
    let gstar = if ystar.is_finite() { g(ystar) } else { f64::NEG_INFINITY };
    let order = ystar.is_finite() && ystar > i0 && gstar - k > g(i0);
    let y = if order { ystar } else { i0 };
    let q = y - i0;
    let profit = |y: f64| g(y) + c * i0 - if y > i0 + 1e-12 { k } else { 0.0 };
    // punto de reorden: por debajo de s conviene pedir hasta y*
    let s_point = if ystar.is_finite() && k > 0.0 {
        let target = gstar - k;
        let (mut lo, mut hi) = (quantile(&law, 1e-6) - 10.0 * law.variance().sqrt() - 1.0, ystar);
        if g(lo) >= target {
            None
        } else {
            for _ in 0..200 {
                let mid = 0.5 * (lo + hi);
                if g(mid) < target {
                    lo = mid;
                } else {
                    hi = mid;
                }
            }
            Some(lo)
        }
    } else {
        Some(ystar)
    };
    let row = |label: &str, y: f64| {
        let n = loss(&law, y);
        vec![
            Cell::from(label),
            money(y - i0),
            money(y),
            Cell::Num(tidy(100.0 * cdf(&law, y))),
            money(n),
            money(y - mu + n),
            money(profit(y)),
        ]
    };
    let mut t = Table::new(Some("Análisis"), &["Política", "Cantidad a pedir", "Inventario disponible", "Nivel de servicio (%)", "Faltante esperado", "Sobrante esperado", "Ganancia esperada"]).left(0);
    t.row_tone(row("Óptima", y), Tone::Highlight);
    if let Some(kq) = m.known_quantity {
        t.row(row("Cantidad conocida", i0 + kq.max(0.0)));
    }
    if let Some(sl) = m.known_service {
        t.row(row(&format!("Servicio de {} %", fmt(sl)), quantile(&law, (sl / 100.0).clamp(0.0, 1.0)).max(i0)));
    }
    // ganancia esperada según la cantidad pedida
    let top = quantile(&law, 0.999).max(ystar.max(i0)) * 1.25 + 1.0;
    let pts: Vec<[f64; 2]> = (0..=200).map(|j| {
        let qq = (top - i0).max(1.0) * j as f64 / 200.0;
        [qq, profit(i0 + qq)]
    }).collect();
    let mut ch = Chart::new(Some("Ganancia esperada según la cantidad pedida"), "Cantidad a pedir", "Ganancia esperada").series(Series::new("Ganancia esperada", SeriesKind::Line, pts).tone(Tone::Highlight));
    ch = ch.vline(q, format!("óptimo {}", fmt(tidy(q))), Tone::Accent);
    let mut blocks = vec![
        metrics(vec![
            metric_main("Cantidad óptima a pedir", tidy(q)),
            metric("Inventario óptimo disponible", tidy(y)),
            metric("Razón crítica", tidy(cr)),
            metric("Ganancia esperada", tidy(profit(y))),
        ]),
        t.clone().block(),
    ];
    if k > 0.0 {
        if let Some(sp) = s_point {
            blocks.push(text(format!("Con costo fijo {}: si el inventario inicial es menor que s = {}, conviene pedir hasta S = {}; si no, no pedir.", fmt(k), fmt(tidy(sp)), fmt(tidy(ystar)))));
        }
    }
    if !order && ystar.is_finite() && ystar <= i0 {
        blocks.push(text("El inventario inicial ya alcanza el nivel óptimo: no conviene pedir."));
    }
    let mut st = vec![];
    if o.steps {
        st.push(steps(vec![
            step(
                "Costos marginales",
                format!(
                    "Faltar una unidad cuesta cᵤ = precio − costo + oportunidad = {} − {} + {} = {}. Que sobre una unidad cuesta cₒ = costo − rescate = {} − {} = {}.",
                    fmt(r),
                    fmt(c),
                    fmt(s),
                    fmt(cu),
                    fmt(c),
                    fmt(v),
                    fmt(co)
                ),
                vec![],
            ),
            step(
                "Razón crítica",
                format!("Conviene tener la cantidad y con P(D ≤ y) = cᵤ/(cᵤ + cₒ) = {}/({} + {}) = {}. Es el nivel de servicio óptimo.", fmt(cu), fmt(cu), fmt(co), fmt(tidy(cr))),
                vec![],
            ),
            step(
                "Cuantil de la demanda",
                if ystar.is_finite() { format!("y* = F⁻¹({}) = {}. Se pide y* − inventario inicial = {} − {} = {}.", fmt(tidy(cr)), fmt(tidy(ystar)), fmt(tidy(ystar)), fmt(i0), fmt(tidy((ystar - i0).max(0.0)))) } else { "El precio no cubre el costo: no conviene pedir.".into() },
                vec![],
            ),
            step(
                "Ganancia esperada",
                "E[ganancia] = precio·E[min(D, y)] + rescate·E[(y − D)⁺] − oportunidad·E[(D − y)⁺] − costo·(y − inventario inicial) − costo fijo. E[(D − y)⁺] es la función de pérdida de la demanda.",
                vec![t.block()],
            ),
        ]));
    }
    Ok(Report::new("its", &m.name, Status::Optimal, format!("Pedir {} unidades (nivel de servicio {} %); ganancia esperada {}.", fmt(tidy(q)), fmt(tidy(100.0 * cdf(&law, y))), fmt(tidy(profit(y)))))
        .section("solucion", "Solución", blocks)
        .section("graficos", "Gráficos", vec![ch.block()])
        .section("pasos", "Paso a paso", st)
        .data(json!({
            "order_quantity": tidy(q),
            "order_up_to": tidy(y),
            "critical_ratio": tidy(cr),
            "service_level": tidy(cdf(&law, y)),
            "expected_profit": tidy(profit(y)),
            "expected_shortage": tidy(loss(&law, y)),
            "reorder_point": s_point.map(tidy),
        })))
}
