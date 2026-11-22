//! Método símplex de tabla con gran M simbólica, como el que muestra WinQSB.
//!
//! El problema se pasa a forma estándar (minimizar, variables >= 0, lado derecho >= 0).
//! Los costes se guardan en dos partes (coeficiente de M, parte constante) para que
//! la tabla muestre C(j)-Z(j) con la M explícita, igual que el original.

use crate::model::{Direction, LpModel, Objective};
use serde::Serialize;

/// Qué representa cada columna de la tabla.
#[derive(Debug, Clone, PartialEq)]
pub enum ColKind {
    /// Parte de una variable original: x_j = shift_j + sign * columna.
    Var { var: usize, sign: f64 },
    /// Holgura (+1) de una fila <=, o exceso (-1) de una fila >=.
    Slack { row: usize, coef: f64 },
    /// Variable artificial de una fila >= o =.
    Artificial { row: usize },
}

#[derive(Debug, Clone)]
pub struct Col {
    pub name: String,
    pub kind: ColKind,
}

/// Origen de cada fila de la tabla.
#[derive(Debug, Clone, PartialEq)]
pub enum RowOrigin {
    Constraint(usize),
    /// Cota superior de una variable, añadida como fila.
    UpperBound(usize),
}

#[derive(Debug, Clone)]
pub struct RowInfo {
    pub name: String,
    pub origin: RowOrigin,
    /// +1 si la fila se dejó como estaba, -1 si se multiplicó por -1 para que b >= 0.
    pub flip: f64,
    /// Columna que vale e_r en la tabla inicial (holgura o artificial).
    pub ident: usize,
}

#[derive(Debug, Clone)]
pub struct VarMap {
    pub shift: f64,
    pub cols: Vec<(usize, f64)>,
    /// Fila de cota superior asociada, si la hay.
    pub ub_row: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct Tableau {
    pub a: Vec<Vec<f64>>,
    pub b: Vec<f64>,
    pub cost_m: Vec<f64>,
    pub cost_c: Vec<f64>,
    pub basis: Vec<usize>,
    pub cols: Vec<Col>,
    pub rows: Vec<RowInfo>,
    pub vars: Vec<VarMap>,
    /// +1 si el problema original es de minimizar, -1 si es de maximizar.
    pub sense: f64,
    /// Término constante del objetivo original (por los desplazamientos de cotas).
    pub offset: f64,
    pub eps: f64,
}

/// Valor con parte en M, para mostrar "2M + 3".
#[derive(Debug, Clone, Copy, Serialize, schemars::JsonSchema, PartialEq)]
pub struct MValue {
    pub m: f64,
    pub c: f64,
}

/// Fotografía de una iteración del símplex.
#[derive(Debug, Clone, Serialize, schemars::JsonSchema)]
pub struct TableauSnapshot {
    pub iteration: usize,
    pub columns: Vec<String>,
    /// C(j) en el sentido original del problema.
    pub cj: Vec<MValue>,
    pub basis: Vec<String>,
    /// C(B) de cada fila.
    pub cb: Vec<MValue>,
    pub rows: Vec<Vec<f64>>,
    pub rhs: Vec<f64>,
    /// C(j)-Z(j) en el sentido original.
    pub cj_minus_zj: Vec<MValue>,
    pub objective: MValue,
    pub entering: Option<String>,
    pub leaving: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SimplexStatus {
    Optimal,
    Infeasible,
    Unbounded,
    IterationLimit,
}

impl Tableau {
    /// Construye la tabla inicial a partir del modelo. `bounds` permite sustituir
    /// las cotas (lo usa branch and bound).
    pub fn build(model: &LpModel, bounds: &[(f64, f64)]) -> Tableau {
        let n = model.variables.len();
        let sense = if model.objective == Objective::Min {
            1.0
        } else {
            -1.0
        };
        let mut cols: Vec<Col> = Vec::new();
        let mut vars: Vec<VarMap> = Vec::with_capacity(n);
        let mut offset = 0.0;

        // 1) columnas de variables originales
        for j in 0..n {
            let v = &model.variables[j];
            let (l, u) = bounds[j];
            if l.is_finite() {
                cols.push(Col {
                    name: v.name.clone(),
                    kind: ColKind::Var { var: j, sign: 1.0 },
                });
                vars.push(VarMap {
                    shift: l,
                    cols: vec![(cols.len() - 1, 1.0)],
                    ub_row: None,
                });
            } else if u.is_finite() {
                cols.push(Col {
                    name: format!("Neg_{}", v.name),
                    kind: ColKind::Var { var: j, sign: -1.0 },
                });
                vars.push(VarMap {
                    shift: u,
                    cols: vec![(cols.len() - 1, -1.0)],
                    ub_row: None,
                });
            } else {
                cols.push(Col {
                    name: v.name.clone(),
                    kind: ColKind::Var { var: j, sign: 1.0 },
                });
                cols.push(Col {
                    name: format!("Neg_{}", v.name),
                    kind: ColKind::Var { var: j, sign: -1.0 },
                });
                vars.push(VarMap {
                    shift: 0.0,
                    cols: vec![(cols.len() - 2, 1.0), (cols.len() - 1, -1.0)],
                    ub_row: None,
                });
            }
            offset += v.cost * vars[j].shift;
        }
        let nstruct = cols.len();

        // 2) filas: restricciones + cotas superiores
        struct RawRow {
            coefs: Vec<f64>,
            dir: Direction,
            rhs: f64,
            name: String,
            origin: RowOrigin,
        }
        let mut raw: Vec<RawRow> = Vec::new();
        for (i, c) in model.constraints.iter().enumerate() {
            let mut coefs = vec![0.0; nstruct];
            let mut rhs = c.rhs;
            for j in 0..n {
                let aij = c.coefs[j];
                if aij == 0.0 {
                    continue;
                }
                rhs -= aij * vars[j].shift;
                for &(col, s) in &vars[j].cols {
                    coefs[col] += aij * s;
                }
            }
            raw.push(RawRow {
                coefs,
                dir: c.direction,
                rhs,
                name: c.name.clone(),
                origin: RowOrigin::Constraint(i),
            });
        }
        for j in 0..n {
            let (l, u) = bounds[j];
            if l.is_finite() && u.is_finite() {
                let mut coefs = vec![0.0; nstruct];
                coefs[vars[j].cols[0].0] = 1.0;
                vars[j].ub_row = Some(raw.len());
                raw.push(RawRow {
                    coefs,
                    dir: Direction::Le,
                    rhs: u - l,
                    name: format!("UB_{}", model.variables[j].name),
                    origin: RowOrigin::UpperBound(j),
                });
            }
        }

        // 3) normalizar b >= 0 y añadir holguras / artificiales
        let m = raw.len();
        let mut rows_info: Vec<RowInfo> = Vec::with_capacity(m);
        let mut row_dirs = Vec::with_capacity(m);
        for r in raw.iter_mut() {
            let mut flip = 1.0;
            if r.rhs < 0.0 {
                flip = -1.0;
                r.rhs = -r.rhs;
                for x in r.coefs.iter_mut() {
                    *x = -*x;
                }
                r.dir = match r.dir {
                    Direction::Le => Direction::Ge,
                    Direction::Ge => Direction::Le,
                    Direction::Eq => Direction::Eq,
                };
            }
            row_dirs.push(r.dir);
            rows_info.push(RowInfo {
                name: r.name.clone(),
                origin: r.origin.clone(),
                flip,
                ident: usize::MAX,
            });
        }
        for (ri, dir) in row_dirs.iter().enumerate() {
            match dir {
                Direction::Le => {
                    cols.push(Col {
                        name: format!("Slack_{}", rows_info[ri].name),
                        kind: ColKind::Slack { row: ri, coef: 1.0 },
                    });
                    rows_info[ri].ident = cols.len() - 1;
                }
                Direction::Ge => {
                    cols.push(Col {
                        name: format!("Surplus_{}", rows_info[ri].name),
                        kind: ColKind::Slack {
                            row: ri,
                            coef: -1.0,
                        },
                    });
                }
                Direction::Eq => {}
            }
        }
        for (ri, dir) in row_dirs.iter().enumerate() {
            if *dir != Direction::Le {
                cols.push(Col {
                    name: format!("Artificial_{}", rows_info[ri].name),
                    kind: ColKind::Artificial { row: ri },
                });
                rows_info[ri].ident = cols.len() - 1;
            }
        }

        let ncol = cols.len();
        let mut a = vec![vec![0.0; ncol]; m];
        let mut b = vec![0.0; m];
        for (ri, r) in raw.iter().enumerate() {
            a[ri][..nstruct].copy_from_slice(&r.coefs);
            b[ri] = r.rhs;
        }
        let mut cost_m = vec![0.0; ncol];
        let mut cost_c = vec![0.0; ncol];
        for (k, c) in cols.iter().enumerate() {
            match c.kind {
                ColKind::Var { var, sign } => cost_c[k] = sense * model.variables[var].cost * sign,
                ColKind::Slack { row, coef } => a[row][k] = coef,
                ColKind::Artificial { row } => {
                    a[row][k] = 1.0;
                    cost_m[k] = 1.0;
                }
            }
        }
        let basis: Vec<usize> = rows_info.iter().map(|r| r.ident).collect();

        // tolerancia de cero como WinQSB: 1e-5 * menor coeficiente no nulo, mínimo 1e-6
        let mut minabs: f64 = 1.0;
        for v in &model.variables {
            if v.cost != 0.0 {
                minabs = minabs.min(v.cost.abs());
            }
        }
        for c in &model.constraints {
            for x in c.coefs.iter().chain(std::iter::once(&c.rhs)) {
                if *x != 0.0 {
                    minabs = minabs.min(x.abs());
                }
            }
        }
        let eps = (minabs * 1e-5).max(1e-6);

        Tableau {
            a,
            b,
            cost_m,
            cost_c,
            basis,
            cols,
            rows: rows_info,
            vars,
            sense,
            offset,
            eps,
        }
    }

    pub fn m(&self) -> usize {
        self.a.len()
    }
    pub fn ncol(&self) -> usize {
        self.cols.len()
    }

    /// Costes reducidos (parte M, parte constante) del problema interno de minimizar.
    pub fn reduced_costs(&self) -> (Vec<f64>, Vec<f64>) {
        let n = self.ncol();
        let mut dm = self.cost_m.clone();
        let mut dc = self.cost_c.clone();
        for (r, &bc) in self.basis.iter().enumerate() {
            let (cm, cc) = (self.cost_m[bc], self.cost_c[bc]);
            if cm == 0.0 && cc == 0.0 {
                continue;
            }
            let row = &self.a[r];
            for j in 0..n {
                let x = row[j];
                if x != 0.0 {
                    dm[j] -= cm * x;
                    dc[j] -= cc * x;
                }
            }
        }
        for &bc in &self.basis {
            dm[bc] = 0.0;
            dc[bc] = 0.0;
        }
        (dm, dc)
    }

    fn is_basic(&self, j: usize) -> bool {
        self.basis.contains(&j)
    }

    fn pivot(&mut self, r: usize, e: usize) {
        let p = self.a[r][e];
        let n = self.ncol();
        for j in 0..n {
            self.a[r][j] /= p;
        }
        self.b[r] /= p;
        self.a[r][e] = 1.0;
        let prow = self.a[r].clone();
        let pb = self.b[r];
        for i in 0..self.m() {
            if i == r {
                continue;
            }
            let f = self.a[i][e];
            if f == 0.0 {
                continue;
            }
            let row = &mut self.a[i];
            for j in 0..n {
                if prow[j] != 0.0 {
                    row[j] -= f * prow[j];
                }
            }
            row[e] = 0.0;
            self.b[i] -= f * pb;
            if self.b[i].abs() < 1e-12 {
                self.b[i] = 0.0;
            }
        }
        self.basis[r] = e;
    }

    /// Elige la columna que entra: la de C(j)-Z(j) más favorable, mirando primero
    /// el coeficiente de M y después la parte constante (regla de WinQSB).
    fn choose_entering(&self, dm: &[f64], dc: &[f64], bland: bool) -> Option<usize> {
        let tol = self.eps;
        let mut best: Option<usize> = None;
        for j in 0..self.ncol() {
            if self.is_basic(j) {
                continue;
            }
            let negative = dm[j] < -tol || (dm[j].abs() <= tol && dc[j] < -tol);
            if !negative {
                continue;
            }
            if bland {
                return Some(j);
            }
            best = match best {
                None => Some(j),
                Some(k) => {
                    let better = if dm[j] < dm[k] - tol {
                        true
                    } else if (dm[j] - dm[k]).abs() <= tol {
                        dc[j] < dc[k] - tol
                    } else {
                        false
                    };
                    if better {
                        Some(j)
                    } else {
                        Some(k)
                    }
                }
            };
        }
        best
    }

    /// Prueba del cociente mínimo.
    fn choose_leaving(&self, e: usize, bland: bool) -> Option<usize> {
        let mut best: Option<(usize, f64)> = None;
        for r in 0..self.m() {
            let x = self.a[r][e];
            if x <= 1e-9 {
                continue;
            }
            let ratio = self.b[r] / x;
            best = match best {
                None => Some((r, ratio)),
                Some((k, rk)) => {
                    if ratio < rk - 1e-12 * (1.0 + rk.abs()) {
                        Some((r, ratio))
                    } else if (ratio - rk).abs() <= 1e-12 * (1.0 + rk.abs()) {
                        // empate: sale primero una artificial, después el menor índice
                        let art_r =
                            matches!(self.cols[self.basis[r]].kind, ColKind::Artificial { .. });
                        let art_k =
                            matches!(self.cols[self.basis[k]].kind, ColKind::Artificial { .. });
                        let pick_r = if bland {
                            self.basis[r] < self.basis[k]
                        } else if art_r != art_k {
                            art_r
                        } else {
                            false
                        };
                        if pick_r {
                            Some((r, ratio))
                        } else {
                            Some((k, rk))
                        }
                    } else {
                        Some((k, rk))
                    }
                }
            };
        }
        best.map(|(r, _)| r)
    }

    /// Resuelve. Si `trace` no es None, guarda la tabla de cada iteración.
    pub fn solve(
        &mut self,
        mut trace: Option<&mut Vec<TableauSnapshot>>,
        max_iter: usize,
    ) -> (SimplexStatus, usize) {
        let mut iter = 0;
        let bland_after = 50 + 10 * (self.m() + self.ncol());
        loop {
            let (dm, dc) = self.reduced_costs();
            let bland = iter > bland_after;
            let entering = self.choose_entering(&dm, &dc, bland);
            let leaving = entering.and_then(|e| self.choose_leaving(e, bland));
            if let Some(t) = trace.as_deref_mut() {
                t.push(self.snapshot(iter, &dm, &dc, entering, leaving));
            }
            let Some(e) = entering else {
                // óptimo del problema con gran M: comprobar artificiales
                let infeasible = self.basis.iter().enumerate().any(|(r, &bc)| {
                    matches!(self.cols[bc].kind, ColKind::Artificial { .. })
                        && self.b[r] > 1e-7 * (1.0 + self.b[r].abs())
                });
                return (
                    if infeasible {
                        SimplexStatus::Infeasible
                    } else {
                        SimplexStatus::Optimal
                    },
                    iter,
                );
            };
            let Some(r) = leaving else {
                // sin cota: si aún quedan artificiales positivas el problema no es factible
                let infeasible = self.basis.iter().enumerate().any(|(r, &bc)| {
                    matches!(self.cols[bc].kind, ColKind::Artificial { .. }) && self.b[r] > 1e-7
                });
                // con precio lexicográfico solo entra por la parte constante cuando la parte M
                // ya es mínima: si quedan artificiales positivas, el problema no es factible
                let _ = dm;
                return (
                    if infeasible {
                        SimplexStatus::Infeasible
                    } else {
                        SimplexStatus::Unbounded
                    },
                    iter,
                );
            };
            self.pivot(r, e);
            iter += 1;
            if iter >= max_iter {
                return (SimplexStatus::IterationLimit, iter);
            }
        }
    }

    fn orig_cost(&self, j: usize) -> MValue {
        // en el sentido original: max -> los costes internos se cambian de signo
        MValue {
            m: self.sense * self.cost_m[j],
            c: self.sense * self.cost_c[j],
        }
    }

    fn snapshot(
        &self,
        iter: usize,
        dm: &[f64],
        dc: &[f64],
        entering: Option<usize>,
        leaving: Option<usize>,
    ) -> TableauSnapshot {
        let mut zm = 0.0;
        let mut zc = 0.0;
        for (r, &bc) in self.basis.iter().enumerate() {
            zm += self.cost_m[bc] * self.b[r];
            zc += self.cost_c[bc] * self.b[r];
        }
        TableauSnapshot {
            iteration: iter,
            columns: self.cols.iter().map(|c| c.name.clone()).collect(),
            cj: (0..self.ncol()).map(|j| self.orig_cost(j)).collect(),
            basis: self
                .basis
                .iter()
                .map(|&j| self.cols[j].name.clone())
                .collect(),
            cb: self.basis.iter().map(|&j| self.orig_cost(j)).collect(),
            rows: self
                .a
                .iter()
                .map(|r| r.iter().map(|x| clean(*x)).collect())
                .collect(),
            rhs: self.b.iter().map(|x| clean(*x)).collect(),
            cj_minus_zj: (0..self.ncol())
                .map(|j| MValue {
                    m: clean(self.sense * dm[j]),
                    c: clean(self.sense * dc[j]),
                })
                .collect(),
            objective: MValue {
                m: clean(self.sense * zm),
                c: clean(self.sense * zc + self.offset),
            },
            entering: entering.map(|e| self.cols[e].name.clone()),
            leaving: leaving.map(|r| self.cols[self.basis[r]].name.clone()),
        }
    }

    /// Valor de cada columna en la solución actual.
    pub fn col_values(&self) -> Vec<f64> {
        let mut v = vec![0.0; self.ncol()];
        for (r, &bc) in self.basis.iter().enumerate() {
            v[bc] = self.b[r];
        }
        v
    }

    /// Valores de las variables originales.
    pub fn var_values(&self) -> Vec<f64> {
        let cv = self.col_values();
        self.vars
            .iter()
            .map(|vm| clean(vm.shift + vm.cols.iter().map(|&(c, s)| s * cv[c]).sum::<f64>()))
            .collect()
    }
}

/// Redondea a cero el ruido numérico.
pub fn clean(x: f64) -> f64 {
    if x.abs() < 1e-10 {
        0.0
    } else {
        let r = (x * 1e9).round() / 1e9;
        if (r - x).abs() < 1e-9 * (1.0 + x.abs()) {
            r
        } else {
            x
        }
    }
}
