//! Cursos (.ncourse): el curso de muestra es válido y se puede empaquetar, instalar, usar y
//! quitar; sus temas calculan como dicen sus lecciones (Python y R solo si están en el equipo).

use iaqsb_api as api;
use iaqsb_api::cursos;
use iaqsb_core::i18n::Lang;
use serde_json::json;
use std::path::PathBuf;

fn muestra() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../cursos/ejemplo-nativo")
}

#[test]
fn el_curso_de_muestra_es_valido() {
    let (err, avi) = cursos::validar(&muestra());
    assert!(err.is_empty(), "errores: {err:?}");
    assert!(avi.is_empty(), "avisos: {avi:?}");
}

#[test]
fn ciclo_completo_de_un_curso() {
    let dir = std::env::temp_dir().join(format!("nativo-cursos-prueba-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    // SAFETY: solo esta prueba usa la carpeta de cursos; las demás no dependen de ella.
    unsafe { std::env::set_var("IAQSB_CURSOS", &dir) };

    let paquete = cursos::empaquetar(&muestra()).unwrap();
    let r = cursos::instalar(&paquete).unwrap();
    assert_eq!(r["id"], "ejemplo-nativo");
    assert_eq!(r["temas"], 4);
    // los temas aparecen como módulos
    let mods = api::list_modules();
    let ids: Vec<&str> = mods["modules"].as_array().unwrap().iter().filter_map(|m| m["id"].as_str()).collect();
    assert!(ids.contains(&"c.ejemplo-nativo.mezcla-de-productos"));

    // modelo algebraico con HiGHS: 40 sillas y 20 estantes, utilidad 2200
    let ej = api::call("examples", json!({"module": "c.ejemplo-nativo.mezcla-de-productos"})).unwrap();
    let modelo = ej[0]["model"].clone();
    let rep = api::call("solve", json!({"module": "c.ejemplo-nativo.mezcla-de-productos", "model": modelo})).unwrap();
    assert!((rep["objective_value"].as_f64().unwrap() - 2200.0).abs() < 1e-6, "{}", rep["headline"]);
    assert_eq!(rep["sections"][0]["id"], "solucion");

    // la lección del ERP con sus lecciones y referencias en APA
    let g = api::call("guide", json!({"module": "c.ejemplo-nativo.que-es-un-erp"})).unwrap();
    assert_eq!(g["guide"]["lecciones"].as_array().unwrap().len(), 2);
    assert!(g["markdown"].as_str().unwrap().contains("Jacobs"));

    // Python y R: solo con el intérprete del equipo y con permiso
    unsafe { std::env::set_var("IAQSB_CONFIAR_CODIGO", "1") };
    if iaqsb_core::solver::external::python().is_some() {
        let ej = cursos::ejemplos("c.ejemplo-nativo.punto-de-equilibrio").unwrap();
        let r = cursos::resolver("c.ejemplo-nativo.punto-de-equilibrio", ej[0]["model"].clone(), None, Lang::Es).unwrap();
        // 12 000 000 / (180 000 − 110 000) = 171.43 mesas
        assert!((r["data"]["punto_equilibrio"].as_f64().unwrap() - 171.428_571).abs() < 1e-3, "{}", r["headline"]);
    }
    if iaqsb_api::modules::st::motores::r_info().is_some() {
        let ej = cursos::ejemplos("c.ejemplo-nativo.promedios-moviles").unwrap();
        let r = cursos::resolver("c.ejemplo-nativo.promedios-moviles", ej[0]["model"].clone(), None, Lang::Es).unwrap();
        // (150 + 145 + 158) / 3 = 151
        assert!((r["data"]["pronostico"].as_f64().unwrap() - 151.0).abs() < 1e-9, "{}", r["headline"]);
    }
    unsafe { std::env::remove_var("IAQSB_CONFIAR_CODIGO") };

    cursos::quitar("ejemplo-nativo").unwrap();
    assert!(cursos::instalados().is_empty());

    // el asistente de creación: borrador → temas → validar → vista previa → probar → exportar
    let borradores = dir.join("borradores");
    unsafe { std::env::set_var("IAQSB_BORRADORES", &borradores) };
    let b = api::call("draft_new", json!({"title": "Producción y logística"})).unwrap();
    let id = b["id"].as_str().unwrap().to_string();
    assert_eq!(id, "produccion-y-logistica");
    // vacío: no tiene temas y no es válido todavía
    let v = api::call("draft_validate", json!({"id": id})).unwrap();
    assert_eq!(v["ok"], false);
    let t1 = api::call("draft_add_topic", json!({"id": id, "title": "¿Qué es un MRP?", "kind": "leccion"})).unwrap();
    assert_eq!(t1["id"], "que-es-un-mrp");
    let t2 = api::call("draft_add_topic", json!({"id": id, "title": "Mezcla óptima", "kind": "algebra"})).unwrap();
    for k in ["python", "r"] {
        api::call("draft_add_topic", json!({"id": id, "title": format!("Calculadora {k}"), "kind": k})).unwrap();
    }
    api::call("draft_write", json!({"id": id, "path": "curso.json", "content": serde_json::to_string(&{
        let mut c = api::call("draft_read", json!({"id": id})).unwrap()["curso"].clone();
        c["descripcion"] = json!("Curso de prueba");
        c["autores"] = json!([{"nombre": "Prueba"}]);
        c
    }).unwrap()})).unwrap();
    // un JSON roto no se guarda, ni una ruta que se sale del borrador
    assert!(api::call("draft_write", json!({"id": id, "path": "temas/x/tema.json", "content": "{roto"})).is_err());
    assert!(api::call("draft_write", json!({"id": id, "path": "../fuera.md", "content": "x"})).is_err());
    assert!(api::call("draft_write", json!({"id": id, "path": "temas/a/programa.exe", "content": "x"})).is_err());
    let r = api::call("draft_read", json!({"id": id})).unwrap();
    assert_eq!(r["validacion"]["ok"], true, "{}", r["validacion"]);
    assert_eq!(r["temas"].as_array().unwrap().len(), 4);
    let leccion = api::call("draft_read", json!({"id": id, "path": "temas/que-es-un-mrp/leccion.md"})).unwrap();
    assert!(leccion["texto"].as_str().unwrap().starts_with("# ¿Qué es un MRP?"));
    // probar instala la vista previa y resuelve los ejemplos (el modelo algebraico sin depender del equipo)
    let p = api::call("draft_test", json!({"id": id, "topic": t2["id"]})).unwrap();
    assert_eq!(p["resultados"][0]["ok"], true, "{}", p);
    assert!(cursos::tema(&format!("c.produccion-y-logistica.{}", t2["id"].as_str().unwrap())).is_some());
    assert_eq!(api::call("draft_list", json!({})).unwrap()["borradores"][0]["instalado"], true);
    // exportar y volver a abrir como borrador
    let pk = api::call("draft_pack", json!({"id": id})).unwrap();
    let bytes = api::base64_decode(pk["base64"].as_str().unwrap()).unwrap();
    let otro = cursos::borradores::importar(&bytes).unwrap();
    assert_eq!(otro["id"], "produccion-y-logistica-2");
    api::call("draft_delete", json!({"id": "produccion-y-logistica-2", "path": "temas/que-es-un-mrp"})).unwrap();
    assert_eq!(api::call("draft_validate", json!({"id": "produccion-y-logistica-2"})).unwrap()["ok"], false);
    api::call("draft_remove", json!({"id": "produccion-y-logistica-2"})).unwrap();
    api::call("draft_remove", json!({"id": id, "with_preview": true})).unwrap();
    assert!(cursos::instalados().is_empty());
    assert_eq!(api::call("draft_list", json!({})).unwrap()["borradores"].as_array().unwrap().len(), 0);
    unsafe { std::env::remove_var("IAQSB_BORRADORES") };
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn texto_de_diapositivas_y_word() {
    let mut z = iaqsb_core::zip::ZipWriter::new();
    z.add("ppt/slides/slide2.xml", br#"<p:sld><a:p><a:r><a:t>MRP II</a:t></a:r></a:p><a:p><a:r><a:t>Capacidad &amp; carga</a:t></a:r></a:p></p:sld>"#);
    z.add("ppt/slides/slide1.xml", br#"<p:sld><a:p><a:r><a:t>Planeaci&#243;n</a:t></a:r><a:r><a:t> de la producci&#xF3;n</a:t></a:r></a:p></p:sld>"#);
    z.add("ppt/notesSlides/notesSlide2.xml", br#"<p:notes><a:p><a:r><a:t>Explicar el CRP</a:t></a:r></a:p></p:notes>"#);
    let r = cursos::borradores::texto_office(&z.finish(), 10_000).unwrap();
    let t = r["text"].as_str().unwrap();
    assert_eq!(r["diapositivas"], 2);
    let i1 = t.find("## Diapositiva 1").unwrap();
    let i2 = t.find("## Diapositiva 2").unwrap();
    assert!(i1 < i2, "{t}");
    assert!(t.contains("MRP II
Capacidad & carga"), "{t}");
    assert!(t.contains("Notas: Explicar el CRP"), "{t}");
    let mut z = iaqsb_core::zip::ZipWriter::new();
    z.add("word/document.xml", br#"<w:document><w:body><w:p><w:r><w:t>Hola</w:t></w:r><w:r><w:t xml:space="preserve"> mundo</w:t></w:r></w:p><w:p><w:r><w:t>Fin</w:t></w:r></w:p></w:body></w:document>"#);
    let r = cursos::borradores::texto_office(&z.finish(), 10_000).unwrap();
    assert_eq!(r["text"].as_str().unwrap().trim(), "Hola mundo
Fin");
}

#[test]
fn un_paquete_con_rutas_peligrosas_no_se_instala() {
    let mut z = iaqsb_core::zip::ZipWriter::new();
    z.add("../fuera.txt", b"x");
    let e = cursos::instalar(&z.finish()).unwrap_err().to_string();
    assert!(e.contains("ruta no permitida"), "{e}");
}
