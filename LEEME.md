# Nativo

*Antes IAQSB; «Nativo» es un nombre provisional. El comando de la línea de comandos sigue siendo `iaqsb`.*

Para aprender y aplicar la ingeniería industrial, con IA. Hecho por IA Nativa.

Código: <https://github.com/vangoghdata/iaqsb> · Instaladores: <https://github.com/vangoghdata/iaqsb/releases> · [Cómo instalar](INSTALAR.md)

Software libre (licencia MIT) en español, inglés y portugués, pensado para estudiantes, profesores y profesionales de Latinoamérica: del primer curso de pregrado hasta la tesis de doctorado y el trabajo en la empresa. *[English: README.md](README.md) · [Português: LEIAME.md](LEIAME.md)*

| Documento | Para qué |
| --- | --- |
| [INSTALAR.md](INSTALAR.md) | Cómo instalar IAQSB en Windows, macOS (chip Apple o Intel) y Linux |
| [docs/DOCENTES.md](docs/DOCENTES.md) | Guía para docentes: cómo usar IAQSB en pregrado, maestría y doctorado, actividades y salas de cómputo |
| [docs/LENGUAJE.md](docs/LENGUAJE.md) | Referencia del lenguaje de modelado algebraico (con 29 modelos probados) |
| [docs/CURSOS.md](docs/CURSOS.md) | Cursos (`.ncourse`): cómo crear, compartir e instalar temas propios, a mano, con el asistente de creación de la app o con agentes de IA |
| [docs/FUENTES.md](docs/FUENTES.md) | Bibliografía en APA 7 y origen de cada ejemplo |
| [COMPATIBILIDAD.md](COMPATIBILIDAD.md) | Compatibilidad con los archivos de WinQSB, módulo por módulo |
| [CONTRIBUTING.md](CONTRIBUTING.md) | Cómo reportar errores, proponer mejoras, traducir y contribuir código |
| [CITATION.cff](CITATION.cff) | Cómo citar IAQSB |

IAQSB es un programa para aprender y resolver problemas de ingeniería industrial: investigación de operaciones, teoría de decisiones, producción, logística, inventarios, pronósticos y calidad. Cada tema trae una guía (qué es, para qué sirve en la vida real, su historia con fuentes, cómo usarlo y cómo se hace hoy en la industria), ejemplos de libros y de cursos, y un motor de cálculo moderno (HiGHS, Gurobi o CPLEX si están, R y Python) que muestra cada solución paso a paso. Está pensado para que alguien pueda estudiar un tema solo con IAQSB y para que los profesores lo usen en clase.

Nació para reemplazar programas que se seguían enseñando aunque ya no corren en los computadores actuales, como WinQSB: abre y guarda sus archivos, pero no es una copia. Hoy trae 28 módulos (optimización, redes, proyectos, decisiones, colas y simulación, inventarios, pronósticos y series de tiempo, planeación de la producción, MRP, programación de trabajos, localización, calidad y logística) y un asistente de IA, Nativo, que plantea, resuelve y explica con el mismo motor. Se usa de tres maneras, con el mismo motor de cálculo:

- **Aplicación de escritorio** (Tauri 2 + Svelte) para estudiantes y profesores, con un diseño inspirado en macOS: letra del sistema (SF Pro en Mac, Inter en Windows), colores del sistema de Apple, temas claro, oscuro, automático y personalizado. La barra lateral de módulos aparece al llevar el mouse al borde izquierdo (o con Ctrl+K) y se esconde sola; se puede fijar. Las opciones del cálculo van en un panel «Opciones» para no quitarle espacio a los resultados. Cada gráfica tiene botones para ampliarla (visor a pantalla completa con zoom y arrastre), copiarla como imagen y descargarla en PNG o SVG, y los gráficos muestran el valor de cada serie al pasar el mouse. Al entrar a una celda se selecciona su valor, así que lo que se escribe lo reemplaza.
- **Servidor MCP** (`iaqsb-mcp`), para que Claude u otro asistente compatible plantee y resuelva problemas.
- **Línea de comandos** (`iaqsb`), con salida JSON para scripts y agentes de IA sin MCP.

## Módulos

Cada módulo tiene su guía (botón **Guíame**). La columna «WinQSB» dice qué programa viejo reemplaza y qué archivos abre; «Nuevo» son módulos que WinQSB no tenía. Cada informe trae un resumen, resultados estructurados, tablas y gráficos, y una pestaña **Paso a paso** con las fórmulas aplicadas a los datos.

| WinQSB | IAQSB | Lo principal |
| --- | --- | --- |
| LP-ILP | Programación lineal y entera | Símplex con tablas, branch and bound, sensibilidad, método gráfico; modelos de cualquier tamaño con el motor (HiGHS, Gurobi, CPLEX, SCIP, CBC o GLPK), variables semicontinuas y semienteras, varios objetivos con prioridades, diagnóstico de infactibilidad (IIS), archivos MPS y LP |
| NET | Modelos de redes | Flujo de costo mínimo (transbordo) con costo y capacidad en los nodos y demandas con mínimo y máximo, transporte, asignación, ruta más corta, flujo máximo, árbol de expansión, agente viajero; modelo matemático de cada red y botón para abrirlo como programa lineal |
| PERT-CPM | PERT y ruta crítica | CPM, PERT con probabilidades, compresión de costo mínimo, Gantt, simulación |
| GP-IGP | Programación por metas | Metas con prioridades y pesos, enteras, símplex de metas |
| QP-IQP | Programación cuadrática | Convexa (Clarabel), no convexa global, entera |
| NLP | Programación no lineal | Con y sin restricciones, varios arranques, condiciones KKT |
| DP | Programación dinámica | Diligencia, mochila, producción e inventario |
| AP | Planificación agregada | Estrategias de producción, inventario, horas extra y subcontratación, por PL y transporte |
| DA | Análisis de decisiones | Tablas de pagos (7 criterios, VEIP, VEIM), Bayes, juegos de suma cero, árboles |
| MKP | Procesos de Markov | Clases, estado estable, absorción, primera pasada, evolución |
| ITS | Inventarios | EOQ y POQ (con días hábiles, tasa de producción diaria y H = i·C), descuentos (por % o por precio de cada rango), vendedor de periódicos, lotes dinámicos (10 métodos), sistemas (s,Q), (s,S), (R,S), (R,s,S) |
| FC | Pronósticos y regresión | 12 métodos de series de tiempo con búsqueda de parámetros, regresión múltiple |
| QA | Teoría de colas | M/M/s exacto con capacidad y población finitas, lotes, presión y desaliento; M/G/1, G/G/s, simulación, costos, capacidad, sensibilidad |
| QSS | Simulación de colas | Poblaciones, servidores, colas y recolectores; 8 reglas de entrada (ensamble) y 8 de salida (desensamble), 10 disciplinas, bloqueo, lotes |
| JOB | Programación de trabajos | 15 reglas de despacho, Johnson, CDS, Palmer, NEH y otras heurísticas de flujo, 18 criterios, Gantt |
| MRP | Planificación de requerimientos | Explosión por nivel bajo, 10 reglas de lote, desperdicio, capacidad, costos, árbol de materiales |
| FLL | Localización y distribución | Localización rectilínea, euclidiana y cuadrada; distribución tipo CRAFT; balanceo de líneas (10 reglas, óptimo, COMSOAL) |
| QCC | Gráficos de control de calidad | 36 gráficos (Shewhart, CUSUM, EWMA, medias móviles, tendencia, modificados, de aceptación, estandarizados), reglas, curva OC, capacidad, histograma con ji cuadrada, probabilidad, Pareto |
| ASA | Muestreo de aceptación | Simple, doble, múltiple, secuencial, ChSP-1, CSP-1, SkSP-2; por variables (media, métodos k y M, secuencial); curvas OC, AOQ, ATI, ASN y costo; búsqueda del plan |

Módulos de logística que agrega IAQSB (no existen en WinQSB):

| Módulo | Lo principal |
| --- | --- |
| Ruteo de vehículos (VRP) | Depósito, clientes con demanda y vehículos con capacidad (y, si se quiere, distancia máxima, flota limitada y costos). Barrido (con ángulo y sentido, o probando todos los inicios), ahorros de Clarke y Wright, vecino más cercano y óptimo por programación entera; mejora 2-opt; comparación de métodos; mapa de rutas con las zonas del barrido. Clientes por coordenadas (distancia euclidiana o rectilínea) o red de calles (la tabla se completa con la ruta más corta) |
| Gestión de inventarios | EOQ y POQ al revés (despeja en cadena lo que falte a partir de lo observado y comprueba todas las relaciones), clasificación ABC con Pareto (cortes por quiebre de pendiente o por % acumulado), punto de reorden con demanda variable (continuo y periódico, Z exacto o de tabla) y simulación día a día de la política (Q, R) con ventas perdidas o back orders, escenarios, gráfica del inventario y búsqueda de la mejor combinación |
| Finanzas del inventario | ROI con escenarios y reducción de inventario, liquidez (razón corriente, prueba ácida, inventario obsoleto) y capital de trabajo (ciclo operativo, desfase, utilidad por ciclo; también al revés) con línea de tiempo |
| Almacenes | Paletizado (cajas por tendido, tendidos por altura, montacargas y resistencia), estibas por lote, bodega en bloque de área mínima con plano a escala, y dimensionado del almacén propio o rentado (costo mes a mes, alternativas, curva en V y óptimo por análisis marginal; amortización simple o con factor A/P) |
| Indicadores logísticos | A tiempo, completos, fill rate, ciclo de la orden, documentación, pedido perfecto, OTIF, urgentes frente a estándar (prueba t de Welch) y costo logístico sobre ventas frente al estándar |
| Selección del modo de transporte | Trade-off transporte-inventarios (Ballou): flete, inventario en la planta, en tránsito y en el centro de distribución; gráficos de composición y de trade-off, costo según la demanda y demanda en la que dos medios cuestan lo mismo; ventajas y desventajas de cada modo |

Módulos de planificación de la producción que agrega IAQSB:

| Módulo | Lo principal |
| --- | --- |
| Programa maestro (PMP) | Básico y de ensamble bajo pedido (ATO): barreras de tiempo de demanda y de planificación, disponible para promesa discreto y acumulado, respuesta a solicitudes de pedidos (si se aceptan, cuánto se puede prometer, desde cuándo completas y cómo queda el PMP sin tocar lo congelado), plan aproximado de capacidad (RCCP) y PMP óptimo por programación entera con el motor, comparado con la regla del curso |
| MRP II por matrices (Gozinto) | Matriz de requerimientos totales (o la estructura directa, con LM = (I − N)⁻¹), requerimientos brutos y netos, demanda neta, necesidades a programar con los sobrantes repartidos en el horizonte, tiempos de máquina frente a la capacidad de cada departamento, puntos de reorden y ajuste por calidad |
| Series de tiempo (ARIMA, ETS, TBATS) | Exploración (autocorrelación y autocorrelación parcial con bandas, Ljung-Box, descomposición clásica, fuerza de la tendencia y de la estacionalidad, gráfico estacional, λ de Box-Cox) y pronósticos: ARIMA automático (Hyndman-Khandakar) o con órdenes dados, ETS, TBATS (varias estacionalidades y no enteras), Theta, Croston (demanda intermitente) e ingenuos, con intervalos, medidas de error como `accuracy()` y revisión de residuos; en modo automático compara los modelos en un periodo de prueba y elige por MASE. **Evaluación como en clase con R:** entrenamiento hasta un periodo (`window()`), el autocorrelograma y la suavización que sugiere, varios modelos (`ses`, `holt`, `hw`, `naive`, `snaive`, `tbats`, `auto.arima`…), `accuracy()` en entrenamiento y prueba y `checkresiduals()`. **Da los mismos números que R:** usa el R del equipo con el paquete forecast; si no hay, la app de escritorio trae R (webR: el R oficial compilado a WebAssembly, con forecast) y lo usa sin instalar nada; si tampoco, Python con statsforecast o el motor nativo en Rust. El motor nativo trae `ets()` traducido de forecast (ses, holt, hw y ETS dan lo mismo que R) y anofox-forecast para ARIMA, TBATS y Theta. La exploración y los coeficientes ARIMA coinciden con R |

Los ejercicios de las clases de transporte, distribución y ruteo vienen como ejemplos (menú Ejemplos): transbordo casos 1, 2 y 3 y el problema 1, ruta más corta (con el modelo binario), árboles de expansión, flujo máximo, agente viajero de 4 y 6 nodos, la red de bebidas para el vecino más cercano y la inserción más barata, el barrido del almacén de alimentos frescos y el trade-off tren, camión o avión. También los de inventarios y almacenes: Ferretería Nacional (EOQ y POQ), el té y la solución de limpieza (Schroeder), los descuentos de tres rangos, la tarea del POQ, el ABC de 15 artículos, Sell-Rite, los problemas del parcial, el ROI, la liquidez y el capital de trabajo, la planta envasadora, la compañía química y los talleres 1, 2 y 3 del Grupo 8. Y los de programa maestro y MRP II del profesor Arango: el programa maestro básico, el de ensamble bajo pedido, el producto A con sus tres solicitudes de pedidos y los estantes (Gozinto); el ejercicio MRP de siete artículos de su hoja de cálculo se resuelve igual que su solución en el módulo MRP. Y los de pronósticos con los datos de fpp2 que usa en R: usnetelec, h02 (con λ = 0), qcement (ETS frente a ARIMA) y gas (TBATS), más las utilidades trimestrales de su hoja de autocorrelación.

## Herramientas de software profesional

Lo que se espera de un programa de optimización profesional, con la misma interfaz para aprender y para investigar:

- **Lenguaje de modelado algebraico** (como AMPL, GAMS o LINGO), con palabras clave en español, inglés o portugués: conjuntos, parámetros con tablas de datos, variables indexadas, sumatorias con condiciones y restricciones generales que se linealizan solas (valor absoluto, máximo y mínimo, funciones lineales por tramos, y/o/no, implicaciones, disyunciones, «a lo sumo k de…»), objetivo cuadrático y varios objetivos por prioridad. El editor revisa el modelo mientras se escribe (errores con línea y columna), resalta la sintaxis, trae ejemplos y muestra la solución en tablas por índices. La hoja de LP-ILP se pasa al lenguaje con un botón. Un objetivo cuadrático no convexo o con variables enteras se resuelve con la búsqueda global del módulo QP-IQP. Referencia completa: [docs/LENGUAJE.md](docs/LENGUAJE.md).
- **Informes para entregar** en Word (.docx), PDF, HTML, Excel o Markdown: portada (autor, curso, institución), formulación, resultados, tablas y figuras numeradas al estilo APA 7, paso a paso opcional y lista de referencias con sangría francesa. El PDF lo arma IAQSB (no hace falta tener Word ni LibreOffice).
- **Excel:** pegar una tabla copiada de Excel o de Google Sheets en cualquier hoja (Ctrl+V), y abrir y guardar libros .xlsx. Un libro guardado por IAQSB trae el modelo completo y se vuelve a abrir tal cual; uno hecho a mano se lee como datos del módulo abierto.
- **¿Qué pasa si…?** Deslizadores para cambiar un dato y ver la solución al instante, barrido de un dato con la gráfica del objetivo, y análisis paramétrico exacto de LP (el objetivo como función de un costo o de un lado derecho, por tramos, con los puntos donde cambia la base).
- **Modo práctica:** el estudiante hace los pivotes del símplex y las ramificaciones del branch and bound, y IAQSB revisa cada paso, da pistas y muestra el resumen al final.
- **Ajuste automático del motor** (como el *tuning* de Gurobi o CPLEX): prueba varias configuraciones en el modelo abierto, comprueba que den el mismo resultado y propone la más rápida (solo si mejora al menos un 10 %).
- **Deshacer y rehacer** (Ctrl+Z, Ctrl+Y), **autoguardado** del trabajo en el equipo con recuperación al volver a abrir, y **archivos recientes**.
- **Editor visual de redes:** nodos y arcos se agregan, mueven y conectan con el mouse; las coordenadas geográficas se ven en un **mapa real** (OpenStreetMap), y el ruteo de vehículos calcula distancias en km con la fórmula del haversine.
- **Adjuntar ejercicios al asistente:** fotos o capturas del enunciado (también pegadas con Ctrl+V o arrastradas al chat), PDF, tablas de Excel, CSV y archivos de WinQSB o de IAQSB. Las imágenes se reducen solas; los PDF van como documento a los modelos que los leen; los archivos de modelos llegan ya importados, listos para resolver o abrir en el editor. El asistente dice qué datos leyó para que los revises antes de resolver.
- **Cadena jerárquica de planificación:** el plan agregado se desagrega en programas maestros por producto, el PMP pasa al MRP y las órdenes planeadas del MRP dan la carga de cada centro de trabajo (CRP) (operación `link` del núcleo; herramienta `link_models` del MCP y del asistente).
- **Cursos que crea cualquiera (`.ncourse`):** un profesor, un estudiante o una IA arma un curso con lecciones, calculadoras en Python o R y modelos de optimización en el lenguaje algebraico; se comparte como un archivo y quien lo instala ve sus temas en la barra lateral como módulos, con su pantalla de datos, informe, paso a paso y guía. El código de los cursos corre aislado dentro de la app. Se crean dentro de la app con el **asistente de creación** (**Crear un curso…**: le das tus diapositivas, libros o programas y Nativo escribe las lecciones, la guía, la pantalla de datos y el cálculo, lo valida, prueba los ejemplos y te muestra la vista previa antes de exportarlo), con `iaqsb curso nuevo` o pidiéndoselo a Claude Code o Codex con la skill `skills/crear-curso-nativo`; el formato está en [docs/CURSOS.md](docs/CURSOS.md) y el curso de muestra en `cursos/ejemplo-nativo`.
- **Guíame en cada módulo, para estudiar o enseñar solo con IAQSB:** el botón **Guíame** explica qué es el tema, para qué sirve en empresas reales, su historia con bibliografía en APA 7, qué datos necesita, cómo usar el módulo paso a paso, cómo lo resuelve el motor, cómo leer los resultados, qué software y librerías se usan hoy en la industria (SAP, Oracle, Gurobi, OR-Tools, R, Python…), alternativas modernas, errores comunes, objetivos y actividades para el profesor y un glosario. Están las 28 guías (`crates/iaqsb-core/guias/`); la IA las pide con `get_module_guide` (MCP) y la CLI con `iaqsb guia <módulo>`.
- **Nativo, el asistente animado de IA Nativa:** piensa, calcula con el motor, explica y celebra mientras responde (animaciones hechas con Blender; se apagan en Ajustes → Apariencia). Su tono es profesional y educativo.
- **Asistente de IA dentro de la app, con el modelo que elijas:** un chat que ve el problema abierto y puede resolver, comparar y explicar con el mismo motor, y el botón **Explicar** en cada resultado. En **Ajustes → Inteligencia artificial → Conectar la API** se elige el proveedor: OpenRouter (cientos de modelos con una sola clave: GPT, Gemini, Claude, Qwen, DeepSeek, Llama, Mistral…), Anthropic (Claude), OpenAI (ChatGPT), Google Gemini, Qwen (Alibaba Cloud), DeepSeek, Mistral, Groq, xAI (Grok), Moonshot (Kimi), modelos gratis en el propio equipo con Ollama o LM Studio, o cualquier servidor compatible con la API de OpenAI (por ejemplo, el vLLM de una universidad). La lista de modelos se trae del proveedor, se puede probar la conexión y se guarda una clave por proveedor, solo en el equipo. Con los modelos que usan herramientas, el asistente resuelve con el motor de IAQSB; si un modelo no las admite, lo avisa y responde sin ellas. En la app de escritorio las peticiones las hace Rust, así que funciona con cualquier dirección (también servidores locales). El nivel (pregrado o posgrado) cambia cómo explica, y el **nivel de razonamiento** (de bajo a máximo) cuánto piensa el modelo antes de responder. El asistente **ejecuta R y Python dentro de la app** (webR y Pyodide, aislados, sin instalar nada): los ejercicios escritos para R (`ses`, `tbats`, `accuracy`, `read_excel`…) dan exactamente los números de R, con sus gráficas, y los archivos adjuntos (Excel, CSV) quedan en la carpeta de trabajo. **Las conversaciones se guardan solas en el equipo**: siguen ahí al cerrar la app y se retoman desde el historial.
- **Ajustes** (Ctrl+,): idioma, nivel, tema, decimales y separador decimal (coma o punto), motor y sus parámetros, autoguardado, archivos recientes, datos de la portada de los informes, conexión del asistente de IA (proveedor, clave y modelo), atajos de teclado y cómo citar IAQSB.

## Archivos de WinQSB

IAQSB abre y guarda los archivos de los 19 módulos en el formato de WinQSB, que el programa original puede volver a abrir. Los formatos se comprobaron con los **64 archivos de ejemplo que trae WinQSB 2.0**: todos se leen, se resuelven, se guardan y se vuelven a leer con el mismo resultado (`crates/iaqsb-api/tests/todo/winqsb_samples.rs`). El detalle por módulo está en [COMPATIBILIDAD.md](COMPATIBILIDAD.md).

## Motor de optimización

Todo lo que se resuelve con programación lineal o entera (LP-ILP, metas, transporte, asignación, planificación agregada, ruteo óptimo, balanceo de líneas, distribución en planta…) pasa por un solo motor, `iaqsb_core::solver`. Por defecto es **HiGHS** (incluido, libre, sin límite de tamaño): símplex dual y primal, punto interior, PDLP y ramificación y corte con presolve, cortes y heurísticas. Si en el equipo están instalados **Gurobi, CPLEX, SCIP, CBC o GLPK** (el programa o su paquete de Python: `gurobipy`, `cplex`, `pyscipopt`), IAQSB los detecta y los usa cuando se piden; la licencia es la de cada usuario. IAQSB no trae Gurobi ni CPLEX porque son comerciales, y no promete su velocidad: en problemas enteros difíciles suelen ser más rápidos que HiGHS, y para eso está el conector.

- **Sin límites heredados.** WinQSB limita el tamaño de los modelos; IAQSB no. Un transporte de 200 × 300 (60 000 variables) se resuelve en una fracción de segundo. Lo único que se mantiene es que los archivos que guarda se puedan abrir en WinQSB: si el modelo es más grande de lo que WinQSB admite, el archivo igual se escribe (en su forma algebraica) y ya depende de WinQSB poder abrirlo.
- **Parámetros como en Gurobi:** motor (`solver`), tiempo máximo (`time_limit`), brecha MIP (`mip_gap`, `mip_abs_gap`), hilos (`threads`), presolve, método (`auto`, `dual_simplex`, `primal_simplex`, `barrier`, `pdlp`), semilla, límites de nodos y de soluciones, tolerancias, registro del motor (`log`) y solución inicial (`start`, el *MIP start*). Si se acaba el tiempo, se devuelve la mejor solución encontrada con su cota y su brecha.
- **Estadísticas:** cada informe dice qué motor resolvió, en cuánto tiempo, el tamaño del modelo, los nodos, la cota y la brecha.
- **Sensibilidad del motor con la semántica de WinQSB:** precios sombra, costos reducidos y rangos de c(j) y del lado derecho, comprobados contra el símplex de tablas con 40 modelos aleatorios.
- **Por qué no es factible:** si un modelo no tiene solución, el informe trae el subsistema irreducible infactible (IIS: las restricciones y cotas que no se pueden cumplir a la vez; basta relajar una para romper el conflicto) y la relajación de factibilidad (el menor cambio de los lados derechos que lo vuelve factible).
- **Varios objetivos** por prioridades, con tolerancia de degradación por nivel (como `setObjectiveN` de Gurobi); **variables semicontinuas y semienteras**; **restricciones dispersas** (`terms`) para modelos grandes.
- **Formatos de la industria:** abre y guarda MPS y LP (el de CPLEX), además de .LPP de WinQSB y JSON. Un modelo demasiado grande para la hoja se queda como texto y se resuelve directamente.
- **Lo didáctico se mantiene:** con los valores predeterminados, los modelos pequeños de LP-ILP se resuelven con el símplex de tablas y el branch and bound de WinQSB (para ver el paso a paso) y el resultado se comprueba con el motor; al pedir un motor, un tiempo, una brecha o hilos, se usa el motor.

Comparación con el símplex de tablas de IAQSB (el mismo método de WinQSB), en problemas de transporte aleatorios, en este equipo de pruebas (2 núcleos; `cargo run --profile pruebas -p iaqsb-lp --example banco`):

| Modelo | Variables × restricciones | Símplex de tablas | Motor (HiGHS) | Mismo óptimo |
| :-- | --: | --: | --: | :-: |
| Transporte 20 × 30 | 600 × 50 | 0.025 s | 0.002 s | sí |
| Transporte 40 × 60 | 2 400 × 100 | 0.835 s | 0.008 s | sí |
| Transporte 60 × 90 | 5 400 × 150 | 7.08 s | 0.011 s | sí |
| Transporte 200 × 300 | 60 000 × 500 | — | 0.24 s | — |

En entera, una localización de 30 plantas y 200 clientes (6 030 variables, 30 binarias) llega a una brecha de 0.08 % en unos 8 s, y en una de 12 plantas y 60 clientes (732 variables) los seis motores (HiGHS, Gurobi, CPLEX, SCIP, CBC y GLPK) dan el mismo óptimo.

En la aplicación, el botón del motor (barra de arriba) elige el motor y sus parámetros para todos los módulos, y en LP-ILP tiene el **ajuste automático** para el modelo abierto. En la CLI: `iaqsb motores`, `iaqsb resolver modelo.mps --motor gurobi --tiempo 60 --brecha 0.01 --hilos 4 --registro`, `iaqsb convertir modelo.lp modelo.LPP` y `iaqsb diagnosticar modelo.lp`. En MCP: `solver_info`, `diagnose_infeasibility`, `convert_model` y los parámetros en `options` (LP-ILP) u `options.engine` (los demás módulos); `solve` acepta `{"format": "lp" | "mps", "text": "…"}` como modelo.

## Fuentes

IAQSB es una herramienta de investigación: cada método, fórmula, paso y ejemplo dice de dónde sale, en **APA 7.ª edición**.

- Cada informe tiene una pestaña **Fuentes** con el origen de los datos (libro, artículo, diapositiva de clase u original de IAQSB, con capítulo o diapositiva), los métodos y fórmulas usados con su referencia, y la lista de referencias con su DOI. Se puede copiar en APA o en BibTeX.
- El menú **Ejemplos** muestra la fuente de cada ejemplo.
- Lo que viene del curso Taller de Producción y Logística (Ingeniería Industrial, Universidad Nacional de Colombia) se cita como Sarache (2026a, b, c…); si la diapositiva nombra un libro (Bronson, Ballou, Taha), se cita también.
- Cada referencia está **verificada** (DOI, editor o catálogo) o **por verificar**, y en ese caso se dice qué falta. No se inventan páginas ni ediciones.
- La bibliografía está en `crates/iaqsb-core/fuentes/bibliografia.json` y el origen de los datos de cada ejemplo en `crates/iaqsb-core/fuentes/ejemplos.json` y qué métodos cita cada módulo en `crates/iaqsb-core/fuentes/metodos.json`. [docs/FUENTES.md](docs/FUENTES.md) y [docs/iaqsb.bib](docs/iaqsb.bib) se generan con `python3 tools/generar_fuentes.py`.
- CLI: `iaqsb fuentes`, `iaqsb fuentes --bibtex`, `iaqsb fuentes --pendientes`. MCP: `get_bibliography`; cada `solve` trae `references` y `data_source`.
- Las pruebas (`crates/iaqsb-api/tests/todo/fuentes.rs`) exigen que cada ejemplo de todos los módulos tenga fuente, que cada informe cite sus métodos, que cada cita exista en la bibliografía y que el resultado publicado en la fuente se reproduzca.

Todos los módulos y todos los ejemplos están documentados. Los ejemplos tomados de libros dicen capítulo y sección; los que siguen por verificar dicen qué falta (casi siempre, confirmar la página en un ejemplar impreso).

## Cómo se verificó

Los algoritmos se escribieron a partir de libros de texto (Hillier y Lieberman, Taha, Render y Heizer, Montgomery, Winston) y de bibliotecas actuales (HiGHS, Clarabel, nalgebra, statrs). El código de WinQSB solo se consultó para los formatos de archivo, los valores por defecto y la lista de funciones; las fórmulas de colas, simulación, control de calidad y muestreo se tomaron de su ayuda.

- **233 pruebas automáticas** (`cargo test`), con resultados publicados en los libros: Wyndor, Goferbroke (VEIP 142.5), Parque Seervada, las cámaras de Hillier, la sala de emergencias (L = 2), la tarjeta de circuitos de Montgomery (c̄ = 19.85, LCS = 33.22), las latas de jugo (p̄ = 0.2313), las rectas de Wald (h1 = 1.22, h2 = 1.57), las constantes d2, d3 y c4, entre muchos otros.
- **Comparaciones con implementaciones independientes** en Python (scipy, numpy, HiGHS), con cientos de modelos aleatorios por módulo y 0 diferencias: 400 programas lineales, 240 cuadráticos, 300 de metas, 645 de redes, 300 de PERT, 180 de programación dinámica, 60 colas contra una cadena de Markov completa, 932 comprobaciones de muestreo de aceptación, los 36 gráficos de control, 300 redes de transbordo con costo y capacidad en los nodos (contra scipy), 80 problemas de ruteo contra la enumeración de todas las rutas, entre otros.
- **Pruebas de la interfaz** (`apps/desktop/tools`): 593 comprobaciones del modo práctica contra el núcleo, 84 del pegado desde Excel, 29 de punta a punta en el navegador (pegar, exportar el informe en los cinco formatos, copiar tablas e ida y vuelta por Excel), 47 del formato de los proveedores de IA y 95 de punta a punta del asistente (también los adjuntos, R dentro de la app y el historial de conversaciones) contra servidores simulados (API compatible con OpenAI y API de Anthropic; herramientas, lista de modelos, prueba de conexión, modelo sin herramientas, clave equivocada y detener), en el navegador y simulando la app de escritorio. Además, `cargo test` en `apps/desktop/src-tauri` prueba las peticiones de red hechas desde Rust.
- Las simulaciones (colas, PERT) se comprobaron contra las fórmulas exactas cuando existen.
- El servidor MCP se probó con MCP Inspector.

## Estructura

```text
crates/iaqsb-core      tipos del informe, especificación de la interfaz, lectura y escritura de WinQSB
crates/iaqsb-lp        programación lineal: símplex, branch and bound, sensibilidad, método gráfico
crates/iaqsb-modules   los otros 18 módulos y los de logística (un directorio por módulo, con sus ejemplos)
crates/iaqsb-formats   archivos .LPP
crates/iaqsb-api       catálogo de módulos y llamadas JSON; lo usan las tres aplicaciones
crates/iaqsb-render    gráficas de los informes en SVG y PNG y resumen en Markdown para la IA
apps/cli               iaqsb (línea de comandos)
apps/mcp               iaqsb-mcp (servidor MCP por stdio)
apps/desktop           aplicación de escritorio: Svelte + Tauri 2
scripts                compilación e instaladores
```

## Compilar en Windows

```powershell
powershell -ExecutionPolicy Bypass -File scripts\build-windows.ps1
```

El script revisa que estén instalados Rust (toolchain MSVC), Node.js LTS, Microsoft C++ Build Tools, CMake y LLVM, y dice cómo instalar lo que falte:

```powershell
winget install Rustlang.Rustup
winget install OpenJS.NodeJS.LTS
winget install Microsoft.VisualStudio.2022.BuildTools --override "--wait --passive --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"
winget install Kitware.CMake
winget install LLVM.LLVM
```

Después de instalar, cierra PowerShell, abre una ventana nueva y vuelve a ejecutar el script. Compila la CLI y el servidor MCP, los incluye en el instalador y genera un solo instalador, `IAQSB_0.2.1_x64-setup.exe` (NSIS), en `target\release\bundle\nsis\`. Con `-Pruebas` ejecuta antes las pruebas. El instalador lleva el logo de IAQSB (ícono, cabecera y panel lateral, generados con `python3 tools/instalador_imagenes.py`) y deja elegir español, inglés o portugués. Los archivos `.iaqsb` (modelos algebraicos) y los de WinQSB se abren con doble clic.

En Linux o macOS: `scripts/build.sh` (hacen falta además `cmake` y `clang`; `--pruebas` ejecuta las pruebas).

**Por qué tarda y qué se hizo.** No es por Rust ni por Tauri en sí: la primera compilación construye HiGHS desde su código en C++ (con CMake), unas 500 dependencias de Rust (Tauri y su sistema de ventanas, el renderizado de gráficas, Clarabel, nalgebra…) y el enlace final con optimización entre módulos (LTO). Para que tarde lo menos posible:

- Todo comparte una sola carpeta `target/` (la CLI, el servidor MCP y la aplicación), así que lo que ya se compiló para uno no se vuelve a compilar para otro.
- Las pruebas de cada paquete van en un solo programa (antes eran 28 programas, cada uno enlazado aparte).
- `cargo test` usa un perfil de desarrollo: el código propio se compila rápido y por partes, y las dependencias, que se compilan una sola vez, van optimizadas, así que las pruebas numéricas corren casi a la velocidad final. `cargo test --profile pruebas` las corre optimizadas y sin LTO.
- La versión final usa LTO «thin» y 16 unidades en paralelo (casi la misma velocidad que el LTO completo, con mucho menos tiempo de enlace).
- La aplicación de escritorio se compila solo como biblioteca de Rust (sin las variantes para móviles, que triplicaban el enlace).
- El instalador ya no ejecuta las pruebas por defecto.

Después de la primera vez, un cambio en el código propio recompila solo lo que cambió.

Para trabajar en la interfaz con recarga en caliente: `cd apps/desktop`, `npm install` y `npx tauri dev` (antes, compila la CLI y el servidor MCP y cópialos a `apps/desktop/src-tauri/binaries/` como lo hace `scripts/build.sh`; los detalles están en [CONTRIBUTING.md](CONTRIBUTING.md)).

## Conectar con Claude

Tras instalar, abre IAQSB y pulsa **Conectar con Claude**: muestra la configuración exacta con la ruta de tu instalación. Es este bloque en `claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "iaqsb": { "command": "C:\\Users\\<usuario>\\AppData\\Local\\IAQSB\\iaqsb-mcp.exe" }
  }
}
```

Herramientas: `list_modules`, `describe_module`, `solve`, `compare_methods`, `get_report_part`, `graphic_method`, `import_winqsb_file`, `export_winqsb_file`, `get_bibliography`, `get_example`, `solver_info`, `diagnose_infeasibility`, `convert_model`, `check_model` (revisa un modelo del lenguaje algebraico), `what_if` (barrido de un dato), `parametric_analysis`, `tune_parameters`, `export_report` (Word, PDF, HTML), `export_excel`, `import_excel` y `link_models` (cadena plan agregado → programa maestro → MRP). `solve` acepta modelos del lenguaje algebraico: `{"format": "iaqsb", "text": "…"}`. Los ejemplos se publican también como recursos MCP, y el prompt `resolver_problema` guía a Claude para plantear un problema escrito en palabras.

**Pensado para la IA.** Las respuestas vienen en Markdown y son cortas a propósito: el resultado, las métricas, las tablas principales, las gráficas descritas con sus números (los arcos del recorrido, los valores de cada serie, las rutas) y las fuentes. Lo grande no se manda si no se pide: el paso a paso, las tablas largas y el modelo lineal equivalente (`data.lp_model`). Cada informe trae un `report_id`, y con `get_report_part` se pide una sección, un paso (`step`) o un dato (`path`, por ejemplo `data.lp_model`). `detail` elige cuánto: `summary` (por defecto), `steps` (por defecto cuando `options.steps` es `true`), `full` o `json` (el informe entero). `describe_module` devuelve el esquema y un solo ejemplo, del tipo de problema pedido (`problem_type`). Un agente viajero de 10 nodos, que antes devolvía unos 200 KB, ahora devuelve unos 3 KB.

**Gráficas como imágenes.** Con `"images": true`, `solve`, `compare_methods` y `get_report_part` devuelven además las gráficas en PNG (redes, recorridos, gráficos, diagramas de Gantt, mapas de rutas, planos de bodega y árboles), cada una con un rótulo que dice de qué parte del informe sale. Con `"options": {"steps": true}` se incluyen las del paso a paso: en el agente viajero, el recorrido parcial después de cada iteración del vecino más cercano (NNH) o de la inserción más barata (CIH). `max_images` limita cuántas (12 por defecto). La letra va dentro del programa, así que las imágenes salen iguales en cualquier equipo.

**Comparar métodos.** `compare_methods` resuelve el mismo modelo con cada valor de la opción de método del tipo de problema y devuelve una tabla con todas las métricas, la mejor (y los empates), la diferencia de cada una con la mejor, un gráfico de barras y la solución resumida de cada variante. Sirve en todos los módulos que tienen varios métodos: agente viajero (NNH, CIH, 2-opt, óptimo), soluciones iniciales del transporte, árbol de expansión mínima, ruteo de vehículos, tamaño de lote, pronósticos, planificación agregada, programación de trabajos, distribución en planta y gestión de inventarios. Con `vary` y `values` se elige qué opción cambia y qué valores; con `variants`, combinaciones libres de opciones; con `metric`, la métrica que decide (por ejemplo `MSE` en lugar de `MAD`). En la aplicación es el botón **Comparar**, que aparece cuando el tipo de problema tiene métodos para comparar.

## Idiomas

Toda la app está en **español, inglés y portugués de Brasil**. El idioma se elige en **Ajustes** (Ctrl+,) y cambia la interfaz, los informes, los ejemplos, las fuentes (APA 7 en inglés: «&», «10th ed.», «n.d.»; en portugués: «&» en la lista de referencias, «e» en las citas narrativas, «10. ed.», «s.d.»; los títulos se dejan en su idioma original) y las gráficas que se descargan. El cambio es inmediato: la interfaz cambia primero y el problema abierto se vuelve a resolver en segundo plano (con un aviso mientras tanto); los catálogos del núcleo se preparan al abrir la app. La primera vez se usa el idioma del sistema.

- Interfaz: los textos se escriben en español y pasan por `t()` (`apps/desktop/src/lib/i18n.svelte.ts`); el inglés está en `apps/desktop/src/lib/i18n/en_*.ts` y el portugués en `pt*.ts`.
- Núcleo: `crates/iaqsb-core/src/i18n.rs` traduce lo que devuelve la API con los catálogos `crates/iaqsb-core/i18n/en.json` y `pt.json` (unos 6 000 textos y plantillas sacados del código y de los datos con `python3 tools/i18n_extraer.py`) y `en_extra.json` y `pt_extra.json` (palabras sueltas y frases armadas por partes). Los ids, las claves y los códigos no se traducen, y los resultados numéricos son los mismos en los tres idiomas.
- El lenguaje algebraico acepta palabras clave en los tres idiomas (`minimizar`/`minimize`/`minimizar`, `sujeto a`/`subject to`/`sujeito a`, `suma`/`sum`/`soma`…).
- CLI: `--lang en`, `--lang pt` o `IAQSB_LANG`. MCP: el argumento `language: "en"` o `"pt"` en cualquier herramienta (o `IAQSB_LANG` en la configuración del servidor).
- La prueba `crates/iaqsb-api/tests/todo/idioma.rs` resuelve los ejemplos en inglés y comprueba que salen en inglés, con los mismos números y con su fuente; `crates/iaqsb-core/src/i18n.rs` prueba el portugués.

Para agregar otro idioma (por ejemplo, francés para Haití o el Caribe) mira [CONTRIBUTING.md](CONTRIBUTING.md).

## Línea de comandos

```powershell
iaqsb modulos
iaqsb resolver problema.LPP --pasos        # tablas del símplex
iaqsb resolver QUEUE1.QAA                  # cualquier archivo de WinQSB
iaqsb resolver tsp.json --pasos --graficas figuras   # guarda las gráficas en PNG (--svg: también en SVG)
iaqsb comparar tsp.json                    # todos los métodos del tipo de problema
iaqsb --md resolver tsp.json               # resumen breve en Markdown, para agentes de IA
iaqsb comparar pronostico.json --metrica MSE --valores single_exponential,linear_trend --graficas figuras
iaqsb importar C_CHART.QCC > modelo.json
iaqsb exportar modelo.json -s copia.QCC
iaqsb --json llamar solve < peticion.json  # API completa, para agentes
iaqsb --lang en resolver tsp.json         # en inglés (también: variable IAQSB_LANG=en); --lang pt: portugués
iaqsb resolver transporte.iaqsb            # modelo del lenguaje algebraico
iaqsb revisar transporte.iaqsb             # solo revisa: errores con línea y columna, y tamaño
iaqsb informe wyndor.LPP -s informe.docx --pasos --autor "Grupo 8" --curso "Taller de Producción y Logística"   # .docx, .pdf, .html, .xlsx o .md
iaqsb convertir wyndor.LPP wyndor.xlsx     # a Excel (y de vuelta); también .iaqsb, .mps, .lp, .json
iaqsb barrer wyndor.LPP --dato constraints.2.rhs --desde 10 --hasta 30 --pasos 10   # ¿qué pasa si…?
iaqsb parametrico wyndor.LPP --lado Planta3   # objetivo según el lado derecho de Planta3, por tramos
iaqsb ajustar modelo.mps                   # ajuste automático de los parámetros del motor
```

Los módulos nuevos se llaman `vrp`, `modo`, `inv`, `fin`, `alm` y `kpi`; sus archivos (`.VRP`, `.MOD`, `.INV`, `.FIN`, `.ALM`, `.KPI`) son de IAQSB, no de WinQSB.

Las operaciones de `llamar` son `list_modules`, `describe_module`, `spec`, `template`, `solve`, `compare`, `comparable`, `render` (una gráfica del informe en SVG o PNG), `graphic`, `import_text`, `export_text`, `examples`, `bibliography`, `translate`, `solver_info`, `diagnose`, `bibtex`, `digest`, `convert`, `check_model`, `sweep`, `parametric`, `tune`, `export_report`, `export_xlsx`, `import_xlsx` y `parse_pasted`; todas reciben y devuelven JSON, y con `"lang": "en"` o `"pt"` responden en ese idioma.

## Modelos

Cada módulo describe su modelo con un esquema JSON (`iaqsb describir <módulo>`) y trae ejemplos (`iaqsb --json llamar examples`). Por ejemplo, programación lineal:

```json
{
  "name": "Wyndor",
  "objective": "max",
  "variables": [
    {"name": "X1", "cost": 3},
    {"name": "X2", "cost": 5, "upper": "M", "type": "integer"}
  ],
  "constraints": [
    {"name": "C1", "coefs": [1, 0], "direction": "<=", "rhs": 4},
    {"name": "C2", "coefs": [3, 2], "direction": "<=", "rhs": 18}
  ]
}
```

Las cotas infinitas se escriben `"M"` y `"-M"`, como en WinQSB. Un modelo como ese, escrito en el lenguaje algebraico:

```text
maximizar z: 3 X1 + 5 X2;
sujeto a
C1: X1 <= 4;
C2: 3 X1 + 2 X2 <= 18;
```

(en el modo rápido, sin `var`, las variables se declaran solas y son no negativas; ver [docs/LENGUAJE.md](docs/LENGUAJE.md)).

## Cómo citar

Si usas IAQSB en un trabajo académico, cita la versión que usaste (los datos están en [CITATION.cff](CITATION.cff) y en **Ajustes → Acerca de IAQSB**, que también lo copia en BibTeX):

IA Nativa. (2026). *IAQSB: investigación de operaciones nativa para IA* (Versión 0.2.1) [Software].

Cita además los métodos y los datos: cada informe trae sus referencias en APA 7.

## Licencia y créditos

Código bajo licencia MIT: se puede usar, estudiar, modificar y redistribuir, también en cursos, salas de cómputo y proyectos de investigación. Las contribuciones son bienvenidas ([CONTRIBUTING.md](CONTRIBUTING.md)); la comunidad sigue el [código de conducta](CODE_OF_CONDUCT.md). IAQSB es una reimplementación independiente de WinQSB, cuyos derechos pertenecen a su autor, Yih-Long Chang. El código de IAQSB no contiene código de WinQSB.

La carpeta `crates/iaqsb-api/tests/winqsb` guarda los 64 archivos de ejemplo de la distribución de WinQSB 2.0, que solo se usan en las pruebas de compatibilidad; si vas a publicar el código, revisa que tu permiso lo cubra o borra esa carpeta (la prueba se puede quitar sin afectar a nada más). Asegúrate de que tu permiso cubre el uso que quieras darle a IAQSB.

Las gráficas que dibuja el programa (servidor MCP, línea de comandos y descargas de la aplicación) usan las letras Inter (licencia SIL Open Font 1.1) y DejaVu Sans (licencia Bitstream Vera), incluidas con sus licencias en `crates/iaqsb-render/fuentes`.
