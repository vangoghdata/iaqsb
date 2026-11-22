use iaqsb_core::Module;
use iaqsb_modules::modo::Modo;
use serde_json::json;

#[test]
fn tren_camion_avion_de_la_clase() {
    let m = Modo.examples().into_iter().next().unwrap().1;
    let r = Modo.solve(&m, &json!({})).unwrap();
    assert_eq!(r.data["best"], "Camión");
    let t = |k: usize, f: &str| r.data["modes"][k][f].as_f64().unwrap();
    // valores de la diapositiva: almacenaje en planta, CD y tránsito del tren
    assert_eq!(t(0, "plant"), 900000.0);
    assert_eq!(t(0, "dc"), 903000.0);
    assert!((t(0, "transit") - 362465.75).abs() < 0.01);
    assert!((t(0, "total") - 2235465.75).abs() < 0.01);
    assert_eq!(t(1, "dc"), 453000.0);
    assert_eq!(t(2, "dc"), 235500.0);
    assert!((t(1, "total") - 1129301.37).abs() < 0.01);
}
