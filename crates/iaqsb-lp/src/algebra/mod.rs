//! Lenguaje de modelado algebraico de IAQSB (al estilo de AMPL y LINGO), con
//! palabras clave en español, inglés y portugués:
//!
//! ```text
//! set Plantas = {Cali, Medellin, Bogota};
//! set Clientes = 1..5;
//! param oferta{Plantas} = [300, 500, 400];
//! param demanda{Clientes} = [120, 200, 180, 260, 140];
//! param c{Plantas, Clientes} = [[4, 6, 9, 5, 7], [5, 3, 7, 6, 4], [8, 5, 4, 7, 6]];
//! var x{Plantas, Clientes} >= 0;
//! minimizar costo: suma{i en Plantas, j en Clientes} c[i,j] * x[i,j];
//! sujeto a
//! oferta_i{i en Plantas}: suma{j en Clientes} x[i,j] <= oferta[i];
//! demanda_j{j en Clientes}: suma{i en Plantas} x[i,j] >= demanda[j];
//! ```
//!
//! Restricciones generales (se reformulan solas con big-M a partir de las cotas):
//! `abs(e)`, `max(e1, e2, …)`, `min(…)`, `pwl(x; (x0, f0), (x1, f1), …)`,
//! `and(b1, b2)`, `or(…)`, `not(b)`, indicadoras `si y[i] = 1 entonces …` (o `y[i] = 1 -> …`),
//! disyunciones `rel1 o rel2`, `atmost(k, …)`, `atleast(k, …)`, `exactly(k, …)`, `implies(a, b)`.

pub mod compile;
pub mod parse;

pub use compile::{Compiled, Family};
pub use parse::{Error, Pos};

use crate::model::{Direction, LpModel, Objective as Sense, VarType};
use serde_json::{json, Value};

/// Lee y arma un modelo escrito en el lenguaje. `data`: datos que reemplazan
/// los del texto ({"c": [[…]], "Plantas": ["A", "B"]}).
pub fn compile_text(src: &str, name: &str, data: Option<&Value>) -> Result<Compiled, Error> {
    let stmts = parse::parse(src)?;
    compile::compile(&stmts, name, data)
}

/// Revisión del texto para el editor: errores con su posición y el tamaño del modelo.
pub fn check(src: &str, data: Option<&Value>) -> Value {
    match compile_text(src, "", data) {
        Ok(c) => json!({
            "ok": true,
            "errors": [],
            "stats": stats(&c),
        }),
        Err(e) => json!({
            "ok": false,
            "errors": [{"line": e.pos.line, "col": e.pos.col, "message": e.msg}],
        }),
    }
}

/// Tamaño y forma del modelo armado.
pub fn stats(c: &Compiled) -> Value {
    let ints = c.model.cols.iter().filter(|x| x.kind.is_integer()).count();
    json!({
        "variables": c.model.cols.len(),
        "integer": ints,
        "constraints": c.model.rows.len(),
        "nonzeros": c.model.nonzeros(),
        "aux": c.aux,
        "sets": c.sets.iter().map(|(n, k)| json!({"name": n, "size": k})).collect::<Vec<_>>(),
        "families": c.vars.iter().map(|f| json!({"name": f.name, "indices": f.sets, "count": f.entries.len()})).collect::<Vec<_>>(),
        "reformulations": c.notes,
        "quadratic": !c.model.q.is_empty(),
        "objectives": c.model.objectives.len().max(1),
    })
}

fn r6(x: f64) -> f64 {
    let r = (x * 1e6).round() / 1e6;
    if r == 0.0 {
        0.0
    } else {
        r
    }
}

/// Tablas de la solución por índices: cada familia de variables (x[i,j] como
/// tabla de doble entrada; con 1 o 3+ índices, en lista) y cada familia de
/// restricciones (con su precio sombra). `values`, `reduced` y `duals` en el
/// orden de las columnas y filas del modelo.
pub fn tables(c: &Compiled, values: &[f64], reduced: Option<&[f64]>, duals: Option<&[f64]>, activity: Option<&[f64]>) -> Vec<Value> {
    let mut out = vec![];
    for f in &c.vars {
        if f.entries.is_empty() {
            continue;
        }
        let arity = f.entries[0].0.len();
        let title = if arity == 0 { format!("Variable {}", f.name) } else { format!("Variable {}[{}]", f.name, f.sets.iter().map(|s| if s.is_empty() { "·".to_string() } else { s.clone() }).collect::<Vec<_>>().join(", ")) };
        if arity == 2 && f.entries.len() > 1 {
            // tabla de doble entrada
            let mut rows: Vec<String> = vec![];
            let mut cols: Vec<String> = vec![];
            for (k, _) in &f.entries {
                if !rows.contains(&k[0]) {
                    rows.push(k[0].clone());
                }
                if !cols.contains(&k[1]) {
                    cols.push(k[1].clone());
                }
            }
            if cols.len() <= 40 {
                let map: std::collections::HashMap<(&str, &str), usize> = f.entries.iter().map(|(k, j)| ((k[0].as_str(), k[1].as_str()), *j)).collect();
                let mut columns = vec![json!({"label": format!("{} \\ {}", f.sets.first().cloned().unwrap_or_default(), f.sets.get(1).cloned().unwrap_or_default())})];
                columns.extend(cols.iter().map(|c| json!({"label": c, "align": "right"})));
                columns.push(json!({"label": "Total", "align": "right"}));
                let mut trs = vec![];
                let mut col_tot = vec![0.0; cols.len()];
                for r in &rows {
                    let mut cells = vec![json!(r)];
                    let mut tot = 0.0;
                    for (k, cn) in cols.iter().enumerate() {
                        match map.get(&(r.as_str(), cn.as_str())) {
                            Some(j) => {
                                let v = r6(values.get(*j).copied().unwrap_or(0.0));
                                tot += v;
                                col_tot[k] += v;
                                cells.push(json!(v));
                            }
                            None => cells.push(Value::Null),
                        }
                    }
                    cells.push(json!(r6(tot)));
                    trs.push(json!({"cells": cells}));
                }
                let mut last = vec![json!("Total")];
                last.extend(col_tot.iter().map(|x| json!(r6(*x))));
                last.push(json!(r6(col_tot.iter().sum())));
                trs.push(json!({"cells": last, "tone": "total"}));
                out.push(json!({"type": "table", "title": title, "columns": columns, "rows": trs}));
                continue;
            }
        }
        // lista: solo las distintas de cero si son muchas
        let many = f.entries.len() > 40;
        let mut columns: Vec<Value> = f.sets.iter().enumerate().map(|(i, s)| json!({"label": if s.is_empty() { format!("Índice {}", i + 1) } else { s.clone() }})).collect();
        if arity == 0 {
            columns.push(json!({"label": "Variable"}));
        }
        columns.push(json!({"label": "Valor", "align": "right"}));
        if reduced.is_some() {
            columns.push(json!({"label": "Costo reducido", "align": "right"}));
        }
        let mut trs = vec![];
        for (k, j) in &f.entries {
            let v = r6(values.get(*j).copied().unwrap_or(0.0));
            if many && v == 0.0 {
                continue;
            }
            let mut cells: Vec<Value> = k.iter().map(|x| json!(x)).collect();
            if arity == 0 {
                cells.push(json!(f.name));
            }
            cells.push(json!(v));
            if let Some(rc) = reduced {
                cells.push(json!(r6(rc.get(*j).copied().unwrap_or(0.0))));
            }
            trs.push(json!({"cells": cells}));
            if trs.len() >= 2000 {
                break;
            }
        }
        let mut t = json!({"type": "table", "title": title, "columns": columns, "rows": trs});
        if many {
            t["note"] = json!(format!("Solo las distintas de cero ({} de {}).", t["rows"].as_array().unwrap().len(), f.entries.len()));
        }
        out.push(t);
    }
    if let Some(d) = duals {
        for f in &c.cons {
            if f.entries.is_empty() || f.entries.len() > 2000 {
                continue;
            }
            let arity = f.entries[0].0.len();
            let mut columns: Vec<Value> = f.sets.iter().enumerate().map(|(i, s)| json!({"label": if s.is_empty() { format!("Índice {}", i + 1) } else { s.clone() }})).collect();
            if arity == 0 {
                columns.push(json!({"label": "Restricción"}));
            }
            columns.push(json!({"label": "Lado izquierdo", "align": "right"}));
            columns.push(json!({"label": "Precio sombra", "align": "right"}));
            let mut trs = vec![];
            for (k, i) in &f.entries {
                let mut cells: Vec<Value> = k.iter().map(|x| json!(x)).collect();
                if arity == 0 {
                    cells.push(json!(f.name));
                }
                cells.push(json!(r6(activity.and_then(|a| a.get(*i)).copied().unwrap_or(0.0))));
                cells.push(json!(r6(d.get(*i).copied().unwrap_or(0.0))));
                trs.push(json!({"cells": cells}));
            }
            out.push(json!({"type": "table", "title": format!("Restricción {}", f.name), "columns": columns, "rows": trs}));
        }
    }
    out
}

fn ident(s: &str) -> String {
    let t: String = s.chars().map(|c| if c.is_alphanumeric() || c == '_' { c } else { '_' }).collect();
    // el lector exige que un nombre empiece por letra o «_» (no por dígito, «²»…)
    if t.chars().next().is_none_or(|c| !(c.is_alphabetic() || c == '_')) {
        format!("v_{t}")
    } else if parse::kw(&t).is_some() || FUNCIONES.contains(&t.to_lowercase().as_str()) {
        // una palabra del lenguaje (Suma, en, o, max…) no puede ser un nombre
        format!("{t}_")
    } else {
        t
    }
}

/// Funciones del lenguaje (no pueden ser nombres de variables ni de restricciones).
const FUNCIONES: &[&str] = &[
    "abs", "min", "max", "sqrt", "exp", "log", "floor", "ceil", "round", "card", "pwl", "if", "atmost", "atleast", "exactly", "implies", "z",
];

/// Nombres válidos y distintos (al limpiar «x 1» y «x_1» quedarían iguales).
fn idents<'a>(it: impl Iterator<Item = &'a str>) -> Vec<String> {
    let mut usados = std::collections::HashSet::new();
    it.map(|s| {
        let base = ident(s);
        let mut n = base.clone();
        let mut k = 2;
        while !usados.insert(n.to_lowercase()) {
            n = format!("{base}_{k}");
            k += 1;
        }
        n
    })
    .collect()
}

fn num(x: f64) -> String {
    compile::fmt_num(x)
}

/// La hoja de LP-ILP escrita en el lenguaje (para pasar de la hoja al modelo algebraico).
pub fn from_sheet(m: &LpModel) -> String {
    // un solo espacio de nombres: variables y restricciones no pueden repetirse
    let mut todos = idents(m.variables.iter().map(|v| v.name.as_str()).chain(m.constraints.iter().map(|c| c.name.as_str())));
    let rnames = todos.split_off(m.variables.len());
    let names = todos;
    // el nombre va en un comentario: sin saltos de línea (no puede volverse una instrucción)
    let mut out = format!("# {}\n# Escrito por IAQSB a partir de la hoja.\n\n", m.name.replace(['\r', '\n'], " "));
    for (j, v) in m.variables.iter().enumerate() {
        let (lb, ub) = m.effective_bounds(j);
        let mut line = format!("var {}", names[j]);
        match v.var_type {
            VarType::Binary => line.push_str(" binary"),
            VarType::Unrestricted => line.push_str(" free"),
            _ => {
                if lb == f64::NEG_INFINITY {
                    line.push_str(" free");
                } else if lb != 0.0 {
                    line.push_str(&format!(" >= {}", num(lb)));
                }
                if ub.is_finite() {
                    line.push_str(&format!(" <= {}", num(ub)));
                }
                match v.var_type {
                    VarType::Integer => line.push_str(" integer"),
                    VarType::SemiContinuous => line.push_str(" semicontinuous"),
                    VarType::SemiInteger => line.push_str(" semiinteger"),
                    _ => {}
                }
            }
        }
        line.push_str(";\n");
        out.push_str(&line);
    }
    let expr = |coefs: &[f64]| -> String {
        let mut s = String::new();
        for (j, a) in coefs.iter().enumerate() {
            if *a == 0.0 || j >= names.len() {
                continue;
            }
            let mag = if (a.abs() - 1.0).abs() < 1e-15 { String::new() } else { format!("{}*", num(a.abs())) };
            if s.is_empty() {
                s = format!("{}{mag}{}", if *a < 0.0 { "-" } else { "" }, names[j]);
            } else {
                s.push_str(&format!(" {} {mag}{}", if *a < 0.0 { "-" } else { "+" }, names[j]));
            }
        }
        if s.is_empty() { "0".into() } else { s }
    };
    let costs: Vec<f64> = m.variables.iter().map(|v| v.cost).collect();
    out.push_str(&format!("\n{} z: {};\n\nsujeto a\n", if m.objective == Sense::Max { "maximizar" } else { "minimizar" }, expr(&costs)));
    for (c, nombre) in m.constraints.iter().zip(&rnames) {
        let d = match c.direction {
            Direction::Le => "<=",
            Direction::Ge => ">=",
            Direction::Eq => "=",
        };
        out.push_str(&format!("{nombre}: {} {d} {};\n", expr(&c.coefs), num(c.rhs)));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use iaqsb_core::solver::{self, Params, Status};

    fn solve(src: &str) -> (Compiled, solver::Solution) {
        let c = compile_text(src, "prueba", None).unwrap_or_else(|e| panic!("{e}"));
        let s = solver::solve(&c.model, &Params::default());
        (c, s)
    }

    #[test]
    fn transporte_con_indices_en_espanol() {
        let src = "
            # Transporte: 3 plantas y 4 ciudades (Winston, Powerco)
            conjunto Plantas = {P1, P2, P3};
            conjunto Ciudades = {C1, C2, C3, C4};
            parametro oferta{Plantas} = [35, 50, 40];
            parametro demanda{Ciudades} = [45, 20, 30, 30];
            parametro c{Plantas, Ciudades} = [[8, 6, 10, 9], [9, 12, 13, 7], [14, 9, 16, 5]];
            variable x{Plantas, Ciudades} >= 0;
            minimizar costo: suma{i en Plantas, j en Ciudades} c[i,j] * x[i,j];
            sujeto a
            ofe{i en Plantas}: suma{j en Ciudades} x[i,j] <= oferta[i];
            dem{j en Ciudades}: suma{i en Plantas} x[i,j] >= demanda[j];
        ";
        let (c, s) = solve(src);
        assert_eq!(s.status, Status::Optimal);
        assert!((s.objective.unwrap() - 1020.0).abs() < 1e-6);
        assert_eq!(c.model.cols.len(), 12);
        assert_eq!(c.model.cols[0].name, "x[P1,C1]");
        let t = tables(&c, &s.x, None, s.row_dual.as_deref(), Some(&s.row_activity));
        assert_eq!(t[0]["rows"].as_array().unwrap().len(), 4); // 3 plantas + total
    }

    #[test]
    fn ingles_y_portugues_y_wyndor_sin_indices() {
        let en = "maximize z: 3x + 5y; subject to c1: x <= 4; c2: 2y <= 12; c3: 3x + 2y <= 18;";
        let (_, s) = solve(en);
        assert!((s.objective.unwrap() - 36.0).abs() < 1e-6);
        let pt = "variavel x >= 0; variavel y >= 0; maximizar z: 3*x + 5*y; sujeito a c1: x <= 4; c2: 2*y <= 12; c3: 3*x + 2*y <= 18;";
        let (_, s) = solve(pt);
        assert!((s.objective.unwrap() - 36.0).abs() < 1e-6);
        // con alguna declaración var, un nombre sin declarar es un error (evita erratas)
        let err = compile_text("var x; maximize z: 3x + 2w;", "", None).unwrap_err();
        assert!(err.msg.contains("no está definido"), "{err}");
    }

    #[test]
    fn rangos_condiciones_y_parametros_calculados() {
        let src = "
            set T = 1..6;
            param d{T} = [100, 120, 90, 150, 130, 110];
            param h = 2; param cp = 10;
            param cap{t in T} = if t <= 3 then 120 else 140;
            var p{T} >= 0; var s{T} >= 0;
            minimize total: sum{t in T} (cp * p[t] + h * s[t]);
            bal{t in T: t > 1}: s[t-1] + p[t] - s[t] = d[t];
            ini: p[1] - s[1] = d[1];
            capm{t in T}: p[t] <= cap[t];
        ";
        let (_, s) = solve(src);
        assert_eq!(s.status, Status::Optimal);
        // demanda total 700, costo de producción 7000 más el inventario que obliga la capacidad
        assert!(s.objective.unwrap() >= 7000.0 - 1e-6);
    }

    #[test]
    fn conjuntos_de_pares_y_red() {
        // ruta más corta en una red con arcos como conjunto de pares
        let src = "
            set N = {A, B, C, D};
            set Arcos = {(A,B), (A,C), (B,C), (B,D), (C,D)};
            param long{Arcos} = {(A,B): 4, (A,C): 2, (B,C): 1, (B,D): 5, (C,D): 8};
            var x{Arcos} >= 0 <= 1;
            minimize dist: sum{(i,j) in Arcos} long[i,j] * x[i,j];
            flujo{n in N}: sum{(i,j) in Arcos: i = n} x[i,j] - sum{(i,j) in Arcos: j = n} x[i,j] = if n = A then 1 else if n = D then -1 else 0;
        ";
        let (_, s) = solve(src);
        assert!((s.objective.unwrap() - 9.0).abs() < 1e-6, "{:?}", s.objective);
    }

    #[test]
    fn restricciones_generales_contra_enumeracion() {
        // min |x − 3| + max(y, 2x − 5) con x, y enteros en [0, 6] y x + y >= 4
        let src = "
            var x >= 0 <= 6 integer; var y >= 0 <= 6 integer;
            minimize f: abs(x - 3) + max(y, 2*x - 5);
            c: x + y >= 4;
        ";
        let (_, s) = solve(src);
        let mut best = f64::INFINITY;
        for x in 0..=6 {
            for y in 0..=6 {
                if x + y >= 4 {
                    let v = ((x - 3) as f64).abs() + (y as f64).max(2.0 * x as f64 - 5.0);
                    best = best.min(v);
                }
            }
        }
        assert!((s.objective.unwrap() - best).abs() < 1e-6, "{:?} vs {best}", s.objective);
    }

    #[test]
    fn indicadoras_disyunciones_y_logicas() {
        // localización: abrir una planta (y) permite producir hasta 100; costo fijo 500
        let src = "
            set P = {A, B, C};
            param fijo{P} = [500, 300, 400];
            param cu{P} = [2, 5, 3];
            var y{P} binary; var q{P} >= 0 <= 100;
            minimize costo: sum{p in P} (fijo[p]*y[p] + cu[p]*q[p]);
            dem: sum{p in P} q[p] >= 150;
            ind{p in P}: si y[p] = 0 entonces q[p] <= 0;
            alomas: atmost(2, y[A], y[B], y[C]);
        ";
        let (_, s) = solve(src);
        // abrir A y C: 900 + 2·100 + 3·50 = 1250; A y B: 800 + 200 + 250 = 1250; B y C: 700 + 3·100 + 5·50 = 1250 … se enumera
        let fijo = [500.0, 300.0, 400.0];
        let cu = [2.0, 5.0, 3.0];
        let mut best = f64::INFINITY;
        for mask in 1..8u32 {
            if mask.count_ones() > 2 {
                continue;
            }
            let open: Vec<usize> = (0..3).filter(|k| mask & (1 << k) != 0).collect();
            if open.len() as f64 * 100.0 < 150.0 {
                continue;
            }
            let mut by: Vec<usize> = open.clone();
            by.sort_by(|a, b| cu[*a].partial_cmp(&cu[*b]).unwrap());
            let mut left: f64 = 150.0;
            let mut cost: f64 = open.iter().map(|k| fijo[*k]).sum();
            for k in by {
                let q = left.min(100.0);
                cost += q * cu[k];
                left -= q;
            }
            best = best.min(cost);
        }
        assert!((s.objective.unwrap() - best).abs() < 1e-6, "{:?} vs {best}", s.objective);
        // disyunción: dos tareas en una máquina, una antes que la otra
        let src = "
            var s1 >= 0 <= 100; var s2 >= 0 <= 100; var fin >= 0 <= 200;
            minimize m: fin;
            orden: s1 + 5 <= s2 o s2 + 3 <= s1;
            f1: fin >= s1 + 5; f2: fin >= s2 + 3;
        ";
        let (_, s) = solve(src);
        assert!((s.objective.unwrap() - 8.0).abs() < 1e-6, "{:?}", s.objective);
        let src = "var a binary; var b binary; var z binary; maximize f: z - 0.5*a - 0.4*b; c: z = and(a, b);";
        let (_, s) = solve(src);
        assert!((s.objective.unwrap() - 0.1).abs() < 1e-6, "{:?}", s.objective);
    }

    #[test]
    fn lineal_por_tramos() {
        // costo cóncavo por tramos (descuentos): 0..100 a 5, 100..300 a 3; se piden al menos 150
        let src = "var q >= 0 <= 300; minimize costo: pwl(q; (0, 0), (100, 500), (300, 1100)); dem: q >= 150;";
        let (_, s) = solve(src);
        assert!((s.objective.unwrap() - 650.0).abs() < 1e-6, "{:?}", s.objective);
    }

    #[test]
    fn objetivo_cuadratico_y_varios_objetivos() {
        let src = "var x >= 0; var y >= 0; minimize f: (x - 2)^2 + (y - 1)^2; c: x + y <= 2;";
        let (_, s) = solve(src);
        assert!((s.objective.unwrap() - 0.5).abs() < 1e-5, "{:?}", s.objective);
        let src = "var x >= 0 <= 10; var y >= 0 <= 10; maximize a: x + y; maximize b: x; c: x + y <= 12;";
        let (_, s) = solve(src);
        assert!((s.x[0] - 10.0).abs() < 1e-6 && (s.x[1] - 2.0).abs() < 1e-6, "{:?}", s.x);
    }

    #[test]
    fn errores_con_posicion() {
        let e = compile_text("var x >= 0;\nmaximize z: 3x +;\n", "", None).unwrap_err();
        assert_eq!(e.pos.line, 2);
        let e = compile_text("set I = {a, b};\nvar x{I};\nc: x[c] <= 1;", "", None).unwrap_err();
        assert!(e.msg.contains("no existe"), "{e}");
        let e = compile_text("var x; var y; c: x * y <= 1;", "", None).unwrap_err();
        assert!(e.msg.contains("no es lineal"), "{e}");
        let e = compile_text("var x free; minimize f: abs(x);", "", None).unwrap_err();
        assert!(e.msg.contains("cotas finitas"), "{e}");
        assert!(check("var x free; option bigM = 1000; minimize f: abs(x - 3);", None)["ok"] == true);
    }

    #[test]
    fn datos_externos_reemplazan_los_del_texto() {
        let src = "set I; param a{I}; var x{I} >= 0 <= 10; maximize f: sum{i in I} a[i]*x[i]; c: sum{i in I} x[i] <= 12;";
        // «set I;» sin valor: se admite con datos
        let src = src.replace("set I;", "set I = {};").replace("param a{I};", "param a{I} default 0;");
        let data = json!({"I": ["p", "q"], "a": [3, 5]});
        let c = compile_text(&src, "", Some(&data)).unwrap();
        let s = solver::solve(&c.model, &Params::default());
        assert!((s.objective.unwrap() - (5.0 * 10.0 + 3.0 * 2.0)).abs() < 1e-6, "{:?}", s.objective);
    }

    #[test]
    fn hoja_a_lenguaje_y_de_vuelta() {
        let m: LpModel = serde_json::from_value(json!({
            "name": "Wyndor", "objective": "max",
            "variables": [{"name": "Puertas", "cost": 3}, {"name": "Ventanas", "cost": 5}],
            "constraints": [
                {"name": "Planta1", "coefs": [1, 0], "direction": "<=", "rhs": 4},
                {"name": "Planta2", "coefs": [0, 2], "direction": "<=", "rhs": 12},
                {"name": "Planta3", "coefs": [3, 2], "direction": "<=", "rhs": 18}
            ]
        }))
        .unwrap();
        let txt = from_sheet(&m);
        let (_, s) = solve(&txt);
        assert!((s.objective.unwrap() - 36.0).abs() < 1e-6, "{txt}");
    }

    #[test]
    fn modelo_grande_rapido() {
        // transporte de 200 × 300 (60 000 variables) en lenguaje: armarlo debe tomar menos de unos segundos
        let src = "set I = 1..200; set J = 1..300;
            param c{i in I, j in J} = 1 + ((i * 7 + j * 13) mod 50);
            var x{I, J} >= 0;
            minimize z: sum{i in I, j in J} c[i,j]*x[i,j];
            o{i in I}: sum{j in J} x[i,j] <= 30;
            d{j in J}: sum{i in I} x[i,j] >= 19;";
        // «mod» no existe: se usa una fórmula equivalente
        let src = src.replace("((i * 7 + j * 13) mod 50)", "(i * 7 + j * 13) - 50 * floor((i * 7 + j * 13) / 50)");
        let t0 = std::time::Instant::now();
        let c = compile_text(&src, "", None).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(c.model.cols.len(), 60_000);
        assert!(t0.elapsed().as_secs_f64() < 20.0, "{:?}", t0.elapsed());
    }
}
