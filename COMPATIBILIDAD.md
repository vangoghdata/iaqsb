# Compatibilidad con WinQSB 2.0

IAQSB lee y escribe los archivos de texto de los 19 módulos de WinQSB 2.0. Un archivo guardado por IAQSB lo puede abrir el WinQSB original, y al revés. El módulo se reconoce por la primera palabra del archivo, así que la extensión no importa (también se abren los `.DAT` y `.TXT` que WinQSB exporta).

## Archivos comprobados

Los 64 archivos de ejemplo de la distribución original se leen, se resuelven, se guardan y se vuelven a leer con el mismo resultado. Es una prueba automática: `cargo test -p iaqsb-api --test todo winqsb_samples`.

| Módulo | Extensión | Ejemplos originales comprobados |
| --- | --- | --- |
| LP-ILP | `.LPP` | LP, LPNORMAL, ILP (formato matriz y modelo normal) |
| NET | `.NET` | NETFLOW, TRNSPORT, ASSIMENT, SHTPATH, MAXFLOW, SPANTREE, TSP |
| PERT-CPM | `.CPM` | CPM, CPMGRAPH, PERT, PERTGRPH (con las posiciones del gráfico) |
| GP-IGP | `.GPP` | GP, GPNORMAL, IGP |
| QP-IQP | `.QPP` | QP, QPNORMAL, IQP |
| NLP | `.NLP` | NLP1, NLP2, NLP3 |
| DP | `.DPP` | STAGE, KNAPSACK, PRODINVT |
| AP | `.APP` | APSIMPLE, APLP, APTRP |
| DA | `.DAA` | PAYOFF, BAYESIAN, GAME, DTREE |
| MKP | `.MKP` | MKP1, MKP2 |
| ITS | `.ITS` | EOQ, DISCOUNT, NEWSBOY, LOTSIZE, CRSQ, CRSS, PRRS, PRRSS |
| FC | `.FCC` | SALES, LINEREG |
| QA | `.QAA` | QUEUE1 (M/M), QUEUE2 (general) |
| QSS | `.QSS` | QSS1, QSS2, QSS3 (forma matriz), QSSGRAPH (modelo gráfico con posiciones) |
| JOB | `.JOB` | JOBSHOP, FLOWSHOP |
| MRP | `.MRP` | QSB |
| FLL | `.FLL` | LOCATION, LAYOUT, LINEBAL |
| QCC | `.QCC` | VARIABLE, PROBPLOT, C_CHART, P_CHART, PARETO |
| ASA | `.ASA` | ASA1 (simple), ASA2 (doble) |

Las extensiones salen de los cuadros de diálogo de cada ejecutable de WinQSB.

## Cómo se interpretan algunos casos

Son decisiones tomadas para leer los archivos originales tal como WinQSB los entiende:

- **Líneas en blanco.** WinQSB deja a veces una línea vacía entre secciones (por ejemplo, en IGP entre metas y restricciones). Se saltan.
- **DA, árbol de decisión.** Los nodos pueden repetir nombre ("High", "Low" en cada rama), así que los sucesores se indican por número. Si las probabilidades de un nodo de azar suman casi 1 (0.99 en el ejemplo DTREE), se usan tal cual y el informe lo avisa.
- **DP, producción e inventario.** La línea del inventario inicial es opcional; si falta, vale 0.
- **FLL, distribución.** Si dos departamentos reclaman la misma celda, gana el que aparece después (en el ejemplo LAYOUT, el pasillo fijo pisa parte de un departamento). El informe lo avisa.
- **QA.** El orden de los datos sigue el archivo: en el modelo simple, el servicio va antes que las llegadas; en el general están el coeficiente de presión del servicio, el de desaliento de las llegadas y la distribución del tamaño del lote. Una población de más de 10 000 clientes se toma como infinita, como en WinQSB. Pw es Σ P(n) con n ≥ s, la definición de la ayuda de WinQSB.
- **QSS.** Las distribuciones se escriben como en WinQSB (`Exp/0/1.2`, `Exp//2`, `CONSTANT/1`, `A/TRI/1/1.5/2`), los nombres no distinguen mayúsculas y las reglas se reconocen por sus tres primeras letras. Una cola sin capacidad vale 50. Si los seguidores tienen probabilidades y no se indicó regla de salida, se reparten por probabilidad.
- **QCC.** Los datos por atributos pueden venir como número o como proporción: si todos los valores están entre 0 y 1 y alguno tiene decimales (P_CHART), son proporciones. Los límites de especificación 0, 0, 0 significan que no hay. El formato horizontal se lee y se vuelve a escribir horizontal. En el análisis de Pareto, si la primera característica es el total de las demás (PARETO), se deja fuera.
- **ASA.** Las etiquetas y el orden de los datos siguen al programa original en los 11 planes, incluido el de la media de la muestra (límite de aceptación, de especificación, AQL y RQL). La marca de inspección rectificadora decide la fórmula del AOQ.

## Datos que WinQSB no tiene

- **Transbordo con costo o capacidad en los nodos y demandas con mínimo y máximo** (casos 2 y 3 y problema 1 de la clase). WinQSB no los guarda, así que el archivo `.NET` lleva la red equivalente: cada nodo con costo o capacidad se divide en dos (`3` y `3'`) unidos por un arco con ese costo y esa capacidad, y las demandas con rango se completan con un destino `Ficticio` que recibe la oferta que sobra. WinQSB la abre y da el mismo costo. Si la oferta o las capacidades no alcanzan, IAQSB entrega lo máximo posible y, con eso, busca el menor costo; el informe dice cuánto le falta a cada destino.
- **Ruteo de vehículos y selección del modo de transporte** no existen en WinQSB: sus archivos (`.VRP`, `.MOD`) son de IAQSB.
- **Modelos de LP-ILP de cualquier tamaño, semicontinuas, semienteras y varios objetivos.** IAQSB no limita el tamaño (WinQSB sí). Al guardar en `.LPP`, un modelo que no cabe en la hoja se escribe en la forma algebraica de WinQSB (solo los coeficientes distintos de cero); las variables semienteras se listan como enteras y las semicontinuas como continuas, que es lo más cercano que WinQSB entiende, y de un modelo con varios objetivos se guarda el de la hoja (los c(j)). Para no perder nada, el mismo modelo se puede guardar en MPS o LP, los formatos que leen Gurobi, CPLEX, HiGHS, SCIP, CBC y GLPK.
- **Modelo como programa lineal.** El transbordo, la ruta más corta y el agente viajero traen su modelo matemático y un botón para abrirlo en Programación lineal; desde ahí se guarda como `.LPP` para WinQSB, igual que en las diapositivas.

## Diferencias de cálculo que conviene conocer

- **Resultados exactos donde WinQSB aproxima.** En QA, los sistemas con distribuciones exponenciales se resuelven exactamente aunque tengan lotes, presión, desaliento o capacidad finita: es una cadena de Markov resuelta por balance de cortes. Cuando no hay fórmula se usa la aproximación de WinQSB o la simulación.
- **Programación cuadrática entera.** La ramificación es "mejor primero". El ejemplo IQP da el óptimo −303.2 en (19, 3, 1), comprobado por enumeración.
- **Constantes de los gráficos de control** (d2, d3, d4, c4, e3, e4). Se calculan por integración numérica para cualquier tamaño de subgrupo, en lugar de leerse de una tabla, y coinciden con las tablas publicadas.
- **CUSUM.** Si la característica tiene valor objetivo, se usa como w₀; si no, la línea central estimada.
- **Simulaciones.** Usan su propio generador de números aleatorios. Con la misma semilla se repiten los resultados en IAQSB, pero no son los mismos números que da WinQSB.
