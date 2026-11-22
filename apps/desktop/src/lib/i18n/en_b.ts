// Interfaz en inglés: textos del grupo b (español → inglés).
export const b: Record<string, string> = {
  // gráficas (Figure, ChartView)
  '{0} según {1}': '{0} vs. {1}',
  grafica: 'chart',
  'Gráfica': 'Chart',
  'Gráfica guardada: {0}': 'Chart saved: {0}',
  'No se pudo guardar la gráfica: {0}': 'Couldn’t save the chart: {0}',
  'Gráfica copiada: pégala en Word, PowerPoint o un chat': 'Chart copied. Paste it into Word, PowerPoint, or a chat.',
  'No se pudo copiar: {0}': 'Couldn’t copy: {0}',
  Ampliar: 'Enlarge',
  'Copiar como imagen': 'Copy as image',
  Copiar: 'Copy',
  'Descargar en PNG': 'Download as PNG',
  'Descargar en SVG (vectorial)': 'Download as SVG (vector)',
  'Alejar (−)': 'Zoom out (−)',
  Alejar: 'Zoom out',
  'Tamaño original (0)': 'Actual size (0)',
  'Acercar (+)': 'Zoom in (+)',
  Acercar: 'Zoom in',
  'Cerrar (Esc)': 'Close (Esc)',
  Cerrar: 'Close',
  'Rueda del mouse para acercar · arrastra para mover · doble clic para ampliar · Esc para cerrar':
    'Scroll to zoom · drag to move · double-click to enlarge · Esc to close',

  // red, mapa, árbol, Gantt, plano
  Red: 'Network',
  'Mapa de rutas': 'Route map',
  'demanda {0}': 'demand {0}',
  'Árbol': 'Tree',
  'decisión': 'decision',
  azar: 'chance',
  resultado: 'outcome',
  'Diagrama de Gantt': 'Gantt chart',
  Plano: 'Floor plan',
  'Escala: {0} × {1} {2}': 'Scale: {0} × {1} {2}',

  // fuentes
  'Datos comprobados en el DOI, el editor o un catálogo': 'Details checked against the DOI, the publisher, or a catalog',
  Verificada: 'Verified',
  'Por verificar': 'To be verified',
  'Por verificar: {0}': 'To be verified: {0}',
  'Origen de los datos': 'Data source',
  'Resultado en la fuente:': 'Result in the source:',
  'Métodos, fórmulas y pasos': 'Methods, formulas, and steps',
  Referencias: 'References',
  'APA 7.ª ed.': 'APA 7th ed.',
  '{0} por verificar': '{0} to be verified',
  Copiado: 'Copied',
  'Copiar en APA': 'Copy APA',
  'Copiar BibTeX': 'Copy BibTeX',

  // paso a paso
  'Paso a paso': 'Step by step',
  Inicio: 'First',
  Anterior: 'Previous',
  'Paso {0}': 'Step {0}',
  'de {0}': 'of {0}',
  Siguiente: 'Next',
  Final: 'Last',
  'Usa las flechas ← y → para avanzar.': 'Use the ← and → arrow keys to move between steps.',

  // método gráfico
  'El método gráfico está disponible para modelos de dos variables. Resuelve el problema para dibujarlo.':
    'The graphical method is available for two-variable models. Solve the problem to draw it.',
  'Región factible en el plano {0}, {1}': 'Feasible region in the {0}, {1} plane',
  'Región factible': 'Feasible region',
  '(continúa fuera del dibujo)': '(extends beyond the plot)',
  'Objetivo en el óptimo (z = {0})': 'Objective at the optimum (z = {0})',
  'Punto óptimo': 'Optimal point',

  // tablas del símplex
  'En problemas enteros el paso a paso está en la pestaña Branch and bound.':
    'For integer problems, the step-by-step solution is in the Branch and bound tab.',
  'Activa «Mostrar pasos» y resuelve de nuevo para ver las tablas del símplex.':
    'Turn on “Show steps” and solve again to see the simplex tableaus.',
  'Tablas del símplex': 'Simplex tableaus',
  'Tabla inicial': 'Initial tableau',
  'Iteración {0}': 'Iteration {0}',
  'Tabla final': 'Final tableau',
  Entra: 'Entering:',
  sale: 'leaving:',
  'La columna no tiene elementos positivos: el problema no está acotado.':
    'The column has no positive entries, so the problem is unbounded.',
  'Ningún C(j)−Z(j) mejora el objetivo: esta es la tabla final.':
    'No C(j)−Z(j) improves the objective, so this is the final tableau.',
  Base: 'Basis',
  'Lado derecho': 'Right-hand side',
  'Usa las flechas ← y → para avanzar. La columna que entra está resaltada y el pivote, enmarcado.':
    'Use the ← and → arrow keys to move between tableaus. The entering column is highlighted and the pivot is boxed.',

  // ramificación y acotamiento
  'El modelo no tiene variables enteras ni binarias.': 'The model has no integer or binary variables.',
  '{0} nodos explorados, {1} solución entera mejorada.': '{0} nodes explored, {1} improved integer solution.',
  '{0} nodos explorados, {1} soluciones enteras mejoradas.': '{0} nodes explored, {1} improved integer solutions.',
  'Óptimo relajado:': 'Relaxed optimum:',
  'Nodo {0}': 'Node {0}',
  'Relajación lineal': 'LP relaxation',
  'La sangría indica la profundidad de cada rama. Se explora primero la rama «≤» y se poda todo nodo que no puede mejorar la mejor solución entera.':
    'Indentation shows the depth of each branch. The “≤” branch is explored first, and any node that can’t improve on the best integer solution is pruned.'
};
