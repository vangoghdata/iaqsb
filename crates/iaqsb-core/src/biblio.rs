//! Fuentes de IAQSB: bibliografía central (APA 7.ª ed. en español), citas en
//! los informes y origen de los datos de cada ejemplo.
//!
//! Los datos están en `fuentes/bibliografia.json` y `fuentes/ejemplos.json`
//! para que se puedan revisar y corregir sin tocar el código. Cada referencia
//! tiene un estado: `verificada` (comprobada en el DOI, el editor o un
//! catálogo) o `por_verificar` (con lo que falta en `pendiente`).

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::OnceLock;

use crate::i18n::Lang;

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema, Default)]
#[serde(default)]
pub struct Reference {
    pub id: String,
    /// articulo, libro, capitulo, informe, tesis, norma, diapositivas, software, web
    pub tipo: String,
    /// Autores en forma APA: "Apellido, I. I."
    pub autores: Vec<String>,
    /// Los autores son editores del libro (Eds.).
    pub editores_como_autores: bool,
    pub anio: String,
    /// Letra para distinguir obras del mismo autor y año (2026a, 2026b...).
    pub sufijo: String,
    pub titulo: String,
    pub revista: String,
    pub volumen: String,
    pub numero: String,
    pub paginas: String,
    pub edicion: String,
    pub editorial: String,
    pub serie: String,
    /// Software: versión («Versión 0.35.0»).
    pub version: String,
    /// Capítulo: libro que lo contiene y sus editores ("T. C. Koopmans").
    pub libro: String,
    pub editores_libro: Vec<String>,
    /// Descripción entre corchetes (p. ej. "Diapositivas de clase").
    pub descripcion: String,
    pub doi: String,
    pub url: String,
    pub isbn: String,
    pub estado: String,
    pub pendiente: String,
    pub nota: String,
    pub tema: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema, Default)]
#[serde(default)]
pub struct ExampleRef {
    pub id: String,
    pub donde: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema, Default)]
#[serde(default)]
pub struct ExampleOrigin {
    pub origen: String,
    pub refs: Vec<ExampleRef>,
    pub resultado: String,
    pub nota: String,
    pub estado: String,
    pub pendiente: String,
}

#[derive(Deserialize)]
struct BibFile {
    referencias: Vec<Reference>,
}
#[derive(Deserialize)]
struct ExFile {
    ejemplos: BTreeMap<String, ExampleOrigin>,
}

/// Regla de `fuentes/metodos.json`: qué fuentes cita un módulo y cuándo.
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema, Default)]
#[serde(default)]
pub struct MethodRule {
    pub modulo: String,
    /// Condiciones (todas deben cumplirse): clave → valores aceptados.
    pub si: BTreeMap<String, Vec<String>>,
    pub refs: Vec<MethodRef>,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema, Default)]
#[serde(default)]
pub struct MethodRef {
    pub id: String,
    pub para: String,
}

#[derive(Deserialize)]
struct MethodFile {
    reglas: Vec<MethodRule>,
}

static METHODS: OnceLock<Vec<MethodRule>> = OnceLock::new();

/// Todas las reglas de citas de métodos.
pub fn method_rules() -> &'static [MethodRule] {
    METHODS.get_or_init(|| {
        let f: MethodFile = serde_json::from_str(include_str!("../fuentes/metodos.json")).expect("metodos.json mal formado");
        f.reglas
    })
}

/// Fuentes de los métodos que usa un módulo. `value(clave)` da el valor de
/// una condición como texto (problem_type, una opción, un dato del resultado o
/// «seccion»); puede dar varios (p. ej. las secciones del informe).
pub fn method_citations(module: &str, value: impl Fn(&str) -> Vec<String>) -> Vec<(&'static str, &'static str)> {
    let mut out = vec![];
    for r in method_rules().iter().filter(|r| r.modulo == module) {
        let ok = r.si.iter().all(|(k, vals)| {
            let have = value(k);
            have.iter().any(|h| {
                vals.iter().any(|v| match v.as_str() {
                    "*" => !h.is_empty() && h != "false" && h.parse::<f64>().map(|x| x != 0.0).unwrap_or(true),
                    _ if v.starts_with('!') => h != &v[1..],
                    _ => v == h,
                })
            })
        });
        if ok {
            out.extend(r.refs.iter().map(|x| (x.id.as_str(), x.para.as_str())));
        }
    }
    out
}

static BIB: OnceLock<Vec<Reference>> = OnceLock::new();
static EX: OnceLock<BTreeMap<String, ExampleOrigin>> = OnceLock::new();

/// Todas las referencias, en el orden del archivo.
pub fn all() -> &'static [Reference] {
    BIB.get_or_init(|| {
        let f: BibFile = serde_json::from_str(include_str!("../fuentes/bibliografia.json")).expect("bibliografia.json mal formado");
        let mut v = f.referencias;
        // APA: obras del mismo autor y año llevan a, b, c… según el orden alfabético del título
        let key = |r: &Reference| (r.autores.join("; "), r.anio.clone());
        let mut groups: std::collections::BTreeMap<(String, String), Vec<usize>> = Default::default();
        for (i, r) in v.iter().enumerate() {
            groups.entry(key(r)).or_default().push(i);
        }
        for (_, mut ix) in groups {
            if ix.len() < 2 {
                if let Some(&i) = ix.first() {
                    v[i].sufijo.clear();
                }
                continue;
            }
            ix.sort_by_key(|&i| v[i].titulo.to_lowercase());
            for (k, &i) in ix.iter().enumerate() {
                v[i].sufijo = ((b'a' + k as u8) as char).to_string();
            }
        }
        v
    })
}

pub fn get(id: &str) -> Option<&'static Reference> {
    all().iter().find(|r| r.id == id)
}

/// Origen de los ejemplos, por clave "módulo/ejemplo".
pub fn example_origins() -> &'static BTreeMap<String, ExampleOrigin> {
    EX.get_or_init(|| {
        let f: ExFile = serde_json::from_str(include_str!("../fuentes/ejemplos.json")).expect("ejemplos.json mal formado");
        f.ejemplos
    })
}

pub fn verified(r: &Reference) -> bool {
    r.estado == "verificada"
}

fn family(a: &str) -> &str {
    a.split(',').next().unwrap_or(a).trim()
}

fn year_l(r: &Reference, lang: Lang) -> String {
    let nd = match lang {
        Lang::En => "n.d.",
        Lang::Pt => "s.d.",
        Lang::Es => "s. f.",
    };
    let y = if r.anio.is_empty() { nd.to_string() } else { r.anio.clone() };
    format!("{y}{}", r.sufijo)
}

/// Une nombres al estilo APA en español: "A", "A y B", "A, B y C".
/// En inglés (APA 7): "A, & B", "A, B, & C".
fn join_l(v: &[String], lang: Lang) -> String {
    match (v.len(), lang) {
        (0, _) => String::new(),
        (1, _) => v[0].clone(),
        (n, Lang::Es) => format!("{} y {}", v[..n - 1].join(", "), v[n - 1]),
        // APA 7 en inglés y en portugués: "A, & B", "A, B, & C"
        (n, Lang::En | Lang::Pt) => format!("{}, & {}", v[..n - 1].join(", "), v[n - 1]),
    }
}

/// Cita en el texto, sin paréntesis: "Dijkstra, 1959", "Clarke y Wright, 1964", "Miller et al., 1960".
pub fn cite(r: &Reference) -> String {
    cite_l(r, Lang::Es)
}

/// Cita en el texto en el idioma pedido ("Clarke & Wright, 1964" en inglés).
pub fn cite_l(r: &Reference, lang: Lang) -> String {
    let fam: Vec<String> = r.autores.iter().map(|a| family(a).to_string()).collect();
    let who = match fam.len() {
        0 => r.titulo.clone(),
        1 => fam[0].clone(),
        2 => format!("{} {} {}", fam[0], if lang == Lang::Es { "y" } else { "&" }, fam[1]),
        _ => format!("{} et al.", fam[0]),
    };
    format!("{who}, {}", year_l(r, lang))
}

/// Cita entre paréntesis por id, para escribirla dentro de un texto: "Sarache, 2026d".
pub fn c(id: &str) -> String {
    get(id).map(cite).unwrap_or_else(|| id.to_string())
}

/// Cita narrativa por id: "Miller et al. (1960)".
pub fn cn(id: &str) -> String {
    get(id).map(cite_narrative).unwrap_or_else(|| id.to_string())
}

/// Cita narrativa: "Miller et al. (1960)".
pub fn cite_narrative(r: &Reference) -> String {
    cite_narrative_l(r, Lang::Es)
}

/// Cita narrativa: "Hillier y Lieberman (2015a)" / "Hillier and Lieberman (2015a)".
pub fn cite_narrative_l(r: &Reference, lang: Lang) -> String {
    let c = cite_l(r, lang).replace(" & ", if lang == Lang::Pt { " e " } else { " and " });
    let (who, y) = c.rsplit_once(", ").unwrap_or((&c, ""));
    format!("{who} ({y})")
}

fn end(s: &str) -> String {
    let s = s.trim();
    if s.ends_with('.') || s.ends_with('?') || s.ends_with('!') {
        s.to_string()
    } else {
        format!("{s}.")
    }
}

fn edition(e: &str, lang: Lang) -> String {
    if e.is_empty() || e == "1" {
        String::new()
    } else if e.chars().all(|c| c.is_ascii_digit()) {
        match lang {
            Lang::Es => format!("{e}.ª ed."),
            // APA en portugués: "10. ed."
            Lang::Pt => format!("{e}. ed."),
            Lang::En => {
                let n: u32 = e.parse().unwrap_or(0);
                let suf = match (n % 10, n % 100) {
                    (_, 11..=13) => "th",
                    (1, _) => "st",
                    (2, _) => "nd",
                    (3, _) => "rd",
                    _ => "th",
                };
                format!("{n}{suf} ed.")
            }
        }
    } else {
        crate::i18n::translate(e, lang)
    }
}

/// Enlace preferido: DOI o URL.
pub fn link(r: &Reference) -> Option<String> {
    if !r.doi.is_empty() {
        Some(format!("https://doi.org/{}", r.doi))
    } else if !r.url.is_empty() {
        Some(r.url.clone())
    } else {
        None
    }
}

/// Referencia completa en APA 7 (español). Las cursivas van entre asteriscos
/// (*así*), que la aplicación dibuja en cursiva y la CLI quita.
pub fn apa(r: &Reference) -> String {
    apa_l(r, Lang::Es)
}

/// Referencia APA 7 en el idioma pedido. En inglés cambian los conectores
/// ("&", "In", "ed.", "Version", "n.d.") y la descripción entre corchetes; los
/// títulos se dejan en su idioma original, como pide APA.
pub fn apa_l(r: &Reference, lang: Lang) -> String {
    let mut who = join_l(&r.autores, lang);
    if r.editores_como_autores {
        who = format!("{} ({}).", who, if r.autores.len() > 1 { "Eds." } else { "Ed." });
    }
    let head = if who.is_empty() { String::new() } else { format!("{} ", end(&who)) };
    let mut s = format!("{head}({}). ", year_l(r, lang));
    let ed = edition(&r.edicion, lang);
    match r.tipo.as_str() {
        "articulo" => {
            s.push_str(&end(&r.titulo));
            s.push(' ');
            let mut src = format!("*{}*", r.revista);
            if !r.volumen.is_empty() {
                src.push_str(&format!(", *{}*", r.volumen));
                if !r.numero.is_empty() {
                    src.push_str(&format!("({})", r.numero));
                }
            }
            if !r.paginas.is_empty() {
                src.push_str(&format!(", {}", r.paginas));
            }
            s.push_str(&end(&src));
        }
        "capitulo" => {
            s.push_str(&end(&r.titulo));
            s.push_str(if lang == Lang::Es { " En " } else { " In " });
            if !r.editores_libro.is_empty() {
                s.push_str(&format!("{} ({}), ", join_l(&r.editores_libro, lang), if r.editores_libro.len() > 1 { "Eds." } else { "Ed." }));
            }
            s.push_str(&format!("*{}*", r.libro));
            let mut par = vec![];
            if !ed.is_empty() {
                par.push(ed.clone());
            }
            if !r.paginas.is_empty() {
                par.push(format!("pp. {}", r.paginas));
            }
            if !par.is_empty() {
                s.push_str(&format!(" ({})", par.join(", ")));
            }
            s.push_str(". ");
            s.push_str(&end(&r.editorial));
        }
        _ => {
            // libro, informe, diapositivas, software, web
            let mut t = format!("*{}*", r.titulo.trim_end_matches('.'));
            let mut par = vec![];
            if !ed.is_empty() {
                par.push(ed.clone());
            }
            if r.tipo == "informe" && !r.serie.is_empty() {
                par.push(r.serie.clone());
            }
            if !r.version.is_empty() {
                par.push(format!("{} {}", match lang { Lang::En => "Version", Lang::Pt => "Versão", Lang::Es => "Versión" }, r.version));
            }
            if !par.is_empty() {
                t.push_str(&format!(" ({})", par.join(", ")));
            }
            if !r.descripcion.is_empty() {
                t.push_str(&format!(" [{}]", crate::i18n::translate(&r.descripcion, lang)));
            }
            s.push_str(&end(&t));
            // en una tesis la universidad ya va entre corchetes
            if !r.editorial.is_empty() && r.tipo != "tesis" {
                s.push(' ');
                s.push_str(&end(&r.editorial));
            }
        }
    }
    if let Some(l) = link(r) {
        s.push(' ');
        s.push_str(&l);
    }
    s
}

/// Quita las marcas de cursiva (para texto plano).
pub fn plain(s: &str) -> String {
    s.replace('*', "")
}

fn bib_escape(s: &str) -> String {
    s.replace('&', "\\&").replace('–', "--")
}

/// Entrada BibTeX.
pub fn bibtex(r: &Reference) -> String {
    let kind = match r.tipo.as_str() {
        "articulo" => "article",
        "libro" => "book",
        "capitulo" => "incollection",
        "informe" => "techreport",
        "tesis" => "mastersthesis",
        _ => "misc",
    };
    // BibTeX escribe los sufijos como "Apellido, Jr., Nombre"
    let bib_name = |a: &String| {
        let p: Vec<&str> = a.split(',').map(|x| x.trim()).collect();
        if p.len() == 3 { format!("{}, {}, {}", p[0], p[2], p[1]) } else { a.clone() }
    };
    let names = r.autores.iter().map(bib_name).collect::<Vec<_>>().join(" and ");
    let mut f: Vec<(&str, String)> = vec![];
    f.push((if r.editores_como_autores { "editor" } else { "author" }, names));
    f.push(("title", format!("{{{}}}", r.titulo)));
    f.push(("year", r.anio.clone()));
    let opt = |f: &mut Vec<(&str, String)>, k: &'static str, v: &str| {
        if !v.is_empty() {
            f.push((k, v.to_string()));
        }
    };
    opt(&mut f, "journal", &r.revista);
    opt(&mut f, "booktitle", &r.libro);
    if !r.editores_libro.is_empty() {
        f.push(("editor", r.editores_libro.join(" and ")));
    }
    opt(&mut f, "volume", &r.volumen);
    opt(&mut f, "number", if r.tipo == "informe" { &r.serie } else { &r.numero });
    opt(&mut f, "pages", &r.paginas);
    opt(&mut f, "edition", &r.edicion);
    let org = match r.tipo.as_str() {
        "informe" => "institution",
        "tesis" => "school",
        _ => "publisher",
    };
    opt(&mut f, org, &r.editorial);
    opt(&mut f, "version", &r.version);
    if r.tipo != "tesis" {
        opt(&mut f, "howpublished", &r.descripcion);
    }
    opt(&mut f, "isbn", &r.isbn);
    opt(&mut f, "doi", &r.doi);
    opt(&mut f, "url", &r.url);
    if !verified(r) {
        f.push(("note", format!("Por verificar: {}", r.pendiente)));
    }
    let body: Vec<String> = f.iter().map(|(k, v)| format!("  {k} = {{{}}}", bib_escape(v))).collect();
    format!("@{kind}{{{},\n{}\n}}\n", r.id, body.join(",\n"))
}

/// Toda la bibliografía en BibTeX.
pub fn bibtex_all() -> String {
    all().iter().map(bibtex).collect::<Vec<_>>().join("\n")
}

/// Bibliografía ordenada como en una lista APA (por autor y año).
pub fn sorted() -> Vec<&'static Reference> {
    let mut v: Vec<&Reference> = all().iter().collect();
    v.sort_by_key(|r| (plain(&apa(r)).to_lowercase(), r.id.clone()));
    v
}

// ---------------------------------------------------------------- en los informes

/// Una fuente citada en un informe, con para qué se usó.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct Citation {
    pub id: String,
    /// Cita en el texto: "Miller et al., 1960".
    pub cite: String,
    /// Referencia completa en APA 7 (cursivas entre asteriscos).
    pub apa: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub link: Option<String>,
    pub verified: bool,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub pending: String,
    /// Qué parte del informe se apoya en esta fuente.
    pub used_for: Vec<String>,
    /// Entrada BibTeX, para copiarla a LaTeX o a un gestor de referencias.
    pub bibtex: String,
}

impl Citation {
    pub fn new(id: &str, used_for: &str) -> Citation {
        match get(id) {
            Some(r) => Citation {
                id: id.into(),
                cite: cite(r),
                apa: apa(r),
                link: link(r),
                verified: verified(r),
                pending: r.pendiente.clone(),
                used_for: vec![used_for.into()],
                bibtex: bibtex(r),
            },
            None => Citation {
                id: id.into(),
                cite: id.into(),
                apa: format!("Referencia «{id}» no encontrada en la bibliografía."),
                link: None,
                verified: false,
                pending: "Falta en bibliografia.json".into(),
                used_for: vec![used_for.into()],
                bibtex: String::new(),
            },
        }
    }
}

/// Origen de los datos de un ejemplo, listo para mostrar.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct DataSource {
    /// "módulo/ejemplo"
    pub key: String,
    /// libro, clase, adaptado, winqsb u original
    pub origin: String,
    /// Frase para mostrar: "Diapositivas de clase", "Libro de texto"...
    pub origin_label: String,
    /// Una línea: "Winston (2004), secc. 7.1, ejemplo Powerco".
    pub summary: String,
    pub refs: Vec<Citation>,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub expected: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub note: String,
    pub verified: bool,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub pending: String,
}

pub fn origin_label(o: &str) -> &'static str {
    match o {
        "libro" => "Libro o artículo",
        "clase" => "Diapositivas de clase",
        "adaptado" => "Adaptado de una fuente",
        "winqsb" => "Ejemplo de WinQSB",
        "original" => "Original de IAQSB",
        _ => "Sin especificar",
    }
}

pub fn data_source(module: &str, example: &str) -> Option<DataSource> {
    data_source_l(module, example, Lang::Es)
}

/// Origen de los datos de un ejemplo en el idioma pedido.
pub fn data_source_l(module: &str, example: &str, lang: Lang) -> Option<DataSource> {
    let tr = |s: &str| crate::i18n::translate(s, lang);
    let key = format!("{module}/{example}");
    let e = example_origins().get(&key)?;
    let refs: Vec<Citation> = e.refs.iter().map(|x| Citation::new(&x.id, &tr(&x.donde))).collect();
    let summary = if e.refs.is_empty() {
        if e.nota.is_empty() { tr("Datos escritos para IAQSB") } else { tr(&e.nota) }
    } else {
        e.refs
            .iter()
            .map(|x| {
                let c = get(&x.id).map(|r| cite_narrative_l(r, lang)).unwrap_or_else(|| x.id.clone());
                if x.donde.is_empty() {
                    c
                } else {
                    format!("{c}, {}", tr(&x.donde))
                }
            })
            .collect::<Vec<_>>()
            .join("; ")
    };
    Some(DataSource {
        key,
        origin: e.origen.clone(),
        origin_label: tr(origin_label(&e.origen)),
        summary,
        refs,
        expected: e.resultado.clone(),
        note: if e.refs.is_empty() { String::new() } else { tr(&e.nota) },
        verified: e.estado == "verificada",
        pending: tr(&e.pendiente),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apa_articulo_tres_autores() {
        let r = get("miller1960").unwrap();
        assert_eq!(cite(r), "Miller et al., 1960");
        assert_eq!(
            apa(r),
            "Miller, C. E., Tucker, A. W. y Zemlin, R. A. (1960). Integer programming formulation of traveling salesman problems. *Journal of the ACM*, *7*(4), 326–329. https://doi.org/10.1145/321043.321046"
        );
    }

    #[test]
    fn apa_libro_y_diapositivas() {
        assert_eq!(cite(get("clarkewright1964").unwrap()), "Clarke y Wright, 1964");
        assert_eq!(
            apa(get("taha2012").unwrap()),
            "Taha, H. A. (2012). *Investigación de operaciones* (9.ª ed.). Pearson Educación."
        );
        assert_eq!(
            apa(get("tothvigo2014").unwrap()),
            "Toth, P. y Vigo, D. (Eds.). (2014). *Vehicle routing: Problems, methods, and applications* (2.ª ed.). Society for Industrial and Applied Mathematics. https://doi.org/10.1137/1.9781611973594"
        );
        let s = apa(get("sarache2026ruteo").unwrap());
        assert_eq!(
            s,
            "Sarache, W. A. (2026d). *Problemas de ruteo urbano* [Diapositivas de clase del curso Taller de Producción y Logística, Ingeniería Industrial]. Universidad Nacional de Colombia."
        );
        assert_eq!(
            apa(get("karush1939").unwrap()),
            "Karush, W. (1939). *Minima of functions of several variables with inequalities as side conditions* [Tesis de maestría, University of Chicago]."
        );
        assert!(apa(get("nalgebra").unwrap()).starts_with("Crozet, S. (2026). *nalgebra* (Versión 0.35.0) [Software]. https://"));
        assert!(bibtex(get("karush1939").unwrap()).starts_with("@mastersthesis{karush1939,"));
        assert_eq!(cite_narrative(get("sarache2026ruteo").unwrap()), "Sarache (2026d)");
        assert_eq!(c("sarache2026almacenes"), "Sarache, 2026a");
        assert_eq!(c("sarache_parcial"), "Sarache, s. f.");
    }

    #[test]
    fn reglas_de_metodos_con_referencias_existentes() {
        assert!(method_rules().len() > 50);
        for r in method_rules() {
            for x in &r.refs {
                assert!(get(&x.id).is_some(), "metodos.json ({}) cita {}, que no está en la bibliografía", r.modulo, x.id);
                assert!(!x.para.is_empty());
            }
        }
        let c = method_citations("its", |k| match k {
            "problem_type" => vec!["lot_sizing".into()],
            "method" => vec!["silver_meal".into()],
            _ => vec![],
        });
        let ids: Vec<&str> = c.iter().map(|x| x.0).collect();
        assert!(ids.contains(&"silvermeal1973") && ids.contains(&"wagnerwhitin1958") && !ids.contains(&"harris1913"), "{ids:?}");
    }

    #[test]
    fn ids_unicos_y_ejemplos_con_referencias_existentes() {
        let mut ids: Vec<&str> = all().iter().map(|r| r.id.as_str()).collect();
        let n = ids.len();
        ids.sort();
        ids.dedup();
        assert_eq!(n, ids.len(), "ids repetidos");
        for r in all() {
            assert!(r.estado == "verificada" || r.estado == "por_verificar", "{}: estado {}", r.id, r.estado);
            assert!(r.estado == "verificada" || !r.pendiente.is_empty(), "{}: falta decir qué está pendiente", r.id);
        }
        for (k, e) in example_origins() {
            assert!(["libro", "clase", "adaptado", "winqsb", "original"].contains(&e.origen.as_str()), "{k}");
            assert!(e.origen == "original" || !e.refs.is_empty(), "{k}: sin referencias");
            for x in &e.refs {
                assert!(get(&x.id).is_some(), "{k}: referencia {} no existe", x.id);
            }
        }
        assert!(bibtex_all().contains("@article{miller1960,"));
        // APA: dos obras del mismo autor y año llevan letra (2004a, 2004b)
        let mut seen = std::collections::HashMap::new();
        for r in all() {
            if let Some(o) = seen.insert(cite(r), r.id.clone()) {
                panic!("{} y {} tienen la misma cita «{}»: falta el sufijo", o, r.id, cite(r));
            }
        }
    }
}
