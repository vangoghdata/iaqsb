//! Ejecutar código de Python o de R de un tema de curso (`.ncourse`).
//!
//! En la app de escritorio el código corre aislado dentro de la ventana (Pyodide y webR): la app
//! registra aquí su ejecutor. En la línea de comandos y en el servidor MCP no hay ventana: el
//! código corre con el Python o el R del equipo solo si el usuario confía en el curso
//! (`IAQSB_CONFIAR_CODIGO=1` o `iaqsb --confiar`), porque ahí sí puede tocar los archivos.

use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::OnceLock;

/// Lo que se ejecuta: el lenguaje, los archivos que van en la carpeta de trabajo y el programa.
#[derive(Debug, Clone)]
pub struct Programa {
    /// "python" o "r".
    pub lenguaje: String,
    /// (nombre, contenido) de los archivos de la carpeta de trabajo (entrada.json, ayudas, datos…).
    pub archivos: Vec<(String, Vec<u8>)>,
    /// El código que se ejecuta.
    pub codigo: String,
    /// Paquetes que necesita (de PyPI o de CRAN). En la app se instalan solos dentro de la
    /// ventana (micropip en Pyodide, webR); en el equipo, si faltan, el error dice cómo instalarlos.
    pub paquetes: Vec<String>,
}

pub type Ejecutor = Box<dyn Fn(&Programa) -> Result<String, String> + Send + Sync>;

static EXTERNO: OnceLock<Ejecutor> = OnceLock::new();

/// La app de escritorio registra su ejecutor aislado (Pyodide y webR).
pub fn registrar(e: Ejecutor) {
    let _ = EXTERNO.set(e);
}

/// ¿Se puede ejecutar código de cursos (aislado en la app o con permiso en el equipo)?
pub fn disponible() -> bool {
    EXTERNO.get().is_some() || confiar()
}

fn confiar() -> bool {
    std::env::var("IAQSB_CONFIAR_CODIGO").is_ok_and(|v| v == "1" || v.eq_ignore_ascii_case("si") || v.eq_ignore_ascii_case("true"))
}

/// Ejecuta el programa y devuelve lo que escribió en la salida estándar.
pub fn ejecutar(p: &Programa) -> Result<String, String> {
    if let Some(e) = EXTERNO.get() {
        return e(p);
    }
    if !confiar() {
        return Err(format!(
            "este tema calcula con código de {}. En la app de escritorio corre aislado; en la línea de comandos o el servidor MCP corre con el {} del equipo, que puede leer y escribir archivos: si confías en el autor del curso, usa `iaqsb --confiar` o define IAQSB_CONFIAR_CODIGO=1",
            nombre(&p.lenguaje),
            nombre(&p.lenguaje)
        ));
    }
    let dir = std::env::temp_dir().join(format!("nativo-tema-{}-{}", std::process::id(), std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0)));
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let r = correr(&dir, p);
    let _ = std::fs::remove_dir_all(&dir);
    r
}

fn nombre(l: &str) -> &'static str {
    if l == "r" {
        "R"
    } else {
        "Python"
    }
}

fn correr(dir: &PathBuf, p: &Programa) -> Result<String, String> {
    for (n, d) in &p.archivos {
        let ruta = dir.join(n);
        if let Some(padre) = ruta.parent() {
            std::fs::create_dir_all(padre).map_err(|e| e.to_string())?;
        }
        std::fs::write(&ruta, d).map_err(|e| e.to_string())?;
    }
    let out = match p.lenguaje.as_str() {
        "python" => {
            let (py, pre) = crate::solver::external::python().ok_or("no hay Python en este equipo")?;
            let f = dir.join("__programa__.py");
            std::fs::write(&f, &p.codigo).map_err(|e| e.to_string())?;
            Command::new(py).args(pre).arg(&f).current_dir(dir).env("PYTHONIOENCODING", "utf-8").stdin(Stdio::null()).output()
        }
        "r" => {
            let rs = rscript().ok_or("no hay R en este equipo")?;
            let f = dir.join("__programa__.R");
            std::fs::write(&f, &p.codigo).map_err(|e| e.to_string())?;
            Command::new(rs).arg("--vanilla").arg(&f).current_dir(dir).stdin(Stdio::null()).output()
        }
        l => return Err(format!("lenguaje desconocido: «{l}» (usa python o r)")),
    }
    .map_err(|e| format!("no se pudo ejecutar {}: {e}", nombre(&p.lenguaje)))?;
    let txt = String::from_utf8_lossy(&out.stdout).to_string();
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        let mut m = format!("{}{}", txt, err.lines().rev().take(12).collect::<Vec<_>>().into_iter().rev().collect::<Vec<_>>().join("\n"));
        if !p.paquetes.is_empty() {
            let lista = p.paquetes.join(" ");
            m.push_str(&if p.lenguaje == "r" {
                format!("\nEste tema usa los paquetes de R {lista}; si faltan, instálalos en R con install.packages(c({}))", p.paquetes.iter().map(|x| format!("\"{x}\"")).collect::<Vec<_>>().join(", "))
            } else {
                format!("\nEste tema usa los paquetes de Python {lista}; si faltan, instálalos con: python -m pip install {lista}")
            });
        }
        return Err(m);
    }
    Ok(txt)
}

fn rscript() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("IAQSB_RSCRIPT") {
        return Some(PathBuf::from(p));
    }
    if let Some(p) = crate::solver::external::find_exe("Rscript") {
        return Some(p);
    }
    if cfg!(windows) {
        for base in ["C:\\Program Files\\R", "C:\\Program Files (x86)\\R"] {
            if let Ok(rd) = std::fs::read_dir(base) {
                let mut dirs: Vec<PathBuf> = rd.flatten().map(|e| e.path()).collect();
                dirs.sort();
                if let Some(d) = dirs.into_iter().rev().map(|d| d.join("bin").join("Rscript.exe")).find(|p| p.is_file()) {
                    return Some(d);
                }
            }
        }
        None
    } else {
        ["/opt/homebrew/bin/Rscript", "/usr/local/bin/Rscript", "/usr/bin/Rscript", "/Library/Frameworks/R.framework/Resources/bin/Rscript"].iter().map(PathBuf::from).find(|p| p.is_file())
    }
}
