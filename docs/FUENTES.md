# Fuentes de IAQSB

Todo método, fórmula, paso y ejemplo de IAQSB dice de dónde sale. Este documento se genera con `python3 tools/generar_fuentes.py` a partir de `crates/iaqsb-core/fuentes/bibliografia.json` y `crates/iaqsb-core/fuentes/ejemplos.json` y `crates/iaqsb-core/fuentes/metodos.json`; la aplicación muestra lo mismo en la pestaña **Fuentes** de cada informe, la CLI con `iaqsb fuentes` (y `--bibtex`) y el servidor MCP con `get_bibliography`.

- **Estilo:** APA 7.ª edición, en español.
- **Verificada:** los datos se comprobaron en el DOI, el editor o un catálogo de biblioteca.
- **Por verificar:** falta confirmar algo (se dice qué). Nada se inventa: si no se conoce una página o una edición, no se escribe.
- **Material de clase:** las diapositivas y los talleres del curso Taller de Producción y Logística de la carrera de Ingeniería Industrial de la Universidad Nacional de Colombia se citan por su profesor: Sarache (2026a, b, c…) en logística y Arango Marín (2020, 2021) en programa maestro y MRP II, aunque la diapositiva a su vez cite un libro; si la diapositiva nombra la fuente original (Bronson, Ballou, Taha), se cita también. El año de las diapositivas de Arango Marín sale de los metadatos de los archivos y queda por verificar.
- **Métodos:** cada informe cita los métodos que usó. Los módulos de logística lo hacen desde su código y los demás con las reglas de `crates/iaqsb-core/fuentes/metodos.json` (resumidas abajo).

## Referencias (163, 21 por verificar)

- Ahuja, R. K., Magnanti, T. L. y Orlin, J. B. (1993). *Network flows: Theory, algorithms, and applications*. Prentice Hall.  
  `ahuja1993` · Flujo en redes; transformación de nodos con costo o capacidad en arcos (división del nodo).
- American Society for Quality. (2018a). *Sampling procedures and tables for inspection by attributes (ANSI/ASQ Z1.4-2003 [R2018])*. https://asq.org/quality-press/display-item?item=T1164 — _Designación vigente confirmada: ASQ la llama "THIS IS THE CURRENT EDITION OF Z1.4" (publicada 2018, 94 p.) y la ANSI Webstore (https://webstore.ansi.org/standards/asq/ansiasqz12003r2018) la marca "Most recent", reemplazando a la R2013. ASQ la escribe "ASQ/ANSI Z1.4:2003 (R2018)". Autor y editor coinciden (ASQ), por eso se omite la editorial. Sin ISBN publicado._  
  `ansiz14` · Planes de muestreo de aceptación por atributos (niveles de inspección, AQL, inspección normal, rigurosa y reducida).
- American Society for Quality. (2018b). *Sampling procedures and tables for inspection by variables for percent nonconforming (ANSI/ASQ Z1.9-2003 [R2018])*. https://asq.org/quality-press/display-item?item=T1165 — _ANSI Webstore: "Most recent", reemplaza a la R2013. ASQ (https://asq.org/quality-press/display-item?item=T1165) la llama "THIS IS THE CURRENT EDITION OF Z1.9" (publicada 2018, 116 p.) y la escribe "ASQ/ANSI Z1.9:2003 (R2018)". Sin ISBN publicado._  
  `ansiz19` · Muestreo de aceptación por variables: métodos k (forma 1) y M (forma 2).
- Arango Marín, J. A. (2020). *Programa maestro* [Diapositivas de clase del curso Taller de Producción y Logística, Ingeniería Industrial]. Universidad Nacional de Colombia. **[Por verificar: El año sale de los metadatos del archivo (creado en 2016, última modificación en 2020); confirmar la versión usada en el curso y la sede.]** — _Diapositivas y hojas de cálculo del curso Taller de Producción y Logística de Ingeniería Industrial de la Universidad Nacional de Colombia, entregadas por un estudiante del curso. Autor según los metadatos de los archivos (Jaime Antero Arango Marín) y el pie de las diapositivas._  
  `arango2020maestro` · Programa maestro básico y de ensamble bajo pedido (ATO), horizonte, barreras de tiempo y disponible para promesa, con ejemplos resueltos en Excel.
- Arango Marín, J. A. (2021a). *Exploración y visualización de series de tiempo* [Diapositivas de clase del curso Taller de Producción y Logística, Ingeniería Industrial]. Universidad Nacional de Colombia. **[Por verificar: El año sale de los metadatos del archivo (creado en 2020, última modificación en 2021); confirmar la versión usada en el curso.]** — _Diapositivas y hojas de cálculo del curso Taller de Producción y Logística de Ingeniería Industrial de la Universidad Nacional de Colombia, entregadas por un estudiante del curso. Autor según los metadatos de los archivos (Jaime Antero Arango Marín) y el pie de las diapositivas._  
  `arango2021series` · Gráficos de series de tiempo, gráficos estacionales, autocorrelación y ruido blanco, con el ejemplo de utilidades trimestrales en R.
- Arango Marín, J. A. (2021b). *MRP II (Gozinto)* [Diapositivas de clase del curso Taller de Producción y Logística, Ingeniería Industrial]. Universidad Nacional de Colombia. **[Por verificar: El año sale de los metadatos del archivo (creado en 2014, última modificación en 2021); confirmar la versión usada en el curso.]** — _Diapositivas y hojas de cálculo del curso Taller de Producción y Logística de Ingeniería Industrial de la Universidad Nacional de Colombia, entregadas por un estudiante del curso. Autor según los metadatos de los archivos (Jaime Antero Arango Marín) y el pie de las diapositivas._  
  `arango2021gozinto` · Matrices de entrada (LM, Ds, Ifd, Exd, Opd, Ipr, Reo, Tpr, Ctr, Fcal) y operaciones del MRP II por el método Gozinto, con el ejemplo de los estantes.
- Arango Marín, J. A. (2021c). *Procedimiento MRP* [Diapositivas de clase del curso Taller de Producción y Logística, Ingeniería Industrial]. Universidad Nacional de Colombia. **[Por verificar: El año sale de los metadatos del archivo (creado en 2017, última modificación en 2021); confirmar la versión usada en el curso.]** — _Diapositivas y hojas de cálculo del curso Taller de Producción y Logística de Ingeniería Industrial de la Universidad Nacional de Colombia, entregadas por un estudiante del curso. Autor según los metadatos de los archivos (Jaime Antero Arango Marín) y el pie de las diapositivas._  
  `arango2021mrp` · Procedimiento MRP paso a paso (necesidades brutas y netas con stock de seguridad, recepciones programadas, lanzamiento de órdenes) y ejercicio de siete artículos resuelto en Excel.
- Arango Marín, J. A. (2022). *Pronósticos por suavización exponencial* [Diapositivas de clase del curso Taller de Producción y Logística, Ingeniería Industrial]. Universidad Nacional de Colombia. **[Por verificar: El año sale de los metadatos del archivo (creado en 2020, última modificación en 2022) y del pie de las diapositivas (14/10/2022); confirmar la versión usada en el curso.]** — _Diapositivas del curso Taller de Producción y Logística de Ingeniería Industrial de la Universidad Nacional de Colombia, entregadas por un estudiante del curso. Autor según los metadatos del archivo (Jaime Antero Arango Marín) y el pie de las diapositivas._  
  `arango2022suavizacion` · Suavización exponencial simple, Holt y Holt-Winters con ses(), holt() y hw() del paquete forecast de R; conjuntos de entrenamiento y prueba, accuracy() y comparación con el pronóstico ingenuo.
- Arango Marín, J. A. (2024). *Modelos ARIMA* [Diapositivas de clase del curso Taller de Producción y Logística, Ingeniería Industrial]. Universidad Nacional de Colombia. **[Por verificar: El año sale de los metadatos del archivo (creado en 2020, última modificación en 2024); confirmar la versión usada en el curso.]** — _Diapositivas y hojas de cálculo del curso Taller de Producción y Logística de Ingeniería Industrial de la Universidad Nacional de Colombia, entregadas por un estudiante del curso. Autor según los metadatos de los archivos (Jaime Antero Arango Marín) y el pie de las diapositivas._  
  `arango2024arima` · ARIMA y ARIMA estacional con auto.arima en R, comparación con ETS y modelos TBATS, con los datos de fpp2.
- Arcus, A. L. (1965). A computer method of sequencing operations for assembly lines. *International Journal of Production Research*, *4*(4), 259–277. https://doi.org/10.1080/00207546508919982 — _Corrección: el registro del editor (Taylor & Francis vía Crossref) fecha el número en 1965 y el título no lleva el prefijo 'COMSOAL:'; suele citarse como 1966 con ese prefijo. Se mantiene el id arcus1966 para no romper referencias, pero el año citado es 1965._  
  `arcus1966` · Balanceo de líneas con el método COMSOAL.
- Armour, G. C. y Buffa, E. S. (1963). A heuristic algorithm and simulation approach to relative location of facilities. *Management Science*, *9*(2), 294–309. https://doi.org/10.1287/mnsc.9.2.294  
  `armourbuffa1963` · Distribución en planta por intercambio de pares (CRAFT).
- Arrow, K. J., Harris, T. y Marschak, J. (1951). Optimal inventory policy. *Econometrica*, *19*(3), 250–272. https://doi.org/10.2307/1906813  
  `arrow1951` · Modelo de un período (vendedor de periódicos) y origen de las políticas (s, S).
- Assimakopoulos, V. y Nikolopoulos, K. (2000). The theta model: A decomposition approach to forecasting. *International Journal of Forecasting*, *16*(4), 521–530. https://doi.org/10.1016/S0169-2070(00)00066-2  
  `assimakopoulos2000` · Método Theta.
- Association for Supply Chain Management. (2022). *Supply chain operations reference model: SCOR digital standard* (Versión 14.0). ASCM. https://www.ascm.org/corporate-solutions/standards-tools/scor-ds/  
  `scor` · Métrica de pedido perfecto (Perfect Order Fulfillment, RL.1.1).
- Ballou, R. H. (2004a). *Business logistics/supply chain management: Planning, organizing, and controlling the supply chain* (5.ª ed.). Pearson Prentice Hall.  
  `ballou2004` · Selección del servicio de transporte por costo total (transporte frente a inventarios).
- Ballou, R. H. (2004b). *Logística: Administración de la cadena de suministro* (5.ª ed.). Pearson Educación. — _Lectura obligatoria del curso (caps. 3 y 6)._  
  `ballou2004es` · Edición en español de Ballou (2004).
- Bellman, R. (1957). *Dynamic programming*. Princeton University Press. — _No se pone DOI: el de Princeton (10.1515/9781400835386) es de la reedición de 2010, no de la de 1957. El ISBN 069107951X del catálogo es de reimpresiones posteriores._  
  `bellman1957` · Programación dinámica (principio de optimalidad).
- Bellman, R. (1958). On a routing problem. *Quarterly of Applied Mathematics*, *16*(1), 87–90. https://doi.org/10.1090/qam/102435  
  `bellman1958` · Ruta más corta con costos negativos (Bellman-Ford).
- Bernstein, D. J. (2008). ChaCha, a variant of Salsa20. En *SASC 2008: The State of the Art of Stream Ciphers (workshop record)*. . https://cr.yp.to/chacha/chacha-20080128.pdf — _No tiene DOI ni páginas confirmadas. La versión definitiva es la del 28 de enero de 2008 (documento 4027b5256e17b9796842e6d0f68b0b5e) en la página del autor. El congreso SASC 2008 aparece en la referencia [6] de Bernstein et al. (2015, LNCS, Springer); la página del autor no lo nombra._  
  `bernstein2008` · Generador ChaCha8 de números aleatorios (simulación de colas y de inventarios).
- Blank, L. y Tarquin, A. (2020). *Ingeniería económica* (8.ª ed.). McGraw-Hill. **[Por verificar: Confirmar el año de la 8.ª ed. en español y su ISBN (solo se halló el del paquete con Connect).]**  
  `blanktarquin` · Factor de recuperación de capital (A/P, i, n).
- Bowman, E. H. (1956). Production scheduling by the transportation method of linear programming. *Operations Research*, *4*(1), 100–103. https://doi.org/10.1287/opre.4.1.100  
  `bowman1956` · Planificación agregada mediante el método de transporte de la programación lineal.
- Bronson, R. (1983). *Teoría y problemas de investigación de operaciones*. McGraw-Hill. — _Edición en español del original en inglés de 1982. Las diapositivas lo citan como Bronson (1982) y Bronson (1983); el capítulo 15 trata el análisis de redes._  
  `bronson1983` · Algoritmos de clase para ruta más corta, árbol de expansión mínima y flujo máximo.
- Brown, R. G. (1959). *Statistical forecasting for inventory control*. McGraw-Hill. — _Obra anterior al ISBN; el catálogo de Texas A&M registra McGraw-Hill, Nueva York, 1959._  
  `brown1959` · Suavizamiento exponencial simple y doble.
- Campbell, H. G., Dudek, R. A. y Smith, M. L. (1970). A heuristic algorithm for the n job, m machine sequencing problem. *Management Science*, *16*(10), B630–B637. https://doi.org/10.1287/mnsc.16.10.B630 — _En el título original n y m van en cursiva._  
  `campbell1970` · Heurística CDS para el taller de flujo (flow shop) de m máquinas.
- Chang, Y.-L. y Desai, K. (2003). *WinQSB: Decision support software for MS/OM (Version 2.0)*. Wiley. **[Por verificar: Solo se halló en librerías; falta confirmar título exacto y autores en un catálogo.]**  
  `chang2003` · Manual de WinQSB 2.0: métodos y formatos que IAQSB reproduce.
- Charnes, A. y Cooper, W. W. (1954). The stepping stone method of explaining linear programming calculations in transportation problems. *Management Science*, *1*(1), 49–69. https://doi.org/10.1287/mnsc.1.1.49  
  `charnescooper1954` · Método del salto de piedra en piedra (ciclos de mejora).
- Charnes, A. y Cooper, W. W. (1961). *Management models and industrial applications of linear programming*. Wiley. — _Obra en 2 volúmenes (New York: Wiley, 1961; LCCN 61014807). El catálogo no da ISBN (es anterior al ISBN)._  
  `charnescooper1961` · Programación por metas.
- Charnes, A., Cooper, W. W. y Ferguson, R. O. (1955). Optimal estimation of executive compensation by linear programming. *Management Science*, *1*(2), 138–151. https://doi.org/10.1287/mnsc.1.2.138  
  `charnes1955` · Origen de la programación por metas.
- Chinneck, J. W. (2008). *Feasibility and infeasibility in optimization: Algorithms and computational methods*. Springer. https://doi.org/10.1007/978-0-387-74932-7 — _Serie International Series in Operations Research & Management Science. Datos confirmados en RePEc/IDEAS (registro spr/isorms 978-0-387-74932-7)._  
  `chinneck2008` · Subsistemas irreducibles infactibles (IIS) y relajación de factibilidad: el diagnóstico de los modelos no factibles.
- Clarke, G. y Wright, J. W. (1964). Scheduling of vehicles from a central depot to a number of delivery points. *Operations Research*, *12*(4), 568–581. https://doi.org/10.1287/opre.12.4.568  
  `clarkewright1964` · Método de los ahorros s(i,j) = d(0,i) + d(0,j) − d(i,j).
- Cobb, C. W. y Douglas, P. H. (1928). A theory of production. *The American Economic Review*. https://www.jstor.org/stable/1811556 **[Por verificar: Confirmar en JSTOR el volumen, el número (suplemento) y las páginas: se citan como vol. 18, n.º 1, suplemento (Papers and Proceedings, marzo de 1928), pp. 139–165, pero JSTOR no se pudo leer (pide JavaScript).]** — _No tiene DOI en Crossref. La URL estable de JSTOR (1811556) sale en el buscador con el título «A Theory of Production on JSTOR», pero no se pudo abrir la página._  
  `cobbdouglas1928` · Función de producción Cobb-Douglas (ejemplo NLP).
- Colaboradores de statrs. (s. f.). *statrs (versión 0.19)* [Software]. https://github.com/statrs-dev/statrs  
  `statrs` · Distribuciones de probabilidad en Rust: valores Z e inversas de t de Student.
- Croes, G. A. (1958). A method for solving traveling-salesman problems. *Operations Research*, *6*(6), 791–812. https://doi.org/10.1287/opre.6.6.791  
  `croes1958` · Mejora por intercambio de dos arcos (2-opt).
- Croston, J. D. (1972). Forecasting and stock control for intermittent demands. *Operational Research Quarterly*, *23*(3), 289–303. — _Volumen, número y páginas confirmados en EconBiz. La revista se llama hoy Journal of the Operational Research Society. No se pone DOI porque no se pudo confirmar._  
  `croston1972` · Pronóstico de demanda intermitente (método de Croston).
- Crozet, S. (2026). *nalgebra* (Versión 0.35.0) [Software]. https://github.com/dimforge/nalgebra — _Versión 0.35.0, publicada el 24 de mayo de 2026 en crates.io (docs.rs); es la que fija Cargo.lock en IAQSB. Autor según Cargo.toml del crate: Sébastien Crozet; licencia Apache-2.0. Corrección: el sitio oficial es https://nalgebra.rs, no nalgebra.org. Se añade el campo «version», que no está en FORMATO.md, para la mención APA «(Versión 0.35.0) [Software]»._  
  `nalgebra` · Álgebra lineal (cadenas de Markov, regresión).
- Dannenbring, D. G. (1977). An evaluation of flow shop sequencing heuristics. *Management Science*, *23*(11), 1174–1182. https://doi.org/10.1287/mnsc.23.11.1174  
  `dannenbring1977` · Heurística de Dannenbring (aproximación rápida, RA) para el taller de flujo.
- Dantzig, G. B. (1951). Application of the simplex method to a transportation problem. En T. C. Koopmans (Ed.), *Activity analysis of production and allocation* (pp. 359–373). Wiley. **[Por verificar: Confirmar páginas y editorial en un catálogo.]**  
  `dantzig1951` · Método de transporte (potenciales u-v, conocido en los textos como MODI).
- Dantzig, G. B. (1963). *Linear programming and extensions*. Princeton University Press. https://doi.org/10.1515/9781400884179 — _El DOI es el de la edición digital de Princeton University Press; Crossref lo registra con fecha de publicación 1963. No se pone ISBN porque la edición de 1963 es anterior al ISBN (9780691059136 es la reimpresión de la serie Princeton Landmarks)._  
  `dantzig1963` · Método símplex y programación lineal (módulo LP).
- Dantzig, G. B. y Ramser, J. H. (1959). The truck dispatching problem. *Management Science*, *6*(1), 80–91. https://doi.org/10.1287/mnsc.6.1.80  
  `dantzigramser1959` · Origen del problema de ruteo de vehículos (VRP).
- Dantzig, G., Fulkerson, R. y Johnson, S. (1954). Solution of a large-scale traveling-salesman problem. *Journal of the Operations Research Society of America*, *2*(4), 393–410. https://doi.org/10.1287/opre.2.4.393  
  `dantzig1954` · Agente viajero: restricciones de eliminación de subciclos (cortes DFJ).
- De Livera, A. M., Hyndman, R. J. y Snyder, R. D. (2011). Forecasting time series with complex seasonal patterns using exponential smoothing. *Journal of the American Statistical Association*, *106*(496), 1513–1527. https://doi.org/10.1198/jasa.2011.tm09771  
  `delivera2011` · Modelos BATS y TBATS (Box-Cox, estacionalidad trigonométrica, errores ARMA y tendencia).
- DeMatteis, J. J. (1968). An economic lot-sizing technique, I: The part-period algorithm. *IBM Systems Journal*, *7*(1), 30–38. https://doi.org/10.1147/sj.71.0030 — _El título registrado lleva coma: "An economic lot-sizing technique, I: The part-period algorithm"._  
  `dematteis1968` · Balanceo de períodos (part-period balancing) para tamaño de lote.
- Dickie, H. F. (1951). ABC inventory analysis shoots for dollars, not pennies. *Factory Management and Maintenance*, *109*, 92–94. **[Por verificar: Confirmar el número del fascículo (6 o 7).]**  
  `dickie1951` · Origen de la clasificación ABC de inventarios.
- Dijkstra, E. W. (1959). A note on two problems in connexion with graphs. *Numerische Mathematik*, *1*, 269–271. https://doi.org/10.1007/BF01386390  
  `dijkstra1959` · Ruta más corta con costos no negativos (algoritmo de Dijkstra).
- Dodge, H. F. (1943). A sampling inspection plan for continuous production. *The Annals of Mathematical Statistics*, *14*(3), 264–279. https://doi.org/10.1214/aoms/1177731420 — _DOI corregido: 10.1214/aoms/1177731358 corresponde a Wald y Wolfowitz (1943), "An exact test for randomness...", 14(4), 378–388. El DOI correcto de Dodge es 10.1214/aoms/1177731420._  
  `dodge1943` · Plan de muestreo para producción continua CSP-1.
- Dodge, H. F. (1955a). Chain sampling inspection plan. *Industrial Quality Control*. **[Por verificar: Falta confirmar volumen, número y páginas en una fuente primaria (archivo de ASQ o catálogo); la ficha de ASQ (https://asq.org/qic/display-item/index.html?item=4086) no se pudo leer.]** — _Solo se encontró en fuentes secundarias, p. ej. la documentación del paquete R Dodge (https://search.r-project.org/CRAN/refmans/Dodge/html/ChainBinomial.html), que la cita como 11(4), 10–13, igual que tu dato._  
  `dodge1955` · Muestreo en cadena (ChSP-1).
- Dodge, H. F. (1955b). Skip-lot sampling plan. *Industrial Quality Control*. **[Por verificar: No se encontró en línea un registro primario (ASQ, catálogo) con volumen, número y páginas; el dato propuesto 11(5), 3–5 está sin confirmar.]**  
  `dodge1955skip` · Muestreo con salto de lotes (plan original de Dodge).
- Dodge, H. F. y Romig, H. G. (1959). *Sampling inspection tables: Single and double sampling* (2.ª ed.). Wiley. — _Catálogo de Penn: "Second edition, revised and expanded", New York, Wiley, 1959, 224 p., OCLC 568319._  
  `dodgeromig1959` · Planes de muestreo simple y doble; AOQ, AOQL e inspección total promedio (ATI).
- Edmonds, J. y Karp, R. M. (1972). Theoretical improvements in algorithmic efficiency for network flow problems. *Journal of the ACM*, *19*(2), 248–264. https://doi.org/10.1145/321694.321699  
  `edmondskarp1972` · Caminos de aumento más cortos (BFS): flujo máximo en O(V·E²).
- Erlang, A. K. (1909). Sandsynlighedsregning og telefonsamtaler. *Nyt Tidsskrift for Matematik*. **[Por verificar: Falta confirmar en JSTOR o un catálogo la sección B, el volumen 20 y las páginas 33–39, que solo aparecen en citas secundarias.]** — _JSTOR y Google Books no se pudieron consultar desde aquí (robots.txt); Crossref no tiene registro. Las citas secundarias coinciden en "Nyt Tidsskrift for Matematik B, 20, 33–39" (p. ej., https://www.scirp.org/reference/referencespapers?referenceid=2982431)._  
  `erlang1909` · Origen de la teoría de colas: llegadas de Poisson y tiempos exponenciales.
- Fiacco, A. V. y McCormick, G. P. (1968). *Nonlinear programming: Sequential unconstrained minimization techniques*. Wiley. — _Sin ISBN: el único que aparece en el catálogo viene del enlace de la portada y no se pudo confirmar. No usar el DOI 10.1137/1.9781611971316, que es la reimpresión de SIAM (1990), no la edición de Wiley de 1968._  
  `fiaccomccormick1968` · Método de penalización (el que usa WinQSB en NLP).
- Floyd, R. W. (1962). Algorithm 97: Shortest path. *Communications of the ACM*, *5*(6), 345. https://doi.org/10.1145/367766.368168  
  `floyd1962` · Rutas más cortas entre todos los pares (Floyd-Warshall).
- Ford, L. R., Jr. y Fulkerson, D. R. (1956). Maximal flow through a network. *Canadian Journal of Mathematics*, *8*, 399–404. https://doi.org/10.4153/CJM-1956-045-5  
  `fordfulkerson1956` · Flujo máximo por caminos de aumento; teorema de flujo máximo y corte mínimo.
- Francis, R. L., McGinnis, L. F., Jr. y White, J. A. (1992). *Facility layout and location: An analytical approach* (2.ª ed.). Prentice Hall. — _Autores, edición, editorial y año también en https://library.herts.ac.uk/bib/14178 (allí sin 'Jr.'). Lugar: Englewood Cliffs, N.J. No se encontró ISBN en un catálogo._  
  `francis1992` · Localización de instalaciones con distancia rectilínea, euclidiana y euclidiana al cuadrado.
- Frazelle, E. H. y Sojo Quirós, R. (2007). *Logística de almacenamiento y manejo de materiales de clase mundial*. Grupo Editorial Norma.  
  `frazelle2007` · Diseño de almacenes, unidades de carga y manejo de materiales.
- Gaither, N. y Frazier, G. (2000). *Administración de producción y operaciones* (8.ª ed.). International Thomson Editores.  
  `gaither2000` · Candidato de origen del ejemplo Sell-Rite (inventario de seguridad).
- Gavish, B. y Graves, S. C. (1978). *The travelling salesman problem and related problems* (Working Paper OR 078-78). Massachusetts Institute of Technology, Operations Research Center. https://dspace.mit.edu/handle/1721.1/5363  
  `gavishgraves1978` · Agente viajero: formulación de flujo de un solo producto.
- Giffler, B. y Thompson, G. L. (1960). Algorithms for solving production-scheduling problems. *Operations Research*, *8*(4), 487–503. https://doi.org/10.1287/opre.8.4.487  
  `gifflerthompson1960` · Generación de programas activos en el taller de trabajos (job shop) con reglas de despacho.
- Gillett, B. E. y Miller, L. R. (1974). A heuristic algorithm for the vehicle-dispatch problem. *Operations Research*, *22*(2), 340–349. https://doi.org/10.1287/opre.22.2.340  
  `gillettmiller1974` · Método del barrido (sweep).
- Goulart, P. J. y Chen, Y. (2024). Clarabel: An interior-point solver for conic programs with quadratic objectives. *arXiv*. https://doi.org/10.48550/arXiv.2405.12762 — _Preprint (arXiv:2405.12762 [math.OC], v1 del 21 de mayo de 2024); en APA lleva [Preprint]. El artículo ya salió en revista: Mathematical Programming Computation (Springer, 2026, doi 10.1007/s12532-026-00320-7, publicado en línea el 21 de mayo de 2026 según el repositorio ORA de Oxford). No se pudieron confirmar el volumen ni las páginas porque Springer y Crossref respondieron con límite de solicitudes. Conviene pasar a la versión de revista cuando se confirmen._  
  `goulartchen2024` · Solver Clarabel (programación cuadrática y subproblemas SQP).
- Gupta, J. N. D. (1971). A functional heuristic algorithm for the flowshop scheduling problem. *Operational Research Quarterly*, *22*(1), 39–47. https://doi.org/10.1057/jors.1971.18 — _Se usa el DOI del editor (10.1057/jors.1971.18). No se pudo consultar el registro del DOI de JSTOR 10.2307/3008015 (límite de peticiones), por lo que no se incluye. El editor registra hoy la revista como Journal of the Operational Research Society; en 1971 se llamaba Operational Research Quarterly._  
  `gupta1971` · Heurística de Gupta para el taller de flujo.
- Hadley, G. y Whitin, T. M. (1963). *Analysis of inventory systems*. Prentice-Hall.  
  `hadleywhitin1963` · Descuentos por cantidad y punto de reorden con demanda aleatoria.
- Harris, F. W. (1913). How many parts to make at once. *Factory, The Magazine of Management*, *10*(2), 135–136, 152. — _Reimpreso en Operations Research, 38(6), 947–950 (1990), https://doi.org/10.1287/opre.38.6.947._  
  `harris1913` · Origen del modelo de lote económico (EOQ).
- Heizer, J. y Render, B. (2014). *Principios de administración de operaciones* (9.ª ed.). Pearson Educación. — _Edición, año (2014), editorial e ISBN confirmados también en Open Library (https://openlibrary.org/isbn/9786073223362) y en el catálogo de la UNI de Nicaragua (Pearson Educación, México, 2014). Algunos catálogos y librerías ponen a Render primero; se sigue el orden Heizer–Render de Dykinson y de la obra original._  
  `heizerrender2014` · Planificación agregada, MRP, balanceo de líneas y programación de trabajos.
- Heizer, J., Render, B. y Munson, C. (2017). *Operations management: Sustainability and supply chain management* (12.ª ed.). Pearson. — _Ese catálogo da 2016 (fecha de salida); Bookshare (https://www.bookshare.org/browse/book/2287522) da copyright 2017, Pearson Education, ISBN 9780134130422; APA usa el año de copyright._  
  `heizer2017` · Planificación agregada, MRP, balanceo de líneas y programación de trabajos.
- Held, M. y Karp, R. M. (1962). A dynamic programming approach to sequencing problems. *Journal of the Society for Industrial and Applied Mathematics*, *10*(1), 196–210. https://doi.org/10.1137/0110015  
  `heldkarp1962` · Agente viajero exacto por programación dinámica, O(n²·2ⁿ).
- Helgeson, W. B. y Birnie, D. P. (1961). Assembly line balancing using the ranked positional weight technique. *Journal of Industrial Engineering*, *12*(6), 394–398. **[Por verificar: No se encontró en línea un registro del editor o de un catálogo que confirme volumen, número y páginas (la revista no tiene DOI); solo aparece en citas secundarias.]**  
  `helgesonbirnie1961` · Balanceo de líneas por el método del peso posicional (RPW).
- Hillier, F. S. y Lieberman, G. J. (2015a). *Introduction to operations research* (10.ª ed.). McGraw-Hill Education. — _Índice del editor: cap. 9 = transporte y asignación; cap. 10 = modelos de optimización de redes (ejemplo del Parque Seervada). En la 9.ª ed. (2010) son los caps. 8 y 9._  
  `hillier2015en` · Ejemplo prototipo del Parque Seervada.
- Hillier, F. S. y Lieberman, G. J. (2015b). *Investigación de operaciones* (10.ª ed.). McGraw-Hill Interamericana. **[Por verificar: Confirmar el título exacto de la 10.ª ed. en español (puede ser «Introducción a la investigación de operaciones») y el ISBN en el ejemplar o en el catálogo de McGraw-Hill.]**  
  `hillier2015` · Texto general de investigación de operaciones.
- Hitchcock, F. L. (1941). The distribution of a product from several sources to numerous localities. *Journal of Mathematics and Physics*, *20*, 224–230. https://doi.org/10.1002/sapm1941201224  
  `hitchcock1941` · Formulación del problema de transporte.
- Ho, J. C. y Chang, Y.-L. (1991). A new heuristic for the n-job, M-machine flow-shop problem. *European Journal of Operational Research*, *52*(2), 194–202. https://doi.org/10.1016/0377-2217(91)90080-F  
  `hochang1991` · Heurística de Ho y Chang para el taller de flujo.
- Holt, C. C. (2004). Forecasting seasonals and trends by exponentially weighted moving averages. *International Journal of Forecasting*, *20*(1), 5–10. https://doi.org/10.1016/j.ijforecast.2003.09.015 — _El resumen indica que es la versión reimpresa del informe de 1957 a la Office of Naval Research ("ONR 52"); "Carnegie Institute of Technology" no aparece en las páginas consultadas. En APA puede citarse como (Holt, 1957/2004)._  
  `holt2004` · Suavizamiento exponencial con tendencia (método de Holt).
- Holt, C. C., Modigliani, F., Muth, J. F. y Simon, H. A. (1960). *Planning production, inventories, and work force*. Prentice-Hall. — _Los catálogos también registran contribuciones de C. P. Bonini (no es coautor principal). Serie Prentice-Hall International Series in Management; Englewood Cliffs, N.J. No se encontró ISBN (anterior al sistema ISBN)._  
  `holt1960` · Planificación agregada de la producción.
- Huangfu, Q. y Hall, J. A. J. (2018). Parallelizing the dual revised simplex method. *Mathematical Programming Computation*, *10*(1), 119–142. https://doi.org/10.1007/s12532-017-0130-5  
  `huangfuhall2018` · HiGHS: el solucionador lineal y entero que usa IAQSB.
- Hundal, T. S. y Rajgopal, J. (1988). An extension of Palmer's heuristic for the flow shop scheduling problem. *International Journal of Production Research*, *26*(6), 1119–1124. https://doi.org/10.1080/00207548808947922  
  `hundalrajgopal1988` · Heurística de Hundal y Rajgopal (extensión de Palmer) para el taller de flujo.
- Hurwicz, L. (1951). *Optimality criteria for decision making under ignorance* (Discussion Paper Statistics No. 370). Cowles Commission. **[Por verificar: No se encontró la ficha del archivo de Cowles ni un catálogo para el Discussion Paper Statistics No. 370; solo hay citas secundarias.]** — _Alternativa verificable: Arrow, K. J. y Hurwicz, L. (1977). Appendix: An optimality criterion for decision-making under ignorance. En K. J. Arrow y L. Hurwicz (Eds.), Studies in resource allocation processes (pp. 461–472). Cambridge University Press. https://doi.org/10.1017/CBO9780511752940.015. Otro documento de Hurwicz en el archivo de Cowles, Statistics 355 (feb. 1951), es otro trabajo ("The generalized Bayes-minimax principle")._  
  `hurwicz1951` · Criterio de Hurwicz (coeficiente de optimismo).
- Hyndman, R. J. y Athanasopoulos, G. (2018). *Forecasting: Principles and practice* (2.ª ed.). OTexts. https://otexts.com/fpp2/ — _Melbourne, Australia. Edición, año, editorial e ISBN confirmados en catálogos de bibliotecas (Texas A&M) y librerías. Es la edición que usa el paquete forecast de R (la 3.ª, de 2021, usa fable)._  
  `hyndman2018fpp` · Exploración de series de tiempo, descomposición, suavización exponencial (ETS), ARIMA, TBATS y evaluación de pronósticos.
- Hyndman, R. J. y Khandakar, Y. (2008). Automatic time series forecasting: The forecast package for R. *Journal of Statistical Software*, *27*(3), 1–22. https://doi.org/10.18637/jss.v027.i03  
  `hyndmankhandakar2008` · Algoritmos automáticos de ARIMA (Hyndman-Khandakar) y de suavización exponencial del paquete forecast.
- Hyndman, R. J. y Koehler, A. B. (2006). Another look at measures of forecast accuracy. *International Journal of Forecasting*, *22*(4), 679–688. https://doi.org/10.1016/j.ijforecast.2006.03.001  
  `hyndmankoehler2006` · Medidas de error de pronóstico: MAD, MSE y MAPE.
- Hyndman, R. J., Koehler, A. B., Snyder, R. D. y Grose, S. (2002). A state space framework for automatic forecasting using exponential smoothing methods. *International Journal of Forecasting*, *18*(3), 439–454. — _Volumen, número y páginas confirmados en RePEc y EconBiz. No se pone DOI porque no se pudo confirmar._  
  `hyndman2002ets` · Modelos ETS en espacio de estados con innovaciones y selección automática.
- Instituto Colombiano de Normas Técnicas y Certificación. (1999). *Estiba intercambiable de madera, no reversible, de cuatro entradas (NTC 4680)*. ICONTEC.  
  `ntc4680` · Estiba estándar colombiana de 1200 × 1000 mm.
- Johnson, S. M. (1954). Optimal two- and three-stage production schedules with setup times included. *Naval Research Logistics Quarterly*, *1*(1), 61–68. https://doi.org/10.1002/nav.3800010110  
  `johnson1954` · Regla de Johnson para secuenciar trabajos en dos (y tres) máquinas.
- Juran, J. M. (Ed.). (1951). *Quality-control handbook*. McGraw-Hill. — _El título de la 1.ª ed. lleva guion: "Quality-control handbook" (New York, McGraw-Hill, 1951, viii, 800 p.; serie McGraw-Hill industrial organization and management). Confirmado también en https://catalogosiidca.csuca.org/Record/UCR.000321809._  
  `juran1951` · Análisis de Pareto de los defectos.
- Kane, V. E. (1986). Process capability indices. *Journal of Quality Technology*, *18*(1), 41–52. https://doi.org/10.1080/00224065.1986.11978984  
  `kane1986` · Índices de capacidad de proceso Cp y Cpk.
- Karush, W. (1939). *Minima of functions of several variables with inequalities as side conditions* [Tesis de maestría, University of Chicago]. — _Corrección del título: es «...as side conditions», no «...as side constraints». Así lo dan la reimpresión de Springer (Giorgi y Kjeldsen, Eds., Traces and emergence of nonlinear programming, 2014, pp. 217–245, doi 10.1007/978-3-0348-0439-4_10) y Cottle (2012, Documenta Mathematica, Extra Volume ISMP). Es una tesis de maestría del Department of Mathematics de la University of Chicago (diciembre de 1939). En APA: [Tesis de maestría, University of Chicago]. No se vio el registro en un catálogo de biblioteca._  
  `karush1939` · Condiciones de Karush-Kuhn-Tucker.
- Kelley, J. E., Jr. (1961). Critical-path planning and scheduling: Mathematical basis. *Operations Research*, *9*(3), 296–320. https://doi.org/10.1287/opre.9.3.296  
  `kelley1961` · Compresión de tiempos de proyecto al menor costo (crashing).
- Kelley, J. E., Jr. y Walker, M. R. (1959). Critical-path planning and scheduling. En *Papers presented at the December 1–3, 1959, Eastern Joint IRE-AIEE-ACM Computer Conference* (pp. 160–173). Association for Computing Machinery. https://doi.org/10.1145/1460299.1460318 — _Crossref registra el volumen como 'Papers presented at the December 1-3, 1959, eastern joint IRE-AIEE-ACM computer conference' (IRE-AIEE-ACM '59 Eastern, Boston), publicado por ACM; se cita como capítulo de actas sin editores._  
  `kelleywalker1959` · Método de la ruta crítica (CPM).
- Kemeny, J. G. y Snell, J. L. (1960). *Finite Markov chains*. D. Van Nostrand. — _Princeton, NJ, D. Van Nostrand, 1960, viii, 210 p. (The university series in undergraduate mathematics); LCCN 59015644._  
  `kemenysnell1960` · Cadenas de Markov: estado estable, estados absorbentes y tiempos de primera pasada.
- Kendall, D. G. (1953). Stochastic processes occurring in the theory of queues and their analysis by the method of the imbedded Markov chain. *The Annals of Mathematical Statistics*, *24*(3), 338–354. https://doi.org/10.1214/aoms/1177728975  
  `kendall1953` · Notación de Kendall y análisis de la cola M/G/1 con cadena de Markov inmersa.
- Kruskal, J. B., Jr. (1956). On the shortest spanning subtree of a graph and the traveling salesman problem. *Proceedings of the American Mathematical Society*, *7*(1), 48–50. https://doi.org/10.1090/S0002-9939-1956-0078686-7  
  `kruskal1956` · Árbol de expansión mínima: algoritmo de Kruskal.
- Kuhn, H. W. (1955). The Hungarian method for the assignment problem. *Naval Research Logistics Quarterly*, *2*(1–2), 83–97. https://doi.org/10.1002/nav.3800020109  
  `kuhn1955` · Problema de asignación: método húngaro.
- Kuhn, H. W. y Tucker, A. W. (1951). Nonlinear programming. En J. Neyman (Ed.), *Proceedings of the Second Berkeley Symposium on Mathematical Statistics and Probability* (pp. 481–492). University of California Press. — _Páginas 481–492 y año 1951 según zbMATH (Zbl 0044.05903); editor J. Neyman, University of California Press (Berkeley y Los Ángeles, 1951) según la reseña del Bulletin of the AMS. No tiene DOI; Project Euclid no se pudo leer (bloqueo antibots)._  
  `kuhntucker1951` · Condiciones de Karush-Kuhn-Tucker.
- Land, A. H. y Doig, A. G. (1960). An automatic method of solving discrete programming problems. *Econometrica*, *28*(3), 497–520. https://doi.org/10.2307/1910129  
  `landdoig1960` · Ramificación y acotamiento (branch and bound) para programas enteros.
- Law, A. M. (2015). *Simulation modeling and analysis* (5.ª ed.). McGraw-Hill Education.  
  `law2015` · Simulación de eventos discretos (módulo QSS).
- Lin, S. y Kernighan, B. W. (1973). An effective heuristic algorithm for the traveling-salesman problem. *Operations Research*, *21*(2), 498–516. https://doi.org/10.1287/opre.21.2.498  
  `lin1973` · Heurística de Lin-Kernighan (k-opt variable) para instancias grandes.
- Little, J. D. C. (1961). A proof for the queuing formula: L = λW. *Operations Research*, *9*(3), 383–387. https://doi.org/10.1287/opre.9.3.383  
  `little1961` · Ley de Little (L = λW).
- Ljung, G. M. y Box, G. E. P. (1978). On a measure of lack of fit in time series models. *Biometrika*, *65*(2), 297–303. https://doi.org/10.1093/biomet/65.2.297  
  `ljungbox1978` · Prueba de Ljung-Box para la autocorrelación de los residuos.
- Long, D. (2003). *International logistics: Global supply chain management*. Kluwer Academic Publishers. — _Las diapositivas lo citan como Long (2004); la edición encontrada es de 2003._  
  `long2003` · Importancia económica del transporte; unitarización de la carga.
- Malcolm, D. G., Roseboom, J. H., Clark, C. E. y Fazar, W. (1959). Application of a technique for research and development program evaluation. *Operations Research*, *7*(5), 646–669. https://doi.org/10.1287/opre.7.5.646  
  `malcolm1959` · Técnica PERT: tres estimaciones de duración y media y varianza de la distribución beta.
- Markowitz, H. (1952). Portfolio selection. *The Journal of Finance*, *7*(1), 77–91. https://doi.org/10.1111/j.1540-6261.1952.tb01525.x  
  `markowitz1952` · Cartera de mínimo riesgo (modelo media-varianza).
- Miller, C. E., Tucker, A. W. y Zemlin, R. A. (1960). Integer programming formulation of traveling salesman problems. *Journal of the ACM*, *7*(4), 326–329. https://doi.org/10.1145/321043.321046  
  `miller1960` · Agente viajero: restricciones MTZ u_i − u_j + p·x_ij ≤ p − 1 (ciudad base 0 y n ciudades; p = n). Con N = n + 1 nodos es la forma de la clase: U_i − U_j + (N − 1)·X_ij ≤ N − 2.
- Montgomery, D. C. (2004). *Control estadístico de la calidad* (3.ª ed.). Limusa Wiley. — _Traducción de Rodolfo Piña García de Introduction to statistical quality control. Los catálogos registran reimpresiones de la 3.ª edición (2006, 2008, 2013, 2016) con el mismo ISBN 978-968-18-6234-3 (ISBN-10 968-18-6234-1)._  
  `montgomery2004` · Control estadístico de la calidad (traducción al español).
- Montgomery, D. C. (2020). *Introduction to statistical quality control* (8.ª ed.). Wiley. — _El año es 2020 (John Wiley & Sons, Hoboken, NJ). Los catálogos usan varios ISBN según el formato: 9781119723097 (rústica) y 9781119657071. No se confirmó 9781119399308 en ningún catálogo._  
  `montgomery2020` · Control estadístico de la calidad (texto original en inglés).
- Mora García, L. A. (2008). *Indicadores de la gestión logística* (2.ª ed.). Ecoe Ediciones.  
  `mora2008` · Indicadores clave de la gestión logística.
- Müller, S. (s. f.). *anofox-forecast (versión 0.21)* [Software]. https://github.com/sipemu/anofox-forecast — _Autor, versión y licencia (MIT) según los metadatos del paquete en crates.io._  
  `anofox2026` · Biblioteca de pronósticos en Rust (AutoARIMA, AutoETS, TBATS, Theta, Croston): el motor nativo de IAQSB.
- Nawaz, M., Enscore, E. E., Jr. y Ham, I. (1983). A heuristic algorithm for the m-machine, n-job flow-shop sequencing problem. *Omega*, *11*(1), 91–95. https://doi.org/10.1016/0305-0483(83)90088-9  
  `neh1983` · Heurística NEH para el taller de flujo.
- Nocedal, J. y Wright, S. J. (2006). *Numerical optimization* (2.ª ed.). Springer. https://doi.org/10.1007/978-0-387-40065-5  
  `nocedalwright2006` · Programación cuadrática sucesiva (SQP) y actualización BFGS.
- Orden, A. (1956). The transhipment problem. *Management Science*, *2*(3), 276–285. https://doi.org/10.1287/mnsc.2.3.276  
  `orden1956` · Problema de transbordo.
- Orlicky, J. (1975). *Material requirements planning: The new way of life in production and inventory management*. McGraw-Hill.  
  `orlicky1975` · Planificación de requerimientos de materiales (MRP).
- Page, E. S. (1954). Continuous inspection schemes. *Biometrika*, *41*(1/2), 100–115. https://doi.org/10.1093/biomet/41.1-2.100  
  `page1954` · Gráficos de sumas acumuladas (CUSUM).
- Palmer, D. S. (1965). Sequencing jobs through a multi-stage process in the minimum total time—A quick method of obtaining a near optimum. *Operational Research Quarterly*, *16*(1), 101–107. https://doi.org/10.1057/jors.1965.8 — _Existen los dos DOI: 10.1057/jors.1965.8 (editor; se prefiere) y 10.2307/3006688 (JSTOR). El editor registra hoy la revista como Journal of the Operational Research Society y JSTOR como 'OR'; se usa el nombre con que se citaba en 1965, Operational Research Quarterly (pasó a llamarse Journal of the Operational Research Society en 1978)._  
  `palmer1965` · Heurística de Palmer (índice de pendiente) para el taller de flujo.
- Panwalkar, S. S. y Iskander, W. (1977). A survey of scheduling rules. *Operations Research*, *25*(1), 45–61. https://doi.org/10.1287/opre.25.1.45  
  `panwalkar1977` · Reglas de despacho o prioridad (SPT, EDD, CR, etc.).
- Perry, R. L. (1973). Skip-lot sampling plans. *Journal of Quality Technology*, *5*(3), 123–130. https://doi.org/10.1080/00224065.1973.11980585  
  `perry1973` · Muestreo con salto de lotes (planes SkSP-2).
- Pinedo, M. L. (2016). *Scheduling: Theory, algorithms, and systems* (5.ª ed.). Springer. https://doi.org/10.1007/978-3-319-26580-3 — _Edición (5.ª) y subtítulo confirmados en el catálogo de la British Library; DOI e ISBN (impreso 978-3-319-26578-0; electrónico 978-3-319-26580-3) confirmados en Crossref (Springer International Publishing, Cham)._  
  `pinedo2016` · Criterios de desempeño en la programación de trabajos.
- Powell, M. J. D. (1978). A fast algorithm for nonlinearly constrained optimization calculations. En G. A. Watson (Ed.), *Numerical analysis (Lecture Notes in Mathematics, Vol. 630)* (pp. 144–157). Springer. https://doi.org/10.1007/BFb0067703 — _Título completo del volumen: Numerical analysis: Proceedings of the Biennial Conference held at Dundee, June 28–July 1, 1977._  
  `powell1978` · SQP con BFGS amortiguado.
- Powell, M. J. D. (1994). A direct search optimization method that models the objective and constraint functions by linear interpolation. En S. Gomez y J.-P. Hennart (Eds.), *Advances in optimization and numerical analysis (Mathematics and Its Applications, Vol. 275)* (pp. 51–67). Kluwer. https://doi.org/10.1007/978-94-015-8330-5_4 — _Editorial: el libro impreso de 1994 es de Kluwer (Dordrecht), según el catálogo; Crossref lo atribuye hoy a Springer Netherlands, que adquirió el fondo. ISBN 0-7923-2673-3 (978-0-7923-2673-1) es el de la edición de Kluwer._  
  `powell1994` · COBYLA (punto de partida sin derivadas).
- Prim, R. C. (1957). Shortest connection networks and some generalizations. *Bell System Technical Journal*, *36*(6), 1389–1401. https://doi.org/10.1002/j.1538-7305.1957.tb01515.x  
  `prim1957` · Árbol de expansión mínima: algoritmo de Prim.
- Raiffa, H. y Schlaifer, R. (1961). *Applied statistical decision theory*. Harvard University, Graduate School of Business Administration, Division of Research. — _Catálogo: Boston, Division of Research, Graduate School of Business Administration, Harvard University, c1961._  
  `raiffaschlaifer1961` · Teorema de Bayes en decisiones y valor esperado de la información.
- Reinfeld, N. V. y Vogel, W. R. (1958). *Mathematical programming*. Prentice-Hall.  
  `reinfeldvogel1958` · Método de aproximación de Vogel (VAM) para la solución inicial.
- Render, B., Stair, R. M., Jr. y Hanna, M. E. (2012). *Métodos cuantitativos para los negocios* (11.ª ed.). Pearson. — _El catálogo confirma la 11.ª edición en español, Pearson, México, 2012, con Render, Stair Jr. y Hanna (Hale se suma en la 12.ª edición en español, 2016). No se encontró el ISBN en un catálogo: 9786073212649 aparece en librerías y en Open Library, pero sin datos de edición fiables, así que se deja vacío._  
  `render2012` · Ejemplos de inventarios, pronósticos, colas y cadenas de Markov (General Foundry, Harrison Electric, Triple A Construction, García-Golding Recycling, Brass Department Store).
- Render, B., Stair, R. M., Jr., Hanna, M. E. y Hale, T. S. (2018). *Quantitative analysis for management* (13.ª ed.). Pearson. — _Hale se incorporó en la 12.ª edición (2015), no en la 13.ª (https://research.kwaretech.com/Record/in00000484058?print=1). El ISBN 9780134543161 es el de la edición estadounidense; la Global Edition (Pearson Education Limited, Harlow, 2018) aparece en la British Library con ISBN 9781292217680 (y relacionado 9781292217659)._  
  `render2018` · Texto de referencia en inglés para los métodos cuantitativos y sus ejemplos.
- Richards, V. D. y Laughlin, E. J. (1980). A cash conversion cycle approach to liquidity analysis. *Financial Management*, *9*(1), 32–38. https://doi.org/10.2307/3665310  
  `richards1980` · Ciclo de conversión del efectivo (días de inventario + cartera − pago).
- Roberts, S. W. (1959). Control chart tests based on geometric moving averages. *Technometrics*, *1*(3), 239–250. https://doi.org/10.1080/00401706.1959.10489860  
  `roberts1959` · Gráficos de promedio móvil geométrico o exponencialmente ponderado (GMA/EWMA).
- Ropke, S. y Pisinger, D. (2006). An adaptive large neighborhood search heuristic for the pickup and delivery problem with time windows. *Transportation Science*, *40*(4), 455–472. https://doi.org/10.1287/trsc.1050.0135  
  `ropke2006` · Búsqueda adaptativa de vecindario grande (ALNS).
- Rosenbrock, H. H. (1960). An automatic method for finding the greatest or least value of a function. *The Computer Journal*, *3*(3), 175–184. https://doi.org/10.1093/comjnl/3.3.175  
  `rosenbrock1960` · Función de prueba de Rosenbrock (ejemplo NLP).
- Rosenkrantz, D. J., Stearns, R. E. y Lewis, P. M., II. (1977). An analysis of several heuristics for the traveling salesman problem. *SIAM Journal on Computing*, *6*(3), 563–581. https://doi.org/10.1137/0206041  
  `rosenkrantz1977` · Heurísticas del vecino más cercano (NNH) y de inserción más barata (CIH) y sus garantías.
- Russell, E. J. (1969). Extension of Dantzig's algorithm to finding an initial near-optimal basis for the transportation problem. *Operations Research*, *17*(1), 187–191. https://doi.org/10.1287/opre.17.1.187  
  `russell1969` · Método de aproximación de Russell para la solución inicial.
- Sarache, W. A. (2026a). *Almacenes y centros de distribución* [Diapositivas de clase del curso Taller de Producción y Logística, Ingeniería Industrial]. Universidad Nacional de Colombia. — _Curso Taller de Producción y Logística de la carrera de Ingeniería Industrial de la Universidad Nacional de Colombia, confirmado por un estudiante del curso. El año sale de los metadatos de los archivos._  
  `sarache2026almacenes` · Paletizado, sistemas de almacenaje y dimensionado del almacén.
- Sarache, W. A. (2026b). *Gestión de inventarios* [Diapositivas de clase del curso Taller de Producción y Logística, Ingeniería Industrial]. Universidad Nacional de Colombia. — _Curso Taller de Producción y Logística de la carrera de Ingeniería Industrial de la Universidad Nacional de Colombia, confirmado por un estudiante del curso. El año sale de los metadatos de los archivos._  
  `sarache2026inventarios` · EOQ, POQ, descuentos, ABC, demanda variable y efecto de los inventarios en el ROI, la liquidez y el capital de trabajo.
- Sarache, W. A. (2026c). *Logística del transporte: Introducción, modos y medios* [Diapositivas de clase del curso Taller de Producción y Logística, Ingeniería Industrial]. Universidad Nacional de Colombia. — _Curso Taller de Producción y Logística de la carrera de Ingeniería Industrial de la Universidad Nacional de Colombia, confirmado por un estudiante del curso. El año sale de los metadatos de los archivos._  
  `sarache2026transporte` · Modos y medios de transporte; trade-off transporte-inventarios.
- Sarache, W. A. (2026d). *Problemas de ruteo urbano* [Diapositivas de clase del curso Taller de Producción y Logística, Ingeniería Industrial]. Universidad Nacional de Colombia. — _Curso Taller de Producción y Logística de la carrera de Ingeniería Industrial de la Universidad Nacional de Colombia, confirmado por un estudiante del curso. El año sale de los metadatos de los archivos._  
  `sarache2026ruteo` · TSP (NNH, CIH, modelo con y sin relajación de subtours), barrido y VRP.
- Sarache, W. A. (2026e). *Redes logísticas y ruteamiento* [Diapositivas de clase del curso Taller de Producción y Logística, Ingeniería Industrial]. Universidad Nacional de Colombia. — _Curso Taller de Producción y Logística de la carrera de Ingeniería Industrial de la Universidad Nacional de Colombia, confirmado por un estudiante del curso. El año sale de los metadatos de los archivos._  
  `sarache2026redes` · Transbordo (casos 1 a 3), ruta más corta, árbol de expansión mínima y flujo máximo.
- Sarache, W. A. (2026f). *Taller 1: Indicadores logísticos* [Taller de clase del curso Taller de Producción y Logística, Ingeniería Industrial]. Universidad Nacional de Colombia. **[Por verificar: No se tuvo el enunciado: los datos se tomaron del documento de ejercicios resueltos del grupo. Falta el título exacto del taller.]** — _Curso Taller de Producción y Logística de la carrera de Ingeniería Industrial de la Universidad Nacional de Colombia, confirmado por un estudiante del curso. El año sale de los metadatos de los archivos._  
  `sarache2026taller1` · Indicadores de servicio y costo logístico a partir de 30 pedidos.
- Sarache, W. A. (2026g). *Taller 3: Dimensionado de almacenes* [Taller de clase del curso Taller de Producción y Logística, Ingeniería Industrial]. Universidad Nacional de Colombia. **[Por verificar: No se tuvo el enunciado: los datos se tomaron del documento de ejercicios resueltos del grupo. Falta el título exacto del taller.]** — _Curso Taller de Producción y Logística de la carrera de Ingeniería Industrial de la Universidad Nacional de Colombia, confirmado por un estudiante del curso. El año sale de los metadatos de los archivos._  
  `sarache2026taller3` · Cajas por estiba, espacio requerido y alternativas de almacén propio o rentado.
- Sarache, W. A. (2026h). *Taller: Gestión de inventarios* [Taller de clase del curso Taller de Producción y Logística, Ingeniería Industrial]. Universidad Nacional de Colombia. — _Curso Taller de Producción y Logística de la carrera de Ingeniería Industrial de la Universidad Nacional de Colombia, confirmado por un estudiante del curso. El año sale de los metadatos de los archivos._  
  `sarache2026taller2` · Política (Q, R) con escenarios, ventas perdidas y back orders.
- Sarache, W. A. (s. f.). *Parcial de práctica: Producción y logística* [Examen de práctica del curso Taller de Producción y Logística, Ingeniería Industrial, sin publicar]. Universidad Nacional de Colombia. **[Por verificar: No se tuvo el enunciado original: los tres problemas se tomaron del documento de ejercicios resueltos. Falta el año y el título exacto.]** — _Curso Taller de Producción y Logística de la carrera de Ingeniería Industrial de la Universidad Nacional de Colombia, confirmado por un estudiante del curso. El año sale de los metadatos de los archivos._  
  `sarache_parcial` · Problemas «al revés» de POQ, capital de trabajo y EOQ.
- Savage, L. J. (1951). The theory of statistical decision. *Journal of the American Statistical Association*, *46*(253), 55–67. https://doi.org/10.1080/01621459.1951.10500768  
  `savage1951` · Criterio de mínimo arrepentimiento (minimax regret) de Savage.
- Scarf, H. (1960). The optimality of (S, s) policies in the dynamic inventory problem. En K. J. Arrow, S. Karlin y P. Suppes (Eds.), *Mathematical methods in the social sciences, 1959: Proceedings of the First Stanford Symposium* (pp. 196–202). Stanford University Press. — _El libro completo lleva el subtítulo "Proceedings of the First Stanford Symposium" y es el n.º 4 de la serie Stanford Mathematical Studies in the Social Sciences._  
  `scarf1960` · Optimalidad de las políticas (s, S) y (R, s, S) en inventarios dinámicos.
- Schroeder, R. G. y Goldstein, S. M. (2018). *Operations management in the supply chain: Decisions and cases* (7.ª ed.). McGraw-Hill Education. — _La coautora firma como Susan Meyer Goldstein (las diapositivas la citan como «Meyer Goldstein»). El capítulo 14 es «Independent Demand Inventory»._  
  `schroeder2018` · Inventarios de demanda independiente: EOQ y descuentos por cantidad.
- Shewhart, W. A. (1931). *Economic control of quality of manufactured product*. D. Van Nostrand. — _Reseña en Bulletin of the AMS: New York, Van Nostrand, 1931, xiv+501 pp. El catálogo de Makerere (https://makula.mak.ac.ug/bib/420994) lo confirma; el ISBN 0873890760 de ese registro es de la reimpresión conmemorativa de ASQ Quality Press (1980), no de la edición de 1931, por eso no se incluye._  
  `shewhart1931` · Gráficos de control de Shewhart.
- Shortle, J. F., Thompson, J. M., Gross, D. y Harris, C. M. (2018). *Fundamentals of queueing theory* (5.ª ed.). Wiley. https://doi.org/10.1002/9781119453765 — _Crossref registra por error edition-number 1; la página del autor y la ficha de Wiley confirman la 5.ª edición (2018). ISBN impreso 9781118943526 y electrónico 9781119453765 según Crossref._  
  `gross2018` · Modelos M/M/s, población finita, capacidad limitada, llegadas en lotes, M/G/1 (Pollaczek-Khinchine) y servicio dependiente del estado.
- Silver, E. A. y Meal, H. C. (1973). A heuristic for selecting lot size quantities for the case of a deterministic time-varying demand rate and discrete opportunities for replenishment. *Production and Inventory Management*. **[Por verificar: Falta confirmar volumen, número y páginas (14(2), 64–74 según citas secundarias coincidentes) en una fuente primaria; la revista de APICS no tiene DOI ni registro en Crossref, JSTOR u OpenAlex.]** — _Los datos 14(2), 64–74 aparecen igual en citas secundarias (p. ej., https://www.fww.ovgu.de/fww_media/femm/femm_2009/2009_38.pdf), pero no en un catálogo o página del editor; por eso se dejan vacíos._  
  `silvermeal1973` · Heurística de Silver-Meal para tamaño de lote con demanda variable en el tiempo.
- Silver, E. A., Pyke, D. F. y Thomas, D. J. (2017). *Inventory and production management in supply chains* (4.ª ed.). CRC Press. https://doi.org/10.1201/9781315374406  
  `silver2017` · Inventario de seguridad, sistemas (s, Q), simulación de políticas y clasificación ABC.
- Sinnott, R. W. (1984). Virtues of the haversine. *Sky and Telescope*, *68*(2), 159.  
  `sinnott1984` · Distancia del gran círculo entre dos puntos dados por latitud y longitud (fórmula del haversine).
- Taft, E. W. (1918). The most economical production lot. *The Iron Age*, *101*(18), 1410–1412. — _Datos de fuente secundaria; el título completo lleva un subtítulo sobre fórmulas exactas y aproximadas._  
  `taft1918` · Origen del lote económico de producción (POQ/EPQ) con tasa finita.
- Taha, H. A. (2012). *Investigación de operaciones* (9.ª ed.). Pearson Educación. — _Las diapositivas citan Taha (2011), que corresponde a la 9.ª edición en inglés._  
  `taha2012` · Modelos de transporte y de redes.
- Toth, P. y Vigo, D. (Eds.). (2014). *Vehicle routing: Problems, methods, and applications* (2.ª ed.). Society for Industrial and Applied Mathematics. https://doi.org/10.1137/1.9781611973594  
  `tothvigo2014` · Referencia de conjunto del VRP: variantes, formulaciones y métodos.
- Trigg, D. W. (1964). Monitoring a forecasting system. *Operational Research Quarterly*, *15*(3), 271–274. https://doi.org/10.1057/jors.1964.48 — _Corrección: 10.2307/3007214 es de otro artículo (Maranzana, "On the location of supply points to minimize transport costs"); la copia de JSTOR de Trigg es 10.2307/3007215. Se usa el DOI del editor. Crossref muestra el nombre actual (Journal of the Operational Research Society) y JSTOR el de la época como "OR"; se mantiene Operational Research Quarterly, nombre de la revista en 1964._  
  `trigg1964` · Señal de rastreo para monitorear pronósticos.
- Trigg, D. W. y Leach, A. G. (1967). Exponential smoothing with an adaptive response rate. *Operational Research Quarterly*, *18*(1), 53–59. https://doi.org/10.1057/jors.1967.5 — _Corrección: 10.2307/3010661 no corresponde a este trabajo (es una reseña de Klein, 1998, en JORS 49(1)); se usa el DOI del editor 10.1057/jors.1967.5. Crossref muestra el nombre actual de la revista; en 1967 se llamaba Operational Research Quarterly._  
  `triggleach1967` · Suavizamiento exponencial con tasa de respuesta adaptativa.
- Vazsonyi, A. (1954). The use of mathematics in production and inventory control. *Management Science*, *1*(1), 70–85. https://doi.org/10.1287/mnsc.1.1.70  
  `vazsonyi1954` · Matriz de requerimientos totales («Gozinto») para explotar listas de materiales.
- Vidal, T. (2022). Hybrid genetic search for the CVRP: Open-source implementation and SWAP* neighborhood. *Computers & Operations Research*, *140*, Artículo 105643. https://doi.org/10.1016/j.cor.2021.105643  
  `vidal2022` · Estado del arte en metaheurísticas para el CVRP (HGS).
- Vollmann, T. E., Berry, W. L., Whybark, D. C. y Jacobs, F. R. (2005). *Manufacturing planning and control systems for supply chain management* (5.ª ed.). McGraw-Hill. — _Edición, año, editorial e ISBN confirmados en el catálogo de la biblioteca de East Carolina University (LCCN 2004275109). Hay impresiones con otros ISBN (0-07-112133-1, internacional; 0-07-229990-8, con el título corto Manufacturing planning and control for supply chain management)._  
  `vollmann2005` · Programa maestro, barreras de tiempo, disponible para promesa, plan aproximado de capacidad y MRP.
- von Neumann, J. y Morgenstern, O. (1944). *Theory of games and economic behavior*. Princeton University Press.  
  `vonneumann1944` · Juegos de suma cero y estrategias mixtas.
- Wagner, H. M. y Whitin, T. M. (1958). Dynamic version of the economic lot size model. *Management Science*, *5*(1), 89–96. https://doi.org/10.1287/mnsc.5.1.89  
  `wagnerwhitin1958` · Tamaño de lote óptimo con demanda dinámica determinística (algoritmo de Wagner-Whitin).
- Wald, A. (1947). *Sequential analysis*. Wiley. — _New York, J. Wiley & Sons; London, Chapman & Hall, 1947, xii, 212 p. (Wiley Mathematical Statistics Series)._  
  `wald1947` · Muestreo secuencial por atributos y por variables.
- Warshall, S. (1962). A theorem on Boolean matrices. *Journal of the ACM*, *9*(1), 11–12. https://doi.org/10.1145/321105.321107  
  `warshall1962` · Cierre transitivo; base del algoritmo de Floyd-Warshall.
- Weiszfeld, E. (1937). Sur le point pour lequel la somme des distances de n points donnés est minimum. *Tohoku Mathematical Journal, First Series*, *43*, 355–386. https://www.jstage.jst.go.jp/article/tmj1911/43/0/43_0_355/_article — _Volumen 43 y páginas 355–386 confirmados en J-STAGE, que titula la revista 'Tohoku Mathematical Journal, First Series' (sin número de fascículo ni DOI). En el título original n va en cursiva._  
  `weiszfeld1937` · Localización con distancia euclidiana (algoritmo de Weiszfeld).
- Welch, B. L. (1947). The generalization of 'Student's' problem when several different population variances are involved. *Biometrika*, *34*(1–2), 28–35. https://doi.org/10.1093/biomet/34.1-2.28  
  `welch1947` · Prueba t con varianzas distintas.
- Western Electric Company. (1956). *Statistical quality control handbook*. Western Electric. — _Google Books: Western Electric Company, 1956, 328 p. (OCLC 33858387). HathiTrust (https://catalog.hathitrust.org/Record/007049271) registra una 2.ª ed. de 1958, ©1956, New York, xii, 328 p.; se cita la de 1956._  
  `westernelectric1956` · Reglas de corridas (reglas de Western Electric) para detectar puntos fuera de control.
- Winston, W. L. (2004). *Operations research: Applications and algorithms* (4.ª ed.). Brooks/Cole–Thomson Learning.  
  `winston2004` · Ejemplo Powerco (transporte) y formulación del agente viajero con u_i − u_j + N·x_ij ≤ N − 1.
- Winters, P. R. (1960). Forecasting sales by exponentially weighted moving averages. *Management Science*, *6*(3), 324–342. https://doi.org/10.1287/mnsc.6.3.324  
  `winters1960` · Suavizamiento de Holt-Winters aditivo y multiplicativo con estacionalidad.
- Wolfe, P. (1959). The simplex method for quadratic programming. *Econometrica*, *27*(3), 382–398. https://doi.org/10.2307/1909468  
  `wolfe1959` · Método de Wolfe para programación cuadrática.
- Wren, A. y Holliday, A. (1972). Computer scheduling of vehicles from one or more depots to a number of delivery points. *Operational Research Quarterly*, *23*(3), 333–344. https://doi.org/10.1057/jors.1972.53  
  `wrenholliday1972` · Antecedente del barrido: agrupación de clientes por ángulo polar.

## Origen de los datos de cada ejemplo

| Módulo | Ejemplo | Origen | Fuente | Estado |
|---|---|---|---|---|
| lp | Wyndor Glass (Hillier y Lieberman) | Libro o artículo | Hillier y Lieberman (2015a), cap. 3, secc. 3.1, ejemplo prototipo de Wyndor Glass Co. | Verificada |
| lp | Dieta mínima | Original de IAQSB | Dieta de dos alimentos escrita para IAQSB. Resultado comprobado con un cálculo independiente en Python. | Verificada |
| lp | Producción con lotes enteros | Original de IAQSB | Producción de mesas y sillas en lotes enteros, escrita para IAQSB. Resultado comprobado con un cálculo independiente en Python. | Verificada |
| lp | Mochila binaria | Original de IAQSB | Mochila binaria de cuatro artículos, escrita para IAQSB. Resultado comprobado con un cálculo independiente en Python. | Verificada |
| net | Powerco: distribución de energía (Winston) | Libro o artículo | Winston (2004), secc. 7.1, ejemplo Powerco | Por verificar: Confirmar número de página en la 4.ª ed. (el ejemplo y sus datos coinciden con las notas de curso que siguen el capítulo 7 de Winston). |
| net | Asignación de máquinas | Original de IAQSB | Matriz didáctica de 4 × 4 escrita para IAQSB; la misma matriz circula en tutoriales del método húngaro sin autor identificable. | Verificada |
| net | Parque Seervada: ruta más corta | Libro o artículo | Hillier y Lieberman (2015a), cap. 10, ejemplo prototipo del Parque Seervada (ruta más corta) | Por verificar: Confirmar el número de sección (10.3 en la 10.ª ed.). |
| net | Parque Seervada: viajes de tranvía | Libro o artículo | Hillier y Lieberman (2015a), cap. 10, Parque Seervada (flujo máximo de tranvías) | Por verificar: Confirmar el número de sección (10.5 en la 10.ª ed.). |
| net | Parque Seervada: cableado telefónico | Libro o artículo | Hillier y Lieberman (2015a), cap. 10, Parque Seervada (líneas telefónicas) | Por verificar: Confirmar el número de sección (10.4 en la 10.ª ed.). |
| net | Ruta de ventas | Original de IAQSB | Cinco ciudades del Perú con distancias por carretera aproximadas (km), redondeadas para el ejemplo; no son datos oficiales. | Verificada |
| net | Transbordo con almacén | Original de IAQSB | Red pequeña con dos fábricas, un almacén con un arco acotado y dos tiendas, escrita para IAQSB. | Verificada |
| net | Transbordo caso 1: sin costo en los nodos (clase) | Diapositivas de clase | Sarache (2026e), diapositivas 16–19 (caso 1: sin costo ni capacidad en los nodos) | Verificada |
| net | Transbordo caso 2: con costo en los nodos (clase) | Diapositivas de clase | Sarache (2026e), diapositivas 20–24 (caso 2: con costo en los nodos de transbordo) | Verificada |
| net | Transbordo caso 3: capacidad en los nodos (clase) | Diapositivas de clase | Sarache (2026e), diapositivas 25–27 (caso 3: capacidad en los nodos) | Verificada |
| net | Problema 1: plataformas logísticas (clase) | Diapositivas de clase | Sarache (2026e), diapositiva 28 (problema 1) | Verificada |
| net | Ruta más corta con programación binaria (clase) | Diapositivas de clase | Sarache (2026e), diapositivas 34–37 (ruta más corta con programación entera binaria, A → H = 14 km) | Verificada |
| net | Ruta más corta: ejercicio 1 (clase) | Diapositivas de clase | Sarache (2026e), diapositiva 32 (ejercicio 1); Bronson (1983), cap. 15 (algoritmo de la diapositiva 31) | Verificada |
| net | Árbol de expansión mínima: ejercicio 3 (clase) | Diapositivas de clase | Sarache (2026e), diapositivas 39–40 (ejercicio 3: interconexión eléctrica) | Verificada |
| net | Árbol de expansión mínima: ejercicio 4 (clase) | Diapositivas de clase | Sarache (2026e), diapositiva 41 (ejercicio 4) | Verificada |
| net | Flujo máximo de A a D (clase) | Diapositivas de clase | Sarache (2026e), diapositivas 44–50 (de A a D); Bronson (1983), cap. 15 (reglas y algoritmo de las diapositivas 42–43) | Verificada |
| net | Flujo máximo de A a J: ejercicio (clase) | Diapositivas de clase | Sarache (2026e), diapositiva 51 (ejercicio de A a J) | Verificada |
| net | Agente viajero de 4 nodos (clase) | Diapositivas de clase | Sarache (2026d), diapositivas 11–29 (red de 4 nodos: sin relajación, 32 restricciones; con relajación, 14) | Verificada |
| net | Agente viajero de 6 nodos (clase) | Diapositivas de clase | Sarache (2026d), diapositivas 30–49 (red de 6 nodos: sin relajación, 132 restricciones; con relajación, 32) | Verificada |
| net | Red de bebidas: NNH y CIH (clase) | Diapositivas de clase | Sarache (2026d), diapositiva 4 (red de bebidas: heurísticas NNH y CIH) | Verificada |
| pert | Reliable Construction (Hillier) | Libro o artículo | Hillier y Lieberman (2015a), cap. 22 (en línea), secc. 22.1, proyecto de Reliable Construction Co. | Verificada |
| pert | Reliable Construction con tres estimaciones (Hillier) | Libro o artículo | Hillier y Lieberman (2015a), cap. 22 (en línea), Reliable Construction Co. con tres estimaciones (secc. 22.4 en ediciones anteriores) | Por verificar: Confirmar en el texto de la 10.ª ed. que las tres estimaciones están en la secc. 22.4. |
| pert | General Foundry: control de costos (Render) | Libro o artículo | Render et al. (2018), cap. 11, secc. 11.1–11.2, General Foundry, Inc. (PERT/costo) | Por verificar: Confirmar en el libro el presupuesto por actividad y la tabla de seguimiento de costos. |
| gp | Harrison Electric (Render) | Libro o artículo | Render et al. (2018), cap. 10, secc. 10.3, Harrison Electric Company (programación por metas) | Por verificar: Confirmar los números de tabla y la página en el libro. |
| qp | Cartera de mínimo riesgo (Markowitz) | Original de IAQSB | Markowitz (1952), modelo de media y varianza | Verificada |
| qp | Ejemplo del método de Wolfe (Hillier) | Libro o artículo | Hillier y Lieberman (2015a), cap. 13, secc. 13.7 (programación cuadrática), ejemplo resuelto con el «símplex modificado» de Wolfe | Por verificar: Confirmar la sección y la página en un ejemplar impreso de la 10.ª ed. (las secciones salen del índice de la 10.ª ed.; el contenido se vio en otra edición). |
| nlp | Producción Cobb-Douglas | Original de IAQSB | Cobb y Douglas (1928), forma de la función de producción | Verificada |
| nlp | Función de Rosenbrock | Adaptado de una fuente | Rosenbrock (1960), función de prueba de Rosenbrock | Verificada |
| nlp | Wyndor no lineal (Hillier) | Libro o artículo | Hillier y Lieberman (2015a), cap. 13, secc. 13.2, problema de Wyndor con función objetivo no lineal | Por verificar: Confirmar la sección y la página en un ejemplar impreso de la 10.ª ed. (las secciones salen del índice de la 10.ª ed.; el contenido se vio en otra edición). |
| dp | La diligencia (Hillier) | Libro o artículo | Hillier y Lieberman (2015a), cap. 11, secc. 11.1, ejemplo 1: problema de la diligencia | Por verificar: Confirmar la sección y la página en un ejemplar impreso de la 10.ª ed. (las secciones salen del índice de la 10.ª ed.; el contenido se vio en otra edición). |
| dp | Carga de un avión | Original de IAQSB | Carga de tres artículos con capacidad 4, escrita para IAQSB. Resultado comprobado con un cálculo independiente en Python. | Verificada |
| dp | Plan de producción de cuatro meses | Original de IAQSB | Plan de producción de cuatro meses con costo de preparación y de almacenamiento, escrito para IAQSB. Resultado comprobado con un cálculo independiente en Python. | Verificada |
| ap | Plan de seis meses (modelo simple) | Original de IAQSB | Plan de seis meses escrito para IAQSB. | Verificada |
| ap | Farnsworth Tire Company (Heizer y Render) | Libro o artículo | Heizer y Render (2014), cap. 13, ejemplo 5: método de transporte de programación lineal (Farnsworth Tire Company) | Por verificar: Confirmar la edición en español de la copia consultada y el número de tabla. |
| ap | Plan de seis meses (programación lineal) | Original de IAQSB | Plan de seis meses escrito para IAQSB. | Verificada |
| pmp | Programa maestro básico (Arango) | Diapositivas de clase | Arango Marín (2020), diapositivas 8 a 12 (ejemplo programa maestro básico) y su hoja de Excel | Verificada |
| pmp | Programa maestro ATO (Arango) | Diapositivas de clase | Arango Marín (2020), diapositivas 13 a 18 (programa maestro para ATO) y la hoja «Ejemplo p mae ato resuelto» | Verificada |
| pmp | Producto A, ensamble bajo pedido (Arango) | Diapositivas de clase | Arango Marín (2020), diapositivas 19 y 20 (ejemplo del producto A con solicitudes de 20, 40 y 120 unidades) | Por verificar: La solución se comparó con la hoja «ejemplo 2 pmp ato» hecha por un estudiante (DPP 27, 7, 40, 65, 122, 143 y 150); falta la solución del profesor para las tres solicitudes. |
| pmp | Ensamble con capacidad y costos | Original de IAQSB | Ensamble bajo pedido con stock de seguridad, costos y capacidad del recurso clave, escrito para IAQSB para mostrar el plan aproximado de capacidad y el PMP óptimo. | Verificada |
| da | Goferbroke (Hillier) | Libro o artículo | Hillier y Lieberman (2015a), cap. 16, secc. 16.1–16.2, ejemplo prototipo de Goferbroke Co. | Por verificar: Confirmar la sección y la página en un ejemplar impreso de la 10.ª ed. (las secciones salen del índice de la 10.ª ed.; el contenido se vio en otra edición). |
| da | Sondeo sismico | Libro o artículo | Hillier y Lieberman (2015a), cap. 16, secc. 16.3, Goferbroke con sondeo sísmico (fig. 16.2) | Por verificar: Confirmar en la 10.ª ed. las verosimilitudes 0.6 y 0.2 y las probabilidades a posteriori. |
| da | Campana politica (Hillier, variante 3) | Libro o artículo | Hillier y Lieberman (2015a), cap. 15, campaña política, variante 3 (secc. 15.2, 15.4 y 15.5) | Por verificar: Confirmar la sección y la página en un ejemplar impreso de la 10.ª ed. (las secciones salen del índice de la 10.ª ed.; el contenido se vio en otra edición). |
| da | Goferbroke con sondeo | Libro o artículo | Hillier y Lieberman (2015a), cap. 16, secc. 16.4, árbol de decisión de Goferbroke con sondeo | Por verificar: Confirmar en la 10.ª ed. el costo del sondeo (30) y el valor 123. |
| mkp | Inventario de camaras (Hillier) | Libro o artículo | Hillier y Lieberman (2015a), cap. 29 (en línea), inventario de cámaras de la tienda de Dave (secc. 29.1, 29.2 y 29.5) | Por verificar: Confirmar la sección y la página en un ejemplar impreso de la 10.ª ed. (las secciones salen del índice de la 10.ª ed.; el contenido se vio en otra edición). |
| mkp | Cambio de tienda (Render) | Libro o artículo | Render et al. (2018), cap. 14, secc. 14.1–14.3, tres tiendas de abarrotes (American Food Store, Food Mart, Atlas Foods) | Por verificar: Confirmar si el libro calcula el estado estable de este ejemplo (el estado inicial y la matriz coinciden; el libro muestra las participaciones del período siguiente: 0.41, 0.31, 0.28). |
| mkp | Cuentas por cobrar (Render) | Libro o artículo | Render et al. (2018), cap. 14, secc. 14.6, estados absorbentes: cuentas por cobrar | Por verificar: Confirmar en el libro impreso los redondeos de F y FA. |
| its | Altavoces (Hillier) | Libro o artículo | Hillier y Lieberman (2015a), cap. 18, secc. 18.1 y 18.3, ejemplo 1: fabricante de bocinas para televisores | Por verificar: Confirmar la sección y la página en un ejemplar impreso de la 10.ª ed. (las secciones salen del índice de la 10.ª ed.; el contenido se vio en otra edición). |
| its | Altavoces con faltantes planeados (Hillier) | Adaptado de una fuente | Hillier y Lieberman (2015a), cap. 18, secc. 18.3, el mismo ejemplo con faltantes planeados | Por verificar: Confirmar en la 10.ª ed. el costo por faltante p = 1.10. |
| its | Carritos de juguete (Render) | Libro o artículo | Render et al. (2018), cap. 6, secc. 6.6, Brass Department Store (descuentos por cantidad) | Por verificar: Confirmar en el libro la tabla de descuentos y el costo total. |
| its | Freddie el vendedor de periodicos (Hillier) | Libro o artículo | Hillier y Lieberman (2015a), Freddie el vendedor de periódicos | Por verificar: No se ubicó la sección en la 10.ª ed. (posiblemente la 18.7, modelo de un período para productos perecederos) ni se confirmaron los datos. |
| its | Plan de produccion de 8 semanas | Original de IAQSB | Plan de producción de 8 semanas escrito para IAQSB. Resultado comprobado con un cálculo independiente en Python. | Verificada |
| its | Revision continua (s, Q) | Original de IAQSB | Revisión continua con demanda normal, escrita para IAQSB. | Verificada |
| its | Revision periodica (R, S) | Original de IAQSB | Revisión periódica con demanda normal, escrita para IAQSB. | Verificada |
| its | Revision periodica (R, s, S) | Original de IAQSB | Revisión periódica (R, s, S), escrita para IAQSB. | Verificada |
| its | Ferretería Nacional: EOQ (clase) | Diapositivas de clase | Sarache (2026b), diapositiva 27 (ejercicio 1, EOQ simple) | Verificada |
| its | Ferretería Nacional: POQ (clase) | Diapositivas de clase | Sarache (2026b), diapositiva 31 (ejercicio 1, POQ) | Verificada |
| its | Always Fresh Grocery: té (Schroeder) | Libro o artículo | Sarache (2026b), diapositiva 28 (ejercicio 2, EOQ simple); Schroeder y Goldstein (2018), cap. 14, problema 1 (Always Fresh Grocery Store) | Por verificar: Confirmar que es el problema 1 del capítulo 14 de Schroeder y Goldstein (2018); la diapositiva solo cita el libro. |
| its | Solución de limpieza: descuento por cantidad (Schroeder) | Libro o artículo | Sarache (2026b), diapositiva 35 (ejercicio 1, descuento por cantidad); Schroeder y Goldstein (2018), cap. 14, problema 2 (solución de limpieza) | Por verificar: Confirmar el número del problema en Schroeder y Goldstein (2018). |
| its | Plan de descuentos de tres rangos (clase) | Diapositivas de clase | Sarache (2026b), diapositiva 36 (ejercicio 2, descuento por cantidad) | Verificada |
| inv | Parcial: lote con entregas parciales (POQ al revés) | Diapositivas de clase | Sarache (s. f.), problema 1 (lote con entregas parciales) | Por verificar: Enunciado tomado del documento de ejercicios resueltos, no del parcial original. |
| inv | Parcial: EOQ a partir de la gráfica | Diapositivas de clase | Sarache (s. f.), problema 3 (EOQ a partir de la gráfica) | Por verificar: Enunciado tomado del documento de ejercicios resueltos, no del parcial original. |
| inv | POQ: tarea (problema inverso) | Diapositivas de clase | Sarache (2026b), diapositiva 32 (ejercicio 2, POQ: tarea) | Verificada |
| inv | Clasificación ABC: 15 artículos (clase) | Diapositivas de clase | Sarache (2026b), diapositivas 40–42 (ejemplo ABC de 15 artículos) | Verificada |
| inv | Sell-Rite Discount Stores: inventario de seguridad | Adaptado de una fuente | Sarache (2026b), diapositiva 45 (Sell-Rite Discount Stores); Gaither y Frazier (2000), libro candidato | Por verificar: La diapositiva no dice de qué libro es el ejemplo; Gaither y Frazier (2000) es solo el candidato más probable. |
| inv | Taller 2: política Q-R, Grupo 8 | Diapositivas de clase | Sarache (2026h), ejercicio 1, escenarios A a E, y ejercicio 2 (back orders) | Verificada |
| fin | Efecto de los inventarios en el ROI: escenarios A y B (clase) | Diapositivas de clase | Sarache (2026b), diapositiva 7 (efecto de los inventarios en el ROI) | Verificada |
| fin | Cálculo del ROI: compañía manufacturera (clase) | Diapositivas de clase | Sarache (2026b), diapositiva 8 (ejercicio: cálculo del ROI) | Verificada |
| fin | Efecto en el flujo de efectivo: almacén comercial (clase) | Diapositivas de clase | Sarache (2026b), diapositiva 10 (ejercicio: efecto en el flujo de efectivo) | Verificada |
| fin | Capital de trabajo: situación actual y reducción (clase) | Diapositivas de clase | Sarache (2026b), diapositivas 13–16 (capital de trabajo y efecto de su reducción) | Verificada |
| fin | Parcial: capital de trabajo (desfase que se repite 4 veces) | Diapositivas de clase | Sarache (s. f.), problema 2 (capital de trabajo) | Por verificar: Enunciado tomado del documento de ejercicios resueltos, no del parcial original. |
| alm | Dimensionamiento desde el paletizado: planta envasadora (clase) | Diapositivas de clase | Sarache (2026a), diapositiva 25 (dimensionamiento desde el paletizado) | Verificada |
| alm | Taller 3, punto 1: cajas por estiba (Grupo 8) | Diapositivas de clase | Sarache (2026g), punto 1 | Por verificar: Datos tomados del documento de ejercicios resueltos del Grupo 8. |
| alm | Dimensionado del almacén: compañía química (clase) | Adaptado de una fuente | Sarache (2026a), diapositivas 60–71 (dimensionado del almacén: compañía química); Ballou (2004a), cap. 12 «Decisiones sobre almacenamiento y manejo» | Por verificar: Confirmar que el ejemplo está en el capítulo 12 de Ballou (2004); la diapositiva pide leer los capítulos 11 y 12. |
| alm | Taller 3, puntos 2 y 3: dimensionado del almacén (Grupo 8) | Diapositivas de clase | Sarache (2026g), puntos 2 y 3 | Por verificar: Datos tomados del documento de ejercicios resueltos del Grupo 8. |
| kpi | Taller 1: indicadores logísticos, distribuidor mayorista | Diapositivas de clase | Sarache (2026f), 30 pedidos y costos del período | Por verificar: Datos tomados del documento de ejercicios resueltos (no se tuvo el enunciado del taller). |
| fc | Ventas mensuales | Original de IAQSB | Doce meses de ventas escritos para IAQSB. | Verificada |
| fc | Demanda trimestral estacional | Original de IAQSB | Demanda trimestral con estacionalidad, escrita para IAQSB. | Verificada |
| fc | Constructora Triple A (Render) | Libro o artículo | Render et al. (2018), cap. 4, secc. 4.2–4.5, Triple A Construction (tablas 4.1 a 4.4 en la 10.ª ed.) | Verificada |
| st | Utilidades trimestrales (Arango, eje_autocorr) | Diapositivas de clase | Arango Marín (2021a), hoja «eje_autocorr» y el script de R de la clase (ggseasonplot, ggAcf) | Verificada |
| st | Generación eléctrica neta de EE. UU. (fpp2::usnetelec) | Diapositivas de clase | Arango Marín (2024), diapositivas 3 a 5 (auto.arima(usnetelec)); Hyndman y Athanasopoulos (2018), datos usnetelec del paquete fpp2 | Verificada |
| st | Ventas de corticosteroides (fpp2::h02) | Diapositivas de clase | Arango Marín (2024), diapositivas 13 y 14 (auto.arima(h02, lambda = 0)); Hyndman y Athanasopoulos (2018), datos h02 del paquete fpp2 | Verificada |
| st | Producción trimestral de cemento (fpp2::qcement) | Diapositivas de clase | Arango Marín (2024), diapositivas 15 y 16 (ETS frente a ARIMA en la producción de cemento, entrenamiento 1988 a 2007 y prueba de 25 trimestres); Hyndman y Athanasopoulos (2018), datos qcement del paquete fpp2 | Verificada |
| st | Producción de gas en Australia desde 1980 (fpp2::gas) | Diapositivas de clase | Arango Marín (2024), diapositiva 5 de «Modelos TBATS» (tbats(gas)); aquí desde 1980 para que sea rápido; Hyndman y Athanasopoulos (2018), datos gas del paquete fpp2 | Verificada |
| st | Ejercicio de clase: suavización, ingenuo y TBATS | Diapositivas de clase | Arango Marín (2022), cuestionario del curso con el mismo procedimiento de las diapositivas: ts() desde mayo de 2021, window() hasta diciembre de 2023, ses(), naive(), tbats() y accuracy() | Verificada |
| qa | Sala de emergencias (Hillier) | Adaptado de una fuente | Hillier y Lieberman (2015a), cap. 17, secc. 17.1 y 17.6, sala de emergencias del County Hospital | Por verificar: Confirmar la sección y la página en un ejemplar impreso de la 10.ª ed. (las secciones salen del índice de la 10.ª ed.; el contenido se vio en otra edición). |
| qa | Reciclaje Garcia-Golding, servicio constante (Render) | Libro o artículo | Render et al. (2018), cap. 12, secc. 12.5, Garcia-Golding Recycling, Inc. (tiempo de servicio constante, M/D/1) | Por verificar: Confirmar en el libro λ = 8 y μ = 12 por hora. |
| qa | Reparacion de 5 maquinas (poblacion finita) | Original de IAQSB | Reparación de 5 máquinas (población finita), escrita para IAQSB. Resultado comprobado con un cálculo independiente en Python. | Verificada |
| qa | Barberia con 3 sillas de espera | Original de IAQSB | Barbería M/M/1 con 3 sillas de espera (capacidad 4), escrita para IAQSB. Resultado comprobado con un cálculo independiente en Python. | Verificada |
| qa | Llegadas en lotes (M^X/M/2, lotes de 1 a 3) | Original de IAQSB | Llegadas en lotes de 1 a 3 con dos servidores, escrita para IAQSB. | Verificada |
| qa | Ventanilla con servicio presionado y clientes desalentados | Original de IAQSB | Ventanilla con servicio presionado y clientes que se desalientan, escrita para IAQSB. | Verificada |
| qss | Banco con dos cajeros y una fila (M/M/2) | Original de IAQSB | Banco con dos cajeros y una fila (M/M/2), escrito para IAQSB; el resultado depende de la semilla de la simulación. | Verificada |
| qss | Taller en serie con reproceso y bloqueo | Original de IAQSB | Taller en serie con reproceso y bloqueo, escrito para IAQSB. | Verificada |
| qss | M/M/1 para comprobar (W = 20) | Original de IAQSB | M/M/1 cuyo tiempo medio teórico en el sistema es W = 20, para comparar la simulación con la fórmula; escrito para IAQSB. | Verificada |
| qss | Ensamble de dos líneas con prioridad | Original de IAQSB | Ensamble de dos líneas con prioridad, escrito para IAQSB. | Verificada |
| job | Taller con cuatro trabajos | Original de IAQSB | Taller de cuatro trabajos y tres máquinas, escrito para IAQSB. | Verificada |
| job | Dos maquinas: regla de Johnson (Render) | Libro o artículo | Heizer y Render (2014), cap. 15, regla de Johnson: cinco trabajos en dos centros de trabajo | Por verificar: Los tiempos y la secuencia B-E-D-C-A se hallaron en fuentes secundarias que no citan el libro; confirmar en Heizer y Render el ejemplo y el total de 35 horas. |
| job | Flujo de seis trabajos en cinco maquinas | Original de IAQSB | Flujo de seis trabajos en cinco máquinas, escrito para IAQSB. | Verificada |
| mrp | Producto A (Heizer y Render) | Libro o artículo | Heizer y Render (2014), cap. 14, planeación de requerimientos de materiales, estructura del producto A | Por verificar: Confirmar si el ejemplo es de Heizer y Render (cap. 14) o de Render, Stair, Hanna y Hale (secc. 6.10), y sus datos. |
| mrp | Bicicletas con lotes y capacidad | Original de IAQSB | Bicicletas con reglas de lote y capacidad, escrito para IAQSB. | Verificada |
| mrp | Programación MRP de 7 artículos (Arango) | Diapositivas de clase | Arango Marín (2021c), hojas «ejemplo programacion mrp» y «solución ejemplo programacion mrp» | Verificada |
| mrp2 | Estantes (Arango, MRP II) | Diapositivas de clase | Arango Marín (2021b), diapositivas 4 a 14 (ejemplo de los estantes) y la hoja «Ejemplo MRP II» | Por verificar: Los datos son los de la hoja del curso; falta la solución del profesor para comparar las necesidades a programar y los tiempos residuales (en particular, cómo «se reparten» los negativos). |
| fll | Almacen nuevo para cuatro clientes | Original de IAQSB | Un almacén para cuatro clientes con distancia rectilínea, escrito para IAQSB. Resultado comprobado con un cálculo independiente en Python. | Verificada |
| fll | Dos plantas nuevas | Original de IAQSB | Dos plantas nuevas, escrito para IAQSB. | Verificada |
| fll | Distribucion de una planta 4 x 4 | Original de IAQSB | Distribución de 16 departamentos en una cuadrícula de 4 × 4, escrita para IAQSB. | Verificada |
| fll | Ensamble de alas (Heizer y Render) | Libro o artículo | Heizer y Render (2014), cap. 9, balanceo de líneas de ensamble: componente del ala | Por verificar: Confirmar el número de ejemplo en la 9.ª ed.: los datos (66 minutos, ciclo de 12) coinciden con la 7.ª ed. en español (p. 367); otra edición usa 65 minutos y ciclo de 11. |
| asa | Muestreo simple n = 50, c = 1 | Original de IAQSB | Plan simple n = 50, c = 1, escrito para IAQSB. | Verificada |
| asa | Muestreo doble n1 = 50, c1 = 1, n2 = 100, c2 = 3 (Montgomery) | Libro o artículo | Montgomery (2020), cap. 15, secc. 15.3.1, planes de muestreo doble (6.ª ed.) | Por verificar: Confirmar que el libro usa el plan n1 = 50, c1 = 1, n2 = 100, c2 = 3. |
| asa | Secuencial de Wald: p1 = 1 %, p2 = 6 %, α = 5 %, β = 10 % (Montgomery) | Libro o artículo | Montgomery (2020), cap. 15, secc. 15.3.3, planes de muestreo secuencial (6.ª ed.) | Por verificar: Confirmar que el libro usa p1 = 0.01, α = 0.05, p2 = 0.06, β = 0.10 (un ejercicio publicado usa p2 = 0.10). |
| asa | Producción continua CSP-1: f = 2 %, i = 38 | Original de IAQSB | Plan CSP-1 con f = 2 % e i = 38, escrito para IAQSB. | Verificada |
| asa | Resistencia mínima: media de la muestra | Original de IAQSB | Muestreo por variables de la resistencia mínima, escrito para IAQSB. | Verificada |
| asa | Fracción defectuosa por el método k (límite superior) | Original de IAQSB | Muestreo por variables por el método k, escrito para IAQSB. | Verificada |
| qcc | Diámetro de anillos de pistón (Montgomery) | Ejemplo de WinQSB | Chang y Desai (2003), archivo de ejemplo VARIABLE.QC («Piston-ring Production»); Montgomery (2020), cap. 6, anillos de pistón (tabla 6.3) | Por verificar: Confirmar la numeración en la 8.ª ed. |
| qcc | Latas de jugo defectuosas, n = 50 (Montgomery) | Ejemplo de WinQSB | Chang y Desai (2003), archivo de ejemplo P_CHART.QC; Montgomery (2020), cap. 7, latas de jugo de naranja concentrado, gráfico p con n = 50 | Por verificar: Confirmar el número de ejemplo y de tabla en la 8.ª ed. |
| qcc | Defectos en tarjetas de circuito impreso (Montgomery) | Ejemplo de WinQSB | Chang y Desai (2003), archivo de ejemplo C_CHART.QC; Montgomery (2020), cap. 7, disconformidades en tarjetas de circuito impreso, gráfico c | Por verificar: Confirmar el número de ejemplo y de tabla en la 8.ª ed. |
| qcc | Defectos de conectores por tipo | Ejemplo de WinQSB | Chang y Desai (2003), archivo de ejemplo PARETO.QC («QCC Pareto Sample Problem») | Verificada |
| vrp | Barrido: almacén de alimentos frescos (clase) | Diapositivas de clase | Sarache (2026d), diapositiva 5 (método del barrido: almacén de alimentos frescos, 15 tiendas, capacidad 3000) | Verificada |
| vrp | Reparto en la ciudad | Original de IAQSB | Clientes y demandas inventados para mostrar el ruteo por coordenadas. | Verificada |
| vrp | Reparto en Bogotá (latitud y longitud) | Original de IAQSB | Datos originales de IAQSB: un centro de distribución en Puente Aranda y ocho tiendas en localidades de Bogotá, con latitud y longitud aproximadas de cada sector (no son direcciones de negocios reales); demandas en cajas, capacidad y costos inventados para mostrar el ruteo con coordenadas geográficas y distancias en km. | Verificada |
| modo | Trade-off transporte-inventarios: tren, camión o avión (clase) | Adaptado de una fuente | Sarache (2026c), diapositivas 38–40 (trade-off transporte-inventarios); Ballou (2004a), cap. 7 «Decisiones sobre el transporte», ejemplo de Carry-All Luggage Company | Por verificar: Confirmar en el texto que el ejemplo de Carry-All está en el capítulo 7 de Ballou (2004); el índice ubica allí la selección del servicio de transporte. |

## Métodos que cita cada módulo

| Módulo | Cuándo | Fuente | Para |
|---|---|---|---|
| lp | siempre | Dantzig, 1963 | Método símplex y análisis de sensibilidad |
| lp | enteras = true | Land y Doig, 1960 | Ramificación y acotamiento para las variables enteras o binarias |
| lp | siempre | Hillier y Lieberman, 2015a | Formulación de la programación lineal y método símplex |
| lp | metodo = engine, branch_and_cut | Huangfu y Hall, 2018 | Motor HiGHS: símplex dual revisado, y ramificación y corte en los modelos enteros |
| lp | diagnostico = true | Chinneck, 2008 | Diagnóstico de infactibilidad: subsistema irreducible infactible (IIS) y relajación de factibilidad |
| pert | problem_type = cpm | Kelley y Walker, 1959 | Método de la ruta crítica (CPM): tiempos más tempranos y más tardíos y holguras |
| pert | problem_type = pert | Malcolm et al., 1959 | PERT: tres estimaciones, media y varianza de cada actividad y probabilidad de terminar a tiempo |
| pert | crashing = min_cost, deadline, budget | Kelley, 1961 | Compresión del proyecto al menor costo |
| pert | seccion = compresion | Kelley, 1961 | Compresión del proyecto al menor costo |
| pert | seccion = control | Render et al., 2018 | Control de costos del proyecto (PERT/costo, cap. 11) |
| pert | seccion = simulacion | Law, 2015 | Simulación de la duración del proyecto |
| pert | seccion = simulacion | Bernstein, 2008 | Generador ChaCha8 de números aleatorios |
| pert | siempre | Hillier y Lieberman, 2015a | Administración de proyectos con PERT/CPM (cap. 22) |
| gp | siempre | Charnes et al., 1955 | Origen de la programación por metas |
| gp | siempre | Charnes y Cooper, 1961 | Metas con variables de desviación y prioridades preferentes |
| gp | siempre | Huangfu y Hall, 2018 | Programa lineal o entero de cada prioridad resuelto con HiGHS |
| gp | siempre | Render et al., 2018 | Programación por metas con prioridades (cap. 10) |
| qp | siempre | Goulart y Chen, 2024 | Solución de la programación cuadrática convexa con Clarabel (punto interior) |
| qp | siempre | Kuhn y Tucker, 1951 | Condiciones de Karush-Kuhn-Tucker del óptimo |
| qp | siempre | Karush, 1939 | Condiciones de Karush-Kuhn-Tucker del óptimo |
| qp | seccion = pasos | Wolfe, 1959 | Método de Wolfe del paso a paso |
| qp | siempre | Hillier y Lieberman, 2015a | Programación cuadrática (secc. 13.7) |
| nlp | siempre | Nocedal y Wright, 2006 | Programación cuadrática sucesiva (SQP) con búsqueda lineal |
| nlp | siempre | Powell, 1978 | Actualización BFGS amortiguada de la hessiana |
| nlp | siempre | Goulart y Chen, 2024 | Subproblemas cuadráticos de cada iteración resueltos con Clarabel |
| nlp | siempre | Kuhn y Tucker, 1951 | Condiciones de Karush-Kuhn-Tucker del óptimo |
| nlp | siempre | Karush, 1939 | Condiciones de Karush-Kuhn-Tucker del óptimo |
| nlp | seccion = pasos | Fiacco y McCormick, 1968 | Método de penalización del paso a paso (el de WinQSB) |
| nlp | siempre | Hillier y Lieberman, 2015a | Programación no lineal (cap. 13) |
| dp | siempre | Bellman, 1957 | Principio de optimalidad y recursión hacia atrás |
| dp | siempre | Hillier y Lieberman, 2015a | Programación dinámica determinística (cap. 11) |
| ap | siempre | Holt et al., 1960 | Planificación agregada de la producción y la fuerza de trabajo |
| ap | problem_type = transportation | Bowman, 1956 | Planificación agregada con el método de transporte |
| ap | problem_type = lp | Dantzig, 1963 | Plan óptimo por programación lineal |
| ap | problem_type = simple; strategy = lp | Dantzig, 1963 | Plan óptimo por programación lineal |
| ap | siempre | Heizer y Render, 2014 | Estrategias de nivelación, persecución y método de transporte (cap. 13) |
| da | problem_type = payoff | Savage, 1951 | Criterio de mínimo arrepentimiento |
| da | problem_type = payoff | Hurwicz, 1951 | Criterio de Hurwicz |
| da | problem_type = payoff | Raiffa y Schlaifer, 1961 | Valor esperado y valor esperado de la información perfecta |
| da | problem_type = bayesian | Raiffa y Schlaifer, 1961 | Probabilidades a posteriori con el teorema de Bayes |
| da | problem_type = tree | Raiffa y Schlaifer, 1961 | Análisis del árbol de decisión hacia atrás |
| da | problem_type = zero_sum | von Neumann y Morgenstern, 1944 | Juego de suma cero, punto de silla y estrategias mixtas |
| da | problem_type = zero_sum | Dantzig, 1963 | Estrategias mixtas por programación lineal |
| da | siempre | Hillier y Lieberman, 2015a | Análisis de decisiones y teoría de juegos (caps. 15 y 16) |
| mkp | siempre | Kemeny y Snell, 1960 | Clasificación de estados, estado estable, primer paso y absorción |
| mkp | siempre | Crozet, 2026 | Sistemas lineales y matriz fundamental |
| mkp | siempre | Hillier y Lieberman, 2015a | Cadenas de Markov (cap. 29) |
| its | problem_type = eoq | Harris, 1913 | Cantidad económica de pedido |
| its | problem_type = eoq; production_rate = !M | Taft, 1918 | Cantidad económica de producción |
| its | problem_type = eoq; production_per_day = * | Taft, 1918 | Cantidad económica de producción |
| its | problem_type = eoq; max_backorder = * | Hadley y Whitin, 1963 | EOQ con faltantes planeados |
| its | problem_type = discount | Hadley y Whitin, 1963 | EOQ con descuentos por cantidad |
| its | problem_type = newsboy | Arrow et al., 1951 | Modelo de un período (vendedor de periódicos) |
| its | problem_type = lot_sizing; method = wagner_whitin | Wagner y Whitin, 1958 | Tamaño de lote óptimo de Wagner-Whitin |
| its | problem_type = lot_sizing; method = silver_meal | Silver y Meal, 1973 | Heurística de Silver-Meal |
| its | problem_type = lot_sizing; method = ppb | DeMatteis, 1968 | Balanceo de períodos (part-period) |
| its | problem_type = lot_sizing | Wagner y Whitin, 1958 | Costo óptimo con el que se compara cada regla de lote |
| its | problem_type = lot_sizing | Silver et al., 2017 | Reglas de tamaño de lote |
| its | problem_type = sq, ss, rs, rss | Silver et al., 2017 | Sistemas de revisión continua y periódica con demanda incierta |
| its | problem_type = ss, rss | Scarf, 1960 | Optimalidad de las políticas (s, S) |
| its | seccion = simulacion | Bernstein, 2008 | Generador ChaCha8 de la simulación |
| its | siempre | Hillier y Lieberman, 2015a | Teoría de inventarios (cap. 18) |
| fc | problem_type = series; method = single_exponential, double_exponential | Brown, 1959 | Suavizamiento exponencial |
| fc | problem_type = series; method = single_exponential_trend, double_exponential_trend, moving_average_trend | Holt, 2004 | Suavizamiento con tendencia |
| fc | problem_type = series; method = holt_winters_additive, holt_winters_multiplicative | Winters, 1960 | Suavizamiento de Holt-Winters con estacionalidad |
| fc | problem_type = series; method = holt_winters_additive, holt_winters_multiplicative | Holt, 2004 | Nivel y tendencia de Holt |
| fc | problem_type = series; method = adaptive_exponential | Trigg y Leach, 1967 | Suavizamiento con tasa de respuesta adaptativa |
| fc | problem_type = series | Hyndman y Koehler, 2006 | Medidas de error: MAD, MSE y MAPE |
| fc | problem_type = series | Trigg, 1964 | Señal de rastreo |
| fc | problem_type = regression | Render et al., 2018 | Regresión lineal y su análisis (cap. 4) |
| fc | problem_type = regression | Crozet, 2026 | Mínimos cuadrados |
| fc | problem_type = series | Render et al., 2018 | Pronósticos con series de tiempo (cap. 5) |
| qa | siempre | Erlang, 1909 | Llegadas de Poisson y tiempos exponenciales |
| qa | siempre | Kendall, 1953 | Notación de Kendall y cola M/G/1 |
| qa | siempre | Little, 1961 | Ley de Little (L = λW) |
| qa | siempre | Shortle et al., 2018 | Fórmulas de los modelos de colas |
| qa | seccion = simulacion | Law, 2015 | Simulación de eventos discretos |
| qa | seccion = simulacion | Bernstein, 2008 | Generador ChaCha8 de números aleatorios |
| qa | siempre | Hillier y Lieberman, 2015a | Teoría de colas (cap. 17) |
| qss | siempre | Law, 2015 | Simulación de eventos discretos y su análisis |
| qss | siempre | Bernstein, 2008 | Generador ChaCha8 de números aleatorios |
| qss | siempre | Little, 1961 | Relación L = λW entre los resultados |
| job | problem_type = job_shop | Panwalkar y Iskander, 1977 | Reglas de despacho |
| job | problem_type = job_shop | Giffler y Thompson, 1960 | Programa del taller con reglas de despacho |
| job | problem_type = flow_shop; flow_method = johnson | Johnson, 1954 | Regla de Johnson |
| job | problem_type = flow_shop; flow_method = cds, all_heuristics | Campbell et al., 1970 | Heurística CDS |
| job | problem_type = flow_shop; flow_method = cds, all_heuristics | Johnson, 1954 | Regla de Johnson en cada subproblema de CDS |
| job | problem_type = flow_shop; flow_method = palmer, all_heuristics | Palmer, 1965 | Índice de pendiente de Palmer |
| job | problem_type = flow_shop; flow_method = gupta, all_heuristics | Gupta, 1971 | Heurística de Gupta |
| job | problem_type = flow_shop; flow_method = dannenbring, all_heuristics | Dannenbring, 1977 | Aproximación rápida de Dannenbring |
| job | problem_type = flow_shop; flow_method = hundal_rajgopal, all_heuristics | Hundal y Rajgopal, 1988 | Heurística de Hundal y Rajgopal |
| job | problem_type = flow_shop; flow_method = ho_chang, all_heuristics | Ho y Chang, 1991 | Heurística de Ho y Chang |
| job | problem_type = flow_shop; flow_method = neh, all_heuristics | Nawaz et al., 1983 | Heurística NEH |
| job | siempre | Pinedo, 2016 | Criterios de desempeño del programa |
| job | siempre | Heizer y Render, 2014 | Programación de operaciones (cap. 15) |
| mrp | siempre | Orlicky, 1975 | Explosión de materiales, necesidades netas y órdenes planeadas |
| mrp | siempre | Heizer y Render, 2014 | MRP y reglas de lote (cap. 14) |
| fll | problem_type = location | Francis et al., 1992 | Localización con distancia rectilínea, euclidiana o euclidiana al cuadrado |
| fll | problem_type = location; distance = euclidean | Weiszfeld, 1937 | Algoritmo de Weiszfeld |
| fll | problem_type = layout | Armour y Buffa, 1963 | Mejora de la distribución por intercambios (CRAFT) |
| fll | problem_type = layout | Francis et al., 1992 | Costo de flujo por distancia |
| fll | problem_type = line_balancing; method = heuristic; rule = rpw | Helgeson y Birnie, 1961 | Peso posicional (RPW) |
| fll | problem_type = line_balancing; method = comsoal | Arcus, 1965 | Método COMSOAL |
| fll | problem_type = line_balancing | Heizer y Render, 2014 | Balanceo de líneas de ensamble (cap. 9) |
| asa | problem_type = single, double, multiple | Dodge y Romig, 1959 | Curva OC, AOQ, AOQL e inspección total promedio |
| asa | problem_type = multiple | American Society for Quality, 2018a | Planes múltiples por atributos |
| asa | problem_type = sequential, sequential_variable | Wald, 1947 | Prueba secuencial de razón de probabilidades |
| asa | problem_type = chain | Dodge, 1955a | Muestreo en cadena ChSP-1 |
| asa | problem_type = continuous | Dodge, 1943 | Plan CSP-1 para producción continua |
| asa | problem_type = skip_lot | Dodge, 1955b | Muestreo con salto de lotes |
| asa | problem_type = skip_lot | Perry, 1973 | Planes SkSP-2 |
| asa | problem_type = mean, k_method, m_method | American Society for Quality, 2018b | Muestreo por variables (métodos k y M) |
| asa | siempre | Montgomery, 2020 | Muestreo de aceptación (caps. 15 y 16) |
| qcc | chart = xbar, r, s, variance, x, median, midrange, p, np, c, u, demerits, std_p, std_np, std_c, std_u, modified_xbar, modified_x, acceptance_xbar, acceptance_x | Shewhart, 1931 | Gráfico de control de Shewhart |
| qcc | chart = cusum_xbar, cusum_x, cusum_r, cusum_s, cusum_p, cusum_c | Page, 1954 | Gráfico de sumas acumuladas (CUSUM) |
| qcc | chart = gma_xbar, gma_x, gma_p, gma_c | Roberts, 1959 | Promedio móvil geométrico (EWMA) |
| qcc | check_rules = true | Western Electric Company, 1956 | Reglas de corridas para detectar puntos fuera de control |
| qcc | seccion = capacidad | Kane, 1986 | Índices de capacidad Cp y Cpk |
| qcc | seccion = pareto | Juran, 1951 | Análisis de Pareto |
| qcc | siempre | Montgomery, 2020 | Gráficos de control y capacidad del proceso (caps. 6 a 9) |

