//! Archivos de LP-ILP (.LPP), en sus dos variantes:
//!
//! * `MatrixFormat`: la hoja de cálculo con una columna por variable.
//! * `NormalModel`: el modelo escrito como expresiones (3X1+5X2, X1+X2<=4).
//!
//! Estructura deducida de las rutinas de carga y guardado del LP-ILP.EXE original.

use crate::{decode_ansi, encode_ansi, fields, FormatError};
use iaqsb_lp::model::{fmt_num, parse_number};
use iaqsb_lp::{Bound, Constraint, Direction, LpModel, Objective, VarType, Variable};

/// Formato de entrada de datos.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LppLayout {
    Matrix,
    Normal,
}

const CRLF: &str = "\r\n";

fn bad(at: impl Into<String>, msg: impl Into<String>) -> FormatError {
    FormatError::Bad {
        at: at.into(),
        msg: msg.into(),
    }
}

fn cell(v: &[String], i: usize) -> &str {
    v.get(i).map(|s| s.as_str()).unwrap_or("")
}

fn num(s: &str, at: &str) -> Result<f64, FormatError> {
    if s.trim().is_empty() {
        return Ok(0.0);
    }
    parse_number(s).ok_or_else(|| bad(at, format!("{s:?} no es un número")))
}

fn bound(s: &str, at: &str) -> Result<Bound, FormatError> {
    Bound::parse(s).ok_or_else(|| bad(at, format!("{s:?} no es una cota válida")))
}

fn objective_of(s: &str) -> Objective {
    if s.trim().to_ascii_uppercase().starts_with("MIN") {
        Objective::Min
    } else {
        Objective::Max
    }
}

fn direction_of(s: &str, at: &str) -> Result<Direction, FormatError> {
    match s.trim() {
        "<=" | "=<" | "<" => Ok(Direction::Le),
        ">=" | "=>" | ">" => Ok(Direction::Ge),
        "=" => Ok(Direction::Eq),
        o => Err(bad(at, format!("sentido {o:?} no reconocido"))),
    }
}

/// Lee un archivo .LPP (bytes tal cual del disco).
pub fn read(bytes: &[u8]) -> Result<(LpModel, LppLayout), FormatError> {
    let text = decode_ansi(bytes);
    let mut lines = text.split('\n').map(|l| l.trim_end_matches('\r'));
    let head = fields(
        lines
            .next()
            .ok_or_else(|| FormatError::Truncated("la cabecera".into()))?,
    );
    if !cell(&head, 0).to_ascii_uppercase().starts_with("LP") {
        return Err(FormatError::WrongModule("LP-ILP"));
    }
    let layout = if cell(&head, 1).to_ascii_uppercase().starts_with('M') {
        LppLayout::Matrix
    } else {
        LppLayout::Normal
    };
    let name = cell(&head, 2).to_string();
    let n = num(cell(&head, 3), "cabecera")? as usize;
    let m = num(cell(&head, 4), "cabecera")? as usize;
    if n == 0 {
        return Err(bad("cabecera", "el número de variables es 0"));
    }
    let mut next = |what: &str| -> Result<Vec<String>, FormatError> {
        lines
            .next()
            .map(fields)
            .ok_or_else(|| FormatError::Truncated(what.into()))
    };
    let model = match layout {
        LppLayout::Matrix => read_matrix(&mut next, name, n, m)?,
        LppLayout::Normal => read_normal(&mut next, name, n, m)?,
    };
    Ok((model, layout))
}

fn read_matrix(
    next: &mut dyn FnMut(&str) -> Result<Vec<String>, FormatError>,
    name: String,
    n: usize,
    m: usize,
) -> Result<LpModel, FormatError> {
    let names = next("los nombres de variables")?;
    let obj = next("la función objetivo")?;
    let mut variables = Vec::with_capacity(n);
    for j in 0..n {
        let mut vname = cell(&names, j + 1).trim().to_string();
        if vname.is_empty() {
            vname = format!("X{}", j + 1);
        }
        variables.push(Variable {
            name: vname,
            cost: num(cell(&obj, j + 1), "función objetivo")?,
            lower: Bound(0.0),
            upper: Bound::INF,
            var_type: VarType::Continuous,
            priority: None,
        });
    }
    let mut constraints = Vec::with_capacity(m);
    for i in 0..m {
        let row = next(&format!("la restricción {}", i + 1))?;
        let at = format!("restricción {}", i + 1);
        let mut cname = cell(&row, 0).trim().to_string();
        if cname.is_empty() {
            cname = format!("C{}", i + 1);
        }
        let coefs = (0..n)
            .map(|j| num(cell(&row, j + 1), &at))
            .collect::<Result<Vec<_>, _>>()?;
        constraints.push(Constraint {
            terms: None,
            name: cname,
            coefs,
            direction: direction_of(cell(&row, n + 1), &at)?,
            rhs: num(cell(&row, n + 2), &at)?,
        });
    }
    let lb = next("las cotas inferiores")?;
    let ub = next("las cotas superiores")?;
    let ty = next("los tipos de variable")?;
    for (j, v) in variables.iter_mut().enumerate() {
        v.var_type = match cell(&ty, j + 1)
            .trim()
            .chars()
            .next()
            .map(|c| c.to_ascii_uppercase())
        {
            Some('I') => VarType::Integer,
            Some('B') => VarType::Binary,
            Some('U') => VarType::Unrestricted,
            _ => VarType::Continuous,
        };
        let l = cell(&lb, j + 1);
        let u = cell(&ub, j + 1);
        v.lower = if l.trim().is_empty() {
            Bound(0.0)
        } else {
            bound(l, "cota inferior")?
        };
        v.upper = if u.trim().is_empty() {
            Bound::INF
        } else {
            bound(u, "cota superior")?
        };
    }
    Ok(LpModel {
        objectives: vec![],
        start: None,
        name,
        objective: objective_of(cell(&obj, 0)),
        variables,
        constraints,
    })
}

/// Separa una expresión como "3X1+5X2-X3" en (coeficiente, nombre).
fn parse_terms(expr: &str, at: &str) -> Result<Vec<(f64, String)>, FormatError> {
    let s: String = expr.chars().filter(|c| !c.is_whitespace()).collect();
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;
    let mut out = Vec::new();
    while i < chars.len() {
        // signo y número
        let start = i;
        if chars[i] == '+' || chars[i] == '-' {
            i += 1;
        }
        while i < chars.len() && !chars[i].is_ascii_alphabetic() && chars[i] != '_' {
            i += 1;
        }
        let coef_txt: String = chars[start..i].iter().collect();
        let coef = match coef_txt.as_str() {
            "" | "+" => 1.0,
            "-" => -1.0,
            t => {
                let t = t.trim_end_matches('*');
                parse_number(t).ok_or_else(|| bad(at, format!("coeficiente {t:?} no válido")))?
            }
        };
        // nombre hasta el siguiente + o -
        let ns = i;
        while i < chars.len() && chars[i] != '+' && chars[i] != '-' {
            i += 1;
        }
        let name: String = chars[ns..i].iter().collect();
        if name.is_empty() {
            return Err(bad(at, "falta el nombre de una variable"));
        }
        out.push((coef, name));
    }
    Ok(out)
}

fn split_relation(s: &str) -> Option<(&str, Direction, &str)> {
    for (op, d) in [
        ("<=", Direction::Le),
        ("=<", Direction::Le),
        (">=", Direction::Ge),
        ("=>", Direction::Ge),
        ("<", Direction::Le),
        (">", Direction::Ge),
        ("=", Direction::Eq),
    ] {
        if let Some(p) = s.find(op) {
            return Some((&s[..p], d, &s[p + op.len()..]));
        }
    }
    None
}

fn read_normal(
    next: &mut dyn FnMut(&str) -> Result<Vec<String>, FormatError>,
    name: String,
    n: usize,
    m: usize,
) -> Result<LpModel, FormatError> {
    next("la fila de títulos")?;
    let obj = next("la función objetivo")?;
    let objective = objective_of(cell(&obj, 0));
    let mut rows = Vec::with_capacity(m);
    for i in 0..m {
        rows.push(next(&format!("la restricción {}", i + 1))?);
    }
    let ints = next("la lista de enteras")?;
    let bins = next("la lista de binarias")?;
    let unres = next("la lista de libres")?;
    let mut bound_rows = Vec::with_capacity(n);
    for j in 0..n {
        bound_rows.push(next(&format!("las cotas de la variable {}", j + 1))?);
    }
    // los nombres de variables son los de las filas de cotas
    let mut variables: Vec<Variable> = bound_rows
        .iter()
        .enumerate()
        .map(|(j, r)| {
            let nm = cell(r, 0).trim();
            Variable {
                name: if nm.is_empty() {
                    format!("X{}", j + 1)
                } else {
                    nm.to_string()
                },
                cost: 0.0,
                lower: Bound(0.0),
                upper: Bound::INF,
                var_type: VarType::Continuous,
                priority: None,
            }
        })
        .collect();
    let index = |vars: &Vec<Variable>, nm: &str, at: &str| -> Result<usize, FormatError> {
        vars.iter()
            .position(|v| v.name.eq_ignore_ascii_case(nm))
            .ok_or_else(|| bad(at, format!("variable desconocida {nm:?}")))
    };
    for (c, nm) in parse_terms(cell(&obj, 1), "función objetivo")? {
        let j = index(&variables, &nm, "función objetivo")?;
        variables[j].cost += c;
    }
    let mut constraints = Vec::with_capacity(m);
    for (i, r) in rows.iter().enumerate() {
        let at = format!("restricción {}", i + 1);
        let txt: String = cell(r, 1).chars().filter(|c| !c.is_whitespace()).collect();
        let (lhs, dir, rhs) = split_relation(&txt).ok_or_else(|| bad(&at, "falta <=, >= o ="))?;
        let mut coefs = vec![0.0; n];
        for (c, nm) in parse_terms(lhs, &at)? {
            coefs[index(&variables, &nm, &at)?] += c;
        }
        let mut cname = cell(r, 0).trim().to_string();
        if cname.is_empty() {
            cname = format!("C{}", i + 1);
        }
        constraints.push(Constraint {
            terms: None,
            name: cname,
            coefs,
            direction: dir,
            rhs: num(rhs, &at)?,
        });
    }
    for (list, t) in [
        (&ints, VarType::Integer),
        (&bins, VarType::Binary),
        (&unres, VarType::Unrestricted),
    ] {
        for nm in cell(list, 1)
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            let j = index(&variables, nm, "lista de tipos")?;
            variables[j].var_type = t;
        }
    }
    for (j, r) in bound_rows.iter().enumerate() {
        if matches!(
            variables[j].var_type,
            VarType::Binary | VarType::Unrestricted
        ) {
            continue;
        }
        let at = format!("cotas de {}", variables[j].name);
        for part in cell(r, 1)
            .split(',')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
        {
            let compact: String = part.chars().filter(|c| !c.is_whitespace()).collect();
            let (_, d, val) = split_relation(&compact)
                .ok_or_else(|| bad(&at, format!("{part:?} no es una cota")))?;
            let b = bound(val, &at)?;
            match d {
                Direction::Le => variables[j].upper = b,
                Direction::Ge => variables[j].lower = b,
                Direction::Eq => {
                    variables[j].lower = b;
                    variables[j].upper = b;
                }
            }
        }
    }
    Ok(LpModel {
        objectives: vec![],
        start: None,
        name,
        objective,
        variables,
        constraints,
    })
}

fn type_name(t: VarType) -> &'static str {
    match t {
        // WinQSB no tiene semicontinuas ni semienteras: se guardan como su tipo base
        VarType::Continuous | VarType::SemiContinuous => "Continuous",
        VarType::Integer | VarType::SemiInteger => "Integer",
        VarType::Binary => "Binary",
        VarType::Unrestricted => "Unrestricted",
    }
}

fn objective_word(o: Objective) -> &'static str {
    match o {
        Objective::Max => "Maximize",
        Objective::Min => "Minimize",
    }
}

/// Escribe un modelo como .LPP (bytes ANSI listos para guardar).
pub fn write(model: &LpModel, layout: LppLayout) -> Vec<u8> {
    let n = model.variables.len();
    let m = model.constraints.len();
    let mut out = String::new();
    let fmt = match layout {
        LppLayout::Matrix => "MatrixFormat",
        LppLayout::Normal => "NormalModel",
    };
    out += &format!("LP\t{fmt}\t{}\t{n}\t{m}{CRLF}", model.name);
    let mut row = |cells: Vec<String>| {
        out += &cells.join("\t");
        out += CRLF;
    };
    match layout {
        LppLayout::Matrix => {
            let blank2 = || vec![String::new(), String::new()];
            let mut r = vec!["Variable -->".to_string()];
            r.extend(model.variables.iter().map(|v| v.name.clone()));
            r.push("Direction".into());
            r.push("R. H. S.".into());
            row(r);
            let mut r = vec![objective_word(model.objective).to_string()];
            r.extend(model.variables.iter().map(|v| fmt_num(v.cost)));
            r.extend(blank2());
            row(r);
            for c in &model.constraints {
                let mut r = vec![c.name.clone()];
                r.extend(c.coefs.iter().map(|x| fmt_num(*x)));
                r.push(c.direction.symbol().into());
                r.push(fmt_num(c.rhs));
                row(r);
            }
            let mut lb = vec!["LowerBound".to_string()];
            let mut ub = vec!["UpperBound".to_string()];
            let mut ty = vec!["VariableType".to_string()];
            for j in 0..n {
                let (l, u) = model.effective_bounds(j);
                lb.push(Bound(l).to_winqsb());
                ub.push(Bound(u).to_winqsb());
                ty.push(type_name(model.variables[j].var_type).into());
            }
            for mut r in [lb, ub, ty] {
                r.extend(blank2());
                row(r);
            }
        }
        LppLayout::Normal => {
            let expr = |coefs: &mut dyn Iterator<Item = (f64, &str)>| -> String {
                let mut s = String::new();
                for (c, nm) in coefs {
                    if c == 0.0 {
                        continue;
                    }
                    let sign = if c < 0.0 {
                        "-"
                    } else if s.is_empty() {
                        ""
                    } else {
                        "+"
                    };
                    let mag = c.abs();
                    let coef = if mag == 1.0 {
                        String::new()
                    } else {
                        fmt_num(mag)
                    };
                    s += &format!("{sign}{coef}{nm}");
                }
                if s.is_empty() {
                    s = format!("0{}", model.variables[0].name);
                }
                s
            };
            row(vec![
                "OBJ/Constraint/VariableType/Bound".into(),
                String::new(),
            ]);
            row(vec![
                objective_word(model.objective).into(),
                expr(&mut model.variables.iter().map(|v| (v.cost, v.name.as_str()))),
            ]);
            for c in &model.constraints {
                let lhs = expr(
                    &mut c
                        .coefs
                        .iter()
                        .zip(&model.variables)
                        .map(|(a, v)| (*a, v.name.as_str())),
                );
                row(vec![
                    c.name.clone(),
                    format!("{lhs}{}{}", c.direction.symbol(), fmt_num(c.rhs)),
                ]);
            }
            for (label, t) in [
                ("Integer:", VarType::Integer),
                ("Binary:", VarType::Binary),
                ("Unrestricted:", VarType::Unrestricted),
            ] {
                let names: Vec<&str> = model
                    .variables
                    .iter()
                    .filter(|v| v.var_type == t || (t == VarType::Integer && v.var_type == VarType::SemiInteger))
                    .map(|v| v.name.as_str())
                    .collect();
                row(vec![label.into(), names.join(",")]);
            }
            for j in 0..n {
                let (l, u) = model.effective_bounds(j);
                row(vec![
                    model.variables[j].name.clone(),
                    format!(">={}, <={}", Bound(l).to_winqsb(), Bound(u).to_winqsb()),
                ]);
            }
        }
    }
    encode_ansi(&out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> LpModel {
        LpModel {
            objectives: vec![],
            start: None,
            name: "Prueba".into(),
            objective: Objective::Max,
            variables: vec![
                Variable {
                    name: "X1".into(),
                    cost: 3.0,
                    lower: Bound(0.0),
                    upper: Bound::INF,
                    var_type: VarType::Continuous,
                    priority: None,
                },
                Variable {
                    name: "X2".into(),
                    cost: 5.0,
                    lower: Bound(0.0),
                    upper: Bound(10.0),
                    var_type: VarType::Integer,
                    priority: None,
                },
                Variable {
                    name: "Y".into(),
                    cost: -1.0,
                    lower: Bound::NEG_INF,
                    upper: Bound::INF,
                    var_type: VarType::Unrestricted,
                    priority: None,
                },
            ],
            constraints: vec![
                Constraint {
                    terms: None,
                    name: "C1".into(),
                    coefs: vec![1.0, 0.0, 1.0],
                    direction: Direction::Le,
                    rhs: 4.0,
                },
                Constraint {
                    terms: None,
                    name: "C2".into(),
                    coefs: vec![0.0, 2.0, -1.0],
                    direction: Direction::Ge,
                    rhs: 1.5,
                },
            ],
        }
    }

    fn same(a: &LpModel, b: &LpModel) {
        assert_eq!(a.name, b.name);
        assert_eq!(a.objective, b.objective);
        assert_eq!(a.variables.len(), b.variables.len());
        for (x, y) in a.variables.iter().zip(&b.variables) {
            assert_eq!(x.name, y.name);
            assert_eq!(x.cost, y.cost);
            assert_eq!(x.var_type, y.var_type);
            assert_eq!(x.upper.0, y.upper.0, "{}", x.name);
        }
        for (x, y) in a.constraints.iter().zip(&b.constraints) {
            assert_eq!(x.coefs, y.coefs);
            assert_eq!(x.direction, y.direction);
            assert_eq!(x.rhs, y.rhs);
        }
    }

    #[test]
    fn ida_y_vuelta_matriz() {
        let m = sample();
        let bytes = write(&m, LppLayout::Matrix);
        let (r, l) = read(&bytes).unwrap();
        assert_eq!(l, LppLayout::Matrix);
        same(&m, &r);
    }

    #[test]
    fn ida_y_vuelta_normal() {
        let m = sample();
        let bytes = write(&m, LppLayout::Normal);
        let text = String::from_utf8_lossy(&bytes).to_string();
        assert!(text.contains("3X1+5X2-Y"), "{text}");
        let (r, l) = read(&bytes).unwrap();
        assert_eq!(l, LppLayout::Normal);
        same(&m, &r);
    }

    #[test]
    fn modelo_normal_escrito_a_mano() {
        let txt = "LP\tNormalModel\tDieta\t2\t2\r\nOBJ/Constraint/VariableType/Bound\t\r\nMinimize\t2.15Pickle + 3.2Burger\r\nC1\tPickle+Burger>=4\r\nC2\tpickle + 3burger => 6\r\nInteger:\t\r\nBinary:\t\r\nUnrestricted:\t\r\nPickle\t>=0, <=M\r\nBurger\t>=1, <=10\r\n";
        let (m, _) = read(txt.as_bytes()).unwrap();
        assert_eq!(m.objective, Objective::Min);
        assert_eq!(m.variables[0].cost, 2.15);
        assert_eq!(m.constraints[1].coefs, vec![1.0, 3.0]);
        assert_eq!(m.constraints[1].direction, Direction::Ge);
        assert_eq!(m.variables[1].lower.0, 1.0);
        assert_eq!(m.variables[1].upper.0, 10.0);
    }
}

/// Escribe un modelo del motor (de cualquier tamaño) en el formato de WinQSB,
/// como «modelo normal» (algebraico): solo los coeficientes distintos de cero,
/// así que sirve también para modelos grandes y dispersos. Las filas con dos
/// lados se parten en dos, las libres se omiten y las variables semicontinuas o
/// semienteras quedan continuas o enteras (WinQSB no las tiene).
pub fn write_engine(m: &iaqsb_core::solver::Model) -> Vec<u8> {
    use iaqsb_core::solver::Kind;
    // WinQSB lee nombres de letras, cifras y «_»: x[P1,C1] → x_P1_C1 (sin repetir)
    let mut used = std::collections::HashSet::new();
    let names: Vec<String> = m
        .cols
        .iter()
        .map(|c| {
            let mut n: String = c.name.chars().map(|ch| if ch.is_alphanumeric() || ch == '_' { ch } else { '_' }).collect::<String>().trim_end_matches('_').to_string();
            while n.contains("__") {
                n = n.replace("__", "_");
            }
            if n.is_empty() || n.starts_with(|ch: char| ch.is_ascii_digit()) {
                n = format!("X{n}");
            }
            let mut k = 2;
            let base = n.clone();
            while !used.insert(n.clone()) {
                n = format!("{base}_{k}");
                k += 1;
            }
            n
        })
        .collect();
    let expr = |coefs: &mut dyn Iterator<Item = (usize, f64)>| -> String {
        let mut s = String::new();
        for (j, c) in coefs {
            if c == 0.0 {
                continue;
            }
            let sign = if c < 0.0 { "-" } else if s.is_empty() { "" } else { "+" };
            let mag = c.abs();
            let coef = if mag == 1.0 { String::new() } else { fmt_num(mag) };
            s += &format!("{sign}{coef}{}", names[j]);
        }
        if s.is_empty() {
            s = format!("0{}", names.first().cloned().unwrap_or_else(|| "X1".into()));
        }
        s
    };
    let mut rows: Vec<(String, String)> = vec![];
    for r in &m.rows {
        let lhs = expr(&mut r.coefs.iter().copied());
        if r.lo == r.hi {
            rows.push((r.name.clone(), format!("{lhs}={}", fmt_num(r.lo))));
        } else {
            if r.lo.is_finite() {
                rows.push((if r.hi.is_finite() { format!("{}_min", r.name) } else { r.name.clone() }, format!("{lhs}>={}", fmt_num(r.lo))));
            }
            if r.hi.is_finite() {
                rows.push((if r.lo.is_finite() { format!("{}_max", r.name) } else { r.name.clone() }, format!("{lhs}<={}", fmt_num(r.hi))));
            }
        }
    }
    let mut out = String::new();
    out += &format!("LP\tNormalModel\t{}\t{}\t{}{CRLF}", m.name, m.cols.len(), rows.len());
    out += &format!("OBJ/Constraint/VariableType/Bound\t{CRLF}");
    out += &format!("{}\t{}{CRLF}", if m.maximize { "Maximize" } else { "Minimize" }, expr(&mut m.cols.iter().enumerate().map(|(j, c)| (j, c.cost))));
    for (name, text) in rows {
        let clean: String = name.chars().map(|ch| if ch.is_alphanumeric() || ch == '_' || ch == ' ' { ch } else { '_' }).collect::<String>().trim_end_matches('_').to_string();
        out += &format!("{clean}\t{text}{CRLF}");
    }
    let list = |f: &dyn Fn(Kind, f64, f64) -> bool| -> String {
        m.cols.iter().zip(&names).filter(|(c, _)| f(c.kind, c.lb, c.ub)).map(|(_, n)| n.clone()).collect::<Vec<_>>().join(",")
    };
    out += &format!("Integer:\t{}{CRLF}", list(&|k, _, _| matches!(k, Kind::Integer | Kind::SemiInteger)));
    out += &format!("Binary:\t{}{CRLF}", list(&|k, _, _| k == Kind::Binary));
    out += &format!("Unrestricted:\t{}{CRLF}", list(&|k, l, u| k == Kind::Continuous && l == f64::NEG_INFINITY && u == f64::INFINITY));
    for (c, n) in m.cols.iter().zip(&names) {
        let (l, u) = if c.kind == Kind::Binary { (c.lb.max(0.0), c.ub.min(1.0)) } else { (c.lb, c.ub) };
        out += &format!("{n}\t>={}, <={}{CRLF}", Bound(l).to_winqsb(), Bound(u).to_winqsb());
    }
    encode_ansi(&out)
}
