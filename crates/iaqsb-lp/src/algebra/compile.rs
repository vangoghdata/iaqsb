//! Del árbol del lenguaje al modelo del motor: conjuntos, parámetros, variables
//! indexadas, sumas, restricciones indexadas y restricciones generales (valor
//! absoluto, máximo, mínimo, lineal por tramos, indicadoras, disyunciones y
//! lógicas), estas últimas con big-M calculado a partir de las cotas.

use super::parse::{err, Bind, Cond, ConBody, Error, Expr, Indexing, ParamValue, Pos, Rel, SetExpr, Stmt, VarAttrs};
use iaqsb_core::solver::{self, Kind, Model};
use serde_json::Value;
use std::collections::{HashMap, HashSet};

/// Elemento de un conjunto: número o nombre.
#[derive(Debug, Clone, PartialEq)]
pub enum Elem {
    N(f64),
    S(String),
}

impl Elem {
    pub fn key(&self) -> String {
        match self {
            Elem::N(x) => fmt_num(*x),
            Elem::S(s) => s.clone(),
        }
    }
}

pub fn fmt_num(x: f64) -> String {
    if x.fract() == 0.0 && x.abs() < 1e15 {
        format!("{}", x as i64)
    } else {
        format!("{x}")
    }
}

type Key = Vec<String>;

fn key_of(t: &[Elem]) -> Key {
    t.iter().map(|e| e.key()).collect()
}

struct SetData {
    items: Vec<Vec<Elem>>,
    keys: HashSet<Key>,
}

struct ParamData {
    arity: usize,
    values: HashMap<Key, f64>,
    default: Option<f64>,
}

/// Una familia de variables o de restricciones (x[i,j], oferta[i]).
#[derive(Debug, Clone)]
pub struct Family {
    pub name: String,
    /// Nombres de los conjuntos de cada índice (para los títulos de las tablas).
    pub sets: Vec<String>,
    /// (índices, columna o fila del modelo)
    pub entries: Vec<(Vec<String>, usize)>,
}

/// Lo que se armó: el modelo y cómo leer sus columnas y filas por índices.
#[derive(Debug, Clone, Default)]
pub struct Compiled {
    pub model: Model,
    pub vars: Vec<Family>,
    pub cons: Vec<Family>,
    /// Cómo se reformularon las restricciones generales (para explicarlo).
    pub notes: Vec<String>,
    /// Conjuntos y su tamaño.
    pub sets: Vec<(String, usize)>,
    /// Columnas auxiliares agregadas por las restricciones generales.
    pub aux: usize,
}

#[derive(Debug, Clone, Default)]
struct Lin {
    terms: Vec<(usize, f64)>,
    c: f64,
    quad: Vec<(usize, usize, f64)>,
}

impl Lin {
    fn konst(c: f64) -> Lin {
        Lin { c, ..Default::default() }
    }
    fn var(j: usize) -> Lin {
        Lin { terms: vec![(j, 1.0)], ..Default::default() }
    }
    fn is_const(&self) -> bool {
        self.terms.iter().all(|t| t.1 == 0.0) && self.quad.is_empty()
    }
    fn add(mut self, o: Lin, k: f64) -> Lin {
        self.terms.extend(o.terms.into_iter().map(|(j, a)| (j, a * k)));
        self.quad.extend(o.quad.into_iter().map(|(i, j, a)| (i, j, a * k)));
        self.c += o.c * k;
        self
    }
    fn scale(mut self, k: f64) -> Lin {
        for t in &mut self.terms {
            t.1 *= k;
        }
        for q in &mut self.quad {
            q.2 *= k;
        }
        self.c *= k;
        self
    }
    /// Junta los términos de la misma columna.
    fn merged(&self) -> Vec<(usize, f64)> {
        let mut m: HashMap<usize, f64> = HashMap::new();
        let mut order = vec![];
        for (j, a) in &self.terms {
            if !m.contains_key(j) {
                order.push(*j);
            }
            *m.entry(*j).or_default() += a;
        }
        order.into_iter().filter_map(|j| m.get(&j).filter(|a| **a != 0.0).map(|a| (j, *a))).collect()
    }
}

pub struct Ctx<'a> {
    m: Model,
    sets: HashMap<String, SetData>,
    set_order: Vec<String>,
    params: HashMap<String, ParamData>,
    vars: HashMap<String, Family>,
    /// índices → columna, por familia (búsqueda rápida)
    var_index: HashMap<String, HashMap<Key, usize>>,
    var_order: Vec<String>,
    var_arity: HashMap<String, usize>,
    cons: Vec<Family>,
    binds: Vec<(String, Elem)>,
    notes: Vec<String>,
    bigm: Option<f64>,
    aux: usize,
    objectives: usize,
    /// Nombre del primer objetivo (para los modelos con varios).
    obj1: String,
    data: Option<&'a Value>,
    cols_limit: usize,
    /// Sin ninguna declaración «var», los nombres sueltos son variables ≥ 0 (modo rápido: «max 3x + 5y; …»).
    auto_vars: bool,
}

const COLS_LIMIT: usize = 5_000_000;

impl<'a> Ctx<'a> {
    fn lookup_bind(&self, name: &str) -> Option<&Elem> {
        self.binds.iter().rev().find(|(n, _)| n == name).map(|(_, e)| e)
    }

    // ------------------------------------------------------------ conjuntos

    fn set_items(&mut self, s: &SetExpr) -> Result<Vec<Vec<Elem>>, Error> {
        match s {
            SetExpr::Name(n, pos) => match self.sets.get(n) {
                Some(d) => Ok(d.items.clone()),
                None => err(*pos, format!("el conjunto «{n}» no está definido (defínelo antes con set {n} = {{…}};)")),
            },
            SetExpr::Range(a, b) => {
                let (a, b) = (self.num(a)?, self.num(b)?);
                if (b - a) > 10_000_000.0 {
                    return err(Pos::default(), "el rango es demasiado grande");
                }
                let (a, b) = (a.round() as i64, b.round() as i64);
                Ok((a..=b).map(|k| vec![Elem::N(k as f64)]).collect())
            }
            SetExpr::Literal(items) => {
                let mut out = vec![];
                for t in items {
                    let mut v = vec![];
                    for e in t {
                        v.push(self.elem(e)?);
                    }
                    out.push(v);
                }
                Ok(out)
            }
            SetExpr::Union(a, b) | SetExpr::Inter(a, b) | SetExpr::Diff(a, b) => {
                let (x, y) = (self.set_items(a)?, self.set_items(b)?);
                let ky: HashSet<Key> = y.iter().map(|t| key_of(t)).collect();
                Ok(match s {
                    SetExpr::Union(..) => {
                        let kx: HashSet<Key> = x.iter().map(|t| key_of(t)).collect();
                        let mut v = x;
                        v.extend(y.into_iter().filter(|t| !kx.contains(&key_of(t))));
                        v
                    }
                    SetExpr::Inter(..) => x.into_iter().filter(|t| ky.contains(&key_of(t))).collect(),
                    _ => x.into_iter().filter(|t| !ky.contains(&key_of(t))).collect(),
                })
            }
            SetExpr::Build(ix) => {
                let mut out = vec![];
                let ix = &self.named(ix)?;
                let vars: Vec<String> = ix.binds.iter().flat_map(|b| b.vars.clone()).collect();
                self.each(ix, &mut |c| {
                    let t: Vec<Elem> = vars.iter().map(|v| c.lookup_bind(v).cloned().unwrap()).collect();
                    out.push(t);
                    Ok(())
                })?;
                Ok(out)
            }
        }
    }

    fn set_name(s: &SetExpr) -> String {
        match s {
            SetExpr::Name(n, _) => n.clone(),
            SetExpr::Range(..) => "".into(),
            _ => "".into(),
        }
    }

    /// Recorre todas las combinaciones de índices que cumplen la condición.
    fn each(&mut self, ix: &Indexing, f: &mut dyn FnMut(&mut Ctx<'a>) -> Result<(), Error>) -> Result<(), Error> {
        self.each_from(&ix.binds, 0, &ix.cond, f)
    }

    fn each_from(&mut self, binds: &[Bind], k: usize, cond: &Option<Cond>, f: &mut dyn FnMut(&mut Ctx<'a>) -> Result<(), Error>) -> Result<(), Error> {
        if k == binds.len() {
            if let Some(c) = cond {
                if !self.cond(c)? {
                    return Ok(());
                }
            }
            return f(self);
        }
        let b = &binds[k];
        let items = self.set_items(&b.set)?;
        for t in items {
            if t.len() != b.vars.len() {
                return err(Pos::default(), format!("el conjunto {} tiene elementos de {} índices y se recorre con {} ({})", Self::set_name(&b.set), t.len(), b.vars.len(), b.vars.join(", ")));
            }
            let n0 = self.binds.len();
            for (v, e) in b.vars.iter().zip(t) {
                self.binds.push((v.clone(), e));
            }
            let r = self.each_from(binds, k + 1, cond, f);
            self.binds.truncate(n0);
            r?;
        }
        Ok(())
    }

    // ------------------------------------------------------------ valores

    fn elem(&mut self, e: &Expr) -> Result<Elem, Error> {
        match e {
            Expr::Str(s) => Ok(Elem::S(s.clone())),
            Expr::Num(x) => Ok(Elem::N(*x)),
            Expr::Ref { name, idx, .. } if idx.is_empty() => {
                if let Some(b) = self.lookup_bind(name) {
                    return Ok(b.clone());
                }
                if let Some(p) = self.params.get(name).filter(|p| p.arity == 0) {
                    if let Some(v) = p.values.get(&vec![]).copied().or(p.default) {
                        return Ok(Elem::N(v));
                    }
                }
                // un nombre suelto como índice es un elemento: x[Bogota]
                Ok(Elem::S(name.clone()))
            }
            other => Ok(Elem::N(self.num(other)?)),
        }
    }

    fn num(&mut self, e: &Expr) -> Result<f64, Error> {
        let l = self.lin(e, false)?;
        if !l.is_const() {
            return err(pos_of(e), "aquí va un número o un dato, no una variable de decisión");
        }
        Ok(l.c)
    }

    fn cond(&mut self, c: &Cond) -> Result<bool, Error> {
        Ok(match c {
            Cond::And(a, b) => self.cond(a)? && self.cond(b)?,
            Cond::Or(a, b) => self.cond(a)? || self.cond(b)?,
            Cond::Not(a) => !self.cond(a)?,
            Cond::Cmp { op, l, r } => {
                let (a, b) = (self.elem(l)?, self.elem(r)?);
                match (a, b) {
                    (Elem::N(x), Elem::N(y)) => match *op {
                        "<" => x < y,
                        "<=" => x <= y + 1e-12,
                        ">" => x > y,
                        ">=" => x + 1e-12 >= y,
                        "=" => (x - y).abs() <= 1e-12,
                        _ => (x - y).abs() > 1e-12,
                    },
                    (a, b) => match *op {
                        "=" => a.key() == b.key(),
                        "!=" => a.key() != b.key(),
                        _ => return err(pos_of(l), "solo se pueden comparar con < o > números"),
                    },
                }
            }
            Cond::In { items, set } => {
                let mut t = vec![];
                for e in items {
                    if let Expr::Points(p) = e {
                        for (a, b) in p {
                            t.push(self.elem(a)?);
                            t.push(self.elem(b)?);
                        }
                    } else {
                        t.push(self.elem(e)?);
                    }
                }
                let k = key_of(&t);
                if let SetExpr::Name(n, _) = set {
                    if let Some(d) = self.sets.get(n) {
                        return Ok(d.keys.contains(&k));
                    }
                }
                self.set_items(set)?.iter().any(|x| key_of(x) == k)
            }
        })
    }

    fn param_value(&mut self, name: &str, idx: &[Expr], pos: Pos) -> Result<f64, Error> {
        let mut k = vec![];
        for e in idx {
            k.push(self.elem(e)?.key());
        }
        let p = self.params.get(name).ok_or_else(|| Error { pos, msg: format!("«{name}» no está definido") })?;
        if k.len() != p.arity {
            return err(pos, format!("«{name}» lleva {} índices y aquí tiene {}", p.arity, k.len()));
        }
        match p.values.get(&k).copied().or(p.default) {
            Some(v) => Ok(v),
            None => err(pos, format!("falta el dato {name}[{}] (dalo o pon un valor por defecto: param {name}{{…}} default 0;)", k.join(","))),
        }
    }

    // ------------------------------------------------------------ expresiones lineales

    fn lin(&mut self, e: &Expr, quad: bool) -> Result<Lin, Error> {
        match e {
            Expr::Num(x) => Ok(Lin::konst(*x)),
            Expr::Str(s) => err(Pos::default(), format!("«{s}» es un texto: aquí va un número")),
            Expr::Neg(a) => Ok(self.lin(a, quad)?.scale(-1.0)),
            Expr::Points(_) => err(pos_of(e), "un par (x, y) solo va dentro de pwl( … )"),
            Expr::Ref { name, idx, pos } => {
                if idx.is_empty() {
                    if let Some(b) = self.lookup_bind(name).cloned() {
                        return match b {
                            Elem::N(x) => Ok(Lin::konst(x)),
                            Elem::S(s) => err(*pos, format!("el índice {name} vale «{s}», que no es un número")),
                        };
                    }
                }
                if self.var_arity.contains_key(name) {
                    let ar = self.var_arity[name];
                    if ar != idx.len() {
                        return err(*pos, format!("la variable {name} lleva {ar} índices y aquí tiene {}", idx.len()));
                    }
                    let mut k = vec![];
                    for x in idx {
                        k.push(self.elem(x)?.key());
                    }
                    let col = self.var_index.get(name).and_then(|m| m.get(&k)).copied();
                    return match col {
                        Some(c) => Ok(Lin::var(c)),
                        None => err(*pos, format!("la variable {name}[{}] no existe (no está en sus conjuntos)", k.join(","))),
                    };
                }
                if self.params.contains_key(name) {
                    return Ok(Lin::konst(self.param_value(name, idx, *pos)?));
                }
                if self.sets.contains_key(name) {
                    return err(*pos, format!("{name} es un conjunto: usa card({name}) para su tamaño o sum{{i in {name}}} …"));
                }
                if self.auto_vars && idx.is_empty() {
                    let j = self.m.add_col(name.clone(), 0.0, 0.0, f64::INFINITY, Kind::Continuous);
                    self.var_arity.insert(name.clone(), 0);
                    self.var_order.push(name.clone());
                    self.var_index.insert(name.clone(), [(vec![], j)].into_iter().collect());
                    self.vars.insert(name.clone(), Family { name: name.clone(), sets: vec![], entries: vec![(vec![], j)] });
                    return Ok(Lin::var(j));
                }
                err(*pos, format!("«{name}» no está definido (¿falta declararlo con param, var o set?)"))
            }
            Expr::Bin { op, l, r, pos } => {
                let a = self.lin(l, quad)?;
                let b = self.lin(r, quad)?;
                match op {
                    '+' => Ok(a.add(b, 1.0)),
                    '-' => Ok(a.add(b, -1.0)),
                    '*' => {
                        if a.is_const() {
                            Ok(b.scale(a.c))
                        } else if b.is_const() {
                            Ok(a.scale(b.c))
                        } else if quad && a.quad.is_empty() && b.quad.is_empty() {
                            // producto de dos expresiones lineales (objetivo cuadrático)
                            let mut out = Lin::konst(a.c * b.c);
                            for (j, x) in &a.terms {
                                out.terms.push((*j, x * b.c));
                            }
                            for (j, x) in &b.terms {
                                out.terms.push((*j, x * a.c));
                            }
                            for (i, x) in &a.terms {
                                for (j, y) in &b.terms {
                                    out.quad.push((*i, *j, x * y));
                                }
                            }
                            Ok(out)
                        } else {
                            err(*pos, "producto de variables de decisión: el modelo no es lineal (en el objetivo se admite un término cuadrático; en las restricciones, no)")
                        }
                    }
                    '/' => {
                        if !b.is_const() {
                            return err(*pos, "no se puede dividir entre una variable de decisión");
                        }
                        if b.c == 0.0 {
                            return err(*pos, "división entre cero");
                        }
                        Ok(a.scale(1.0 / b.c))
                    }
                    _ => {
                        if !b.is_const() {
                            return err(*pos, "el exponente debe ser un número");
                        }
                        if a.is_const() {
                            return Ok(Lin::konst(a.c.powf(b.c)));
                        }
                        if b.c == 1.0 {
                            return Ok(a);
                        }
                        if b.c == 2.0 && quad && a.quad.is_empty() {
                            return self.lin(&Expr::Bin { op: '*', l: l.clone(), r: l.clone(), pos: *pos }, quad);
                        }
                        err(*pos, "potencia de una variable de decisión: el modelo no es lineal")
                    }
                }
            }
            Expr::Sum { over, body, .. } => {
                let mut acc = Lin::default();
                let body = body.clone();
                let over = self.named(over)?;
                self.each(&over, &mut |c| {
                    let x = c.lin(&body, quad)?;
                    acc = std::mem::take(&mut acc).add(x, 1.0);
                    Ok(())
                })?;
                Ok(acc)
            }
            Expr::If { cond, a, b } => {
                if self.cond(cond)? {
                    self.lin(a, quad)
                } else {
                    self.lin(b, quad)
                }
            }
            Expr::Call { name, args, pos } => self.call(name, args, *pos, quad),
        }
    }

    fn call(&mut self, name: &str, args: &[Expr], pos: Pos, quad: bool) -> Result<Lin, Error> {
        // card(Conjunto)
        if name == "card" || name == "tamano" || name == "tamanho" {
            if let [Expr::Ref { name: n, idx, .. }] = args {
                if idx.is_empty() {
                    let s = SetExpr::Name(n.clone(), pos);
                    return Ok(Lin::konst(self.set_items(&s)?.len() as f64));
                }
            }
            return err(pos, "card lleva el nombre de un conjunto: card(I)");
        }
        if name == "pwl" || name == "tramos" || name == "portramos" {
            return self.pwl(args, pos);
        }
        let mut vals = vec![];
        for a in args {
            vals.push(self.lin(a, false)?);
        }
        let all_const = vals.iter().all(|v| v.is_const());
        let one = |v: &[Lin]| -> Result<f64, Error> {
            if v.len() != 1 {
                return err(pos, format!("{name} lleva un argumento"));
            }
            Ok(v[0].c)
        };
        if all_const {
            let c: Vec<f64> = vals.iter().map(|v| v.c).collect();
            let x = match name {
                "abs" => one(&vals)?.abs(),
                "sqrt" | "raiz" => one(&vals)?.sqrt(),
                "exp" => one(&vals)?.exp(),
                "log" | "ln" => one(&vals)?.ln(),
                "log10" => one(&vals)?.log10(),
                "floor" | "piso" => one(&vals)?.floor(),
                "ceil" | "techo" | "teto" => one(&vals)?.ceil(),
                "round" | "redondear" | "arredondar" => one(&vals)?.round(),
                "min" | "minimo" => c.iter().cloned().fold(f64::INFINITY, f64::min),
                "max" | "maximo" => c.iter().cloned().fold(f64::NEG_INFINITY, f64::max),
                "not" | "no" | "nao" => 1.0 - one(&vals)?,
                "and" | "y" | "e" => c.iter().all(|x| *x > 0.5) as i32 as f64,
                "or" | "o" | "ou" => c.iter().any(|x| *x > 0.5) as i32 as f64,
                other => return err(pos, format!("función desconocida: {other}")),
            };
            if !x.is_finite() {
                return err(pos, format!("{name} no da un número finito aquí"));
            }
            return Ok(Lin::konst(x));
        }
        let _ = quad;
        match name {
            "abs" => {
                let e = vals.into_iter().next().ok_or_else(|| Error { pos, msg: "abs lleva un argumento".into() })?;
                self.gen_abs(e, pos)
            }
            "max" | "maximo" => self.gen_maxmin(vals, true, pos),
            "min" | "minimo" => self.gen_maxmin(vals, false, pos),
            "not" | "no" | "nao" => {
                let b = self.binary_arg(&vals[0], pos)?;
                Ok(Lin::konst(1.0).add(b, -1.0))
            }
            "and" | "y" | "e" | "or" | "o" | "ou" => {
                let and = matches!(name, "and" | "y" | "e");
                let mut bs = vec![];
                for v in &vals {
                    bs.push(self.binary_arg(v, pos)?);
                }
                let z = self.aux_col(if and { "_and" } else { "_or" }, 0.0, 1.0, Kind::Binary);
                let n = bs.len() as f64;
                for b in &bs {
                    // y ≤ b (and) o y ≥ b (or)
                    let row = Lin::var(z).add(b.clone(), -1.0);
                    if and {
                        self.add_row("_and", &row, f64::NEG_INFINITY, 0.0);
                    } else {
                        self.add_row("_or", &row, 0.0, f64::INFINITY);
                    }
                }
                let mut sum = Lin::default();
                for b in bs {
                    sum = sum.add(b, 1.0);
                }
                if and {
                    // y ≥ Σb − (n − 1)
                    let row = Lin::var(z).add(sum, -1.0);
                    self.add_row("_and", &row, -(n - 1.0), f64::INFINITY);
                } else {
                    let row = Lin::var(z).add(sum, -1.0);
                    self.add_row("_or", &row, f64::NEG_INFINITY, 0.0);
                }
                self.notes.push(format!("{} de {} binarias: variable binaria nueva con {} restricciones.", if and { "Y lógico" } else { "O lógico" }, n, n + 1.0));
                Ok(Lin::var(z))
            }
            "sqrt" | "raiz" | "exp" | "log" | "ln" | "log10" | "floor" | "piso" | "ceil" | "techo" | "teto" | "round" | "redondear" | "arredondar" => {
                err(pos, format!("la función {name} no admite variables de decisión"))
            }
            other => err(pos, format!("función desconocida: {other}")),
        }
    }

    /// Un argumento binario (una variable binaria, o 1 − binaria).
    fn binary_arg(&self, v: &Lin, pos: Pos) -> Result<Lin, Error> {
        let t = v.merged();
        let ok = t.iter().all(|(j, _)| self.m.cols[*j].kind == Kind::Binary) && v.quad.is_empty();
        let lo = v.c + t.iter().map(|(_, a)| a.min(0.0)).sum::<f64>();
        let hi = v.c + t.iter().map(|(_, a)| a.max(0.0)).sum::<f64>();
        if !ok || lo < -1e-9 || hi > 1.0 + 1e-9 {
            return err(pos, "las funciones lógicas (and, or, not) y las indicadoras van con variables binarias");
        }
        Ok(v.clone())
    }

    /// Cotas de una expresión lineal según las cotas de sus variables.
    fn bounds(&self, v: &Lin) -> (f64, f64) {
        let mut lo = v.c;
        let mut hi = v.c;
        for (j, a) in v.merged() {
            let c = &self.m.cols[j];
            let (l, u) = (if c.kind == Kind::SemiContinuous || c.kind == Kind::SemiInteger { c.lb.min(0.0) } else { c.lb }, c.ub);
            if a > 0.0 {
                lo += a * l;
                hi += a * u;
            } else {
                lo += a * u;
                hi += a * l;
            }
        }
        (lo, hi)
    }

    fn finite(&self, (lo, hi): (f64, f64), what: &str, pos: Pos) -> Result<(f64, f64), Error> {
        if lo.is_finite() && hi.is_finite() {
            return Ok((lo, hi));
        }
        match self.bigm {
            Some(m) => Ok((if lo.is_finite() { lo } else { -m }, if hi.is_finite() { hi } else { m })),
            None => err(
                pos,
                format!("para {what} hacen falta cotas finitas en las variables que intervienen (p. ej. var x <= 100;) o un big-M general: option bigM = 10000;"),
            ),
        }
    }

    fn aux_col(&mut self, base: &str, lb: f64, ub: f64, kind: Kind) -> usize {
        self.aux += 1;
        self.m.add_col(format!("{base}{}", self.aux), 0.0, lb, ub, kind)
    }

    fn add_row(&mut self, name: &str, l: &Lin, lo: f64, hi: f64) -> usize {
        let coefs = l.merged();
        let (lo, hi) = (lo - l.c, hi - l.c);
        self.m.add_row(format!("{name}{}", self.m.rows.len() + 1), coefs, lo, hi)
    }

    fn gen_abs(&mut self, e: Lin, pos: Pos) -> Result<Lin, Error> {
        let (lo, hi) = self.finite(self.bounds(&e), "abs( )", pos)?;
        if lo >= 0.0 {
            return Ok(e);
        }
        if hi <= 0.0 {
            return Ok(e.scale(-1.0));
        }
        let t = self.aux_col("_abs", 0.0, (-lo).max(hi), Kind::Continuous);
        let b = self.aux_col("_absb", 0.0, 1.0, Kind::Binary);
        let tv = Lin::var(t);
        // t ≥ e y t ≥ −e
        self.add_row("_abs", &tv.clone().add(e.clone(), -1.0), 0.0, f64::INFINITY);
        self.add_row("_abs", &tv.clone().add(e.clone(), 1.0), 0.0, f64::INFINITY);
        // b = 1 ⇔ e ≥ 0: e ≥ L(1 − b), e ≤ U·b
        self.add_row("_abs", &e.clone().add(Lin::var(b), lo), lo, f64::INFINITY);
        self.add_row("_abs", &e.clone().add(Lin::var(b), -hi), f64::NEG_INFINITY, 0.0);
        // t ≤ e + (−2L)(1 − b) y t ≤ −e + 2U·b
        self.add_row("_abs", &tv.clone().add(e.clone(), -1.0).add(Lin::var(b), -2.0 * lo), f64::NEG_INFINITY, -2.0 * lo);
        self.add_row("_abs", &tv.add(e, 1.0).add(Lin::var(b), -2.0 * hi), f64::NEG_INFINITY, 0.0);
        self.notes.push(format!("Valor absoluto con big-M (cotas {} y {}): 2 variables nuevas (una binaria) y 6 restricciones.", fmt_num(lo), fmt_num(hi)));
        Ok(Lin::var(t))
    }

    fn gen_maxmin(&mut self, vals: Vec<Lin>, is_max: bool, pos: Pos) -> Result<Lin, Error> {
        if vals.is_empty() {
            return err(pos, "max y min llevan al menos un argumento");
        }
        if vals.len() == 1 {
            return Ok(vals.into_iter().next().unwrap());
        }
        let what = if is_max { "max( )" } else { "min( )" };
        let mut b: Vec<(f64, f64)> = vec![];
        for v in &vals {
            b.push(self.finite(self.bounds(v), what, pos)?);
        }
        // con el signo cambiado, min es max
        let s = if is_max { 1.0 } else { -1.0 };
        let vals: Vec<Lin> = vals.into_iter().map(|v| v.scale(s)).collect();
        let bs: Vec<(f64, f64)> = b.iter().map(|&(l, u)| if is_max { (l, u) } else { (-u, -l) }).collect();
        let lmax = bs.iter().map(|x| x.0).fold(f64::NEG_INFINITY, f64::max);
        let umax = bs.iter().map(|x| x.1).fold(f64::NEG_INFINITY, f64::max);
        let y = self.aux_col(if is_max { "_max" } else { "_min" }, lmax, umax, Kind::Continuous);
        let mut pick = Lin::default();
        for (k, v) in vals.iter().enumerate() {
            // y ≥ e_k
            self.add_row("_max", &Lin::var(y).add(v.clone(), -1.0), 0.0, f64::INFINITY);
            // y ≤ e_k + (Umax − L_k)(1 − b_k)
            let bk = self.aux_col("_maxb", 0.0, 1.0, Kind::Binary);
            let m = umax - bs[k].0;
            self.add_row("_max", &Lin::var(y).add(v.clone(), -1.0).add(Lin::var(bk), m), f64::NEG_INFINITY, m);
            pick = pick.add(Lin::var(bk), 1.0);
        }
        self.add_row("_max", &pick, 1.0, 1.0);
        self.notes.push(format!("{} de {} expresiones con big-M: {} variables nuevas ({} binarias) y {} restricciones.", if is_max { "Máximo" } else { "Mínimo" }, vals.len(), vals.len() + 1, vals.len(), 2 * vals.len() + 1));
        Ok(Lin::var(y).scale(s))
    }

    /// pwl(x; (x0, f0), (x1, f1), …): f(x) lineal por tramos (formulación λ con binarias por tramo).
    fn pwl(&mut self, args: &[Expr], pos: Pos) -> Result<Lin, Error> {
        if args.len() < 3 {
            return err(pos, "pwl lleva la expresión y al menos dos puntos: pwl(x; (0, 0), (10, 50), (20, 80))");
        }
        let x = self.lin(&args[0], false)?;
        let mut pts: Vec<(f64, f64)> = vec![];
        for a in &args[1..] {
            match a {
                Expr::Points(p) => {
                    for (u, v) in p {
                        pts.push((self.num(u)?, self.num(v)?));
                    }
                }
                _ => return err(pos_of(a), "cada punto de pwl se escribe (x, f(x))"),
            }
        }
        if pts.windows(2).any(|w| w[1].0 <= w[0].0) {
            return err(pos, "los puntos de pwl van con x creciente");
        }
        let n = pts.len();
        let lam: Vec<usize> = (0..n).map(|_| self.aux_col("_pwl", 0.0, 1.0, Kind::Continuous)).collect();
        let z: Vec<usize> = (0..n - 1).map(|_| self.aux_col("_pwlz", 0.0, 1.0, Kind::Binary)).collect();
        let mut sum_l = Lin::default();
        let mut xl = Lin::default();
        let mut fl = Lin::default();
        for (k, &l) in lam.iter().enumerate() {
            sum_l = sum_l.add(Lin::var(l), 1.0);
            xl = xl.add(Lin::var(l), pts[k].0);
            fl = fl.add(Lin::var(l), pts[k].1);
        }
        self.add_row("_pwl", &sum_l, 1.0, 1.0);
        self.add_row("_pwl", &x.add(xl, -1.0), 0.0, 0.0);
        let mut sum_z = Lin::default();
        for &zk in &z {
            sum_z = sum_z.add(Lin::var(zk), 1.0);
        }
        self.add_row("_pwl", &sum_z, 1.0, 1.0);
        for k in 0..n {
            // λ_k ≤ z_{k−1} + z_k (solo dos puntos vecinos pueden ser positivos)
            let mut r = Lin::var(lam[k]);
            if k > 0 {
                r = r.add(Lin::var(z[k - 1]), -1.0);
            }
            if k < n - 1 {
                r = r.add(Lin::var(z[k]), -1.0);
            }
            self.add_row("_pwl", &r, f64::NEG_INFINITY, 0.0);
        }
        self.notes.push(format!("Función lineal por tramos de {} puntos: {} variables nuevas ({} binarias); la expresión debe quedar entre {} y {}.", n, 2 * n - 1, n - 1, fmt_num(pts[0].0), fmt_num(pts[n - 1].0)));
        Ok(fl)
    }

    // ------------------------------------------------------------ declaraciones

    fn data_for(&self, name: &str) -> Option<Value> {
        let d = self.data?;
        d.get(name).or_else(|| d.get("params").and_then(|p| p.get(name))).or_else(|| d.get("sets").and_then(|p| p.get(name))).cloned()
    }

    fn declare_set(&mut self, name: &str, value: &SetExpr, pos: Pos) -> Result<(), Error> {
        let items = match self.data_for(name) {
            Some(Value::Array(a)) => a
                .iter()
                .map(|x| match x {
                    Value::Array(t) => t.iter().map(json_elem).collect(),
                    other => vec![json_elem(other)],
                })
                .collect(),
            _ => self.set_items(value)?,
        };
        let keys: HashSet<Key> = items.iter().map(|t| key_of(t)).collect();
        if keys.len() != items.len() {
            return err(pos, format!("el conjunto {name} tiene elementos repetidos"));
        }
        if self.sets.contains_key(name) || self.params.contains_key(name) || self.var_arity.contains_key(name) {
            return err(pos, format!("«{name}» ya está definido"));
        }
        self.set_order.push(name.into());
        self.sets.insert(name.into(), SetData { items, keys });
        Ok(())
    }

    fn declare_param(&mut self, name: &str, over: &Option<Indexing>, value: &Option<ParamValue>, default: &Option<Expr>, pos: Pos) -> Result<(), Error> {
        if self.sets.contains_key(name) || self.params.contains_key(name) || self.var_arity.contains_key(name) {
            return err(pos, format!("«{name}» ya está definido"));
        }
        let default = match default {
            Some(e) => Some(self.num(e)?),
            None => None,
        };
        let mut values: HashMap<Key, f64> = HashMap::new();
        let data = self.data_for(name);
        match over {
            None => {
                let v = match (&data, value) {
                    (Some(d), _) => d.as_f64().ok_or_else(|| Error { pos, msg: format!("el dato {name} debe ser un número") })?,
                    (None, Some(ParamValue::Expr(e))) => self.num(e)?,
                    (None, Some(_)) => return err(pos, format!("{name} no tiene índices: su valor es un número")),
                    (None, None) => match default {
                        Some(d) => d,
                        None => return err(pos, format!("falta el valor de {name}")),
                    },
                };
                values.insert(vec![], v);
                self.params.insert(name.into(), ParamData { arity: 0, values, default });
                return Ok(());
            }
            Some(ix) => {

                // las combinaciones de índices, en orden
                let vars: Vec<String> = ix.binds.iter().flat_map(|b| b.vars.clone()).collect();
                let mut keys: Vec<(Key, Vec<Elem>)> = vec![];
                // los índices que se nombran con un conjunto ({I, J}) se recorren con nombres propios
                let ix2 = self.named(ix)?;
                let arity: usize = ix2.binds.iter().map(|b| b.vars.len()).sum();
                let vars2: Vec<String> = ix2.binds.iter().flat_map(|b| b.vars.clone()).collect();
                self.each(&ix2, &mut |c| {
                    let t: Vec<Elem> = vars2.iter().map(|v| c.lookup_bind(v).cloned().unwrap()).collect();
                    keys.push((key_of(&t), t));
                    Ok(())
                })?;
                let _ = vars;
                // dimensiones de cada conjunto (para las listas anidadas)
                let dims: Vec<Vec<Vec<Elem>>> = {
                    let mut d = vec![];
                    for b in &ix2.binds {
                        d.push(self.set_items(&b.set)?);
                    }
                    d
                };
                if let Some(d) = data {
                    fill_json(&d, &dims, &mut values, name, pos)?;
                } else {
                    match value {
                        None => {}
                        Some(ParamValue::Expr(e)) => {
                            // param d{i in I} = 2 * a[i];
                            let e = e.clone();
                            let vs: Vec<String> = vars2.clone();
                            let mut out = vec![];
                            self.each(&ix2, &mut |c| {
                                let t: Vec<Elem> = vs.iter().map(|v| c.lookup_bind(v).cloned().unwrap()).collect();
                                let x = c.num(&e)?;
                                out.push((key_of(&t), x));
                                Ok(())
                            })?;
                            values.extend(out);
                        }
                        Some(ParamValue::List(_)) => {
                            let v = value.clone().unwrap();
                            self.fill_list(&v, &dims, &mut values, &mut vec![], name, pos)?;
                        }
                        Some(ParamValue::Keyed(items)) => {
                            for (k, e) in items {
                                let mut kk = vec![];
                                for x in k {
                                    kk.push(self.elem(x)?.key());
                                }
                                if kk.len() != arity {
                                    return err(pos, format!("{name} lleva {arity} índices y la clave ({}) tiene {}", kk.join(","), kk.len()));
                                }
                                let x = self.num(e)?;
                                values.insert(kk, x);
                            }
                        }
                    }
                }
                // cada clave debe ser de los conjuntos
                let valid: HashSet<Key> = keys.iter().map(|k| k.0.clone()).collect();
                if let Some(bad) = values.keys().find(|k| !valid.contains(*k)) {
                    return err(pos, format!("{name}[{}] no corresponde a los conjuntos de {name}", bad.join(",")));
                }
                self.params.insert(name.into(), ParamData { arity, values, default });
            }
        }
        Ok(())
    }

    fn fill_list(&mut self, v: &ParamValue, dims: &[Vec<Vec<Elem>>], out: &mut HashMap<Key, f64>, prefix: &mut Key, name: &str, pos: Pos) -> Result<(), Error> {
        let level = prefix.len();
        let Some(items) = dims.get(level) else {
            return match v {
                ParamValue::Expr(e) => {
                    let x = self.num(e)?;
                    out.insert(prefix.clone(), x);
                    Ok(())
                }
                _ => err(pos, format!("{name}: la lista tiene más niveles que índices")),
            };
        };
        match v {
            ParamValue::List(list) => {
                if list.len() != items.len() {
                    return err(pos, format!("{name}: la lista tiene {} valores y el conjunto {} elementos", list.len(), items.len()));
                }
                for (x, it) in list.iter().zip(items) {
                    let mut p = prefix.clone();
                    p.extend(key_of(it));
                    self.fill_list(x, dims, out, &mut p, name, pos)?;
                }
                Ok(())
            }
            _ => err(pos, format!("{name}: falta un nivel de la lista (uno por cada conjunto)")),
        }
    }

    fn declare_var(&mut self, name: &str, over: &Option<Indexing>, attrs: &VarAttrs, pos: Pos) -> Result<(), Error> {
        if self.sets.contains_key(name) || self.params.contains_key(name) || self.var_arity.contains_key(name) {
            return err(pos, format!("«{name}» ya está definido"));
        }
        let kind = match attrs.kind.as_deref() {
            Some("binary") => Kind::Binary,
            Some("integer") => Kind::Integer,
            Some("semicontinuous") => Kind::SemiContinuous,
            Some("semiinteger") => Kind::SemiInteger,
            _ => Kind::Continuous,
        };
        let mut fam = Family { name: name.into(), sets: vec![], entries: vec![] };
        let mut set_names: Vec<String> = vec![];
        let mut make = |c: &mut Ctx<'a>, key: Vec<Elem>| -> Result<(), Error> {
            let lb = match &attrs.lb {
                Some(e) => c.num(e)?,
                None if attrs.free => f64::NEG_INFINITY,
                None => 0.0,
            };
            let ub = match &attrs.ub {
                Some(e) => c.num(e)?,
                None => f64::INFINITY,
            };
            let (lb, ub) = if kind == Kind::Binary { (lb.max(0.0), ub.min(1.0)) } else { (lb, ub) };
            if lb > ub + 1e-12 {
                return err(pos, format!("la variable {name} tiene la cota inferior ({}) mayor que la superior ({})", fmt_num(lb), fmt_num(ub)));
            }
            let k = key_of(&key);
            let cname = if k.is_empty() { name.to_string() } else { format!("{name}[{}]", k.join(",")) };
            if c.m.cols.len() >= c.cols_limit {
                return err(pos, format!("el modelo pasa de {} variables", c.cols_limit));
            }
            let j = c.m.add_col(cname, 0.0, lb, ub, kind);
            fam.entries.push((k, j));
            Ok(())
        };
        match over {
            None => make(self, vec![])?,
            Some(ix) => {
                let ix2 = self.named(ix)?;
                set_names = ix2.binds.iter().flat_map(|b| {
                    let n = Self::set_name(&b.set);
                    std::iter::repeat_n(n, b.vars.len())
                }).collect();
                let vars: Vec<String> = ix2.binds.iter().flat_map(|b| b.vars.clone()).collect();
                self.each(&ix2, &mut |c| {
                    let t: Vec<Elem> = vars.iter().map(|v| c.lookup_bind(v).cloned().unwrap()).collect();
                    make(c, t)
                })?;
                self.var_arity.insert(name.into(), vars.len());
            }
        }
        fam.sets = set_names;
        self.var_arity.entry(name.into()).or_insert(0);
        self.var_order.push(name.into());
        self.var_index.insert(name.into(), fam.entries.iter().map(|(k, j)| (k.clone(), *j)).collect());
        self.vars.insert(name.into(), fam);
        Ok(())
    }

    fn rel_rows(&mut self, rel: &Rel, row_name: &str) -> Result<Vec<(Lin, f64, f64)>, Error> {
        let Rel::Chain(es, ops) = rel;
        if ops.len() == 1 {
            let l = self.lin(&es[0], false)?.add(self.lin(&es[1], false)?, -1.0);
            if l.is_const() {
                let ok = match ops[0] {
                    "<=" => l.c <= 1e-9,
                    ">=" => l.c >= -1e-9,
                    _ => l.c.abs() <= 1e-9,
                };
                if !ok {
                    return err(pos_of(&es[0]), format!("la restricción {row_name} no tiene variables y no se cumple"));
                }
                return Ok(vec![]);
            }
            let (lo, hi) = match ops[0] {
                "<=" => (f64::NEG_INFINITY, 0.0),
                ">=" => (0.0, f64::INFINITY),
                _ => (0.0, 0.0),
            };
            return Ok(vec![(l, lo, hi)]);
        }
        // a <= expr <= b (también a >= expr >= b)
        let a = self.lin(&es[0], false)?;
        let mid = self.lin(&es[1], false)?;
        let b = self.lin(&es[2], false)?;
        if !a.is_const() || !b.is_const() {
            return err(pos_of(&es[0]), "en a <= expresión <= b, a y b deben ser números o datos");
        }
        let (lo, hi) = match (ops[0], ops[1]) {
            ("<=", "<=") => (a.c, b.c),
            (">=", ">=") => (b.c, a.c),
            _ => return err(pos_of(&es[0]), "una restricción doble va con el mismo sentido: a <= x <= b"),
        };
        Ok(vec![(mid, lo, hi)])
    }

    /// La restricción «b = v ⇒ relación» con big-M.
    fn indicator(&mut self, b: &Lin, val: f64, rows: Vec<(Lin, f64, f64)>, name: &str, pos: Pos) -> Result<(), Error> {
        // s = 1 cuando la condición se cumple
        let s = if val == 1.0 { b.clone() } else { Lin::konst(1.0).add(b.clone(), -1.0) };
        for (l, lo, hi) in rows {
            let (bl, bh) = self.finite(self.bounds(&l), "una restricción indicadora (si … entonces)", pos)?;
            if hi.is_finite() {
                // l ≤ hi + (bh − hi)(1 − s)
                let m = (bh - hi).max(0.0);
                let row = l.clone().add(s.clone(), m);
                self.add_named(name, &row, f64::NEG_INFINITY, hi + m);
            }
            if lo.is_finite() {
                // l ≥ lo − (lo − bl)(1 − s)
                let m = (lo - bl).max(0.0);
                let row = l.clone().add(s.clone(), -m);
                self.add_named(name, &row, lo - m, f64::INFINITY);
            }
        }
        Ok(())
    }

    fn add_named(&mut self, name: &str, l: &Lin, lo: f64, hi: f64) -> usize {
        let coefs = l.merged();
        let (lo, hi) = (lo - l.c, hi - l.c);
        let n = if self.m.rows.iter().any(|r| r.name == name) || name.is_empty() { format!("{}{}", if name.is_empty() { "R" } else { name }, self.m.rows.len() + 1) } else { name.to_string() };
        self.m.add_row(n, coefs, lo, hi)
    }

    fn constraint(&mut self, name: &str, over: &Option<Indexing>, body: &ConBody, pos: Pos) -> Result<(), Error> {
        let mut fam = Family { name: if name.is_empty() { format!("R{}", self.cons.len() + 1) } else { name.to_string() }, sets: vec![], entries: vec![] };
        let base = fam.name.clone();
        let mut set_names: Vec<String> = vec![];
        let mut one = |c: &mut Ctx<'a>, key: Vec<Elem>| -> Result<(), Error> {
            let k = key_of(&key);
            let rname = if k.is_empty() { base.clone() } else { format!("{base}[{}]", k.join(",")) };
            let first_row = c.m.rows.len();
            match body {
                ConBody::Rel(r) => {
                    for (l, lo, hi) in c.rel_rows(r, &rname)? {
                        c.add_named(&rname, &l, lo, hi);
                    }
                }
                ConBody::Indicator { bin, val, then } => {
                    let b = c.lin(bin, false)?;
                    let b = c.binary_arg(&b, pos)?;
                    let rows = c.rel_rows(then, &rname)?;
                    c.indicator(&b, *val, rows, &rname, pos)?;
                }
                ConBody::Either(rels) => {
                    let mut sum = Lin::default();
                    for r in rels {
                        let d = c.aux_col("_o", 0.0, 1.0, Kind::Binary);
                        let rows = c.rel_rows(r, &rname)?;
                        c.indicator(&Lin::var(d), 1.0, rows, &rname, pos)?;
                        sum = sum.add(Lin::var(d), 1.0);
                    }
                    c.add_named(&format!("{rname}_alguna"), &sum, 1.0, f64::INFINITY);
                    c.notes.push(format!("{rname}: disyunción de {} relaciones con big-M ({} binarias).", rels.len(), rels.len()));
                }
                ConBody::Logic { name: op, args } => {
                    if op == "implies" {
                        if args.len() != 2 {
                            return err(pos, "implies lleva dos binarias: implies(a, b)");
                        }
                        let a = c.lin(&args[0], false)?;
                        let a = c.binary_arg(&a, pos)?;
                        let b = c.lin(&args[1], false)?;
                        let b = c.binary_arg(&b, pos)?;
                        c.add_named(&rname, &a.add(b, -1.0), f64::NEG_INFINITY, 0.0);
                    } else {
                        if args.len() < 2 {
                            return err(pos, format!("{op} lleva un número y las binarias: {op}(2, y[1], y[2], y[3])"));
                        }
                        let k = c.num(&args[0])?;
                        let mut sum = Lin::default();
                        for a in &args[1..] {
                            let v = c.lin(a, false)?;
                            sum = sum.add(v, 1.0);
                        }
                        let (lo, hi) = match op.as_str() {
                            "atmost" => (f64::NEG_INFINITY, k),
                            "atleast" => (k, f64::INFINITY),
                            _ => (k, k),
                        };
                        c.add_named(&rname, &sum, lo, hi);
                    }
                }
            }
            if c.m.rows.len() > first_row {
                fam.entries.push((k, first_row));
            }
            Ok(())
        };
        match over {
            None => one(self, vec![])?,
            Some(ix) => {
                let ix = &self.named(ix)?;
                set_names = ix.binds.iter().flat_map(|b| {
                    let n = Self::set_name(&b.set);
                    std::iter::repeat_n(n, b.vars.len())
                }).collect();
                let vars: Vec<String> = ix.binds.iter().flat_map(|b| b.vars.clone()).collect();
                self.each(ix, &mut |c| {
                    let t: Vec<Elem> = vars.iter().map(|v| c.lookup_bind(v).cloned().unwrap()).collect();
                    one(c, t)
                })?;
            }
        }
        fam.sets = set_names;
        self.cons.push(fam);
        Ok(())
    }

    fn objective(&mut self, maximize: bool, name: &str, e: &Expr, pos: Pos) -> Result<(), Error> {
        let l = self.lin(e, true)?;
        self.objectives += 1;
        if self.objectives == 1 {
            self.obj1 = name.to_string();
            self.m.maximize = maximize;
            for (j, a) in l.merged() {
                self.m.cols[j].cost += a;
            }
            self.m.offset = l.c;
            // ½ xᵀQx: Q_ij = a (i ≠ j, triángulo inferior), Q_ii = 2a
            let mut q: HashMap<(usize, usize), f64> = HashMap::new();
            for (i, j, a) in &l.quad {
                let (r, c) = if i >= j { (*i, *j) } else { (*j, *i) };
                *q.entry((r, c)).or_default() += if r == c { 2.0 * a } else { *a };
            }
            let mut qv: Vec<(usize, usize, f64)> = q.into_iter().filter(|x| x.1 != 0.0).map(|((r, c), a)| (r, c, a)).collect();
            qv.sort_by_key(|x| (x.1, x.0));
            if !qv.is_empty() {
                self.notes.push(format!("Objetivo cuadrático con {} términos (convexo para minimizar, cóncavo para maximizar).", qv.len()));
            }
            self.m.q = qv;
        } else {
            if !l.quad.is_empty() {
                return err(pos, "los objetivos adicionales deben ser lineales");
            }
            // varios objetivos: por prioridad, en el orden en que se escriben
            if self.m.objectives.is_empty() {
                // el primero también pasa a la lista
                let coefs = self.m.cols.iter().filter(|c| c.cost != 0.0).map(|c| (c.name.clone(), c.cost)).collect();
                self.m.objectives.push(solver::Objective { name: if self.obj1.is_empty() { "1".into() } else { self.obj1.clone() }, coefs, maximize: self.m.maximize, priority: 1000, weight: 1.0, rel_tol: 0.0, abs_tol: 0.0 });
            }
            let coefs = l.merged().into_iter().map(|(j, a)| (self.m.cols[j].name.clone(), a)).collect();
            let pr = 1000 - self.objectives as i32;
            self.m.objectives.push(solver::Objective { name: if name.is_empty() { format!("{}", self.objectives) } else { name.into() }, coefs, maximize, priority: pr, weight: 1.0, rel_tol: 0.0, abs_tol: 0.0 });
            self.notes.push("Varios objetivos: se optimizan por prioridad, en el orden en que se escribieron.".into());
        }
        Ok(())
    }
}

fn json_elem(v: &Value) -> Elem {
    match v {
        Value::Number(n) => Elem::N(n.as_f64().unwrap_or(0.0)),
        Value::String(s) => Elem::S(s.clone()),
        other => Elem::S(other.to_string()),
    }
}

/// Datos de un parámetro desde JSON: número, lista (anidada) en el orden de los conjuntos u objeto {clave: valor}.
fn fill_json(d: &Value, dims: &[Vec<Vec<Elem>>], out: &mut HashMap<Key, f64>, name: &str, pos: Pos) -> Result<(), Error> {
    fn go(d: &Value, dims: &[Vec<Vec<Elem>>], level: usize, prefix: &mut Key, out: &mut HashMap<Key, f64>, name: &str, pos: Pos) -> Result<(), Error> {
        if level == dims.len() {
            let x = d.as_f64().or_else(|| d.as_str().and_then(|s| s.replace(',', ".").parse().ok()));
            return match x {
                Some(x) => {
                    out.insert(prefix.clone(), x);
                    Ok(())
                }
                None if d.is_null() => Ok(()),
                None => err(pos, format!("{name}: el dato {} no es un número", d)),
            };
        }
        match d {
            Value::Array(a) => {
                if a.len() != dims[level].len() {
                    return err(pos, format!("{name}: hay {} datos y el conjunto tiene {}", a.len(), dims[level].len()));
                }
                for (x, it) in a.iter().zip(&dims[level]) {
                    let n0 = prefix.len();
                    prefix.extend(key_of(it));
                    go(x, dims, level + 1, prefix, out, name, pos)?;
                    prefix.truncate(n0);
                }
                Ok(())
            }
            Value::Object(o) => {
                for (k, x) in o {
                    let n0 = prefix.len();
                    // claves «A,B» para varios índices de una vez
                    let parts: Vec<String> = k.split(',').map(|s| s.trim().to_string()).collect();
                    let skip = parts.len();
                    prefix.extend(parts);
                    go(x, dims, level + skip, prefix, out, name, pos)?;
                    prefix.truncate(n0);
                }
                Ok(())
            }
            _ => err(pos, format!("{name}: se esperaba una lista o un objeto")),
        }
    }
    go(d, dims, 0, &mut vec![], out, name, pos)
}

/// {I, J} (conjuntos sin variable de índice) → {·1 in I, ·2 in J}; el número de
/// índices de cada uno sale de sus elementos.
impl<'a> Ctx<'a> {
    fn named(&mut self, ix: &Indexing) -> Result<Indexing, Error> {
        let mut out = ix.clone();
        let mut k = 0;
        for b in &mut out.binds {
            if b.vars.is_empty() {
                let arity = self.set_items(&b.set)?.first().map(|t| t.len()).unwrap_or(1);
                b.vars = (0..arity).map(|_| {
                    k += 1;
                    format!("\u{1}{k}")
                }).collect();
            }
        }
        Ok(out)
    }
}

fn pos_of(e: &Expr) -> Pos {
    match e {
        Expr::Ref { pos, .. } | Expr::Bin { pos, .. } | Expr::Sum { pos, .. } | Expr::Call { pos, .. } => *pos,
        Expr::Neg(a) => pos_of(a),
        _ => Pos::default(),
    }
}

/// Arma el modelo.
pub fn compile(stmts: &[Stmt], name: &str, data: Option<&Value>) -> Result<Compiled, Error> {
    let mut c = Ctx {
        m: Model { name: name.into(), ..Default::default() },
        sets: HashMap::new(),
        set_order: vec![],
        params: HashMap::new(),
        vars: HashMap::new(),
        var_index: HashMap::new(),
        var_order: vec![],
        var_arity: HashMap::new(),
        cons: vec![],
        binds: vec![],
        notes: vec![],
        bigm: None,
        aux: 0,
        objectives: 0,
        obj1: String::new(),
        data,
        cols_limit: COLS_LIMIT,
        auto_vars: !stmts.iter().any(|s| matches!(s, Stmt::Var { .. })),
    };
    for s in stmts {
        match s {
            Stmt::Option { name, value, pos } => match name.as_str() {
                "bigm" | "big_m" => c.bigm = Some(value.abs()),
                other => return err(*pos, format!("opción desconocida: {other} (la que hay es bigM)")),
            },
            Stmt::Set { name, value, pos } => c.declare_set(name, value, *pos)?,
            Stmt::Param { name, over, value, default, pos } => c.declare_param(name, over, value, default, *pos)?,
            Stmt::Var { name, over, attrs, pos } => c.declare_var(name, over, attrs, *pos)?,
            Stmt::Objective { maximize, name, expr, pos } => c.objective(*maximize, name, expr, *pos)?,
            Stmt::Constraint { name, over, body, pos } => c.constraint(name, over, body, *pos)?,
        }
    }
    if c.var_order.is_empty() {
        return err(Pos::default(), "el modelo no tiene variables (decláralas con var x{…};)");
    }
    if c.auto_vars && !c.var_order.is_empty() {
        c.notes.push(format!("Sin declaraciones var: {} se tomaron como variables continuas ≥ 0.", c.var_order.join(", ")));
    }
    if c.objectives == 0 {
        c.notes.push("El modelo no tiene función objetivo: se busca una solución factible.".into());
    }
    let sets = c.set_order.iter().map(|n| (n.clone(), c.sets[n].items.len())).collect();
    let vars = c.var_order.iter().map(|n| c.vars[n].clone()).collect();
    Ok(Compiled { model: c.m, vars, cons: c.cons, notes: c.notes, sets, aux: c.aux })
}
