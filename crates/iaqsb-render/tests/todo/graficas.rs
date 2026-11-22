//! Todas las gráficas de todos los ejemplos se dibujan sin errores.

use iaqsb_api as api;
use iaqsb_render as render;
use serde_json::json;

#[test]
fn todas_las_graficas_de_los_ejemplos() {
    let mut total = 0;
    let mut kinds = std::collections::BTreeSet::new();
    for (module, name, model) in api::examples() {
        let Ok(r) = api::solve(&module, model, Some(json!({"steps": true}))) else { continue };
        let (figs, left) = render::figures(&r, render::Pick { steps: true, max: usize::MAX });
        assert_eq!(left, 0);
        for f in &figs {
            render::validate(&f.svg).unwrap_or_else(|e| panic!("{module}/{name} · {}: {e}\n{}", f.place, f.svg));
            kinds.insert(f.kind.clone());
        }
        // una por ejemplo en PNG (la conversión es más lenta)
        if let Some(f) = figs.first() {
            let png = render::png(&f.svg, 1.0).unwrap();
            assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
        }
        total += figs.len();
    }
    assert!(total > 100, "solo {total} gráficas");
    for k in ["chart", "network", "map", "gantt", "layout", "tree"] {
        assert!(kinds.contains(k), "ningún ejemplo dibujó {k}");
    }
}

#[test]
fn recorrido_parcial_en_cada_paso() {
    let model = api::examples().into_iter().find(|e| e.1 == "clase_tsp_bebidas").unwrap().2;
    let r = api::solve("net", model, Some(json!({"tsp_method": "nearest_neighbor", "steps": true}))).unwrap();
    let (figs, _) = render::figures(&r, render::Pick::default());
    let places: Vec<&str> = figs.iter().map(|f| f.place.as_str()).collect();
    assert_eq!(places[0], "Red");
    assert!(places.iter().filter(|p| p.contains("› Paso ")).count() == 5, "{places:?}");
    // sin el paso a paso, solo la red final; y el máximo se respeta
    let (figs, left) = render::figures(&r, render::Pick { steps: false, max: 24 });
    assert_eq!((figs.len(), left), (1, 0));
    let (figs, left) = render::figures(&r, render::Pick { steps: true, max: 3 });
    assert_eq!(figs.len(), 3);
    assert!(left > 0);
    assert_eq!(render::base64(b"Man"), "TWFu");
    assert_eq!(render::base64(b"Ma"), "TWE=");
}

#[test]
fn mapa_con_latitud_y_longitud() {
    // ruteo en Bogotá: el plano usa longitud y latitud (proyección equirectangular)
    let model = api::examples().into_iter().find(|e| e.1 == "reparto_bogota").unwrap().2;
    let r = api::solve("vrp", model, Some(json!({"method": "sweep", "steps": false}))).unwrap();
    let map = r["sections"].as_array().unwrap().iter().flat_map(|s| s["blocks"].as_array().unwrap().iter()).find(|b| b["type"] == "map").unwrap().clone();
    assert_eq!(map["geo"], true);
    let svg = render::svg(&map).expect("el mapa se dibuja");
    render::validate(&svg).unwrap();
    // marcas de los ejes en grados y el rayo de inicio del barrido
    assert!(svg.contains("°"), "{svg}");
    assert!(svg.contains("inicio"));
    let png = render::png(&svg, 1.0).unwrap();
    assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
}
