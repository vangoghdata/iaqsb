//! MODO · Selección del modo de transporte (no existe en WinQSB).
//!
//! Compara medios de transporte por el costo total de la cadena, no solo por
//! la tarifa: un medio barato y lento obliga a tener más inventario en la
//! planta, en tránsito y en el centro de distribución (Ballou, 2004, caps. 6–7;
//! diapositivas de transporte de Sarache). Para
//! cada medio:
//!
//! - Almacenaje en la planta: i · C · (inventario promedio en la planta)
//! - Almacenaje en tránsito: i · C · D · T / 365
//! - Almacenaje en el CD:    i · (C + R) · (inventario promedio en el CD)
//! - Transporte:             D · R
//!
//! con D la demanda anual, C el costo del producto, i el costo anual de
//! mantener inventario, R el flete por unidad y T los días en tránsito.

use iaqsb_core::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
pub struct TransMode {
    pub name: String,
    /// Flete por unidad (R).
    #[serde(default)]
    pub rate: f64,
    /// Días en tránsito (T).
    #[serde(default)]
    pub transit_days: f64,
    /// Despachos al año (opcional: sirve para calcular los inventarios si no se dan).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shipments: Option<f64>,
    /// Inventario promedio en la planta; vacío = la mitad de cada despacho.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plant_inventory: Option<f64>,
    /// Inventario promedio en el centro de distribución; vacío = la mitad de cada despacho.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dc_inventory: Option<f64>,
}

fn days() -> f64 {
    365.0
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ModeModel {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub problem_type: String,
    /// Demanda anual (D).
    pub demand: f64,
    /// Costo del producto en la planta (C).
    pub unit_cost: f64,
    /// Costo anual de mantener inventario, en % del valor (i).
    pub holding_rate: f64,
    /// Días del año para el tránsito.
    #[serde(default = "days")]
    pub days_per_year: f64,
    pub modes: Vec<TransMode>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct ModeOptions {
    pub steps: bool,
    /// Gráfico del costo total según la demanda anual.
    pub sensitivity: bool,
}

impl Default for ModeOptions {
    fn default() -> Self {
        ModeOptions { steps: true, sensitivity: true }
    }
}

struct Costs {
    ip: f64,
    icd: f64,
    plant: f64,
    transit: f64,
    dc: f64,
    transport: f64,
    note_ip: String,
    note_icd: String,
}

impl Costs {
    fn holding(&self) -> f64 {
        self.plant + self.transit + self.dc
    }
    fn total(&self) -> f64 {
        self.holding() + self.transport
    }
}

fn costs(m: &ModeModel, t: &TransMode, d: f64) -> Result<Costs, ModError> {
    let i = m.holding_rate / 100.0;
    let c = m.unit_cost;
    let half = t.shipments.filter(|s| *s > 0.0).map(|s| d / s / 2.0);
    let (ip, note_ip) = match (t.plant_inventory, half) {
        (Some(v), _) => (v, "dato".to_string()),
        (None, Some(h)) => (h, format!("D / (2 × {}) despachos", fmt(t.shipments.unwrap()))),
        (None, None) => return Err(ModError::Invalid(format!("{}: falta el inventario en la planta o el número de despachos al año", t.name))),
    };
    let (icd, note_icd) = match (t.dc_inventory, half) {
        (Some(v), _) => (v, "dato".to_string()),
        (None, Some(h)) => (h, format!("D / (2 × {}) despachos", fmt(t.shipments.unwrap()))),
        (None, None) => return Err(ModError::Invalid(format!("{}: falta el inventario en el CD o el número de despachos al año", t.name))),
    };
    let r2 = |x: f64| (x * 100.0).round() / 100.0;
    Ok(Costs {
        ip,
        icd,
        plant: r2(i * c * ip),
        transit: r2(i * c * d * t.transit_days / m.days_per_year),
        dc: r2(i * (c + t.rate) * icd),
        transport: r2(d * t.rate),
        note_ip,
        note_icd,
    })
}

pub struct Modo;

impl TypedModule for Modo {
    type Model = ModeModel;
    type Options = ModeOptions;

    fn file_tag(&self) -> &'static str {
        "IAQSB-MODO"
    }

    fn spec(&self) -> ModuleSpec {
        ModuleSpec {
            id: "modo".into(),
            winqsb: "Nuevo".into(),
            name: "Selección del modo de transporte".into(),
            description: "Compara tren, camión, avión, barco u otros medios por el costo total: flete más el inventario en la planta, en tránsito y en el centro de distribución".into(),
            file_ext: "MOD".into(),
            problem_types: vec![ProblemType {
                id: "tradeoff".into(),
                label: "Trade-off transporte-inventarios".into(),
                description: "Costo total de cada medio de transporte (Ballou)".into(),
                dims: vec![Field::integer("modes", "Medios de transporte", 3)],
                layout: vec![
                    Widget::params(
                        Some("Producto y demanda"),
                        vec![
                            Field::number("demand", "Demanda anual (D)"),
                            Field::number("unit_cost", "Costo del producto en la planta (C)"),
                            Field::number("holding_rate", "Costo de mantener inventario (i, % anual)").help("30 = 30 % del valor del producto al año"),
                            Field::number("days_per_year", "Días del año").default(365.0),
                        ],
                    ),
                    Widget::records(
                        Some("Medios de transporte"),
                        "modes",
                        vec![
                            Field::text("name", "Medio"),
                            Field::number("rate", "Flete por unidad (R)"),
                            Field::number("transit_days", "Días en tránsito (T)"),
                            Field::number("shipments", "Despachos al año").optional(),
                            Field::number("plant_inventory", "Inventario en la planta").optional().help("Vacío = la mitad de cada despacho"),
                            Field::number("dc_inventory", "Inventario en el CD").optional().help("Vacío = la mitad de cada despacho"),
                        ],
                        Some("Añadir medio"),
                    ),
                ],
            }],
            options: vec![
                Field::boolean("steps", "Mostrar paso a paso", true),
                Field::boolean("sensitivity", "Costo según la demanda anual", true),
            ],
        }
    }

    fn template(&self, _pt: &str, dims: &Value) -> Result<ModeModel, ModError> {
        let n = dim(dims, "modes", 3, 1, 20)?;
        let base = [("Tren", 0.1, 21.0, 10.0), ("Camión", 0.2, 5.0, 20.0), ("Avión", 1.4, 2.0, 40.0), ("Barco", 0.05, 30.0, 6.0)];
        let modes = (0..n)
            .map(|k| {
                let (nm, r, t, s) = base.get(k).copied().unwrap_or(("Medio", 0.0, 0.0, 12.0));
                TransMode { name: if k < base.len() { nm.into() } else { format!("Medio{}", k + 1) }, rate: r, transit_days: t, shipments: Some(s), plant_inventory: None, dc_inventory: None }
            })
            .collect();
        Ok(ModeModel { name: "Selección del modo nueva".into(), problem_type: "tradeoff".into(), demand: 100000.0, unit_cost: 10.0, holding_rate: 25.0, days_per_year: 365.0, modes })
    }

    fn solve(&self, m: &ModeModel, o: &ModeOptions) -> Result<Report, ModError> {
        check_names("medios", &m.modes.iter().map(|x| &x.name).collect::<Vec<_>>())?;
        if m.modes.is_empty() {
            return invalid("hace falta al menos un medio de transporte");
        }
        if m.demand < 0.0 || m.unit_cost < 0.0 || m.holding_rate < 0.0 || !(m.days_per_year > 0.0) {
            return invalid("la demanda, el costo, el porcentaje de mantener y los días deben ser positivos");
        }
        for t in &m.modes {
            if t.rate < 0.0 || t.transit_days < 0.0 || t.plant_inventory.is_some_and(|v| v < 0.0) || t.dc_inventory.is_some_and(|v| v < 0.0) {
                return invalid(format!("{}: los datos no pueden ser negativos", t.name));
            }
        }
        let cs: Vec<Costs> = m.modes.iter().map(|t| costs(m, t, m.demand)).collect::<Result<_, _>>()?;
        let best = (0..cs.len()).min_by(|&a, &b| cs[a].total().total_cmp(&cs[b].total())).unwrap();
        let bt = cs[best].total();
        let i = m.holding_rate / 100.0;

        // tabla como en la clase: filas = costos, columnas = medios
        let mut heads = vec!["Costo".to_string(), "Fórmula".to_string()];
        heads.extend(m.modes.iter().map(|t| t.name.clone()));
        let mut tab = Table::new(Some("Costo total por medio"), &heads).left(0).left(1);
        let rows: [(&str, &str, fn(&Costs) -> f64); 5] = [
            ("Almacenaje en la planta", "i × C × inventario", |c| c.plant),
            ("Almacenaje en tránsito", "i × C × D × T / 365", |c| c.transit),
            ("Almacenaje en el CD", "i × (C + R) × inventario", |c| c.dc),
            ("Transporte", "D × R", |c| c.transport),
            ("Total", "", |c| c.total()),
        ];
        for (k, (label, f, g)) in rows.iter().enumerate() {
            let mut r = vec![Cell::from(*label), Cell::from(*f)];
            r.extend(cs.iter().map(|c| Cell::from(clean(g(c)))));
            if k == 4 {
                tab.row_tone(r, Tone::Total);
            } else {
                tab.row(r);
            }
        }
        tab.mark(4, 2 + best, Tone::Accent);

        let mut rank: Vec<usize> = (0..cs.len()).collect();
        rank.sort_by(|&a, &b| cs[a].total().total_cmp(&cs[b].total()));
        let mut mets = vec![metric_main("Medio recomendado", m.modes[best].name.clone()), metric("Costo total anual", clean(bt))];
        if rank.len() > 1 {
            let second = rank[1];
            mets.push(metric(format!("Ahorro frente a {}", m.modes[second].name), clean(cs[second].total() - bt)));
        }

        let cats: Vec<String> = m.modes.iter().map(|t| t.name.clone()).collect();
        let pts = |g: &dyn Fn(&Costs) -> f64| cs.iter().enumerate().map(|(k, c)| [k as f64, clean(g(c))]).collect::<Vec<_>>();
        let mut stack = Chart::new(Some("Composición del costo de cada medio"), "Medio", "Costo anual").stacked();
        stack.categories = Some(cats.clone());
        stack.series.push(Series::new("Almacenaje en la planta", SeriesKind::Bar, pts(&|c| c.plant)));
        stack.series.push(Series::new("Almacenaje en tránsito", SeriesKind::Bar, pts(&|c| c.transit)));
        stack.series.push(Series::new("Almacenaje en el CD", SeriesKind::Bar, pts(&|c| c.dc)));
        stack.series.push(Series::new("Transporte", SeriesKind::Bar, pts(&|c| c.transport)));
        let mut trade = Chart::new(Some("Trade-off transporte vs. inventarios"), "Medio", "Costo anual");
        trade.categories = Some(cats.clone());
        trade.y_zero = true;
        trade.series.push(Series::new("Almacenaje (planta + tránsito + CD)", SeriesKind::Line, pts(&|c| c.holding())));
        trade.series.push(Series::new("Transporte", SeriesKind::Line, pts(&|c| c.transport)).tone(Tone::Bad));
        trade.series.push(Series::new("Total", SeriesKind::Line, pts(&|c| c.total())).tone(Tone::Accent).dashed());

        let mut rep = Report::new(
            "modo",
            &m.name,
            Status::Optimal,
            format!(
                "Conviene {}: costo total de {} al año{}.",
                m.modes[best].name,
                fmt(clean(bt)),
                if rank.len() > 1 { format!(", {} menos que {}", fmt(clean(cs[rank[1]].total() - bt)), m.modes[rank[1]].name) } else { String::new() }
            ),
        )
        .section(
            "solucion",
            "Solución",
            vec![
                metrics(mets),
                text(format!(
                    "El flete más barato no siempre gana: un medio lento obliga a tener más inventario en tránsito y en las bodegas. Con una demanda de {} unidades al año, un producto de {} y un costo de mantener del {} % anual, el menor costo total es el de {}.",
                    fmt(m.demand),
                    fmt(m.unit_cost),
                    fmt(m.holding_rate),
                    m.modes[best].name
                )),
                tab.block(),
            ],
        )
        .section("graficos", "Gráficos", vec![stack.block(), trade.block()]);

        // costo total según la demanda (los inventarios dados quedan fijos)
        if o.sensitivity && m.demand > 0.0 {
            let mut ch = Chart::new(Some("Costo total según la demanda anual"), "Demanda anual", "Costo total");
            ch.y_zero = true;
            let k = 40;
            let lo = 0.0;
            let hi = m.demand * 2.0;
            let curve = |t: &TransMode| -> Vec<[f64; 2]> {
                (0..=k)
                    .map(|s| {
                        let d = lo + (hi - lo) * s as f64 / k as f64;
                        // con despachos al año y sin inventarios dados, el inventario crece con la demanda
                        [d, costs(m, t, d).map(|c| c.total()).unwrap_or(f64::NAN)]
                    })
                    .collect()
            };
            for (k, t) in m.modes.iter().enumerate() {
                let mut s = Series::new(t.name.clone(), SeriesKind::Line, curve(t));
                if k == best {
                    s = s.tone(Tone::Accent);
                }
                ch.series.push(s);
            }
            ch = ch.vline(m.demand, "Demanda actual", Tone::Muted);
            // puntos de indiferencia: donde dos medios cuestan lo mismo
            let mut cross = Table::new(Some("Demanda en la que dos medios cuestan lo mismo"), &["Medios", "Demanda", "Por debajo conviene", "Por encima conviene"]).left(0).left(2).left(3);
            let mut found = 0;
            for a in 0..m.modes.len() {
                for b in a + 1..m.modes.len() {
                    let f = |d: f64| costs(m, &m.modes[a], d).map(|c| c.total()).unwrap_or(f64::NAN) - costs(m, &m.modes[b], d).map(|c| c.total()).unwrap_or(f64::NAN);
                    // los costos son lineales en D: se busca el cero en [0, 3D]
                    let (x0, x1) = (0.0, m.demand * 3.0);
                    let (f0, f1) = (f(x0), f(x1));
                    if f0.is_finite() && f1.is_finite() && f0 * f1 < 0.0 {
                        let mut lo2 = x0;
                        let mut hi2 = x1;
                        for _ in 0..200 {
                            let mid = (lo2 + hi2) / 2.0;
                            if f(lo2) * f(mid) <= 0.0 {
                                hi2 = mid;
                            } else {
                                lo2 = mid;
                            }
                        }
                        let d = (lo2 + hi2) / 2.0;
                        let (below, above) = if f0 < 0.0 { (a, b) } else { (b, a) };
                        cross.row(cells![format!("{} y {}", m.modes[a].name, m.modes[b].name), d.round(), m.modes[below].name.clone(), m.modes[above].name.clone()]);
                        if d <= hi {
                            ch = ch.vline(d, format!("{} = {}", m.modes[a].name, m.modes[b].name), Tone::Highlight);
                        }
                        found += 1;
                    }
                }
            }
            let mut blocks = vec![ch.block()];
            if found > 0 {
                blocks.push(cross.block());
            } else {
                blocks.push(text("Entre 0 y el triple de la demanda actual ningún par de medios cambia de orden: el recomendado sigue siéndolo."));
            }
            blocks.push(text("Los inventarios escritos como dato se dejan fijos; los que se calculan con los despachos al año crecen con la demanda."));
            rep = rep.section("sensibilidad", "Sensibilidad", blocks);
        }

        if o.steps {
            let mut st = vec![];
            for (t, c) in m.modes.iter().zip(&cs) {
                let mut tb = Table::new(None, &["Costo", "Cálculo", "Valor"]).left(0).left(1);
                tb.row(cells!["Almacenaje en la planta", format!("{} × {} × {} ({})", fmt(i), fmt(m.unit_cost), fmt(c.ip), c.note_ip), clean(c.plant)]);
                tb.row(cells![
                    "Almacenaje en tránsito",
                    format!("{} × {} × {} × {} / {}", fmt(i), fmt(m.unit_cost), fmt(m.demand), fmt(t.transit_days), fmt(m.days_per_year)),
                    clean(c.transit)
                ]);
                tb.row(cells!["Almacenaje en el CD", format!("{} × ({} + {}) × {} ({})", fmt(i), fmt(m.unit_cost), fmt(t.rate), fmt(c.icd), c.note_icd), clean(c.dc)]);
                tb.row(cells!["Transporte", format!("{} × {}", fmt(m.demand), fmt(t.rate)), clean(c.transport)]);
                tb.row_tone(cells!["Total", "", clean(c.total())], Tone::Total);
                st.push(step(
                    t.name.clone(),
                    format!(
                        "Flete de {} por unidad y {} días en tránsito. En el CD el producto vale C + R = {}: ya lleva el flete encima.",
                        fmt(t.rate),
                        fmt(t.transit_days),
                        fmt(m.unit_cost + t.rate)
                    ),
                    vec![tb.block()],
                ));
            }
            st.push(step(
                "Comparar",
                format!("Se elige el medio con el menor costo total: {} ({}).", m.modes[best].name, fmt(clean(bt))),
                vec![],
            ));
            rep = rep.section("pasos", "Paso a paso", vec![steps(st)]);
        }

        let mut info = Table::new(Some(&format!("Ventajas y desventajas de cada modo ({})", iaqsb_core::biblio::c("sarache2026transporte"))), &["Modo", "Ventajas", "Desventajas"]).left(0).left(1).left(2);
        for (a, b, c) in [
            ("Carretero", "Flexible en rutas, horarios y tipo de carga; entrega puerta a puerta; se combina con todos los medios", "Menor capacidad; más caro que el tren y el barco; más accidentes y robos; depende de la infraestructura"),
            ("Ferroviario", "Más capacidad que el camión y el avión; economías de escala y de distancia; bueno para sistemas intermodales; más fiable y rápido que el barco", "Limitado donde hay poca infraestructura; menos flexible que el camión; más caro que el barco"),
            ("Por ducto", "Rápido frente al camión y al tren; muy fiable y fácil de controlar; bajo costo variable", "Pocos productos; necesita otros medios; riesgos ambientales"),
            ("Marítimo y fluvial", "Gran capacidad y flexibilidad; máximas economías de escala; cobertura mundial", "Lento; depende de los puertos; embalaje especial; servicio menos frecuente; más riesgo de contaminar la carga"),
            ("Aéreo", "El más rápido; menos gasto en empaque; más seguridad para la carga; gran alcance", "El más caro; capacidad limitada; regulación severa"),
        ] {
            info.row(cells![a, b, c]);
        }
        rep = rep.section("modos", "Modos", vec![info.block()]);

        let data: Vec<Value> = m
            .modes
            .iter()
            .zip(&cs)
            .map(|(t, c)| json!({"mode": t.name, "plant": clean(c.plant), "transit": clean(c.transit), "dc": clean(c.dc), "transport": clean(c.transport), "total": clean(c.total())}))
            .collect();
        let rep = rep
            .cite("ballou2004", "Selección del servicio de transporte por costo total: flete, inventario en planta, en tránsito y en el centro de distribución")
            .cite("sarache2026transporte", "Planteamiento, fórmulas y ejemplo de la clase (trade-off transporte-inventarios, diapositivas 37–40)")
            .cite("sarache2026transporte", "Ventajas y desventajas de cada modo (diapositivas 9–36)");
        Ok(rep.data(json!({"best": m.modes[best].name, "total": clean(bt), "modes": data})))
    }

    fn read(&self, bytes: &[u8]) -> Result<ModeModel, ModError> {
        let s = String::from_utf8_lossy(bytes);
        let s = s.trim_start_matches('\u{feff}');
        let (head, rest) = s.split_once('\n').unwrap_or((s, ""));
        if !head.trim().to_ascii_uppercase().starts_with("IAQSB-MODO") {
            return Err(ModError::Format("no es un archivo de selección del modo de IAQSB".into()));
        }
        serde_json::from_str(rest).map_err(|e| ModError::Format(e.to_string()))
    }

    fn write(&self, m: &ModeModel) -> Result<Vec<u8>, ModError> {
        Ok(format!("IAQSB-MODO 1\n{}\n", serde_json::to_string_pretty(m).unwrap()).into_bytes())
    }

    fn examples(&self) -> Vec<(&'static str, ModeModel)> {
        let ex = |s: &str| serde_json::from_str::<ModeModel>(s).unwrap();
        vec![("tren_camion_avion", ex(include_str!("ejemplos/clase.json")))]
    }
}
