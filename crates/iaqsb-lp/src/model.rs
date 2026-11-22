//! Modelo de programación lineal y entera, equivalente al de LP-ILP de WinQSB.

use schemars::JsonSchema;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Criterio del objetivo.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "lowercase")]
pub enum Objective {
    #[default]
    Max,
    Min,
}

/// Tipo de variable, como en WinQSB.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "lowercase")]
pub enum VarType {
    /// Continua (cota inferior por defecto 0).
    #[default]
    Continuous,
    /// Entera.
    Integer,
    /// Binaria (0 o 1).
    Binary,
    /// Sin restricción de signo (cotas -M y M).
    Unrestricted,
    /// Semicontinua: 0 o un valor entre sus cotas (p. ej. «no producir o producir
    /// al menos el lote mínimo»). WinQSB no la tiene: al guardar en .LPP queda continua.
    #[serde(rename = "semicontinuous")]
    SemiContinuous,
    /// Semientera: 0 o un entero entre sus cotas. En .LPP queda entera.
    #[serde(rename = "semiinteger")]
    SemiInteger,
}

/// Sentido de una restricción.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum Direction {
    #[serde(rename = "<=", alias = "=<", alias = "<")]
    Le,
    #[serde(rename = ">=", alias = "=>", alias = ">")]
    Ge,
    #[serde(rename = "=")]
    Eq,
}

impl Direction {
    pub fn symbol(self) -> &'static str {
        match self {
            Direction::Le => "<=",
            Direction::Ge => ">=",
            Direction::Eq => "=",
        }
    }
}

/// Cota de una variable: un número o infinito. En JSON se escribe como número,
/// o como "M" / "-M" (la notación de WinQSB), o "inf" / "-inf".
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bound(pub f64);

impl Bound {
    pub const INF: Bound = Bound(f64::INFINITY);
    pub const NEG_INF: Bound = Bound(f64::NEG_INFINITY);
    pub fn is_finite(self) -> bool {
        self.0.is_finite()
    }
    /// Texto al estilo WinQSB.
    pub fn to_winqsb(self) -> String {
        if self.0 == f64::INFINITY {
            "M".into()
        } else if self.0 == f64::NEG_INFINITY {
            "-M".into()
        } else {
            fmt_num(self.0)
        }
    }
    pub fn parse(s: &str) -> Option<Bound> {
        let t = s.trim().to_ascii_uppercase().replace(' ', "");
        match t.as_str() {
            "M" | "+M" | "INF" | "+INF" | "INFINITY" | "+INFINITY" => Some(Bound::INF),
            "-M" | "-INF" | "-INFINITY" => Some(Bound::NEG_INF),
            "" => None,
            _ => parse_number(&t).map(Bound),
        }
    }
}

impl Serialize for Bound {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        if self.0.is_finite() {
            s.serialize_f64(self.0)
        } else {
            s.serialize_str(if self.0 > 0.0 { "M" } else { "-M" })
        }
    }
}

impl<'de> Deserialize<'de> for Bound {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Raw {
            N(f64),
            S(String),
        }
        match Raw::deserialize(d)? {
            Raw::N(v) => Ok(Bound(v)),
            Raw::S(s) => Bound::parse(&s).ok_or_else(|| {
                serde::de::Error::custom(format!(
                    "cota no válida: {s:?} (usa un número, \"M\" o \"-M\")"
                ))
            }),
        }
    }
}

impl JsonSchema for Bound {
    fn schema_name() -> String {
        "Bound".into()
    }
    fn json_schema(_: &mut schemars::gen::SchemaGenerator) -> schemars::schema::Schema {
        serde_json::from_value(serde_json::json!({
            "description": "Número, o \"M\" para +infinito y \"-M\" para -infinito",
            "anyOf": [
                {"type": "number"},
                {"type": "string", "enum": ["M", "-M", "inf", "-inf"]}
            ]
        }))
        .unwrap()
    }
}

fn default_lower() -> Bound {
    Bound(0.0)
}
fn default_upper() -> Bound {
    Bound::INF
}

/// Variable de decisión.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Variable {
    /// Nombre (como X1, Pickle, Burger).
    pub name: String,
    /// Coeficiente en la función objetivo.
    #[serde(default)]
    pub cost: f64,
    /// Cota inferior. Por defecto 0.
    #[serde(default = "default_lower")]
    pub lower: Bound,
    /// Cota superior. Por defecto "M" (sin límite).
    #[serde(default = "default_upper")]
    pub upper: Bound,
    /// Tipo de variable.
    #[serde(default, rename = "type")]
    pub var_type: VarType,
    /// Prioridad de ramificación en branch and bound (mayor = antes).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub priority: Option<i32>,
}

/// Restricción lineal.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Constraint {
    /// Nombre (como C1).
    pub name: String,
    /// Coeficientes, uno por variable y en el mismo orden. En modelos grandes
    /// conviene dar solo los distintos de cero en `terms`.
    #[serde(default)]
    pub coefs: Vec<f64>,
    /// Alternativa dispersa a `coefs`: {"X1": 3, "X7": -1}. Si se da, manda.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub terms: Option<std::collections::BTreeMap<String, f64>>,
    /// Sentido: "<=", ">=" o "=".
    pub direction: Direction,
    /// Lado derecho.
    pub rhs: f64,
}

/// Problema completo de LP-ILP.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LpModel {
    /// Nombre del problema.
    #[serde(default)]
    pub name: String,
    /// "max" o "min".
    #[serde(default)]
    pub objective: Objective,
    pub variables: Vec<Variable>,
    #[serde(default)]
    pub constraints: Vec<Constraint>,
    /// Varias funciones objetivo por prioridad y peso (como en Gurobi). Si se
    /// dan, reemplazan a los costos de las variables. WinQSB no las tiene: al
    /// guardar en .LPP se guarda solo la de mayor prioridad.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub objectives: Vec<iaqsb_core::solver::Objective>,
    /// Solución inicial (arranque MIP) por nombre de variable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start: Option<std::collections::BTreeMap<String, f64>>,
}

impl LpModel {
    /// Pasa los `terms` dispersos a `coefs` (uno por variable).
    pub fn normalize(&mut self) -> Result<(), crate::LpError> {
        let n = self.variables.len();
        let index: std::collections::HashMap<String, usize> = self.variables.iter().enumerate().map(|(j, v)| (v.name.to_uppercase(), j)).collect();
        for c in &mut self.constraints {
            if let Some(t) = c.terms.take() {
                let mut coefs = vec![0.0; n];
                for (name, v) in t {
                    let j = index
                        .get(&name.to_uppercase())
                        .ok_or_else(|| crate::LpError::Invalid(format!("la restricción {} usa la variable {name}, que no existe", c.name)))?;
                    coefs[*j] += v;
                }
                c.coefs = coefs;
            }
        }
        Ok(())
    }

    /// Cotas efectivas de una variable según su tipo (binaria = [0,1], libre = [-M, M]).
    pub fn effective_bounds(&self, j: usize) -> (f64, f64) {
        let v = &self.variables[j];
        match v.var_type {
            VarType::Binary => (v.lower.0.max(0.0), v.upper.0.min(1.0)),
            VarType::Unrestricted => (f64::NEG_INFINITY, f64::INFINITY),
            _ => (v.lower.0, v.upper.0),
        }
    }

    pub fn is_integer(&self, j: usize) -> bool {
        matches!(
            self.variables[j].var_type,
            VarType::Integer | VarType::Binary | VarType::SemiInteger
        )
    }

    pub fn has_integers(&self) -> bool {
        (0..self.variables.len()).any(|j| self.is_integer(j))
    }

    /// Comprueba que el modelo es coherente.
    pub fn validate(&self) -> Result<(), crate::LpError> {
        use crate::LpError::Invalid;
        let n = self.variables.len();
        if n == 0 {
            return Err(Invalid("el problema no tiene variables".into()));
        }
        let mut seen = std::collections::HashSet::new();
        for v in &self.variables {
            if v.name.trim().is_empty() {
                return Err(Invalid("hay una variable sin nombre".into()));
            }
            if !seen.insert(v.name.to_uppercase()) {
                return Err(Invalid(format!("la variable {} está repetida", v.name)));
            }
            if !v.cost.is_finite() {
                return Err(Invalid(format!(
                    "el coste de {} no es un número finito",
                    v.name
                )));
            }
        }
        for j in 0..n {
            let (l, u) = self.effective_bounds(j);
            if l > u {
                return Err(Invalid(format!(
                    "la cota inferior de {} es mayor que la superior",
                    self.variables[j].name
                )));
            }
        }
        for c in &self.constraints {
            if c.coefs.len() != n {
                return Err(Invalid(format!(
                    "la restricción {} tiene {} coeficientes y hay {} variables",
                    c.name,
                    c.coefs.len(),
                    n
                )));
            }
            if !c.rhs.is_finite() || c.coefs.iter().any(|x| !x.is_finite()) {
                return Err(Invalid(format!(
                    "la restricción {} tiene valores no finitos",
                    c.name
                )));
            }
        }
        Ok(())
    }
}

/// Formatea un número sin ceros sobrantes (3, 2.5, -0.125).
pub fn fmt_num(x: f64) -> String {
    if x == x.trunc() && x.abs() < 1e15 {
        format!("{}", x as i64)
    } else {
        let s = format!("{:.10}", x);
        let s = s.trim_end_matches('0').trim_end_matches('.');
        s.to_string()
    }
}

/// Formatea con 4 decimales como máximo, para mensajes.
pub fn fmt_short(x: f64) -> String {
    let s = format!("{:.4}", x);
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s == "-0" {
        "0".into()
    } else {
        s.to_string()
    }
}

/// Lee un número aceptando separadores de miles con coma y el símbolo $, como WinQSB.
pub fn parse_number(s: &str) -> Option<f64> {
    let t: String = s
        .chars()
        .filter(|c| *c != ',' && *c != '$' && !c.is_whitespace())
        .collect();
    if t.is_empty() {
        return None;
    }
    t.parse::<f64>().ok()
}
