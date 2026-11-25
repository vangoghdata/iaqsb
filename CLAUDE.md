# IAQSB: contexto para Claude Code

IAQSB es un programa para aprender y aplicar la ingeniería industrial (investigación de operaciones, decisiones, producción, logística y más), en Rust, Tauri 2 y Svelte 5, con una CLI (`iaqsb`) y un servidor MCP (`iaqsb-mcp`). Empezó reemplazando a WinQSB (abre y guarda sus archivos), pero Luis no quiere que se presente como una copia de WinQSB: es una plataforma de aprendizaje que va a crecer con temas que creen profesores y estudiantes. Es software libre (MIT), en español, inglés y portugués de Brasil, para universidades de Latinoamérica: de pregrado a doctorado. Marca: «Hecho por IA Nativa». Autor: Luis (Grupo 8, Taller de Producción y Logística, Ingeniería Industrial, Universidad Nacional de Colombia).

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

- Pruebas de Rust: `cargo test --workspace --release` (son 233; la primera vez compila HiGHS y tarda). Más rápido: `cargo test --workspace --profile pruebas`.
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

## Estado (noviembre de 2026, versión 0.5.0)

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
- Repositorio publicado en https://github.com/vangoghdata/iaqsb (cuenta de GitHub de Luis: vangoghdata). Los instaladores de Windows, macOS y Linux los genera `.github/workflows/instaladores.yml` al subir una etiqueta `v*`. La versión 0.5.0 está publicada (https://github.com/vangoghdata/iaqsb/releases/tag/v0.5.0, la más reciente) con los seis instaladores: Windows, macOS chip Apple (macos-15) e Intel (macos-15-intel), .deb, .rpm y AppImage. Para una versión nueva: subir la versión en Cargo.toml, package.json y tauri.conf.json, crear la etiqueta y publicar el borrador. Opcional: certificado de Apple Developer para firmar y notarizar en macOS.
- Probar los adjuntos del asistente con proveedores reales (imágenes y PDF): se probaron con un servidor simulado compatible con OpenAI.
- Cadena jerárquica (ap → pmp → mrp → crp) completa en el núcleo; faltan botones en la interfaz para pasar de un nivel a otro (hoy se hace con `link` desde la IA, la CLI o el MCP).
- Programa Jantero del profesor (`C:\jantero`, Visual FoxPro): pronósticos (promedios móviles, suavización simple y doble, Holt, Winters, correlograma), EOQ, vendedor de periódicos, planeación agregada (nivelación, persecución, óptimo), programa maestro (básico y ATO), MRP y secuenciación. IAQSB ya cubre todo salvo el correlograma. Luis no dio acceso de pantalla a Jantero; se compara con sus Excel.

Hecho el 25 de noviembre de 2026 (versión 0.5.0): cursos hechos de temas que se reutilizan.
- **Tipo de tema `modulo`** (`"base": "<id de un módulo>"`, nunca `lp`): usa la pantalla, el motor, el paso a paso, la guía y todo lo demás del módulo. En `lib.rs`, las operaciones de un tema con base que no son describe_module, spec, template, solve, examples ni guide se pasan al módulo base; `cursos::spec/ejemplos/plantilla/describir/resolver/guia` las arman con lo del curso encima (ejemplos del curso primero y luego los del módulo salvo `"ejemplos_base": false`; la guía del módulo con los campos de `guia.json` del tema reemplazándola). **Ajuste** opcional (`"ajuste": {"lenguaje": "python"|"r", "archivo", "paquetes"}`): recibe el informe del módulo como `base` (`from nativo import base, Informe`; `Informe.desde(base)` conserva secciones, pasos y datos). En la interfaz, `basesDeTemas` (grafo.ts, lo llena `shell.init`) hace que los temas con base se comporten como su módulo (redes) y tomen su ícono.
- **Biblioteca**: operación `topic_library` (módulos de Nativo y temas de cursos instalados) y `draft_import_topic` (`from`: `modulo:<id>` o `c.<curso>.<tema>`; la copia guarda `origen` con curso, versión, autores y licencia, y pasa a `fuentes.json` las referencias propias que cita). En el creador: «Agregar un tema» ofrece temas nuevos, módulos de Nativo y temas de otros cursos; el asistente tiene `list_topic_library` e `import_topic` y sus instrucciones le piden dividir la materia en temas y reutilizar antes de escribir.
- **Paquetes**: `calculo.paquetes` / `ajuste.paquetes` → `Programa.paquetes` (iaqsb-core/ejecutor.rs) → evento `codigo-ejecutar` → micropip (Pyodide) o webR. En el equipo, si faltan, el error dice cómo instalarlos. `run_python` y `run_r` del asistente aceptan `packages`. Rust no corre dentro de un curso.
- **Validación de `guia.json`** con los campos de Guíame (`CAMPOS_GUIA` en cursos/mod.rs; rechaza campos inventados como `usos_reales`) y `course_format` incluye `ESQUEMA.md`. El id de `tema.json` debe ser el de su carpeta.
- **Error de Claude «Invalid signature in thinking block… The tools list differs»**: la lista de herramientas cambiaba al instalar un curso (el id del módulo llevaba un `enum` con los módulos instalados). Ahora es fija, y si Claude rechaza una firma, `quitarRazonamiento()` quita los bloques de razonamiento del historial y vuelve a pedir.
- Pendiente: crear cursos desde Claude o ChatGPT en la web (contexto descargable o servidor MCP remoto en un VPS); exponer los borradores y la biblioteca en la herramienta MCP `courses`.

Hecho el 24 de noviembre de 2026 (versión 0.4.0):
- **Asistente de creación de cursos en la app** (fase 2). Núcleo: `crates/iaqsb-api/src/cursos/borradores.rs` (borradores en `<carpeta de datos>/borradores/<id>`, al lado de `cursos/`; `IAQSB_BORRADORES` la cambia). Operaciones de la API: `draft_list`, `draft_new` (`from`: vacio, plantilla o `curso:<id>`), `draft_import` (abrir un .ncourse), `draft_read` (todo o un archivo), `draft_write` (texto o base64; revisa los JSON, rutas seguras, extensiones permitidas), `draft_delete`, `draft_add_topic` (plantillas que funcionan: leccion, python, r, algebra), `draft_validate`, `draft_install` (vista previa: lo instala como curso; el id con que quedó va en `.borrador.json`), `draft_test` (instala y resuelve los ejemplos), `draft_pack`, `draft_remove`; además `course_format` (docs/CURSOS.md) y `office_text` (texto de .pptx con notas del orador y de .docx, para adjuntarlos al chat). App: `lib/creador.svelte.ts` y `components/creador/CreadorPage.svelte` (inicio con borradores; editor con estructura, archivos, validación, pruebas, vista previa y exportar .ncourse), «Crear un curso…» en la barra lateral y «Volver al curso» en la barra superior. El chat tiene `modo` tutor o creador (`chat.entrarCreador(id)` / `salirCreador()`): en el creador cambian las instrucciones (`sistemaCreador`), el estado que va en cada pregunta (`estadoCreador`, con las herramientas que permite el autor) y se suman las herramientas `get_course_format`, `get_course`, `read_course_file`, `write_course_file`, `delete_course_file`, `copy_attachment_to_course`, `validate_course`, `test_course`, `preview_course` y `search_bibliography`; cada borrador tiene su historial de conversaciones. Pruebas: `ciclo_completo_de_un_curso` y `texto_de_diapositivas_y_word` en `crates/iaqsb-api/tests/todo/cursos.rs`, y la sección «Asistente de creación de cursos» de `tools/test_ia_ui.mjs` (modelo simulado `mock/creador`; usar el servidor `iaqsb-prueba-cursos` de `.claude/launch.json`, con carpetas de cursos y borradores aparte).
- Pendiente de esta línea: biblioteca de PDF de Luis (fase 3: enriquecer guías y crear temas con sus libros), catálogo en línea y traducción de guías y cursos (fase 4).

Hecho el 24 de noviembre de 2026 (versión 0.3.0):
- **Nombre provisional «Nativo».** El nombre visible sale de una constante: `MARCA` en `crates/iaqsb-core/src/i18n.rs` (lo aplican `translate`, `translate_value` y `translate_markdown`, también en español) y en `apps/desktop/src/lib/marca.ts` (lo aplica `t()`). Cambia «IAQSB» suelto, no «IAQSB-ST», «.iaqsb» ni nombres internos. Siguen igual: crates, comando `iaqsb`, servidor `iaqsb-mcp`, formatos, identificador `com.ianativa.iaqsb` (carpeta de datos), `productName` y nombres de los instaladores (para no duplicar instalaciones mientras el nombre no sea definitivo). Título de la ventana: Nativo.
- **Cursos `.ncourse`** (formato en `docs/CURSOS.md`): `crates/iaqsb-api/src/cursos/` (validar, empaquetar, instalar, quitar, nuevo, probar; cada tema es el módulo `c.<curso>.<tema>` para spec, examples, template, solve, describe_module y guide). Cálculo en Python (`nativo.py`), R (`nativo.R`, sin paquetes: los datos llegan como `entrada.R`) o el lenguaje algebraico (huecos `{{campo}}` y `{{tabla.columna}}`, HiGHS). El código corre con `iaqsb_core::ejecutor`: en la app, aislado en la ventana (evento `codigo-ejecutar` en `puentes.rs` → Pyodide o webR en `motores_web.svelte.ts`); en CLI y MCP, con el Python o el R del equipo solo con `--confiar` o `IAQSB_CONFIAR_CODIGO=1`. Cursos instalados en `<carpeta de datos>/cursos` (la misma para app, CLI y MCP; `IAQSB_CURSOS` la cambia). CLI `iaqsb curso nuevo|validar|probar|empaquetar|instalar|quitar|lista`; MCP `courses`; skill `skills/crear-curso-nativo/SKILL.md`; curso de muestra `cursos/ejemplo-nativo` (lección del ERP, equilibrio en Python, mezcla en algebraico, promedios móviles en R). App: grupo por curso en la barra lateral, «Instalar un curso…», arrastrar el `.ncourse` (la ventana tiene `dragDropEnabled: false` para recibir archivos por HTML) o abrirlo con doble clic; `TemaPage.svelte` para los temas que son solo lección. Pruebas: `crates/iaqsb-api/tests/todo/cursos.rs`.

Hecho el 23 de noviembre de 2026 (versión 0.2.3):
- **Guíame**: guía didáctica de los 28 módulos en `crates/iaqsb-core/guias/<id>.json` (formato en `guias/ESQUEMA.md`): qué es, para qué sirve, historia con fuentes, qué necesitas, pasos en IAQSB, cómo lo resuelve, cómo leer los resultados, industria, alternativas, errores comunes, sección del profesor, glosario y referencias (ids de la bibliografía; se agregaron 19 referencias, 3 por verificar). Operación `guide` de la API (`src/guias.rs` resuelve las citas a APA 7), CLI `iaqsb guia <módulo>`, herramienta MCP y del asistente `get_module_guide`, botón **Guíame** en la barra de arriba (`GuiaPanel.svelte`, `lib/guia.svelte.ts`). Las guías están solo en español (pendiente: inglés y portugués). Prueba: `crates/iaqsb-api/tests/todo/guias.rs`.
- **Nativo** (el robot de IA Nativa) en el asistente: animaciones en `apps/desktop/public/nativo/*.webp` (webp animado con transparencia, 256 px, 20 fps; `-0.webp` quietas). Se hacen en el repositorio de Luis `C:\Users\luisf\Proyectos\ia-nativa-videos`: `personajes/panel_iaqsb.py` (Blender 5.3, tomas reposo, pensando, calculando, explicando, celebrando, confundido, saludo y chat) y `personajes/iaqsb.mjs` (ffmpeg a webp y copia a IAQSB). `chat.nativo` elige la toma según lo que pasa; `Nativo.svelte`; ajuste `ajustes.nativo`. Tono del asistente: profesional y educativo (Luis no quiere chistes ni emojis).

Hecho el 23 de noviembre de 2026 (versión 0.2.2):
- Causa de los números distintos de R en el Mac: el equipo no tenía R y el motor nativo (anofox) no es el de R. Ahora:
  - `crates/iaqsb-modules/src/st/ets.rs`: `ets()` de forecast traducido a Rust (initstate, initparam, admissible, etscalc, el Nelder-Mead `nmmin` de R, selección por AICc y forecast.ets con las clases 1, 2 y 3). ses, holt, hw y ETS dan lo mismo que R (parámetros con diferencias de 1e-10). TBATS nativo sigue siendo el de anofox: solo con R da lo de R.
  - La app de escritorio trae R: webR 0.6.0 (R 4.6 en WebAssembly) desde https://webr.r-wasm.org, con forecast de https://repo.r-wasm.org. Si el equipo no tiene R con forecast, el módulo st le pide a la ventana que ejecute el programa de R (`registrar_r_externo` en `st/motores.rs`, puente en `apps/desktop/src-tauri/src/puentes.rs`, `src/lib/motores_web.svelte.ts`). Da los mismos números que R. La CSP permite webr.r-wasm.org, repo.r-wasm.org y cdn.jsdelivr.net, con 'wasm-unsafe-eval' y 'unsafe-eval' (webR los necesita).
  - Tipo de problema `evaluacion` en st: start ("2021-05"), train_end ("2023-12"), models ("ses, naive, tbats"); accuracy() en entrenamiento y prueba, checkresiduals() (Ljung-Box como forecast 9: gl = retardos salvo en ARIMA, retardos = min(2m, round(n/5))) y la suavización que sugiere el correlograma. Ejemplo `st/evaluacion_clase` (cuestionario del curso).
- Asistente: herramientas `run_r` (webR) y `run_python` (Pyodide 0.29.5) con los adjuntos en la carpeta de trabajo y las gráficas en el chat; conversaciones guardadas en la carpeta de datos de la app (`conversaciones/`, comandos `conversacion_*` en puentes.rs; en el navegador, localStorage) con historial (botón del reloj).
- Respuestas de los ejercicios que mandó Luis: series (datos de mayo de 2021): 1) 3, 2) 881.5052, 3) 889.3556, 4) 954.0058, 5) 1 (la versión ×3.1875: 2809.6829, 2834.8211, 3040.8936); PMP ATO: último periodo con PMP = 0 es el 14; MRP II: necesidad ajustada del periodo 3 = 0 (277 − 533 − 36 < 0); barrera de tiempo de demanda de A = 21.5 h (A 3.0 + B 9.2 + D 9.3, la ruta más larga).

Hecho el 23 de noviembre de 2026 (versión 0.2.1):
- Asistente con Claude: el sistema queda fijo por conversación (`sistemaFijo` en `chat.svelte.ts`) y el estado de la app va en un bloque `<app_state>` al inicio de cada pregunta, porque Claude ata los bloques de razonamiento al historial exacto (sistema incluido) y rechazaba la conversación con «Invalid `signature` in `thinking` block». Al restaurar una conversación guardada se quitan los bloques de razonamiento. `MAX_TOKENS` subió a 64 000.
- Ajustes → Inteligencia artificial → Nivel de razonamiento (`ajustes.razonamiento`): `output_config.effort` en Claude; `reasoning_effort` (o `reasoning.effort` en OpenRouter) en los compatibles con OpenAI. Si el modelo no admite el nivel, baja a «high» o se quita.

Hecho el 22 de noviembre de 2026 (segunda parte):
- Módulo **pmp** (programa maestro): básico y ATO, DPP con préstamo, solicitudes de pedidos, RCCP y PMP óptimo (MIP con HiGHS, comprobado contra la enumeración en 60 problemas al azar). Reproduce exactamente los Excel del profesor.
- Módulo **mrp2** (MRP II por matrices, Gozinto), con el ejemplo de los estantes (por verificar: no hay solución del profesor).
- Ejemplo `mrp/siete_articulos_arango`: el MRP existente da exactamente la solución del Excel del profesor.
- Linux: compila y pasan las pruebas en Ubuntu 24.04 (WSL, en `/root/iaqsb`); genera `.deb` y `.AppImage`.
- `tools/dev_server.py` y `tools/generar_fuentes.py` ahora usan UTF-8 y LF también en Windows (antes dañaban las tildes y cambiaban los finales de línea).
- Módulo **st** (series de tiempo): exploración igual a R (Acf, Pacf, Box.test, decompose, BoxCox.lambda) y pronósticos ARIMA/ETS/TBATS/Theta/Croston con el mejor motor disponible: R con forecast (`options.forecast_engine`; `options.engine` queda para el optimizador), Python con statsforecast o el nativo en Rust (crate `anofox-forecast`, MIT). Los ejemplos son los de fpp2 que usa el profesor. Las pruebas comparan contra valores sacados de R y corren también sin R (`IAQSB_SIN_R=1` simula un equipo sin R).
- Motor de Python del módulo st probado con statsforecast 2.1.1 (instalado en el equipo de Luis con `pip install --user statsforecast`). Su auto.arima elige a veces modelos distintos a los de R: es otra implementación.
- Asistente: adjuntos (imágenes, capturas con Ctrl+V, PDF, xlsx con la operación `xlsx_text`, CSV y archivos de modelos) en `apps/desktop/src/lib/adjuntos.ts`; herramienta `link_models`.
- Operación `link` (cadena jerárquica ap → pmp → mrp) en `crates/iaqsb-api/src/cadena.rs`; herramienta MCP `link_models`.
- En Git Bash, los argumentos que empiezan con `/` se convierten en rutas de Windows (usa `MSYS_NO_PATHCONV=1`), y `\\n` dentro de heredocs de Python puede volverse un salto de línea real: para scripts largos, escribir el archivo con la herramienta Write.

Hecho el 22 de noviembre de 2026: compilación completa en Windows (CLI, MCP, app e instalador NSIS de 23.7 MB) y la app probada con capturas.
