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
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn un_paquete_con_rutas_peligrosas_no_se_instala() {
    let mut z = iaqsb_core::zip::ZipWriter::new();
    z.add("../fuera.txt", b"x");
    let e = cursos::instalar(&z.finish()).unwrap_err().to_string();
    assert!(e.contains("ruta no permitida"), "{e}");
}
