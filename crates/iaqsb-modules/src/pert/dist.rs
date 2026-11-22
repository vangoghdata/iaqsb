//! Las distribuciones de tiempo de las actividades de PERT (las mismas 20 de
//! WinQSB, con sus parámetros en el mismo orden): media, varianza y muestreo.

use iaqsb_core::report::fmt;
use iaqsb_core::ModError;
use rand::RngExt;
use rand_distr::Distribution;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum Dist {
    /// Estimación de tres tiempos (optimista, más probable, pesimista).
    #[default]
    ThreeTime,
    Normal,
    Uniform,
    Triangular,
    Exponential,
    Beta,
    Gamma,
    Erlang,
    Weibull,
    Lognormal,
    Laplace,
    Pareto,
    PowerFunction,
    Binomial,
    Poisson,
    Geometric,
    Hypergeometric,
    Constant,
    /// Valores discretos "v/p, v/p, ...".
    Discrete,
    /// Media y desviación conocidas (se simula como normal).
    Arbitrary,
    /// Cada actividad tiene su propia distribución.
    Mixed,
}

pub const ALL: [Dist; 21] = [
    Dist::ThreeTime,
    Dist::Normal,
    Dist::Uniform,
    Dist::Triangular,
    Dist::Exponential,
    Dist::Beta,
    Dist::Gamma,
    Dist::Erlang,
    Dist::Weibull,
    Dist::Lognormal,
    Dist::Laplace,
    Dist::Pareto,
    Dist::PowerFunction,
    Dist::Binomial,
    Dist::Poisson,
    Dist::Geometric,
    Dist::Hypergeometric,
    Dist::Constant,
    Dist::Discrete,
    Dist::Arbitrary,
    Dist::Mixed,
];

impl Dist {
    pub fn id(self) -> &'static str {
        match self {
            Dist::ThreeTime => "three_time",
            Dist::Normal => "normal",
            Dist::Uniform => "uniform",
            Dist::Triangular => "triangular",
            Dist::Exponential => "exponential",
            Dist::Beta => "beta",
            Dist::Gamma => "gamma",
            Dist::Erlang => "erlang",
            Dist::Weibull => "weibull",
            Dist::Lognormal => "lognormal",
            Dist::Laplace => "laplace",
            Dist::Pareto => "pareto",
            Dist::PowerFunction => "power_function",
            Dist::Binomial => "binomial",
            Dist::Poisson => "poisson",
            Dist::Geometric => "geometric",
            Dist::Hypergeometric => "hypergeometric",
            Dist::Constant => "constant",
            Dist::Discrete => "discrete",
            Dist::Arbitrary => "arbitrary",
            Dist::Mixed => "mixed",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Dist::ThreeTime => "Tres tiempos (beta PERT)",
            Dist::Normal => "Normal",
            Dist::Uniform => "Uniforme",
            Dist::Triangular => "Triangular",
            Dist::Exponential => "Exponencial",
            Dist::Beta => "Beta",
            Dist::Gamma => "Gamma",
            Dist::Erlang => "Erlang",
            Dist::Weibull => "Weibull",
            Dist::Lognormal => "Lognormal",
            Dist::Laplace => "Laplace",
            Dist::Pareto => "Pareto",
            Dist::PowerFunction => "Potencia",
            Dist::Binomial => "Binomial",
            Dist::Poisson => "Poisson",
            Dist::Geometric => "Geométrica",
            Dist::Hypergeometric => "Hipergeométrica",
            Dist::Constant => "Constante",
            Dist::Discrete => "Discreta",
            Dist::Arbitrary => "Arbitraria (media y desviación)",
            Dist::Mixed => "Mixta (una por actividad)",
        }
    }

    /// Nombre en los archivos de WinQSB.
    pub fn winqsb(self) -> &'static str {
        match self {
            Dist::ThreeTime => "3-Time estimate",
            Dist::Normal => "Normal",
            Dist::Uniform => "Uniform",
            Dist::Triangular => "Triangular",
            Dist::Exponential => "Exponential",
            Dist::Beta => "Beta",
            Dist::Gamma => "Gamma",
            Dist::Erlang => "Erlang",
            Dist::Weibull => "Weibull",
            Dist::Lognormal => "LogNormal",
            Dist::Laplace => "Laplace",
            Dist::Pareto => "Pareto",
            Dist::PowerFunction => "Power Function",
            Dist::Binomial => "Binomial",
            Dist::Poisson => "Poisson",
            Dist::Geometric => "Geometric",
            Dist::Hypergeometric => "HyperGeometric",
            Dist::Constant => "Constant",
            Dist::Discrete => "Discrete",
            Dist::Arbitrary => "Arbitrary",
            Dist::Mixed => "Mixed",
        }
    }

    /// Reconoce el nombre de WinQSB (por sus tres primeras letras) o el de IAQSB.
    pub fn parse(s: &str) -> Option<Dist> {
        let t = s.trim();
        if let Some(d) = ALL.iter().find(|d| d.id().eq_ignore_ascii_case(t)) {
            return Some(*d);
        }
        let u: String = t.to_ascii_uppercase().chars().take(3).collect();
        Some(match u.as_str() {
            "3-T" | "THR" | "" => Dist::ThreeTime,
            "NOR" => Dist::Normal,
            "UNI" => Dist::Uniform,
            "TRI" => Dist::Triangular,
            "EXP" => Dist::Exponential,
            "BET" => Dist::Beta,
            "GAM" => Dist::Gamma,
            "ERL" => Dist::Erlang,
            "WEI" => Dist::Weibull,
            "LOG" => Dist::Lognormal,
            "LAP" => Dist::Laplace,
            "PAR" => Dist::Pareto,
            "POW" => Dist::PowerFunction,
            "BIN" => Dist::Binomial,
            "POI" => Dist::Poisson,
            "GEO" => Dist::Geometric,
            "HYP" => Dist::Hypergeometric,
            "CON" => Dist::Constant,
            "DIS" => Dist::Discrete,
            "ARB" => Dist::Arbitrary,
            "MIX" => Dist::Mixed,
            _ => return None,
        })
    }

    /// Nombres de los parámetros que usa (1 a 3).
    pub fn params(self) -> &'static [&'static str] {
        match self {
            Dist::ThreeTime => &["Optimista (a)", "Más probable (m)", "Pesimista (b)"],
            Dist::Normal | Dist::Arbitrary => &["Media (μ)", "Desviación (σ)"],
            Dist::Uniform => &["Mínimo (a)", "Máximo (b)"],
            Dist::Triangular => &["Mínimo (a)", "Moda (b)", "Máximo (c)"],
            Dist::Exponential => &["Posición (a)", "Media (b)"],
            Dist::Beta => &["Escala (b)", "Forma (v)", "Forma (w)"],
            Dist::Gamma => &["Posición (a)", "Escala (b)", "Forma (c)"],
            Dist::Erlang => &["Posición (a)", "Escala (b)", "Fases (c, entero)"],
            Dist::Weibull | Dist::PowerFunction => &["Posición (a)", "Escala (b)", "Forma (c)"],
            Dist::Lognormal => &["Media de ln x (μ)", "Desviación de ln x (σ)"],
            Dist::Laplace => &["Media (a)", "Escala (b)"],
            Dist::Pareto => &["Posición (a)", "Forma (c)"],
            Dist::Binomial => &["Ensayos (n)", "Probabilidad (p)"],
            Dist::Poisson => &["Media (μ)"],
            Dist::Geometric => &["Probabilidad (p)"],
            Dist::Hypergeometric => &["Población (N)", "Éxitos (K)", "Muestra (n)"],
            Dist::Constant => &["Tiempo"],
            Dist::Discrete => &["Número de valores"],
            Dist::Mixed => &[],
        }
    }
}

/// Distribución concreta de una actividad, ya validada.
#[derive(Debug, Clone)]
pub struct Law {
    pub dist: Dist,
    pub p: [f64; 3],
    /// Valores y probabilidades (discreta).
    pub values: Vec<(f64, f64)>,
}

fn need(ok: bool, act: &str, msg: &str) -> Result<(), ModError> {
    if ok {
        Ok(())
    } else {
        Err(ModError::Invalid(format!("{act}: {msg}")))
    }
}

/// Lee "v/p, v/p, ..." (probabilidades individuales o acumuladas).
pub fn parse_discrete(s: &str, act: &str) -> Result<Vec<(f64, f64)>, ModError> {
    let mut v = Vec::new();
    for part in s.split([',', ';']).map(str::trim).filter(|p| !p.is_empty()) {
        let (a, b) = part
            .split_once('/')
            .ok_or_else(|| ModError::Invalid(format!("{act}: «{part}» debe tener la forma valor/probabilidad")))?;
        let x: f64 = a.trim().parse().map_err(|_| ModError::Invalid(format!("{act}: «{a}» no es un número")))?;
        let p: f64 = b.trim().parse().map_err(|_| ModError::Invalid(format!("{act}: «{b}» no es una probabilidad")))?;
        need(p >= 0.0, act, "las probabilidades no pueden ser negativas")?;
        v.push((x, p));
    }
    need(!v.is_empty(), act, "faltan los valores discretos (v/p, v/p, ...)")?;
    let sum: f64 = v.iter().map(|x| x.1).sum();
    let cumulative = v.len() > 1 && v.windows(2).all(|w| w[1].1 >= w[0].1) && (v.last().unwrap().1 - 1.0).abs() < 1e-6 && (sum - 1.0).abs() > 1e-6;
    if cumulative {
        let mut prev = 0.0;
        for x in v.iter_mut() {
            let c = x.1;
            x.1 = c - prev;
            prev = c;
        }
    } else {
        need((sum - 1.0).abs() < 1e-4, act, &format!("las probabilidades suman {}, no 1", fmt(sum)))?;
        for x in v.iter_mut() {
            x.1 /= sum;
        }
    }
    Ok(v)
}

impl Law {
    pub fn new(dist: Dist, p: [Option<f64>; 3], discrete: Option<&str>, act: &str) -> Result<Law, ModError> {
        let np = dist.params().len();
        let mut q = [0.0; 3];
        for k in 0..np {
            if dist == Dist::Discrete {
                break;
            }
            q[k] = p[k].ok_or_else(|| ModError::Invalid(format!("{act}: falta «{}» ({})", dist.params()[k], dist.label())))?;
            need(q[k].is_finite(), act, "los parámetros deben ser números finitos")?;
        }
        let [a, b, c] = q;
        let mut values = vec![];
        match dist {
            Dist::ThreeTime => {
                need(a >= 0.0, act, "el tiempo optimista no puede ser negativo")?;
                need(a <= b && b <= c, act, "debe cumplirse optimista ≤ más probable ≤ pesimista")?;
            }
            Dist::Normal | Dist::Arbitrary => need(b >= 0.0, act, "la desviación no puede ser negativa")?,
            Dist::Uniform => need(a <= b, act, "el mínimo no puede superar al máximo")?,
            Dist::Triangular => need(a <= b && b <= c && a < c, act, "debe cumplirse mínimo ≤ moda ≤ máximo, con mínimo < máximo")?,
            Dist::Exponential => need(b > 0.0, act, "la media debe ser positiva")?,
            Dist::Beta => need(a > 0.0 && b > 0.0 && c > 0.0, act, "la escala y las formas deben ser positivas")?,
            Dist::Gamma | Dist::Weibull | Dist::PowerFunction => need(b > 0.0 && c > 0.0, act, "la escala y la forma deben ser positivas")?,
            Dist::Erlang => need(b > 0.0 && c >= 1.0 && c.fract() == 0.0, act, "la escala debe ser positiva y las fases un entero ≥ 1")?,
            Dist::Lognormal => need(b > 0.0, act, "la desviación de ln x debe ser positiva")?,
            Dist::Laplace => need(b > 0.0, act, "la escala debe ser positiva")?,
            Dist::Pareto => need(a > 0.0 && b > 0.0, act, "la posición y la forma deben ser positivas")?,
            Dist::Binomial => need(a >= 1.0 && a.fract() == 0.0 && (0.0..=1.0).contains(&b), act, "n debe ser un entero ≥ 1 y p estar entre 0 y 1")?,
            Dist::Poisson => need(a > 0.0, act, "la media debe ser positiva")?,
            Dist::Geometric => need(a > 0.0 && a <= 1.0, act, "p debe estar en (0, 1]")?,
            Dist::Hypergeometric => need(
                a >= 1.0 && b >= 0.0 && c >= 1.0 && b <= a && c <= a && [a, b, c].iter().all(|x| x.fract() == 0.0),
                act,
                "N, K y n deben ser enteros con K ≤ N y n ≤ N",
            )?,
            Dist::Constant => need(a >= 0.0, act, "el tiempo no puede ser negativo")?,
            Dist::Discrete => values = parse_discrete(discrete.unwrap_or(""), act)?,
            Dist::Mixed => return Err(ModError::Invalid(format!("{act}: indica su distribución"))),
        }
        Ok(Law { dist, p: q, values })
    }

    pub fn mean(&self) -> f64 {
        let [a, b, c] = self.p;
        match self.dist {
            Dist::ThreeTime => (a + 4.0 * b + c) / 6.0,
            Dist::Normal | Dist::Arbitrary => a,
            Dist::Uniform => (a + b) / 2.0,
            Dist::Triangular => (a + b + c) / 3.0,
            Dist::Exponential => a + b,
            Dist::Beta => a * b / (b + c),
            Dist::Gamma | Dist::Erlang => a + b * c,
            Dist::Weibull => a + b * gamma(1.0 + 1.0 / c),
            Dist::Lognormal => (a + b * b / 2.0).exp(),
            Dist::Laplace => a,
            Dist::Pareto => {
                if b > 1.0 {
                    b * a / (b - 1.0)
                } else {
                    f64::INFINITY
                }
            }
            Dist::PowerFunction => a + b * c / (c + 1.0),
            Dist::Binomial => a * b,
            Dist::Poisson => a,
            Dist::Geometric => (1.0 - a) / a,
            Dist::Hypergeometric => c * b / a,
            Dist::Constant => a,
            Dist::Discrete => self.values.iter().map(|(x, p)| x * p).sum(),
            Dist::Mixed => f64::NAN,
        }
    }

    pub fn variance(&self) -> f64 {
        let [a, b, c] = self.p;
        match self.dist {
            Dist::ThreeTime => ((c - a) / 6.0).powi(2),
            Dist::Normal | Dist::Arbitrary => b * b,
            Dist::Uniform => (b - a).powi(2) / 12.0,
            Dist::Triangular => (a * a + b * b + c * c - a * b - a * c - b * c) / 18.0,
            Dist::Exponential => b * b,
            Dist::Beta => a * a * b * c / ((b + c).powi(2) * (b + c + 1.0)),
            Dist::Gamma | Dist::Erlang => b * b * c,
            Dist::Weibull => {
                let g1 = gamma(1.0 + 1.0 / c);
                b * b * (gamma(1.0 + 2.0 / c) - g1 * g1)
            }
            Dist::Lognormal => ((b * b).exp() - 1.0) * (2.0 * a + b * b).exp(),
            Dist::Laplace => 2.0 * b * b,
            Dist::Pareto => {
                if b > 2.0 {
                    a * a * b / ((b - 1.0).powi(2) * (b - 2.0))
                } else {
                    f64::INFINITY
                }
            }
            Dist::PowerFunction => b * b * c / ((c + 1.0).powi(2) * (c + 2.0)),
            Dist::Binomial => a * b * (1.0 - b),
            Dist::Poisson => a,
            Dist::Geometric => (1.0 - a) / (a * a),
            Dist::Hypergeometric => {
                let (n_, k, n) = (a, b, c);
                if n_ <= 1.0 {
                    0.0
                } else {
                    n * (k / n_) * ((n_ - k) / n_) * ((n_ - n) / (n_ - 1.0))
                }
            }
            Dist::Constant => 0.0,
            Dist::Discrete => {
                let m = self.mean();
                self.values.iter().map(|(x, p)| p * (x - m).powi(2)).sum()
            }
            Dist::Mixed => f64::NAN,
        }
    }

    /// Fórmula de la media y la varianza, para el modo didáctico.
    pub fn formula(&self) -> String {
        let [a, b, c] = self.p;
        let (m, v) = (fmt(self.mean()), fmt(self.variance()));
        match self.dist {
            Dist::ThreeTime => format!(
                "t = (a + 4m + b)/6 = ({} + 4·{} + {})/6 = {m};  σ² = ((b − a)/6)² = (({} − {})/6)² = {v}",
                fmt(a),
                fmt(b),
                fmt(c),
                fmt(c),
                fmt(a)
            ),
            Dist::Uniform => format!("t = (a + b)/2 = {m};  σ² = (b − a)²/12 = {v}"),
            Dist::Triangular => format!("t = (a + b + c)/3 = {m};  σ² = (a² + b² + c² − ab − ac − bc)/18 = {v}"),
            Dist::Normal | Dist::Arbitrary => format!("t = μ = {m};  σ² = {v}"),
            Dist::Constant => format!("t = {m};  σ² = 0"),
            _ => format!("t = E[T] = {m};  σ² = Var[T] = {v}"),
        }
    }

    /// Una muestra del tiempo (nunca negativa).
    pub fn sample<R: rand::Rng + ?Sized>(&self, rng: &mut R) -> f64 {
        let [a, b, c] = self.p;
        let x = match self.dist {
            Dist::ThreeTime => {
                if c - a <= 0.0 {
                    a
                } else {
                    let al = 1.0 + 4.0 * (b - a) / (c - a);
                    let be = 1.0 + 4.0 * (c - b) / (c - a);
                    a + (c - a) * rand_distr::Beta::new(al, be).unwrap().sample(rng)
                }
            }
            Dist::Normal | Dist::Arbitrary => {
                if b == 0.0 {
                    a
                } else {
                    rand_distr::Normal::new(a, b).unwrap().sample(rng)
                }
            }
            Dist::Uniform => {
                if b > a {
                    rng.random_range(a..b)
                } else {
                    a
                }
            }
            Dist::Triangular => rand_distr::Triangular::new(a, c, b).unwrap().sample(rng),
            Dist::Exponential => a + rand_distr::Exp::new(1.0 / b).unwrap().sample(rng),
            Dist::Beta => a * rand_distr::Beta::new(b, c).unwrap().sample(rng),
            Dist::Gamma | Dist::Erlang => a + rand_distr::Gamma::new(c, b).unwrap().sample(rng),
            Dist::Weibull => a + rand_distr::Weibull::new(b, c).unwrap().sample(rng),
            Dist::Lognormal => rand_distr::LogNormal::new(a, b).unwrap().sample(rng),
            Dist::Laplace => {
                let u: f64 = rng.random::<f64>() - 0.5;
                a - b * u.signum() * (1.0 - 2.0 * u.abs()).max(1e-300).ln()
            }
            Dist::Pareto => rand_distr::Pareto::new(a, b).unwrap().sample(rng),
            Dist::PowerFunction => a + b * rng.random::<f64>().powf(1.0 / c),
            Dist::Binomial => rand_distr::Binomial::new(a as u64, b).unwrap().sample(rng) as f64,
            Dist::Poisson => rand_distr::Poisson::new(a).unwrap().sample(rng),
            Dist::Geometric => rand_distr::Geometric::new(a).unwrap().sample(rng) as f64,
            Dist::Hypergeometric => rand_distr::Hypergeometric::new(a as u64, b as u64, c as u64).unwrap().sample(rng) as f64,
            Dist::Constant => a,
            Dist::Discrete => {
                let u: f64 = rng.random();
                let mut acc = 0.0;
                let mut out = self.values.last().unwrap().0;
                for (x, p) in &self.values {
                    acc += p;
                    if u < acc {
                        out = *x;
                        break;
                    }
                }
                out
            }
            Dist::Mixed => 0.0,
        };
        x.max(0.0)
    }
}

fn gamma(x: f64) -> f64 {
    statrs::function::gamma::gamma(x)
}

/// Función de distribución normal estándar Φ(z).
pub fn phi(z: f64) -> f64 {
    0.5 * statrs::function::erf::erfc(-z / std::f64::consts::SQRT_2)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;

    #[test]
    fn medias_simuladas() {
        let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(7);
        let cases = [
            (Dist::ThreeTime, [2.0, 5.0, 14.0]),
            (Dist::Triangular, [1.0, 3.0, 8.0]),
            (Dist::Gamma, [1.0, 2.0, 3.0]),
            (Dist::Weibull, [0.0, 4.0, 2.0]),
            (Dist::Laplace, [20.0, 1.5, 0.0]),
            (Dist::PowerFunction, [1.0, 4.0, 2.0]),
            (Dist::Beta, [10.0, 2.0, 3.0]),
            (Dist::Hypergeometric, [20.0, 7.0, 5.0]),
            (Dist::Geometric, [0.3, 0.0, 0.0]),
            (Dist::Lognormal, [1.0, 0.4, 0.0]),
        ];
        for (d, p) in cases {
            let law = Law::new(d, [Some(p[0]), Some(p[1]), Some(p[2])], None, "x").unwrap();
            let n = 200_000;
            let xs: Vec<f64> = (0..n).map(|_| law.sample(&mut rng)).collect();
            let m = xs.iter().sum::<f64>() / n as f64;
            let v = xs.iter().map(|x| (x - m).powi(2)).sum::<f64>() / n as f64;
            assert!((m - law.mean()).abs() < 0.02 * law.mean().abs().max(1.0), "{d:?}: media {m} vs {}", law.mean());
            // la beta PERT no tiene exactamente la varianza ((b−a)/6)²
            if d != Dist::ThreeTime {
                assert!((v - law.variance()).abs() < 0.04 * law.variance().max(0.5), "{d:?}: varianza {v} vs {}", law.variance());
            }
        }
    }

    #[test]
    fn discreta_acumulada() {
        let v = parse_discrete("2/0.2, 4/0.7, 6/1", "x").unwrap();
        assert!((v[1].1 - 0.5).abs() < 1e-12 && (v[2].1 - 0.3).abs() < 1e-12);
        let w = parse_discrete("2/0.2, 4/0.5, 6/0.3", "x").unwrap();
        assert!((w[2].1 - 0.3).abs() < 1e-12);
    }
}
