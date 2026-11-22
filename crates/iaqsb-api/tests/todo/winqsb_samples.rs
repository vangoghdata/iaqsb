//! Los 64 archivos de ejemplo que trae WinQSB 2.0: cada uno se lee, se
//! resuelve, se guarda en el formato de WinQSB y se vuelve a leer con el mismo
//! resultado. (Los nombres perdieron la última letra de la extensión al
//! descomprimirlos con EXPAND: ASA1.AS = ASA1.ASA.)

use iaqsb_api as api;
use serde_json::json;

#[test]
fn ejemplos_originales_de_winqsb() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/winqsb");
    let mut n = 0;
    let mut fails = vec![];
    let mut files: Vec<_> = std::fs::read_dir(&dir).unwrap().map(|e| e.unwrap().path()).collect();
    files.sort();
    for p in files {
        let bytes = std::fs::read(&p).unwrap();
        let content: String = bytes.iter().map(|&b| b as char).collect();
        let name = p.file_name().unwrap().to_string_lossy().to_string();
        let run = || -> Result<(), String> {
            let imp = api::call("import_text", json!({"content": content})).map_err(|e| format!("lectura: {e}"))?;
            let module = imp["module"].as_str().unwrap().to_string();
            let a = api::solve(&module, imp["model"].clone(), None).map_err(|e| format!("solución: {e}"))?;
            let ex = api::call("export_text", json!({"module": module, "model": imp["model"]})).map_err(|e| format!("escritura: {e}"))?;
            let back = api::call("import_text", json!({"content": ex["content"]})).map_err(|e| format!("relectura: {e}"))?;
            let b = api::solve(&module, back["model"].clone(), None).map_err(|e| format!("segunda solución: {e}"))?;
            let key = |v: &serde_json::Value| if v["headline"].is_string() { v["headline"].clone() } else { v["objective_value"].clone() };
            if key(&a) != key(&b) {
                return Err(format!("cambia al guardar: {} / {}", key(&a), key(&b)));
            }
            Ok(())
        };
        if let Err(e) = run() {
            fails.push(format!("{name}: {e}"));
        }
        n += 1;
    }
    assert_eq!(n, 64);
    assert!(fails.is_empty(), "{fails:#?}");
}
