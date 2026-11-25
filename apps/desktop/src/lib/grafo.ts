// Modelos que son una red de nodos y arcos: qué hoja guarda los arcos, qué
// datos tiene cada nodo, dónde se guardan las posiciones del editor visual,
// distribuciones automáticas y la solución del informe dibujada sobre la red.
//
// El grafo se deduce de la especificación del módulo: una matriz nodo × nodo
// (la misma lista en filas y columnas) o, en transporte y asignación, una
// matriz origen × destino (red bipartita).

import type { Block, Field, MapBlock, NetworkBlock, ProblemType, Report, Widget } from './gtypes';

export type MatrixW = Extract<Widget, { type: 'matrix' }>;
export type RecordsW = Extract<Widget, { type: 'records' }>;

/** Módulos cuyo modelo es una red (en otros, una matriz cuadrada es una tabla de flujos, no un grafo). */
const MODULOS_RED = new Set(['net', 'dp', 'mkp', 'vrp']);

/** Módulo de Nativo en que se basa cada tema de curso (c.<curso>.<tema> → net, mrp2…); lo llena shell.init. */
export const basesDeTemas = new Map<string, string>();
const baseModulo = (m: string) => basesDeTemas.get(m) ?? m;

export interface Grafo {
  /** Transporte y asignación: orígenes a la izquierda y destinos a la derecha. */
  bipartita: boolean;
  /** Matriz con el valor de cada arco (coste, distancia, capacidad…). */
  matriz: MatrixW;
  /** Todas las matrices del editor (al añadir o quitar un nodo se ajustan todas). */
  matrices: MatrixW[];
  /** Hojas de registros de los mismos nodos (p. ej. los puntos del ruteo). */
  registros: RecordsW[];
  /** Datos de los nodos de las filas (en una red cuadrada, de todos los nodos). */
  camposFila: Field[];
  /** Datos de los nodos de las columnas (destinos en la bipartita). */
  camposCol: Field[];
  /**
   * 'plano': los nodos tienen columnas X e Y visibles (coordenadas del usuario, Y hacia arriba).
   * 'pantalla': posiciones ocultas `x`, `y` en cada nodo, en unidades del lienzo (Y hacia abajo).
   */
  coords: 'plano' | 'pantalla';
  /** Se permiten lazos i → i (cadenas de Markov). */
  lazos: boolean;
  /** Campos de nodo inicial y final del problema (ruta más corta, flujo máximo). */
  inicio: Field | null;
  fin: Field | null;
  /** Hoja de cotas por arco (from, to, lower, upper), si la hay. */
  cotas: RecordsW | null;
}

export function detectarGrafo(modulo: string, pt: ProblemType | null | undefined): Grafo | null {
  if (!pt || !MODULOS_RED.has(baseModulo(modulo))) return null;
  const matrices = pt.layout.filter((w): w is MatrixW => w.type === 'matrix');
  const sq = matrices.find((w) => w.rows.path === w.cols.path);
  const bip = !sq && baseModulo(modulo) === 'net' ? matrices.find((w) => w.rows.path !== w.cols.path) : undefined;
  const matriz = sq ?? bip;
  if (!matriz) return null;
  const registros = pt.layout.filter((w): w is RecordsW => w.type === 'records' && (w.path === matriz.rows.path || w.path === matriz.cols.path));
  const extraDe = (path: string) => [
    ...matrices.flatMap((m) => [...(m.rows.path === path ? m.rows.extra : []), ...(m.cols.path === path ? m.cols.extra : [])]),
    ...registros.filter((r) => r.path === path).flatMap((r) => r.columns)
  ];
  const limpiar = (fs: Field[], labelKey: string) => {
    const vistos = new Set<string>();
    return fs.filter((f) => {
      if (f.key === labelKey || f.key === 'x' || f.key === 'y' || vistos.has(f.key)) return false;
      vistos.add(f.key);
      return true;
    });
  };
  const todos = [...extraDe(matriz.rows.path), ...(sq ? [] : extraDe(matriz.cols.path))];
  const plano = todos.some((f) => f.key === 'x') && todos.some((f) => f.key === 'y');
  const params = pt.layout.filter((w) => w.type === 'params').flatMap((w) => (w.type === 'params' && !w.path ? w.fields : []));
  const cotas = pt.layout.find(
    (w): w is RecordsW => w.type === 'records' && w.columns.some((c) => c.key === 'from') && w.columns.some((c) => c.key === 'to')
  );
  return {
    bipartita: !sq,
    matriz,
    matrices,
    registros,
    camposFila: limpiar(extraDe(matriz.rows.path), matriz.rows.label_key),
    camposCol: limpiar(sq ? extraDe(matriz.rows.path) : extraDe(matriz.cols.path), matriz.cols.label_key),
    coords: plano ? 'plano' : 'pantalla',
    lazos: !!matriz.allow_diagonal,
    inicio: params.find((f) => f.key === 'start') ?? null,
    fin: params.find((f) => f.key === 'end') ?? null,
    cotas: cotas ?? null
  };
}

/** Red no dirigida: árbol de expansión (siempre) o modelos marcados como simétricos. */
export function noDirigida(modulo: string, model: any): boolean {
  return model?.symmetric === true || (baseModulo(modulo) === 'net' && model?.problem_type === 'min_spanning_tree');
}

/** ¿Hay arco en esta celda? Con celdas obligatorias (probabilidades), el 0 significa que no. */
export function hayArco(g: Grafo, v: unknown): boolean {
  if (v === null || v === undefined || v === '') return false;
  if (!g.matriz.cell.optional && v === 0) return false;
  return true;
}

/** Valor de una celda sin arco. */
export function sinArco(g: Grafo): unknown {
  return g.matriz.cell.optional ? null : 0;
}

/** Valor de un arco nuevo: el de la especificación o 1. */
export function arcoNuevo(g: Grafo): number {
  const d = g.matriz.cell.default;
  return typeof d === 'number' && d !== 0 ? d : 1;
}

// ------------------------------------------------------------ distribuciones
// Todas en [0, 1] × [0, 1] con Y hacia abajo, como el dibujo del informe (iaqsb-core/layout.rs).

export type P = [number, number];

export function circulo(n: number): P[] {
  return Array.from({ length: n }, (_, i) => {
    const a = -Math.PI / 2 + (2 * Math.PI * i) / Math.max(n, 1);
    return [0.5 + 0.45 * Math.cos(a), 0.5 + 0.45 * Math.sin(a)] as P;
  });
}

function repartir(k: number, i: number): number {
  return k <= 1 ? 0.5 : 0.05 + (0.9 * i) / (k - 1);
}

export function bipartita(ns: number, nd: number): P[] {
  return [...Array.from({ length: ns }, (_, i) => [0.08, repartir(ns, i)] as P), ...Array.from({ length: nd }, (_, j) => [0.92, repartir(nd, j)] as P)];
}

/**
 * Capas por distancia (en arcos) desde el inicio siguiendo el sentido de los
 * arcos; los nodos que no se alcanzan, por la red sin sentido. En redes de flujo
 * reparte mejor que el camino más largo (que deja todo en una línea).
 */
export function capasDesde(n: number, arcos: [number, number][], inicio = 0): P[] {
  if (!n) return [];
  const out: number[][] = Array.from({ length: n }, () => []);
  const und: number[][] = Array.from({ length: n }, () => []);
  for (const [a, b] of arcos) {
    if (a === b || a >= n || b >= n) continue;
    out[a].push(b);
    und[a].push(b);
    und[b].push(a);
  }
  const capa = Array(n).fill(-1);
  const s = Math.min(Math.max(inicio, 0), n - 1);
  capa[s] = 0;
  const cola = [s];
  while (cola.length) {
    const u = cola.shift()!;
    for (const v of out[u]) if (capa[v] < 0) (capa[v] = capa[u] + 1), cola.push(v);
  }
  // los demás: junto a un vecino ya ubicado, o al final
  for (let vuelta = 0; vuelta < n && capa.some((l) => l < 0); vuelta++) {
    for (let v = 0; v < n; v++) {
      if (capa[v] >= 0) continue;
      const ub = und[v].filter((u) => capa[u] >= 0).map((u) => capa[u]);
      if (ub.length) capa[v] = Math.max(...ub) + 1;
    }
  }
  const maxl = Math.max(0, ...capa);
  for (let v = 0; v < n; v++) if (capa[v] < 0) capa[v] = maxl + 1;
  const nl = Math.max(0, ...capa) + 1;
  const cuenta = Array(nl).fill(0);
  for (const l of capa) cuenta[l]++;
  const idx = Array(nl).fill(0);
  return capa.map((l) => [repartir(nl, l), repartir(cuenta[l], idx[l]++)] as P);
}

export function cuadricula(n: number): P[] {
  const c = Math.max(1, Math.ceil(Math.sqrt(n * 1.6)));
  const r = Math.max(1, Math.ceil(n / c));
  return Array.from({ length: n }, (_, i) => [repartir(c, i % c), repartir(r, Math.floor(i / c))] as P);
}

/** Posiciones guardadas por WinQSB (fila, columna) llevadas a [0, 1]. */
export function desdeCuadricula(celdas: [number, number][]): P[] | null {
  if (!celdas.length || celdas.every((c) => c[0] === celdas[0][0] && c[1] === celdas[0][1])) return null;
  const rs = celdas.map((c) => c[0]);
  const cs = celdas.map((c) => c[1]);
  const f = (v: number, lo: number, hi: number) => (hi === lo ? 0.5 : 0.05 + (0.9 * (v - lo)) / (hi - lo));
  const [r0, r1, c0, c1] = [Math.min(...rs), Math.max(...rs), Math.min(...cs), Math.max(...cs)];
  return celdas.map(([r, c]) => [f(c, c0, c1), f(r, r0, r1)] as P);
}

// ------------------------------------------------------------ solución

export interface Superpuesta {
  titulo?: string;
  /** Arcos de la solución: índices de nodo (en la bipartita, destino = ns + j). */
  arcos: { a: number; b: number; etiqueta?: string; ruta?: number }[];
  nodos: Set<number>;
  /** Rutas del ruteo (colores por vehículo). */
  rutas: { nombre: string; nota?: string }[];
}

function bloquesPrincipales(r: Report): Block[] {
  return r.sections.filter((s) => s.id !== 'fuentes' && s.id !== 'pasos').flatMap((s) => s.blocks);
}

/**
 * La solución del informe sobre los nodos del editor: el primer dibujo de red
 * con arcos resaltados o, en el ruteo, el primer mapa de rutas. Los nodos se
 * reconocen por el nombre (o por el id n3, s0, d2 que usa el módulo de redes).
 */
export function solucionEnRed(r: Report | null | undefined, nombres: string[], ns = 0): Superpuesta | null {
  if (!r) return null;
  const porNombre = new Map<string, number>();
  nombres.forEach((n, i) => {
    if (!porNombre.has(n)) porNombre.set(n, i);
  });
  const bloques = bloquesPrincipales(r);
  const red = bloques.find((b): b is NetworkBlock => b.type === 'network' && b.edges.some((e) => e.tone === 'highlight'));
  if (red) {
    const idx = new Map<string, number>();
    for (const nd of red.nodes) {
      const m = /^([nsd])(\d+)$/.exec(nd.id);
      let i: number | undefined;
      if (m) {
        const k = Number(m[2]) + (m[1] === 'd' ? ns : 0);
        if (nombres[k] === nd.label) i = k;
      }
      i ??= porNombre.get(nd.label);
      if (i !== undefined) idx.set(nd.id, i);
    }
    const arcos = red.edges
      .filter((e) => e.tone === 'highlight' && idx.has(e.from) && idx.has(e.to))
      .map((e) => ({ a: idx.get(e.from)!, b: idx.get(e.to)!, etiqueta: e.label }));
    if (!arcos.length) return null;
    const nodos = new Set<number>();
    for (const e of arcos) nodos.add(e.a).add(e.b);
    return { titulo: red.title, arcos, nodos, rutas: [] };
  }
  const mapa = bloques.find((b): b is MapBlock => b.type === 'map' && b.routes.length > 0);
  if (mapa) {
    const idx = new Map<string, number>();
    for (const p of mapa.points) {
      const i = porNombre.get(p.label);
      if (i !== undefined) idx.set(p.id, i);
    }
    const arcos: Superpuesta['arcos'] = [];
    mapa.routes.forEach((rt, k) => {
      for (let s = 0; s + 1 < rt.stops.length; s++) {
        const a = idx.get(rt.stops[s]);
        const b = idx.get(rt.stops[s + 1]);
        if (a !== undefined && b !== undefined && a !== b) arcos.push({ a, b, ruta: k });
      }
    });
    if (!arcos.length) return null;
    const nodos = new Set<number>();
    for (const e of arcos) nodos.add(e.a).add(e.b);
    return { titulo: mapa.title, arcos, nodos, rutas: mapa.routes.map((x) => ({ nombre: x.name, nota: x.note })) };
  }
  return null;
}

/** Primer mapa de rutas del informe (para dibujar las rutas sobre el mapa real). */
export function mapaDelInforme(r: Report | null | undefined): MapBlock | null {
  if (!r) return null;
  return (bloquesPrincipales(r).find((b) => b.type === 'map') as MapBlock | undefined) ?? null;
}

/** Colores de las rutas, los mismos del plano del informe (MapView y iaqsb-render). */
export const COLORES_RUTA = ['var(--solver)', 'var(--binding)', '#10b981', '#ef4444', '#0ea5e9', '#d946ef', '#84cc16', '#f97316', '#14b8a6', '#a855f7', '#64748b', '#eab308'];
export const colorRuta = (k: number | undefined) => (k === undefined ? 'var(--ink-3)' : COLORES_RUTA[k % COLORES_RUTA.length]);

/** ¿Modelo con puntos en latitud y longitud (mapa real)? Devuelve la hoja de los puntos. */
export function detectarMapa(pt: ProblemType | null | undefined, model: any): RecordsW | null {
  if (!pt || model?.metric !== 'haversine') return null;
  return (
    pt.layout.find((w): w is RecordsW => w.type === 'records' && w.columns.some((c) => c.key === 'x') && w.columns.some((c) => c.key === 'y')) ?? null
  );
}

/** Punto del mapa real (latitud y longitud en grados). */
export interface PuntoGeo {
  lat: number;
  lon: number;
  nombre: string;
  deposito?: boolean;
  nota?: string;
  /** Ruta a la que pertenece (para el color del marcador). */
  ruta?: number;
  sel?: boolean;
}

/** Recorrido de un vehículo en el mapa real: [latitud, longitud] de cada parada. */
export interface RutaGeo {
  nombre: string;
  coords: [number, number][];
  /** Ruta de una resolución anterior a los últimos cambios. */
  vieja?: boolean;
}
