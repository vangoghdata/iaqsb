//! PERT/Costo (presupuesto por periodo según inicios tempranos y tardíos),
//! informe de control de costos con valor ganado y análisis de avance a una fecha.

use super::cpm::{Net, Sched};
use super::PertModel;
use iaqsb_core::report::*;
use iaqsb_core::cells;
use serde_json::{json, Value};

/// Costo gastado por la actividad i hasta el instante x si empieza en `start`.
fn spent(cost: f64, start: f64, dur: f64, x: f64) -> f64 {
    if dur <= 1e-12 {
        if x >= start {
            cost
        } else {
            0.0
        }
    } else {
        cost * ((x - start) / dur).clamp(0.0, 1.0)
    }
}

pub fn pert_cost(m: &PertModel, net: &Net, s: &Sched, cost: &[f64]) -> Vec<Block> {
    let unit = m.unit_plural();
    let total = s.total;
    if total <= 0.0 {
        return vec![];
    }
    let periods = total.ceil() as usize;
    let width = if periods > 100 { (periods as f64 / 100.0).ceil() } else { 1.0 };
    let np = (total / width).ceil() as usize;
    let budget: f64 = cost.iter().sum();
    let mut t = Table::new(
        Some(&format!("Presupuesto por periodo ({} {})", fmt(width), if width == 1.0 { m.time_unit.clone() } else { unit.clone() })),
        &["Periodo", "Hasta", "Costo según IT (temprano)", "Costo según IT (tardío)", "Acumulado temprano", "Acumulado tardío"],
    );
    let mut ces = vec![[0.0, 0.0]];
    let mut cls = vec![[0.0, 0.0]];
    let mut prev_e = 0.0;
    let mut prev_l = 0.0;
    for k in 1..=np {
        let x = (k as f64 * width).min(total);
        let e: f64 = (0..net.n).map(|i| spent(cost[i], s.es[i], s.t[i], x)).sum();
        let l: f64 = (0..net.n).map(|i| spent(cost[i], s.ls[i], s.t[i], x)).sum();
        t.row(cells![k, x, e - prev_e, l - prev_l, e, l]);
        ces.push([x, e]);
        cls.push([x, l]);
        prev_e = e;
        prev_l = l;
    }
    let t = t.note("El costo de cada actividad se reparte por igual a lo largo de su duración. IT: inicio de las actividades.");
    let ch = Chart::new(Some("Presupuesto acumulado"), &format!("Tiempo ({unit})"), "Costo acumulado")
        .series(Series::new("Inicios tempranos", SeriesKind::Line, ces).tone(Tone::Accent))
        .series(Series::new("Inicios tardíos", SeriesKind::Line, cls).tone(Tone::Highlight))
        .hline(budget, "presupuesto total", Tone::Muted);
    vec![
        metrics(vec![metric_main("Costo total presupuestado", budget), metric("Duración", total)]),
        text("Cualquier programa factible gasta dentro de la banda entre las dos curvas: arriba, empezando todo lo antes posible; abajo, lo más tarde posible."),
        ch.block(),
        t.block(),
    ]
}

/// Informe de control de costos y análisis de avance a la fecha actual.
pub fn control(m: &PertModel, net: &Net, s: &Sched, now: Option<f64>) -> (Vec<Block>, Option<Value>) {
    let has_pct = m.has(|a| a.percent_complete);
    let has_actual = m.has(|a| a.actual_cost);
    if !has_pct && !has_actual {
        return (vec![], None);
    }
    let n = net.n;
    let unit = m.unit_plural();
    let pct: Vec<f64> = m.activities.iter().map(|a| a.percent_complete.unwrap_or(0.0)).collect();
    let bud: Vec<f64> = m.activities.iter().map(|a| a.normal_cost.unwrap_or(0.0)).collect();
    let act: Vec<f64> = m.activities.iter().map(|a| a.actual_cost.unwrap_or(0.0)).collect();
    let mut blocks = vec![];
    let mut data = json!({});

    // --- control de costos (valor ganado)
    let mut t = Table::new(
        Some("Informe de control de costos"),
        &["Actividad", "Tiempo", "Costo presupuestado", "% terminado", "Valor ganado", "Costo real", "Sobrecosto"],
    )
    .left(0);
    let (mut bt, mut ev, mut ac) = (0.0, 0.0, 0.0);
    for i in 0..n {
        let v = bud[i] * pct[i] / 100.0;
        let over = act[i] - v;
        let tone = if over > 1e-9 { Tone::Bad } else { Tone::Normal };
        t.row_tone(cells![&net.names[i], s.t[i], bud[i], pct[i], v, act[i], over], tone);
        bt += bud[i];
        ev += v;
        ac += act[i];
    }
    t.row_tone(cells!["Total", "", bt, if bt > 0.0 { Cell::Num(100.0 * ev / bt) } else { Cell::Empty }, ev, ac, ac - ev], Tone::Total);
    let t = t.note("Valor ganado = costo presupuestado × % terminado. Sobrecosto = costo real − valor ganado (en rojo, las actividades que gastan más de lo que avanzan).");
    let mut ms = vec![metric_main("Sobrecosto", ac - ev), metric("Valor ganado (EV)", ev), metric("Costo real (AC)", ac), metric("Presupuesto total (BAC)", bt)];
    let mut evm_text = vec![];
    if ac > 0.0 && ev > 0.0 {
        let cpi = ev / ac;
        ms.push(metric("Índice de costo (CPI = EV/AC)", cpi));
        ms.push(metric("Costo estimado al terminar (BAC/CPI)", bt / cpi));
        evm_text.push(format!(
            "Por cada unidad gastada se ha producido {} de valor; si sigue así, el proyecto costará {} en total.",
            fmt(cpi),
            fmt(bt / cpi)
        ));
    }
    if let Some(x) = now {
        let pv: f64 = (0..n).map(|i| spent(bud[i], s.es[i], s.t[i], x)).sum();
        ms.push(metric("Valor planeado (PV) a la fecha", pv));
        if pv > 0.0 {
            ms.push(metric("Índice de plazo (SPI = EV/PV)", ev / pv));
            evm_text.push(format!(
                "A la fecha {} se había planeado avanzar por {} (según inicios tempranos) y se ha avanzado por {}: {}.",
                fmt(x),
                fmt(pv),
                fmt(ev),
                if ev + 1e-9 >= pv { "va al día o adelantado" } else { "va por detrás del programa temprano" }
            ));
        }
        data["planned_value"] = json!(clean(pv));
    }
    data["earned_value"] = json!(clean(ev));
    data["actual_cost"] = json!(clean(ac));
    data["budget"] = json!(clean(bt));
    data["overrun"] = json!(clean(ac - ev));
    blocks.push(metrics(ms));
    if !evm_text.is_empty() {
        blocks.push(text(evm_text.join("\n")));
    }
    blocks.push(t.block());

    // --- análisis de avance a la fecha
    if let (Some(x), true) = (now, has_pct) {
        let planned = |start: f64, i: usize| -> f64 {
            if s.t[i] <= 1e-12 {
                if x >= start {
                    100.0
                } else {
                    0.0
                }
            } else {
                (100.0 * (x - start) / s.t[i]).clamp(0.0, 100.0)
            }
        };
        let mut a = Table::new(
            Some(&format!("Análisis de avance en {} {}", fmt(x), if x == 1.0 { m.time_unit.clone() } else { unit.clone() })),
            &["Actividad", "¿Crítica?", "Tiempo", "Inicio tardío", "Fin tardío", "% planeado (tardío)", "% planeado (temprano)", "% real", "Estado"],
        )
        .left(0)
        .left(1)
        .left(8);
        let mut late = vec![];
        let mut states = vec![];
        for i in 0..n {
            let pl = planned(s.ls[i], i);
            let pe = planned(s.es[i], i);
            let (st, tone) = if pct[i] >= 100.0 - 1e-9 {
                ("terminada", Tone::Good)
            } else if pct[i] + 1e-9 < pl {
                late.push(net.names[i].clone());
                ("atrasada", Tone::Bad)
            } else if pct[i] > pe + 1e-9 {
                ("adelantada", Tone::Good)
            } else {
                ("a tiempo", Tone::Normal)
            };
            states.push(st);
            a.row_tone(cells![&net.names[i], if s.critical[i] { "sí" } else { "no" }, s.t[i], s.ls[i], s.lf[i], pl, pe, pct[i], st], tone);
        }
        // fin previsto: lo que falta de cada actividad a partir de hoy
        let mut fin = vec![0.0; n];
        for &j in &net.order {
            let rem = s.t[j] * (1.0 - pct[j] / 100.0);
            if pct[j] >= 100.0 - 1e-9 {
                fin[j] = net.preds[j].iter().map(|&i| fin[i]).fold(0.0, f64::max).min(x);
                continue;
            }
            let ready = net.preds[j].iter().map(|&i| fin[i]).fold(x, f64::max);
            fin[j] = ready + rem;
        }
        let forecast = fin.iter().cloned().fold(x, f64::max);
        let status = if late.is_empty() {
            format!("El proyecto va a tiempo: ninguna actividad está por detrás de su programa más tardío. Fin previsto con lo que falta: {} {unit} (planeado {}).", fmt(forecast), fmt(s.total))
        } else {
            format!(
                "El proyecto va atrasado: {} {} por detrás de su programa más tardío. Si lo que falta se hace en el tiempo previsto, terminará en {} {unit} en lugar de {}.",
                late.join(", "),
                if late.len() == 1 { "está" } else { "están" },
                fmt(forecast),
                fmt(s.total)
            )
        };
        blocks.push(text_tone(status, if late.is_empty() { Tone::Good } else { Tone::Bad }));
        blocks.push(
            a.note("% planeado: lo que debería llevar hecho cada actividad si empezó en su inicio tardío (o temprano). Por detrás del programa tardío, el proyecto se retrasa.")
                .block(),
        );
        data["forecast_completion"] = json!(clean(forecast));
        data["late_activities"] = json!(late);
        data["states"] = json!((0..n).map(|i| json!({"name": net.names[i], "state": states[i]})).collect::<Vec<_>>());
    } else if has_pct {
        blocks.push(text("Indica la fecha actual en los parámetros del análisis para comparar el avance real con el planeado."));
    }
    (blocks, Some(data))
}
