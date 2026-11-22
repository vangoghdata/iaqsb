//! Clasificación ABC (Pareto).
//!
//! Valor de cada artículo = precio × cantidad; se ordena de mayor a menor, se
//! acumula el porcentaje y se corta en dos puntos:
//! - por quiebre de pendiente (como en clase): las dos mayores caídas relativas
//!   entre artículos consecutivos de la lista ordenada;
//! - por porcentaje acumulado: A hasta a_cut %, B hasta b_cut %.

use super::{InvModel, InvOptions};
use iaqsb_core::*;
use serde_json::json;

pub fn solve(m: &InvModel, o: &InvOptions) -> Result<Report, ModError> {
    let mut items = vec![];
    for (k, it) in m.items.iter().enumerate() {
        let name = if it.name.trim().is_empty() { (k + 1).to_string() } else { it.name.trim().to_string() };
        let v = match (it.price, it.quantity, it.value) {
            (Some(p), Some(q), _) => p * q,
            (_, _, Some(v)) => v,
            _ => return invalid(format!("al artículo {name} le falta el precio y la cantidad (o el valor)")),
        };
        if v < 0.0 {
            return invalid(format!("el valor del artículo {name} es negativo"));
        }
        items.push((name, it.price, it.quantity, v));
    }
    if items.len() < 3 {
        return invalid("se necesitan al menos 3 artículos para formar los grupos A, B y C");
    }
    let total: f64 = items.iter().map(|x| x.3).sum();
    if total <= 0.0 {
        return invalid("el valor total es cero");
    }
    let mut ord: Vec<usize> = (0..items.len()).collect();
    ord.sort_by(|&a, &b| items[b].3.partial_cmp(&items[a].3).unwrap().then(a.cmp(&b)));
    let pct: Vec<f64> = ord.iter().map(|&i| 100.0 * items[i].3 / total).collect();
    let mut acc = vec![];
    let mut s = 0.0;
    for p in &pct {
        s += p;
        acc.push(s);
    }
    let n = pct.len();
    // cortes: índice del último artículo de A y de B
    let (ea, eb, why) = if o.abc_method == "porcentajes" {
        if !(o.a_cut > 0.0 && o.a_cut < o.b_cut && o.b_cut < 100.0) {
            return invalid("los cortes deben cumplir 0 < A < B < 100");
        }
        let ea = acc.iter().position(|a| *a >= o.a_cut - 1e-9).unwrap_or(0);
        let eb = acc.iter().position(|a| *a >= o.b_cut - 1e-9).unwrap_or(n - 2).max(ea + 1).min(n - 2);
        (ea, eb, format!("A: hasta llegar al {} % acumulado; B: hasta el {} %; C: el resto.", fmt(o.a_cut), fmt(o.b_cut)))
    } else {
        // caída relativa entre el artículo k y el k+1 (sin dividir por cero)
        let mut drops: Vec<(f64, usize)> = (0..n - 1).map(|k| (if pct[k + 1] > 0.0 { pct[k] / pct[k + 1] } else { f64::INFINITY }, k)).collect();
        // el corte de C no puede ser el último par (C quedaría con un solo artículo... se permite) pero A necesita al menos uno
        drops.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap().then(a.1.cmp(&b.1)));
        let mut cuts: Vec<usize> = drops.iter().take(2).map(|d| d.1).collect();
        cuts.sort();
        if cuts.len() < 2 {
            return invalid("no hay suficientes artículos para dos cortes");
        }
        let txt = format!(
            "Se corta donde la participación cae más de un artículo al siguiente: después del artículo {} ({} % → {} %, cae {} veces) y después del {} ({} % → {} %, cae {} veces).",
            items[ord[cuts[0]]].0,
            fmt(tidy2(pct[cuts[0]])),
            fmt(tidy2(pct[cuts[0] + 1])),
            fmt(tidy2(pct[cuts[0]] / pct[cuts[0] + 1])),
            items[ord[cuts[1]]].0,
            fmt(tidy2(pct[cuts[1]])),
            fmt(tidy2(pct[cuts[1] + 1])),
            fmt(tidy2(pct[cuts[1]] / pct[cuts[1] + 1]))
        );
        (cuts[0], cuts[1], txt)
    };
    let group = |k: usize| if k <= ea { "A" } else if k <= eb { "B" } else { "C" };
    let mut t = Table::new(Some("Artículos ordenados por valor"), &["#", "Artículo", "Precio", "Cantidad", "Valor", "% del valor", "% acumulado", "Grupo"]).left(1);
    for (k, &i) in ord.iter().enumerate() {
        let it = &items[i];
        let tone = match group(k) {
            "A" => Tone::Highlight,
            "B" => Tone::Normal,
            _ => Tone::Muted,
        };
        t.row_tone(
            cells![k + 1, &it.0, it.1.map(Cell::Num).unwrap_or(Cell::from("—")), it.2.map(Cell::Num).unwrap_or(Cell::from("—")), tidy(it.3), tidy2(pct[k]), tidy2(acc[k]), group(k)],
            tone,
        );
    }
    let mut g = Table::new(Some("Resumen por grupo"), &["Grupo", "Artículos", "% de los artículos", "Valor", "% del valor", "Control sugerido"]).left(0).left(1).left(5);
    let mut summary = vec![];
    for (gname, lo, hi, ctrl) in [
        ("A", 0, ea, "Estricto: revisión continua, nivel de servicio alto, conteos frecuentes"),
        ("B", ea + 1, eb, "Intermedio"),
        ("C", eb + 1, n - 1, "Reglas simples, pero no se eliminan: sostienen el nivel de servicio"),
    ] {
        let ids: Vec<String> = (lo..=hi).map(|k| items[ord[k]].0.clone()).collect();
        let val: f64 = (lo..=hi).map(|k| items[ord[k]].3).sum();
        g.row(cells![gname, ids.join(", "), tidy2(100.0 * ids.len() as f64 / n as f64), tidy(val), tidy2(100.0 * val / total), ctrl]);
        summary.push(json!({"group": gname, "items": ids, "value": tidy(val), "value_pct": tidy2(100.0 * val / total)}));
    }
    let cats: Vec<String> = ord.iter().map(|&i| items[i].0.clone()).collect();
    let mut ch = Chart::new(Some("Diagrama de Pareto"), "Artículo (de mayor a menor valor)", "% del valor").stacked();
    ch.categories = Some(cats);
    for (gname, lo, hi, tone) in [("A", 0, ea, Tone::Highlight), ("B", ea + 1, eb, Tone::Normal), ("C", eb + 1, n - 1, Tone::Muted)] {
        ch = ch.series(Series::new(format!("Grupo {gname}"), SeriesKind::Bar, (lo..=hi).map(|k| [k as f64, tidy2(pct[k])]).collect()).tone(tone));
    }
    ch = ch.series(Series::new("% acumulado", SeriesKind::Line, (0..n).map(|k| [k as f64, tidy2(acc[k])]).collect()).tone(Tone::Accent));
    ch = ch.hline(acc[ea], "fin de A (% acumulado)", Tone::Muted).hline(acc[eb], "fin de B (% acumulado)", Tone::Muted);
    ch.y_zero = true;
    let head = format!(
        "A = {} ({} % del valor) · B = {} artículos · C = {} artículos.",
        (0..=ea).map(|k| items[ord[k]].0.clone()).collect::<Vec<_>>().join(", "),
        fmt(tidy2(acc[ea])),
        eb - ea,
        n - 1 - eb
    );
    let mut st = vec![];
    if o.steps {
        st.push(steps(vec![
            step("Valor de cada artículo", format!("Valor = precio × cantidad. Total = {}.", fmt(tidy(total))), vec![]),
            step("Ordenar y acumular", "Se ordenan de mayor a menor valor y se acumula el porcentaje de cada uno sobre el total.", vec![t.clone().block()]),
            step("Cortar los grupos", why.clone(), vec![g.clone().block()]),
        ]));
    }
    Ok(Report::new("inv", &m.name, Status::Solved, head)
        .section("solucion", "Solución", vec![metrics(vec![metric_main("Artículos A", (ea + 1) as f64), metric("Artículos B", (eb - ea) as f64), metric("Artículos C", (n - 1 - eb) as f64), metric("Valor total", tidy(total))]), text(why), g.block(), t.block()])
        .section("graficos", "Pareto", vec![ch.block()])
        .section("pasos", "Paso a paso", st)
        .data(json!({"total": tidy(total), "groups": summary, "order": ord.iter().map(|&i| items[i].0.clone()).collect::<Vec<_>>()}))
        .cite("dickie1951", "Clasificación ABC de los inventarios por su valor")
        .cite("silver2017", "Uso de la clasificación ABC para decidir el nivel de control de cada grupo")
        .cite("sarache2026inventarios", "Procedimiento ABC y gráfico de Pareto de la clase (diapositivas 38–43)"))
}

fn tidy2(x: f64) -> f64 {
    (x * 100.0).round() / 100.0
}
