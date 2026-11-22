//! Regresión lineal múltiple con inferencia.

use super::series::parse_list;
use super::{FcModel, FcOptions};
use iaqsb_core::*;
use nalgebra::{DMatrix, DVector};
use serde_json::json;
use statrs::distribution::{ContinuousCDF, FisherSnedecor, StudentsT};

pub fn solve(m: &FcModel, o: &FcOptions) -> Result<Report, ModError> {
    let v = m.variables.len();
    let n = m.observations.len();
    if v < 2 {
        return invalid("hacen falta al menos dos variables (una dependiente y una independiente)");
    }
    check_names("variables", &m.variables.iter().map(|x| &x.name).collect::<Vec<_>>())?;
    let names: Vec<&str> = m.variables.iter().map(|x| x.name.as_str()).collect();
    let dep = {
        let d = o.dependent.trim();
        if d.is_empty() {
            v - 1
        } else if let Some(j) = names.iter().position(|x| x.eq_ignore_ascii_case(d)) {
            j
        } else if let Ok(k) = d.parse::<usize>() {
            if k >= 1 && k <= v {
                k - 1
            } else {
                return invalid(format!("no hay variable número {k}"));
            }
        } else {
            return invalid(format!("no hay ninguna variable llamada «{d}»"));
        }
    };
    let ind: Vec<usize> = (0..v).filter(|j| *j != dep).collect();
    let k = ind.len();
    if n <= k + 1 {
        return invalid(format!("hacen falta más de {} observaciones para estimar {} coeficientes con inferencia", k + 1, k + 1));
    }
    let val = |i: usize, j: usize| m.values.get(i).and_then(|r| r.get(j)).copied().unwrap_or(0.0);
    let x = DMatrix::from_fn(n, k + 1, |i, j| if j == 0 { 1.0 } else { val(i, ind[j - 1]) });
    let y = DVector::from_fn(n, |i, _| val(i, dep));
    let xtx = x.transpose() * &x;
    let xty = x.transpose() * &y;
    let c = xtx.clone().try_inverse().ok_or_else(|| ModError::Invalid("las variables independientes son linealmente dependientes (o alguna es constante): no hay solución única".into()))?;
    // coeficientes por SVD (más estable que las ecuaciones normales) y C = (X'X)⁻¹ para los errores estándar
    let b = x.clone().svd(true, true).solve(&y, 1e-12).unwrap_or_else(|_| &c * &xty);
    let yhat = &x * &b;
    let e = &y - &yhat;
    let ym = y.mean();
    let sse: f64 = e.iter().map(|r| r * r).sum();
    let sst: f64 = y.iter().map(|t| (t - ym).powi(2)).sum();
    let ssr = (sst - sse).max(0.0);
    let df_e = (n - k - 1) as f64;
    let mse = sse / df_e;
    let se = mse.sqrt();
    let r2 = if sst > 0.0 { ssr / sst } else { 1.0 };
    let r2a = 1.0 - (1.0 - r2) * (n as f64 - 1.0) / df_e;
    let fval = if mse > 0.0 { (ssr / k as f64) / mse } else { f64::INFINITY };
    let fp = if fval.is_finite() { FisherSnedecor::new(k as f64, df_e).map(|d| 1.0 - d.cdf(fval)).unwrap_or(f64::NAN) } else { 0.0 };
    let tdist = StudentsT::new(0.0, 1.0, df_e).unwrap();
    let alpha = (o.significance / 100.0).clamp(1e-6, 0.999);
    let tcrit = tdist.inverse_cdf(1.0 - alpha / 2.0);
    let coef_names: Vec<String> = std::iter::once("Constante".to_string()).chain(ind.iter().map(|&j| names[j].to_string())).collect();
    let mut ct = Table::new(Some("Coeficientes"), &["Variable", "Coeficiente", "Error estándar", "t", "p", &format!("Límite inferior ({} %)", fmt(100.0 - o.significance)), "Límite superior"]).left(0);
    let mut coefs = vec![];
    for j in 0..=k {
        let sej = (mse * c[(j, j)]).max(0.0).sqrt();
        let t = if sej > 0.0 { b[j] / sej } else { f64::INFINITY };
        let p = if t.is_finite() { 2.0 * (1.0 - tdist.cdf(t.abs())) } else { 0.0 };
        ct.row_tone(cells![&coef_names[j], tidy(b[j]), tidy(sej), tidy(t), tidy(p), tidy(b[j] - tcrit * sej), tidy(b[j] + tcrit * sej)], if p < alpha { Tone::Normal } else { Tone::Muted });
        coefs.push(json!({"variable": coef_names[j], "coefficient": tidy(b[j]), "std_error": tidy(sej), "t": tidy(t), "p_value": tidy(p)}));
    }
    let eq = {
        let mut s = format!("{} = {}", names[dep], fmt(tidy(b[0])));
        for j in 1..=k {
            let cj = tidy(b[j]);
            s += &format!(" {} {}·{}", if cj < 0.0 { "−" } else { "+" }, fmt(cj.abs()), coef_names[j]);
        }
        s
    };
    let mut at = Table::new(Some("Análisis de varianza (ANOVA)"), &["Fuente", "Suma de cuadrados", "Grados de libertad", "Cuadrado medio", "F", "p"]).left(0);
    at.row(cells!["Regresión", tidy(ssr), k, tidy(ssr / k as f64), tidy(fval), tidy(fp)]);
    at.row(cells!["Error", tidy(sse), df_e, tidy(mse), Cell::Empty, Cell::Empty]);
    at.row_tone(cells!["Total", tidy(sst), n - 1, Cell::Empty, Cell::Empty, Cell::Empty], Tone::Total);
    // correlaciones
    let col = |j: usize| (0..n).map(|i| val(i, j)).collect::<Vec<f64>>();
    let corr = |a: &[f64], b: &[f64]| {
        let (ma, mb) = (a.iter().sum::<f64>() / n as f64, b.iter().sum::<f64>() / n as f64);
        let sab: f64 = a.iter().zip(b).map(|(x, y)| (x - ma) * (y - mb)).sum();
        let saa: f64 = a.iter().map(|x| (x - ma).powi(2)).sum();
        let sbb: f64 = b.iter().map(|y| (y - mb).powi(2)).sum();
        if saa > 0.0 && sbb > 0.0 { sab / (saa * sbb).sqrt() } else { f64::NAN }
    };
    let cols: Vec<Vec<f64>> = (0..v).map(col).collect();
    let mut cr = Table::new(Some("Correlaciones"), &std::iter::once("Variable".to_string()).chain(names.iter().map(|s| s.to_string())).collect::<Vec<_>>()).left(0);
    for a in 0..v {
        let mut row = vec![Cell::from(names[a])];
        row.extend((0..v).map(|b| Cell::Num(tidy(corr(&cols[a], &cols[b])))));
        cr.row(row);
    }
    // residuos
    let mut rt = Table::new(Some("Residuos"), &["Observación", "Real", "Estimado", "Residuo", "Residuo (%)", "Residuo estandarizado"]).left(0);
    for i in 0..n {
        let std = if se > 0.0 { e[i] / se } else { 0.0 };
        rt.row_tone(
            cells![&m.observations[i].name, y[i], tidy(yhat[i]), tidy(e[i]), if y[i] != 0.0 { Cell::Num(tidy(100.0 * e[i] / y[i])) } else { Cell::Empty }, tidy(std)],
            if std.abs() > 2.0 { Tone::Accent } else { Tone::Normal },
        );
    }
    // predicción
    let mut pred_blocks = vec![];
    let mut pred_data = serde_json::Value::Null;
    if !o.predict.trim().is_empty() {
        let xs = parse_list(&o.predict)?;
        if xs.len() != k {
            return invalid(format!("para predecir hacen falta {k} valores (uno por variable independiente: {})", coef_names[1..].join(", ")));
        }
        let x0 = DVector::from_fn(k + 1, |j, _| if j == 0 { 1.0 } else { xs[j - 1] });
        let yp = x0.dot(&b);
        let h0 = (x0.transpose() * &c * &x0)[(0, 0)];
        let sm = se * h0.max(0.0).sqrt();
        let sp = se * (1.0 + h0).max(0.0).sqrt();
        let mut pt = Table::new(Some(&format!("Predicción ({} % de confianza)", fmt(100.0 - o.significance))), &["Concepto", "Valor", "Límite inferior", "Límite superior"]).left(0);
        pt.row_tone(cells!["Predicción", tidy(yp), Cell::Empty, Cell::Empty], Tone::Highlight);
        pt.row(cells!["Intervalo de confianza de la media", Cell::Empty, tidy(yp - tcrit * sm), tidy(yp + tcrit * sm)]);
        pt.row(cells!["Intervalo de predicción (un valor)", Cell::Empty, tidy(yp - tcrit * sp), tidy(yp + tcrit * sp)]);
        pred_blocks.push(text(format!("Con {}", coef_names[1..].iter().zip(&xs).map(|(n, x)| format!("{n} = {}", fmt(*x))).collect::<Vec<_>>().join(", "))));
        pred_blocks.push(pt.block());
        pred_data = json!({"value": tidy(yp), "confidence": [tidy(yp - tcrit * sm), tidy(yp + tcrit * sm)], "prediction": [tidy(yp - tcrit * sp), tidy(yp + tcrit * sp)]});
    }
    // gráfico
    let chart = if k == 1 {
        let xs = &cols[ind[0]];
        let (lo, hi) = xs.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), x| (a.min(*x), b.max(*x)));
        Chart::new(Some("Datos y recta de regresión"), names[ind[0]], names[dep])
            .series(Series::new("Datos", SeriesKind::Scatter, (0..n).map(|i| [xs[i], y[i]]).collect()).tone(Tone::Accent))
            .series(Series::new("Recta", SeriesKind::Line, vec![[lo, b[0] + b[1] * lo], [hi, b[0] + b[1] * hi]]).tone(Tone::Highlight))
            .block()
    } else {
        let (lo, hi) = y.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), x| (a.min(*x), b.max(*x)));
        Chart::new(Some("Real contra estimado"), "Estimado", "Real")
            .series(Series::new("Observaciones", SeriesKind::Scatter, (0..n).map(|i| [yhat[i], y[i]]).collect()).tone(Tone::Accent))
            .series(Series::new("Ajuste perfecto", SeriesKind::Line, vec![[lo, lo], [hi, hi]]).tone(Tone::Muted).dashed())
            .block()
    };
    let res_chart = Chart::new(Some("Residuos"), "Observación", "Residuo").series(Series::new("Residuo", SeriesKind::Scatter, (0..n).map(|i| [i as f64 + 1.0, e[i]]).collect()).tone(Tone::Accent)).hline(0.0, "", Tone::Muted).block();
    let mut st = vec![];
    if o.steps {
        let mut v2 = vec![step(
            "Modelo",
            format!("{} = b₀ + {}+ error. Con la matriz X (una columna de unos y una por variable independiente), los coeficientes resuelven las ecuaciones normales X'X·b = X'y.", names[dep], coef_names[1..].iter().enumerate().map(|(j, nm)| format!("b{}·{} ", j + 1, nm)).collect::<String>()),
            vec![],
        )];
        if k <= 6 {
            let mut t1 = Table::new(Some("X'X"), &std::iter::once(String::new()).chain(coef_names.iter().cloned()).collect::<Vec<_>>()).left(0);
            for a in 0..=k {
                let mut row = vec![Cell::from(&coef_names[a])];
                row.extend((0..=k).map(|bb| Cell::Num(tidy(xtx[(a, bb)]))));
                t1.row(row);
            }
            let mut t2 = Table::new(Some("X'y"), &["", "Valor"]).left(0);
            for a in 0..=k {
                t2.row(cells![&coef_names[a], tidy(xty[a])]);
            }
            v2.push(step("Ecuaciones normales", format!("Resolviéndolas: {eq}."), vec![t1.block(), t2.block()]));
        }
        v2.push(step(
            "Bondad del ajuste",
            format!("SST = {} se divide en SSR = {} (explicada) y SSE = {} (error). R² = SSR/SST = {}; R² ajustado = {}; error estándar Se = √(SSE/(n − k − 1)) = {}.", fmt(tidy(sst)), fmt(tidy(ssr)), fmt(tidy(sse)), fmt(tidy(r2)), fmt(tidy(r2a)), fmt(tidy(se))),
            vec![at.clone().block()],
        ));
        v2.push(step(
            "Pruebas de significancia",
            format!("F = (SSR/k)/(SSE/(n − k − 1)) = {} con p = {}. Cada coeficiente se prueba con t = b/error estándar; con p < {} es significativo al {} %.", fmt(tidy(fval)), fmt(tidy(fp)), fmt(alpha), fmt(o.significance)),
            vec![ct.clone().block()],
        ));
        st.push(steps(v2));
    }
    let mut sol = vec![
        metrics(vec![metric_main("R²", tidy(r2)), metric("R² ajustado", tidy(r2a)), metric("Error estándar", tidy(se)), metric("F", tidy(fval)), metric("p de F", tidy(fp))]),
        text(format!("Ecuación: {eq}")),
        ct.block(),
        at.block(),
    ];
    sol.extend(pred_blocks.clone());
    let mut r = Report::new("fc", &m.name, Status::Solved, format!("{eq}; R² = {}.", fmt(tidy(r2))))
        .section("solucion", "Solución", sol)
        .section("grafico", "Gráfico", vec![chart, res_chart])
        .section("residuos", "Residuos", vec![rt.block()])
        .section("correlaciones", "Correlaciones", vec![cr.block()]);
    if !pred_blocks.is_empty() {
        r = r.section("prediccion", "Predicción", pred_blocks);
    }
    Ok(r.section("pasos", "Paso a paso", st).data(json!({
        "dependent": names[dep],
        "coefficients": coefs,
        "r_square": tidy(r2), "adjusted_r_square": tidy(r2a), "std_error": tidy(se), "f": tidy(fval), "f_p_value": tidy(fp),
        "sse": tidy(sse), "ssr": tidy(ssr), "sst": tidy(sst),
        "prediction": pred_data,
    })))
}
