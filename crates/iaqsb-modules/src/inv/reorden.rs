//! Punto de reorden con demanda variable (demanda diaria normal, tiempo de
//! entrega constante).
//!
//! - Sistema continuo: R = d·L + Z·σ·√L; inventario de seguridad SS = Z·σ·√L.
//! - Sistema periódico: inventario objetivo I = d(T + L) + Z·σ·√(T + L) y, al
//!   revisar con e unidades, se pide Q = I − e.

use super::{mean_sd, values, z_value, InvModel, InvOptions};
use iaqsb_core::*;
use serde_json::json;

pub fn solve(m: &InvModel, o: &InvOptions) -> Result<Report, ModError> {
    let hist = values(&m.history);
    let (hm, hs) = if hist.len() >= 2 { mean_sd(&hist) } else { (f64::NAN, f64::NAN) };
    let d = m.mean_demand.or((!hm.is_nan()).then_some(hm)).ok_or_else(|| ModError::Invalid("falta la demanda promedio diaria (o un histórico)".into()))?;
    let s = m.std_demand.or((!hs.is_nan()).then_some(hs)).ok_or_else(|| ModError::Invalid("falta la desviación estándar diaria (o un histórico)".into()))?;
    let l = m.lead_time.ok_or_else(|| ModError::Invalid("falta el tiempo de entrega".into()))?;
    let ns = m.service_level.ok_or_else(|| ModError::Invalid("falta el nivel de servicio".into()))?;
    if d < 0.0 || s < 0.0 || l < 0.0 {
        return invalid("la demanda, la desviación y el tiempo de entrega no pueden ser negativos");
    }
    let periodic = m.review.as_deref() == Some("periodico");
    let t = if periodic { m.review_days.ok_or_else(|| ModError::Invalid("falta el período de revisión T".into()))? } else { 0.0 };
    let span = t + l;
    let z = z_value(ns, &o.z_mode)?;
    let z_exact = z_value(ns, "exacto")?;
    let z_tab = z_value(ns, "tabla")?;
    let mean_l = d * span;
    let sd_l = s * span.sqrt();
    let ss = z * sd_l;
    let r = mean_l + ss;
    let alt = mean_l + if o.z_mode == "tabla" { z_exact } else { z_tab } * sd_l;
    let what = if periodic { "Inventario objetivo (I)" } else { "Punto de reorden (R)" };
    let mut ms = vec![metric_main(what, tidy(r)), metric("Inventario de seguridad", tidy(ss)), metric("Valor Z", tidy(z)), metric(if periodic { "Demanda en T + L" } else { "Demanda en el tiempo de entrega" }, tidy(mean_l))];
    let mut q_order = None;
    if periodic {
        if let Some(e) = m.on_hand {
            let q = (r - e).max(0.0);
            q_order = Some(q);
            ms.push(metric("Cantidad a pedir Q = I − e", tidy(q)));
        }
    }
    let mut t1 = Table::new(Some("Cálculo"), &["Concepto", "Fórmula", "Valor"]).left(0).left(1);
    if hist.len() >= 2 && (m.mean_demand.is_none() || m.std_demand.is_none()) {
        t1.row(cells!["Demanda promedio (histórico)", format!("promedio de {} datos", hist.len()), tidy(hm)]);
        t1.row(cells!["Desviación estándar (histórico)", "desviación muestral (n − 1)", tidy(hs)]);
    }
    t1.row(cells!["Valor Z", format!("nivel de servicio {} %{}", fmt(ns), if o.z_mode == "tabla" { " (tabla, dos decimales)" } else { "" }), tidy(z)]);
    let lbl = if periodic { "T + L" } else { "L" };
    t1.row(cells![format!("Demanda durante {lbl}"), format!("d·({lbl}) = {}·{}", fmt(tidy(d)), fmt(tidy(span))), tidy(mean_l)]);
    t1.row(cells![format!("Desviación durante {lbl}"), format!("σ·√({lbl}) = {}·√{}", fmt(tidy(s)), fmt(tidy(span))), tidy(sd_l)]);
    t1.row(cells!["Inventario de seguridad", "Z·σ·√(…)", tidy(ss)]);
    t1.row_tone(cells![what, "demanda + inventario de seguridad", tidy(r)], Tone::Highlight);
    if let Some(q) = q_order {
        t1.row(cells!["Cantidad a pedir", format!("I − e = {} − {}", fmt(tidy(r)), fmt(tidy(m.on_hand.unwrap()))), tidy(q)]);
    }
    let mut sens = Table::new(Some("Según el nivel de servicio"), &["Nivel de servicio (%)", "Z", "Inventario de seguridad", what]);
    for lv in [50.0, 75.0, 80.0, 85.0, 90.0, 95.0, 97.5, 98.0, 99.0, 99.5, 99.9] {
        let zz = z_value(lv, &o.z_mode)?;
        sens.row_tone(cells![lv, tidy(zz), tidy(zz * sd_l), tidy(mean_l + zz * sd_l)], if (lv - ns).abs() < 1e-9 { Tone::Highlight } else { Tone::Normal });
    }
    // curva normal de la demanda en el período de riesgo
    let mut charts = vec![];
    if sd_l > 0.0 {
        use statrs::distribution::{Continuous, Normal};
        let nd = Normal::new(mean_l, sd_l).unwrap();
        let xs: Vec<f64> = (0..=200).map(|k| mean_l - 4.0 * sd_l + 8.0 * sd_l * k as f64 / 200.0).collect();
        let pts: Vec<[f64; 2]> = xs.iter().map(|&x| [x, nd.pdf(x)]).collect();
        let tail: Vec<[f64; 2]> = xs.iter().filter(|&&x| x >= r).map(|&x| [x, nd.pdf(x)]).collect();
        let mut ch = Chart::new(Some(&format!("Demanda durante {lbl}: probabilidad de faltante = {} %", fmt(tidy(100.0 - ns)))), "Unidades", "Densidad")
            .series(Series::new("Demanda", SeriesKind::Line, pts).tone(Tone::Highlight))
            .series(Series::new("Faltante", SeriesKind::Area, tail).tone(Tone::Bad));
        ch = ch.vline(mean_l, format!("media {}", fmt(tidy(mean_l))), Tone::Muted).vline(r, format!("{} = {}", if periodic { "I" } else { "R" }, fmt(tidy(r))), Tone::Accent);
        ch.y_zero = true;
        charts.push(ch.block());
        // perfil del sistema continuo (como la diapositiva): diente de sierra con R
        if !periodic && d > 0.0 {
            let q = (d * 30.0).max(ss + d * l);
            let top = r + q - d * l;
            let cyc = (top - r) / d + l;
            let mut p = vec![];
            for k in 0..3 {
                let t0 = k as f64 * cyc;
                p.push([t0, top]);
                p.push([t0 + (top - r) / d, r]);
                p.push([t0 + cyc, ss]);
                p.push([t0 + cyc, ss + q]);
            }
            let mut c2 = Chart::new(Some("Sistema continuo (demanda promedio): se pide al llegar a R y el pedido llega L días después"), "Días", "Inventario").series(Series::new("Inventario", SeriesKind::Line, p).tone(Tone::Highlight));
            c2 = c2.hline(r, "R", Tone::Accent).hline(ss, "SS", Tone::Muted);
            c2.y_zero = true;
            charts.push(c2.block());
        }
    }
    let mut blocks = vec![metrics(ms), t1.clone().block()];
    blocks.push(text(format!(
        "Con Z {} = {} el resultado sería {}. La variabilidad de {} días no es {}·σ sino σ·√{} porque los días buenos y malos se compensan (las varianzas se suman).",
        if o.z_mode == "tabla" { "exacto" } else { "de la tabla" },
        fmt(tidy(if o.z_mode == "tabla" { z_exact } else { z_tab })),
        fmt(tidy(alt)),
        fmt(tidy(span)),
        fmt(tidy(span)),
        fmt(tidy(span))
    )));
    blocks.push(sens.block());
    let mut st = vec![];
    if o.steps {
        st.push(steps(vec![
            step("Valor Z", format!("Para {} % se busca el área {} en la tabla normal (o la inversa de la normal): Z = {}.", fmt(ns), fmt(tidy(ns / 100.0 - 0.5)), fmt(tidy(z))), vec![]),
            step("Demanda y variabilidad en el período de riesgo", format!("Media = d·{lbl} = {}. Desviación = σ·√({lbl}) = {}.", fmt(tidy(mean_l)), fmt(tidy(sd_l))), vec![]),
            step(
                if periodic { "Inventario objetivo" } else { "Punto de reorden" },
                format!("{} = {} + {}·{} = {} (inventario de seguridad {}).", if periodic { "I" } else { "R" }, fmt(tidy(mean_l)), fmt(tidy(z)), fmt(tidy(sd_l)), fmt(tidy(r)), fmt(tidy(ss))),
                vec![t1.block()],
            ),
        ]));
    }
    let head = format!("{} = {} unidades (inventario de seguridad {}, Z = {}).", if periodic { "Inventario objetivo I" } else { "Punto de reorden R" }, fmt(tidy(r)), fmt(tidy(ss)), fmt(tidy(z)));
    Ok(Report::new("inv", &m.name, Status::Solved, head)
        .section("solucion", "Solución", blocks)
        .section("graficos", "Gráficos", charts)
        .section("pasos", "Paso a paso", st)
        .data(json!({"z": tidy(z), "safety_stock": tidy(ss), "reorder_point": tidy(r), "order_quantity": q_order.map(tidy), "mean_demand": tidy(d), "std_demand": tidy(s)}))
        .cite("hadleywhitin1963", "Punto de reorden con demanda aleatoria durante el tiempo de entrega")
        .cite("silver2017", "Inventario de seguridad Z·σ·√L para demanda diaria normal e independiente, y sistema periódico con T + L")
        .cite("sarache2026inventarios", "Modelos con demanda variable: sistema continuo R = dL + Zσ√L y periódico (diapositivas 44–47)")
        .cite("statrs", "Valor Z (inversa de la distribución normal estándar)"))
}
