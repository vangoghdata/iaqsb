//! Guías «Guíame»: una por módulo, con las secciones completas y solo referencias de la
//! bibliografía central (nunca inventadas).

use iaqsb_api as api;
use iaqsb_api::core::{biblio, guias};
use serde_json::json;

#[test]
fn cada_modulo_tiene_su_guia() {
    let mods = api::list_modules();
    for m in mods["modules"].as_array().unwrap() {
        let id = m["id"].as_str().unwrap();
        if id.starts_with("c.") {
            continue; // temas de cursos instalados: su guía es del curso
        }
        assert!(guias::modulos().contains(&id), "el módulo {id} no tiene guía en crates/iaqsb-core/guias/{id}.json");
    }
}

#[test]
fn las_guias_estan_completas_y_citan_fuentes_existentes() {
    for id in guias::modulos() {
        let g = guias::cruda(id).unwrap_or_else(|| panic!("guias/{id}.json no es JSON válido"));
        assert_eq!(g["modulo"], id, "{id}: el campo modulo no coincide con el archivo");
        for k in ["titulo", "resumen", "analogia"] {
            assert!(g[k].as_str().is_some_and(|s| !s.trim().is_empty()), "{id}: falta {k}");
        }
        for k in ["que_es", "para_que_sirve", "historia", "necesitas", "pasos", "como_resuelve", "interpretar", "industria", "errores_comunes", "glosario", "referencias"] {
            assert!(g[k].as_array().is_some_and(|a| !a.is_empty()), "{id}: falta la sección {k}");
        }
        assert!(g["profesor"]["objetivos"].as_array().is_some_and(|a| !a.is_empty()), "{id}: falta profesor.objetivos");
        let mut refs: Vec<String> = g["referencias"].as_array().unwrap().iter().map(|x| x.as_str().unwrap().to_string()).collect();
        for h in g["historia"].as_array().unwrap() {
            refs.extend(h["refs"].as_array().into_iter().flatten().map(|x| x.as_str().unwrap().to_string()));
        }
        for r in refs {
            assert!(biblio::get(&r).is_some(), "{id}: cita «{r}», que no está en fuentes/bibliografia.json");
        }
    }
}

#[test]
fn la_guia_sale_por_la_api_con_las_referencias_en_apa() {
    let r = api::call("guide", json!({"module": "mrp"})).unwrap();
    assert_eq!(r["module"], "mrp");
    assert!(r["guide"]["referencias_apa"].as_array().is_some_and(|a| !a.is_empty()));
    let md = r["markdown"].as_str().unwrap();
    assert!(md.contains("## Referencias (APA 7)") && md.contains("## Cómo usarlo en Nativo"));
    assert!(api::call("guide", json!({"module": "no_existe"})).is_err());
}
