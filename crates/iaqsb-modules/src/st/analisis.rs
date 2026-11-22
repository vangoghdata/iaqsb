//! Análisis exploratorio de series de tiempo, con las mismas definiciones que R
//! (paquetes stats y forecast) para que los resultados coincidan:
//! autocorrelación (`acf`), autocorrelación parcial (`pacf`, Durbin-Levinson),
//! prueba de Ljung-Box (`Box.test`), descomposición clásica (`decompose`),
//! fuerza de la tendencia y de la estacionalidad (Hyndman y Athanasopoulos,
//! 2018, sec. 6.7), λ de Box-Cox por el método de Guerrero (`BoxCox.lambda`) y
//! medidas de error (`accuracy`).

use statrs::distribution::{ChiSquared, ContinuousCDF};

fn media(x: &[f64]) -> f64 {
    x.iter().sum::<f64>() / x.len() as f64
}

fn varianza(x: &[f64]) -> f64 {
    let v: Vec<f64> = x.iter().copied().filter(|v| v.is_finite()).collect();
    if v.len() < 2 {
        return 0.0;
    }
    let m = media(&v);
    v.iter().map(|a| (a - m).powi(2)).sum::<f64>() / (v.len() - 1) as f64
}

/// Autocorrelaciones r(1..=k) como en `acf()` de R (denominador n, serie centrada).
pub fn acf(x: &[f64], k: usize) -> Vec<f64> {
    let n = x.len();
    let m = media(x);
    let c0: f64 = x.iter().map(|v| (v - m).powi(2)).sum();
    (1..=k.min(n.saturating_sub(1)))
        .map(|h| if c0 == 0.0 { 0.0 } else { (0..n - h).map(|t| (x[t] - m) * (x[t + h] - m)).sum::<f64>() / c0 })
        .collect()
}

/// Autocorrelaciones parciales por la recursión de Durbin-Levinson (`pacf()` de R).
pub fn pacf(x: &[f64], k: usize) -> Vec<f64> {
    let r = acf(x, k);
    let k = r.len();
    let mut out = Vec::with_capacity(k);
    let mut phi: Vec<f64> = vec![];
    let mut v = 1.0;
    for h in 1..=k {
        let num = r[h - 1] - (1..h).map(|j| phi[j - 1] * r[h - j - 1]).sum::<f64>();
        let a = if v == 0.0 { 0.0 } else { num / v };
        let mut nuevo = vec![0.0; h];
        for j in 1..h {
            nuevo[j - 1] = phi[j - 1] - a * phi[h - j - 1];
        }
        nuevo[h - 1] = a;
        v *= 1.0 - a * a;
        phi = nuevo;
        out.push(a);
    }
    out
}

/// Número de retardos por defecto de `Acf()` del paquete forecast: 10·log10(n), y al menos 2m en series estacionales.
pub fn retardos_por_defecto(n: usize, m: usize) -> usize {
    let base = (10.0 * (n as f64).log10()).floor() as usize;
    let k = if m > 1 { base.max(2 * m) } else { base };
    k.clamp(1, n.saturating_sub(1).max(1))
}

pub struct LjungBox {
    pub q: f64,
    pub gl: usize,
    pub p: f64,
    pub retardos: usize,
}

/// Prueba de Ljung-Box: Q = n(n+2) Σ r²(k)/(n−k), con `gl` = retardos − parámetros del modelo.
pub fn ljung_box(x: &[f64], retardos: usize, param: usize) -> LjungBox {
    let n = x.len() as f64;
    let r = acf(x, retardos);
    let q = n * (n + 2.0) * r.iter().enumerate().map(|(k, rk)| rk * rk / (n - (k + 1) as f64)).sum::<f64>();
    let gl = retardos.saturating_sub(param).max(1);
    let p = ChiSquared::new(gl as f64).map(|c| 1.0 - c.cdf(q)).unwrap_or(f64::NAN);
    LjungBox { q, gl, p, retardos }
}

/// Retardos de `checkresiduals()`: 10 en series sin estacionalidad, 2m con estacionalidad, a lo sumo n/5.
pub fn retardos_residuos(n: usize, m: usize) -> usize {
    let k = if m > 1 { 2 * m } else { 10 };
    k.min(n / 5).max(1)
}

pub struct Descomposicion {
    pub tendencia: Vec<f64>,
    pub estacional: Vec<f64>,
    pub aleatorio: Vec<f64>,
    /// Índices estacionales (uno por estación, empezando en la estación 1).
    pub indices: Vec<f64>,
    pub multiplicativa: bool,
}

/// Descomposición clásica como `decompose()` de R: tendencia por promedio móvil
/// centrado (2×m si m es par), índices como promedio de cada estación del
/// componente sin tendencia, normalizados (suma 0 o promedio 1).
/// `inicio` es la estación del primer dato (1..=m).
pub fn descomponer(x: &[f64], m: usize, inicio: usize, multiplicativa: bool) -> Option<Descomposicion> {
    let n = x.len();
    if m < 2 || n < 2 * m {
        return None;
    }
    let pesos: Vec<f64> = if m % 2 == 0 {
        let mut w = vec![1.0 / m as f64; m + 1];
        w[0] = 0.5 / m as f64;
        w[m] = 0.5 / m as f64;
        w
    } else {
        vec![1.0 / m as f64; m]
    };
    let mitad = pesos.len() / 2;
    let mut tendencia = vec![f64::NAN; n];
    for t in mitad..n.saturating_sub(pesos.len() - 1 - mitad) {
        tendencia[t] = pesos.iter().enumerate().map(|(j, w)| w * x[t + j - mitad]).sum();
    }
    let sin_tend: Vec<f64> = (0..n).map(|t| if multiplicativa { x[t] / tendencia[t] } else { x[t] - tendencia[t] }).collect();
    let estacion = |t: usize| (t + inicio - 1) % m;
    let mut indices = vec![0.0; m];
    for (s, idx) in indices.iter_mut().enumerate() {
        let v: Vec<f64> = (0..n).filter(|&t| estacion(t) == s && sin_tend[t].is_finite()).map(|t| sin_tend[t]).collect();
        *idx = if v.is_empty() { f64::NAN } else { media(&v) };
    }
    let prom = media(&indices);
    for v in indices.iter_mut() {
        *v = if multiplicativa { *v / prom } else { *v - prom };
    }
    let estacional: Vec<f64> = (0..n).map(|t| indices[estacion(t)]).collect();
    let aleatorio: Vec<f64> = (0..n).map(|t| if multiplicativa { x[t] / (estacional[t] * tendencia[t]) } else { x[t] - estacional[t] - tendencia[t] }).collect();
    Some(Descomposicion { tendencia, estacional, aleatorio, indices, multiplicativa })
}

/// Fuerza de la tendencia y de la estacionalidad (0 = nada, 1 = máxima).
pub fn fuerza(d: &Descomposicion) -> (f64, f64) {
    let n = d.tendencia.len();
    let (tr, sr): (Vec<f64>, Vec<f64>) = if d.multiplicativa {
        ((0..n).map(|t| d.tendencia[t] * d.aleatorio[t]).collect(), (0..n).map(|t| d.estacional[t] * d.aleatorio[t]).collect())
    } else {
        ((0..n).map(|t| d.tendencia[t] + d.aleatorio[t]).collect(), (0..n).map(|t| d.estacional[t] + d.aleatorio[t]).collect())
    };
    let vr = varianza(&d.aleatorio);
    let f = |v: f64| if v > 0.0 { (1.0 - vr / v).max(0.0) } else { 0.0 };
    (f(varianza(&tr)), f(varianza(&sr)))
}

/// Transformación de Box-Cox (λ = 0: logaritmo).
pub fn box_cox(x: f64, lambda: f64) -> f64 {
    if lambda == 0.0 {
        x.ln()
    } else {
        (x.signum() * x.abs().powf(lambda) - 1.0) / lambda
    }
}

/// Inversa de Box-Cox.
pub fn box_cox_inv(y: f64, lambda: f64) -> f64 {
    if lambda == 0.0 {
        y.exp()
    } else {
        let z = lambda * y + 1.0;
        z.signum() * z.abs().powf(1.0 / lambda)
    }
}

/// λ de Box-Cox por el método de Guerrero (1993), como `BoxCox.lambda()` (en [−1, 2]).
pub fn lambda_guerrero(x: &[f64], m: usize) -> Option<f64> {
    if x.iter().any(|v| *v <= 0.0) {
        return None;
    }
    let p = m.max(2);
    let grupos = x.len() / p;
    if grupos < 2 {
        return None;
    }
    // como R: se usan los últimos `grupos` periodos completos
    let ini = x.len() - grupos * p;
    let cv = |lam: f64| {
        let razones: Vec<f64> = (0..grupos)
            .map(|g| {
                let s = &x[ini + g * p..ini + (g + 1) * p];
                let mu = media(s);
                varianza(s).sqrt() / mu.powf(1.0 - lam)
            })
            .collect();
        varianza(&razones).sqrt() / media(&razones)
    };
    // búsqueda de sección dorada en [−1, 2], como optimize() de R
    let (mut a, mut b) = (-1.0f64, 2.0f64);
    let g = (5f64.sqrt() - 1.0) / 2.0;
    let (mut c, mut d) = (b - g * (b - a), a + g * (b - a));
    for _ in 0..200 {
        if cv(c) < cv(d) {
            b = d;
        } else {
            a = c;
        }
        c = b - g * (b - a);
        d = a + g * (b - a);
        if (b - a).abs() < 1e-9 {
            break;
        }
    }
    Some((a + b) / 2.0)
}

/// Medidas de error de `accuracy()`: ME, RMSE, MAE, MPE, MAPE, MASE y ACF1.
#[derive(Debug, Clone, Default)]
pub struct Errores {
    pub me: f64,
    pub rmse: f64,
    pub mae: f64,
    pub mpe: f64,
    pub mape: f64,
    pub mase: f64,
    pub acf1: f64,
}

/// `escala_de`: serie de entrenamiento con la que se escala el MASE (error medio del ingenuo estacional).
pub fn errores(real: &[f64], prono: &[f64], escala_de: &[f64], m: usize) -> Errores {
    let e: Vec<f64> = real.iter().zip(prono).filter(|(a, b)| a.is_finite() && b.is_finite()).map(|(a, b)| a - b).collect();
    let pe: Vec<f64> = real.iter().zip(prono).filter(|(a, b)| a.is_finite() && b.is_finite() && **a != 0.0).map(|(a, b)| 100.0 * (a - b) / a).collect();
    if e.is_empty() {
        // sin valores para comparar: vacíos (se muestran «—»), no ceros que parecerían un ajuste perfecto
        let x = f64::NAN;
        return Errores { me: x, rmse: x, mae: x, mpe: x, mape: x, mase: x, acf1: x };
    }
    let lag = if m > 1 && escala_de.len() > m { m } else { 1 };
    let dif: Vec<f64> = (lag..escala_de.len()).map(|t| (escala_de[t] - escala_de[t - lag]).abs()).collect();
    let escala = if dif.is_empty() { f64::NAN } else { media(&dif) };
    let mae = e.iter().map(|x| x.abs()).sum::<f64>() / e.len() as f64;
    Errores {
        me: media(&e),
        rmse: (e.iter().map(|x| x * x).sum::<f64>() / e.len() as f64).sqrt(),
        mae,
        mpe: if pe.is_empty() { f64::NAN } else { media(&pe) },
        mape: if pe.is_empty() { f64::NAN } else { pe.iter().map(|x| x.abs()).sum::<f64>() / pe.len() as f64 },
        mase: mae / escala,
        acf1: acf(&e, 1).first().copied().unwrap_or(f64::NAN),
    }
}
