//! Fuentes de cada método de NET: qué algoritmo, fórmula o planteamiento se
//! usó y de dónde sale (ver `iaqsb-core/fuentes/bibliografia.json`).

use super::{InitialMethod, NetModel, NetOptions, NetType, TspMethod};
use iaqsb_core::Report;

const HIGHS: &str = "Solución del programa lineal o entero con HiGHS (simplex dual revisado y ramificación)";

pub fn cite(mut r: Report, m: &NetModel, o: &NetOptions) -> Report {
    let method = r.data.get("method").and_then(|v| v.as_str()).unwrap_or("").to_lowercase();
    let has = |id: &str| r.sections.iter().any(|s| s.id == id);
    let (steps, model) = (has("pasos"), has("modelo"));
    match m.problem_type {
        NetType::Transportation => {
            r.add_cite("hitchcock1941", "Formulación del problema de transporte");
            match o.initial {
                InitialMethod::Vogel => r.add_cite("reinfeldvogel1958", "Solución inicial por el método de aproximación de Vogel"),
                InitialMethod::Russell => r.add_cite("russell1969", "Solución inicial por el método de aproximación de Russell"),
                InitialMethod::NorthwestCorner => r.add_cite("taha2012", "Solución inicial por la regla de la esquina noroeste"),
                InitialMethod::MinimumCost => r.add_cite("taha2012", "Solución inicial por el método del costo mínimo"),
            }
            r.add_cite("dantzig1951", "Iteraciones con potenciales u(i), v(j) y costos reducidos (método MODI)");
            r.add_cite("charnescooper1954", "Ciclo de mejora y valor θ (salto de piedra en piedra)");
            r.add_cite("huangfuhall2018", "Comprobación del óptimo con HiGHS");
        }
        NetType::Assignment => {
            r.add_cite("kuhn1955", "Método húngaro: reducción de filas y columnas y cubrimiento mínimo de ceros");
            r.add_cite("huangfuhall2018", "Comprobación del óptimo con HiGHS");
        }
        NetType::NetworkFlow => {
            r.add_cite("orden1956", "Problema de transbordo");
            r.add_cite("ahuja1993", "Flujo de costo mínimo: conservación del flujo en cada nodo");
            if super::flowx::any_extended(m) {
                r.add_cite("sarache2026redes", "Costo y capacidad en los nodos de transbordo y demandas con rango (casos 2 y 3, diapositivas 20–27)");
                r.add_cite("ahuja1993", "Red equivalente para WinQSB: cada nodo con costo o capacidad se divide en dos unidos por un arco");
            }
            r.add_cite("huangfuhall2018", HIGHS);
        }
        NetType::ShortestPath => {
            if method.contains("dijkstra") {
                r.add_cite("dijkstra1959", "Ruta más corta con etiquetas permanentes (Dijkstra)");
            } else if method.contains("bellman") {
                r.add_cite("bellman1958", "Ruta más corta con distancias negativas (Bellman-Ford)");
            } else {
                r.add_cite("ahuja1993", "Camino más largo en una red acíclica (orden topológico)");
            }
            if steps {
                r.add_cite("bronson1983", "Algoritmo de la clase para la ruta más corta (cap. 15)");
            }
            if model {
                r.add_cite("sarache2026redes", "Modelo de programación entera binaria de la ruta más corta (diapositivas 34–37)");
                r.add_cite("huangfuhall2018", HIGHS);
            }
        }
        NetType::MaxFlow => {
            r.add_cite("fordfulkerson1956", "Caminos de aumento en la red residual y teorema de flujo máximo y corte mínimo");
            r.add_cite("edmondskarp1972", "Elección del camino de aumento con menos arcos (búsqueda en anchura)");
            if steps {
                r.add_cite("bronson1983", "Reglas y algoritmo de la clase para el flujo máximo (cap. 15; diapositivas 42–43 de Sarache)");
            }
        }
        NetType::MinSpanningTree => {
            if o.mst_method.eq_ignore_ascii_case("prim") {
                r.add_cite("prim1957", "Árbol de expansión mínima por el algoritmo de Prim");
            } else {
                r.add_cite("kruskal1956", "Árbol de expansión mínima por el algoritmo de Kruskal");
            }
            r.add_cite("bronson1983", "Definición del árbol de expansión mínima usada en clase (cap. 15)");
        }
        NetType::Tsp => {
            match o.tsp_method {
                TspMethod::Exact => {
                    r.add_cite("dantzig1954", "Óptimo: restricciones de eliminación de subciclos agregadas por rondas");
                    r.add_cite("huangfuhall2018", HIGHS);
                }
                TspMethod::NearestNeighbor => {
                    r.add_cite("rosenkrantz1977", "Heurística del vecino más cercano (NNH)");
                    r.add_cite("sarache2026ruteo", "Heurísticas NNH y CIH como se ven en clase (diapositiva 4)");
                }
                TspMethod::CheapestInsertion => {
                    r.add_cite("rosenkrantz1977", "Heurística de inserción más barata (CIH)");
                    r.add_cite("sarache2026ruteo", "Heurísticas NNH y CIH como se ven en clase (diapositiva 4)");
                }
                TspMethod::TwoOpt => {
                    r.add_cite("rosenkrantz1977", "Recorrido inicial por el vecino más cercano");
                    r.add_cite("croes1958", "Mejora por intercambio de dos arcos (2-opt)");
                }
            }
            if model {
                r.add_cite("sarache2026ruteo", "Modelo del agente viajero: de cada nodo sale y a cada nodo llega un vehículo (diapositivas 8–10)");
                r.add_cite(
                    "dantzig1954",
                    "Ecuaciones de rompimiento de subtour sin relajación: versión por ciclos dirigidos de tres nodos de las restricciones de subciclo",
                );
                if o.mtz_form.eq_ignore_ascii_case("libro") {
                    r.add_cite("miller1960", "Variables de orden U y restricciones de Miller, Tucker y Zemlin (origen de la relajación)");
                    r.add_cite("winston2004", "Forma de libro de texto U_i − U_j + n·X_ij ≤ n − 1 con n = todos los nodos (capítulo y página por verificar)");
                } else {
                    r.add_cite("miller1960", "Relajación de subtours: es la restricción original de Miller, Tucker y Zemlin (p = n − 1 ciudades sin la base)");
                    r.add_cite("sarache2026ruteo", "Relajación de subtours como en clase: U_i − U_j + (n − 1)X_ij ≤ n − 2 (diapositivas 23–28)");
                }
            }
        }
    }
    r
}
