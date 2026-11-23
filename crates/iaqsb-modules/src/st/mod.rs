//! ST · Series de tiempo y pronósticos modernos (no existe en WinQSB; el módulo
//! FC conserva los doce métodos de WinQSB).
//!
//! - **Exploración**: gráfico de la serie, autocorrelación y autocorrelación
//!   parcial con sus bandas, prueba de Ljung-Box, descomposición clásica,
//!   fuerza de la tendencia y de la estacionalidad, gráfico estacional y λ de
//!   Box-Cox sugerido (Hyndman y Athanasopoulos, 2018).
//! - **Pronóstico**: ARIMA (automático por el algoritmo de Hyndman-Khandakar o
//!   con los órdenes dados), ETS (automático o un modelo como MAM), TBATS (varias
//!   estacionalidades, también no enteras), Theta, Croston (demanda intermitente)
//!   e ingenuos, con intervalos de predicción, medidas de error, revisión de los
//!   residuos y, en modo automático, comparación de los modelos en un periodo de
//!   prueba como `accuracy()` (el mejor por MASE se vuelve a ajustar con toda la serie).
//! - **Evaluación** (como en clase con R): conjunto de entrenamiento hasta un
//!   periodo, varios modelos (ses, holt, hw, naive, snaive, tbats, auto.arima…),
//!   `accuracy()` en entrenamiento y prueba, `checkresiduals()` y el modelo de
//!   suavización que sugiere el correlograma.
//! - **Motor**: R con forecast si está instalado (la referencia); en la app de
//!   escritorio, si no hay R, el R que trae la app (webR, con forecast: los mismos
//!   resultados); si no, statsforecast de Python y, si no, el motor nativo en Rust.

pub mod analisis;
pub mod ets;
pub mod motores;

use analisis::*;
use iaqsb_core::*;
use motores::{Ajuste, Familia, Motor, Serie, Trabajo};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Obs {
    #[serde(default)]
    pub name: String,
    pub value: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct StModel {
    #[serde(default)]
    pub name: String,
    /// "exploracion", "pronostico" o "evaluacion".
    #[serde(default = "pron")]
    pub problem_type: String,
    #[serde(default = "mes")]
    pub time_unit: String,
    /// Periodo estacional (12 mensual, 4 trimestral, 7 diario semanal, 52.18 semanal; 1 = sin estacionalidad).
    #[serde(default = "uno")]
    pub frequency: f64,
    /// Estación del primer dato (1 = enero o primer trimestre).
    #[serde(default = "uno_u")]
    pub start_season: u32,
    pub data: Vec<Obs>,
    /// Primer periodo, p. ej. "2021-05" (mensual) o "2021-2" (trimestral): nombra los periodos
    /// sin nombre y fija la estación del primer dato, como ts(x, start = c(2021, 5)).
    #[serde(default)]
    pub start: String,
    /// Modelo: auto (comparar), ses, holt, hw, arima, ets, tbats, theta, croston, ingenuo, ingenuo_estacional.
    #[serde(default = "auto")]
    pub model: String,
    /// ARIMA: "p,d,q" o "p,d,q,P,D,Q" (vacío = automático).
    #[serde(default)]
    pub arima_order: String,
    #[serde(default)]
    pub drift: bool,
    /// ETS: "MAM", "AAdN"… (vacío = automático).
    #[serde(default)]
    pub ets_model: String,
    /// holt y hw: tendencia amortiguada.
    #[serde(default)]
    pub damped: bool,
    /// hw: "aditiva" o "multiplicativa".
    #[serde(default)]
    pub seasonal_type: String,
    /// Evaluación: modelos separados por comas (ses, holt, hw, naive, snaive, tbats, arima, ets, theta…);
    /// vacío = la suavización que sugiere el correlograma, el ingenuo que corresponda y TBATS.
    #[serde(default)]
    pub models: String,
    /// Evaluación: último periodo del entrenamiento (su nombre, p. ej. "2023-12"), como window(y, end = c(2023, 12)).
    #[serde(default)]
    pub train_end: String,
    /// TBATS: periodos estacionales separados por comas (vacío = la frecuencia).
    #[serde(default)]
    pub seasonal_periods: String,
    /// λ de Box-Cox: vacío (sin transformar), "auto" (Guerrero) o un número (0 = logaritmo).
    #[serde(default)]
    pub lambda: String,
    /// Periodos a pronosticar (0 = dos ciclos o 10).
    #[serde(default)]
    pub horizon: u32,
    /// Periodos de prueba para comparar modelos (0 = el horizonte, a lo sumo la quinta parte de la serie).
    #[serde(default)]
    pub test_size: u32,
    #[serde(default = "noventa_cinco")]
    pub level: f64,
}

fn pron() -> String {
    "pronostico".into()
}
fn mes() -> String {
    "mes".into()
}
fn uno() -> f64 {
    1.0
}
fn uno_u() -> u32 {
    1
}
fn auto() -> String {
    "auto".into()
}
fn noventa_cinco() -> f64 {
    95.0
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct StOptions {
    pub steps: bool,
    /// Motor de pronóstico: auto (el mejor disponible), r, python o nativo.
    /// (`options.engine` queda para los parámetros del optimizador, como en los otros módulos.)
    pub forecast_engine: String,
    /// Retardos del correlograma (0 = como Acf de R).
    pub lags: u32,
    /// Descomposición: aditiva o multiplicativa.
    pub decomposition: String,
}
impl Default for StOptions {
    fn default() -> Self {
        StOptions { steps: true, forecast_engine: "auto".into(), lags: 0, decomposition: "aditiva".into() }
    }
}

fn r4(x: f64) -> f64 {
    if x.is_finite() {
        tidy((x * 1e4).round() / 1e4)
    } else {
        x
    }
}

fn celda(x: f64) -> Cell {
    if x.is_finite() {
        Cell::from(r4(x))
    } else {
        Cell::from("—")
    }
}

struct Datos {
    y: Vec<f64>,
    nombres: Vec<String>,
    m: f64,
    mi: usize,
    inicio: usize,
}

fn datos(m: &StModel) -> Result<Datos, ModError> {
    let mut y = vec![];
    let mut nombres = vec![];
    for (k, o) in m.data.iter().enumerate() {
        match o.value {
            Some(v) if v.is_finite() => {
                y.push(v);
                nombres.push(if o.name.trim().is_empty() { (k + 1).to_string() } else { o.name.trim().to_string() });
            }
            _ => return invalid(format!("falta el valor del periodo {} (la serie no puede tener huecos)", k + 1)),
        }
    }
    if y.len() < 4 {
        return invalid("la serie necesita al menos 4 datos");
    }
    if !(m.frequency >= 1.0) {
        return invalid("la frecuencia debe ser 1 o más (12 mensual, 4 trimestral…)");
    }
    let mi = m.frequency.round() as usize;
    let mut inicio = (m.start_season as usize).clamp(1, mi.max(1));
    if let Some((anio, est)) = periodo_inicial(&m.start, mi)? {
        inicio = est;
        // los nombres vacíos o de la plantilla (1, 2, 3…) se cambian por los periodos
        let plantilla = nombres.iter().enumerate().all(|(k, n)| *n == (k + 1).to_string());
        for (k, n) in nombres.iter_mut().enumerate() {
            if plantilla || n.trim().is_empty() {
                *n = nombre_periodo(anio, est, k, mi);
            }
        }
    }
    Ok(Datos { y, nombres, m: m.frequency, mi, inicio })
}

/// "2021-05", "2021/5", "2021-T2" o "2021-Q2" → (año, estación).
fn periodo_inicial(t: &str, mi: usize) -> Result<Option<(i64, usize)>, ModError> {
    let t = t.trim();
    if t.is_empty() {
        return Ok(None);
    }
    let partes: Vec<&str> = t.split(['-', '/', ' ', '.', ',']).filter(|x| !x.is_empty()).collect();
    let mal = || ModError::Invalid(format!("primer periodo no válido: «{t}» (usa año-estación, p. ej. 2021-05 para mayo de 2021)"));
    let anio: i64 = partes.first().and_then(|a| a.parse().ok()).ok_or_else(mal)?;
    let est: usize = match partes.get(1) {
        None => 1,
        Some(e) => e.trim_start_matches(['T', 'Q', 't', 'q', 'S', 's']).parse().map_err(|_| mal())?,
    };
    if est < 1 || (mi > 1 && est > mi) {
        return Err(mal());
    }
    Ok(Some((anio, est)))
}

/// Nombre del periodo k (desde 0) contando desde (año, estación).
fn nombre_periodo(anio: i64, est: usize, k: usize, mi: usize) -> String {
    if mi <= 1 {
        return (anio + k as i64).to_string();
    }
    let idx = est - 1 + k;
    let (a, e) = (anio + (idx / mi) as i64, idx % mi + 1);
    match mi {
        12 => format!("{a}-{e:02}"),
        4 => format!("{a}-T{e}"),
        _ => format!("{a}-{e}"),
    }
}

fn grafico_barras(titulo: &str, valores: &[f64], banda: f64, eje: &str) -> Block {
    let mut ch = Chart::new(Some(titulo), "Retardo", eje);
    ch = ch.series(Series::new(eje, SeriesKind::Bar, valores.iter().enumerate().map(|(k, v)| [(k + 1) as f64, r4(*v)]).collect()).tone(Tone::Accent));
    ch = ch.hline(banda, "+1.96/√n", Tone::Bad).hline(-banda, "−1.96/√n", Tone::Bad);
    ch.y_zero = true;
    ch.block()
}

// ------------------------------------------------------------------ exploración

fn explorar(m: &StModel, o: &StOptions, d: &Datos) -> Result<Report, ModError> {
    let n = d.y.len();
    let k = if o.lags > 0 { (o.lags as usize).min(n - 1) } else { retardos_por_defecto(n, d.mi) };
    let r = acf(&d.y, k);
    let p = pacf(&d.y, k);
    let banda = 1.96 / (n as f64).sqrt();
    let lb_k = if d.mi > 1 { (2 * d.mi).min(n - 1) } else { 10.min(n - 1) };
    let lb = ljung_box(&d.y, lb_k, 0);
    let mult = o.decomposition == "multiplicativa";
    if mult && d.y.iter().any(|v| *v <= 0.0) {
        return invalid("la descomposición multiplicativa necesita datos positivos");
    }
    let dec = descomponer(&d.y, d.mi, d.inicio, mult);
    let lambda = lambda_guerrero(&d.y, d.mi);
    let media = d.y.iter().sum::<f64>() / n as f64;
    let de = (d.y.iter().map(|v| (v - media).powi(2)).sum::<f64>() / (n - 1) as f64).sqrt();

    let mut ms = vec![metric_main("Observaciones", n as f64), metric("Media", r4(media)), metric("Desviación estándar", r4(de)), metric("r(1)", r4(r[0]))];
    let mut conclusiones = vec![];
    let (ft, fs) = dec.as_ref().map(fuerza).unwrap_or((f64::NAN, f64::NAN));
    if dec.is_some() {
        ms.push(metric("Fuerza de la tendencia", r4(ft)));
        ms.push(metric("Fuerza de la estacionalidad", r4(fs)));
    }
    let sig: Vec<usize> = r.iter().enumerate().filter(|(_, v)| v.abs() > banda).map(|(i, _)| i + 1).collect();
    conclusiones.push(if lb.p < 0.05 {
        format!("La serie no es ruido blanco: la prueba de Ljung-Box con {} retardos da Q = {} y p = {} (< 0.05); {} de {} autocorrelaciones salen de las bandas ±{}.", lb.retardos, fmt(r4(lb.q)), fmt(r4(lb.p)), sig.len(), k, fmt(r4(banda)))
    } else {
        format!("La serie se comporta como ruido blanco: Ljung-Box con {} retardos da Q = {} y p = {} (≥ 0.05). No hay patrón que pronosticar más allá de la media.", lb.retardos, fmt(r4(lb.q)), fmt(r4(lb.p)))
    });
    if r.len() >= 3 && r[0] > 0.5 && r[1] > 0.4 && r[2] > 0.3 {
        conclusiones.push("Las autocorrelaciones bajan despacio desde valores altos: hay tendencia (la serie no es estacionaria; un ARIMA necesitará diferenciarla).".into());
    }
    if d.mi > 1 && d.mi <= r.len() && r[d.mi - 1] > banda {
        conclusiones.push(format!("La autocorrelación en el retardo {} (un ciclo) es {}: hay estacionalidad de periodo {}.", d.mi, fmt(r4(r[d.mi - 1])), d.mi));
    }
    if dec.is_some() {
        conclusiones.push(format!(
            "Según la descomposición, la fuerza de la tendencia es {} y la de la estacionalidad {} (0 = nada, 1 = máxima; más de 0.64 se considera fuerte).",
            fmt(r4(ft)),
            fmt(r4(fs))
        ));
    }
    if let Some(l) = lambda {
        conclusiones.push(format!("λ de Box-Cox sugerido (Guerrero): {}{}", fmt(r4(l)), if l.abs() < 0.25 { "; cercano a 0: conviene trabajar con el logaritmo (la variación crece con el nivel)." } else if (l - 1.0).abs() < 0.25 { "; cercano a 1: no hace falta transformar." } else { "." }));
    }

    let mut serie = Chart::new(Some("Serie de tiempo"), &m.time_unit, "Valor");
    serie.categories = Some(d.nombres.clone());
    serie = serie.series(Series::new("Serie", SeriesKind::Line, d.y.iter().enumerate().map(|(k, v)| [k as f64, *v]).collect()).tone(Tone::Accent));
    let mut sol = vec![metrics(ms), serie.block(), grafico_barras("Autocorrelación (ACF)", &r, banda, "ACF"), grafico_barras("Autocorrelación parcial (PACF)", &p, banda, "PACF")];
    for c in &conclusiones {
        sol.push(text(c.clone()));
    }
    let mut tb = Table::new(Some("Correlograma"), &["Retardo", "ACF", "PACF", "¿Fuera de las bandas?"]).left(3);
    for i in 0..r.len() {
        let fuera = r[i].abs() > banda;
        tb.row_tone(cells![(i + 1) as f64, r4(r[i]), r4(p[i]), if fuera { "Sí" } else { "No" }], if fuera { Tone::Highlight } else { Tone::Normal });
    }
    sol.push(tb.block());

    let mut dec_bloques = vec![];
    if let Some(dc) = &dec {
        let mut ch = Chart::new(Some(if mult { "Descomposición clásica multiplicativa" } else { "Descomposición clásica aditiva" }), &m.time_unit, "Valor");
        ch.categories = Some(d.nombres.clone());
        let pts = |v: &[f64]| v.iter().enumerate().filter(|(_, x)| x.is_finite()).map(|(k, x)| [k as f64, r4(*x)]).collect::<Vec<_>>();
        ch = ch.series(Series::new("Serie", SeriesKind::Line, pts(&d.y)).tone(Tone::Muted)).series(Series::new("Tendencia", SeriesKind::Line, pts(&dc.tendencia)).tone(Tone::Accent));
        dec_bloques.push(ch.block());
        let mut ce = Chart::new(Some("Componente estacional"), &m.time_unit, "Índice");
        ce.categories = Some(d.nombres.clone());
        ce = ce.series(Series::new("Estacional", SeriesKind::Line, pts(&dc.estacional)).tone(Tone::Highlight));
        dec_bloques.push(ce.block());
        let mut cr = Chart::new(Some("Componente aleatorio"), &m.time_unit, "Residuo");
        cr.categories = Some(d.nombres.clone());
        cr = cr.series(Series::new("Aleatorio", SeriesKind::Bar, pts(&dc.aleatorio)).tone(Tone::Muted));
        dec_bloques.push(cr.block());
        let mut ti = Table::new(Some("Índices estacionales"), &["Estación", "Índice"]).left(0);
        for (s, v) in dc.indices.iter().enumerate() {
            ti.row(cells![(s + 1) as f64, r4(*v)]);
        }
        dec_bloques.push(ti.block());
        // gráfico estacional: una línea por ciclo
        let mut ge = Chart::new(Some("Gráfico estacional (una línea por ciclo)"), "Estación", "Valor");
        ge.categories = Some((1..=d.mi).map(|s| s.to_string()).collect());
        let mut ciclo = 0;
        let mut pts_c: Vec<[f64; 2]> = vec![];
        for t in 0..n {
            let s = (t + d.inicio - 1) % d.mi;
            if s == 0 && !pts_c.is_empty() {
                ciclo += 1;
                ge = ge.series(Series::new(format!("Ciclo {ciclo}"), SeriesKind::Line, std::mem::take(&mut pts_c)));
            }
            pts_c.push([s as f64, d.y[t]]);
        }
        if !pts_c.is_empty() {
            ge = ge.series(Series::new(format!("Ciclo {}", ciclo + 1), SeriesKind::Line, pts_c));
        }
        dec_bloques.push(ge.block());
    } else if d.mi > 1 {
        dec_bloques.push(text(format!("Para descomponer se necesitan al menos dos ciclos completos ({} datos).", 2 * d.mi)));
    }

    let pasos = if o.steps {
        vec![steps(vec![
            step("Autocorrelación", format!("r(k) = Σ (yₜ − ȳ)(yₜ₊ₖ − ȳ) / Σ (yₜ − ȳ)², con ȳ = {}. Las bandas ±1.96/√n = ±{} marcan las autocorrelaciones significativas al 5 %.", fmt(r4(media)), fmt(r4(banda))), vec![]),
            step("Autocorrelación parcial", "Recursión de Durbin-Levinson: φₖₖ = (r(k) − Σ φₖ₋₁,ⱼ r(k−j)) / (1 − Σ φₖ₋₁,ⱼ r(j)).", vec![]),
            step("Prueba de Ljung-Box", format!("Q = n(n+2) Σ r(k)²/(n−k) = {} con {} grados de libertad; p = {}.", fmt(r4(lb.q)), lb.gl, fmt(r4(lb.p))), vec![]),
            step(
                "Descomposición clásica",
                if d.mi % 2 == 0 { format!("Tendencia: promedio móvil centrado 2×{}; índices: promedio de cada estación de la serie sin tendencia, normalizados.", d.mi) } else { format!("Tendencia: promedio móvil de {}; índices: promedio de cada estación de la serie sin tendencia, normalizados.", d.mi) },
                vec![],
            ),
        ])]
    } else {
        vec![]
    };
    let head = format!(
        "{}{}{}",
        if lb.p < 0.05 { "La serie tiene autocorrelación (Ljung-Box p < 0.05)" } else { "La serie se comporta como ruido blanco" },
        if dec.is_some() { format!("; fuerza de la tendencia {} y de la estacionalidad {}", fmt(r4(ft)), fmt(r4(fs))) } else { String::new() },
        lambda.map(|l| format!("; λ de Box-Cox sugerido {}.", fmt(r4(l)))).unwrap_or_else(|| ".".into())
    );
    let rep = Report::new("st", &m.name, Status::Solved, head)
        .section("solucion", "Exploración", sol)
        .section("descomposicion", "Descomposición", dec_bloques)
        .section("pasos", "Paso a paso", pasos)
        .data(json!({"acf": r.iter().map(|x| r4(*x)).collect::<Vec<_>>(), "pacf": p.iter().map(|x| r4(*x)).collect::<Vec<_>>(), "ljung_box": {"q": r4(lb.q), "df": lb.gl, "p_value": lb.p, "lags": lb.retardos},
            "trend_strength": r4(ft), "seasonal_strength": r4(fs), "lambda": lambda.map(r4), "seasonal_indices": dec.as_ref().map(|x| x.indices.iter().map(|v| r4(*v)).collect::<Vec<_>>()),
            "trend": dec.as_ref().map(|x| x.tendencia.iter().map(|v| if v.is_finite() { json!(r4(*v)) } else { Value::Null }).collect::<Vec<_>>()) }))
        .cite("hyndman2018fpp", "Exploración de series de tiempo: autocorrelación, descomposición, fuerza de la tendencia y de la estacionalidad")
        .cite("ljungbox1978", "Prueba de Ljung-Box")
        .cite("arango2021series", "Exploración y visualización de series de tiempo en el curso");
    Ok(rep)
}

// ------------------------------------------------------------------ pronóstico

fn familia(m: &StModel, nombre: &str, d: &Datos) -> Result<Familia, ModError> {
    Ok(match nombre {
        "arima" => {
            let t = m.arima_order.trim();
            let orden = if t.is_empty() {
                None
            } else {
                let v: Vec<usize> = t.split([',', ' ', ';']).filter(|s| !s.is_empty()).map(|s| s.trim().parse::<usize>()).collect::<Result<_, _>>().map_err(|_| ModError::Invalid(format!("orden ARIMA no válido: «{t}» (usa p,d,q o p,d,q,P,D,Q)")))?;
                match v.len() {
                    3 => Some([v[0], v[1], v[2], 0, 0, 0]),
                    6 => Some([v[0], v[1], v[2], v[3], v[4], v[5]]),
                    _ => return invalid(format!("orden ARIMA no válido: «{t}» (usa p,d,q o p,d,q,P,D,Q)")),
                }
            };
            Familia::Arima { orden, deriva: m.drift }
        }
        "ets" => Familia::Ets { spec: Some(m.ets_model.trim().to_uppercase()).filter(|s| !s.is_empty() && s != "ZZZ") },
        "ses" | "suavizacion_simple" => Familia::Ses,
        "holt" => Familia::Holt { amortiguada: m.damped },
        "holt_amortiguado" => Familia::Holt { amortiguada: true },
        "hw" | "holt_winters" => Familia::Hw { multiplicativa: m.seasonal_type.trim().to_lowercase().starts_with('m'), amortiguada: m.damped },
        "hw_multiplicativo" => Familia::Hw { multiplicativa: true, amortiguada: m.damped },
        "hw_aditivo" => Familia::Hw { multiplicativa: false, amortiguada: m.damped },
        "tbats" => {
            let t = m.seasonal_periods.trim();
            let periodos: Vec<f64> = if t.is_empty() { vec![d.m] } else { t.split([',', ';', ' ']).filter(|s| !s.is_empty()).map(|s| s.trim().replace(',', ".").parse::<f64>()).collect::<Result<_, _>>().map_err(|_| ModError::Invalid(format!("periodos estacionales no válidos: «{t}»")))? };
            Familia::Tbats { periodos }
        }
        "theta" => Familia::Theta,
        "croston" => Familia::Croston,
        "ingenuo" | "naive" => Familia::Ingenuo,
        "ingenuo_estacional" | "snaive" => Familia::IngenuoEstacional,
        "auto.arima" | "auto_arima" => Familia::Arima { orden: None, deriva: false },
        "thetaf" => Familia::Theta,
        o => return invalid(format!("modelo desconocido: «{o}» (usa auto, ses, holt, hw, arima, ets, tbats, theta, croston, ingenuo o ingenuo_estacional)")),
    })
}

/// Motores a probar, en orden de preferencia.
fn motores_pedidos(o: &StOptions) -> Result<Vec<Motor>, ModError> {
    let disp = motores::disponibles();
    Ok(match o.forecast_engine.as_str() {
        "" | "auto" => disp,
        "r" => {
            if !disp.contains(&Motor::R) {
                return invalid("R con el paquete forecast no está instalado en este equipo (instala R y luego install.packages(\"forecast\"), o usa el motor automático)");
            }
            vec![Motor::R]
        }
        "python" => {
            if !disp.contains(&Motor::Python) {
                return invalid("statsforecast no está instalado en el Python de este equipo (pip install statsforecast, o usa el motor automático)");
            }
            vec![Motor::Python]
        }
        "nativo" => vec![Motor::Nativo],
        e => return invalid(format!("motor desconocido: «{e}» (usa auto, r, python o nativo)")),
    })
}

/// Ajusta con el primer motor que funcione; devuelve el motor usado.
fn ajustar(motores_: &[Motor], s: &Serie, trabajos: &[Trabajo]) -> Result<(Motor, Vec<Result<Ajuste, String>>, Vec<String>), ModError> {
    let mut avisos = vec![];
    for &mo in motores_ {
        match motores::ajustar(mo, s, trabajos) {
            Ok(r) if r.iter().any(|x| x.is_ok()) => return Ok((mo, r, avisos)),
            Ok(r) => avisos.push(format!("{}: {}", mo.nombre(), r.iter().filter_map(|x| x.as_ref().err()).next().cloned().unwrap_or_default())),
            Err(e) => avisos.push(format!("{}: {e}", mo.nombre())),
        }
    }
    invalid(format!("ningún motor pudo ajustar el modelo ({})", avisos.join("; ")))
}

fn pronosticar(m: &StModel, o: &StOptions, d: &Datos) -> Result<Report, ModError> {
    let n = d.y.len();
    let h = if m.horizon > 0 { m.horizon as usize } else if d.mi > 1 { 2 * d.mi } else { 10 };
    let lambda = match m.lambda.trim() {
        "" => None,
        "auto" => Some(lambda_guerrero(&d.y, d.mi).ok_or_else(|| ModError::Invalid("el λ automático de Box-Cox necesita datos positivos y al menos dos ciclos".into()))?),
        t => Some(t.replace(',', ".").parse::<f64>().map_err(|_| ModError::Invalid(format!("λ de Box-Cox no válido: «{t}»")))?),
    };
    if lambda.is_some() && d.y.iter().any(|v| *v <= 0.0) {
        return invalid("la transformación de Box-Cox necesita datos positivos");
    }
    if !(50.0..100.0).contains(&m.level) {
        return invalid("el nivel de los intervalos debe estar entre 50 y 99.9 %");
    }
    let serie = Serie { y: &d.y, m: d.m, inicio: d.inicio, lambda, nivel: m.level };
    let mot = motores_pedidos(o)?;
    let modelo = m.model.trim().to_lowercase();

    // comparación en un periodo de prueba
    let mut comparacion: Option<(Vec<(String, Errores)>, usize, usize)> = None;
    let mut avisos = vec![];
    let fam = if modelo == "auto" || modelo.is_empty() {
        let k = if m.test_size > 0 { m.test_size as usize } else { h.min(n / 5).max(1) };
        if n - k < 8.max(2 * d.mi) {
            return invalid(format!("para comparar modelos con {k} periodos de prueba hacen falta al menos {} datos", k + 8.max(2 * d.mi)));
        }
        let mut cand = vec![Familia::Arima { orden: None, deriva: false }, Familia::Ets { spec: None }, Familia::Theta];
        if d.mi > 1 && n - k >= 2 * d.mi {
            cand.push(Familia::Tbats { periodos: vec![d.m] });
            cand.push(Familia::IngenuoEstacional);
        } else {
            cand.push(Familia::Ingenuo);
        }
        let trabajos: Vec<Trabajo> = cand.iter().map(|f| Trabajo { familia: f.clone(), n: n - k, h: k }).collect();
        let (mo, res, av) = ajustar(&mot, &serie, &trabajos)?;
        avisos.extend(av);
        let mut filas = vec![];
        for (f, r) in cand.iter().zip(&res) {
            match r {
                Ok(a) => filas.push((format!("{} → {}", f.etiqueta(), a.modelo), errores(&d.y[n - k..], &a.media, &d.y[..n - k], d.mi), f.clone())),
                Err(e) => avisos.push(format!("{}: {e}", f.etiqueta())),
            }
        }
        if filas.is_empty() {
            return invalid("ningún modelo se pudo ajustar en el periodo de entrenamiento");
        }
        let mejor = filas.iter().filter(|f| f.1.mase.is_finite()).min_by(|a, b| a.1.mase.partial_cmp(&b.1.mase).unwrap()).or(filas.first()).unwrap().2.clone();
        let idx = filas.iter().position(|f| f.2 == mejor).unwrap();
        comparacion = Some((filas.iter().map(|f| (f.0.clone(), f.1.clone())).collect(), idx, k));
        let _ = mo;
        mejor
    } else {
        familia(m, &modelo, d)?
    };

    let (motor, res, av) = ajustar(&mot, &serie, &[Trabajo { familia: fam.clone(), n, h }])?;
    avisos.extend(av);
    let a = res.into_iter().next().unwrap().map_err(ModError::Invalid)?;
    let residuos: Vec<f64> = d.y.iter().zip(&a.ajustados).map(|(y, f)| if f.is_finite() { y - f } else { f64::NAN }).collect();
    let res_ok: Vec<f64> = residuos.iter().copied().filter(|x| x.is_finite()).collect();
    let err = errores(&d.y, &a.ajustados, &d.y, d.mi);
    let lb = prueba_residuos(&a, &fam, &res_ok, d.mi);
    let (npar, lbk) = (lb.retardos - lb.gl, lb.retardos);

    // nombres de los periodos futuros
    let futuros: Vec<String> = (1..=h).map(|i| format!("+{i}")).collect();
    let mut tabla = Table::new(Some(&format!("Pronóstico con intervalos del {} %", fmt(m.level))), &["Periodo", "Pronóstico", "Límite inferior", "Límite superior"]).left(0);
    for i in 0..h {
        tabla.row(cells![futuros[i].as_str(), celda(a.media.get(i).copied().unwrap_or(f64::NAN)), celda(a.inferior.get(i).copied().unwrap_or(f64::NAN)), celda(a.superior.get(i).copied().unwrap_or(f64::NAN))]);
    }
    let mut ch = Chart::new(Some(&format!("{}: historia, ajuste y pronóstico", a.modelo)), &m.time_unit, "Valor");
    let mut cats = d.nombres.clone();
    cats.extend(futuros.iter().cloned());
    ch.categories = Some(cats);
    ch = ch
        .series(Series::new("Serie", SeriesKind::Line, d.y.iter().enumerate().map(|(k, v)| [k as f64, *v]).collect()).tone(Tone::Muted))
        .series(Series::new("Ajustados", SeriesKind::Line, a.ajustados.iter().enumerate().filter(|(_, v)| v.is_finite()).map(|(k, v)| [k as f64, r4(*v)]).collect()).tone(Tone::Highlight).dashed())
        .series(Series::new("Pronóstico", SeriesKind::Line, a.media.iter().enumerate().map(|(k, v)| [(n + k) as f64, r4(*v)]).collect()).tone(Tone::Accent));
    if a.inferior.iter().any(|v| v.is_finite()) {
        ch = ch
            .series(Series::new(format!("Inferior {} %", fmt(m.level)), SeriesKind::Line, a.inferior.iter().enumerate().filter(|(_, v)| v.is_finite()).map(|(k, v)| [(n + k) as f64, r4(*v)]).collect()).tone(Tone::Accent).dashed())
            .series(Series::new(format!("Superior {} %", fmt(m.level)), SeriesKind::Line, a.superior.iter().enumerate().filter(|(_, v)| v.is_finite()).map(|(k, v)| [(n + k) as f64, r4(*v)]).collect()).tone(Tone::Accent).dashed());
    }
    let mut ms = vec![metric_main("Modelo", Cell::from(a.modelo.as_str())), metric("RMSE", r4(err.rmse)), metric("MAE", r4(err.mae)), metric("MAPE (%)", r4(err.mape)), metric("MASE", r4(err.mase))];
    if let Some(x) = a.aicc.or(a.aic) {
        ms.push(metric(if a.aicc.is_some() { "AICc" } else { "AIC" }, r4(x)));
    }
    let mut sol = vec![metrics(ms), ch.block(), tabla.block()];
    sol.push(text(format!("Motor: {}.", motor.nombre())));
    if let Some(l) = lambda {
        sol.push(text(format!("Se trabajó con la transformación de Box-Cox λ = {} y los resultados se devolvieron a la escala original.", fmt(r4(l)))));
    }
    if let Some(nota) = &a.nota {
        sol.push(text_tone(format!("Nota: {nota}."), Tone::Highlight));
    }
    if !avisos.is_empty() {
        sol.push(text_tone(format!("Avisos: {}.", avisos.join("; ")), Tone::Muted));
    }

    // comparación
    let mut cmp_bloques = vec![];
    if let Some((filas, idx, k)) = &comparacion {
        let mut tc = Table::new(Some(&format!("Comparación en los últimos {k} periodos (entrenamiento con los {} primeros)", n - k)), &["Modelo", "RMSE", "MAE", "MAPE (%)", "MASE"]).left(0);
        for (i, (nme, e)) in filas.iter().enumerate() {
            tc.row_tone(cells![nme.as_str(), celda(e.rmse), celda(e.mae), celda(e.mape), celda(e.mase)], if i == *idx { Tone::Highlight } else { Tone::Normal });
        }
        cmp_bloques.push(tc.block());
        cmp_bloques.push(text(format!("Se elige el de menor MASE en el periodo de prueba ({}) y se vuelve a ajustar con toda la serie. MASE < 1 quiere decir que el modelo pronostica mejor que el método ingenuo{}.", filas[*idx].0, if d.mi > 1 { " estacional" } else { "" })));
        let mut cc = Chart::new(Some("MASE en el periodo de prueba"), "Modelo", "MASE");
        cc.categories = Some(filas.iter().map(|f| f.0.clone()).collect());
        cc = cc.series(Series::new("MASE", SeriesKind::Bar, filas.iter().enumerate().map(|(i, f)| [i as f64, r4(f.1.mase)]).collect()).tone(Tone::Accent));
        cc.y_zero = true;
        cmp_bloques.push(cc.block());
    }

    // residuos
    let mut res_bloques = vec![];
    res_bloques.push(text(if lb.p < 0.05 {
        format!("Los residuos tienen autocorrelación (Ljung-Box Q* = {}, gl = {}, p = {}): al modelo le queda información sin aprovechar.", fmt(r4(lb.q)), lb.gl, fmt(r4(lb.p)))
    } else {
        format!("Los residuos se comportan como ruido blanco (Ljung-Box Q* = {}, gl = {}, p = {} ≥ 0.05): el modelo aprovecha la información de la serie.", fmt(r4(lb.q)), lb.gl, fmt(r4(lb.p)))
    }));
    let racf = acf(&res_ok, retardos_por_defecto(res_ok.len(), d.mi));
    res_bloques.push(grafico_barras("ACF de los residuos", &racf, 1.96 / (res_ok.len() as f64).sqrt(), "ACF"));
    let mut cr = Chart::new(Some("Residuos"), &m.time_unit, "Residuo");
    cr.categories = Some(d.nombres.clone());
    cr = cr.series(Series::new("Residuo", SeriesKind::Bar, residuos.iter().enumerate().filter(|(_, v)| v.is_finite()).map(|(k, v)| [k as f64, r4(*v)]).collect()).tone(Tone::Muted));
    res_bloques.push(cr.block());
    let mut te = Table::new(Some("Medidas de error en el entrenamiento (como accuracy de R)"), &["ME", "RMSE", "MAE", "MPE", "MAPE", "MASE", "ACF1"]);
    te.row(cells![celda(err.me), celda(err.rmse), celda(err.mae), celda(err.mpe), celda(err.mape), celda(err.mase), celda(err.acf1)]);
    res_bloques.push(te.block());

    // parámetros
    let mut par_bloques = vec![];
    if !a.coef.is_empty() {
        let mut tp = Table::new(Some("Parámetros estimados"), &["Parámetro", "Valor"]).left(0);
        for (k, v) in &a.coef {
            tp.row(cells![k.as_str(), celda(*v)]);
        }
        par_bloques.push(tp.block());
    }
    let mut ti = Table::new(Some("Criterios de información"), &["AIC", "AICc", "BIC", "σ²"]);
    ti.row(cells![a.aic.map(celda).unwrap_or(Cell::from("—")), a.aicc.map(celda).unwrap_or(Cell::from("—")), a.bic.map(celda).unwrap_or(Cell::from("—")), a.sigma2.map(celda).unwrap_or(Cell::from("—"))]);
    par_bloques.push(ti.block());

    let pasos = if o.steps {
        let explica = match &fam {
            Familia::Arima { orden: None, .. } => "ARIMA automático (Hyndman y Khandakar, 2008): el número de diferencias d con pruebas de raíz unitaria (KPSS), las diferencias estacionales D con la fuerza de la estacionalidad; luego búsqueda paso a paso de p, q, P y Q minimizando el AICc, con los parámetros por máxima verosimilitud.".to_string(),
            Familia::Arima { orden: Some(_), .. } => "ARIMA con los órdenes dados: parámetros por máxima verosimilitud.".to_string(),
            Familia::Ets { .. } => "ETS (error, tendencia, estacionalidad) en espacio de estados con innovaciones (Hyndman et al., 2002): se prueban los modelos admisibles y se elige el de menor AICc; los parámetros se estiman por máxima verosimilitud.".to_string(),
            Familia::Ses | Familia::Holt { .. } | Familia::Hw { .. } => explicacion_suavizacion(&fam),
            Familia::Tbats { .. } => "TBATS (De Livera et al., 2011): Box-Cox, estacionalidad trigonométrica (también varias y no enteras), errores ARMA y tendencia (posiblemente amortiguada); se elige la configuración de menor AIC.".to_string(),
            Familia::Theta => "Método Theta (Assimakopoulos y Nikolopoulos, 2000): suavización exponencial simple con deriva igual a la mitad de la pendiente de la tendencia lineal, sobre la serie desestacionalizada.".to_string(),
            Familia::Croston => "Croston (1972): suaviza por separado el tamaño de las demandas distintas de cero y el intervalo entre ellas (α = 0.1); el pronóstico es tamaño / intervalo.".to_string(),
            Familia::Ingenuo => "Ingenuo: el pronóstico es el último valor observado.".to_string(),
            Familia::IngenuoEstacional => "Ingenuo estacional: el pronóstico es el valor de la misma estación del último ciclo.".to_string(),
        };
        let mut v = vec![step("Modelo", explica, vec![])];
        if comparacion.is_some() {
            v.insert(0, step("Comparación de modelos", "Se separan los últimos periodos como prueba, se ajusta cada modelo con el resto y se miden sus errores en la prueba; el de menor MASE gana.", cmp_bloques.clone()));
        }
        v.push(step("Revisión de los residuos", format!("Ljung-Box con {} retardos y {} parámetros: Q* = {}, p = {}.", lb.retardos, npar.min(lbk.saturating_sub(1)), fmt(r4(lb.q)), fmt(r4(lb.p))), vec![]));
        vec![steps(v)]
    } else {
        vec![]
    };

    let head = format!(
        "{}: pronóstico de {} periodos{}; RMSE {}, MASE {}.",
        a.modelo,
        h,
        if comparacion.is_some() { " (el mejor en el periodo de prueba)" } else { "" },
        fmt(r4(err.rmse)),
        fmt(r4(err.mase))
    );
    let fam_id = match &fam {
        Familia::Arima { .. } => "arima",
        Familia::Ets { .. } => "ets",
        Familia::Ses => "ses",
        Familia::Holt { .. } => "holt",
        Familia::Hw { .. } => "hw",
        Familia::Tbats { .. } => "tbats",
        Familia::Theta => "theta",
        Familia::Croston => "croston",
        Familia::Ingenuo => "ingenuo",
        Familia::IngenuoEstacional => "ingenuo_estacional",
    };
    let mut rep = Report::new("st", &m.name, Status::Solved, head)
        .section("solucion", "Pronóstico", sol)
        .section("comparacion", "Comparación", cmp_bloques)
        .section("residuos", "Residuos", res_bloques)
        .section("parametros", "Parámetros", par_bloques)
        .section("pasos", "Paso a paso", pasos)
        .data(json!({
            "model": a.modelo, "family": fam_id, "engine": match motor { Motor::R => "r", Motor::Python => "python", Motor::Nativo => "nativo" }, "engine_name": motor.nombre(),
            "forecast": a.media.iter().map(|v| r4(*v)).collect::<Vec<_>>(), "lower": a.inferior.iter().map(|v| if v.is_finite() { json!(r4(*v)) } else { Value::Null }).collect::<Vec<_>>(),
            "upper": a.superior.iter().map(|v| if v.is_finite() { json!(r4(*v)) } else { Value::Null }).collect::<Vec<_>>(), "level": m.level,
            "coefficients": a.coef.iter().map(|(k, v)| json!({"name": k, "value": v})).collect::<Vec<_>>(), "aic": a.aic, "aicc": a.aicc, "bic": a.bic, "sigma2": a.sigma2,
            "accuracy": {"me": err.me, "rmse": err.rmse, "mae": err.mae, "mpe": err.mpe, "mape": err.mape, "mase": err.mase, "acf1": err.acf1},
            "ljung_box": {"q": lb.q, "df": lb.gl, "p_value": lb.p}, "lambda": lambda,
            "comparison": comparacion.as_ref().map(|(f, i, k)| json!({"test_size": k, "best": f[*i].0, "models": f.iter().map(|(nme, e)| json!({"model": nme, "rmse": e.rmse, "mae": e.mae, "mape": e.mape, "mase": e.mase})).collect::<Vec<_>>()})),
        }))
        .cite("hyndman2018fpp", "Pronósticos con ARIMA, ETS, TBATS, medidas de error y revisión de residuos")
        .cite("ljungbox1978", "Prueba de Ljung-Box de los residuos")
        .cite("arango2024arima", "Modelos ARIMA, ETS y TBATS en R en el curso");
    rep = match &fam {
        Familia::Arima { .. } => rep.cite("hyndmankhandakar2008", "Algoritmo de Hyndman-Khandakar (auto.arima) y paquete forecast"),
        Familia::Ets { .. } => rep.cite("hyndman2002ets", "Modelos ETS en espacio de estados y selección automática").cite("hyndmankhandakar2008", "Paquete forecast: ets()"),
        Familia::Ses | Familia::Holt { .. } | Familia::Hw { .. } => rep.cite("hyndman2002ets", "Suavización exponencial como modelo ETS (ses, holt y hw de forecast)").cite("hyndmankhandakar2008", "Paquete forecast"),
        Familia::Tbats { .. } => rep.cite("delivera2011", "Modelo TBATS"),
        Familia::Theta => rep.cite("assimakopoulos2000", "Método Theta"),
        Familia::Croston => rep.cite("croston1972", "Método de Croston para demanda intermitente"),
        _ => rep,
    };
    if comparacion.is_some() {
        rep = rep.cite("hyndmankoehler2006", "MASE para comparar modelos en el periodo de prueba");
    }
    rep = rep.cite_if(motor == Motor::Nativo, "anofox2026", "Motor nativo de pronósticos en Rust");
    Ok(rep)
}

/// Ljung-Box de los residuos como checkresiduals() de forecast 9: con R se toma la de R;
/// si no, retardos = 2m (o 10 sin estacionalidad), a lo sumo n/5 redondeado, y los grados
/// de libertad del modelo solo cuentan en los ARIMA.
fn prueba_residuos(a: &Ajuste, fam: &Familia, res_ok: &[f64], mi: usize) -> LjungBox {
    if let Some((q, gl, p, ret)) = a.ljung_box {
        return LjungBox { q, gl, p, retardos: ret.max(gl) };
    }
    let df = if matches!(fam, Familia::Arima { .. }) { a.coef.iter().filter(|(k, _)| k != "lambda").count() } else { 0 };
    let k = retardos_residuos(res_ok.len(), mi).max(df + 3).min(res_ok.len().saturating_sub(1)).max(1);
    let mut lb = ljung_box(res_ok, k, df);
    lb.gl = k.saturating_sub(df).max(1);
    lb
}

fn explicacion_suavizacion(f: &Familia) -> String {
    match f {
        Familia::Ses => "Suavización exponencial simple, ses() de R: ℓₜ = α yₜ + (1 − α) ℓₜ₋₁ y el pronóstico es el último nivel. α y el nivel inicial ℓ₀ se estiman minimizando el error cuadrático medio (ETS(A,N,N) con opt.crit = \"mse\").".into(),
        Familia::Holt { amortiguada } => format!(
            "Tendencia lineal de Holt, holt() de R{}: nivel ℓₜ = α yₜ + (1 − α)(ℓₜ₋₁ + {}bₜ₋₁) y tendencia bₜ = β*(ℓₜ − ℓₜ₋₁) + (1 − β*){}bₜ₋₁. Los parámetros y los estados iniciales se estiman minimizando el error cuadrático medio.",
            if *amortiguada { " con tendencia amortiguada (φ)" } else { "" },
            if *amortiguada { "φ" } else { "" },
            if *amortiguada { "φ" } else { "" }
        ),
        Familia::Hw { multiplicativa, .. } => format!(
            "Holt-Winters {}, hw() de R: nivel, tendencia y un índice por estación ({}). Los parámetros α, β, γ y los estados iniciales se estiman minimizando el error cuadrático medio.",
            if *multiplicativa { "multiplicativo" } else { "aditivo" },
            if *multiplicativa { "los índices promedian 1" } else { "los índices suman 0" }
        ),
        _ => String::new(),
    }
}

/// Theil's U de accuracy() en el conjunto de prueba.
fn theil_u(real: &[f64], prono: &[f64]) -> f64 {
    let n = real.len().min(prono.len());
    if n < 2 {
        return f64::NAN;
    }
    let (mut a, mut b) = (0.0, 0.0);
    for t in 1..n {
        let x0 = real[t - 1];
        if x0 == 0.0 {
            continue;
        }
        a += ((prono[t] - real[t]) / x0).powi(2);
        b += ((real[t] - x0) / x0).powi(2);
    }
    if b > 0.0 {
        (a / b).sqrt()
    } else {
        f64::NAN
    }
}

/// Suavización exponencial que sugiere el correlograma del entrenamiento.
fn recomendar_suavizacion(r: &[f64], banda: f64, mi: usize) -> (Familia, String) {
    let f = |x: f64| fmt(r4(x));
    if mi > 1 && mi <= r.len() && r[mi - 1] > banda {
        (
            Familia::Hw { multiplicativa: false, amortiguada: false },
            format!("La autocorrelación en el retardo {mi} (un ciclo) es {}, fuera de la banda ±{}: hay estacionalidad. Lo apropiado es Holt-Winters (suavización con tendencia y estacionalidad).", f(r[mi - 1]), f(banda)),
        )
    } else if r.len() >= 2 && r[0] > banda && r[1] > banda && r[0] >= r[1] {
        (
            Familia::Holt { amortiguada: false },
            format!("Las primeras autocorrelaciones son positivas, salen de la banda ±{} y bajan despacio (r(1) = {}, r(2) = {}): hay tendencia sin estacionalidad. Lo apropiado es la suavización doble de Holt.", f(banda), f(r[0]), f(r[1])),
        )
    } else {
        let fuera = r.iter().filter(|x| x.abs() > banda).count();
        (
            Familia::Ses,
            format!(
                "Ninguna autocorrelación muestra tendencia (no hay valores altos que bajen despacio) ni estacionalidad (r({}) no sale de la banda ±{}){}. Lo apropiado es la suavización exponencial simple.",
                mi.max(1),
                f(banda),
                if fuera > 0 { format!("; {fuera} de {} retardos salen de la banda, sin formar un patrón", r.len()) } else { "; todas quedan dentro de la banda".into() }
            ),
        )
    }
}

fn nombre_modelo(f: &Familia) -> &'static str {
    match f {
        Familia::Ses => "ses",
        Familia::Holt { amortiguada: false } => "holt",
        Familia::Holt { amortiguada: true } => "holt_amortiguado",
        Familia::Hw { multiplicativa: false, .. } => "hw_aditivo",
        Familia::Hw { multiplicativa: true, .. } => "hw_multiplicativo",
        Familia::Arima { .. } => "arima",
        Familia::Ets { .. } => "ets",
        Familia::Tbats { .. } => "tbats",
        Familia::Theta => "theta",
        Familia::Croston => "croston",
        Familia::Ingenuo => "ingenuo",
        Familia::IngenuoEstacional => "ingenuo_estacional",
    }
}

/// Evaluación como en clase: entrenamiento y prueba, accuracy() y checkresiduals() de cada modelo.
fn evaluar(m: &StModel, o: &StOptions, d: &Datos) -> Result<Report, ModError> {
    let n = d.y.len();
    let nt = if !m.train_end.trim().is_empty() {
        let fin = m.train_end.trim();
        let normal = |x: &str| x.trim().to_lowercase().replace(['/', ' ', '.'], "-");
        let pos = d.nombres.iter().position(|x| normal(x) == normal(fin)).or_else(|| {
            // 2023-12 también como 2023-12 frente a «2023-12» de otro formato (2023-T4, 2023-4…)
            let (a, e) = periodo_inicial(fin, d.mi).ok().flatten()?;
            d.nombres.iter().position(|x| periodo_inicial(x, d.mi).ok().flatten() == Some((a, e)))
        });
        match pos {
            Some(i) => i + 1,
            None => return invalid(format!("el último periodo del entrenamiento «{fin}» no está entre los periodos de la serie ({} … {}); indica el primer periodo (start, p. ej. 2021-05) o usa test_size", d.nombres[0], d.nombres[n - 1])),
        }
    } else if m.test_size > 0 {
        n.saturating_sub(m.test_size as usize)
    } else {
        let h = if m.horizon > 0 { m.horizon as usize } else if d.mi > 1 { d.mi } else { (n / 5).max(1) };
        n.saturating_sub(h)
    };
    if nt < 4 || nt >= n {
        return invalid(format!("el entrenamiento debe tener al menos 4 datos y dejar al menos uno para la prueba (hay {n} datos y el entrenamiento tendría {nt})"));
    }
    let k = n - nt;
    let tr = &d.y[..nt];
    let te = &d.y[nt..];

    // correlograma del entrenamiento
    let lags = if o.lags > 0 { (o.lags as usize).min(nt - 1) } else { retardos_por_defecto(nt, d.mi) };
    let r = acf(tr, lags);
    let banda = 1.96 / (nt as f64).sqrt();
    let (sugerida, por_que) = recomendar_suavizacion(&r, banda, d.mi);
    let estacional = matches!(sugerida, Familia::Hw { .. });

    // modelos pedidos
    let lista = m.models.trim().to_lowercase();
    let fams: Vec<Familia> = if lista.is_empty() || lista == "auto" {
        let mut v = vec![sugerida.clone(), if estacional { Familia::IngenuoEstacional } else { Familia::Ingenuo }];
        v.push(Familia::Tbats { periodos: vec![d.m] });
        v
    } else {
        let mut v = vec![];
        for nombre in lista.split([',', ';']).map(str::trim).filter(|x| !x.is_empty()) {
            let nombre = nombre.replace(['(', ')', ' '], "");
            let f = match nombre.as_str() {
                "suavizacion" | "sugerida" | "recomendada" => sugerida.clone(),
                "ingenuo_sugerido" => if estacional { Familia::IngenuoEstacional } else { Familia::Ingenuo },
                x => familia(m, x, d)?,
            };
            if !v.contains(&f) {
                v.push(f);
            }
        }
        v
    };
    let lambda = match m.lambda.trim() {
        "" => None,
        "auto" => lambda_guerrero(tr, d.mi),
        t => Some(t.replace(',', ".").parse::<f64>().map_err(|_| ModError::Invalid(format!("λ de Box-Cox no válido: «{t}»")))?),
    };
    let serie = Serie { y: &d.y, m: d.m, inicio: d.inicio, lambda, nivel: m.level };
    let mot = motores_pedidos(o)?;
    let trabajos: Vec<Trabajo> = fams.iter().map(|f| Trabajo { familia: f.clone(), n: nt, h: k }).collect();
    let (motor, res, mut avisos) = ajustar(&mot, &serie, &trabajos)?;

    struct Fila {
        fam: Familia,
        a: Ajuste,
        ent: Errores,
        pru: Errores,
        u: f64,
        lb: LjungBox,
    }
    let mut filas: Vec<Fila> = vec![];
    for (f, r) in fams.iter().zip(res) {
        match r {
            Ok(a) => {
                let ent = errores(tr, &a.ajustados, tr, d.mi);
                let pru = errores(te, &a.media, tr, d.mi);
                let u = theil_u(te, &a.media);
                let res_ok: Vec<f64> = tr.iter().zip(&a.ajustados).filter(|(_, f)| f.is_finite()).map(|(y, f)| y - f).collect();
                let lb = prueba_residuos(&a, f, &res_ok, d.mi);
                filas.push(Fila { fam: f.clone(), a, ent, pru, u, lb });
            }
            Err(e) => avisos.push(format!("{}: {e}", f.etiqueta())),
        }
    }
    if filas.is_empty() {
        return invalid(format!("ningún modelo se pudo ajustar ({})", avisos.join("; ")));
    }
    let mejor = |g: &dyn Fn(&Fila) -> f64| filas.iter().enumerate().filter(|(_, x)| g(x).is_finite()).min_by(|a, b| g(a.1).partial_cmp(&g(b.1)).unwrap()).map(|(i, _)| i);
    let mejor_ent = mejor(&|x: &Fila| x.ent.rmse);
    let mejor_pru = mejor(&|x: &Fila| x.pru.rmse);
    let etiqueta = |x: &Fila| format!("{} ({})", x.fam.etiqueta(), x.a.modelo);

    // tablas
    let mut ta = Table::new(Some("Exactitud: accuracy(pronóstico, prueba)"), &["Modelo", "Conjunto", "ME", "RMSE", "MAE", "MPE", "MAPE", "MASE", "ACF1", "Theil's U"]).left(1);
    for (i, x) in filas.iter().enumerate() {
        let e = &x.ent;
        ta.row_tone(cells![etiqueta(x).as_str(), "Entrenamiento", celda(e.me), celda(e.rmse), celda(e.mae), celda(e.mpe), celda(e.mape), celda(e.mase), celda(e.acf1), "—"], if Some(i) == mejor_ent { Tone::Highlight } else { Tone::Normal });
        let e = &x.pru;
        ta.row_tone(cells![etiqueta(x).as_str(), "Prueba", celda(e.me), celda(e.rmse), celda(e.mae), celda(e.mpe), celda(e.mape), celda(e.mase), celda(e.acf1), celda(x.u)], if Some(i) == mejor_pru { Tone::Highlight } else { Tone::Normal });
    }
    let mut tr_ = Table::new(Some("RMSE de cada modelo"), &["Modelo", "RMSE entrenamiento", "RMSE prueba"]).left(0);
    for x in &filas {
        tr_.row(cells![etiqueta(x).as_str(), celda(x.ent.rmse), celda(x.pru.rmse)]);
    }
    let mut tl = Table::new(Some("Residuos: checkresiduals() (prueba de Ljung-Box)"), &["Modelo", "Q*", "gl", "p", "Retardos", "¿Ruido blanco?"]).left(0);
    for x in &filas {
        let rb = x.lb.p >= 0.05;
        tl.row_tone(cells![etiqueta(x).as_str(), celda(x.lb.q), x.lb.gl as f64, celda(x.lb.p), x.lb.retardos as f64, if rb { "Sí (1)" } else { "No (2)" }], if rb { Tone::Normal } else { Tone::Bad });
    }
    let mut cab = vec!["Periodo", "Real"];
    let etq: Vec<String> = filas.iter().map(|x| x.fam.etiqueta()).collect();
    cab.extend(etq.iter().map(|s| s.as_str()));
    let mut tp = Table::new(Some("Pronósticos del periodo de prueba"), &cab).left(0);
    for j in 0..k {
        let mut c = cells![d.nombres[nt + j].as_str(), celda(te[j])];
        for x in &filas {
            c.push(celda(x.a.media.get(j).copied().unwrap_or(f64::NAN)));
        }
        tp.row(c);
    }
    let mut ch = Chart::new(Some("Serie, entrenamiento y pronósticos de la prueba"), &m.time_unit, "Valor");
    ch.categories = Some(d.nombres.clone());
    ch = ch.series(Series::new("Serie", SeriesKind::Line, d.y.iter().enumerate().map(|(t, v)| [t as f64, *v]).collect()).tone(Tone::Muted));
    for x in &filas {
        ch = ch.series(Series::new(x.fam.etiqueta(), SeriesKind::Line, x.a.media.iter().enumerate().map(|(j, v)| [(nt + j) as f64, r4(*v)]).collect()));
    }

    let nombre = |i: Option<usize>| i.map(|i| filas[i].fam.etiqueta()).unwrap_or_else(|| "—".into());
    let conclusion = match (mejor_ent, mejor_pru) {
        (Some(a), Some(b)) if a == b => format!("El modelo más recomendable es {}: tiene el menor RMSE tanto en el entrenamiento ({}) como en la prueba ({}).", nombre(Some(a)), fmt(r4(filas[a].ent.rmse)), fmt(r4(filas[a].pru.rmse))),
        (a, b) => format!(
            "En el entrenamiento el menor RMSE es el de {} ({}) y en la prueba el de {} ({}). Para pronosticar pesa más la prueba: son datos que el modelo no vio.",
            nombre(a),
            a.map(|i| fmt(r4(filas[i].ent.rmse))).unwrap_or_default(),
            nombre(b),
            b.map(|i| fmt(r4(filas[i].pru.rmse))).unwrap_or_default()
        ),
    };

    let mut sol = vec![
        metrics(vec![
            metric_main("Mejor en la prueba (RMSE)", Cell::from(nombre(mejor_pru).as_str())),
            metric("Entrenamiento", nt as f64),
            metric("Prueba", k as f64),
            metric("Suavización sugerida", Cell::from(sugerida.etiqueta().as_str())),
        ]),
        text(format!("Entrenamiento: {} a {} ({nt} datos); prueba: {} a {} ({k} datos).", d.nombres[0], d.nombres[nt - 1], d.nombres[nt], d.nombres[n - 1])),
        grafico_barras("Autocorrelograma del entrenamiento (ACF)", &r, banda, "ACF"),
        text(por_que.clone()),
        tr_.block(),
        ta.block(),
        tl.block(),
        text(conclusion.clone()),
        tp.block(),
        ch.block(),
        text(format!("Motor: {}.", motor.nombre())),
    ];
    let distintos: Vec<String> = filas.iter().filter(|x| !x.fam.es_ets() && !matches!(x.fam, Familia::Ingenuo | Familia::IngenuoEstacional)).map(|x| x.fam.etiqueta()).collect();
    if motor != Motor::R && !distintos.is_empty() {
        sol.push(text_tone(format!("Nota: sin R, estos modelos se calculan con otra implementación y pueden diferir de los de R: {}. La suavización exponencial (ses, holt, hw, ets) y los ingenuos sí dan lo mismo que R. Para obtener exactamente los de R, instala R con el paquete forecast o usa la app de escritorio, que trae R.", distintos.join(", ")), Tone::Highlight));
    }
    if let Some(l) = lambda {
        sol.push(text(format!("Se trabajó con la transformación de Box-Cox λ = {}.", fmt(r4(l)))));
    }
    if !avisos.is_empty() {
        sol.push(text_tone(format!("Avisos: {}.", avisos.join("; ")), Tone::Muted));
    }
    let mut par = vec![];
    for x in &filas {
        if !x.a.coef.is_empty() {
            let mut t = Table::new(Some(&format!("Parámetros: {}", etiqueta(x))), &["Parámetro", "Valor"]).left(0);
            for (kk, v) in &x.a.coef {
                t.row(cells![kk.as_str(), celda(*v)]);
            }
            par.push(t.block());
        }
    }
    let pasos = if o.steps {
        let mut v = vec![
            step("Conjuntos de entrenamiento y prueba", format!("Como window(y, end = …) en R: los primeros {nt} datos entrenan los modelos y los {k} siguientes los prueban."), vec![]),
            step("Autocorrelograma", format!("r(k) del entrenamiento con bandas ±1.96/√{nt} = ±{}. {}", fmt(r4(banda)), por_que), vec![]),
        ];
        for x in &filas {
            let ex = match &x.fam {
                Familia::Ses | Familia::Holt { .. } | Familia::Hw { .. } => explicacion_suavizacion(&x.fam),
                Familia::Ingenuo => "Ingenuo, naive() de R: el pronóstico es el último valor del entrenamiento.".into(),
                Familia::IngenuoEstacional => "Ingenuo estacional, snaive() de R: el pronóstico es el valor de la misma estación del último ciclo.".into(),
                Familia::Tbats { .. } => "TBATS, tbats() de R (De Livera et al., 2011): prueba Box-Cox, tendencia, amortiguamiento, estacionalidad trigonométrica y errores ARMA, y se queda con la configuración de menor AIC.".into(),
                f => format!("{}.", f.etiqueta()),
            };
            v.push(step(&etiqueta(x), format!("{ex} RMSE en el entrenamiento {} y en la prueba {}.", fmt(r4(x.ent.rmse)), fmt(r4(x.pru.rmse))), vec![]));
        }
        v.push(step("Exactitud", "accuracy(): en el entrenamiento los errores son yₜ − ŷₜ (valores ajustados); en la prueba, los datos reales menos los pronósticos. RMSE = √(Σe²/n), MAE = Σ|e|/n, MAPE = 100·Σ|e/y|/n y MASE = MAE / (error medio del ingenuo estacional del entrenamiento).", vec![]));
        v.push(step("Residuos", "checkresiduals(): prueba de Ljung-Box de los residuos del entrenamiento; p ≥ 0.05 = parecen ruido blanco.", vec![]));
        vec![steps(v)]
    } else {
        vec![]
    };
    let modelos_json: Vec<Value> = filas
        .iter()
        .map(|x| {
            let acc = |e: &Errores| json!({"me": e.me, "rmse": e.rmse, "mae": e.mae, "mpe": e.mpe, "mape": e.mape, "mase": e.mase, "acf1": e.acf1});
            json!({
                "id": nombre_modelo(&x.fam), "label": x.fam.etiqueta(), "model": x.a.modelo,
                "accuracy": {"training": acc(&x.ent), "test": acc(&x.pru), "theil_u": x.u},
                "ljung_box": {"q": x.lb.q, "df": x.lb.gl, "p_value": x.lb.p, "lags": x.lb.retardos, "white_noise": x.lb.p >= 0.05},
                "forecast": x.a.media, "coefficients": x.a.coef.iter().map(|(kk, v)| json!({"name": kk, "value": v})).collect::<Vec<_>>(), "aic": x.a.aic, "aicc": x.a.aicc, "bic": x.a.bic,
            })
        })
        .collect();
    let head = format!("{} Suavización sugerida por el autocorrelograma: {}.", conclusion, sugerida.etiqueta());
    let mut rep = Report::new("st", &m.name, Status::Solved, head)
        .section("solucion", "Evaluación", sol)
        .section("parametros", "Parámetros", par)
        .section("pasos", "Paso a paso", pasos)
        .data(json!({
            "train_size": nt, "test_size": k, "train_end": d.nombres[nt - 1], "test_start": d.nombres[nt],
            "acf_training": r.iter().map(|x| r4(*x)).collect::<Vec<_>>(), "band": banda,
            "recommended_smoothing": nombre_modelo(&sugerida), "recommended_smoothing_label": sugerida.etiqueta(), "recommended_reason": por_que,
            "models": modelos_json,
            "best_training_rmse": mejor_ent.map(|i| nombre_modelo(&filas[i].fam)), "best_test_rmse": mejor_pru.map(|i| nombre_modelo(&filas[i].fam)),
            "engine": match motor { Motor::R => "r", Motor::Python => "python", Motor::Nativo => "nativo" }, "engine_name": motor.nombre(),
        }))
        .cite("hyndman2018fpp", "Conjuntos de entrenamiento y prueba, medidas de exactitud y revisión de residuos")
        .cite("hyndman2002ets", "Suavización exponencial (ses, holt, hw) como modelos ETS")
        .cite("hyndmankhandakar2008", "Paquete forecast de R")
        .cite("ljungbox1978", "Prueba de Ljung-Box de los residuos")
        .cite("arango2024arima", "Modelos de pronóstico en R en el curso");
    if filas.iter().any(|x| matches!(x.fam, Familia::Tbats { .. })) {
        rep = rep.cite("delivera2011", "Modelo TBATS");
    }
    rep = rep.cite_if(motor == Motor::Nativo, "anofox2026", "Motor nativo de pronósticos en Rust");
    Ok(rep)
}

fn solve_st(m: &StModel, o: &StOptions) -> Result<Report, ModError> {
    let d = datos(m)?;
    match m.problem_type.as_str() {
        "exploracion" => explorar(m, o, &d),
        "pronostico" => pronosticar(m, o, &d),
        "evaluacion" => evaluar(m, o, &d),
        t => invalid(format!("tipo de problema desconocido: «{t}» (usa exploracion, pronostico o evaluacion)")),
    }
}

pub struct St;

impl TypedModule for St {
    type Model = StModel;
    type Options = StOptions;

    fn file_tag(&self) -> &'static str {
        "IAQSB-ST"
    }

    fn spec(&self) -> ModuleSpec {
        let serie = |t: &str| Widget::records(Some(t), "data", vec![Field::text("name", "Periodo"), Field::number("value", "Valor").optional()], Some("Añadir dato"));
        let base = vec![
            Field::number("frequency", "Frecuencia (periodo estacional)").default(12.0).help("12 mensual, 4 trimestral, 7 diaria con semana, 52.18 semanal; 1 = sin estacionalidad"),
            Field::text("start", "Primer periodo").help("Año-estación, p. ej. 2021-05 para mayo de 2021 (como start = c(2021, 5) en R); nombra los periodos"),
            Field::integer("start_season", "Estación del primer dato", 1).help("Solo si no se indica el primer periodo"),
        ];
        ModuleSpec {
            id: "st".into(),
            winqsb: "Nuevo".into(),
            name: "Series de tiempo (ARIMA, ETS, TBATS)".into(),
            description: "Exploración (autocorrelación, descomposición, Ljung-Box, Box-Cox) y pronósticos modernos: ARIMA, ETS, TBATS, Theta y Croston automáticos, con comparación de modelos y el mejor motor disponible (R, Python o nativo)".into(),
            file_ext: "STS".into(),
            problem_types: vec![
                ProblemType {
                    id: "exploracion".into(),
                    label: "Exploración de la serie".into(),
                    description: "Correlograma (ACF y PACF), prueba de Ljung-Box, descomposición clásica, fuerza de la tendencia y de la estacionalidad, gráfico estacional y λ de Box-Cox".into(),
                    dims: vec![Field::integer("observations", "Número de datos", 24)],
                    layout: vec![Widget::params(Some("Serie"), base.clone()), serie("Datos")],
                },
                ProblemType {
                    id: "evaluacion".into(),
                    label: "Evaluar modelos (entrenamiento y prueba)".into(),
                    description: "Como en clase con R: conjunto de entrenamiento hasta un periodo, el autocorrelograma y la suavización que sugiere, varios modelos (ses, holt, hw, naive, snaive, tbats, auto.arima…), accuracy() en entrenamiento y prueba y checkresiduals()".into(),
                    dims: vec![Field::integer("observations", "Número de datos", 40)],
                    layout: vec![
                        Widget::params(
                            Some("Evaluación"),
                            vec![
                                Field::text("train_end", "Último periodo del entrenamiento").help("Su nombre, p. ej. 2023-12 (como window(y, end = c(2023, 12))); vacío = usar los periodos de prueba"),
                                Field::integer("test_size", "Periodos de prueba", 0).help("Si no se indica el último periodo del entrenamiento; 0 = un ciclo"),
                                Field::text("models", "Modelos").help("Separados por comas: ses, holt, hw, naive, snaive, tbats, arima, ets, theta… Vacío = la suavización que sugiere el correlograma, el ingenuo y TBATS"),
                                Field::boolean("damped", "Tendencia amortiguada (holt, hw)", false),
                                Field::choice("seasonal_type", "Estacionalidad (hw)", &[("aditiva", "Aditiva"), ("multiplicativa", "Multiplicativa")]),
                                Field::text("lambda", "λ de Box-Cox").help("Vacío = sin transformar; auto = Guerrero; 0 = logaritmo"),
                            ],
                        ),
                        Widget::params(Some("Serie"), base.clone()),
                        serie("Datos"),
                    ],
                },
                ProblemType {
                    id: "pronostico".into(),
                    label: "Pronóstico (ARIMA, ETS, TBATS…)".into(),
                    description: "Un modelo o la comparación automática de varios en un periodo de prueba, con intervalos de predicción y revisión de los residuos".into(),
                    dims: vec![Field::integer("observations", "Número de datos", 36)],
                    layout: vec![
                        Widget::params(
                            Some("Modelo"),
                            vec![
                                Field::choice(
                                    "model",
                                    "Modelo",
                                    &[
                                        ("auto", "Automático: comparar y elegir el mejor"),
                                        ("ses", "Suavización exponencial simple (ses)"),
                                        ("holt", "Holt: tendencia (holt)"),
                                        ("hw", "Holt-Winters: tendencia y estacionalidad (hw)"),
                                        ("arima", "ARIMA"),
                                        ("ets", "ETS (suavización exponencial)"),
                                        ("tbats", "TBATS (estacionalidad compleja)"),
                                        ("theta", "Theta"),
                                        ("croston", "Croston (demanda intermitente)"),
                                        ("ingenuo", "Ingenuo"),
                                        ("ingenuo_estacional", "Ingenuo estacional"),
                                    ],
                                ),
                                Field::text("arima_order", "Órdenes ARIMA").help("p,d,q o p,d,q,P,D,Q; vacío = automático (auto.arima)").show_if("model", &["arima"]),
                                Field::boolean("drift", "Con deriva", false).show_if("model", &["arima"]),
                                Field::text("ets_model", "Modelo ETS").help("MAM, AAdN, ANN…; vacío = automático").show_if("model", &["ets"]),
                                Field::boolean("damped", "Tendencia amortiguada", false).show_if("model", &["holt", "hw"]),
                                Field::choice("seasonal_type", "Estacionalidad", &[("aditiva", "Aditiva"), ("multiplicativa", "Multiplicativa")]).show_if("model", &["hw"]),
                                Field::text("seasonal_periods", "Periodos estacionales").help("Separados por comas, p. ej. 7, 365.25; vacío = la frecuencia").show_if("model", &["tbats"]),
                                Field::text("lambda", "λ de Box-Cox").help("Vacío = sin transformar; auto = Guerrero; 0 = logaritmo"),
                                Field::integer("horizon", "Periodos a pronosticar", 0).help("0 = dos ciclos o 10"),
                                Field::integer("test_size", "Periodos de prueba para comparar", 0).help("0 = el horizonte (a lo sumo la quinta parte de la serie)").show_if("model", &["auto"]),
                                Field::number("level", "Nivel de los intervalos (%)").default(95.0),
                            ],
                        ),
                        Widget::params(Some("Serie"), base),
                        serie("Datos"),
                    ],
                },
            ],
            options: vec![
                Field::boolean("steps", "Mostrar paso a paso", true),
                Field::choice("forecast_engine", "Motor de pronóstico", &[("auto", "El mejor disponible (R, Python o nativo)"), ("r", "R (paquete forecast)"), ("python", "Python (statsforecast)"), ("nativo", "Nativo de IAQSB (Rust)")]).comparable(),
                Field::integer("lags", "Retardos del correlograma", 0).help("0 = como Acf de R"),
                Field::choice("decomposition", "Descomposición", &[("aditiva", "Aditiva"), ("multiplicativa", "Multiplicativa")]),
            ],
        }
    }

    fn template(&self, pt: &str, dims: &Value) -> Result<StModel, ModError> {
        let n = dim(dims, "observations", match pt { "exploracion" => 24, "evaluacion" => 40, _ => 36 }, 4, 100000)?;
        Ok(StModel {
            name: "Serie de tiempo".into(),
            problem_type: match pt {
                "exploracion" | "evaluacion" => pt.into(),
                _ => "pronostico".into(),
            },
            time_unit: "mes".into(),
            frequency: 12.0,
            start_season: 1,
            data: (1..=n).map(|i| Obs { name: i.to_string(), value: None }).collect(),
            start: String::new(),
            model: "auto".into(),
            arima_order: String::new(),
            drift: false,
            ets_model: String::new(),
            damped: false,
            seasonal_type: String::new(),
            models: String::new(),
            train_end: String::new(),
            seasonal_periods: String::new(),
            lambda: String::new(),
            horizon: 0,
            test_size: 0,
            level: 95.0,
        })
    }

    fn solve(&self, m: &StModel, o: &StOptions) -> Result<Report, ModError> {
        solve_st(m, o)
    }

    fn read(&self, bytes: &[u8]) -> Result<StModel, ModError> {
        let s = String::from_utf8_lossy(bytes);
        let s = s.trim_start_matches('\u{feff}');
        let (head, rest) = s.split_once('\n').unwrap_or((s, ""));
        if !head.trim().to_ascii_uppercase().starts_with("IAQSB-ST") {
            return Err(ModError::Format("no es un archivo de series de tiempo de IAQSB".into()));
        }
        serde_json::from_str(rest).map_err(|e| ModError::Format(e.to_string()))
    }

    fn write(&self, m: &StModel) -> Result<Vec<u8>, ModError> {
        Ok(format!("IAQSB-ST 1\n{}\n", serde_json::to_string_pretty(m).unwrap()).into_bytes())
    }

    fn examples(&self) -> Vec<(&'static str, StModel)> {
        let ex = |s: &str| serde_json::from_str::<StModel>(s).unwrap();
        vec![
            ("utilidades_arango", ex(include_str!("ejemplos/utilidades.json"))),
            ("usnetelec_arima", ex(include_str!("ejemplos/usnetelec.json"))),
            ("h02_arima_log", ex(include_str!("ejemplos/h02.json"))),
            ("qcement_comparar", ex(include_str!("ejemplos/qcement.json"))),
            ("gas_tbats", ex(include_str!("ejemplos/gas.json"))),
            ("evaluacion_clase", ex(include_str!("ejemplos/evaluacion.json"))),
        ]
    }
}
