# Cómo contribuir a IAQSB

*[English summary](#english-summary) · [Resumo em português](#resumo-em-português)*

Gracias por querer mejorar IAQSB. Lo usan estudiantes, docentes e investigadores, así que cada cambio tiene que ser correcto, tener fuente y funcionar en los tres idiomas. Esta guía dice cómo reportar errores, proponer métodos y ejemplos, preparar el entorno, escribir código y pruebas, traducir y enviar un *pull request*. Al participar aceptas el [código de conducta](CODE_OF_CONDUCT.md). Puedes escribir en español, portugués o inglés.

## Contenido

1. [Reportar un error](#1-reportar-un-error)
2. [Proponer un método, un módulo o un ejemplo](#2-proponer-un-método-un-módulo-o-un-ejemplo)
3. [Preparar el entorno](#3-preparar-el-entorno)
4. [Cómo está hecho un módulo](#4-cómo-está-hecho-un-módulo)
5. [Pruebas](#5-pruebas)
6. [Estilo del código](#6-estilo-del-código)
7. [Traducciones](#7-traducciones)
8. [Compatibilidad con WinQSB](#8-compatibilidad-con-winqsb)
9. [Pull requests](#9-pull-requests)
10. [Integridad académica](#10-integridad-académica)

## 1. Reportar un error

Abre un *issue* en el [repositorio](https://github.com/vangoghdata/iaqsb/issues) con la plantilla **«Error / Bug / Erro»** (`.github/ISSUE_TEMPLATE/error.yml`). Pide:

- el **módulo** (LP-ILP, NET, QA…, o lenguaje algebraico, CLI, MCP);
- **qué hiciste**, con el archivo del problema adjunto (`.iaqsb`, `.LPP`, `.json` o `.xlsx`) o un ejemplo mínimo que lo reproduzca;
- **qué esperabas y qué pasó**, con el mensaje de error completo (en el lenguaje algebraico, con su línea y columna);
- la **versión y el idioma** (`iaqsb --version` o **Ajustes → Acerca de IAQSB**) y el sistema operativo.

Si el resultado es incorrecto, di **de dónde sale el correcto**: libro con capítulo, sección y página, u otro programa con su versión. Sin esa referencia no se puede saber cuál de los dos está mal.

## 2. Proponer un método, un módulo o un ejemplo

Usa la plantilla **«Propuesta / Feature request / Sugestão»** (`.github/ISSUE_TEMPLATE/mejora.yml`): qué propones, la fuente y el nivel (pregrado, maestría, doctorado o industria).

### La regla de las fuentes

En IAQSB cada método, fórmula, paso y ejemplo dice de dónde sale, en **APA 7.ª edición**. Un método o un ejemplo nuevo solo entra con su fuente, y las pruebas lo exigen (`crates/iaqsb-api/tests/todo/fuentes.rs`).

- **Nunca inventes datos bibliográficos.** Si no conoces la página, la edición, el número o el DOI, déjalo vacío y márcalo como pendiente.
- Una referencia es `"verificada"` cuando comprobaste sus datos en el DOI, el editor o un catálogo de biblioteca. Si falta algo, es `"por_verificar"` y el campo `"pendiente"` dice exactamente qué falta (por ejemplo, «Confirmar páginas y editorial en un catálogo.»).
- Las diapositivas del curso Taller de Producción y Logística (Universidad Nacional de Colombia) se citan como Sarache (2026a, b, c…), y si la diapositiva nombra un libro, se cita también el libro.

### Dónde se registra

Todo está en `crates/iaqsb-core/fuentes/`; la nota `_nota` al principio de cada archivo explica sus campos.

**`bibliografia.json`**: una entrada por referencia en `referencias`. Campos: `id` (único, por ejemplo `dantzig1954`), `tipo` (`articulo`, `libro`, `capitulo`, `diapositivas`, `informe`, `software`, `norma` o `tesis`), `autores` (`"Apellido, I."`), `anio`, `titulo`, y según el tipo `revista`, `volumen`, `numero`, `paginas`, `doi`, `editorial`, `edicion`, `isbn`, `url`, `libro` y `editores_libro`; además `estado`, `pendiente` (si está por verificar), `tema` (para qué se usa en IAQSB) y `nota`. Ejemplo:

```json
{
  "id": "dantzig1954",
  "tipo": "articulo",
  "autores": ["Dantzig, G.", "Fulkerson, R.", "Johnson, S."],
  "anio": "1954",
  "titulo": "Solution of a large-scale traveling-salesman problem",
  "revista": "Journal of the Operations Research Society of America",
  "volumen": "2",
  "numero": "4",
  "paginas": "393–410",
  "doi": "10.1287/opre.2.4.393",
  "estado": "verificada",
  "tema": "Agente viajero: restricciones de eliminación de subciclos (cortes DFJ)."
}
```

**`ejemplos.json`**: el origen de los datos de cada ejemplo, con la clave `"módulo/ejemplo"` (el nombre que devuelve `examples()` del módulo, por ejemplo `"vrp/barrido_clase"`). Campos: `origen` (`libro`, `clase`, `adaptado`, `winqsb` u `original`), `refs` (lista de `{"id": …, "donde": "cap. 10, …"}`; vacía solo si el origen es `original`), `resultado` (el resultado publicado en la fuente; la prueba comprueba que sus números aparezcan en el informe), `estado`, `pendiente` y `nota`.

**`metodos.json`**: qué métodos cita cada módulo según el tipo de problema y las opciones (reglas `si` → `refs`). Los módulos de logística (`net`, `vrp`, `modo`, `inv`, `fin`, `alm`, `kpi`) citan desde su código con `Report::cite` o `add_cite` (`crates/iaqsb-core/src/report.rs`), como hace `cite()` en `crates/iaqsb-modules/src/vrp/mod.rs`.

Después de cambiar la bibliografía o los ejemplos, regenera los documentos:

```sh
cargo build --release -p iaqsb       # el script usa target/release/iaqsb
python3 tools/generar_fuentes.py     # o: python3 tools/generar_fuentes.py target/debug/iaqsb
```

Esto reescribe `docs/FUENTES.md` y `docs/iaqsb.bib`; inclúyelos en el *pull request*. Los textos nuevos de `descripcion`, `nota`, `pendiente`, `tema`, `donde`, `resultado` y `para` también se traducen (sección 7).

## 3. Preparar el entorno

Hace falta:

- **Rust** estable (con `rustup`; en Windows, el *toolchain* MSVC).
- **Node.js** LTS y npm. Los scripts de prueba en TypeScript de `apps/desktop/tools` piden Node 22.18 o posterior.
- **CMake** y **clang/LLVM**, porque HiGHS se compila desde su código en C++ (en Windows, además, Microsoft C++ Build Tools).
- **Python 3** para los scripts de `tools/`.

En Windows, `scripts\build-windows.ps1` revisa todo eso y dice cómo instalar lo que falte (ver «Compilar en Windows» en [LEEME.md](LEEME.md)).

Núcleo, CLI y servidor MCP (desde la raíz):

```sh
cargo build                 # todo el espacio de trabajo
cargo test                  # todas las pruebas
cargo test --profile pruebas   # optimizadas y sin LTO (más rápidas si son muchas)
cargo run -p iaqsb -- modulos  # la CLI sin instalarla
```

La primera compilación tarda varios minutos (HiGHS y unas 500 dependencias); después solo se recompila lo que cambia.

Aplicación de escritorio (Tauri 2 + Svelte 5):

```sh
cd apps/desktop
npm install
npx tauri dev               # con recarga en caliente
npx svelte-check            # tipos de TypeScript y Svelte
npm run build               # compila la interfaz (vite)
```

`tauri.conf.json` declara la CLI y el servidor MCP como binarios externos (`bundle.externalBin`), y Tauri los busca en `apps/desktop/src-tauri/binaries/` con el triple de la plataforma en el nombre. Esa carpeta no está en el repositorio. Antes del primer `npx tauri dev`, créalos como lo hace `scripts/build.sh`:

```sh
cargo build --release -p iaqsb -p iaqsb-mcp
triple=$(rustc --print host-tuple)
mkdir -p apps/desktop/src-tauri/binaries
cp target/release/iaqsb     apps/desktop/src-tauri/binaries/iaqsb-$triple
cp target/release/iaqsb-mcp apps/desktop/src-tauri/binaries/iaqsb-mcp-$triple
```

(En Windows, con `.exe` al final de ambos nombres, como en `scripts/build-windows.ps1`.) `apps/desktop/src-tauri` está fuera del espacio de trabajo de Cargo, pero compila en la misma carpeta `target/` (`apps/desktop/src-tauri/.cargo/config.toml`).

Para probar la interfaz en un navegador sin Tauri: `cargo build -p iaqsb`, `npm run build` y `python3 tools/dev_server.py` (desde `apps/desktop`; responde a la interfaz llamando a la CLI).

Instaladores: `scripts/build.sh` (Linux y macOS; `--pruebas` corre antes las pruebas) y `scripts\build-windows.ps1` (`-Pruebas`).

## 4. Cómo está hecho un módulo

LP-ILP vive en `crates/iaqsb-lp`. Los demás, cada uno en su carpeta `crates/iaqsb-modules/src/<id>/`:

- `mod.rs` implementa `TypedModule` (`crates/iaqsb-core/src/module.rs`): `spec` (nombre, tipos de problema, campos y opciones que la interfaz dibuja), `template`, `solve` (devuelve un `Report` con resumen, secciones, paso a paso y citas), `read` y `write` (el archivo del módulo), `examples` y `file_tag`. El modelo y las opciones son structs con `serde` y `JsonSchema`, de donde sale el esquema JSON de `iaqsb describir <módulo>`.
- `ejemplos/*.json`: los ejemplos, cargados con `include_str!` en `examples()`. Cada uno necesita su entrada en `fuentes/ejemplos.json`.
- El módulo se registra en `crates/iaqsb-modules/src/lib.rs` (`pub mod` y la lista de `all()`, que da el orden del menú) y, en la aplicación, en los grupos de `apps/desktop/src/components/Sidebar.svelte` y con un ícono en `apps/desktop/src/lib/iconos.ts`. Si tiene una extensión de archivo propia, se asocia en `fileAssociations` de `apps/desktop/src-tauri/tauri.conf.json`.

## 5. Pruebas

Las pruebas de integración de cada paquete se compilan en un solo programa, `tests/todo/main.rs`, que declara un `mod` por archivo:

| Paquete | Dónde | Qué prueba |
| --- | --- | --- |
| `iaqsb-modules` | `crates/iaqsb-modules/tests/todo/<módulo>.rs` | cada módulo contra valores publicados |
| `iaqsb-api` | `crates/iaqsb-api/tests/todo/` | `fuentes.rs`, `idioma.rs`, `winqsb_samples.rs`, `algebraico.rs`, `motor.rs`, `comparar.rs`… |
| `iaqsb-render` | `crates/iaqsb-render/tests/todo/` | gráficas y resumen en Markdown |
| `iaqsb-lp` | `crates/iaqsb-lp/tests/clasicos.rs` | programación lineal y entera |

Además hay pruebas unitarias en `#[cfg(test)] mod tests` dentro de los archivos (por ejemplo, `crates/iaqsb-core/src/i18n.rs`). Para correr una parte:

```sh
cargo test -p iaqsb-modules --test todo its::      # solo ITS
cargo test -p iaqsb-api --test todo fuentes         # solo las fuentes
cargo test -p iaqsb-api --test todo winqsb_samples  # los 64 archivos de WinQSB
cargo test -p iaqsb-core                            # pruebas unitarias del núcleo
```

Qué se espera del código nuevo:

- **Resultados comparados con valores publicados.** Cada método se prueba con un ejemplo de un libro o artículo y su resultado, y la prueba dice de dónde sale (en el nombre o en un comentario: `eoq_hillier`, `descuentos_render`…).
- **O con una implementación independiente** (scipy, numpy, HiGHS u otro programa) sobre modelos aleatorios cuando no hay un valor publicado. Describe en el *pull request* cómo comparaste y cuántos casos, y deja en las pruebas de Rust algunos casos fijos con los valores obtenidos.
- Las simulaciones se comprueban contra la fórmula exacta cuando existe.
- Cada ejemplo nuevo pasa automáticamente por `fuentes.rs` (tiene fuente, cita sus métodos, el resultado publicado aparece en el informe) e `idioma.rs` (el informe sale en inglés con los mismos números).
- Si tocas un formato de WinQSB, `winqsb_samples.rs` tiene que seguir pasando.
- Si cambias la interfaz: `npx svelte-check`, `npm run build` y, si aplica, los scripts de `apps/desktop/tools` (`node tools/test_pegar.ts`, `node tools/test_practica.ts`, `node tools/test_ia.ts`, `node tools/test_ia_ui.mjs`, `node tools/test_excel_ui.mjs`; cada uno dice cómo se usa en su encabezado).

## 6. Estilo del código

- **Español** en comentarios y documentación (`//!`, `///`), nombres de pruebas, archivos de datos y mensajes. Los nombres de los campos del JSON de la API (`name`, `cost`, `problem_type`, `options`…) están en inglés y no se cambian, porque los usan los archivos guardados, la CLI y el MCP. En lo demás, sigue lo que ya use el archivo que editas.
- **Formato.** No hay `rustfmt.toml`, y el código actual no sigue el formato por defecto de `cargo fmt`: `cargo fmt --check` marca diferencias en casi todos los archivos. No ejecutes `cargo fmt` sobre todo el proyecto ni reformatees archivos que no cambias (el diff taparía el cambio real); sigue el estilo del archivo que editas. Adoptar un formato común sería un *pull request* aparte, acordado antes con quienes mantienen el proyecto.
- **Textos para el usuario, en español.** En Rust, los mensajes, títulos, rótulos y errores (por ejemplo, `ModError::Format("no es un archivo de ruteo de IAQSB".into())`) se escriben en español; el núcleo los traduce con los catálogos al responder (`crates/iaqsb-core/src/i18n.rs`). En la interfaz, todo texto visible pasa por `t()` con el texto en español como clave: `t('Resolver')`, `t('Gráfica guardada: {0}', nombre)`.
- Los ids, claves y códigos (`snake_case`, nombres de opciones) no se traducen ni se muestran como texto.
- Los mensajes de `expect`, `panic!` y `assert!` son para desarrolladores y no se traducen.

## 7. Traducciones

IAQSB se escribe en español y se traduce al inglés y al portugués de Brasil al final. Los números no cambian con el idioma.

### Completar el inglés o el portugués

**Núcleo (Rust y datos).** Extrae los textos traducibles y compáralos con los catálogos:

```sh
python3 tools/i18n_extraer.py /tmp/textos.json
python3 -c "
import json
t = json.load(open('/tmp/textos.json'))
for l in ('en', 'pt'):
    c = json.load(open(f'crates/iaqsb-core/i18n/{l}.json'))
    for x in t:
        if x['es'] not in c:
            print(l, x['donde'], repr(x['es']))
"
```

`i18n_extraer.py` saca los literales de `crates/*/src`, `apps/cli`, `apps/mcp` y `apps/desktop/src-tauri/src`, los campos de texto de `crates/iaqsb-core/fuentes/*.json` y los textos de los ejemplos (`**/ejemplos/**/*.json`). Los marcadores de formato de Rust (`{}`, `{x}`, `{:.2}`) quedan como `{0}`, `{1}`… en orden: `format!("Pedir {} unidades cada {:.2} {}", …)` se traduce con la clave `"Pedir {0} unidades cada {1} {2}"`. La lista trae algunos falsos positivos (por ejemplo, los nombres de los meses en inglés de `informe.rs`): no hace falta traducir lo que ya está en el idioma de destino ni lo que no se muestra.

- `crates/iaqsb-core/i18n/en.json` y `pt.json`: texto en español → traducción. Mantén exactamente los marcadores `{0}`, `{1}`… y los saltos de línea.
- `en_extra.json` y `pt_extra.json`: palabras sueltas y trozos con los que se arman frases («día», «semanas», «cercano»…).
- En APA, los títulos de las obras se dejan en su idioma original.

**Interfaz.** Los catálogos son `apps/desktop/src/lib/i18n/en_a.ts` … `en_e.ts` (reunidos en `en.ts`) y `pt.ts` y `pt_b.ts`. Cada texto nuevo que pasa por `t()` necesita su entrada en inglés y en portugués; si falta, la interfaz muestra el español.

Comprueba con `cargo test` (`idioma.rs` y las pruebas de `i18n.rs`) y `npx svelte-check`.

### Agregar un idioma nuevo

Hoy el conjunto de idiomas (español, inglés y portugués) está escrito en varios lugares. Para agregar uno (por ejemplo, francés):

1. **Núcleo, `crates/iaqsb-core/src/i18n.rs`:** una variante en `enum Lang`, y su caso en `Lang::parse`, `Lang::code`, `Lang::ALL`, `warm_up()` y `catalog_for()` (con `include_str!` de los catálogos nuevos). El posprocesado de las citas APA es por idioma (`polish` para el inglés, `polish_pt` para el portugués, elegidos en `translate()`).
2. **Catálogos:** `crates/iaqsb-core/i18n/xx.json` y `xx_extra.json`, con las mismas claves que `en.json` y `en_extra.json`.
3. **APA y fechas:** los `match` sobre `Lang` de `crates/iaqsb-core/src/biblio.rs` («n.d.»/«s.d.», «&», la edición, «Versión») y los nombres de los meses en `today()` de `crates/iaqsb-api/src/informe.rs`.
4. **Interfaz, `apps/desktop/src/lib/i18n.svelte.ts`:** el tipo `Lang`, la lista `IDIOMAS`, `inicial()` (idioma guardado y del sistema), `t()` y `locale()`; los catálogos nuevos en `apps/desktop/src/lib/i18n/`. Otros lugares que comparan `i18n.lang`: `components/RecientesMenu.svelte` y `components/ajustes/AjustesPage.svelte` (formato de fechas, `DESCRIPCION_IDIOMA`), `lib/excel.ts` (`comaInformes()`, coma decimal) y `lib/chat.svelte.ts` (las instrucciones del asistente: `NOMBRE_IDIOMA`, `INSTRUCCION`, `TITULOS`, `PREGUNTA`). Como varios son `Record<Lang, …>`, `npx svelte-check` señala los que falten.
5. **CLI:** `--lang` en `apps/cli/src/main.rs` pasa por `Lang::parse`; actualiza su texto de ayuda («es, en o pt»).
6. **MCP:** en `apps/mcp/src/tools.rs`, la lista `"enum": ["es", "en", "pt"]` del argumento `language` y el texto de las instrucciones que la menciona.
7. **Instalador:** `bundle.windows.nsis.languages` en `apps/desktop/src-tauri/tauri.conf.json` (nombres de idioma de NSIS: hoy `Spanish`, `English`, `PortugueseBR`).
8. **Lenguaje algebraico:** las palabras clave están en `kw()` de `crates/iaqsb-lp/src/algebra/parse.rs` y, con la misma tabla, en `GRUPOS` de `apps/desktop/src/lib/algebra-resaltado.ts` (resaltado del editor). Documéntalas en la sección 12 de `docs/LENGUAJE.md`.
9. **Pruebas y documentación:** una prueba como `idioma.rs` para el idioma nuevo y la mención en `LEEME.md`, `README.md` y `LEIAME.md`.

## 8. Compatibilidad con WinQSB

- **Los archivos que guarda IAQSB tienen que poder abrirse en WinQSB.** Si cambias la lectura o la escritura de un formato de WinQSB, comprueba la ida y vuelta (`winqsb_samples.rs`) y, si puedes, ábrelo en el WinQSB original. El detalle por módulo está en [COMPATIBILIDAD.md](COMPATIBILIDAD.md); actualízalo si cambia algo.
- **No limites el programa a los límites de WinQSB.** Los modelos de IAQSB no tienen el tope de tamaño de WinQSB. Si un modelo es más grande de lo que WinQSB admite, el archivo igual se escribe y ya depende de WinQSB poder abrirlo.
- Los módulos que WinQSB no tiene usan archivos propios de IAQSB con su propia cabecera (por ejemplo, `IAQSB-VRP 1` en `write()` de `crates/iaqsb-modules/src/vrp/mod.rs`). Dentro de un formato de WinQSB no se agregan cosas que WinQSB no entiende: lo que no tiene equivalente se guarda como lo más cercano que WinQSB admite (en `.LPP`, por ejemplo, las variables semicontinuas como continuas) y el modelo completo se puede guardar en otro formato (MPS, LP, JSON, `.xlsx`). [COMPATIBILIDAD.md](COMPATIBILIDAD.md) explica cada caso.
- No copies código de WinQSB: solo se consultó para los formatos, los valores por defecto y la lista de funciones. No agregues más archivos de la distribución de WinQSB a `crates/iaqsb-api/tests/winqsb`.

## 9. Pull requests

1. Un tema por *pull request*, con una descripción de qué cambia y por qué.
2. Llena la plantilla (`.github/pull_request_template.md`): «Qué cambia» y «Cómo se probó».
3. Antes de enviarlo, revisa:
   - [ ] `cargo test` pasa.
   - [ ] Si cambia la interfaz: `npx svelte-check` y `npm run build` en `apps/desktop`.
   - [ ] Los textos nuevos están en español, pasan por `t()` en la interfaz y están traducidos al inglés y al portugués (`apps/desktop/src/lib/i18n/`, `crates/iaqsb-core/i18n/`).
   - [ ] Los métodos y ejemplos nuevos tienen su fuente en APA 7 (`crates/iaqsb-core/fuentes/`), y `docs/FUENTES.md` y `docs/iaqsb.bib` están regenerados con `python3 tools/generar_fuentes.py`.
   - [ ] Las pruebas nuevas comparan con valores publicados o con una implementación independiente, y dicen de dónde sale cada valor.
   - [ ] Los archivos guardados en formato de WinQSB se siguen abriendo en WinQSB.
   - [ ] El diff no reformatea código que no cambió.
   - [ ] Si cambia algo que describe la documentación, están actualizados `LEEME.md` y sus traducciones `README.md` y `LEIAME.md` (y `docs/DOCENTES.md`, `docs/LENGUAJE.md` o `COMPATIBILIDAD.md` si corresponde).

Las contribuciones se publican bajo la [licencia MIT](LICENSE) del proyecto.

## 10. Integridad académica

IAQSB es una herramienta de investigación y de enseñanza, y su valor depende de que se pueda confiar en lo que dice.

- **No inventes** referencias, páginas, ediciones, DOI ni resultados «publicados». Lo que no se pudo comprobar queda `por_verificar` con lo que falta.
- Cita el trabajo ajeno: métodos, ejemplos, datos y código. Usa de los libros solo los datos necesarios para el ejemplo, con su referencia, sin copiar el texto.
- No subas como ejemplo un ejercicio de evaluación que esté en curso, ni datos personales de estudiantes.
- Si usaste un asistente de IA para escribir código, textos o traducciones, revisa el resultado como si fuera tuyo (fórmulas, números, citas) y dilo en el *pull request*.

---

## English summary

IAQSB is written in Spanish first and translated into English and Brazilian Portuguese; issues and pull requests are welcome in any of the three languages.

- **Bugs:** open an issue with the «Error / Bug / Erro» template: module, steps with the model file attached, expected vs. actual result (and the source of the correct value), version and language.
- **Methods and examples** need an APA 7 source in `crates/iaqsb-core/fuentes/bibliografia.json` (`"estado": "verificada"`, or `"por_verificar"` plus `"pendiente"` saying what is missing; never invent pages or editions), an entry in `ejemplos.json` for each example and the citations of each method (`metodos.json` or `Report::cite`). Regenerate the docs with `python3 tools/generar_fuentes.py` (it needs `target/release/iaqsb` or a path to the CLI).
- **Setup:** Rust, Node.js LTS, CMake and clang/LLVM, Python 3. `cargo test`; `cd apps/desktop && npm install && npx tauri dev` (first build and copy the CLI and MCP binaries into `apps/desktop/src-tauri/binaries/` as `scripts/build.sh` does); `npx svelte-check`; `npm run build`. Windows: `scripts\build-windows.ps1`.
- **Tests** compare results with published textbook values or independent implementations; integration tests live in `tests/todo/` of each crate.
- **Style:** Spanish comments and user-facing strings (`t()` in the UI; Rust strings are translated by the catalogs); JSON API field names stay in English. The code base is not `cargo fmt`-formatted, so do not reformat files you do not change.
- **Translations:** `python3 tools/i18n_extraer.py OUT.json` and compare with `crates/iaqsb-core/i18n/en.json` / `pt.json`; UI catalogs in `apps/desktop/src/lib/i18n/`. Section 7 lists every place to change for a new language.
- **WinQSB:** files saved by IAQSB must still open in WinQSB, but IAQSB must not be limited to WinQSB's size limits.
- **Pull requests:** follow `.github/pull_request_template.md` and the checklist in section 9. Contributions are released under the MIT license.

## Resumo em português

O IAQSB é escrito primeiro em espanhol e traduzido para o inglês e o português do Brasil; issues e pull requests são bem-vindos em qualquer um dos três idiomas.

- **Erros:** abra uma issue com o modelo «Error / Bug / Erro»: módulo, passos com o arquivo do modelo anexado, resultado esperado e obtido (e a fonte do valor correto), versão e idioma.
- **Métodos e exemplos** precisam de uma fonte em APA 7 em `crates/iaqsb-core/fuentes/bibliografia.json` (`"estado": "verificada"`, ou `"por_verificar"` com `"pendiente"` dizendo o que falta; nunca invente páginas nem edições), uma entrada em `ejemplos.json` para cada exemplo e as citações de cada método (`metodos.json` ou `Report::cite`). Gere de novo a documentação com `python3 tools/generar_fuentes.py` (precisa de `target/release/iaqsb` ou do caminho da CLI).
- **Ambiente:** Rust, Node.js LTS, CMake e clang/LLVM, Python 3. `cargo test`; `cd apps/desktop && npm install && npx tauri dev` (antes, compile e copie os binários da CLI e do MCP para `apps/desktop/src-tauri/binaries/`, como faz `scripts/build.sh`); `npx svelte-check`; `npm run build`. Windows: `scripts\build-windows.ps1`.
- **Testes** comparam os resultados com valores publicados em livros ou com implementações independentes; os testes de integração ficam em `tests/todo/` de cada crate.
- **Estilo:** comentários e textos para o usuário em espanhol (`t()` na interface; os textos do Rust são traduzidos pelos catálogos); os nomes dos campos do JSON da API ficam em inglês. O código não está formatado com `cargo fmt`, então não reformate arquivos que você não mudou.
- **Traduções:** `python3 tools/i18n_extraer.py SAIDA.json` e compare com `crates/iaqsb-core/i18n/en.json` / `pt.json`; os catálogos da interface estão em `apps/desktop/src/lib/i18n/`. A seção 7 lista todos os lugares a mudar para um idioma novo.
- **WinQSB:** os arquivos salvos pelo IAQSB devem continuar abrindo no WinQSB, mas o IAQSB não deve ficar preso aos limites de tamanho do WinQSB.
- **Pull requests:** siga `.github/pull_request_template.md` e a lista da seção 9. As contribuições são publicadas sob a licença MIT.
