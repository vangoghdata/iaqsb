//! Piezas comunes de los modelos lineales de WinQSB (LP-ILP, GP-IGP, QP-IQP...):
//! cotas con "M", tipos de variable, sentido de las restricciones y lectura y
//! escritura de las dos formas de sus archivos (hoja de cálculo y modelo normal).

use crate::module::ModError;
use crate::winqsb::{num, parse_num, Line};
use schemars::JsonSchema;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Número que admite infinito, escrito "M" / "-M" como en WinQSB.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bnd(pub f64);

impl Bnd {
    pub const INF: Bnd = Bnd(f64::INFINITY);
    pub const NEG_INF: Bnd = Bnd(f64::NEG_INFINITY);
    pub fn text(self) -> String {
        num(self.0)
    }
}

impl Serialize for Bnd {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        if self.0.is_finite() {
            s.serialize_f64(self.0)
        } else {
            s.serialize_str(if self.0 > 0.0 { "M" } else { "-M" })
        }
    }
}

impl<'de> Deserialize<'de> for Bnd {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Raw {
            N(f64),
            S(String),
        }
        match Raw::deserialize(d)? {
            Raw::N(v) => Ok(Bnd(v)),
            Raw::S(s) => parse_num(&s)
                .map(Bnd)
                .ok_or_else(|| serde::de::Error::custom(format!("cota no válida: {s:?} (usa un número, \"M\" o \"-M\")"))),
        }
    }
}

impl JsonSchema for Bnd {
    fn schema_name() -> String {
        "Bnd".into()
    }
    fn json_schema(_: &mut schemars::gen::SchemaGenerator) -> schemars::schema::Schema {
        serde_json::from_value(serde_json::json!({
            "description": "Número, o \"M\" para +infinito y \"-M\" para -infinito",
            "anyOf": [{"type": "number"}, {"type": "string", "enum": ["M", "-M"]}]
        }))
        .unwrap()
    }
}

fn zero() -> Bnd {
    Bnd(0.0)
}
fn inf() -> Bnd {
    Bnd::INF
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "lowercase")]
pub enum VarKind {
    #[default]
    Continuous,
    Integer,
    Binary,
    /// Sin restricción de signo.
    Unrestricted,
}

impl VarKind {
    pub fn winqsb(self) -> &'static str {
        match self {
            VarKind::Continuous => "Continuous",
            VarKind::Integer => "Integer",
            VarKind::Binary => "Binary",
            VarKind::Unrestricted => "Unrestricted",
        }
    }
    pub fn parse(s: &str) -> VarKind {
        match s.trim().chars().next().map(|c| c.to_ascii_uppercase()) {
            Some('I') | Some('E') => VarKind::Integer,
            Some('B') => VarKind::Binary,
            Some('U') | Some('L') | Some('F') => VarKind::Unrestricted,
            _ => VarKind::Continuous,
        }
    }
}

/// Variable de decisión con cotas y tipo.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Var {
    pub name: String,
    #[serde(default = "zero")]
    pub lower: Bnd,
    #[serde(default = "inf")]
    pub upper: Bnd,
    #[serde(default, rename = "type")]
    pub kind: VarKind,
}

impl Var {
    pub fn new(name: impl Into<String>) -> Var {
        Var { name: name.into(), lower: Bnd(0.0), upper: Bnd::INF, kind: VarKind::Continuous }
    }
    /// Cotas efectivas según el tipo (binaria: [0, 1]; libre: sin cotas si no se dieron).
    pub fn bounds(&self) -> (f64, f64) {
        match self.kind {
            VarKind::Binary => (0.0, 1.0),
            VarKind::Unrestricted => {
                let l = if self.lower.0 == 0.0 { f64::NEG_INFINITY } else { self.lower.0 };
                (l, self.upper.0)
            }
            _ => (self.lower.0, self.upper.0),
        }
    }
    pub fn integer(&self) -> bool {
        matches!(self.kind, VarKind::Integer | VarKind::Binary)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
pub enum Dir {
    #[default]
    #[serde(rename = "<=", alias = "=<", alias = "<", alias = "≤")]
    Le,
    #[serde(rename = ">=", alias = "=>", alias = ">", alias = "≥")]
    Ge,
    #[serde(rename = "=")]
    Eq,
}

impl Dir {
    pub fn symbol(self) -> &'static str {
        match self {
            Dir::Le => "<=",
            Dir::Ge => ">=",
            Dir::Eq => "=",
        }
    }
    pub fn parse(s: &str) -> Option<Dir> {
        match s.trim() {
            "<=" | "=<" | "<" | "≤" => Some(Dir::Le),
            ">=" | "=>" | ">" | "≥" => Some(Dir::Ge),
            "=" | "==" => Some(Dir::Eq),
            _ => None,
        }
    }
    pub fn sense(self) -> crate::lp::Sense {
        match self {
            Dir::Le => crate::lp::Sense::Le,
            Dir::Ge => crate::lp::Sense::Ge,
            Dir::Eq => crate::lp::Sense::Eq,
        }
    }
    /// ¿Se cumple lhs (dir) rhs con tolerancia?
    pub fn holds(self, lhs: f64, rhs: f64, tol: f64) -> bool {
        match self {
            Dir::Le => lhs <= rhs + tol,
            Dir::Ge => lhs >= rhs - tol,
            Dir::Eq => (lhs - rhs).abs() <= tol,
        }
    }
}

/// Restricción (los coeficientes van en la matriz del modelo).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Constr {
    pub name: String,
    #[serde(default)]
    pub direction: Dir,
    #[serde(default)]
    pub rhs: f64,
}

/// Separa "3X1+5X2-X3" en (coeficiente, nombre).
pub fn linear_terms(expr: &str) -> Result<Vec<(f64, String)>, String> {
    let s: String = expr.chars().filter(|c| !c.is_whitespace()).collect();
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;
    let mut out = Vec::new();
    while i < chars.len() {
        let start = i;
        if chars[i] == '+' || chars[i] == '-' {
            i += 1;
        }
        while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.' || chars[i] == '*' || chars[i] == ',') {
            i += 1;
        }
        // notación científica 1E+3 seguida de nombre: poco habitual, no se admite
        let coef_txt: String = chars[start..i].iter().filter(|c| **c != '*').collect();
        let coef = match coef_txt.as_str() {
            "" | "+" => 1.0,
            "-" => -1.0,
            t => parse_num(t).ok_or_else(|| format!("coeficiente {t:?} no válido"))?,
        };
        let ns = i;
        while i < chars.len() && chars[i] != '+' && chars[i] != '-' {
            i += 1;
        }
        let name: String = chars[ns..i].iter().collect();
        if name.is_empty() {
            if coef_txt.is_empty() {
                return Err("falta el nombre de una variable".into());
            }
            // constante suelta: se devuelve con nombre vacío
            out.push((coef, String::new()));
            continue;
        }
        out.push((coef, name));
    }
    Ok(out)
}

/// Expresión lineal "3X1+5X2-X3" a partir de coeficientes.
pub fn linear_text(coefs: &[f64], names: &[String]) -> String {
    let mut s = String::new();
    for (c, nm) in coefs.iter().zip(names) {
        if *c == 0.0 {
            continue;
        }
        let sign = if *c < 0.0 {
            "-"
        } else if s.is_empty() {
            ""
        } else {
            "+"
        };
        let mag = c.abs();
        let coef = if mag == 1.0 { String::new() } else { num(mag) };
        s += &format!("{sign}{coef}{nm}");
    }
    if s.is_empty() {
        s = format!("0{}", names.first().map(String::as_str).unwrap_or("X1"));
    }
    s
}

/// Coeficientes de una expresión lineal sobre las variables dadas.
pub fn coefs_of(expr: &str, names: &[String], at: &str) -> Result<Vec<f64>, ModError> {
    let mut v = vec![0.0; names.len()];
    for (c, nm) in linear_terms(expr).map_err(|e| ModError::Format(format!("{at}: {e}")))? {
        if nm.is_empty() {
            continue;
        }
        let j = names
            .iter()
            .position(|x| x.eq_ignore_ascii_case(&nm))
            .ok_or_else(|| ModError::Format(format!("{at}: variable desconocida «{nm}»")))?;
        v[j] += c;
    }
    Ok(v)
}

/// Aplica las cotas "X1  >=0, <=M" de la forma normal.
pub fn apply_bounds(v: &mut Var, text: &str) -> Result<(), ModError> {
    for part in text.split(',').map(str::trim).filter(|s| !s.is_empty()) {
        let compact: String = part.chars().filter(|c| !c.is_whitespace()).collect();
        let (_, d, val) = crate::expr::split_relation(&compact).ok_or_else(|| ModError::Format(format!("«{part}» no es una cota de {}", v.name)))?;
        let b = parse_num(&val).ok_or_else(|| ModError::Format(format!("«{val}» no es una cota de {}", v.name)))?;
        match d {
            "<=" => v.upper = Bnd(b),
            ">=" => v.lower = Bnd(b),
            _ => {
                v.lower = Bnd(b);
                v.upper = Bnd(b);
            }
        }
    }
    Ok(())
}

/// Lee las filas LowerBound / UpperBound / VariableType de la forma de hoja.
pub fn read_bound_rows(vars: &mut [Var], lb: &Line, ub: &Line, ty: &Line) {
    for (j, v) in vars.iter_mut().enumerate() {
        v.kind = VarKind::parse(ty.get(j + 1));
        if let Some(x) = parse_num(lb.get(j + 1)) {
            v.lower = Bnd(x);
        }
        if let Some(x) = parse_num(ub.get(j + 1)) {
            v.upper = Bnd(x);
        }
    }
}

/// Celdas de las filas de cotas y tipos para escribir la forma de hoja.
pub fn bound_rows(vars: &[Var], pad: usize) -> [Vec<String>; 3] {
    let mut lb = vec!["LowerBound".to_string()];
    let mut ub = vec!["UpperBound".to_string()];
    let mut ty = vec!["VariableType".to_string()];
    for v in vars {
        let (l, u) = v.bounds();
        lb.push(num(l));
        ub.push(num(u));
        ty.push(v.kind.winqsb().into());
    }
    for r in [&mut lb, &mut ub, &mut ty] {
        r.extend(std::iter::repeat_n(String::new(), pad));
    }
    [lb, ub, ty]
}

/// Listas "Integer:", "Binary:", "Unrestricted:" de la forma normal.
pub fn kind_lists(vars: &[Var]) -> Vec<Vec<String>> {
    [("Integer:", VarKind::Integer), ("Binary:", VarKind::Binary), ("Unrestricted:", VarKind::Unrestricted)]
        .iter()
        .map(|(label, k)| {
            let names: Vec<&str> = vars.iter().filter(|v| v.kind == *k).map(|v| v.name.as_str()).collect();
            vec![label.to_string(), names.join(",")]
        })
        .collect()
}

pub fn apply_kind_list(vars: &mut [Var], line: &Line, kind: VarKind) -> Result<(), ModError> {
    for nm in line.get(1).split(',').map(str::trim).filter(|s| !s.is_empty()) {
        let v = vars
            .iter_mut()
            .find(|v| v.name.eq_ignore_ascii_case(nm))
            .ok_or_else(|| ModError::Format(format!("variable desconocida «{nm}» en la lista de tipos")))?;
        v.kind = kind;
    }
    Ok(())
}

/// Ajusta una matriz a filas × columnas, rellenando con ceros (la interfaz puede
/// dejarla desigual al añadir o quitar variables).
pub fn fit(mat: &mut Vec<Vec<f64>>, rows: usize, cols: usize) {
    mat.resize(rows, vec![]);
    for r in mat.iter_mut() {
        r.resize(cols, 0.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terminos() {
        let t = linear_terms("3X1 - x2 + 2.5*Y + Z").unwrap();
        assert_eq!(t, vec![(3.0, "X1".into()), (-1.0, "x2".into()), (2.5, "Y".into()), (1.0, "Z".into())]);
        let names: Vec<String> = ["X1", "X2"].iter().map(|s| s.to_string()).collect();
        assert_eq!(linear_text(&[3.0, -1.0], &names), "3X1-X2");
        assert_eq!(coefs_of("x2+2x1+x2", &names, "").unwrap(), vec![2.0, 2.0]);
    }

    #[test]
    fn cotas() {
        let b: Bnd = serde_json::from_str("\"M\"").unwrap();
        assert_eq!(b.0, f64::INFINITY);
        assert_eq!(serde_json::to_string(&Bnd(f64::NEG_INFINITY)).unwrap(), "\"-M\"");
        let mut v = Var::new("X");
        apply_bounds(&mut v, ">=1, <=10").unwrap();
        assert_eq!((v.lower.0, v.upper.0), (1.0, 10.0));
    }
}
