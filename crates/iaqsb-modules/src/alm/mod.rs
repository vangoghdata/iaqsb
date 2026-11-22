//! ALM · Almacenes (no existe en WinQSB):
//!
//! - Paletizado y bodega en bloque: cajas por tendido (dos orientaciones),
//!   tendidos que permiten la altura, el montacargas y la resistencia de la
//!   caja, cajas por estiba, estibas por lote y el área mínima de una bodega
//!   en bloque con pasillos alrededor de cada lote (con plano a escala).
//! - Dimensionado del almacén: espacio requerido en cada período y costo
//!   anual de construir un almacén propio de cierto tamaño y rentar el resto;
//!   se evalúan alternativas y se encuentra el tamaño óptimo.

use iaqsb_core::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum AlmType {
    #[default]
    Paletizado,
    Dimensionado,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
pub struct Period {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub demand: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
pub struct Alternative {
    #[serde(default)]
    pub area: Option<f64>,
}

fn d120() -> f64 {
    120.0
}
fn d100() -> f64 {
    100.0
}
fn d145() -> f64 {
    14.5
}
fn d30() -> f64 {
    30.0
}
fn one() -> f64 {
    1.0
}
fn two() -> f64 {
    2.0
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AlmModel {
    #[serde(default)]
    pub name: String,
    pub problem_type: AlmType,
    // ---- paletizado (medidas en cm y kg)
    #[serde(default)]
    pub box_length: f64,
    #[serde(default)]
    pub box_width: f64,
    #[serde(default)]
    pub box_height: f64,
    /// Peso de la caja llena (o se calcula con las unidades por caja).
    #[serde(default)]
    pub box_weight: Option<f64>,
    #[serde(default)]
    pub units_per_box: Option<f64>,
    #[serde(default)]
    pub unit_weight: Option<f64>,
    #[serde(default)]
    pub empty_box_weight: Option<f64>,
    #[serde(default = "d120")]
    pub pallet_length: f64,
    #[serde(default = "d100")]
    pub pallet_width: f64,
    #[serde(default = "d145")]
    pub pallet_height: f64,
    #[serde(default = "d30")]
    pub pallet_weight: f64,
    /// Capacidad del montacargas (kg, incluida la estiba).
    #[serde(default)]
    pub max_load: Option<f64>,
    /// Altura máxima de la carga (cm, incluida la estiba).
    #[serde(default)]
    pub max_height: Option<f64>,
    /// Peso que resiste el primer tendido (kg encima de él).
    #[serde(default)]
    pub bottom_layer_max: Option<f64>,
    /// Unidades por lote (p. ej. botellas producidas en un día).
    #[serde(default)]
    pub lot_units: Option<f64>,
    /// Días que reposa cada lote (espacios = días + 1).
    #[serde(default)]
    pub rest_days: Option<f64>,
    /// Número de espacios (lotes) si se da directamente.
    #[serde(default)]
    pub spaces: Option<f64>,
    /// Largo total del montacargas (m) y holgura (m): ancho del pasillo.
    #[serde(default)]
    pub forklift_length: Option<f64>,
    #[serde(default)]
    pub aisle_clearance: Option<f64>,
    /// Ancho del pasillo (m) si se da directamente.
    #[serde(default)]
    pub aisle_width: Option<f64>,
    /// Filas de estibas por lote (2 = toda estiba da a un pasillo).
    #[serde(default = "two")]
    pub rows_per_lot: f64,
    /// Estibas apiladas una sobre otra.
    #[serde(default = "one")]
    pub stack_levels: f64,

    // ---- dimensionado
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub periods: Vec<Period>,
    /// Unidad de la demanda (lb, botellas...).
    #[serde(default)]
    pub demand_unit: String,
    /// Unidades de demanda por unidad de manejo (p. ej. 9 botellas por caja).
    #[serde(default)]
    pub units_per_handling: Option<f64>,
    /// Rotación del inventario en cada período (inventario = demanda / rotación).
    #[serde(default = "one")]
    pub turnover: f64,
    /// "area" (área por unidad almacenada) o "estibas".
    #[serde(default)]
    pub space_mode: String,
    #[serde(default)]
    pub area_per_unit: Option<f64>,
    #[serde(default)]
    pub units_per_pallet: Option<f64>,
    #[serde(default)]
    pub pallet_area: Option<f64>,
    /// % del espacio para pasillos y % utilizable por la variabilidad.
    #[serde(default)]
    pub aisle_share: f64,
    #[serde(default = "d100")]
    pub usable_share: f64,
    #[serde(default)]
    pub area_unit: String,
    /// Inversión por unidad de área y su amortización.
    #[serde(default)]
    pub investment_per_area: f64,
    #[serde(default)]
    pub amortization_years: Option<f64>,
    /// Con interés: tasa por período (%) y número de períodos (factor A/P).
    #[serde(default)]
    pub interest_rate: Option<f64>,
    #[serde(default)]
    pub amortization_periods: Option<f64>,
    /// Costos fijos de operación por unidad de área: por año y por período.
    #[serde(default)]
    pub fixed_cost_year: f64,
    #[serde(default)]
    pub fixed_cost_period: f64,
    /// Costo variable propio por unidad de demanda (de manejo).
    #[serde(default)]
    pub variable_cost: f64,
    /// Renta: almacenaje por unidad en inventario por período y manejo por unidad de demanda.
    #[serde(default)]
    pub rent_storage: f64,
    #[serde(default)]
    pub rent_handling: f64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub alternatives: Vec<Alternative>,
    /// Períodos por año (12 = meses).
    #[serde(default)]
    pub periods_per_year: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct AlmOptions {
    pub steps: bool,
}
impl Default for AlmOptions {
    fn default() -> Self {
        AlmOptions { steps: true }
    }
}

fn r2(x: f64) -> f64 {
    (x * 100.0).round() / 100.0
}
fn r4(x: f64) -> f64 {
    (x * 10000.0).round() / 10000.0
}
fn fl(x: f64) -> f64 {
    (x + 1e-9).floor()
}
fn cl(x: f64) -> f64 {
    (x - 1e-9).ceil()
}

fn paletizado(m: &AlmModel, o: &AlmOptions) -> Result<Report, ModError> {
    let (bl, bw, bh) = (m.box_length, m.box_width, m.box_height);
    if bl <= 0.0 || bw <= 0.0 || bh <= 0.0 {
        return invalid("indica las medidas de la caja (largo, ancho y alto en cm)");
    }
    let (pl, pw) = (m.pallet_length, m.pallet_width);
    let (wbox, wtxt) = match (m.box_weight, m.units_per_box, m.unit_weight) {
        (Some(w), _, _) => (w, format!("{} kg (dato)", fmt(w))),
        (None, Some(n), Some(u)) => {
            let e = m.empty_box_weight.unwrap_or(0.0);
            (n * u + e, format!("{}·{} + {} = {} kg", fmt(n), fmt(u), fmt(e), fmt(r4(n * u + e))))
        }
        _ => return invalid("indica el peso de la caja llena (o las unidades por caja y el peso de cada una)"),
    };
    let ora = fl(pl / bl) * fl(pw / bw);
    let orb = fl(pl / bw) * fl(pw / bl);
    let per = ora.max(orb);
    if per < 1.0 {
        return invalid("la caja no cabe en la estiba");
    }
    let layer_w = per * wbox;
    let mut lim = vec![];
    if let Some(h) = m.max_height {
        let n = fl((h - m.pallet_height) / bh);
        lim.push(("Altura", n, format!("⌊({} − {}) / {}⌋ = ⌊{}⌋ = {}", fmt(h), fmt(m.pallet_height), fmt(bh), fmt(r4((h - m.pallet_height) / bh)), fmt(n))));
    }
    if let Some(w) = m.max_load {
        let boxes = (w - m.pallet_weight) / wbox;
        let n = fl(boxes / per);
        lim.push(("Peso del montacargas", n, format!("({} − {}) / {} = {} cajas → ⌊{} / {}⌋ = {}", fmt(w), fmt(m.pallet_weight), fmt(r4(wbox)), fmt(r4(boxes)), fmt(r4(boxes)), fmt(per), fmt(n))));
    }
    if let Some(r) = m.bottom_layer_max {
        let above = fl(r / layer_w);
        lim.push(("Resistencia del primer tendido", above + 1.0, format!("⌊{} / {}⌋ = {} tendidos encima + el primero = {}", fmt(r), fmt(r4(layer_w)), fmt(above), fmt(above + 1.0))));
    }
    if lim.is_empty() {
        return invalid("indica al menos una restricción: altura máxima, capacidad del montacargas o resistencia del primer tendido");
    }
    let (bind, layers, _) = lim.iter().min_by(|a, b| a.1.partial_cmp(&b.1).unwrap()).cloned().unwrap();
    if layers < 1.0 {
        return invalid(format!("con la restricción de {bind} no cabe ni un tendido"));
    }
    let per_pallet = layers * per;
    let load = per_pallet * wbox;
    let height = m.pallet_height + layers * bh;
    let mut t = Table::new(Some("Restricciones de la estiba"), &["Restricción", "Cálculo", "Tendidos"]).left(0).left(1);
    for (n, v, txt) in &lim {
        t.row_tone(cells![*n, txt, *v], if *n == bind { Tone::Highlight } else { Tone::Normal });
    }
    let mut ms = vec![metric_main("Cajas por estiba", per_pallet), metric("Cajas por tendido", per), metric("Tendidos", layers), metric("Peso de la estiba (kg)", r2(load + m.pallet_weight)), metric("Altura de la estiba (cm)", r2(height))];
    let mut blocks = vec![];
    let mut charts = vec![];
    let mut head = format!("{} cajas por estiba ({} tendidos de {}; manda {}).", fmt(per_pallet), fmt(layers), fmt(per), bind.to_lowercase());
    let mut data = json!({"boxes_per_layer": per, "layers": layers, "boxes_per_pallet": per_pallet, "binding": bind, "box_weight": r4(wbox)});
    // vista superior de un tendido
    {
        let (a, b) = if ora >= orb { (bl, bw) } else { (bw, bl) };
        let (na, nb) = (fl(pl / a), fl(pw / b));
        let mut plan = FloorPlan::new(&format!("Vista superior de un tendido: {} cajas ({} × {})", fmt(per), fmt(na), fmt(nb)), pl, pw, "cm");
        plan.rect(0.0, 0.0, pl, pw, None, Tone::Muted, true);
        for i in 0..na as usize {
            for j in 0..nb as usize {
                plan.rect(i as f64 * a, j as f64 * b, a, b, Some(format!("{}×{}", fmt(a), fmt(b))), Tone::Accent, false);
            }
        }
        plan.dim(0.0, -4.0, pl, -4.0, format!("{} cm", fmt(pl)));
        plan.dim(-4.0, 0.0, -4.0, pw, format!("{} cm", fmt(pw)));
        charts.push(plan.block());
    }
    // lotes y bodega en bloque
    if let (Some(lu), Some(upb)) = (m.lot_units, m.units_per_box) {
        let boxes = cl(lu / upb);
        let pallets = cl(boxes / per_pallet);
        let last = boxes - (pallets - 1.0) * per_pallet;
        ms.push(metric("Estibas por lote", pallets));
        head += &format!(" Cada lote: {} cajas en {} estibas.", fmt(boxes), fmt(pallets));
        data["pallets_per_lot"] = json!(pallets);
        blocks.push(text(format!("Lote: {} unidades / {} por caja = {} cajas; ⌈{} / {}⌉ = {} estibas (la última con {} cajas).", fmt(lu), fmt(upb), fmt(boxes), fmt(boxes), fmt(per_pallet), fmt(pallets), fmt(last))));
        let spaces = m.spaces.or(m.rest_days.map(|d| d + 1.0));
        let aisle = m.aisle_width.or(m.forklift_length.map(|f| f + m.aisle_clearance.unwrap_or(0.0)));
        if let (Some(sp), Some(ai)) = (spaces, aisle) {
            let stacks = cl(pallets / m.stack_levels.max(1.0));
            let rows = m.rows_per_lot.max(1.0);
            let cols = cl(stacks / rows);
            let (p1, p2) = (pl / 100.0, pw / 100.0);
            let mut alts = vec![];
            for (face, depth) in [(p2, p1), (p1, p2)] {
                // face = lado de la estiba hacia el pasillo largo del bloque
                let (blk_w, blk_h) = (cols * face, rows * depth);
                for c in 1..=(sp as usize) {
                    let r = cl(sp / c as f64);
                    let w = c as f64 * blk_w + (c as f64 + 1.0) * ai;
                    let h = r * blk_h + (r + 1.0) * ai;
                    alts.push((w * h, c, r, blk_w, blk_h, w, h, face));
                }
            }
            alts.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
            let best = alts[0];
            ms.push(metric("Espacios (lotes)", sp));
            ms.push(metric("Área mínima (m²)", r2(best.0)));
            head += &format!(" Bodega para {} lotes: {} m² (pasillos de {} m).", fmt(sp), fmt(r2(best.0)), fmt(ai));
            data["spaces"] = json!(sp);
            data["aisle"] = json!(ai);
            data["area"] = json!(r2(best.0));
            if m.spaces.is_none() {
                blocks.push(text(format!("Espacios: {} lotes en reposo más 1 para el que entra el día en que sale el más viejo = {}.", fmt(m.rest_days.unwrap()), fmt(sp))));
            }
            blocks.push(text(format!(
                "Pasillo = {} m. Cada lote va en {} filas de {} estibas (así toda estiba da a un pasillo): bloque de {} × {} m. Con {} bloques por fila y {} filas, y pasillos alrededor de todos: {} × {} = {} m² ({} m² son estibas).",
                fmt(ai),
                fmt(rows),
                fmt(cols),
                fmt(r2(best.3)),
                fmt(r2(best.4)),
                best.1,
                fmt(best.2),
                fmt(r2(best.5)),
                fmt(r2(best.6)),
                fmt(r2(best.0)),
                fmt(r2(sp * stacks * p1 * p2))
            )));
            let mut at = Table::new(Some("Distribuciones probadas (de menor a mayor área)"), &["Bloques por fila", "Filas", "Lado hacia el pasillo (m)", "Bloque (m)", "Ancho (m)", "Largo (m)", "Área (m²)"]);
            for a in alts.iter().take(8) {
                at.row_tone(cells![a.1, a.2, a.7, format!("{} × {}", fmt(r2(a.3)), fmt(r2(a.4))), r2(a.5), r2(a.6), r2(a.0)], if std::ptr::eq(a, &alts[0]) { Tone::Highlight } else { Tone::Normal });
            }
            blocks.push(at.block());
            // plano
            let mut plan = FloorPlan::new(&format!("Bodega en bloque: {} lotes, pasillos de {} m", fmt(sp), fmt(ai)), best.5, best.6, "m");
            plan.rect(0.0, 0.0, best.5, best.6, Some("pasillos".into()), Tone::Muted, true);
            let mut lot = 0;
            for rr in 0..best.2 as usize {
                for cc in 0..best.1 {
                    if lot as f64 >= sp {
                        break;
                    }
                    lot += 1;
                    let x0 = ai + cc as f64 * (best.3 + ai);
                    let y0 = ai + rr as f64 * (best.4 + ai);
                    plan.rect(x0, y0, best.3, best.4, None, Tone::Normal, false);
                    let mut k = 0.0;
                    for row in 0..rows as usize {
                        for col in 0..cols as usize {
                            if k >= stacks {
                                break;
                            }
                            k += 1.0;
                            plan.rect(x0 + col as f64 * best.7, y0 + row as f64 * (best.4 / rows), best.7, best.4 / rows, None, Tone::Accent, false);
                        }
                    }
                    plan.rect(x0, y0, best.3, best.4, Some(format!("Lote {lot}")), Tone::Normal, true);
                }
            }
            plan.dim(0.0, -0.6, best.5, -0.6, format!("{} m", fmt(r2(best.5))));
            plan.dim(-0.6, 0.0, -0.6, best.6, format!("{} m", fmt(r2(best.6))));
            charts.push(plan.block());
        }
    }
    blocks.insert(0, metrics(ms));
    blocks.insert(1, t.clone().block());
    let st = if o.steps {
        vec![steps(vec![
            step("Peso de la caja", wtxt.clone(), vec![]),
            step("Cajas por tendido", format!("Se prueban las dos orientaciones: ⌊{}/{}⌋ × ⌊{}/{}⌋ = {} contra ⌊{}/{}⌋ × ⌊{}/{}⌋ = {}. Se toma la mayor: {}.", fmt(pl), fmt(bl), fmt(pw), fmt(bw), fmt(ora), fmt(pl), fmt(bw), fmt(pw), fmt(bl), fmt(orb), fmt(per)), vec![]),
            step("Tendidos por estiba", format!("Cada restricción da un número de tendidos (siempre hacia abajo) y manda la menor: {} → {} tendidos. Un tendido pesa {} kg.", bind, fmt(layers), fmt(r4(layer_w))), vec![t.block()]),
            step("Cajas por estiba", format!("{} × {} = {} cajas; carga {} kg + estiba {} kg = {} kg; altura {} cm.", fmt(layers), fmt(per), fmt(per_pallet), fmt(r2(load)), fmt(m.pallet_weight), fmt(r2(load + m.pallet_weight)), fmt(r2(height))), vec![]),
        ])]
    } else {
        vec![]
    };
    Ok(Report::new("alm", &m.name, Status::Solved, head)
        .section("solucion", "Solución", blocks)
        .section("graficos", "Planos", charts)
        .section("pasos", "Paso a paso", st)
        .data(data)
        .cite("sarache2026almacenes", "Paletizado, almacenaje en bloque y dimensionamiento desde el paletizado (diapositivas 13–25)")
        .cite("ntc4680", "Estiba estándar de 1000 × 1200 mm, 145 mm de alto y 30 kg")
        .cite("frazelle2007", "Diseño del almacén: unidades de carga, sistemas de almacenaje y pasillos"))
}

struct Alt {
    area: f64,
    rows: Vec<[f64; 8]>, // demanda, req, %propio, fijo, var, %rent, alm, manejo
    total: f64,
}

fn dimensionado(m: &AlmModel, o: &AlmOptions) -> Result<Report, ModError> {
    let ds: Vec<(String, f64)> = m.periods.iter().enumerate().map(|(k, p)| (if p.name.trim().is_empty() { (k + 1).to_string() } else { p.name.clone() }, p.demand.unwrap_or(0.0))).collect();
    if ds.is_empty() {
        return invalid("agrega la demanda de cada período");
    }
    if m.turnover <= 0.0 {
        return invalid("la rotación debe ser positiva");
    }
    let au = if m.area_unit.is_empty() { "m²".to_string() } else { m.area_unit.clone() };
    let du = if m.demand_unit.is_empty() { "unidades".to_string() } else { m.demand_unit.clone() };
    let pallet_mode = m.space_mode == "estibas";
    let factor_aisle = 1.0 / (1.0 - m.aisle_share / 100.0);
    let factor_use = 100.0 / m.usable_share;
    if m.aisle_share >= 100.0 || m.usable_share <= 0.0 {
        return invalid("los porcentajes de pasillos y de espacio utilizable deben estar entre 0 y 100");
    }
    // unidades de manejo, inventario y área de cada período
    let mut req = vec![];
    let mut rt = Table::new(Some("Espacio requerido por período"), &["Período", &format!("Demanda ({du})"), "Unidades de manejo", "Inventario", if pallet_mode { "Estibas" } else { "Área de producto" }, &format!("Requerido ({au})")]).left(0);
    let mut how = String::new();
    for (n, d) in &ds {
        let hu = match m.units_per_handling {
            Some(u) if u > 0.0 => cl(d / u),
            _ => *d,
        };
        let (inv, prod, area) = if pallet_mode {
            let upp = m.units_per_pallet.ok_or_else(|| ModError::Invalid("indica las unidades de manejo por estiba".into()))?;
            let pa = m.pallet_area.ok_or_else(|| ModError::Invalid("indica el área de una estiba".into()))?;
            let inv = cl(hu / m.turnover);
            let pal = cl(inv / upp);
            (inv, pal, pal * pa * factor_aisle * factor_use)
        } else {
            let apu = m.area_per_unit.ok_or_else(|| ModError::Invalid("indica el área por unidad almacenada".into()))?;
            let inv = hu / m.turnover;
            (inv, inv * apu, inv * apu * factor_aisle * factor_use)
        };
        rt.row(cells![n, *d, hu, r2(inv), r2(prod), r2(area)]);
        req.push((hu, inv, area));
    }
    if pallet_mode {
        how = format!(
            "Unidades de manejo = ⌈demanda / {}⌉; inventario = ⌈unidades / {}⌉ (rotación); estibas = ⌈inventario / {}⌉; requerido = estibas × {} {au}{}{}.",
            fmt(m.units_per_handling.unwrap_or(1.0)),
            fmt(m.turnover),
            fmt(m.units_per_pallet.unwrap_or(0.0)),
            fmt(m.pallet_area.unwrap_or(0.0)),
            if m.aisle_share > 0.0 { format!(" / {}", fmt(r4(1.0 - m.aisle_share / 100.0))) } else { String::new() },
            if m.usable_share < 100.0 { format!(" / {}", fmt(r4(m.usable_share / 100.0))) } else { String::new() }
        );
    } else if let Some(apu) = m.area_per_unit {
        let k = apu / m.turnover * factor_aisle * factor_use;
        let f6 = |x: f64| {
            let t = format!("{:.6}", x);
            t.trim_end_matches('0').trim_end_matches('.').to_string()
        };
        how = format!(
            "Requerido = demanda × (1/{}) × {}{}{} = demanda × {}.",
            fmt(m.turnover),
            f6(apu),
            if m.aisle_share > 0.0 { format!(" × (1/{})", fmt(r4(1.0 - m.aisle_share / 100.0))) } else { String::new() },
            if m.usable_share < 100.0 { format!(" × (1/{})", fmt(r4(m.usable_share / 100.0))) } else { String::new() },
            f6(k * m.units_per_handling.map(|u| 1.0 / u).unwrap_or(1.0))
        );
    }
    // costo fijo por área y período
    let ppy = m.periods_per_year.unwrap_or(ds.len() as f64);
    let (amort, amort_txt) = match (m.interest_rate, m.amortization_periods, m.amortization_years) {
        (Some(i), Some(n), _) => {
            let i = i / 100.0;
            let f = i * (1.0 + i).powf(n) / ((1.0 + i).powf(n) - 1.0);
            (m.investment_per_area * f, format!("cuota = inversión × i(1+i)ⁿ/((1+i)ⁿ − 1) = {} × {}·{}/({} − 1) = {} × {} = {}", fmt(m.investment_per_area), fmt(i), fmt(r4((1.0 + i).powf(n))), fmt(r4((1.0 + i).powf(n))), fmt(m.investment_per_area), fmt(r4(f)), fmt(r4(m.investment_per_area * f))))
        }
        (_, _, Some(y)) => (m.investment_per_area / (y * ppy), format!("inversión / (años × períodos) = {} / ({} × {}) = {}", fmt(m.investment_per_area), fmt(y), fmt(ppy), fmt(r4(m.investment_per_area / (y * ppy))))),
        _ => (0.0, "sin inversión".into()),
    };
    let fpa = amort + m.fixed_cost_year / ppy + m.fixed_cost_period;
    let ftxt = format!("Costo fijo por {au} y período F = {}{}{} = {}.", amort_txt, if m.fixed_cost_year > 0.0 { format!(" + {}/{}", fmt(m.fixed_cost_year), fmt(ppy)) } else { String::new() }, if m.fixed_cost_period > 0.0 { format!(" + {}", fmt(m.fixed_cost_period)) } else { String::new() }, fmt(r4(fpa)));
    let eval = |a: f64| -> Alt {
        let mut rows = vec![];
        let mut total = 0.0;
        for ((d, (hu, _inv, rq)), _) in ds.iter().map(|x| x.1).zip(req.iter()).zip(0..) {
            let own = if *rq <= 0.0 { 1.0 } else { (a / rq).min(1.0) };
            let rent = 1.0 - own;
            let fixed = a * fpa;
            let var = hu * own * m.variable_cost;
            let alm = hu * rent / m.turnover * m.rent_storage;
            let han = hu * rent * m.rent_handling;
            let tot = fixed + var + alm + han;
            total += tot;
            rows.push([d, *rq, own, fixed, var, rent, alm, han]);
        }
        Alt { area: a, rows, total }
    };
    // óptimo: el costo es lineal por tramos en el área; se evalúa en cada requerimiento
    let mut cand: Vec<f64> = req.iter().map(|r| r.2).collect();
    cand.push(0.0);
    let best = cand.iter().map(|&a| eval(a)).min_by(|a, b| a.total.partial_cmp(&b.total).unwrap()).unwrap();
    let mut alts: Vec<Alt> = m.alternatives.iter().filter_map(|a| a.area).map(eval).collect();
    alts.push(eval(best.area));
    alts.sort_by(|a, b| a.area.partial_cmp(&b.area).unwrap());
    alts.dedup_by(|a, b| (a.area - b.area).abs() < 1e-9);
    let peak = req.iter().map(|r| r.2).fold(0.0, f64::max);
    // análisis marginal
    let saving_per_unit = m.rent_storage / m.turnover + m.rent_handling - m.variable_cost;
    let units_per_area = {
        let (hu, rq): (f64, f64) = req.iter().fold((0.0, 0.0), |s, r| (s.0 + r.0, s.1 + r.2));
        if rq > 0.0 { hu / rq } else { 0.0 }
    };
    let save_area = saving_per_unit * units_per_area;
    let cost_area = fpa * ds.len() as f64;
    let need = if save_area > 0.0 { cost_area / save_area } else { f64::INFINITY };
    let mut sorted: Vec<f64> = req.iter().map(|r| r.2).collect();
    sorted.sort_by(|a, b| b.partial_cmp(a).unwrap());
    let marg = format!(
        "Análisis marginal: un {au} propio cuesta {} × {} = {} en el horizonte y, cada período en que se usa, ahorra ({}/{} + {} − {}) × {} = {} (cada {au} guarda {} {} de demanda). Para pagarse tiene que usarse al menos {} períodos: se construye hasta el requerimiento número ⌈{}⌉ de mayor a menor ({}).",
        fmt(r4(fpa)),
        ds.len(),
        fmt(r4(cost_area)),
        fmt(m.rent_storage),
        fmt(m.turnover),
        fmt(m.rent_handling),
        fmt(m.variable_cost),
        fmt(r4(units_per_area)),
        fmt(r4(save_area)),
        fmt(r4(units_per_area)),
        if m.units_per_handling.is_some() { "unidades de manejo" } else { du.as_str() },
        fmt(r4(need)),
        fmt(r4(need)),
        sorted.get((need.ceil() as usize).saturating_sub(1).min(sorted.len() - 1)).map(|x| format!("{} {au}", fmt(r2(*x)))).unwrap_or_default()
    );
    // tablas
    let mut at = Table::new(Some("Alternativas"), &[&format!("Área propia ({au})"), "Costo total", "Períodos con renta"]);
    for a in &alts {
        let nr = a.rows.iter().filter(|r| r[5] > 1e-12).count();
        at.row_tone(cells![r2(a.area), r2(a.total), nr], if (a.area - best.area).abs() < 1e-9 { Tone::Highlight } else { Tone::Normal });
    }
    let alt_table = |a: &Alt| -> Table {
        let mut t = Table::new(Some(&format!("Alternativa de {} {au}", fmt(r2(a.area)))), &["Período", "Demanda", "Requerido", "% propio", "Fijo", "Variable propio", "% rentado", "Almacenaje rentado", "Manejo", "Total"]).left(0);
        for ((n, _), r) in ds.iter().zip(&a.rows) {
            t.row_tone(cells![n, r[0], r2(r[1]), r2(100.0 * r[2]), r2(r[3]), r2(r[4]), r2(100.0 * r[5]), r2(r[6]), r2(r[7]), r2(r[3] + r[4] + r[6] + r[7])], if r[5] > 1e-12 { Tone::Highlight } else { Tone::Normal });
        }
        t.row_tone(cells!["Total", "", "", "", "", "", "", "", "", r2(a.total)], Tone::Total);
        t
    };
    let cats: Vec<String> = ds.iter().map(|x| x.0.clone()).collect();
    let mut rch = Chart::new(Some("Espacio requerido: propio hasta el área elegida, rentado por encima"), "Período", &format!("Área ({au})")).stacked();
    rch.categories = Some(cats.clone());
    rch = rch
        .series(Series::new("Propio", SeriesKind::Bar, req.iter().enumerate().map(|(k, r)| [k as f64, r2(r.2.min(best.area))]).collect()).tone(Tone::Accent))
        .series(Series::new("Rentado", SeriesKind::Bar, req.iter().enumerate().map(|(k, r)| [k as f64, r2((r.2 - best.area).max(0.0))]).collect()).tone(Tone::Highlight));
    rch = rch.hline(best.area, "área propia", Tone::Accent);
    let hi = peak * 1.15;
    let xs: Vec<f64> = (0..=80).map(|k| hi * k as f64 / 80.0).collect();
    let mut vch = Chart::new(Some("Costo total según el área propia"), &format!("Área propia ({au})"), "Costo total").series(Series::new("Costo total", SeriesKind::Line, xs.iter().map(|&a| [r2(a), r2(eval(a).total)]).collect()).tone(Tone::Highlight));
    vch = vch.series(Series::new("Alternativas", SeriesKind::Scatter, alts.iter().map(|a| [r2(a.area), r2(a.total)]).collect()));
    vch = vch.vline(best.area, format!("óptimo {}", fmt(r2(best.area))), Tone::Accent);
    let head = format!("Construir {} {au} propios (costo total {}); el pico es {} {au}.", fmt(r2(best.area)), fmt(r2(best.total)), fmt(r2(peak)));
    let mut blocks = vec![
        metrics(vec![metric_main(&format!("Área propia óptima ({au})"), r2(best.area)), metric("Costo total", r2(best.total)), metric(&format!("Pico de requerimiento ({au})"), r2(peak)), metric(&format!("F por {au} y período"), r4(fpa))]),
        text(how.clone()),
        text(ftxt.clone()),
        at.clone().block(),
        text(marg.clone()),
        rt.clone().block(),
    ];
    if (best.area - peak).abs() < 1e-9 {
        blocks.insert(1, text("El óptimo es construir hasta el pico: cada área propia se paga aunque se use poco, porque rentar es mucho más caro que construir."));
    }
    let mut steps_v = vec![
        step("Espacio requerido", how, vec![rt.block()]),
        step("Costo fijo del área propia", ftxt, vec![]),
    ];
    for a in &alts {
        steps_v.push(step(format!("Alternativa de {} {au}", fmt(r2(a.area))), "% propio = mín(1; área / requerido). Variable propio = unidades × % propio × costo variable. Almacenaje rentado = unidades × % rentado / rotación × renta (se cobra sobre el inventario). Manejo = unidades × % rentado × costo de manejo (sobre todo lo que entra y sale).", vec![alt_table(a).block()]));
    }
    steps_v.push(step("Tamaño óptimo", marg, vec![at.block()]));
    let _ = o;
    Ok(Report::new("alm", &m.name, Status::Optimal, head)
        .section("solucion", "Solución", blocks)
        .section("graficos", "Gráficos", vec![rch.block(), vch.block()])
        .section("alternativas", "Alternativas", vec![Block::Steps { title: Some("Costo mes a mes de cada alternativa".into()), steps: steps_v }])
        .data(json!({
            "optimal_area": r4(best.area), "optimal_cost": r2(best.total), "peak": r4(peak), "fixed_per_area": r4(fpa),
            "requirements": req.iter().map(|r| r4(r.2)).collect::<Vec<_>>(),
            "alternatives": alts.iter().map(|a| json!({"area": r4(a.area), "total": r2(a.total)})).collect::<Vec<_>>(),
        }))
        .cite("ballou2004", "Dimensionamiento del almacén: almacén propio para la base y rentado para los picos, con el costo total anual")
        .cite("sarache2026almacenes", "Dimensionado estratégico del almacén: espacio requerido y alternativas de tamaño (diapositivas 57–71)")
        .cite_if(m.interest_rate.is_some(), "blanktarquin", "Factor de recuperación de capital (A/P, i, n) para amortizar la inversión"))
}

pub struct Alm;

impl TypedModule for Alm {
    type Model = AlmModel;
    type Options = AlmOptions;

    fn file_tag(&self) -> &'static str {
        "IAQSB-ALM"
    }

    fn spec(&self) -> ModuleSpec {
        let opt = |k: &str, l: &str| Field::number(k, l).optional();
        let pt = |id: &str, label: &str, desc: &str, layout: Vec<Widget>, dims: Vec<Field>| ProblemType { id: id.into(), label: label.into(), description: desc.into(), dims, layout };
        ModuleSpec {
            id: "alm".into(),
            winqsb: "Nuevo".into(),
            name: "Almacenes".into(),
            description: "Paletizado y bodega en bloque (con plano) y dimensionado del almacén: propio o rentado".into(),
            file_ext: "ALM".into(),
            problem_types: vec![
                pt(
                    "paletizado",
                    "Paletizado y bodega en bloque",
                    "Cajas por estiba según altura, peso y resistencia; estibas por lote y área mínima de la bodega con pasillos",
                    vec![
                        Widget::params(
                            Some("Caja (cm y kg)"),
                            vec![
                                Field::number("box_length", "Largo de la caja (cm)"),
                                Field::number("box_width", "Ancho de la caja (cm)"),
                                Field::number("box_height", "Alto de la caja (cm)"),
                                opt("box_weight", "Peso de la caja llena (kg)"),
                                opt("units_per_box", "o unidades por caja"),
                                opt("unit_weight", "y peso de cada unidad (kg)"),
                                opt("empty_box_weight", "y peso de la caja vacía (kg)"),
                            ],
                        ),
                        Widget::params(
                            Some("Estiba y restricciones"),
                            vec![
                                Field::number("pallet_length", "Largo de la estiba (cm)").default(120.0),
                                Field::number("pallet_width", "Ancho de la estiba (cm)").default(100.0),
                                Field::number("pallet_height", "Alto de la estiba (cm)").default(14.5),
                                Field::number("pallet_weight", "Peso de la estiba (kg)").default(30.0),
                                opt("max_load", "Capacidad del montacargas (kg, con la estiba)"),
                                opt("max_height", "Altura máxima de la carga (cm, con la estiba)"),
                                opt("bottom_layer_max", "Peso que resiste el primer tendido (kg)"),
                            ],
                        ),
                        Widget::params(
                            Some("Lotes y bodega en bloque (opcional)"),
                            vec![
                                opt("lot_units", "Unidades por lote"),
                                opt("rest_days", "Días de reposo de cada lote"),
                                opt("spaces", "o número de espacios"),
                                opt("forklift_length", "Largo total del montacargas (m)"),
                                opt("aisle_clearance", "Holgura del pasillo (m)"),
                                opt("aisle_width", "o ancho del pasillo (m)"),
                                Field::number("rows_per_lot", "Filas de estibas por lote").default(2.0),
                                Field::number("stack_levels", "Estibas apiladas").default(1.0),
                            ],
                        ),
                    ],
                    vec![],
                ),
                pt(
                    "dimensionado",
                    "Dimensionado del almacén (propio o rentado)",
                    "Espacio requerido por período y tamaño del almacén propio de menor costo total",
                    vec![
                        Widget::params(
                            Some("Espacio"),
                            vec![
                                Field::text("demand_unit", "Unidad de la demanda").default("unidades"),
                                opt("units_per_handling", "Unidades por unidad de manejo (caja)"),
                                Field::number("turnover", "Rotación por período").default(1.0).help("Inventario = demanda / rotación"),
                                Field::choice("space_mode", "Cómo se calcula el espacio", &[("area", "Área por unidad almacenada"), ("estibas", "Con estibas")]),
                                opt("area_per_unit", "Área por unidad almacenada").show_if("space_mode", &["area"]),
                                opt("units_per_pallet", "Unidades de manejo por estiba").show_if("space_mode", &["estibas"]),
                                opt("pallet_area", "Área de una estiba").show_if("space_mode", &["estibas"]),
                                Field::number("aisle_share", "% del espacio para pasillos").default(0.0),
                                Field::number("usable_share", "% utilizable (holgura por variabilidad)").default(100.0),
                                Field::text("area_unit", "Unidad de área").default("m²"),
                            ],
                        ),
                        Widget::params(
                            Some("Costos"),
                            vec![
                                Field::number("investment_per_area", "Inversión por unidad de área"),
                                opt("amortization_years", "Amortización en años (sin interés)"),
                                opt("interest_rate", "o tasa de interés por período (%)"),
                                opt("amortization_periods", "y número de períodos"),
                                Field::number("fixed_cost_year", "Costo fijo de operación por área y año"),
                                Field::number("fixed_cost_period", "Costo fijo de operación por área y período"),
                                Field::number("variable_cost", "Costo variable propio por unidad de demanda"),
                                Field::number("rent_storage", "Renta: almacenaje por unidad en inventario y período"),
                                Field::number("rent_handling", "Renta: manejo por unidad de demanda"),
                                opt("periods_per_year", "Períodos por año").help("Vacío = los períodos de la tabla"),
                            ],
                        ),
                        Widget::records(Some("Demanda por período"), "periods", vec![Field::text("name", "Período"), opt("demand", "Demanda")], Some("Añadir período")),
                        Widget::records(Some("Alternativas a evaluar (área propia)"), "alternatives", vec![opt("area", "Área")], Some("Añadir alternativa")),
                    ],
                    vec![Field::integer("periods", "Períodos", 12)],
                ),
            ],
            options: vec![Field::boolean("steps", "Mostrar paso a paso", true)],
        }
    }

    fn template(&self, pt: &str, dims: &Value) -> Result<AlmModel, ModError> {
        let t: AlmType = serde_json::from_value(Value::from(pt)).map_err(|_| ModError::Invalid(format!("tipo de problema desconocido: {pt}")))?;
        let mut m: AlmModel = serde_json::from_value(json!({"name": "Almacén nuevo", "problem_type": t})).unwrap();
        if t == AlmType::Dimensionado {
            let n = dim(dims, "periods", 12, 1, 120)?;
            m.periods = (1..=n).map(|i| Period { name: i.to_string(), demand: Some(0.0) }).collect();
            m.space_mode = "area".into();
            m.area_unit = "m²".into();
        } else {
            m.max_load = Some(1000.0);
        }
        Ok(m)
    }

    fn solve(&self, m: &AlmModel, o: &AlmOptions) -> Result<Report, ModError> {
        let mut r = match m.problem_type {
            AlmType::Paletizado => paletizado(m, o)?,
            AlmType::Dimensionado => dimensionado(m, o)?,
        };
        if r.problem.is_empty() {
            r.problem = m.name.clone();
        }
        Ok(r)
    }

    fn read(&self, bytes: &[u8]) -> Result<AlmModel, ModError> {
        let s = String::from_utf8_lossy(bytes);
        let s = s.trim_start_matches('\u{feff}');
        let (head, rest) = s.split_once('\n').unwrap_or((s, ""));
        if !head.trim().to_ascii_uppercase().starts_with("IAQSB-ALM") {
            return Err(ModError::Format("no es un archivo de almacenes de IAQSB".into()));
        }
        serde_json::from_str(rest).map_err(|e| ModError::Format(e.to_string()))
    }

    fn write(&self, m: &AlmModel) -> Result<Vec<u8>, ModError> {
        Ok(format!("IAQSB-ALM 1\n{}\n", serde_json::to_string_pretty(m).unwrap()).into_bytes())
    }

    fn examples(&self) -> Vec<(&'static str, AlmModel)> {
        let ex = |s: &str| serde_json::from_str::<AlmModel>(s).unwrap();
        vec![
            ("envasadora", ex(include_str!("ejemplos/envasadora.json"))),
            ("taller3_estiba", ex(include_str!("ejemplos/taller3_estiba.json"))),
            ("compania_quimica", ex(include_str!("ejemplos/quimica.json"))),
            ("taller3_dimensionado", ex(include_str!("ejemplos/taller3_dimensionado.json"))),
        ]
    }
}
