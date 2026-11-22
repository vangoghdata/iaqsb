# IAQSB: contexto para Claude Code

IAQSB es una reimplementación de WinQSB en Rust, Tauri 2 y Svelte 5, con una CLI (`iaqsb`) y un servidor MCP (`iaqsb-mcp`). Es software libre (MIT), en español, inglés y portugués de Brasil, para universidades de Latinoamérica: de pregrado a doctorado. Marca: «Hecho por IA Nativa». Autor: Luis (Grupo 8, Taller de Producción y Logística, Ingeniería Industrial, Universidad Nacional de Colombia).

Lee primero `LEEME.md`, que es la fuente de verdad sobre funciones, comandos y estructura. `CONTRIBUTING.md` explica cómo se trabaja.

## Cómo trabajar con Luis

- Responde en español. Los informes finales van breves y dicen qué quedó y qué falta.
- Trabajo de nivel de investigación. Las referencias van en APA 7 y **nunca se inventan**: si no puedes verificar una fuente, márcala `por_verificar` en `crates/iaqsb-core/fuentes/bibliografia.json`, con lo que falta confirmar. Después de cambiar fuentes, regenera `docs/FUENTES.md` con `python3 tools/generar_fuentes.py`, que necesita la CLI compilada.
- Compatibilidad con WinQSB: lo que IAQSB guarda en formatos de WinQSB debe abrirse en WinQSB. Esto no limita al programa: puede ir más allá de los límites de WinQSB.
- Diseño nativo para IA: todo lo que hace la app debe poder hacerlo también una IA por la CLI o el MCP.
- Cuando la tarea es grande, trabaja hasta terminarla, prueba y luego reporta.

## Estructura

- `crates/iaqsb-core`: motor de solución (HiGHS, con conectores a Gurobi, CPLEX, SCIP, CBC y GLPK), traducción (`src/i18n.rs` y los catálogos `i18n/en.json`, `pt.json`, `*_extra.json`), bibliografía, ZIP y XLSX.
- `crates/iaqsb-lp`: LP-ILP, incluido el lenguaje algebraico en `src/algebra/` (referencia en `docs/LENGUAJE.md`).
- `crates/iaqsb-modules`: el resto de módulos de WinQSB y los de logística.
- `crates/iaqsb-formats`: archivos de WinQSB.
- `crates/iaqsb-render`: gráficas e informes (DOCX, PDF y HTML hechos a mano).
- `crates/iaqsb-api`: la API JSON que usan la app, la CLI y el MCP (`call(op, args)`).
- `apps/cli`, `apps/mcp`.
- `apps/desktop`: Svelte 5 (runes) y Tauri.
  - `src-tauri/src/red.rs` hace las peticiones del asistente de IA.
  - `src/lib/` tiene los stores: `ajustes`, `chat`, `proveedores`, `ia_openai`, `historial`, `sesion`…

## Comandos (Windows, PowerShell, desde la raíz)

- Pruebas de Rust: `cargo test --workspace --release` (son 231; la primera vez compila HiGHS y tarda). Más rápido: `cargo test --workspace --profile pruebas`.
- Pruebas de las peticiones de red de la app: `cd apps/desktop/src-tauri; cargo test`.
- Interfaz:
  - `cd apps/desktop; npm install; npx svelte-check; npm run build`
  - Pruebas de lógica: `node tools/test_ia.ts`, `node tools/test_pegar.ts`, `node tools/test_practica.ts <ruta a iaqsb.exe>`
  - De punta a punta, con Playwright y el servidor de desarrollo (`python tools/dev_server.py <iaqsb.exe> 1491`): `tools/test_ia_ui.mjs` y `tools/test_excel_ui.mjs`. Cada script explica su uso en el encabezado.
- App en desarrollo: `npx tauri dev`. Antes hay que compilar la CLI y el MCP y copiarlos a `apps/desktop/src-tauri/binaries/`; ver `scripts/build.sh`.
- Instalador de Windows: `powershell -ExecutionPolicy Bypass -File scripts\build-windows.ps1 [-Pruebas]`. Revisa qué falta (Rust MSVC, Node, C++ Build Tools, CMake, LLVM) y dice cómo instalarlo.

## Entorno de pruebas con capturas (Claude Code en Windows)

- **Interfaz en el navegador integrado:** `.claude/launch.json` define `iaqsb-navegador`, que sirve `apps/desktop/dist` con `tools/dev_server.py` y la CLI de `target/release/iaqsb.exe` en el puerto 1491. Antes: `cargo build --release -p iaqsb` y `npm run build` en `apps/desktop`. Luego `preview_start` y capturas con el navegador del panel. Después de cambiar la interfaz basta con `npm run build` y recargar la página.
- **App de escritorio real:** después de `npx tauri build` (o `scripts\build-windows.ps1`), lanzar `target\release\iaqsb-desktop.exe` y pedir acceso de computer use a `iaqsb-desktop.exe`. El nombre «IAQSB» apunta a la versión instalada en `Desktop\monitoria\IAQSB`, no a la recién compilada.
- En Windows hay que definir `LIBCLANG_PATH=C:\Program Files\LLVM\bin` para compilar HiGHS fuera del script.
- **Instaladores de macOS y Linux:** `.github/workflows/instaladores.yml` (GitHub Actions) los genera al publicar una etiqueta `v*`. Instrucciones para usuarios: `INSTALAR.md`.

## Convenciones

- Los identificadores y comentarios van en español, como en el resto del código.
- No apliques `cargo fmt` a todo el proyecto: reescribiría casi todos los archivos. Formatea solo lo que cambies, y si se adopta un formato, hazlo en un cambio aparte.
- Textos visibles:
  - En la interfaz se escriben en español con `t('…')`. La traducción va en `apps/desktop/src/lib/i18n/en_*.ts` (inglés) y `pt*.ts` (portugués; el más reciente es `pt_b.ts`).
  - En Rust se escriben en español y los traducen los catálogos del núcleo. Para encontrar los textos nuevos: `python3 tools/i18n_extraer.py`, compararlo con `en.json` y `pt.json`, y traducir lo que falte.
- Todo método o ejemplo nuevo lleva su fuente, y sus pruebas comparan contra valores publicados o contra otra implementación independiente.

## Estado (noviembre de 2026, versión 0.2.0)

Ya está hecho:
- Ajustes (Ctrl+,), con cambio de idioma que no bloquea la app.
- Portugués en toda la app.
- Informes en Word, PDF, HTML, Excel y Markdown; Excel (pegar, abrir y guardar).
- ¿Qué pasa si…? y análisis paramétrico.
- Modo práctica.
- Lenguaje algebraico con restricciones generales.
- Ajuste automático del motor.
- Deshacer y rehacer, autoguardado y archivos recientes.
- Editor visual de redes y mapa.
- Asistente de IA con cualquier proveedor: OpenRouter, Anthropic, OpenAI, Gemini, Qwen, DeepSeek, Mistral, Groq, xAI, Moonshot, Ollama, LM Studio o cualquier servidor compatible con OpenAI.
- Documentación de código abierto: README en tres idiomas, `CONTRIBUTING`, `CITATION.cff`, guía para docentes y plantillas para reportar errores.

Pendiente:
- Probar el asistente con claves reales de varios proveedores. Hasta ahora solo se probó contra servidores simulados.
- Publicar el repositorio y poner su dirección en `CITATION.cff` y en `CONTRIBUTING.md` (hoy no hay ninguna).
- Decidir si se adopta un formato de código.
- Repositorio publicado en https://github.com/vangoghdata/iaqsb (cuenta de GitHub de Luis: vangoghdata). Los instaladores de Windows, macOS y Linux los genera `.github/workflows/instaladores.yml` al subir una etiqueta `v*`. Opcional: certificado de Apple Developer para firmar y notarizar en macOS.
- Probar el motor de Python (`statsforecast`) del módulo st: el puente está escrito (`crates/iaqsb-modules/src/st/puente_statsforecast.py`) pero no se ha ejecutado, porque statsforecast no está instalado en el equipo de Luis (no se instala nada sin su permiso).
- Probar los adjuntos del asistente con proveedores reales (imágenes y PDF): se probaron con un servidor simulado compatible con OpenAI.
- Cadena jerárquica: falta el último eslabón MRP → CRP (carga de capacidad por centro de trabajo) y botones en la interfaz para pasar de un nivel a otro (hoy se hace con `link` desde la IA, la CLI o el MCP).
- Programa Jantero del profesor (`C:\jantero`, Visual FoxPro): pronósticos (promedios móviles, suavización simple y doble, Holt, Winters, correlograma), EOQ, vendedor de periódicos, planeación agregada (nivelación, persecución, óptimo), programa maestro (básico y ATO), MRP y secuenciación. IAQSB ya cubre todo salvo el correlograma. Luis no dio acceso de pantalla a Jantero; se compara con sus Excel.

Hecho el 22 de noviembre de 2026 (segunda parte):
- Módulo **pmp** (programa maestro): básico y ATO, DPP con préstamo, solicitudes de pedidos, RCCP y PMP óptimo (MIP con HiGHS, comprobado contra la enumeración en 60 problemas al azar). Reproduce exactamente los Excel del profesor.
- Módulo **mrp2** (MRP II por matrices, Gozinto), con el ejemplo de los estantes (por verificar: no hay solución del profesor).
- Ejemplo `mrp/siete_articulos_arango`: el MRP existente da exactamente la solución del Excel del profesor.
- Linux: compila y pasan las pruebas en Ubuntu 24.04 (WSL, en `/root/iaqsb`); genera `.deb` y `.AppImage`.
- `tools/dev_server.py` y `tools/generar_fuentes.py` ahora usan UTF-8 y LF también en Windows (antes dañaban las tildes y cambiaban los finales de línea).
- Módulo **st** (series de tiempo): exploración igual a R (Acf, Pacf, Box.test, decompose, BoxCox.lambda) y pronósticos ARIMA/ETS/TBATS/Theta/Croston con el mejor motor disponible: R con forecast (`options.forecast_engine`; `options.engine` queda para el optimizador), Python con statsforecast o el nativo en Rust (crate `anofox-forecast`, MIT). Los ejemplos son los de fpp2 que usa el profesor. Las pruebas comparan contra valores sacados de R y corren también sin R (`IAQSB_SIN_R=1` simula un equipo sin R).
- Asistente: adjuntos (imágenes, capturas con Ctrl+V, PDF, xlsx con la operación `xlsx_text`, CSV y archivos de modelos) en `apps/desktop/src/lib/adjuntos.ts`; herramienta `link_models`.
- Operación `link` (cadena jerárquica ap → pmp → mrp) en `crates/iaqsb-api/src/cadena.rs`; herramienta MCP `link_models`.
- En Git Bash, los argumentos que empiezan con `/` se convierten en rutas de Windows (usa `MSYS_NO_PATHCONV=1`), y `\\n` dentro de heredocs de Python puede volverse un salto de línea real: para scripts largos, escribir el archivo con la herramienta Write.

Hecho el 22 de noviembre de 2026: compilación completa en Windows (CLI, MCP, app e instalador NSIS de 23.7 MB) y la app probada con capturas.
