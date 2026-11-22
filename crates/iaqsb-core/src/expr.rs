//! Expresiones matemáticas al estilo de WinQSB: "3X1 + 2.5X2^2 - EXP(X1*X2)".
//!
//! Admite + − × ÷ ^, paréntesis, multiplicación implícita ("3X1", "2(X+1)"),
//! funciones (EXP, LN, LOG = ln como en WinQSB, LOG10, SQRT/SQR, ABS, SIN, COS, TAN, ASIN, ACOS,
//! ATAN/ATN, FIX, INT, SGN, CIN, MIN, MAX), división entera \ y MOD de Visual Basic
//! y nombres de variables sin distinguir mayúsculas. Se compila una vez a una
//! secuencia de operaciones y se evalúa muy rápido (lo usan los optimizadores).

use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
enum Op {
    Num(f64),
    Var(usize),
    Neg,
    Add,
    Sub,
    Mul,
    Div,
    /// División entera de Visual Basic (a \ b).
    IDiv,
    /// Resto de Visual Basic (a MOD b).
    Mod,
    Pow,
    Func(Func, usize),
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Func {
    Exp,
    Ln,
    Log10,
    Sqrt,
    Abs,
    Sin,
    Cos,
    Tan,
    Asin,
    Acos,
    Atan,
    /// Parte entera hacia cero (FIX de WinQSB).
    Fix,
    /// Parte entera hacia abajo (INT).
    Floor,
    /// Signo (SGN).
    Sgn,
    /// Identidad (CIN de WinQSB).
    Ident,
    Min,
    Max,
}

#[derive(Debug, Clone)]
pub struct Expr {
    ops: Vec<Op>,
    /// Texto original.
    pub source: String,
}

#[derive(Debug, Clone, thiserror::Error, PartialEq)]
#[error("{0}")]
pub struct ExprError(pub String);

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Num(f64),
    Ident(String),
    Op(char),
    LParen,
    RParen,
    Comma,
}

fn tokenize(s: &str) -> Result<Vec<Tok>, ExprError> {
    let c: Vec<char> = s.chars().collect();
    let mut i = 0;
    let mut out = Vec::new();
    while i < c.len() {
        let ch = c[i];
        if ch.is_whitespace() {
            i += 1;
        } else if ch.is_ascii_digit() || (ch == '.' && i + 1 < c.len() && c[i + 1].is_ascii_digit()) {
            let st = i;
            while i < c.len() && (c[i].is_ascii_digit() || c[i] == '.') {
                i += 1;
            }
            // exponente científico: 1E-5, 2.5e3 (solo si sigue un dígito o signo y dígito)
            if i < c.len() && (c[i] == 'e' || c[i] == 'E') {
                let j = i + 1;
                let k = if j < c.len() && (c[j] == '+' || c[j] == '-') { j + 1 } else { j };
                if k < c.len() && c[k].is_ascii_digit() {
                    // "2E1" podría ser 2*E1 (variable E1); WinQSB usa notación científica
                    i = k;
                    while i < c.len() && c[i].is_ascii_digit() {
                        i += 1;
                    }
                }
            }
            let t: String = c[st..i].iter().collect();
            out.push(Tok::Num(t.parse().map_err(|_| ExprError(format!("número no válido: {t}")))?));
        } else if ch.is_alphabetic() || ch == '_' {
            let st = i;
            while i < c.len() && (c[i].is_alphanumeric() || c[i] == '_' || c[i] == '.') {
                // un punto dentro del nombre solo si va seguido de letra o dígito (X1.5 no se admite)
                if c[i] == '.' && !(i + 1 < c.len() && c[i + 1].is_alphanumeric()) {
                    break;
                }
                i += 1;
            }
            out.push(Tok::Ident(c[st..i].iter().collect()));
        } else {
            match ch {
                '+' | '-' | '*' | '/' | '^' | '\\' => out.push(Tok::Op(ch)),
                '×' | '·' => out.push(Tok::Op('*')),
                '÷' => out.push(Tok::Op('/')),
                '−' => out.push(Tok::Op('-')),
                '(' | '[' | '{' => out.push(Tok::LParen),
                ')' | ']' | '}' => out.push(Tok::RParen),
                ',' | ';' => out.push(Tok::Comma),
                _ => return Err(ExprError(format!("carácter no válido: «{ch}»"))),
            }
            i += 1;
        }
    }
    Ok(out)
}

fn func_of(name: &str) -> Option<(Func, usize)> {
    Some(match name.to_ascii_uppercase().as_str() {
        "EXP" => (Func::Exp, 1),
        // como en WinQSB (y en Visual Basic), LOG es el logaritmo natural
        "LN" | "LOG" => (Func::Ln, 1),
        "LOG10" => (Func::Log10, 1),
        "SQRT" | "SQR" => (Func::Sqrt, 1),
        "ABS" => (Func::Abs, 1),
        "SIN" => (Func::Sin, 1),
        "COS" => (Func::Cos, 1),
        "TAN" => (Func::Tan, 1),
        "ASIN" => (Func::Asin, 1),
        "ACOS" => (Func::Acos, 1),
        "ATAN" | "ATN" => (Func::Atan, 1),
        "FIX" => (Func::Fix, 1),
        "INT" => (Func::Floor, 1),
        "SGN" | "SIGN" => (Func::Sgn, 1),
        "CIN" => (Func::Ident, 1),
        "MIN" => (Func::Min, 2),
        "MAX" => (Func::Max, 2),
        _ => return None,
    })
}

struct Parser<'a> {
    toks: Vec<Tok>,
    pos: usize,
    vars: &'a mut Vars,
    ops: Vec<Op>,
}

/// Tabla de variables: nombre (en mayúsculas) → índice.
#[derive(Debug, Clone, Default)]
pub struct Vars {
    pub names: Vec<String>,
    index: HashMap<String, usize>,
    /// Si es false, una variable desconocida es un error.
    pub open: bool,
}

impl Vars {
    pub fn fixed<S: AsRef<str>>(names: &[S]) -> Vars {
        let mut v = Vars { open: false, ..Default::default() };
        for n in names {
            v.insert(n.as_ref());
        }
        v
    }
    pub fn open() -> Vars {
        Vars { open: true, ..Default::default() }
    }
    fn insert(&mut self, name: &str) -> usize {
        let key = name.to_uppercase();
        if let Some(&i) = self.index.get(&key) {
            return i;
        }
        self.names.push(name.to_string());
        self.index.insert(key, self.names.len() - 1);
        self.names.len() - 1
    }
    pub fn get(&self, name: &str) -> Option<usize> {
        self.index.get(&name.to_uppercase()).copied()
    }
    pub fn len(&self) -> usize {
        self.names.len()
    }
    pub fn is_empty(&self) -> bool {
        self.names.is_empty()
    }
}

impl Parser<'_> {
    fn peek(&self) -> Option<&Tok> {
        self.toks.get(self.pos)
    }
    fn next(&mut self) -> Option<Tok> {
        let t = self.toks.get(self.pos).cloned();
        self.pos += 1;
        t
    }
    // expr := term (('+'|'-') term)*
    fn expr(&mut self) -> Result<(), ExprError> {
        self.term()?;
        while let Some(Tok::Op(c)) = self.peek() {
            let c = *c;
            if c != '+' && c != '-' {
                break;
            }
            self.pos += 1;
            self.term()?;
            self.ops.push(if c == '+' { Op::Add } else { Op::Sub });
        }
        Ok(())
    }
    // term := unary (('*'|'/'|implícito) unary)*
    fn term(&mut self) -> Result<(), ExprError> {
        self.unary()?;
        loop {
            match self.peek() {
                Some(Tok::Op('*')) | Some(Tok::Op('/')) | Some(Tok::Op('\\')) => {
                    let op = match self.next() {
                        Some(Tok::Op('/')) => Op::Div,
                        Some(Tok::Op('\\')) => Op::IDiv,
                        _ => Op::Mul,
                    };
                    self.unary()?;
                    self.ops.push(op);
                }
                Some(Tok::Ident(w)) if w.eq_ignore_ascii_case("MOD") && self.vars.get("MOD").is_none() => {
                    self.pos += 1;
                    self.unary()?;
                    self.ops.push(Op::Mod);
                }
                // multiplicación implícita: 3X1, 2(…), (…)(…), X Y
                Some(Tok::Num(_)) | Some(Tok::Ident(_)) | Some(Tok::LParen) => {
                    self.power()?;
                    self.ops.push(Op::Mul);
                }
                _ => break,
            }
        }
        Ok(())
    }
    fn unary(&mut self) -> Result<(), ExprError> {
        match self.peek() {
            Some(Tok::Op('-')) => {
                self.pos += 1;
                self.unary()?;
                self.ops.push(Op::Neg);
                Ok(())
            }
            Some(Tok::Op('+')) => {
                self.pos += 1;
                self.unary()
            }
            _ => self.power(),
        }
    }
    // power := atom ('^' unary)?   (asociativo a la derecha; -x^2 = -(x^2))
    fn power(&mut self) -> Result<(), ExprError> {
        self.atom()?;
        if let Some(Tok::Op('^')) = self.peek() {
            self.pos += 1;
            self.unary()?;
            self.ops.push(Op::Pow);
        }
        Ok(())
    }
    fn atom(&mut self) -> Result<(), ExprError> {
        match self.next() {
            Some(Tok::Num(v)) => {
                self.ops.push(Op::Num(v));
                Ok(())
            }
            Some(Tok::LParen) => {
                self.expr()?;
                match self.next() {
                    Some(Tok::RParen) => Ok(()),
                    _ => Err(ExprError("falta un paréntesis de cierre".into())),
                }
            }
            Some(Tok::Ident(name)) => {
                if matches!(self.peek(), Some(Tok::LParen)) {
                    if let Some((f, arity)) = func_of(&name) {
                        self.pos += 1;
                        for k in 0..arity {
                            self.expr()?;
                            if k + 1 < arity && self.next() != Some(Tok::Comma) {
                                return Err(ExprError(format!("{name} necesita {arity} argumentos")));
                            }
                        }
                        if self.next() != Some(Tok::RParen) {
                            return Err(ExprError(format!("falta «)» tras los argumentos de {name}")));
                        }
                        self.ops.push(Op::Func(f, arity));
                        return Ok(());
                    }
                }
                let up = name.to_ascii_uppercase();
                if up == "PI" && self.vars.get(&name).is_none() {
                    self.ops.push(Op::Num(std::f64::consts::PI));
                    return Ok(());
                }
                let idx = match self.vars.get(&name) {
                    Some(i) => i,
                    None if self.vars.open => self.vars.insert(&name),
                    None => return Err(ExprError(format!("variable desconocida: {name}"))),
                };
                self.ops.push(Op::Var(idx));
                Ok(())
            }
            Some(t) => Err(ExprError(format!("símbolo inesperado: {t:?}"))),
            None => Err(ExprError("la expresión está incompleta".into())),
        }
    }
}

impl Expr {
    /// Compila una expresión. Con `Vars::open()` las variables nuevas se añaden.
    pub fn parse(src: &str, vars: &mut Vars) -> Result<Expr, ExprError> {
        let toks = tokenize(src)?;
        if toks.is_empty() {
            return Err(ExprError("la expresión está vacía".into()));
        }
        let mut p = Parser { toks, pos: 0, vars, ops: vec![] };
        p.expr()?;
        if p.pos < p.toks.len() {
            return Err(ExprError(format!("sobra algo al final de «{src}»")));
        }
        Ok(Expr { ops: p.ops, source: src.to_string() })
    }

    pub fn eval(&self, x: &[f64]) -> f64 {
        let mut st: Vec<f64> = Vec::with_capacity(16);
        for op in &self.ops {
            match *op {
                Op::Num(v) => st.push(v),
                Op::Var(i) => st.push(x.get(i).copied().unwrap_or(0.0)),
                Op::Neg => {
                    let a = st.pop().unwrap();
                    st.push(-a)
                }
                Op::Func(f, n) => {
                    if n == 2 {
                        let b = st.pop().unwrap();
                        let a = st.pop().unwrap();
                        st.push(if f == Func::Min { a.min(b) } else { a.max(b) });
                    } else {
                        let a = st.pop().unwrap();
                        st.push(match f {
                            Func::Exp => a.exp(),
                            Func::Ln => a.ln(),
                            Func::Log10 => a.log10(),
                            Func::Sqrt => a.sqrt(),
                            Func::Abs => a.abs(),
                            Func::Sin => a.sin(),
                            Func::Cos => a.cos(),
                            Func::Tan => a.tan(),
                            Func::Asin => a.asin(),
                            Func::Acos => a.acos(),
                            Func::Atan => a.atan(),
                            Func::Fix => a.trunc(),
                            Func::Floor => a.floor(),
                            Func::Sgn => {
                                if a > 0.0 {
                                    1.0
                                } else if a < 0.0 {
                                    -1.0
                                } else {
                                    0.0
                                }
                            }
                            Func::Ident => a,
                            Func::Min | Func::Max => unreachable!(),
                        });
                    }
                }
                _ => {
                    let b = st.pop().unwrap();
                    let a = st.pop().unwrap();
                    st.push(match op {
                        Op::Add => a + b,
                        Op::Sub => a - b,
                        Op::Mul => a * b,
                        Op::Div => a / b,
                        Op::IDiv => (a.round() / b.round()).trunc(),
                        Op::Mod => a.round() % b.round(),
                        Op::Pow => {
                            // potencia entera exacta para bases negativas
                            if b == b.trunc() && b.abs() < 64.0 {
                                a.powi(b as i32)
                            } else {
                                a.powf(b)
                            }
                        }
                        _ => unreachable!(),
                    });
                }
            }
        }
        st.pop().unwrap_or(f64::NAN)
    }

    /// Valor y gradiente exactos (diferenciación automática hacia delante).
    pub fn eval_grad(&self, x: &[f64]) -> (f64, Vec<f64>) {
        let n = x.len();
        let mut st: Vec<(f64, Vec<f64>)> = Vec::with_capacity(16);
        let zero = || vec![0.0; n];
        for op in &self.ops {
            match *op {
                Op::Num(v) => st.push((v, zero())),
                Op::Var(i) => {
                    let mut g = zero();
                    if i < n {
                        g[i] = 1.0;
                    }
                    st.push((x.get(i).copied().unwrap_or(0.0), g))
                }
                Op::Neg => {
                    let (a, da) = st.pop().unwrap();
                    st.push((-a, da.iter().map(|v| -v).collect()))
                }
                Op::Func(f, 2) => {
                    let b = st.pop().unwrap();
                    let a = st.pop().unwrap();
                    let take_a = if f == Func::Min { a.0 <= b.0 } else { a.0 >= b.0 };
                    st.push(if take_a { a } else { b });
                }
                Op::Func(f, _) => {
                    let (a, da) = st.pop().unwrap();
                    let (v, d) = match f {
                        Func::Exp => (a.exp(), a.exp()),
                        Func::Ln => (a.ln(), 1.0 / a),
                        Func::Log10 => (a.log10(), 1.0 / (a * std::f64::consts::LN_10)),
                        Func::Sqrt => (a.sqrt(), 0.5 / a.sqrt()),
                        Func::Abs => (a.abs(), if a >= 0.0 { 1.0 } else { -1.0 }),
                        Func::Sin => (a.sin(), a.cos()),
                        Func::Cos => (a.cos(), -a.sin()),
                        Func::Tan => (a.tan(), 1.0 / (a.cos() * a.cos())),
                        Func::Asin => (a.asin(), 1.0 / (1.0 - a * a).sqrt()),
                        Func::Acos => (a.acos(), -1.0 / (1.0 - a * a).sqrt()),
                        Func::Atan => (a.atan(), 1.0 / (1.0 + a * a)),
                        Func::Fix => (a.trunc(), 0.0),
                        Func::Floor => (a.floor(), 0.0),
                        Func::Sgn => (if a > 0.0 { 1.0 } else if a < 0.0 { -1.0 } else { 0.0 }, 0.0),
                        Func::Ident => (a, 1.0),
                        Func::Min | Func::Max => unreachable!(),
                    };
                    st.push((v, da.iter().map(|g| g * d).collect()));
                }
                _ => {
                    let (b, db) = st.pop().unwrap();
                    let (a, da) = st.pop().unwrap();
                    let r = match op {
                        Op::Add => (a + b, da.iter().zip(&db).map(|(p, q)| p + q).collect()),
                        Op::Sub => (a - b, da.iter().zip(&db).map(|(p, q)| p - q).collect()),
                        Op::Mul => (a * b, da.iter().zip(&db).map(|(p, q)| p * b + a * q).collect()),
                        Op::Div => (a / b, da.iter().zip(&db).map(|(p, q)| (p * b - a * q) / (b * b)).collect()),
                        Op::IDiv => ((a.round() / b.round()).trunc(), vec![0.0; n]),
                        Op::Mod => (a.round() % b.round(), vec![0.0; n]),
                        Op::Pow => {
                            let const_exp = db.iter().all(|v| *v == 0.0);
                            if const_exp {
                                let (v, d) = if b == b.trunc() && b.abs() < 64.0 {
                                    (a.powi(b as i32), if b == 0.0 { 0.0 } else { b * a.powi(b as i32 - 1) })
                                } else {
                                    (a.powf(b), b * a.powf(b - 1.0))
                                };
                                (v, da.iter().map(|p| p * d).collect())
                            } else {
                                let v = a.powf(b);
                                (v, da.iter().zip(&db).map(|(p, q)| v * (q * a.ln() + b * p / a)).collect())
                            }
                        }
                        _ => unreachable!(),
                    };
                    st.push(r);
                }
            }
        }
        st.pop().unwrap_or((f64::NAN, vec![f64::NAN; n]))
    }

    /// Gradiente numérico por diferencias centrales.
    pub fn gradient(&self, x: &[f64]) -> Vec<f64> {
        let mut g = vec![0.0; x.len()];
        let mut y = x.to_vec();
        for i in 0..x.len() {
            let h = 1e-6 * (1.0 + x[i].abs());
            y[i] = x[i] + h;
            let a = self.eval(&y);
            y[i] = x[i] - h;
            let b = self.eval(&y);
            y[i] = x[i];
            g[i] = (a - b) / (2.0 * h);
        }
        g
    }
}

/// Coeficientes de una función lineal o cuadrática en n variables:
/// f(x) = c0 + c·x + ½ xᵀQx.
#[derive(Debug, Clone)]
pub struct Quadratic {
    pub c0: f64,
    pub c: Vec<f64>,
    /// Matriz simétrica Q (hessiana).
    pub q: Vec<Vec<f64>>,
}

impl Quadratic {
    pub fn is_linear(&self) -> bool {
        self.q.iter().all(|r| r.iter().all(|v| v.abs() < 1e-12))
    }
    pub fn eval(&self, x: &[f64]) -> f64 {
        let mut s = self.c0;
        for i in 0..x.len() {
            s += self.c[i] * x[i];
            for j in 0..x.len() {
                s += 0.5 * self.q[i][j] * x[i] * x[j];
            }
        }
        s
    }
}

/// Extrae los coeficientes exactos de una expresión cuadrática (o lineal).
/// Devuelve error si la expresión no es un polinomio de grado ≤ 2.
pub fn quadratic(e: &Expr, n: usize) -> Result<Quadratic, ExprError> {
    let z = vec![0.0; n];
    let c0 = e.eval(&z);
    let mut c = vec![0.0; n];
    let mut q = vec![vec![0.0; n]; n];
    let unit = |i: usize, s: f64| {
        let mut v = vec![0.0; n];
        v[i] = s;
        v
    };
    let fp: Vec<f64> = (0..n).map(|i| e.eval(&unit(i, 1.0))).collect();
    let fm: Vec<f64> = (0..n).map(|i| e.eval(&unit(i, -1.0))).collect();
    for i in 0..n {
        c[i] = (fp[i] - fm[i]) / 2.0;
        q[i][i] = fp[i] + fm[i] - 2.0 * c0;
    }
    for i in 0..n {
        for j in i + 1..n {
            let mut v = vec![0.0; n];
            v[i] = 1.0;
            v[j] = 1.0;
            let fij = e.eval(&v);
            let qij = fij - fp[i] - fp[j] + c0;
            q[i][j] = qij;
            q[j][i] = qij;
        }
    }
    let quad = Quadratic { c0, c, q };
    // comprobar en puntos de prueba que de verdad es cuadrática
    let probes: [f64; 5] = [0.37, -1.91, 2.53, 0.71, -0.29];
    for k in 0..3 {
        let x: Vec<f64> = (0..n).map(|i| probes[(i + k) % 5] * (1.0 + k as f64)).collect();
        let a = e.eval(&x);
        let b = quad.eval(&x);
        if !a.is_finite() || (a - b).abs() > 1e-7 * (1.0 + a.abs()) {
            return Err(ExprError(format!("«{}» no es una expresión lineal o cuadrática", e.source)));
        }
    }
    Ok(quad)
}

/// Separa "lhs <= rhs" en (lhs, sentido, rhs). Devuelve None si no hay relación.
pub fn split_relation(s: &str) -> Option<(String, &'static str, String)> {
    for (op, d) in [("<=", "<="), ("=<", "<="), (">=", ">="), ("=>", ">="), ("≤", "<="), ("≥", ">="), ("<", "<="), (">", ">="), ("=", "=")] {
        if let Some(p) = s.find(op) {
            return Some((s[..p].trim().to_string(), d, s[p + op.len()..].trim().to_string()));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ev(s: &str, names: &[&str], x: &[f64]) -> f64 {
        let mut v = Vars::fixed(names);
        Expr::parse(s, &mut v).unwrap().eval(x)
    }

    #[test]
    fn funciones_de_winqsb() {
        assert!((ev("LOG(EXP(2))", &[], &[]) - 2.0).abs() < 1e-12);
        assert_eq!(ev("LOG10(1000)", &[], &[]), 3.0);
        assert_eq!(ev("7 \\ 2 + 7 MOD 3 + INT(-2.5) + FIX(-2.5) + SGN(-3)", &[], &[]), 3.0 + 1.0 - 3.0 - 2.0 - 1.0);
        assert_eq!(ev("2X+300Y^2-3.15COS(Z+W)", &["X", "Y", "Z", "W"], &[1.0, 0.0, 0.0, 0.0]), 2.0 - 3.15);
    }

    #[test]
    fn gradiente_exacto() {
        let mut v = Vars::fixed(&["X", "Y"]);
        let e = Expr::parse("X^2*Y + exp(X*Y) - ln(Y) + sqrt(X) / Y + X^Y", &mut v).unwrap();
        let x = [1.3, 0.7];
        let (f, g) = e.eval_grad(&x);
        assert!((f - e.eval(&x)).abs() < 1e-12);
        let gn = e.gradient(&x);
        for (a, b) in g.iter().zip(&gn) {
            assert!((a - b).abs() < 1e-6 * a.abs().max(1.0), "{g:?} {gn:?}");
        }
    }

    #[test]
    fn aritmetica_y_precedencia() {
        assert_eq!(ev("2+3*4", &[], &[]), 14.0);
        assert_eq!(ev("-2^2", &[], &[]), -4.0);
        assert_eq!(ev("2^3^2", &[], &[]), 512.0);
        assert_eq!(ev("(1+2)(3+4)", &[], &[]), 21.0);
        assert_eq!(ev("3X1 + 2x2", &["X1", "X2"], &[1.0, 10.0]), 23.0);
        assert_eq!(ev("2.5X1^2", &["X1"], &[2.0]), 10.0);
        assert_eq!(ev("EXP(0) + ln(1) + max(2, 5)", &[], &[]), 6.0);
        assert_eq!(ev("1E-2*100", &[], &[]), 1.0);
        assert_eq!(ev("X1*X2 - X1/X2", &["X1", "X2"], &[4.0, 2.0]), 6.0);
    }

    #[test]
    fn variables_abiertas() {
        let mut v = Vars::open();
        let e = Expr::parse("Pickle + 3Burger", &mut v).unwrap();
        assert_eq!(v.names, vec!["Pickle", "Burger"]);
        assert_eq!(e.eval(&[1.0, 2.0]), 7.0);
        assert!(Expr::parse("Y + 1", &mut Vars::fixed(&["X"])).is_err());
    }

    #[test]
    fn coeficientes_cuadraticos() {
        let mut v = Vars::fixed(&["X1", "X2"]);
        let e = Expr::parse("3X1^2 + 2X1*X2 - 4X2 + 7", &mut v).unwrap();
        let q = quadratic(&e, 2).unwrap();
        assert_eq!(q.c0, 7.0);
        assert_eq!(q.c, vec![0.0, -4.0]);
        assert_eq!(q.q, vec![vec![6.0, 2.0], vec![2.0, 0.0]]);
        let e3 = Expr::parse("X1^3", &mut v).unwrap();
        assert!(quadratic(&e3, 2).is_err());
    }
}
