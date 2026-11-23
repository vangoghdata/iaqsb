//! Motores de pronóstico. Se usa el mejor que haya en el equipo, como con los
//! optimizadores (HiGHS incluido; Gurobi o CPLEX si están):
//!
//! 1. **R con el paquete forecast** (Hyndman y Khandakar, 2008): la implementación
//!    de referencia de auto.arima, ets, tbats, thetaf y croston, la que usa el curso.
//! 2. **Python con statsforecast** (Nixtla): los mismos métodos, muy rápidos.
//! 3. **Nativo** (anofox-forecast, en Rust, incluido): siempre disponible, en
//!    cualquier equipo y sin instalar nada.
//!
//! Cada motor recibe un lote de trabajos (modelo, observaciones de entrenamiento y
//! horizonte) para abrir R o Python una sola vez.

use super::analisis::{box_cox, box_cox_inv};
use anofox_forecast::core::TimeSeries;
use anofox_forecast::models::arima::{AutoARIMA, ARIMA, SARIMA};
use anofox_forecast::models::baseline::{Naive, SeasonalNaive};
use anofox_forecast::models::exponential::{AutoETS, ETSSpec, ETS};
use anofox_forecast::models::inspect::Explanation;
use anofox_forecast::models::intermittent::Croston;
use anofox_forecast::models::tbats::AutoTBATS;
use anofox_forecast::models::theta::AutoTheta;
use anofox_forecast::models::Forecaster;
use iaqsb_core::solver::external::{find_exe, python};
use serde_json::{json, Value};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::OnceLock;

#[derive(Debug, Clone, PartialEq)]
pub enum Familia {
    /// `orden` = (p, d, q, P, D, Q); None = automático (Hyndman-Khandakar).
    Arima { orden: Option<[usize; 6]>, deriva: bool },
    /// `spec` = "MAM", "AAdN"…; None = automático (menor AICc).
    Ets { spec: Option<String> },
    /// Periodos estacionales (pueden ser varios y no enteros, p. ej. 7 y 365.25).
    Tbats { periodos: Vec<f64> },
    /// ses() de R: suavización exponencial simple, ETS(A,N,N) por mínimos cuadrados.
    Ses,
    /// holt() de R: tendencia lineal (posiblemente amortiguada), ETS(A,A,N) o ETS(A,Ad,N).
    Holt { amortiguada: bool },
    /// hw() de R: Holt-Winters aditivo, ETS(A,A,A), o multiplicativo, ETS(M,A,M).
    Hw { multiplicativa: bool, amortiguada: bool },
    Theta,
    Croston,
    Ingenuo,
    IngenuoEstacional,
}

impl Familia {
    pub fn etiqueta(&self) -> String {
        match self {
            Familia::Arima { orden: None, .. } => "ARIMA automático".into(),
            Familia::Arima { orden: Some(o), .. } => format!("ARIMA({},{},{})({},{},{})", o[0], o[1], o[2], o[3], o[4], o[5]),
            Familia::Ets { spec: None } => "ETS automático".into(),
            Familia::Ets { spec: Some(s) } => format!("ETS {s}"),
            Familia::Tbats { .. } => "TBATS".into(),
            Familia::Ses => "Suavización exponencial simple".into(),
            Familia::Holt { amortiguada: false } => "Holt (tendencia lineal)".into(),
            Familia::Holt { amortiguada: true } => "Holt amortiguado".into(),
            Familia::Hw { multiplicativa, amortiguada } => format!("Holt-Winters {}{}", if *multiplicativa { "multiplicativo" } else { "aditivo" }, if *amortiguada { " amortiguado" } else { "" }),
            Familia::Theta => "Theta".into(),
            Familia::Croston => "Croston".into(),
            Familia::Ingenuo => "Ingenuo".into(),
            Familia::IngenuoEstacional => "Ingenuo estacional".into(),
        }
    }
    fn usa_lambda(&self) -> bool {
        matches!(self, Familia::Arima { .. } | Familia::Ets { .. } | Familia::Ses | Familia::Holt { .. } | Familia::Hw { multiplicativa: false, .. } | Familia::Theta | Familia::Ingenuo | Familia::IngenuoEstacional)
    }
    /// Modelo ETS equivalente de ses(), holt() y hw() (para los motores sin esas funciones).
    pub fn spec_ets(&self) -> Option<String> {
        match self {
            Familia::Ets { spec } => spec.clone(),
            Familia::Ses => Some("ANN".into()),
            Familia::Holt { amortiguada } => Some(if *amortiguada { "AAdN" } else { "AAN" }.into()),
            Familia::Hw { multiplicativa, amortiguada } => Some(format!("{}A{}{}", if *multiplicativa { "M" } else { "A" }, if *amortiguada { "d" } else { "" }, if *multiplicativa { "M" } else { "A" })),
            _ => None,
        }
    }
    /// ¿Es de la familia de la suavización exponencial (ETS)?
    pub fn es_ets(&self) -> bool {
        matches!(self, Familia::Ets { .. } | Familia::Ses | Familia::Holt { .. } | Familia::Hw { .. })
    }
}

#[derive(Debug, Clone)]
pub struct Trabajo {
    pub familia: Familia,
    /// Observaciones de entrenamiento (las primeras n).
    pub n: usize,
    pub h: usize,
}

#[derive(Debug, Clone, Default)]
pub struct Ajuste {
    /// Nombre del modelo ajustado, p. ej. "ARIMA(2,1,1)(0,1,0)[12]" o "ETS(M,Ad,M)".
    pub modelo: String,
    pub coef: Vec<(String, f64)>,
    pub aic: Option<f64>,
    pub aicc: Option<f64>,
    pub bic: Option<f64>,
    pub sigma2: Option<f64>,
    pub ajustados: Vec<f64>,
    pub media: Vec<f64>,
    pub inferior: Vec<f64>,
    pub superior: Vec<f64>,
    pub nota: Option<String>,
    /// Prueba de Ljung-Box de los residuos tal como la da checkresiduals() de R: (Q*, gl, p, retardos).
    pub ljung_box: Option<(f64, usize, f64, usize)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Motor {
    R,
    Python,
    Nativo,
}

impl Motor {
    pub fn nombre(self) -> String {
        match self {
            Motor::R => match r_info() {
                Some(r) => format!("R {} · forecast {}", r.version, r.forecast),
                None => match (r_externo(), VERSION_R_EXTERNO.lock().ok().and_then(|v| v.clone())) {
                    (Some(d), Some(v)) => format!("R {v} ({d})"),
                    (Some(d), None) => format!("R · forecast ({d})"),
                    _ => "R (forecast)".into(),
                },
            },
            Motor::Python => py_info().map(|v| format!("Python · statsforecast {v}")).unwrap_or_else(|| "Python (statsforecast)".into()),
            Motor::Nativo => "IAQSB nativo en Rust (ETS traducido de forecast; ARIMA, TBATS y Theta con anofox-forecast)".into(),
        }
    }
}

pub struct Serie<'a> {
    pub y: &'a [f64],
    /// Frecuencia (periodo estacional; 1 = sin estacionalidad).
    pub m: f64,
    /// Estación del primer dato (1..=m).
    pub inicio: usize,
    /// λ de Box-Cox (None = sin transformar).
    pub lambda: Option<f64>,
    /// Nivel de los intervalos de predicción (%).
    pub nivel: f64,
}

// ------------------------------------------------------------------ detección

pub struct RInfo {
    pub rscript: PathBuf,
    pub version: String,
    pub forecast: String,
}

fn candidatos_rscript() -> Vec<PathBuf> {
    let mut v = vec![];
    if let Ok(p) = std::env::var("IAQSB_RSCRIPT") {
        v.push(PathBuf::from(p));
    }
    if let Some(p) = find_exe("Rscript") {
        v.push(p);
    }
    if cfg!(windows) {
        for base in ["C:\\Program Files\\R", "C:\\Program Files (x86)\\R"] {
            if let Ok(rd) = std::fs::read_dir(base) {
                let mut dirs: Vec<PathBuf> = rd.flatten().map(|e| e.path()).collect();
                dirs.sort();
                for d in dirs.into_iter().rev() {
                    v.push(d.join("bin").join("Rscript.exe"));
                }
            }
        }
    } else {
        for p in ["/opt/homebrew/bin/Rscript", "/usr/local/bin/Rscript", "/usr/bin/Rscript", "/Library/Frameworks/R.framework/Resources/bin/Rscript"] {
            v.push(PathBuf::from(p));
        }
    }
    v
}

/// R con el paquete forecast, si está instalado (se busca una sola vez).
pub fn r_info() -> Option<&'static RInfo> {
    static R: OnceLock<Option<RInfo>> = OnceLock::new();
    R.get_or_init(|| {
        // IAQSB_SIN_R simula un equipo sin R; IAQSB_SIN_R_LOCAL, uno sin R instalado pero con el de la app (webR)
        if std::env::var("IAQSB_SIN_R").is_ok() || std::env::var("IAQSB_SIN_R_LOCAL").is_ok() {
            return None;
        }
        for p in candidatos_rscript() {
            if !p.is_file() {
                continue;
            }
            let out = Command::new(&p)
                .args(["--vanilla", "-e", "if (requireNamespace('forecast', quietly=TRUE)) cat('OK', paste(R.version$major, R.version$minor, sep='.'), as.character(packageVersion('forecast')))"])
                .stdin(Stdio::null())
                .stderr(Stdio::null())
                .output();
            if let Ok(o) = out {
                let s = String::from_utf8_lossy(&o.stdout);
                let w: Vec<&str> = s.split_whitespace().collect();
                if w.first() == Some(&"OK") && w.len() >= 3 {
                    return Some(RInfo { rscript: p, version: w[1].into(), forecast: w[2].into() });
                }
            }
        }
        None
    })
    .as_ref()
}

/// Ejecuta un programa de R y devuelve lo que escribió en la salida estándar.
pub type EjecutorR = Box<dyn Fn(&str) -> Result<String, String> + Send + Sync>;

static R_EXTERNO: OnceLock<(String, EjecutorR)> = OnceLock::new();
/// Versión de R y de forecast que informó el R externo («4.6.0 · forecast 9.0.2»).
static VERSION_R_EXTERNO: std::sync::Mutex<Option<String>> = std::sync::Mutex::new(None);

/// Registra un R que no está instalado en el equipo pero que la aplicación puede usar
/// (la app de escritorio trae webR: el R oficial compilado a WebAssembly, con el
/// paquete forecast, que da los mismos resultados). `descripcion` va en el nombre del motor.
pub fn registrar_r_externo(descripcion: &str, ejecutor: EjecutorR) {
    let _ = R_EXTERNO.set((descripcion.to_string(), ejecutor));
}

fn r_externo() -> Option<&'static str> {
    if std::env::var("IAQSB_SIN_R").is_ok() {
        return None;
    }
    R_EXTERNO.get().map(|(d, _)| d.as_str())
}

/// ¿Hay R con forecast (instalado o el de la aplicación)?
pub fn r_disponible() -> bool {
    r_info().is_some() || r_externo().is_some()
}

/// Ejecuta un programa de R con el R instalado o, si no hay, con el de la aplicación.
pub fn ejecutar_r(programa: &str) -> Result<String, String> {
    if let Some(info) = r_info() {
        let dir = std::env::temp_dir().join(format!("iaqsb-r-{}-{}", std::process::id(), std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0)));
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let script = dir.join("programa.R");
        std::fs::write(&script, programa).map_err(|e| e.to_string())?;
        let out = Command::new(&info.rscript).arg("--vanilla").arg(&script).current_dir(&dir).stdin(Stdio::null()).output().map_err(|e| format!("no se pudo ejecutar R: {e}"))?;
        let _ = std::fs::remove_dir_all(&dir);
        let txt = String::from_utf8_lossy(&out.stdout).to_string();
        if !out.status.success() && txt.trim().is_empty() {
            let e = String::from_utf8_lossy(&out.stderr);
            return Err(format!("R terminó con error: {}", e.lines().filter(|l| !l.trim().is_empty()).last().unwrap_or("sin detalle")));
        }
        return Ok(txt);
    }
    match (r_externo(), R_EXTERNO.get()) {
        (Some(_), Some((_, f))) => f(programa),
        _ => Err("R con el paquete forecast no está instalado".into()),
    }
}

/// Versión de statsforecast si está instalado en el Python del equipo.
pub fn py_info() -> Option<&'static str> {
    static P: OnceLock<Option<String>> = OnceLock::new();
    P.get_or_init(|| {
        if std::env::var("IAQSB_SIN_PYTHON").is_ok() {
            return None;
        }
        let (py, pre) = python()?;
        let o = Command::new(py).args(pre).args(["-c", "import statsforecast; print(statsforecast.__version__)"]).stdin(Stdio::null()).stderr(Stdio::null()).output().ok()?;
        o.status.success().then(|| String::from_utf8_lossy(&o.stdout).trim().to_string()).filter(|s| !s.is_empty())
    })
    .as_deref()
}

/// Motores disponibles, del mejor al de respaldo.
pub fn disponibles() -> Vec<Motor> {
    let mut v = vec![];
    if r_disponible() {
        v.push(Motor::R);
    }
    if py_info().is_some() {
        v.push(Motor::Python);
    }
    v.push(Motor::Nativo);
    v
}

/// Ajusta un lote de trabajos con el motor indicado.
pub fn ajustar(motor: Motor, s: &Serie, trabajos: &[Trabajo]) -> Result<Vec<Result<Ajuste, String>>, String> {
    match motor {
        Motor::R => con_r(s, trabajos),
        Motor::Python => con_python(s, trabajos),
        Motor::Nativo => Ok(trabajos.iter().map(|t| nativo(s, t)).collect()),
    }
}

// ------------------------------------------------------------------ nativo

fn serie_anofox(y: &[f64]) -> Result<TimeSeries, String> {
    use chrono::{Duration, TimeZone, Utc};
    let t0 = Utc.with_ymd_and_hms(2000, 1, 1, 0, 0, 0).unwrap();
    let ts = (0..y.len()).map(|i| t0 + Duration::days(i as i64)).collect();
    TimeSeries::univariate(ts, y.to_vec()).map_err(|e| e.to_string())
}

fn limpiar(v: &[f64]) -> Vec<f64> {
    v.iter().map(|x| if x.is_finite() { *x } else { f64::NAN }).collect()
}

/// Valores ajustados en la escala de la serie. anofox-forecast devuelve los del
/// ARIMA en la escala diferenciada (n − d valores) y en el SARIMA no los da; los
/// residuos de un paso sí sirven en las dos escalas: ajustado = y − residuo.
/// Los residuos iniciales exactamente en 0 son el arranque del filtro: quedan vacíos.
fn ajustados_en_escala(y: &[f64], ajust: Option<&[f64]>, res: Option<&[f64]>) -> Vec<f64> {
    let n = y.len();
    if let Some(f) = ajust.filter(|f| f.len() == n && f.iter().any(|v| v.is_finite())) {
        return limpiar(f);
    }
    if let Some(r) = res.filter(|r| !r.is_empty() && r.len() <= n) {
        let off = n - r.len();
        let arranque = r.iter().take_while(|v| **v == 0.0 || !v.is_finite()).count();
        return (0..n).map(|t| if t < off + arranque || !r[t - off].is_finite() { f64::NAN } else { y[t] - r[t - off] }).collect();
    }
    vec![f64::NAN; n]
}

fn nativo(s: &Serie, t: &Trabajo) -> Result<Ajuste, String> {
    let m = s.m.round().max(1.0) as usize;
    let lam = if t.familia.usa_lambda() { s.lambda } else { None };
    let y: Vec<f64> = s.y[..t.n].iter().map(|v| lam.map_or(*v, |l| box_cox(*v, l))).collect();
    // suavización exponencial: la traducción de ets() de forecast (los mismos números que R)
    if t.familia.es_ets() {
        match ets_como_r(&y, m, &t.familia, t.h, s.nivel) {
            Ok(mut a) => {
                if let Some(l) = lam {
                    for v in a.ajustados.iter_mut().chain(a.media.iter_mut()).chain(a.inferior.iter_mut()).chain(a.superior.iter_mut()) {
                        if v.is_finite() {
                            *v = box_cox_inv(*v, l);
                        }
                    }
                }
                return Ok(a);
            }
            Err(e) if matches!(t.familia, Familia::Ets { spec: None }) => return Err(e),
            Err(_) => {}
        }
    }
    let ts = serie_anofox(&y)?;
    let nivel = s.nivel / 100.0;
    let mut nota = None;
    let mut modelo: Box<dyn Forecaster> = match &t.familia {
        Familia::Arima { orden: None, .. } => Box::new(if m > 1 { AutoARIMA::seasonal(m) } else { AutoARIMA::new() }),
        Familia::Arima { orden: Some(o), deriva } => {
            if *deriva {
                nota = Some("el motor nativo no incluye la deriva; usa R para ARIMA con deriva".into());
            }
            if m > 1 && (o[3] + o[4] + o[5]) > 0 {
                Box::new(SARIMA::new(o[0], o[1], o[2], o[3], o[4], o[5], m))
            } else {
                Box::new(ARIMA::new(o[0], o[1], o[2]))
            }
        }
        Familia::Ets { spec: None } => Box::new(if m > 1 { AutoETS::with_period(m) } else { AutoETS::non_seasonal() }),
        Familia::Ets { spec: Some(_) } | Familia::Ses | Familia::Holt { .. } | Familia::Hw { .. } => {
            let sp = t.familia.spec_ets().unwrap_or_default();
            let spec = ETSSpec::from_notation(&sp).map_err(|e| format!("modelo ETS «{sp}» no válido: {e}"))?;
            Box::new(ETS::new(spec, m))
        }
        Familia::Tbats { periodos } => {
            let p: Vec<usize> = periodos.iter().map(|x| x.round().max(2.0) as usize).collect();
            nota = Some(if periodos.iter().any(|x| x.fract() != 0.0) {
                "el TBATS del motor nativo (anofox-forecast) no es el de R y redondea los periodos no enteros: los valores pueden diferir de los de R".into()
            } else {
                "el TBATS del motor nativo (anofox-forecast) no es el de R: los valores pueden diferir de los de R".into()
            });
            Box::new(AutoTBATS::new(p))
        }
        Familia::Theta => Box::new(if m > 1 { AutoTheta::seasonal(m) } else { AutoTheta::new() }),
        Familia::Croston => Box::new(Croston::new()),
        Familia::Ingenuo => Box::new(Naive::new()),
        Familia::IngenuoEstacional => Box::new(SeasonalNaive::new(m.max(1))),
    };
    modelo.fit(&ts).map_err(|e| e.to_string())?;
    let fc = modelo.predict_with_intervals(t.h, nivel).or_else(|_| modelo.predict(t.h)).map_err(|e| e.to_string())?;
    let media = fc.primary().to_vec();
    let inferior = fc.lower_series(0).map(|x| x.to_vec()).unwrap_or_else(|_| vec![f64::NAN; t.h]);
    let superior = fc.upper_series(0).map(|x| x.to_vec()).unwrap_or_else(|_| vec![f64::NAN; t.h]);
    let ajustados = ajustados_en_escala(&y, modelo.fitted_values(), modelo.residuals());
    let mut a = Ajuste { ajustados, media, inferior, superior, nota, ..Default::default() };
    let n = t.n as f64;
    match modelo.explanation() {
        Ok(Explanation::Arima(e)) => {
            a.modelo = match e.seasonal_order {
                Some((p, d, q, s)) if p + d + q > 0 => format!("ARIMA({},{},{})({},{},{})[{}]", e.order.0, e.order.1, e.order.2, p, d, q, s),
                _ => format!("ARIMA({},{},{})", e.order.0, e.order.1, e.order.2),
            };
            // los mismos nombres que coef() de R: ar1…, ma1…, sar1…, sma1…
            let (p, q) = (e.order.0, e.order.2);
            let (sp, sq) = e.seasonal_order.map(|(a, _, c, _)| (a, c)).unwrap_or((0, 0));
            let mut nombres: Vec<String> = (1..=p).map(|i| format!("ar{i}")).chain((1..=q).map(|i| format!("ma{i}"))).chain((1..=sp).map(|i| format!("sar{i}"))).chain((1..=sq).map(|i| format!("sma{i}"))).collect();
            if e.coefficients.len() == nombres.len() + 1 {
                nombres.push("intercept".into());
            }
            a.coef = e.coefficients.iter().enumerate().map(|(i, c)| (nombres.get(i).cloned().unwrap_or_else(|| format!("coef{}", i + 1)), *c)).collect();
            let k = e.coefficients.len() as f64 + 1.0;
            a.aic = Some(e.aic).filter(|x| x.is_finite());
            a.bic = Some(e.bic).filter(|x| x.is_finite());
            a.aicc = a.aic.map(|aic| aic + 2.0 * k * (k + 1.0) / (n - k - 1.0));
        }
        Ok(Explanation::Ets(e)) => {
            a.modelo = format!("ETS({})", e.spec.trim_start_matches("ETS(").trim_end_matches(')'));
            a.coef.push(("alpha".into(), e.alpha));
            if let Some(b) = e.beta {
                a.coef.push(("beta".into(), b));
            }
            if let Some(g) = e.gamma {
                a.coef.push(("gamma".into(), g));
            }
            if let Some(p) = e.phi {
                a.coef.push(("phi".into(), p));
            }
        }
        Ok(Explanation::Theta(e)) => {
            a.modelo = format!("Theta ({})", e.variant);
            a.coef.push(("theta".into(), e.theta));
            if let Some(al) = e.alpha {
                a.coef.push(("alpha".into(), al));
            }
        }
        Ok(Explanation::Tbats(e)) => {
            a.modelo = format!("TBATS {}", e.selected_config);
            if let Some(l) = e.box_cox_lambda {
                a.coef.push(("lambda".into(), l));
            }
            a.aic = Some(e.aic).filter(|x| x.is_finite());
        }
        _ => a.modelo = t.familia.etiqueta(),
    }
    // residuos para σ²
    if t.familia.es_ets() {
        a.nota = Some("ETS con anofox-forecast: los valores pueden diferir de los de R".into());
    }
    let res: Vec<f64> = y.iter().zip(&a.ajustados).filter(|(u, v)| u.is_finite() && v.is_finite()).map(|(u, v)| u - v).collect();
    if !res.is_empty() {
        a.sigma2 = Some(res.iter().map(|r| r * r).sum::<f64>() / res.len() as f64);
    }
    if let Some(l) = lam {
        for v in a.ajustados.iter_mut().chain(a.media.iter_mut()).chain(a.inferior.iter_mut()).chain(a.superior.iter_mut()) {
            if v.is_finite() {
                *v = box_cox_inv(*v, l);
            }
        }
    }
    Ok(a)
}

/// ses(), holt(), hw() y ets() como en R (ver ets.rs).
fn ets_como_r(y: &[f64], m: usize, fam: &Familia, h: usize, nivel: f64) -> Result<Ajuste, String> {
    use super::ets::{ets, pronosticar, Criterio};
    let (modelo, amort, crit) = match fam {
        Familia::Ses => ("ANN".to_string(), Some(false), Criterio::Mse),
        Familia::Holt { amortiguada } => ("AAN".to_string(), Some(*amortiguada), Criterio::Mse),
        Familia::Hw { multiplicativa, amortiguada } => ((if *multiplicativa { "MAM" } else { "AAA" }).to_string(), Some(*amortiguada), Criterio::Mse),
        Familia::Ets { spec: None } => ("ZZZ".to_string(), None, Criterio::Lik),
        Familia::Ets { spec: Some(sp) } => {
            let u = sp.to_uppercase();
            let d = u.contains("AD") || u.contains("MD");
            (u.replace('D', ""), Some(d), Criterio::Lik)
        }
        _ => return Err("no es un modelo ETS".into()),
    };
    let f = ets(y, m, &modelo, amort, crit)?;
    let (media, inferior, superior) = pronosticar(&f, h, nivel);
    // checkresiduals() de forecast 9: residuos del modelo, 2m retardos (10 sin estacionalidad), a lo sumo n/5, sin restar parámetros
    let k = super::analisis::retardos_residuos(f.residuos.len(), m).max(3).min(f.residuos.len().saturating_sub(1)).max(1);
    let lb = super::analisis::ljung_box(&f.residuos, k, 0);
    let nombre = match fam {
        Familia::Ses => "Simple exponential smoothing".to_string(),
        Familia::Holt { amortiguada } => (if *amortiguada { "Damped Holt's method" } else { "Holt's method" }).to_string(),
        Familia::Hw { multiplicativa, amortiguada } => format!("{}Holt-Winters' {} method", if *amortiguada { "Damped " } else { "" }, if *multiplicativa { "multiplicative" } else { "additive" }),
        _ => f.metodo(),
    };
    Ok(Ajuste {
        modelo: nombre,
        coef: f.par.clone(),
        aic: Some(f.aic),
        aicc: Some(f.aicc),
        bic: Some(f.bic),
        sigma2: Some(f.sigma2),
        ajustados: f.ajustados.clone(),
        media,
        inferior,
        superior,
        nota: None,
        ljung_box: Some((lb.q, k, lb.p, k)),
    })
}

// ------------------------------------------------------------------ R

fn r_num(x: f64) -> String {
    if x.is_finite() {
        format!("{x:?}")
    } else {
        "NA".into()
    }
}

fn r_script(s: &Serie, trabajos: &[Trabajo]) -> String {
    let lam = match s.lambda {
        Some(l) => r_num(l),
        None => "NULL".into(),
    };
    let mut r = format!(
        r#"suppressMessages(library(forecast))
x <- c({})
num <- function(v) paste(sprintf("%.12g", as.numeric(v)), collapse = " ")
emit <- function(k, v) cat(k, "\t", v, "\n", sep = "")
cat("@@info\t", paste(R.version$major, R.version$minor, sep = "."), " · forecast ", as.character(packageVersion("forecast")), "\n", sep = "")
fin <- function(fit, fc, modelo, cf, ic, s2) {{
  emit("modelo", modelo)
  if (length(cf) > 0) emit("coef", paste(names(cf), sprintf("%.12g", cf), sep = "=", collapse = " "))
  emit("ic", num(ic)); emit("sigma2", num(s2))
  emit("ajustados", num(fc$fitted)); emit("media", num(fc$mean))
  if (!is.null(fc$lower) && length(fc$lower) > 0) {{ emit("inferior", num(fc$lower[, 1])); emit("superior", num(fc$upper[, 1])) }}
  # la misma prueba de Ljung-Box que checkresiduals()
  invisible(capture.output(lb <- tryCatch(checkresiduals(fc, plot = FALSE, test = "LB"), error = function(e) NULL)))
  if (!is.null(lb)) emit("ljung", paste(num(lb$statistic), num(lb$parameter), num(lb$p.value), num(lb$parameter + if (is.null(fit) || !inherits(fit, "Arima")) 0 else length(coef(fit)))))
}}
m <- {}; s0 <- {}; lv <- {}; L <- {}
"#,
        s.y.iter().map(|v| r_num(*v)).collect::<Vec<_>>().join(", "),
        r_num(s.m),
        s.inicio.max(1),
        r_num(s.nivel),
        lam
    );
    for (i, t) in trabajos.iter().enumerate() {
        let n = t.n;
        let h = t.h;
        let ajuste = match &t.familia {
            Familia::Arima { orden: None, .. } => "fit <- auto.arima(y, lambda = L); fc <- forecast(fit, h = H, level = lv); fin(fit, fc, as.character(fit), coef(fit), c(fit$aic, fit$aicc, fit$bic), fit$sigma2)".to_string(),
            Familia::Arima { orden: Some(o), deriva } => format!(
                "fit <- Arima(y, order = c({},{},{}), seasonal = c({},{},{}), include.drift = {}, lambda = L); fc <- forecast(fit, h = H, level = lv); fin(fit, fc, as.character(fit), coef(fit), c(fit$aic, fit$aicc, fit$bic), fit$sigma2)",
                o[0], o[1], o[2], o[3], o[4], o[5], if *deriva { "TRUE" } else { "FALSE" }
            ),
            Familia::Ets { spec } => {
                let (model, damped) = match spec.as_deref() {
                    None => ("ZZZ".to_string(), "NULL"),
                    Some(sp) => {
                        let u = sp.to_uppercase();
                        if u.contains("AD") || u.contains("MD") {
                            (u.replace('D', ""), "TRUE")
                        } else {
                            (u, "FALSE")
                        }
                    }
                };
                format!("fit <- ets(y, model = \"{model}\", damped = {damped}, lambda = L); fc <- forecast(fit, h = H, level = lv); fin(fit, fc, fit$method, fit$par, c(fit$aic, fit$aicc, fit$bic), fit$sigma2)")
            }
            Familia::Tbats { periodos } => {
                let p: Vec<String> = periodos.iter().map(|x| r_num(*x)).collect();
                format!("y <- msts(x[1:{n}], seasonal.periods = c({})); fit <- tbats(y, use.parallel = FALSE); fc <- forecast(fit, h = H, level = lv); fin(fit, fc, as.character(fit), unlist(list(lambda = fit$lambda, alpha = fit$alpha, beta = fit$beta, phi = fit$damping.parameter)), c(fit$AIC, NA, NA), fit$variance)", p.join(", "))
            }
            Familia::Ses => "fc <- ses(y, h = H, level = lv, lambda = L); fit <- fc$model; fin(fit, fc, fit$method, fit$par, c(fit$aic, fit$aicc, fit$bic), fit$sigma2)".into(),
            Familia::Holt { amortiguada } => format!(
                "fc <- holt(y, h = H, damped = {}, level = lv, lambda = L); fit <- fc$model; fin(fit, fc, fit$method, fit$par, c(fit$aic, fit$aicc, fit$bic), fit$sigma2)",
                if *amortiguada { "TRUE" } else { "FALSE" }
            ),
            Familia::Hw { multiplicativa, amortiguada } => format!(
                "fc <- hw(y, h = H, seasonal = \"{}\", damped = {}, level = lv{}); fit <- fc$model; fin(fit, fc, fit$method, fit$par, c(fit$aic, fit$aicc, fit$bic), fit$sigma2)",
                if *multiplicativa { "multiplicative" } else { "additive" },
                if *amortiguada { "TRUE" } else { "FALSE" },
                if *multiplicativa { "" } else { ", lambda = L" }
            ),
            Familia::Theta => "fc <- thetaf(y, h = H, level = lv); fin(NULL, fc, \"Theta\", numeric(0), c(NA, NA, NA), NA)".into(),
            Familia::Croston => "fc <- croston(y, h = H); fin(NULL, fc, \"Croston\", c(alpha = 0.1), c(NA, NA, NA), NA)".into(),
            Familia::Ingenuo => "fc <- naive(y, h = H, level = lv, lambda = L); fin(NULL, fc, \"Ingenuo\", numeric(0), c(NA, NA, NA), NA)".into(),
            Familia::IngenuoEstacional => "fc <- snaive(y, h = H, level = lv, lambda = L); fin(NULL, fc, \"Ingenuo estacional\", numeric(0), c(NA, NA, NA), NA)".into(),
        };
        r.push_str(&format!(
            "cat(\"@@{i}\\n\")\ntryCatch({{ H <- {h}; y <- ts(x[1:{n}], frequency = m, start = c(1, s0)); {ajuste} }}, error = function(e) emit(\"error\", gsub(\"[\\t\\n]\", \" \", conditionMessage(e))))\n"
        ));
    }
    r
}

fn nums(s: &str) -> Vec<f64> {
    s.split_whitespace().map(|x| x.parse::<f64>().unwrap_or(f64::NAN)).collect()
}

fn con_r(s: &Serie, trabajos: &[Trabajo]) -> Result<Vec<Result<Ajuste, String>>, String> {
    let txt = ejecutar_r(&r_script(s, trabajos))?;
    let mut res: Vec<Result<Ajuste, String>> = vec![Err("R no devolvió resultado".into()); trabajos.len()];
    let mut cur: Option<(usize, Ajuste, Option<String>)> = None;
    let guardar = |c: Option<(usize, Ajuste, Option<String>)>, res: &mut Vec<Result<Ajuste, String>>| {
        if let Some((i, a, e)) = c {
            if i < res.len() {
                res[i] = match e {
                    Some(e) => Err(e),
                    None => Ok(a),
                };
            }
        }
    };
    for linea in txt.lines() {
        // la versión de R va antes que los trabajos (@@0, @@1…)
        if let Some(v) = linea.strip_prefix("@@info\t") {
            if r_info().is_none() {
                if let Ok(mut g) = VERSION_R_EXTERNO.lock() {
                    *g = Some(v.trim().to_string());
                }
            }
            continue;
        }
        if let Some(i) = linea.strip_prefix("@@") {
            guardar(cur.take(), &mut res);
            cur = i.trim().parse().ok().map(|i| (i, Ajuste::default(), None));
            continue;
        }
        let Some((_, a, err)) = cur.as_mut() else { continue };
        let (k, v) = linea.split_once('\t').unwrap_or((linea, ""));
        match k {
            "modelo" => a.modelo = v.trim().to_string(),
            "coef" => a.coef = v.split_whitespace().filter_map(|p| p.split_once('=').map(|(n, x)| (n.to_string(), x.parse().unwrap_or(f64::NAN)))).collect(),
            "ic" => {
                let x = nums(v);
                let f = |i: usize| x.get(i).copied().filter(|v| v.is_finite());
                (a.aic, a.aicc, a.bic) = (f(0), f(1), f(2));
            }
            "sigma2" => a.sigma2 = nums(v).first().copied().filter(|v| v.is_finite()),
            "ajustados" => a.ajustados = nums(v),
            "media" => a.media = nums(v),
            "inferior" => a.inferior = nums(v),
            "superior" => a.superior = nums(v),
            "ljung" => {
                let x = nums(v);
                if x.len() >= 3 && x[0].is_finite() {
                    a.ljung_box = Some((x[0], x[1] as usize, x[2], x.get(3).copied().unwrap_or(x[1]) as usize));
                }
            }
            "error" => *err = Some(format!("R: {}", v.trim())),
            _ => {}
        }
    }
    guardar(cur.take(), &mut res);
    if res.iter().all(|r| r.is_err()) && !txt.contains("@@") {
        return Err(format!("R no devolvió resultados: {}", txt.lines().filter(|l| !l.trim().is_empty()).last().unwrap_or("sin detalle")));
    }
    for (r, t) in res.iter_mut().zip(trabajos) {
        if let Ok(a) = r {
            if a.inferior.len() != t.h {
                a.inferior = vec![f64::NAN; t.h];
                a.superior = vec![f64::NAN; t.h];
            }
        }
    }
    Ok(res)
}

// ------------------------------------------------------------------ Python

const PUENTE_PY: &str = include_str!("puente_statsforecast.py");

fn con_python(s: &Serie, trabajos: &[Trabajo]) -> Result<Vec<Result<Ajuste, String>>, String> {
    let (py, pre) = python().ok_or("no hay Python en este equipo")?;
    py_info().ok_or("statsforecast no está instalado (pip install statsforecast)")?;
    let path = std::env::temp_dir().join(format!("iaqsb-statsforecast-{}.py", env!("CARGO_PKG_VERSION")));
    if std::fs::read_to_string(&path).ok().as_deref() != Some(PUENTE_PY) {
        std::fs::write(&path, PUENTE_PY).map_err(|e| e.to_string())?;
    }
    let jobs: Vec<Value> = trabajos
        .iter()
        .map(|t| {
            let (fam, extra) = match &t.familia {
                Familia::Arima { orden, deriva } => ("arima", json!({"orden": orden, "deriva": deriva})),
                Familia::Ets { .. } | Familia::Ses | Familia::Holt { .. } | Familia::Hw { .. } => ("ets", json!({"spec": t.familia.spec_ets()})),
                Familia::Tbats { periodos } => ("tbats", json!({"periodos": periodos})),
                Familia::Theta => ("theta", json!({})),
                Familia::Croston => ("croston", json!({})),
                Familia::Ingenuo => ("ingenuo", json!({})),
                Familia::IngenuoEstacional => ("ingenuo_estacional", json!({})),
            };
            json!({"familia": fam, "n": t.n, "h": t.h, "extra": extra})
        })
        .collect();
    let entrada = json!({"y": s.y, "m": s.m, "nivel": s.nivel, "lambda": s.lambda, "trabajos": jobs}).to_string();
    let mut child = Command::new(py).args(pre).arg(&path).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().map_err(|e| e.to_string())?;
    {
        use std::io::Write;
        child.stdin.take().unwrap().write_all(entrada.as_bytes()).map_err(|e| e.to_string())?;
    }
    let out = child.wait_with_output().map_err(|e| e.to_string())?;
    let v: Value = serde_json::from_slice(&out.stdout).map_err(|_| format!("Python: {}", String::from_utf8_lossy(&out.stderr).lines().last().unwrap_or("sin respuesta")))?;
    let arr = v.as_array().ok_or("respuesta de Python no válida")?;
    let f = |x: &Value| -> Vec<f64> { x.as_array().map(|a| a.iter().map(|v| v.as_f64().unwrap_or(f64::NAN)).collect()).unwrap_or_default() };
    Ok(arr
        .iter()
        .map(|r| {
            if let Some(e) = r.get("error").and_then(|e| e.as_str()) {
                return Err(format!("Python: {e}"));
            }
            Ok(Ajuste {
                modelo: r["modelo"].as_str().unwrap_or("").to_string(),
                coef: r["coef"].as_object().map(|o| o.iter().map(|(k, v)| (k.clone(), v.as_f64().unwrap_or(f64::NAN))).collect()).unwrap_or_default(),
                aic: r["aic"].as_f64(),
                aicc: r["aicc"].as_f64(),
                bic: r["bic"].as_f64(),
                sigma2: r["sigma2"].as_f64(),
                ajustados: f(&r["ajustados"]),
                media: f(&r["media"]),
                inferior: f(&r["inferior"]),
                superior: f(&r["superior"]),
                nota: None,
                ljung_box: None,
            })
        })
        .collect())
}
