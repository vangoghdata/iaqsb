# Guía para docentes

Esta guía explica cómo usar IAQSB en cursos de investigación de operaciones, desde el pregrado hasta el doctorado. Describe solo lo que hace la versión 0.2.0 del programa; los detalles técnicos del lenguaje algebraico están en [LENGUAJE.md](LENGUAJE.md), los de compatibilidad con WinQSB en [COMPATIBILIDAD.md](../COMPATIBILIDAD.md) y las fuentes bibliográficas en [FUENTES.md](FUENTES.md).

IAQSB es software libre (licencia MIT): se puede instalar en todas las salas, entregar a los estudiantes, modificar y redistribuir sin costo ni licencias por equipo. Reimplementa los 19 módulos de WinQSB 2.0, abre y guarda sus archivos, y agrega seis módulos de logística, un motor de optimización moderno, un lenguaje de modelado algebraico y un servidor MCP para asistentes de IA. Se usa de tres maneras con el mismo motor: la aplicación de escritorio, la línea de comandos (`iaqsb`) y el servidor MCP (`iaqsb-mcp`).

## Contenido

1. [Instalación en salas de cómputo](#1-instalación-en-salas-de-cómputo)
2. [Idiomas y ajustes](#2-idiomas-y-ajustes)
3. [Pregrado](#3-pregrado)
4. [Posgrado y maestría](#4-posgrado-y-maestría)
5. [Doctorado e investigación](#5-doctorado-e-investigación)
6. [Actividades y talleres sugeridos](#6-actividades-y-talleres-sugeridos)
7. [Integridad académica y asistente de IA](#7-integridad-académica-y-asistente-de-ia)
8. [Reportar problemas y contribuir](#8-reportar-problemas-y-contribuir)

## 1. Instalación en salas de cómputo

### Windows

El instalador es un solo archivo NSIS, `IAQSB_0.2.0_x64-setup.exe`, que se genera con `scripts\build-windows.ps1` (ver «Compilar en Windows» en [LEEME.md](../LEEME.md)). Basta compilarlo una vez y copiar el `.exe` a los equipos. Lo que conviene saber antes de desplegarlo:

- **Instalación por usuario.** El instalador está configurado con `installMode: currentUser` (`apps/desktop/src-tauri/tauri.conf.json`): instala en el perfil de quien lo ejecuta, en `C:\Users\<usuario>\AppData\Local\IAQSB`. En salas donde el perfil se borra al cerrar sesión, o donde cada estudiante entra con su propio usuario, hay que acordar con el área de TI cómo distribuirlo (por ejemplo, en la imagen del perfil por defecto o con la herramienta de despliegue de la universidad). Esta guía no describe instalación silenciosa ni para todos los usuarios porque no se ha comprobado.
- **Idioma del instalador.** Muestra un selector de idioma con español, inglés y portugués de Brasil.
- **Qué se instala.** La aplicación de escritorio, la línea de comandos `iaqsb.exe` y el servidor MCP `iaqsb-mcp.exe`, en la misma carpeta. Para usar `iaqsb` desde una terminal, escriba la ruta completa o agregue la carpeta al `PATH`.
- **Asociación de archivos.** Registra las extensiones de WinQSB (`.LPP`, `.NET`, `.CPM`, `.QAA`, …), las de los módulos nuevos (`.VRP`, `.MOD`, `.INV`, `.FIN`, `.ALM`, `.KPI`) y la del lenguaje algebraico (`.iaqsb`).
- **Sin internet.** El cálculo es local: el motor HiGHS viene incluido y no hace falta conexión. Solo el asistente de IA (sección 7) se conecta al proveedor de modelos que se elija (o a un modelo local con Ollama o LM Studio, que tampoco necesita internet).
- **Motores comerciales.** Gurobi y CPLEX no vienen con IAQSB. Si la sala ya tiene una licencia académica instalada (el programa o su paquete de Python `gurobipy` o `cplex`), IAQSB los detecta; también SCIP, CBC y GLPK. `iaqsb motores` lista lo que encontró en cada equipo.

### Linux y macOS

Se compila con `scripts/build.sh` (requiere además `cmake` y `clang`). La línea de comandos y el servidor MCP funcionan igual en los tres sistemas.

### Comprobación rápida

Después de instalar, en una terminal:

```sh
iaqsb --version          # iaqsb 0.2.0
iaqsb motores            # HiGHS y los motores externos que haya
iaqsb ejemplos wyndor > wyndor.json
iaqsb resolver wyndor.json    # Objetivo = 36
```

## 2. Idiomas y ajustes

La página **Ajustes** (<kbd>Ctrl</kbd>+<kbd>,</kbd>) guarda las preferencias en el equipo. Las que más importan en clase:

| Ajuste | Qué hace | Recomendación |
| --- | --- | --- |
| **Idioma** | Español, inglés o portugués de Brasil. Cambia la interfaz, los informes, los ejemplos, las fuentes (APA 7 en el idioma: «y», «&», «e») y las gráficas descargadas; el problema abierto se vuelve a resolver en el idioma nuevo. Los números no cambian. | El idioma del curso. Útil para cursos en inglés o con estudiantes de intercambio de Brasil. |
| **Nivel** | Pregrado o Posgrado e investigación. En la versión actual cambia el tono del asistente de IA: en pregrado explica paso a paso y define cada término; en posgrado responde con más rigor técnico (dualidad, condiciones de optimalidad, complejidad). | Pregrado en cursos introductorios; posgrado en maestría y doctorado. |
| **Decimales** | Cuántos decimales se muestran (los cálculos usan siempre toda la precisión). | 2 a 4 en clase; más al comparar con un libro. |
| **Separador decimal** | Punto en la pantalla (como WinQSB); coma (como en Colombia, Brasil y casi toda Latinoamérica); o punto siempre, también en los informes. Al escribir o pegar datos se aceptan la coma y el punto. | Coma, si así se escribe en la institución. |
| **Autoguardado** | Guarda el trabajo en el equipo mientras se trabaja y lo recupera al abrir IAQSB. No reemplaza «Guardar». Se puede borrar desde la misma página. | En salas compartidas, desactivarlo o borrar el autoguardado al terminar la sesión. |
| **Portada de los informes** | Autor o grupo, curso e institución que aparecen en la portada de los informes de Word, PDF y HTML (se pueden cambiar al exportar). | Pedir a cada grupo que lo llene antes del taller. |
| **Paso a paso** | Guarda el paso a paso al resolver (tablas del símplex, iteraciones). | Activo en pregrado. |
| **Motor** | Motor y parámetros por defecto para todos los módulos (tiempo máximo, brecha MIP, hilos, método). | Valores por defecto en pregrado. |
| **Inteligencia artificial** | Conectar la API del asistente: proveedor (OpenRouter, Anthropic, OpenAI, Gemini, Qwen, DeepSeek, Mistral, Groq, xAI, Moonshot, Ollama, LM Studio u otro compatible con OpenAI), dirección, clave y modelo; botón para probar la conexión. | Ver la sección 7. |

La página **Acerca de** trae la versión instalada y la cita de IAQSB en APA y BibTeX (sección 5.6). La línea de comandos acepta `--lang en` o `--lang pt` (o la variable `IAQSB_LANG`), y el servidor MCP el argumento `language`.

## 3. Pregrado

### 3.1 La hoja, como en WinQSB

El módulo **Programación lineal y entera** abre en la **Hoja**: una columna por variable y una fila por restricción, con el criterio del objetivo (maximizar o minimizar), el tipo de cada variable y sus cotas. Los estudiantes que ya conocen WinQSB la reconocen. Al entrar a una celda se selecciona su valor, y se puede pegar una tabla de Excel desde la celda elegida.

Para resolver: botón **Resolver** o <kbd>Ctrl</kbd>+<kbd>Intro</kbd>. La pestaña **Solución** muestra el valor de cada variable, su contribución y costo reducido, y por restricción el lado izquierdo, la holgura o el exceso y el precio sombra, con los rangos de sensibilidad de c(j) y del lado derecho con la semántica de WinQSB.

### 3.2 Método gráfico

En los modelos de dos variables aparece la pestaña **Método gráfico**: región factible, rectas de las restricciones, recta del objetivo en el óptimo y punto óptimo. Las gráficas se amplían a pantalla completa, se copian como imagen y se descargan en PNG o SVG para las presentaciones de clase.

### 3.3 Paso a paso del símplex y del branch and bound

Con los valores predeterminados del motor, los modelos pequeños se resuelven con el **símplex de tablas** y el **branch and bound** de WinQSB, y el resultado se comprueba con el motor. Así se puede seguir el método:

- **Tablas del símplex:** la tabla inicial, cada iteración y la tabla final, con la columna que entra resaltada y el pivote enmarcado; se avanza con las flechas.
- **Branch and bound** (modelos enteros): los nodos explorados, con las ramas de cada uno, el valor de su relajación y la decisión tomada.
- Los demás módulos traen una pestaña **Paso a paso** con las fórmulas aplicadas a los datos (iteraciones del transporte, pasos del vecino más cercano en el agente viajero, cálculos de colas, etc.).

Desde la terminal: `iaqsb resolver wyndor.json --pasos` imprime las tablas y `--nodos` el árbol de branch and bound.

Si el docente o el estudiante elige un motor, un tiempo máximo, una brecha o un número de hilos, la resolución pasa al motor y el paso a paso del símplex de tablas deja de mostrarse. Conviene dejar los valores por defecto en las clases donde se enseña el método.

### 3.4 Modo práctica: el estudiante pivotea y el programa revisa

La pestaña **Práctica** del módulo LP-ILP convierte el problema abierto en un ejercicio. El estudiante hace a mano cada decisión y el programa la revisa con aritmética exacta (fracciones y M simbólica), de modo que un camino distinto al de la regla de Dantzig también se reconoce como correcto.

**Símplex (método de la gran M).** En cada iteración el estudiante:

1. Elige la variable que **entra**. Si la columna no mejora el objetivo, se le explica por qué (el signo de C(j) − Z(j) según se maximice o minimice); si mejora pero no es la de Dantzig, se acepta y se le dice cuál habría elegido la regla.
2. Elige la variable que **sale** por la prueba del cociente mínimo. Se señalan los errores típicos: dividir entre un elemento no positivo o no tomar el mínimo; se avisa de empates y pivoteos degenerados.
3. Escribe la **nueva fila pivote** (acepta fracciones como `1/2` y decimales como `0,5`) y el programa marca cada celda.
4. Pivotea y ve la nueva tabla, o **declara la conclusión**: óptimo (con aviso de óptimos alternativos), no acotado o no factible (artificial positiva en la base).

**Branch and bound** (modelos enteros). En cada nodo el estudiante decide si se poda por no factible, se poda por cota, se registra una nueva mejor solución entera o se ramifica; elige la variable de ramificación y qué rama explorar primero. La exploración es en profundidad.

Ayudas y registro:

- **Pistas** graduales (tecla <kbd>P</kbd>): la primera recuerda la regla, las siguientes dan los cocientes o los valores.
- **Mostrar respuesta**, **Paso anterior**, **Reiniciar** y **Omitir este ejercicio**.
- Números en **fracciones** o **decimales**.
- Al terminar, un **resumen**: decisiones, aciertos sobre intentos, aciertos al primer intento, pistas usadas, respuestas mostradas e iteraciones (comparadas con las del programa), y si el resultado coincide con el del motor.
- Se maneja con el teclado (<kbd>Tab</kbd>, flechas, <kbd>Intro</kbd>).

Límites: la práctica está en la hoja (no en el modo algebraico), para modelos pequeños, sin variables semicontinuas ni varios objetivos. En modelos enteros, el símplex se practica sobre la relajación lineal.

El resumen no se envía a ningún lado ni queda en un registro para el docente: si se quiere usar en una evaluación, se le puede pedir al estudiante una captura de pantalla del resumen, sabiendo que el ejercicio se puede repetir.

### 3.5 Ejemplos con su fuente

El menú **Ejemplos** de cada módulo carga problemas listos (111 en total), y cada uno dice de dónde sale en APA 7: libro con capítulo y sección (por ejemplo, Wyndor Glass, de Hillier y Lieberman, 2015, cap. 3), diapositiva de clase u original de IAQSB comprobado con un cálculo independiente. Los que faltan por confirmar en un ejemplar impreso dicen «por verificar». Cada informe tiene además una pestaña **Fuentes** con los métodos y fórmulas usados, su referencia y su DOI, que se copia en APA o en BibTeX.

Esto permite pedir a los estudiantes que **reproduzcan un resultado publicado** y que citen correctamente el método que usaron. Desde la terminal: `iaqsb ejemplos` lista todos con su fuente.

### 3.6 Archivos de WinQSB

IAQSB abre y guarda los archivos de los 19 módulos de WinQSB 2.0 (`.LPP`, `.NET`, `.CPM`, `.QAA`, `.ITS`, …), y el WinQSB original puede volver a abrirlos. Se comprobó con los 64 archivos de ejemplo de la distribución original (detalle en [COMPATIBILIDAD.md](../COMPATIBILIDAD.md)). Los bancos de ejercicios y las guías de laboratorio que ya existen en formato WinQSB se pueden seguir usando sin cambios, y los estudiantes pueden entregar en ese formato si la cátedra lo exige.

Además, cualquier modelo se puede guardar como libro de Excel (una hoja por tabla del editor) y volver a abrirse en IAQSB sin perder nada.

### 3.7 Comparar métodos

En los módulos con varios métodos (agente viajero, soluciones iniciales del transporte, árbol de expansión, ruteo, tamaño de lote, pronósticos, planificación agregada, programación de trabajos, distribución en planta, gestión de inventarios), el botón **Comparar** resuelve el mismo problema con cada método y muestra una tabla con las métricas, el mejor, los empates y la diferencia de cada uno con el mejor. Sirve para discutir heurísticas frente a métodos exactos.

## 4. Posgrado y maestría

### 4.1 Lenguaje algebraico

En el módulo LP-ILP, el selector **Hoja | Modelo algebraico** abre un editor donde el modelo se escribe con conjuntos, parámetros y variables indexadas, al estilo de AMPL o LINGO, con palabras clave en español, inglés o portugués. Un transporte de cualquier tamaño ocupa las mismas líneas:

```iaqsb
conjunto Plantas  = {Cali, Medellin, Bogota};
conjunto Ciudades = 1..5;
parametro oferta{Plantas}   = [300, 500, 400];
parametro demanda{Ciudades} = [180, 220, 160, 240, 200];
parametro costo{Plantas, Ciudades} = [[4, 6, 9, 5, 7], [7, 3, 6, 6, 4], [9, 5, 2, 4, 5]];
variable x{Plantas, Ciudades} >= 0;
minimizar z: suma{i en Plantas, j en Ciudades} costo[i,j] * x[i,j];
sujeto a
  capacidad{i en Plantas}: suma{j en Ciudades} x[i,j] <= oferta[i];
  atender{j en Ciudades}:  suma{i en Plantas} x[i,j] >= demanda[j];
```

El editor revisa mientras se escribe y señala el error con su línea y columna. El lenguaje acepta además restricciones generales que reformula solo con variables binarias y big-M tomado de las cotas: valor absoluto, máximo y mínimo, funciones lineales por tramos, restricciones lógicas, indicadoras («si y = 1 entonces…»), disyunciones y cardinalidad; también objetivo cuadrático convexo y varios objetivos por prioridad. La solución se presenta por índices (x[i,j] como tabla de doble entrada). Todo está documentado, con ejemplos comprobados, en [LENGUAJE.md](LENGUAJE.md).

Pedagógicamente, el lenguaje separa el **modelo** de los **datos**: es la forma de trabajar de la industria y de la literatura, y obliga a pensar en familias de restricciones y no en filas sueltas. Los botones **Pasar a la hoja** y **Desde la hoja** permiten ver el mismo modelo en las dos formas.

### 4.2 Motores de optimización

Todo lo que se resuelve con programación lineal o entera pasa por un solo motor. Por defecto es **HiGHS**, incluido y sin límite de tamaño (símplex dual y primal, punto interior, PDLP, ramificación y corte). Si en el equipo hay **Gurobi, CPLEX, SCIP, CBC o GLPK**, IAQSB los usa cuando se piden, con la licencia de cada usuario. El botón del motor (barra superior) elige el motor y sus parámetros para todos los módulos: tiempo máximo, brecha MIP, hilos, método, registro del motor. Cada informe dice qué motor resolvió, en cuánto tiempo, el tamaño del modelo y, en modelos enteros, los nodos, la cota y la brecha; si se alcanzó un límite, se devuelve la mejor solución encontrada y se advierte.

En clase esto sirve para mostrar que el mismo modelo da el mismo óptimo con motores distintos y para discutir el efecto de la brecha MIP y del tiempo máximo. Desde la terminal:

```sh
iaqsb resolver modelo.iaqsb --motor gurobi --tiempo 60 --brecha 0.01 --hilos 4 --registro
```

### 4.3 ¿Qué pasa si…? y análisis paramétrico

En la hoja de LP-ILP, después de resolver, la pestaña **¿Qué pasa si…?** abre un escenario sobre una copia del modelo:

- Un deslizador por cada costo c(j), cada lado derecho b(i) y cada coeficiente. Cada cambio vuelve a resolver y dice qué variables entran o salen de la solución y qué restricciones se activan o dejan de estarlo.
- Una **banda verde** marca el rango en que la solución actual sigue siendo óptima (un coeficiente a la vez), es decir, el rango de sensibilidad.
- La curva **Z según el parámetro** sale del análisis paramétrico exacto, con los puntos donde cambia la base óptima; en modelos enteros, de un barrido.
- **Aplicar al modelo** copia los valores del escenario al modelo original; mientras no se pulse, el modelo no cambia.

En los demás módulos, el mismo botón permite elegir cualquier dato numérico, un rango y un número de pasos; se resuelve el problema con cada valor y se grafica la métrica elegida.

Desde la terminal, el análisis paramétrico de un lado derecho o de un costo, y el barrido de cualquier dato:

```sh
iaqsb parametrico modelo.iaqsb --lado maquina --desde 50 --hasta 200
iaqsb parametrico wyndor.json --costo 0 --desde 0 --hasta 10
iaqsb barrer wyndor.json --dato constraints.2.rhs --desde 10 --hasta 30 --pasos 10
```

Esto permite trabajar la teoría de dualidad y sensibilidad con evidencia: el estudiante predice el precio sombra y el rango, y lo contrasta con la curva.

### 4.4 Ajuste automático de parámetros

Para modelos enteros difíciles, **Ajustar para este modelo** (en el botón del motor) prueba varias configuraciones de HiGHS (presolve, método, heurísticas, detección de simetrías, hilos, semilla), mide cada una varias veces y propone la más rápida que da el mismo resultado, como el *tuning tool* de Gurobi o CPLEX. Si el modelo ya se resuelve rápido, lo dice y no recomienda cambios. Desde la terminal: `iaqsb ajustar modelo.iaqsb --tiempo 20`.

Es un buen pretexto para discutir la variabilidad del desempeño de los algoritmos de ramificación y corte y la importancia de medir con repeticiones.

### 4.5 Diagnóstico de infactibilidad

Cuando un modelo de LP-ILP no tiene solución, el informe trae:

- el **subsistema irreducible infactible (IIS)**: las restricciones y cotas que no se pueden cumplir a la vez (basta relajar una para romper el conflicto), y
- la **relajación de factibilidad**: el menor cambio de los lados derechos que vuelve factible el modelo.

```text
$ iaqsb diagnosticar infactible.iaqsb
No se pueden cumplir a la vez: a, b, c. El cambio mínimo para que sea factible suma 1: b: lado derecho de 3 a 2.
  ✗ a: x + y <= 4
  ✗ b: x >= 3
  ✗ c: y >= 2
```

En cursos de modelación, esto convierte el error más frustrante («el problema no es factible») en un ejercicio de análisis.

## 5. Doctorado e investigación

### 5.1 Modelos grandes

IAQSB no hereda los límites de tamaño de WinQSB. Un transporte de 200 × 300 (60 000 variables) se resuelve en una fracción de segundo con HiGHS, y el lenguaje algebraico arma modelos de ese tamaño en pocos segundos. Se abren y guardan los formatos de la industria **MPS** y **LP** (el de CPLEX), de modo que los modelos se pueden intercambiar con otros programas y con las bibliotecas de prueba de la literatura. Un modelo demasiado grande para la hoja se queda como texto y se resuelve directamente.

### 5.2 Reproducibilidad

- **Modelos como texto.** Los archivos `.iaqsb` son texto plano: se versionan en git, se comparan línea a línea, se revisan en un *pull request* y se publican como material suplementario de un artículo o una tesis.
- **Línea de comandos por lotes.** Todas las órdenes aceptan `--json` y devuelven un código de error distinto de cero cuando algo falla, así que se integran en scripts:

  ```sh
  for f in instancias/*.iaqsb; do
    iaqsb revisar "$f" && iaqsb --json resolver "$f" --tiempo 300 > "resultados/$(basename "$f" .iaqsb).json"
  done
  ```

- **Datos separados del modelo.** Un `.json` con `{"module": "lp", "model": {"format": "iaqsb", "text": …, "data": {…}}}` resuelve el mismo modelo con otros datos, y `iaqsb barrer` recorre un parámetro de `data` (ver [LENGUAJE.md](LENGUAJE.md), secciones 5.4 y 16).
- **Qué registrar.** La versión de IAQSB (`iaqsb --version`), el motor y su versión, el tiempo, la cota y la brecha (todos vienen en el informe y en la salida JSON), y los parámetros del motor usados.

### 5.3 Informes para entregar: Word, PDF, HTML, Excel y Markdown

El botón **Exportar el informe** (o `iaqsb informe`) genera un documento con portada (autor o grupo, curso, institución y fecha), planteamiento del problema, tablas y figuras numeradas al estilo **APA 7** («Tabla 1», «Figura 1») y la lista de referencias con sangría francesa, con el paso a paso opcional y coma decimal si se pide.

```sh
iaqsb informe modelo.iaqsb -s informe.docx --autor "Grupo 3" --curso "Investigación de operaciones II" --institucion "Universidad Nacional de Colombia" --coma --pasos
iaqsb informe modelo.iaqsb -s informe.pdf
```

El formato sale de la extensión: `.docx`, `.pdf`, `.html`, `.xlsx` o `.md` (Markdown, texto plano sin portada ni figuras).

### 5.4 Excel

- **El modelo**: cualquier módulo se guarda como libro `.xlsx` con una hoja por tabla del editor y una hoja «IAQSB» con el modelo completo; se puede editar en Excel y volver a abrir en IAQSB. Un libro propio de LP-ILP se lee si sigue la disposición de WinQSB (variables en la primera fila, objetivo en la segunda, cada restricción con su sentido y su lado derecho): `iaqsb convertir libro.xlsx modelo.json --modulo lp`.
- **Los resultados**: el informe en `.xlsx` trae una hoja de resumen y una por cada tabla, con los números como números, listos para análisis posteriores.

### 5.5 Agentes de IA por MCP

El servidor `iaqsb-mcp` expone el motor a Claude Desktop u otro asistente compatible con MCP. En la aplicación, **Conectar con Claude** muestra la configuración exacta. Las herramientas permiten listar y describir módulos, resolver (también modelos en el lenguaje algebraico, LP o MPS), comparar métodos, revisar modelos, barrer datos, hacer análisis paramétrico, ajustar el motor, diagnosticar infactibilidad, convertir formatos, importar y exportar archivos de WinQSB y de Excel, exportar informes y consultar la bibliografía. Las respuestas son cortas y en Markdown, con un `report_id` para pedir partes del informe.

En investigación sirve para estudiar la formulación asistida por IA: el agente escribe el modelo, lo revisa con `check_model`, lo resuelve con un motor verificable y cita las fuentes del informe. Lo que el agente afirma se puede contrastar con el informe del motor. Para agentes sin MCP, la línea de comandos ofrece lo mismo con `iaqsb --json llamar <operación>` y `iaqsb --md resolver`.

### 5.6 Cómo citar IAQSB

En **Ajustes → Acerca de** está la cita lista para copiar en APA o BibTeX:

> IA Nativa. (2026). *IAQSB: investigación de operaciones nativa para IA* (Versión 0.2.0) [Software].

```bibtex
@software{iaqsb2026,
  author = {{IA Nativa}},
  title = {IAQSB},
  year = {2026},
  version = {0.2.0},
  note = {Software de investigación de operaciones de código abierto (licencia MIT)}
}
```

Cite la versión con la que se obtuvieron los resultados. Además de IAQSB, cite los métodos y el motor que aparecen en la pestaña **Fuentes** del informe (por ejemplo, el símplex dual revisado de HiGHS: Huangfu y Hall, 2018). La bibliografía completa del programa está en [iaqsb.bib](iaqsb.bib) y en `iaqsb fuentes --bibtex`.

## 6. Actividades y talleres sugeridos

**Pregrado**

- *Del gráfico al símplex.* Un modelo de dos variables (por ejemplo, Wyndor): el estudiante resuelve primero en el **Método gráfico**, identifica los vértices y luego recorre las mismas soluciones en la **Práctica** del símplex, relacionando cada tabla con un vértice.
- *Práctica con meta.* Tres ejercicios de la Práctica con la condición de terminar con al menos el 80 % de aciertos al primer intento y sin respuestas mostradas; se entrega la captura del resumen.
- *Sensibilidad intuitiva.* Con «¿Qué pasa si…?», mover un lado derecho fuera de la banda verde y explicar por qué cambia la base; contrastar con el precio sombra del informe.
- *Reproducir un libro.* Elegir un ejemplo del menú con fuente verificada, reproducir el resultado y citar en APA el ejemplo y el método.
- *Branch and bound a mano.* Un problema de mochila o de producción entera en la Práctica de branch and bound, dibujando el árbol en papel antes de comprobarlo.

**Posgrado y maestría**

- *De la hoja al lenguaje.* Abrir un `.LPP` de WinQSB, pasarlo al lenguaje con **Desde la hoja** y reescribirlo con conjuntos e índices; verificar que el óptimo no cambia.
- *Costos fijos.* Formular un problema de localización con binarias e indicadoras (ejemplo «Localización con binarias» del editor) y explicar la reformulación que IAQSB lista en la revisión.
- *Motores y brecha.* Resolver una instancia entera con distintos tiempos máximos y brechas; graficar la brecha contra el tiempo a partir de la salida JSON.
- *Infactibilidad.* Recibir un modelo infactible, interpretar el IIS y proponer la modificación de menor costo.

**Doctorado e investigación**

- *Experimento computacional reproducible.* Un repositorio con modelos `.iaqsb`, datos en JSON y un script por lotes que genera la tabla de resultados; otro estudiante lo reproduce en su equipo.
- *Ajuste de parámetros.* Usar `iaqsb ajustar` en una familia de instancias y discutir si la configuración recomendada generaliza.
- *Formulación asistida por IA.* Comparar modelos escritos por un agente vía MCP con los del investigador: validez, tamaño, tiempo de solución y errores típicos.

## 7. Integridad académica y asistente de IA

IAQSB incluye dos funciones de IA:

- **Explicar con IA** (en las vistas de solución) arma un texto con el informe y el modelo para copiarlo a cualquier asistente, o preguntarlo directamente en el chat de IAQSB.
- **Asistente** (barra superior): un chat con el modelo de IA que se conecte en Ajustes (Claude, ChatGPT, Gemini, Qwen, DeepSeek, Llama y cientos más por OpenRouter, o un modelo local con Ollama o LM Studio). Con los modelos que usan herramientas, usa el motor local para resolver, comparar y barrer datos, lee el problema abierto y puede cargar un modelo en el editor (pide confirmación antes de reemplazar trabajo sin guardar). Las claves son de quien lo usa y se guardan solo en ese equipo.

**Para una sala de cómputo o un curso grande** hay dos opciones sin claves personales: un modelo abierto en cada equipo con Ollama o LM Studio (gratis y sin internet; los modelos pequeños se equivocan más al usar las herramientas), o un servidor de la universidad compatible con la API de OpenAI (vLLM, LocalAI…), con la opción «Otro compatible con OpenAI» y su dirección.

Las instrucciones del asistente le piden no inventar cifras: los números deben salir del usuario, del contexto adjunto o del motor, y las fuentes se citan tal como vienen en el informe. Aun así, una explicación generada por IA puede contener errores de interpretación.

Estas son **recomendaciones** para que cada curso defina su propia política, no reglas del programa:

- **Diga explícitamente qué se permite** en cada actividad: sin IA, IA para entender pero no para resolver, o IA libre con declaración de uso. Un mismo curso puede tener reglas distintas para talleres y exámenes.
- **Evalúe el razonamiento, no solo el número.** El motor da el óptimo a cualquiera; lo que distingue al estudiante es la formulación, la interpretación del precio sombra o la justificación de una poda. La Práctica, la defensa oral y las preguntas de «¿qué pasa si…?» evalúan eso mejor que el resultado final.
- **Pida declarar el uso de IA** en los informes (qué herramienta, para qué y cómo se verificó), igual que se citan los métodos. El informe exportado ya cita el motor y los métodos; la declaración completa esa trazabilidad.
- **Use el nivel adecuado.** En pregrado, el asistente explica paso a paso; puede ser un buen tutor fuera de clase si el curso lo permite.
- **En salas compartidas**, no deje claves de API personales guardadas en los equipos y considere desactivar el autoguardado, porque el trabajo autoguardado y la clave quedan en el perfil del equipo.
- **En exámenes en sala**, si no se permite IA, tenga en cuenta que el asistente necesita un modelo conectado (una clave con internet, o un modelo local instalado): sin él, el resto del programa funciona igual.

## 8. Reportar problemas y contribuir

IAQSB es un proyecto abierto y su calidad depende de que los errores se reporten con precisión. La forma de reportar problemas, proponer cambios, agregar ejemplos con su fuente o traducir está en [CONTRIBUTING.md](../CONTRIBUTING.md).

Un buen reporte incluye:

- la versión (`iaqsb --version` o **Ajustes → Acerca de**) y el sistema operativo;
- el archivo del modelo (`.iaqsb`, `.LPP`, `.json` u otro) o un ejemplo mínimo que reproduzca el problema;
- lo que se esperaba y lo que se obtuvo, con el mensaje de error completo (en el lenguaje algebraico, con su línea y columna);
- si el resultado contradice un libro, la referencia exacta (capítulo, sección, página).

Las contribuciones especialmente útiles desde la docencia son los **ejemplos de clase con su fuente** (todo ejemplo nuevo debe decir de dónde sale y reproducir el resultado publicado, porque las pruebas automáticas lo exigen), la verificación de referencias marcadas «por verificar» en un ejemplar impreso y las traducciones.
