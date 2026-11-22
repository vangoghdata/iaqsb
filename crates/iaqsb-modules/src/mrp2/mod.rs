//! MRP II por matrices (método Gozinto; no existe en WinQSB). Con la matriz de
//! la lista de materiales se explotan en una sola multiplicación la demanda
//! sincronizada, el inventario deseado y las órdenes pendientes, y con las tasas
//! de producción se calcula la carga de cada departamento frente a su capacidad
//! (Arango Marín, 2021; Vazsonyi, 1954).
//!
//! - RB = LM × Ds (requerimientos brutos)
//! - Ipr = Ds + Ifd (el inventario final deseado en el último periodo)
//! - RN = LM × Ipr (requerimientos netos)
//! - Dneta = Ipr − Opd ajustada (órdenes pendientes más existencias en el primer periodo)
//! - NP = LM × Dneta (necesidades a programar); los negativos se reparten en los
//!   periodos siguientes del horizonte
//! - Tmq = Tpr × NP (tiempo de máquina), Tdisp = Tmax − Ctr, Tres = Tdisp − Tmq
//! - RA = Fcal × NP (requerimiento ajustado por calidad)
//!
//! LM es la matriz de requerimientos totales (con 1 en la diagonal), como en el
//! curso; también se acepta la estructura directa N y se calcula LM = (I − N)⁻¹.

use iaqsb_core::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

const EPS: f64 = 1e-9;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
pub struct Articulo {
    #[serde(default)]
    pub name: String,
    /// Inventario final deseado (Ifd).
    #[serde(default)]
    pub final_inventory: f64,
    /// Existencias disponibles (Exd).
    #[serde(default)]
    pub on_hand: f64,
    /// Punto de reorden (Reo; vacío = no aplica).
    #[serde(default)]
    pub reorder_point: Option<f64>,
    /// Factor de calidad (Fcal; 1 = sin desperdicio).
    #[serde(default = "uno")]
    pub quality_factor: f64,
}

fn uno() -> f64 {
    1.0
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
pub struct Nombre {
    #[serde(default)]
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Mrp2Model {
    #[serde(default)]
    pub name: String,
    #[serde(default = "pt")]
    pub problem_type: String,
    pub items: Vec<Articulo>,
    pub periods: Vec<Nombre>,
    /// Matriz de la lista de materiales: fila = componente, columna = artículo que lo usa.
    pub bom: Vec<Vec<f64>>,
    /// "total" (LM de requerimientos totales, con 1 en la diagonal) o "directa" (cantidades por unidad del padre inmediato).
    #[serde(default = "total")]
    pub bom_kind: String,
    /// Demanda sincronizada (Ds): artículo × periodo.
    pub demand: Vec<Vec<f64>>,
    /// Órdenes pendientes (Opd) sin las existencias: artículo × periodo.
    #[serde(default)]
    pub receipts: Vec<Vec<f64>>,
    #[serde(default)]
    pub departments: Vec<Nombre>,
    /// Tasa de producción (Tpr): departamento × artículo, tiempo por unidad.
    #[serde(default)]
    pub rates: Vec<Vec<f64>>,
    /// Tiempo máximo (Tmax): departamento × periodo, en horas.
    #[serde(default)]
    pub capacity: Vec<Vec<f64>>,
    /// Carga de trabajo ya comprometida (Ctr): departamento × periodo, en horas.
    #[serde(default)]
    pub workload: Vec<Vec<f64>>,
    /// Unidad de las tasas: "minutos" (se divide por 60) u "horas".
    #[serde(default = "minutos")]
    pub rate_unit: String,
}

fn pt() -> String {
    "gozinto".into()
}
fn total() -> String {
    "total".into()
}
fn minutos() -> String {
    "minutos".into()
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct Mrp2Options {
    pub steps: bool,
    /// Repartir los negativos de NP en los periodos siguientes.
    pub spread_negatives: bool,
}
impl Default for Mrp2Options {
    fn default() -> Self {
        Mrp2Options { steps: true, spread_negatives: true }
    }
}

type Mat = Vec<Vec<f64>>;

fn mul(a: &Mat, b: &Mat) -> Mat {
    let n = a.len();
    let k = b.len();
    let m = b.first().map_or(0, |r| r.len());
    (0..n).map(|i| (0..m).map(|j| tidy((0..k).map(|l| a[i][l] * b[l][j]).sum())).collect()).collect()
}

/// Inversa por Gauss-Jordan con pivoteo parcial; None si es singular.
fn inversa(a: &Mat) -> Option<Mat> {
    let n = a.len();
    let mut m: Mat = a.iter().enumerate().map(|(i, r)| r.iter().cloned().chain((0..n).map(|j| if i == j { 1.0 } else { 0.0 })).collect()).collect();
    for c in 0..n {
        let p = (c..n).max_by(|&x, &y| m[x][c].abs().partial_cmp(&m[y][c].abs()).unwrap())?;
        if m[p][c].abs() < 1e-12 {
            return None;
        }
        m.swap(c, p);
        let d = m[c][c];
        for v in m[c].iter_mut() {
            *v /= d;
        }
        for r in 0..n {
            if r != c && m[r][c] != 0.0 {
                let f = m[r][c];
                let fila = m[c].clone();
                for (v, w) in m[r].iter_mut().zip(fila) {
                    *v -= f * w;
                }
            }
        }
    }
    Some(m.into_iter().map(|r| r[n..].iter().map(|x| tidy((x * 1e9).round() / 1e9)).collect()).collect())
}

fn r2(x: f64) -> f64 {
    tidy((x * 100.0).round() / 100.0)
}

fn check(what: &str, m: &Mat, rows: usize, cols: usize) -> Result<(), ModError> {
    if m.len() != rows || m.iter().any(|r| r.len() != cols) {
        return invalid(format!("la matriz «{what}» debe tener {rows} filas y {cols} columnas"));
    }
    if m.iter().flatten().any(|x| !x.is_finite()) {
        return invalid(format!("la matriz «{what}» tiene valores no numéricos"));
    }
    Ok(())
}

fn tabla(titulo: &str, filas: &[String], cols: &[String], m: &Mat, malo_si_negativo: bool) -> Block {
    let mut c: Vec<String> = vec![String::new()];
    c.extend(cols.iter().cloned());
    let mut t = Table::new(Some(titulo), &c).left(0);
    for (i, f) in filas.iter().enumerate() {
        let mut row = vec![Cell::from(f.as_str())];
        row.extend(m[i].iter().map(|x| Cell::from(r2(*x))));
        let tono = if malo_si_negativo && m[i].iter().any(|x| *x < -EPS) { Tone::Bad } else { Tone::Normal };
        t.row_tone(row, tono);
    }
    t.block()
}

fn solve_mrp2(m: &Mrp2Model, o: &Mrp2Options) -> Result<Report, ModError> {
    let n = m.items.len();
    let p = m.periods.len();
    if n == 0 || p == 0 {
        return invalid("agrega los artículos y los periodos");
    }
    let items: Vec<String> = m.items.iter().enumerate().map(|(k, a)| if a.name.trim().is_empty() { format!("Artículo {}", k + 1) } else { a.name.trim().to_string() }).collect();
    let pers: Vec<String> = m.periods.iter().enumerate().map(|(k, a)| if a.name.trim().is_empty() { (k + 1).to_string() } else { a.name.trim().to_string() }).collect();
    check_names("artículos", &items)?;
    check("lista de materiales", &m.bom, n, n)?;
    check("demanda sincronizada", &m.demand, n, p)?;
    let receipts = if m.receipts.is_empty() { vec![vec![0.0; p]; n] } else { m.receipts.clone() };
    check("órdenes pendientes", &receipts, n, p)?;

    // matriz de requerimientos totales
    let (lm, directa) = match m.bom_kind.as_str() {
        "total" => {
            for i in 0..n {
                if (m.bom[i][i] - 1.0).abs() > 1e-9 {
                    return invalid(format!("la matriz de requerimientos totales lleva 1 en la diagonal ({} tiene {})", items[i], fmt(m.bom[i][i])));
                }
            }
            let inv = inversa(&m.bom).ok_or_else(|| ModError::Invalid("la lista de materiales no es válida (matriz singular)".into()))?;
            let dir: Mat = (0..n).map(|i| (0..n).map(|j| tidy(if i == j { 0.0 } else { -inv[i][j] })).collect()).collect();
            (m.bom.clone(), dir)
        }
        "directa" => {
            let ia: Mat = (0..n).map(|i| (0..n).map(|j| if i == j { 1.0 - m.bom[i][j] } else { -m.bom[i][j] }).collect()).collect();
            let t = inversa(&ia).ok_or_else(|| ModError::Invalid("la lista de materiales tiene un ciclo (un artículo es componente de sí mismo)".into()))?;
            if t.iter().flatten().any(|x| *x < -1e-9) {
                return invalid("la lista de materiales tiene un ciclo (un artículo es componente de sí mismo)");
            }
            (t, m.bom.clone())
        }
        o => return invalid(format!("tipo de lista de materiales desconocido: «{o}» (usa total o directa)")),
    };

    let ds = &m.demand;
    let rb = mul(&lm, ds);
    let mut ipr = ds.clone();
    for i in 0..n {
        ipr[i][p - 1] += m.items[i].final_inventory;
    }
    let rn = mul(&lm, &ipr);
    let mut opd = receipts.clone();
    for i in 0..n {
        opd[i][0] += m.items[i].on_hand;
    }
    let dneta: Mat = (0..n).map(|i| (0..p).map(|t| tidy(ipr[i][t] - opd[i][t])).collect()).collect();
    let np0 = mul(&lm, &dneta);
    // los negativos (sobrantes) cubren las necesidades de los periodos siguientes
    let mut np = np0.clone();
    let mut sobrante = vec![0.0; n];
    if o.spread_negatives {
        for i in 0..n {
            let mut s = 0.0;
            for t in 0..p {
                let v = np0[i][t] - s;
                if v < 0.0 {
                    s = -v;
                    np[i][t] = 0.0;
                } else {
                    s = 0.0;
                    np[i][t] = tidy(v);
                }
            }
            sobrante[i] = tidy(s);
        }
    }
    let ra: Mat = (0..n).map(|i| np[i].iter().map(|x| tidy(x * m.items[i].quality_factor)).collect()).collect();

    // capacidad
    let d = m.departments.len();
    let deps: Vec<String> = m.departments.iter().enumerate().map(|(k, a)| if a.name.trim().is_empty() { format!("Departamento {}", k + 1) } else { a.name.trim().to_string() }).collect();
    let mut cap_bloques = vec![];
    let mut sobrecargas = vec![];
    let mut tres_data = Value::Null;
    if d > 0 {
        check("tasas de producción", &m.rates, d, n)?;
        let div = match m.rate_unit.as_str() {
            "minutos" => 60.0,
            "horas" => 1.0,
            o => return invalid(format!("unidad de las tasas desconocida: «{o}» (usa minutos u horas)")),
        };
        let tmq_min = mul(&m.rates, &np);
        let tmq: Mat = tmq_min.iter().map(|r| r.iter().map(|x| tidy(x / div)).collect()).collect();
        let tmax = if m.capacity.is_empty() { vec![vec![0.0; p]; d] } else { m.capacity.clone() };
        check("tiempo máximo", &tmax, d, p)?;
        let ctr = if m.workload.is_empty() { vec![vec![0.0; p]; d] } else { m.workload.clone() };
        check("carga de trabajo", &ctr, d, p)?;
        let tdisp: Mat = (0..d).map(|k| (0..p).map(|t| tidy(tmax[k][t] - ctr[k][t])).collect()).collect();
        let tres: Mat = (0..d).map(|k| (0..p).map(|t| tidy(tdisp[k][t] - tmq[k][t])).collect()).collect();
        for k in 0..d {
            for t in 0..p {
                if tres[k][t] < -EPS {
                    sobrecargas.push(format!("{} en {} ({} h)", deps[k], pers[t], fmt(r2(tres[k][t]))));
                }
            }
        }
        cap_bloques.push(tabla(if div == 60.0 { "Tiempos de máquina Tmq = Tpr × NP / 60 (horas)" } else { "Tiempos de máquina Tmq = Tpr × NP (horas)" }, &deps, &pers, &tmq, false));
        cap_bloques.push(tabla("Tiempo disponible Tdisp = Tmax − Ctr (horas)", &deps, &pers, &tdisp, false));
        cap_bloques.push(tabla("Tiempo residual Tres = Tdisp − Tmq (horas)", &deps, &pers, &tres, true));
        cap_bloques.push(if sobrecargas.is_empty() {
            text_tone("La capacidad alcanza en todos los departamentos y periodos.", Tone::Good)
        } else {
            text_tone(format!("Falta capacidad: {}. Hay que trabajar horas extra, subcontratar o mover producción a otros periodos.", sobrecargas.join("; ")), Tone::Bad)
        });
        let mut ch = Chart::new(Some("Carga frente al tiempo disponible"), "Departamento y periodo", "Horas");
        let cats: Vec<String> = (0..d).flat_map(|k| pers.iter().map(move |t| (k, t.clone()))).map(|(k, t)| format!("{} · {}", deps[k], t)).collect();
        ch.categories = Some(cats);
        let mut k_idx = 0.0;
        let mut carga = vec![];
        let mut disp = vec![];
        for k in 0..d {
            for t in 0..p {
                carga.push([k_idx, r2(tmq[k][t])]);
                disp.push([k_idx, r2(tdisp[k][t])]);
                k_idx += 1.0;
            }
        }
        ch = ch.series(Series::new("Tiempo de máquina (Tmq)", SeriesKind::Bar, carga).tone(Tone::Accent)).series(Series::new("Tiempo disponible (Tdisp)", SeriesKind::Line, disp).tone(Tone::Bad).dashed());
        cap_bloques.push(ch.block());
        tres_data = json!({"machine_time": tmq, "available_time": tdisp, "residual_time": tres});
    }

    // puntos de reorden
    let bajo_reorden: Vec<String> = m.items.iter().zip(&items).filter(|(a, _)| a.reorder_point.is_some_and(|r| a.on_hand < r - EPS)).map(|(a, nme)| format!("{nme} ({} < {})", fmt(a.on_hand), fmt(a.reorder_point.unwrap()))).collect();

    let total_np: f64 = np.iter().flatten().sum();
    let head = format!(
        "Necesidades a programar de {} artículos en {} periodos (total {}).{}",
        n,
        p,
        fmt(r2(total_np)),
        if d == 0 {
            String::new()
        } else if sobrecargas.is_empty() {
            " La capacidad alcanza en todos los departamentos.".to_string()
        } else if sobrecargas.len() == 1 {
            format!(" Falta capacidad en {}.", sobrecargas[0])
        } else {
            format!(" Falta capacidad en {} casos.", sobrecargas.len())
        }
    );

    // --- solución
    let mut ms = vec![metric_main("Total a programar", r2(total_np)), metric("Artículos", n as f64), metric("Periodos", p as f64)];
    if d > 0 {
        ms.push(metric("Casos sin capacidad", sobrecargas.len() as f64));
    }
    let mut sol = vec![metrics(ms), tabla("Necesidades a programar NP = LM × Dneta", &items, &pers, &np, false), tabla("Requerimiento ajustado por calidad RA = Fcal × NP", &items, &pers, &ra, false)];
    if sobrante.iter().any(|x| *x > EPS) {
        let lista: Vec<String> = (0..n).filter(|&i| sobrante[i] > EPS).map(|i| format!("{} {}", items[i], fmt(r2(sobrante[i])))).collect();
        sol.push(text(format!("Sobrantes al final del horizonte (por encima del inventario final deseado): {}.", lista.join(", "))));
    }
    if !bajo_reorden.is_empty() {
        sol.push(text_tone(format!("Existencias por debajo del punto de reorden: {}.", bajo_reorden.join(", ")), Tone::Highlight));
    }

    let matrices = vec![
        tabla("Requerimientos brutos RB = LM × Ds", &items, &pers, &rb, false),
        tabla("Inventario programado Ipr = Ds + Ifd", &items, &pers, &ipr, false),
        tabla("Requerimientos netos RN = LM × Ipr", &items, &pers, &rn, false),
        tabla("Órdenes pendientes ajustadas (con las existencias)", &items, &pers, &opd, false),
        tabla("Demanda neta Dneta = Ipr − Opd ajustada", &items, &pers, &dneta, false),
        tabla("LM × Dneta antes de repartir los negativos", &items, &pers, &np0, false),
        tabla(if m.bom_kind == "total" { "Lista de materiales LM (requerimientos totales)" } else { "Requerimientos totales LM = (I − N)⁻¹" }, &items, &items, &lm, false),
        tabla("Estructura directa N (unidades por unidad del padre inmediato)", &items, &items, &directa, false),
    ];

    let mut pasos = vec![];
    if o.steps {
        pasos.push(step("Requerimientos brutos", "RB = LM × Ds: cada columna de LM dice cuántas unidades de cada artículo lleva una unidad del producto (contando los niveles intermedios).", vec![matrices[0].clone()]));
        pasos.push(step("Inventario programado y requerimientos netos", "Ipr = Ds con el inventario final deseado sumado al último periodo; RN = LM × Ipr.", vec![matrices[1].clone(), matrices[2].clone()]));
        pasos.push(step("Demanda neta", "Opd ajustada = órdenes pendientes con las existencias sumadas al primer periodo; Dneta = Ipr − Opd ajustada.", vec![matrices[3].clone(), matrices[4].clone()]));
        pasos.push(step(
            "Necesidades a programar",
            if o.spread_negatives { "NP = LM × Dneta. Un valor negativo es un sobrante: se reparte en los periodos siguientes del horizonte (cubre sus necesidades) y la celda queda en 0." } else { "NP = LM × Dneta (sin repartir los negativos)." },
            vec![matrices[5].clone(), tabla("NP", &items, &pers, &np, false)],
        ));
        if d > 0 {
            pasos.push(step("Capacidad", "Tmq = Tpr × NP (dividido por 60 si las tasas están en minutos); Tdisp = Tmax − Ctr; Tres = Tdisp − Tmq. Un Tres negativo es falta de capacidad.", cap_bloques.clone()));
        }
        pasos.push(step("Ajuste por calidad", "RA = Fcal × NP: lo que hay que lanzar para obtener NP unidades buenas.", vec![sol[2].clone()]));
    }

    let rep = Report::new("mrp2", &m.name, Status::Solved, head)
        .section("solucion", "Necesidades a programar", sol)
        .section("matrices", "Matrices", matrices)
        .section("capacidad", "Capacidad", cap_bloques)
        .section("pasos", "Paso a paso", if pasos.is_empty() { vec![] } else { vec![steps(pasos)] })
        .data(json!({
            "items": items, "periods": pers, "gross": rb, "scheduled_inventory": ipr, "net": rn, "adjusted_orders": opd,
            "net_demand": dneta, "to_schedule": np, "quality_adjusted": ra, "surplus": sobrante, "total_requirements": lm,
            "capacity": tres_data, "overloads": sobrecargas, "below_reorder_point": bajo_reorden,
        }))
        .cite("arango2021gozinto", "Método matricial de MRP II (Gozinto): requerimientos, demanda neta, necesidades a programar, tiempos de máquina y factores de calidad")
        .cite("vazsonyi1954", "Matriz de requerimientos totales (Gozinto) para explotar la lista de materiales");
    Ok(rep)
}

pub struct Mrp2;

impl TypedModule for Mrp2 {
    type Model = Mrp2Model;
    type Options = Mrp2Options;

    fn file_tag(&self) -> &'static str {
        "IAQSB-MRP2"
    }

    fn spec(&self) -> ModuleSpec {
        let mat = |title: &str, path: &str, rows: Axis, cols: Axis, diag: bool| Widget::Matrix { title: Some(title.into()), path: path.into(), rows, cols, cell: Field::number("v", "Valor"), allow_diagonal: diag };
        ModuleSpec {
            id: "mrp2".into(),
            winqsb: "Nuevo".into(),
            name: "MRP II por matrices (Gozinto)".into(),
            description: "Explosión de materiales con la matriz Gozinto: requerimientos brutos y netos, demanda neta, necesidades a programar, tiempos de máquina frente a la capacidad y ajuste por calidad".into(),
            file_ext: "MR2".into(),
            problem_types: vec![ProblemType {
                id: "gozinto".into(),
                label: "MRP II (Gozinto)".into(),
                description: "Matrices LM, Ds, Ifd, Exd, Opd, Reo, Tpr, Ctr y Fcal".into(),
                dims: vec![Field::integer("items", "Número de artículos", 4), Field::integer("periods", "Número de periodos", 3), Field::integer("departments", "Número de departamentos", 2)],
                layout: vec![
                    Widget::params(Some("Lista de materiales"), vec![Field::choice("bom_kind", "La matriz LM es", &[("total", "De requerimientos totales (1 en la diagonal)"), ("directa", "La estructura directa (padre inmediato)")]), Field::choice("rate_unit", "Las tasas de producción están en", &[("minutos", "minutos por unidad"), ("horas", "horas por unidad")])]),
                    Widget::records(
                        Some("Artículos"),
                        "items",
                        vec![
                            Field::text("name", "Artículo"),
                            Field::number("final_inventory", "Inventario final deseado (Ifd)"),
                            Field::number("on_hand", "Existencias (Exd)"),
                            Field::number("reorder_point", "Punto de reorden (Reo)").optional(),
                            Field::number("quality_factor", "Factor de calidad (Fcal)").default(1.0),
                        ],
                        Some("Añadir artículo"),
                    ),
                    Widget::records(Some("Periodos"), "periods", vec![Field::text("name", "Periodo")], Some("Añadir periodo")),
                    mat("Lista de materiales LM (fila: componente; columna: artículo que lo usa)", "bom", Axis::new("items", "Componente"), Axis::new("items", "Usado en"), true),
                    mat("Demanda sincronizada (Ds)", "demand", Axis::new("items", "Artículo"), Axis::new("periods", "Periodo"), true),
                    mat("Órdenes pendientes (Opd, sin las existencias)", "receipts", Axis::new("items", "Artículo"), Axis::new("periods", "Periodo"), true),
                    Widget::records(Some("Departamentos"), "departments", vec![Field::text("name", "Departamento")], Some("Añadir departamento")),
                    mat("Tasas de producción (Tpr): tiempo por unidad", "rates", Axis::new("departments", "Departamento"), Axis::new("items", "Artículo"), true),
                    mat("Tiempo máximo (Tmax, horas)", "capacity", Axis::new("departments", "Departamento"), Axis::new("periods", "Periodo"), true),
                    mat("Carga de trabajo comprometida (Ctr, horas)", "workload", Axis::new("departments", "Departamento"), Axis::new("periods", "Periodo"), true),
                ],
            }],
            options: vec![Field::boolean("steps", "Mostrar paso a paso", true), Field::boolean("spread_negatives", "Repartir los negativos en los periodos siguientes", true)],
        }
    }

    fn template(&self, _pt: &str, dims: &Value) -> Result<Mrp2Model, ModError> {
        let n = dim(dims, "items", 4, 1, 500)?;
        let p = dim(dims, "periods", 3, 1, 240)?;
        let d = dim(dims, "departments", 2, 0, 100)?;
        Ok(Mrp2Model {
            name: "MRP II (Gozinto)".into(),
            problem_type: "gozinto".into(),
            items: (1..=n).map(|i| Articulo { name: format!("Artículo {i}"), quality_factor: 1.0, ..Default::default() }).collect(),
            periods: (1..=p).map(|i| Nombre { name: i.to_string() }).collect(),
            bom: (0..n).map(|i| (0..n).map(|j| if i == j { 1.0 } else { 0.0 }).collect()).collect(),
            bom_kind: "total".into(),
            demand: vec![vec![0.0; p]; n],
            receipts: vec![vec![0.0; p]; n],
            departments: (1..=d).map(|i| Nombre { name: format!("Dpto. {i}") }).collect(),
            rates: vec![vec![0.0; n]; d],
            capacity: vec![vec![0.0; p]; d],
            workload: vec![vec![0.0; p]; d],
            rate_unit: "minutos".into(),
        })
    }

    fn solve(&self, m: &Mrp2Model, o: &Mrp2Options) -> Result<Report, ModError> {
        solve_mrp2(m, o)
    }

    fn read(&self, bytes: &[u8]) -> Result<Mrp2Model, ModError> {
        let s = String::from_utf8_lossy(bytes);
        let s = s.trim_start_matches('\u{feff}');
        let (head, rest) = s.split_once('\n').unwrap_or((s, ""));
        if !head.trim().to_ascii_uppercase().starts_with("IAQSB-MRP2") {
            return Err(ModError::Format("no es un archivo de MRP II de IAQSB".into()));
        }
        serde_json::from_str(rest).map_err(|e| ModError::Format(e.to_string()))
    }

    fn write(&self, m: &Mrp2Model) -> Result<Vec<u8>, ModError> {
        Ok(format!("IAQSB-MRP2 1\n{}\n", serde_json::to_string_pretty(m).unwrap()).into_bytes())
    }

    fn examples(&self) -> Vec<(&'static str, Mrp2Model)> {
        vec![("estantes_arango", serde_json::from_str(include_str!("ejemplos/estantes.json")).unwrap())]
    }
}
