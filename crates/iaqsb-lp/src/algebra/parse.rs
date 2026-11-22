//! Lectura del lenguaje de modelado: tokens y árbol de sintaxis.
//!
//! Palabras clave en español, inglés y portugués, sin importar mayúsculas ni
//! tildes («sujeto a», «subject to», «sujeito a»; «suma», «sum», «soma»).

use std::fmt;

/// Posición en el texto (línea y columna desde 1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Pos {
    pub line: usize,
    pub col: usize,
}

/// Error de lectura o de cálculo, con la posición.
#[derive(Debug, Clone, PartialEq)]
pub struct Error {
    pub pos: Pos,
    pub msg: String,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.pos.line > 0 {
            write!(f, "línea {}, columna {}: {}", self.pos.line, self.pos.col, self.msg)
        } else {
            write!(f, "{}", self.msg)
        }
    }
}

pub fn err<T>(pos: Pos, msg: impl Into<String>) -> Result<T, Error> {
    Err(Error { pos, msg: msg.into() })
}

#[derive(Debug, Clone, PartialEq)]
pub enum Tok {
    Ident(String),
    Num(f64),
    Str(String),
    /// Signos: + - * / ^ ( ) [ ] { } , ; : := = == != < <= > >= -> .. |
    P(&'static str),
    End,
}

#[derive(Debug, Clone)]
pub struct Token {
    pub tok: Tok,
    pub pos: Pos,
}

/// Minúsculas y sin tildes (para comparar palabras clave).
pub fn norm(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'á' | 'à' | 'â' | 'ã' | 'Á' | 'À' | 'Â' | 'Ã' => 'a',
            'é' | 'ê' | 'É' | 'Ê' => 'e',
            'í' | 'Í' => 'i',
            'ó' | 'ô' | 'õ' | 'Ó' | 'Ô' | 'Õ' => 'o',
            'ú' | 'ü' | 'Ú' | 'Ü' => 'u',
            'ç' | 'Ç' => 'c',
            'ñ' | 'Ñ' => 'n',
            c => c.to_ascii_lowercase(),
        })
        .collect()
}

pub fn lex(src: &str) -> Result<Vec<Token>, Error> {
    let ch: Vec<char> = src.chars().collect();
    let mut out = vec![];
    let (mut i, mut line, mut col) = (0usize, 1usize, 1usize);
    let adv = |i: &mut usize, line: &mut usize, col: &mut usize, ch: &[char]| {
        if ch[*i] == '\n' {
            *line += 1;
            *col = 1;
        } else {
            *col += 1;
        }
        *i += 1;
    };
    while i < ch.len() {
        let c = ch[i];
        let pos = Pos { line, col };
        if c.is_whitespace() {
            adv(&mut i, &mut line, &mut col, &ch);
            continue;
        }
        // comentarios: # … , // … y /* … */
        if c == '#' || (c == '/' && ch.get(i + 1) == Some(&'/')) {
            while i < ch.len() && ch[i] != '\n' {
                adv(&mut i, &mut line, &mut col, &ch);
            }
            continue;
        }
        if c == '/' && ch.get(i + 1) == Some(&'*') {
            adv(&mut i, &mut line, &mut col, &ch);
            adv(&mut i, &mut line, &mut col, &ch);
            while i < ch.len() && !(ch[i] == '*' && ch.get(i + 1) == Some(&'/')) {
                adv(&mut i, &mut line, &mut col, &ch);
            }
            if i >= ch.len() {
                return err(pos, "comentario /* sin cerrar");
            }
            adv(&mut i, &mut line, &mut col, &ch);
            adv(&mut i, &mut line, &mut col, &ch);
            continue;
        }
        if c.is_ascii_digit() || (c == '.' && ch.get(i + 1).is_some_and(|d| d.is_ascii_digit())) {
            let start = i;
            while i < ch.len() && (ch[i].is_ascii_digit() || (ch[i] == '.' && ch.get(i + 1) != Some(&'.'))) {
                adv(&mut i, &mut line, &mut col, &ch);
            }
            if i < ch.len() && (ch[i] == 'e' || ch[i] == 'E') && ch.get(i + 1).is_some_and(|d| d.is_ascii_digit() || *d == '-' || *d == '+') {
                adv(&mut i, &mut line, &mut col, &ch);
                adv(&mut i, &mut line, &mut col, &ch);
                while i < ch.len() && ch[i].is_ascii_digit() {
                    adv(&mut i, &mut line, &mut col, &ch);
                }
            }
            let t: String = ch[start..i].iter().collect();
            let v: f64 = t.parse().map_err(|_| Error { pos, msg: format!("número no válido: {t}") })?;
            out.push(Token { tok: Tok::Num(v), pos });
            continue;
        }
        // Σ (sigma griega) y ∑ son la suma, no una letra
        if c == 'Σ' || c == '∑' {
            adv(&mut i, &mut line, &mut col, &ch);
            out.push(Token { tok: Tok::Ident("sum".into()), pos });
            continue;
        }
        if c.is_alphabetic() || c == '_' {
            let start = i;
            while i < ch.len() && (ch[i].is_alphanumeric() || ch[i] == '_' || ch[i] == '\'') {
                adv(&mut i, &mut line, &mut col, &ch);
            }
            // «s.t.» y «s.a.»
            let mut t: String = ch[start..i].iter().collect();
            if (t == "s" || t == "S") && i + 2 < ch.len() && ch[i] == '.' && ch[i + 2] == '.' && matches!(ch[i + 1], 't' | 'a' | 'T' | 'A') {
                t = format!("s{}", ch[i + 1].to_ascii_lowercase());
                for _ in 0..3 {
                    adv(&mut i, &mut line, &mut col, &ch);
                }
            }
            out.push(Token { tok: Tok::Ident(t), pos });
            continue;
        }
        if c == '"' || c == '\'' || c == '«' || c == '“' {
            let close = match c {
                '«' => '»',
                '“' => '”',
                x => x,
            };
            adv(&mut i, &mut line, &mut col, &ch);
            let start = i;
            while i < ch.len() && ch[i] != close {
                if ch[i] == '\n' {
                    return err(pos, "texto entre comillas sin cerrar");
                }
                adv(&mut i, &mut line, &mut col, &ch);
            }
            if i >= ch.len() {
                return err(pos, "texto entre comillas sin cerrar");
            }
            let t: String = ch[start..i].iter().collect();
            adv(&mut i, &mut line, &mut col, &ch);
            out.push(Token { tok: Tok::Str(t), pos });
            continue;
        }
        let two: String = ch[i..(i + 2).min(ch.len())].iter().collect();
        let p2 = match two.as_str() {
            ":=" => Some(":="),
            "==" => Some("=="),
            "!=" | "<>" => Some("!="),
            "<=" | "=<" => Some("<="),
            ">=" | "=>" => Some(">="),
            "->" => Some("->"),
            ".." => Some(".."),
            "&&" => Some("&&"),
            "||" => Some("||"),
            _ => None,
        };
        if let Some(p) = p2 {
            adv(&mut i, &mut line, &mut col, &ch);
            adv(&mut i, &mut line, &mut col, &ch);
            out.push(Token { tok: Tok::P(p), pos });
            continue;
        }
        let p1 = match c {
            '+' => "+",
            '-' | '−' => "-",
            '*' | '·' | '×' => "*",
            '/' => "/",
            '^' => "^",
            '(' => "(",
            ')' => ")",
            '[' => "[",
            ']' => "]",
            '{' => "{",
            '}' => "}",
            ',' => ",",
            ';' => ";",
            ':' => ":",
            '=' => "=",
            '<' => "<",
            '>' => ">",
            '≤' => "<=",
            '≥' => ">=",
            '≠' => "!=",
            '|' => "|",
            '∈' => "in",
            'Σ' | '∑' => "sum",
            '→' | '⇒' => "->",
            '!' => "!",
            _ => return err(pos, format!("carácter inesperado «{c}»")),
        };
        adv(&mut i, &mut line, &mut col, &ch);
        if p1 == "in" || p1 == "sum" {
            out.push(Token { tok: Tok::Ident(p1.into()), pos });
        } else {
            out.push(Token { tok: Tok::P(p1), pos });
        }
    }
    out.push(Token { tok: Tok::End, pos: Pos { line, col } });
    Ok(out)
}

// ---------------------------------------------------------------- árbol

#[derive(Debug, Clone)]
pub enum Expr {
    Num(f64),
    Str(String),
    /// Nombre con índices: x[i,j], c[i], T, i.
    Ref { name: String, idx: Vec<Expr>, pos: Pos },
    Neg(Box<Expr>),
    Bin { op: char, l: Box<Expr>, r: Box<Expr>, pos: Pos },
    /// sum{i in I: cond} cuerpo
    Sum { over: Box<Indexing>, body: Box<Expr>, pos: Pos },
    /// Funciones: abs, min, max, sqrt, exp, log, floor, ceil, round, card, pwl, and, or, not, if.
    Call { name: String, args: Vec<Expr>, pos: Pos },
    /// Puntos (x, y) de una función lineal por tramos.
    Points(Vec<(Expr, Expr)>),
    If { cond: Box<Cond>, a: Box<Expr>, b: Box<Expr> },
}

#[derive(Debug, Clone)]
pub enum Cond {
    Cmp { op: &'static str, l: Expr, r: Expr },
    And(Box<Cond>, Box<Cond>),
    Or(Box<Cond>, Box<Cond>),
    Not(Box<Cond>),
    /// i in I (pertenencia de un elemento o una tupla)
    In { items: Vec<Expr>, set: SetExpr },
}

#[derive(Debug, Clone)]
pub enum SetExpr {
    Name(String, Pos),
    /// a..b (enteros)
    Range(Box<Expr>, Box<Expr>),
    /// {A, B, C} o {(A,B), (A,C)}
    Literal(Vec<Vec<Expr>>),
    Union(Box<SetExpr>, Box<SetExpr>),
    Inter(Box<SetExpr>, Box<SetExpr>),
    Diff(Box<SetExpr>, Box<SetExpr>),
    /// {i in I: cond} o {(i,j) in A: …} como conjunto
    Build(Box<Indexing>),
}

#[derive(Debug, Clone)]
pub struct Bind {
    /// Una o varias variables de índice: i o (i, j).
    pub vars: Vec<String>,
    pub set: SetExpr,
}

#[derive(Debug, Clone)]
pub struct Indexing {
    pub binds: Vec<Bind>,
    pub cond: Option<Cond>,
}

#[derive(Debug, Clone)]
pub enum ParamValue {
    Expr(Expr),
    /// [a, b, c] o [[…], […]] en el orden de los conjuntos
    List(Vec<ParamValue>),
    /// {A: 1, (A,B): 2}
    Keyed(Vec<(Vec<Expr>, Expr)>),
}

#[derive(Debug, Clone, Default)]
pub struct VarAttrs {
    pub lb: Option<Expr>,
    pub ub: Option<Expr>,
    /// continuous, integer, binary, semicontinuous, semiinteger
    pub kind: Option<String>,
    pub free: bool,
}

#[derive(Debug, Clone)]
pub enum Rel {
    /// e1 op e2 [op e3]
    Chain(Vec<Expr>, Vec<&'static str>),
}

#[derive(Debug, Clone)]
pub enum ConBody {
    Rel(Rel),
    /// si bin = v entonces relación
    Indicator { bin: Expr, val: f64, then: Rel },
    /// una u otra (al menos una se cumple)
    Either(Vec<Rel>),
    /// atmost / atleast / exactly(k, …), implies(a, b)
    Logic { name: String, args: Vec<Expr> },
}

#[derive(Debug, Clone)]
pub enum Stmt {
    Set { name: String, value: SetExpr, pos: Pos },
    Param { name: String, over: Option<Indexing>, value: Option<ParamValue>, default: Option<Expr>, pos: Pos },
    Var { name: String, over: Option<Indexing>, attrs: VarAttrs, pos: Pos },
    Objective { maximize: bool, name: String, expr: Expr, pos: Pos },
    Constraint { name: String, over: Option<Indexing>, body: ConBody, pos: Pos },
    Option { name: String, value: f64, pos: Pos },
}

// ---------------------------------------------------------------- lector

pub struct Parser {
    t: Vec<Token>,
    i: usize,
    /// Dentro de una condición no hay multiplicación implícita («0 y d > 1»: «y» es «and»).
    in_cond: bool,
}

pub(crate) fn kw(s: &str) -> Option<&'static str> {
    Some(match norm(s).as_str() {
        "set" | "conjunto" | "conj" => "set",
        "param" | "parametro" | "parametros" => "param",
        "var" | "variable" | "variables" | "variavel" | "variaveis" => "var",
        "maximize" | "maximise" | "max" | "maximizar" | "maximice" | "maximo" => "max",
        "minimize" | "minimise" | "min" | "minimizar" | "minimice" | "minimo" => "min",
        "subject" | "sujeto" | "sujeta" | "sujeito" | "st" | "sa" | "restriccion" | "restricciones" | "restricao" | "restricoes" | "constraint" | "constraints" => "st",
        "sum" | "suma" | "soma" | "sumatoria" | "somatorio" => "sum",
        "in" | "en" | "em" => "in",
        "and" | "y" | "e" => "and",
        "or" | "o" | "ou" => "or",
        "not" | "no" | "nao" => "not",
        "if" | "si" | "se" => "if",
        "then" | "entonces" | "entao" => "then",
        "else" | "sino" | "senao" => "else",
        "binary" | "binaria" | "binario" => "binary",
        "integer" | "entera" | "entero" | "inteira" | "inteiro" | "int" => "integer",
        "continuous" | "continua" | "continuo" => "continuous",
        "free" | "libre" | "livre" | "unrestricted" => "free",
        "semicontinuous" | "semicontinua" | "semicontinuo" => "semicontinuous",
        "semiinteger" | "semientera" | "semientero" | "semiinteira" | "semiinteiro" => "semiinteger",
        "default" | "defecto" | "omision" | "padrao" => "default",
        "union" | "uniao" => "union",
        "inter" | "interseccion" | "intersecao" => "inter",
        "diff" | "minus" | "menos" | "diferencia" | "diferenca" => "diff",
        "option" | "opcion" | "opcao" => "option",
        "to" | "a" => "to",
        _ => return None,
    })
}

impl Parser {
    pub fn new(src: &str) -> Result<Parser, Error> {
        Ok(Parser { t: lex(src)?, i: 0, in_cond: false })
    }
    fn peek(&self) -> &Tok {
        &self.t[self.i].tok
    }
    fn peek_at(&self, k: usize) -> &Tok {
        &self.t[(self.i + k).min(self.t.len() - 1)].tok
    }
    fn pos(&self) -> Pos {
        self.t[self.i].pos
    }
    fn next(&mut self) -> Tok {
        let t = self.t[self.i].tok.clone();
        if self.i < self.t.len() - 1 {
            self.i += 1;
        }
        t
    }
    fn is_p(&self, p: &str) -> bool {
        matches!(self.peek(), Tok::P(x) if *x == p)
    }
    fn eat_p(&mut self, p: &str) -> bool {
        if self.is_p(p) {
            self.next();
            true
        } else {
            false
        }
    }
    fn expect_p(&mut self, p: &str) -> Result<(), Error> {
        if self.eat_p(p) {
            Ok(())
        } else {
            err(self.pos(), format!("se esperaba «{p}» y hay {}", self.describe()))
        }
    }
    fn describe(&self) -> String {
        match self.peek() {
            Tok::Ident(s) => format!("«{s}»"),
            Tok::Num(n) => format!("el número {n}"),
            Tok::Str(s) => format!("el texto \"{s}\""),
            Tok::P(p) => format!("«{p}»"),
            Tok::End => "el final del modelo".into(),
        }
    }
    fn is_kw(&self, k: &str) -> bool {
        matches!(self.peek(), Tok::Ident(s) if kw(s) == Some(k))
    }
    fn is_kw_at(&self, off: usize, k: &str) -> bool {
        matches!(self.peek_at(off), Tok::Ident(s) if kw(s) == Some(k))
    }
    fn eat_kw(&mut self, k: &str) -> bool {
        if self.is_kw(k) {
            self.next();
            true
        } else {
            false
        }
    }
    fn ident(&mut self) -> Result<String, Error> {
        match self.next() {
            Tok::Ident(s) => Ok(s),
            _ => {
                self.i -= 1;
                err(self.pos(), format!("se esperaba un nombre y hay {}", self.describe()))
            }
        }
    }

    pub fn program(&mut self) -> Result<Vec<Stmt>, Error> {
        let mut out = vec![];
        while *self.peek() != Tok::End {
            if self.eat_p(";") {
                continue;
            }
            out.push(self.stmt()?);
        }
        Ok(out)
    }

    fn stmt(&mut self) -> Result<Stmt, Error> {
        let pos = self.pos();
        if self.eat_kw("set") {
            let name = self.ident()?;
            if !(self.eat_p("=") || self.eat_p(":=")) {
                return err(self.pos(), format!("se esperaba «=» después del conjunto {name}"));
            }
            let value = self.set_expr()?;
            self.end_stmt()?;
            return Ok(Stmt::Set { name, value, pos });
        }
        if self.eat_kw("param") {
            let name = self.ident()?;
            let over = if self.is_p("{") { Some(self.indexing()?) } else { None };
            let mut default = None;
            let mut value = None;
            loop {
                if self.eat_kw("default") {
                    default = Some(self.expr()?);
                } else if self.eat_p("=") || self.eat_p(":=") {
                    value = Some(self.param_value()?);
                } else {
                    break;
                }
            }
            self.end_stmt()?;
            return Ok(Stmt::Param { name, over, value, default, pos });
        }
        if self.eat_kw("var") {
            let name = self.ident()?;
            let over = if self.is_p("{") { Some(self.indexing()?) } else { None };
            let mut attrs = VarAttrs::default();
            loop {
                if self.eat_p(">=") {
                    attrs.lb = Some(self.expr()?);
                } else if self.eat_p("<=") {
                    attrs.ub = Some(self.expr()?);
                } else if self.eat_p(",") {
                } else if let Tok::Ident(s) = self.peek().clone() {
                    match kw(&s) {
                        Some(k @ ("binary" | "integer" | "continuous" | "semicontinuous" | "semiinteger")) => {
                            self.next();
                            attrs.kind = Some(k.into());
                        }
                        Some("free") => {
                            self.next();
                            attrs.free = true;
                        }
                        _ => break,
                    }
                } else {
                    break;
                }
            }
            self.end_stmt()?;
            return Ok(Stmt::Var { name, over, attrs, pos });
        }
        if self.eat_kw("option") {
            let name = self.ident()?;
            if !(self.eat_p("=") || self.eat_p(":=")) {
                return err(self.pos(), "se esperaba «=»");
            }
            let value = match self.next() {
                Tok::Num(n) => n,
                Tok::P("-") => match self.next() {
                    Tok::Num(n) => -n,
                    _ => return err(pos, "la opción necesita un número"),
                },
                _ => return err(pos, "la opción necesita un número"),
            };
            self.end_stmt()?;
            return Ok(Stmt::Option { name: norm(&name), value, pos });
        }
        if self.is_kw("max") || self.is_kw("min") {
            let maximize = self.is_kw("max");
            self.next();
            // «Maximizar Z = …» o «maximize costo: …»
            let mut name = String::new();
            if let Tok::Ident(s) = self.peek().clone() {
                if matches!(self.peek_at(1), Tok::P(":") | Tok::P("=")) && kw(&s).is_none_or(|k| k != "sum") {
                    self.next();
                    self.next();
                    name = s;
                }
            }
            let expr = self.expr()?;
            self.end_stmt()?;
            return Ok(Stmt::Objective { maximize, name, expr, pos });
        }
        // «subject to» / «sujeto a» / «s.t.»: puede ir antes de la restricción o solo
        if self.is_kw("st") {
            self.next();
            self.eat_kw("to");
            if self.eat_p(":") {
                return self.stmt();
            }
            if matches!(self.peek(), Tok::End) {
                return err(pos, "falta la restricción después de «sujeto a»");
            }
            // «s.t.» seguido directamente de una restricción con nombre o sin él
            return self.constraint(pos);
        }
        self.constraint(pos)
    }

    fn end_stmt(&mut self) -> Result<(), Error> {
        if self.eat_p(";") || matches!(self.peek(), Tok::End) {
            Ok(())
        } else {
            err(self.pos(), format!("se esperaba «;» al final de la instrucción y hay {}", self.describe()))
        }
    }

    fn constraint(&mut self, pos: Pos) -> Result<Stmt, Error> {
        // nombre[{índices}]: … (el nombre es opcional)
        let mut name = String::new();
        let mut over = None;
        if let Tok::Ident(s) = self.peek().clone() {
            let named = match self.peek_at(1) {
                Tok::P(":") => true,
                Tok::P("{") => {
                    // nombre{…}: — hay que ver que después de la llave viene «:»
                    let mut depth = 0;
                    let mut k = 1;
                    loop {
                        match self.peek_at(k) {
                            Tok::P("{") => depth += 1,
                            Tok::P("}") => {
                                depth -= 1;
                                if depth == 0 {
                                    break;
                                }
                            }
                            Tok::End => break,
                            _ => {}
                        }
                        k += 1;
                    }
                    matches!(self.peek_at(k + 1), Tok::P(":"))
                }
                _ => false,
            };
            if named && kw(&s) != Some("sum") {
                self.next();
                name = s;
                if self.is_p("{") {
                    over = Some(self.indexing()?);
                }
                self.expect_p(":")?;
            }
        }
        let body = self.con_body()?;
        self.end_stmt()?;
        Ok(Stmt::Constraint { name, over, body, pos })
    }

    fn con_body(&mut self) -> Result<ConBody, Error> {
        // si y = 1 entonces …
        if self.is_kw("if") && !matches!(self.peek_at(1), Tok::P("(")) || self.is_kw("if") && self.looks_indicator() {
            self.next();
            let (bin, val) = self.indicator_head()?;
            if !self.eat_kw("then") {
                self.expect_p("->")?;
            }
            let then = self.rel()?;
            return Ok(ConBody::Indicator { bin, val, then });
        }
        if let Tok::Ident(s) = self.peek().clone() {
            let n = norm(&s);
            if matches!(n.as_str(), "atmost" | "alomas" | "nomasde" | "nomaximo" | "atleast" | "almenos" | "pelomenos" | "exactly" | "exactamente" | "exatamente" | "implies" | "implica") && matches!(self.peek_at(1), Tok::P("(")) {
                self.next();
                self.next();
                let mut args = vec![];
                while !self.is_p(")") {
                    args.push(self.expr()?);
                    if !self.eat_p(",") && !self.eat_p(";") {
                        break;
                    }
                }
                self.expect_p(")")?;
                let name = match n.as_str() {
                    "atmost" | "alomas" | "nomasde" | "nomaximo" => "atmost",
                    "atleast" | "almenos" | "pelomenos" => "atleast",
                    "exactly" | "exactamente" | "exatamente" => "exactly",
                    _ => "implies",
                };
                return Ok(ConBody::Logic { name: name.into(), args });
            }
        }
        // y = 1 -> … (indicador sin «si»)
        let save = self.i;
        if let Ok((bin, val)) = self.indicator_head() {
            if self.eat_p("->") || self.eat_kw("then") {
                let then = self.rel()?;
                return Ok(ConBody::Indicator { bin, val, then });
            }
        }
        self.i = save;
        let first = self.rel()?;
        if self.is_kw("or") || self.is_p("||") {
            let mut v = vec![first];
            while self.eat_kw("or") || self.eat_p("||") {
                v.push(self.rel()?);
            }
            return Ok(ConBody::Either(v));
        }
        Ok(ConBody::Rel(first))
    }

    fn looks_indicator(&self) -> bool {
        // «si (y[i] = 1) entonces» también vale
        true
    }

    fn indicator_head(&mut self) -> Result<(Expr, f64), Error> {
        let paren = self.eat_p("(");
        let bin = self.primary()?;
        if !(self.eat_p("=") || self.eat_p("==")) {
            return err(self.pos(), "se esperaba «= 0» o «= 1» en la condición");
        }
        let val = match self.next() {
            Tok::Num(v) if v == 0.0 || v == 1.0 => v,
            _ => return err(self.pos(), "la condición de una restricción indicadora es «= 0» o «= 1»"),
        };
        if paren {
            self.expect_p(")")?;
        }
        Ok((bin, val))
    }

    fn rel(&mut self) -> Result<Rel, Error> {
        let mut es = vec![self.expr()?];
        let mut ops = vec![];
        loop {
            let op = match self.peek() {
                Tok::P("<=") | Tok::P("<") => "<=",
                Tok::P(">=") | Tok::P(">") => ">=",
                Tok::P("=") | Tok::P("==") => "=",
                _ => break,
            };
            self.next();
            ops.push(op);
            es.push(self.expr()?);
        }
        if ops.is_empty() {
            return err(self.pos(), format!("se esperaba «<=», «>=» o «=» en la restricción y hay {}", self.describe()));
        }
        if ops.len() > 2 {
            return err(self.pos(), "una restricción tiene a lo sumo dos relaciones (a <= x <= b)");
        }
        Ok(Rel::Chain(es, ops))
    }

    fn param_value(&mut self) -> Result<ParamValue, Error> {
        if self.is_p("[") {
            self.next();
            let mut v = vec![];
            while !self.is_p("]") {
                v.push(self.param_value()?);
                if !self.eat_p(",") {
                    break;
                }
            }
            self.expect_p("]")?;
            return Ok(ParamValue::List(v));
        }
        // {clave: valor, …}: se distingue de una expresión porque lleva «:»
        if self.is_p("{") && self.keyed_ahead() {
            self.next();
            let mut v = vec![];
            while !self.is_p("}") {
                let key = if self.eat_p("(") {
                    let mut k = vec![self.expr()?];
                    while self.eat_p(",") {
                        k.push(self.expr()?);
                    }
                    self.expect_p(")")?;
                    k
                } else {
                    vec![self.atom_key()?]
                };
                self.expect_p(":")?;
                let val = self.expr()?;
                v.push((key, val));
                if !self.eat_p(",") {
                    break;
                }
            }
            self.expect_p("}")?;
            return Ok(ParamValue::Keyed(v));
        }
        Ok(ParamValue::Expr(self.expr()?))
    }

    fn keyed_ahead(&self) -> bool {
        let mut k = 1;
        let mut depth = 0;
        loop {
            match self.peek_at(k) {
                Tok::P("(") | Tok::P("[") => depth += 1,
                Tok::P(")") | Tok::P("]") => depth -= 1,
                Tok::P(":") if depth == 0 => return true,
                Tok::P(",") | Tok::P("}") if depth == 0 => return false,
                Tok::Ident(s) if depth == 0 && kw(s) == Some("in") => return false,
                Tok::End => return false,
                _ => {}
            }
            k += 1;
        }
    }

    /// Clave de un dato: un nombre (se toma como texto) o un número.
    fn atom_key(&mut self) -> Result<Expr, Error> {
        match self.next() {
            Tok::Ident(s) => Ok(Expr::Str(s)),
            Tok::Str(s) => Ok(Expr::Str(s)),
            Tok::Num(n) => Ok(Expr::Num(n)),
            Tok::P("-") => match self.next() {
                Tok::Num(n) => Ok(Expr::Num(-n)),
                _ => err(self.pos(), "se esperaba un número"),
            },
            _ => {
                self.i -= 1;
                err(self.pos(), format!("se esperaba una clave y hay {}", self.describe()))
            }
        }
    }

    // ---------------------------------------------------------- conjuntos

    pub fn set_expr(&mut self) -> Result<SetExpr, Error> {
        let mut l = self.set_atom()?;
        loop {
            if self.eat_kw("union") {
                l = SetExpr::Union(Box::new(l), Box::new(self.set_atom()?));
            } else if self.eat_kw("inter") {
                l = SetExpr::Inter(Box::new(l), Box::new(self.set_atom()?));
            } else if self.eat_kw("diff") {
                l = SetExpr::Diff(Box::new(l), Box::new(self.set_atom()?));
            } else {
                return Ok(l);
            }
        }
    }

    fn set_atom(&mut self) -> Result<SetExpr, Error> {
        let pos = self.pos();
        if self.eat_p("(") {
            let s = self.set_expr()?;
            self.expect_p(")")?;
            return Ok(s);
        }
        if self.is_p("{") {
            // {i in I: …} (conjunto definido por una condición) o {A, B, (A,B)…}
            let builder = matches!(self.peek_at(1), Tok::Ident(_)) && self.is_kw_at(2, "in") || matches!(self.peek_at(1), Tok::P("(")) && self.builder_ahead();
            if builder {
                return Ok(SetExpr::Build(Box::new(self.indexing()?)));
            }
            self.next();
            let mut items = vec![];
            while !self.is_p("}") {
                if self.eat_p("(") {
                    let mut t = vec![self.set_elem()?];
                    while self.eat_p(",") {
                        t.push(self.set_elem()?);
                    }
                    self.expect_p(")")?;
                    items.push(t);
                } else {
                    let a = self.set_elem()?;
                    // A..B dentro de llaves: {1..5}
                    if self.eat_p("..") {
                        let b = self.set_elem()?;
                        self.expect_p("}")?;
                        if !items.is_empty() {
                            return err(pos, "un rango a..b va solo dentro de las llaves");
                        }
                        return Ok(SetExpr::Range(Box::new(a), Box::new(b)));
                    }
                    items.push(vec![a]);
                }
                if !self.eat_p(",") {
                    break;
                }
            }
            self.expect_p("}")?;
            return Ok(SetExpr::Literal(items));
        }
        // a..b
        let save = self.i;
        if let Ok(a) = self.additive() {
            if self.eat_p("..") {
                let b = self.additive()?;
                return Ok(SetExpr::Range(Box::new(a), Box::new(b)));
            }
        }
        self.i = save;
        match self.next() {
            Tok::Ident(s) => Ok(SetExpr::Name(s, pos)),
            _ => {
                self.i -= 1;
                err(pos, format!("se esperaba un conjunto y hay {}", self.describe()))
            }
        }
    }

    /// ¿Desde la posición `off` viene «(i, j) in …»?
    fn builder_ahead_at(&self, off: usize) -> bool {
        let mut k = off + 1;
        loop {
            match self.peek_at(k) {
                Tok::P(")") => return self.is_kw_at(k + 1, "in"),
                Tok::Ident(_) | Tok::P(",") => {}
                _ => return false,
            }
            k += 1;
        }
    }

    fn builder_ahead(&self) -> bool {
        // {(i, j) in A …}
        let mut k = 2;
        loop {
            match self.peek_at(k) {
                Tok::P(")") => return self.is_kw_at(k + 1, "in"),
                Tok::Ident(_) | Tok::P(",") => {}
                _ => return false,
            }
            k += 1;
        }
    }

    /// Elemento de un conjunto escrito: nombre (texto), número, texto entre comillas o expresión entre paréntesis.
    fn set_elem(&mut self) -> Result<Expr, Error> {
        match self.peek().clone() {
            Tok::Ident(s) if !matches!(self.peek_at(1), Tok::P("[")) && !matches!(self.peek_at(1), Tok::P("+") | Tok::P("-") | Tok::P("*") | Tok::P("/")) => {
                self.next();
                Ok(Expr::Str(s))
            }
            _ => self.additive(),
        }
    }

    pub fn indexing(&mut self) -> Result<Indexing, Error> {
        self.expect_p("{")?;
        let mut binds = vec![];
        loop {
            // {I, J}: conjuntos sin nombre de índice (como en AMPL)
            let anonymous = match self.peek() {
                Tok::Ident(_) => !self.is_kw_at(1, "in"),
                Tok::P("(") => !self.builder_ahead_at(0),
                _ => true,
            };
            if anonymous {
                let set = self.set_expr()?;
                binds.push(Bind { vars: vec![], set });
                if !self.eat_p(",") {
                    break;
                }
                continue;
            }
            let vars = if self.eat_p("(") {
                let mut v = vec![self.ident()?];
                while self.eat_p(",") {
                    v.push(self.ident()?);
                }
                self.expect_p(")")?;
                v
            } else {
                vec![self.ident()?]
            };
            if !self.eat_kw("in") {
                return err(self.pos(), format!("se esperaba «in» (o «en») después de {} y hay {}", vars.join(", "), self.describe()));
            }
            let set = self.set_expr()?;
            binds.push(Bind { vars, set });
            if !self.eat_p(",") {
                break;
            }
        }
        let cond = if self.eat_p(":") || self.eat_p("|") { Some(self.cond()?) } else { None };
        self.expect_p("}")?;
        Ok(Indexing { binds, cond })
    }

    // ---------------------------------------------------------- condiciones

    fn cond(&mut self) -> Result<Cond, Error> {
        let prev = self.in_cond;
        self.in_cond = true;
        let r = self.cond_or();
        self.in_cond = prev;
        r
    }
    fn cond_or(&mut self) -> Result<Cond, Error> {
        let mut l = self.cond_and()?;
        while self.eat_kw("or") || self.eat_p("||") {
            l = Cond::Or(Box::new(l), Box::new(self.cond_and()?));
        }
        Ok(l)
    }
    fn cond_and(&mut self) -> Result<Cond, Error> {
        let mut l = self.cond_not()?;
        while self.eat_kw("and") || self.eat_p("&&") {
            l = Cond::And(Box::new(l), Box::new(self.cond_not()?));
        }
        Ok(l)
    }
    fn cond_not(&mut self) -> Result<Cond, Error> {
        if self.eat_kw("not") || self.eat_p("!") {
            return Ok(Cond::Not(Box::new(self.cond_not()?)));
        }
        // (cond) — se prueba primero como condición entre paréntesis
        if self.is_p("(") {
            let save = self.i;
            self.next();
            if let Ok(c) = self.cond() {
                if self.eat_p(")") {
                    return Ok(c);
                }
            }
            self.i = save;
        }
        let l = self.expr()?;
        if self.eat_kw("in") {
            let set = self.set_expr()?;
            return Ok(Cond::In { items: vec![l], set });
        }
        let op = match self.next() {
            Tok::P("<") => "<",
            Tok::P("<=") => "<=",
            Tok::P(">") => ">",
            Tok::P(">=") => ">=",
            Tok::P("=") | Tok::P("==") => "=",
            Tok::P("!=") => "!=",
            _ => {
                self.i -= 1;
                return err(self.pos(), format!("se esperaba una comparación y hay {}", self.describe()));
            }
        };
        let r = self.expr()?;
        Ok(Cond::Cmp { op, l, r })
    }

    // ---------------------------------------------------------- expresiones

    pub fn expr(&mut self) -> Result<Expr, Error> {
        if self.is_kw("if") && !self.looks_constraint_if() {
            self.next();
            let c = self.cond()?;
            if !self.eat_kw("then") {
                return err(self.pos(), "se esperaba «then» (o «entonces»)");
            }
            let a = self.expr()?;
            let b = if self.eat_kw("else") { self.expr()? } else { Expr::Num(0.0) };
            return Ok(Expr::If { cond: Box::new(c), a: Box::new(a), b: Box::new(b) });
        }
        self.additive()
    }

    fn looks_constraint_if(&self) -> bool {
        false
    }

    fn additive(&mut self) -> Result<Expr, Error> {
        let mut l = self.term()?;
        loop {
            let pos = self.pos();
            if self.eat_p("+") {
                l = Expr::Bin { op: '+', l: Box::new(l), r: Box::new(self.term()?), pos };
            } else if self.eat_p("-") {
                l = Expr::Bin { op: '-', l: Box::new(l), r: Box::new(self.term()?), pos };
            } else {
                return Ok(l);
            }
        }
    }

    /// «3 o x >= 8»: una letra que es palabra clave (o, y, e, a) seguida de otro
    /// término es el conector, no una variable multiplicada («3 y + 2» sí es 3·y).
    fn letra_conector(&self) -> bool {
        matches!(self.peek(), Tok::Ident(s) if kw(s).is_some())
            && matches!(self.peek_at(1), Tok::Ident(_) | Tok::Num(_) | Tok::P("("))
    }

    fn term(&mut self) -> Result<Expr, Error> {
        let mut l = self.unary()?;
        loop {
            let pos = self.pos();
            if self.eat_p("*") {
                l = Expr::Bin { op: '*', l: Box::new(l), r: Box::new(self.unary()?), pos };
            } else if self.eat_p("/") {
                l = Expr::Bin { op: '/', l: Box::new(l), r: Box::new(self.unary()?), pos };
            } else if !self.in_cond
                && matches!(l, Expr::Num(_))
                && (matches!(self.peek(), Tok::Ident(s) if kw(s).is_none() || kw(s) == Some("sum") || (s.chars().count() == 1 && !self.letra_conector())) || self.is_p("("))
            {
                // multiplicación implícita: 3x, 3 y, 2(x + y), 5 sum{…}
                l = Expr::Bin { op: '*', l: Box::new(l), r: Box::new(self.unary()?), pos };
            } else {
                return Ok(l);
            }
        }
    }

    fn unary(&mut self) -> Result<Expr, Error> {
        if self.eat_p("-") {
            return Ok(Expr::Neg(Box::new(self.unary()?)));
        }
        if self.eat_p("+") {
            return self.unary();
        }
        let base = self.primary()?;
        let pos = self.pos();
        if self.eat_p("^") {
            let e = self.unary()?;
            return Ok(Expr::Bin { op: '^', l: Box::new(base), r: Box::new(e), pos });
        }
        Ok(base)
    }

    fn primary(&mut self) -> Result<Expr, Error> {
        let pos = self.pos();
        match self.next() {
            Tok::Num(n) => Ok(Expr::Num(n)),
            Tok::Str(s) => Ok(Expr::Str(s)),
            Tok::P("(") => {
                let e = self.expr()?;
                // un punto (x, y) de una función por tramos
                if self.eat_p(",") {
                    let f = self.expr()?;
                    self.expect_p(")")?;
                    return Ok(Expr::Points(vec![(e, f)]));
                }
                self.expect_p(")")?;
                Ok(e)
            }
            Tok::Ident(s) if kw(&s) == Some("sum") => {
                let over = self.indexing()?;
                let body = self.unary_body()?;
                Ok(Expr::Sum { over: Box::new(over), body: Box::new(body), pos })
            }
            Tok::Ident(s) => {
                if self.is_p("(") {
                    self.next();
                    let mut args = vec![];
                    let n = norm(&s);
                    while !self.is_p(")") {
                        // pwl(x; (0,0), (10,5)) — el «;» separa la variable de los puntos
                        args.push(self.expr()?);
                        if !self.eat_p(",") && !self.eat_p(";") {
                            break;
                        }
                    }
                    self.expect_p(")")?;
                    return Ok(Expr::Call { name: n, args, pos });
                }
                let mut idx = vec![];
                if self.eat_p("[") {
                    while !self.is_p("]") {
                        idx.push(self.expr()?);
                        if !self.eat_p(",") {
                            break;
                        }
                    }
                    self.expect_p("]")?;
                }
                Ok(Expr::Ref { name: s, idx, pos })
            }
            _ => {
                self.i -= 1;
                err(pos, format!("se esperaba un número, un nombre o «(» y hay {}", self.describe()))
            }
        }
    }

    /// Cuerpo de una suma: un término (los productos se agrupan; la suma termina en + o −).
    fn unary_body(&mut self) -> Result<Expr, Error> {
        if self.is_p("(") {
            return self.term();
        }
        self.term()
    }
}

pub fn parse(src: &str) -> Result<Vec<Stmt>, Error> {
    Parser::new(src)?.program()
}
