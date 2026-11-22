<script lang="ts">
  // Editor visual de redes: los nodos y arcos del mismo modelo que editan las
  // tablas (la matriz de arcos y los datos de cada nodo). Se arrastran los
  // nodos, doble clic en un espacio vacío crea un nodo, arrastrar desde el
  // punto de un nodo (o Mayús + arrastrar, o el modo «Conectar») crea un arco,
  // y un clic abre un recuadro para editar sus valores. Después de resolver,
  // la solución del informe se dibuja encima.
  import { tick } from 'svelte';
  import type { Field, Report } from '../../lib/gtypes';
  import { shown } from '../../lib/gtypes';
  import { defaults } from '../../lib/shell.svelte';
  import { num } from '../../lib/format';
  import { t } from '../../lib/i18n.svelte';
  import { ui } from '../../lib/ui.svelte';
  import FieldInput from './FieldInput.svelte';
  import { freshName } from './gridnav';
  import {
    arcoNuevo,
    bipartita,
    capasDesde,
    circulo,
    colorRuta,
    cuadricula,
    desdeCuadricula,
    hayArco,
    noDirigida,
    sinArco,
    solucionEnRed,
    type Grafo,
    type P
  } from '../../lib/grafo';

  let {
    g,
    model,
    modulo,
    onchange,
    report = null,
    stale = false
  }: { g: Grafo; model: any; modulo: string; onchange: () => void; report?: Report | null; stale?: boolean } = $props();

  // lienzo virtual (el SVG se escala al ancho disponible)
  const W = 960;
  const H = 540;
  const M = 56;

  // ---------------------------------------------------------------- datos

  const filas = $derived((model[g.matriz.rows.path] ?? []) as any[]);
  const cols = $derived(g.bipartita ? ((model[g.matriz.cols.path] ?? []) as any[]) : filas);
  const ns = $derived(g.bipartita ? filas.length : 0);
  const kf = $derived(g.matriz.rows.label_key);
  const kc = $derived(g.matriz.cols.label_key);
  const mat = $derived((model[g.matriz.path] ?? []) as any[][]);
  const dirigido = $derived(!g.bipartita && !noDirigida(modulo, model));

  type Nodo = { i: number; obj: any; nombre: string; lado: 'f' | 'c' };
  const nodos = $derived<Nodo[]>(
    g.bipartita
      ? [...filas.map((o, i) => ({ i, obj: o, nombre: String(o?.[kf] ?? ''), lado: 'f' as const })), ...cols.map((o, j) => ({ i: ns + j, obj: o, nombre: String(o?.[kc] ?? ''), lado: 'c' as const }))]
      : filas.map((o, i) => ({ i, obj: o, nombre: String(o?.[kf] ?? ''), lado: 'f' as const }))
  );
  const nombres = $derived(nodos.map((n) => n.nombre));

  /** Celda de la matriz que guarda el arco a → b. */
  function celda(a: number, b: number): [number, number] {
    return g.bipartita ? [a, b - ns] : [a, b];
  }
  function leer(a: number, b: number): unknown {
    const [r, c] = celda(a, b);
    return mat[r]?.[c];
  }
  function existe(a: number, b: number): boolean {
    if (g.bipartita) return a < ns && b >= ns && hayArco(g, leer(a, b));
    return hayArco(g, leer(a, b)) || (!dirigido && hayArco(g, leer(b, a)));
  }
  function valor(a: number, b: number): unknown {
    if (!g.bipartita && !dirigido && !hayArco(g, leer(a, b))) return leer(b, a);
    return leer(a, b);
  }

  type Arco = { a: number; b: number; v: unknown };
  const arcos = $derived.by<Arco[]>(() => {
    const out: Arco[] = [];
    if (g.bipartita) {
      for (let i = 0; i < filas.length; i++) for (let j = 0; j < cols.length; j++) if (hayArco(g, mat[i]?.[j])) out.push({ a: i, b: ns + j, v: mat[i][j] });
      return out;
    }
    const n = filas.length;
    for (let i = 0; i < n; i++) {
      for (let j = dirigido ? 0 : i; j < n; j++) {
        if (i === j && !g.lazos) continue;
        if (dirigido) {
          if (hayArco(g, mat[i]?.[j])) out.push({ a: i, b: j, v: mat[i][j] });
        } else if (hayArco(g, mat[i]?.[j]) || hayArco(g, mat[j]?.[i])) {
          out.push({ a: i, b: j, v: hayArco(g, mat[i]?.[j]) ? mat[i][j] : mat[j][i] });
        }
      }
    }
    return out;
  });

  // ---------------------------------------------------------------- posiciones

  /** Distribución automática en [0, 1]², la misma del dibujo del informe. */
  function automatica(tipo: 'auto' | 'circulo' | 'capas' | 'cuadricula' | 'columnas'): P[] {
    const n = nodos.length;
    if (g.bipartita) return tipo === 'circulo' ? circulo(n) : tipo === 'cuadricula' ? cuadricula(n) : bipartita(ns, cols.length);
    if (tipo === 'auto') {
      const wq = Array.isArray(model.positions) && model.positions.length === n ? desdeCuadricula(model.positions) : null;
      if (wq) return wq;
      if (g.coords === 'plano') {
        // ruteo sin coordenadas: el depósito al centro y los demás en círculo (como el informe)
        return nodos.map((_, i) => {
          if (i === 0) return [0.5, 0.5] as P;
          const a = -Math.PI / 2 + (2 * Math.PI * (i - 1)) / Math.max(n - 1, 1);
          return [0.5 + 0.42 * Math.cos(a), 0.5 + 0.42 * Math.sin(a)] as P;
        });
      }
      // árboles, agente viajero y cadenas de Markov (todos con todos): en círculo
      tipo = model.problem_type === 'min_spanning_tree' || model.problem_type === 'tsp' || g.lazos ? 'circulo' : 'capas';
    }
    if (tipo === 'circulo') return circulo(n);
    if (tipo === 'cuadricula') return cuadricula(n);
    const ini = nombres.findIndex((x) => x.toUpperCase() === String(model.start ?? '').trim().toUpperCase());
    return capasDesde(n, arcos.flatMap((e) => (dirigido ? [[e.a, e.b] as [number, number]] : [[e.a, e.b] as [number, number], [e.b, e.a] as [number, number]])), Math.max(ini, 0));
  }
  // con lazos hace falta más margen (salen hacia fuera del dibujo)
  const aPantalla = (p: P): P => {
    const mx = g.lazos ? M + 50 : M;
    const my = g.lazos ? M + 22 : M * 0.8;
    return [mx + p[0] * (W - 2 * mx), my + p[1] * (H - 2 * my)];
  };
  const auto = $derived(automatica('auto').map(aPantalla));

  const guardada = (o: any) => typeof o?.x === 'number' && typeof o?.y === 'number' && Number.isFinite(o.x) && Number.isFinite(o.y);

  // coordenadas del usuario (X, Y visibles, Y hacia arriba): se encuadran en el lienzo
  type Vista = { x0: number; y0: number; s: number };
  function encuadre(): Vista {
    const con = nodos.filter((n) => guardada(n.obj));
    if (con.length < 2) return { x0: -12 - (W / 2 - M) / 20, y0: -12, s: 20 };
    const xs = con.map((n) => n.obj.x);
    const ys = con.map((n) => n.obj.y);
    let [x0, x1, y0, y1] = [Math.min(...xs), Math.max(...xs), Math.min(...ys), Math.max(...ys)];
    if (x1 - x0 < 1e-9) [x0, x1] = [x0 - 1, x1 + 1];
    if (y1 - y0 < 1e-9) [y0, y1] = [y0 - 1, y1 + 1];
    const s = Math.min((W - 2 * M) / (x1 - x0), (H - 1.6 * M) / (y1 - y0));
    // centrado
    const cx = (x0 + x1) / 2;
    const cy = (y0 + y1) / 2;
    return { x0: cx - W / 2 / s, y0: cy - H / 2 / s, s };
  }
  let vistaFija = $state<Vista | null>(null);
  const vista = $derived(g.coords === 'plano' ? (vistaFija ?? encuadre()) : { x0: 0, y0: 0, s: 1 });
  function deMundo(o: any): P {
    if (g.coords === 'pantalla') return [o.x, o.y];
    return [(o.x - vista.x0) * vista.s, H - (o.y - vista.y0) * vista.s];
  }
  function aMundo(p: P): P {
    if (g.coords === 'pantalla') return [Math.round(p[0]), Math.round(p[1])];
    const paso = 10 ** Math.floor(Math.log10(1 / vista.s));
    const r = (v: number) => Math.round(v / paso) * paso;
    return [Number(r(vista.x0 + p[0] / vista.s).toPrecision(12)), Number(r(vista.y0 + (H - p[1]) / vista.s).toPrecision(12))];
  }

  // posición mientras se arrastra (se guarda al soltar)
  let arrastre = $state<{ i: number; x: number; y: number; dx: number; dy: number; x0: number; y0: number; movido: boolean; id: number } | null>(null);

  const pos = $derived(
    nodos.map((n, k): P => {
      if (arrastre && arrastre.i === k) return [arrastre.x, arrastre.y];
      return guardada(n.obj) ? deMundo(n.obj) : auto[k] ?? [W / 2, H / 2];
    })
  );
  const ancho = (k: number) => Math.max(46, (nombres[k]?.length ?? 0) * 7.4 + 24);
  const ALTO = 32;

  function limitar(p: P): P {
    return [Math.min(W - 24, Math.max(24, p[0])), Math.min(H - 20, Math.max(20, p[1]))];
  }

  /** Guarda en el modelo la posición de todos los nodos (los que no la tenían quedan donde se ven). */
  function fijarPosiciones(cambios: Map<number, P>) {
    for (const n of nodos) {
      const p = cambios.get(n.i) ?? pos[n.i];
      if (!cambios.has(n.i) && guardada(n.obj)) continue;
      const [x, y] = aMundo(limitar(p));
      n.obj.x = x;
      n.obj.y = y;
    }
    onchange();
  }

  function ordenar(tipo: 'circulo' | 'capas' | 'cuadricula' | 'columnas') {
    vistaFija = null;
    const ps = automatica(tipo).map(aPantalla);
    if (g.coords === 'plano') {
      // en el plano del usuario se conserva la escala actual
      vistaFija = vista;
    }
    fijarPosiciones(new Map(ps.map((p, k) => [k, p])));
    vistaFija = null;
    ui.avisar(t('Red ordenada'), 'info');
  }

  // ---------------------------------------------------------------- dibujo

  function borde(k: number, dx: number, dy: number): P {
    const [x, y] = pos[k];
    const len = Math.hypot(dx, dy) || 1;
    const ux = dx / len;
    const uy = dy / len;
    const tx = Math.abs(ux) > 1e-9 ? ancho(k) / 2 / Math.abs(ux) : Infinity;
    const ty = Math.abs(uy) > 1e-9 ? ALTO / 2 / Math.abs(uy) : Infinity;
    const s = Math.min(tx, ty) + 3;
    return [x + ux * s, y + uy * s];
  }

  type Geo = { d: string; en: (u: number) => P };
  function geometria(a: number, b: number, curva = 0): Geo | null {
    const A = pos[a];
    const B = pos[b];
    if (!A || !B) return null;
    if (a === b) {
      // lazo hacia fuera del dibujo
      let ox = A[0] - W / 2;
      let oy = A[1] - H / 2;
      const ol = Math.hypot(ox, oy);
      [ox, oy] = ol < 1 ? [0, -1] : [ox / ol, oy / ol];
      const [px, py] = [-oy, ox];
      const base = borde(a, ox, oy);
      const s: P = [base[0] + px * 9, base[1] + py * 9];
      const e: P = [base[0] - px * 9, base[1] - py * 9];
      const c1: P = [s[0] + ox * 46 + px * 22, s[1] + oy * 46 + py * 22];
      const c2: P = [e[0] + ox * 46 - px * 22, e[1] + oy * 46 - py * 22];
      const top: P = [base[0] + ox * 40, base[1] + oy * 40];
      return { d: `M${s[0]},${s[1]} C${c1[0]},${c1[1]} ${c2[0]},${c2[1]} ${e[0]},${e[1]}`, en: () => top };
    }
    const dx = B[0] - A[0];
    const dy = B[1] - A[1];
    const len = Math.hypot(dx, dy) || 1;
    const cx = (A[0] + B[0]) / 2 + (-dy / len) * curva;
    const cy = (A[1] + B[1]) / 2 + (dx / len) * curva;
    const s = borde(a, cx - A[0], cy - A[1]);
    const e = borde(b, cx - B[0], cy - B[1]);
    if (!curva) return { d: `M${s[0]},${s[1]} L${e[0]},${e[1]}`, en: (u) => [s[0] + (e[0] - s[0]) * u, s[1] + (e[1] - s[1]) * u] };
    return {
      d: `M${s[0]},${s[1]} Q${cx},${cy} ${e[0]},${e[1]}`,
      en: (u) => [(1 - u) ** 2 * s[0] + 2 * (1 - u) * u * cx + u * u * e[0], (1 - u) ** 2 * s[1] + 2 * (1 - u) * u * cy + u * u * e[1]]
    };
  }
  const curvaDe = (a: number, b: number) => (dirigido && a !== b && existe(b, a) ? 20 : 0);

  function texto(v: unknown): string {
    if (typeof v === 'number') return Number.isFinite(v) ? num(v, 4) : v > 0 ? 'M' : '-M';
    return String(v ?? '');
  }

  /** Datos que se ven debajo de cada nodo: oferta, demanda… (los distintos de cero). */
  function nota(n: Nodo): string {
    const fs = (n.lado === 'c' ? g.camposCol : g.camposFila).filter((f) => ['number', 'integer', 'bound'].includes(f.kind.type));
    return fs
      .filter((f) => n.obj?.[f.key] !== null && n.obj?.[f.key] !== undefined && n.obj[f.key] !== 0 && shown(f, (k) => n.obj?.[k] ?? model[k]))
      .slice(0, 2)
      .map((f) => `${f.label.toLowerCase()} ${texto(n.obj[f.key])}`)
      .join(' · ');
  }

  const nombreInicio = $derived(g.inicio ? String(model[g.inicio.key] ?? '').trim() : '');
  const nombreFin = $derived(g.fin ? String(model[g.fin.key] ?? '').trim() : '');
  function esInicio(k: number) {
    if (!g.inicio) return false;
    return nombreInicio ? nombres[k]?.toUpperCase() === nombreInicio.toUpperCase() : k === 0;
  }
  function esFin(k: number) {
    if (!g.fin) return false;
    return nombreFin ? nombres[k]?.toUpperCase() === nombreFin.toUpperCase() : k === nodos.length - 1;
  }

  // ---------------------------------------------------------------- solución

  let verSolucion = $state(true);
  const sol = $derived(solucionEnRed(report, nombres, ns));
  /** Arcos del modelo que están en la solución, con su etiqueta (flujo, distancia…). */
  const enSol = $derived.by(() => {
    const m = new Map<string, { etiqueta?: string; ruta?: number }>();
    if (!sol || !verSolucion) return m;
    for (const e of sol.arcos) {
      m.set(`${e.a}-${e.b}`, e);
      if (!dirigido && !g.bipartita) m.set(`${e.b}-${e.a}`, e);
    }
    return m;
  });

  /** Texto de cada arco (valor y, si está en la solución, su flujo). */
  function etiquetaArco(e: Arco): { txt: string; sol: boolean; ruta?: number } {
    const lt = texto(e.v);
    const es = enSol.get(`${e.a}-${e.b}`);
    // en la solución se ve lo que dice el informe (flujo, envío…); sin ella, el valor del modelo
    return { txt: es?.etiqueta ?? lt, sol: !!es, ruta: es?.ruta };
  }

  // etiquetas sin solaparse: se prueba el centro del arco y después otros puntos a lo largo de él
  const lugares = $derived.by(() => {
    const puestos: { x: number; y: number; w: number }[] = nodos.map((_, k) => ({ x: pos[k][0], y: pos[k][1], w: ancho(k) }));
    const out = new Map<string, P>();
    const orden = [...arcos].sort((p, q) => Number(enSol.has(`${q.a}-${q.b}`)) - Number(enSol.has(`${p.a}-${p.b}`)));
    const us = g.bipartita ? [0.3, 0.22, 0.38, 0.46, 0.16, 0.54] : [0.5, 0.36, 0.64, 0.26, 0.74, 0.44, 0.56, 0.3, 0.7];
    for (const e of orden) {
      const geo = geometria(e.a, e.b, curvaDe(e.a, e.b));
      if (!geo) continue;
      const w = etiquetaArco(e).txt.length * 7 + 12;
      let mejor = geo.en(us[0]);
      for (const u of us) {
        const q = geo.en(u);
        if (!puestos.some((o) => Math.abs(o.x - q[0]) < (o.w + w) / 2 + 2 && Math.abs(o.y - q[1]) < 21)) {
          mejor = q;
          break;
        }
      }
      puestos.push({ x: mejor[0], y: mejor[1], w });
      out.set(`${e.a}-${e.b}`, mejor);
    }
    return out;
  });

  // ---------------------------------------------------------------- selección y edición

  type Sel = { tipo: 'nodo'; i: number } | { tipo: 'arco'; a: number; b: number } | null;
  let sel = $state<Sel>(null);
  let recuadro = $state(false);
  let modo = $state<'mover' | 'conectar'>('mover');
  /** Primer nodo de un arco que se está creando con clics (Mayús + clic, modo Conectar o la tecla C). */
  let desde = $state<number | null>(null);
  let enlace = $state<{ a: number; x: number; y: number; sobre: number | null } | null>(null);
  let svg: SVGSVGElement | undefined = $state();
  let anchoLienzo = $state(W);
  let altoPop = $state(200);
  let anchoPop = $state(270);
  let caja: HTMLDivElement | undefined = $state();

  $effect(() => {
    // si la selección deja de existir (se borró desde la tabla), se cierra
    if (sel?.tipo === 'nodo' && sel.i >= nodos.length) cerrar();
    if (sel?.tipo === 'arco' && (sel.a >= nodos.length || sel.b >= nodos.length || !existe(sel.a, sel.b))) cerrar();
  });

  function cerrar() {
    sel = null;
    recuadro = false;
  }

  async function abrir(s: Sel, foco = true) {
    sel = s;
    recuadro = !!s;
    if (!s || !foco) return;
    await tick();
    const el = caja?.querySelector<HTMLElement>('.pop input:not([type=checkbox]), .pop select');
    el?.focus();
    if (el instanceof HTMLInputElement) el.select();
  }

  function punto(e: PointerEvent | MouseEvent): P {
    const m = svg?.getScreenCTM();
    if (!m || !svg) return [0, 0];
    const p = new DOMPoint(e.clientX, e.clientY).matrixTransform(m.inverse());
    return [p.x, p.y];
  }

  function nodoEn(p: P): number | null {
    for (let k = nodos.length - 1; k >= 0; k--) {
      const [x, y] = pos[k];
      if (Math.abs(p[0] - x) <= ancho(k) / 2 + 4 && Math.abs(p[1] - y) <= ALTO / 2 + 4) return k;
    }
    return null;
  }

  function bajarEnNodo(e: PointerEvent, k: number, conAsa = false) {
    if (e.button !== 0) return;
    e.stopPropagation();
    e.preventDefault();
    (e.currentTarget as Element).closest('g.nodo')?.querySelector<SVGGElement>('.cuerpo')?.focus?.();
    svg?.setPointerCapture(e.pointerId);
    const p = punto(e);
    if (conAsa || modo === 'conectar' || e.shiftKey) {
      enlace = { a: k, x: p[0], y: p[1], sobre: null };
      return;
    }
    arrastre = { i: k, x: pos[k][0], y: pos[k][1], dx: p[0] - pos[k][0], dy: p[1] - pos[k][1], x0: p[0], y0: p[1], movido: false, id: e.pointerId };
    if (g.coords === 'plano') vistaFija = vista;
  }

  function mover(e: PointerEvent) {
    if (!arrastre && !enlace) return;
    const p = punto(e);
    if (arrastre) {
      if (!arrastre.movido && Math.hypot(p[0] - arrastre.x0, p[1] - arrastre.y0) < 4) return;
      const [x, y] = limitar([p[0] - arrastre.dx, p[1] - arrastre.dy]);
      arrastre = { ...arrastre, x, y, movido: true };
    } else if (enlace) {
      const s = nodoEn(p);
      enlace = { ...enlace, x: p[0], y: p[1], sobre: s };
    }
  }

  function soltar(e: PointerEvent) {
    if (arrastre) {
      const a = arrastre;
      if (a.movido) {
        fijarPosiciones(new Map([[a.i, [a.x, a.y] as P]]));
        arrastre = null;
        vistaFija = null;
      } else {
        arrastre = null;
        vistaFija = null;
        clicEnNodo(a.i, e.shiftKey);
      }
    } else if (enlace) {
      const en = enlace;
      enlace = null;
      const b = nodoEn(punto(e));
      if (b === null) return;
      if (b === en.a && !g.lazos) {
        // clic en el asa sin arrastrar: el arco se termina con el próximo clic
        desde = en.a;
        ui.avisar(t('Elige el nodo de llegada del arco'), 'info');
        return;
      }
      conectar(en.a, b);
    }
  }

  function clicEnNodo(k: number, mayus: boolean) {
    if (desde !== null) {
      const a = desde;
      desde = null;
      if (a !== k || g.lazos) {
        conectar(a, k);
        return;
      }
    }
    if (mayus || modo === 'conectar') {
      desde = k;
      cerrar();
      return;
    }
    abrir({ tipo: 'nodo', i: k }, false);
  }

  function conectar(a: number, b: number) {
    if (g.bipartita) {
      if (a >= ns && b < ns) [a, b] = [b, a];
      if (!(a < ns && b >= ns)) {
        ui.avisar(t('En esta red los arcos van de un origen a un destino'), 'error');
        return;
      }
    }
    if (a === b && !g.lazos) return;
    if (!dirigido && !g.bipartita && a > b) [a, b] = [b, a];
    if (!existe(a, b)) {
      const [r, c] = celda(a, b);
      model[g.matriz.path][r][c] = arcoNuevo(g);
      onchange();
    }
    abrir({ tipo: 'arco', a, b });
  }

  function fondoAbajo(e: PointerEvent) {
    if (e.button !== 0) return;
    desde = null;
    cerrar();
  }

  function dobleClic(e: MouseEvent) {
    if (nodoEn(punto(e)) !== null) return;
    agregar(punto(e));
  }

  // ---------------------------------------------------------------- nodos

  function nuevoElemento(path: string, label: string, key: string) {
    const list = (model[path] ??= []) as any[];
    const extras = [
      ...g.matrices.flatMap((s) => [...(s.rows.path === path ? s.rows.extra : []), ...(s.cols.path === path ? s.cols.extra : [])]),
      ...g.registros.filter((r) => r.path === path).flatMap((r) => r.columns.filter((c) => c.key !== key))
    ];
    return { ...defaults(extras), [key]: freshName(list.map((x) => x[key]), label) };
  }

  function crecer(path: string) {
    for (const s of g.matrices) {
      const mtx = (model[s.path] ??= []) as any[][];
      const b = s.cell.optional ? null : (s.cell.default ?? 0);
      if (s.rows.path === path && s.cols.path === path) {
        for (const row of mtx) row.push(b);
        mtx.push(Array(model[path].length).fill(b));
      } else if (s.cols.path === path) {
        for (const row of mtx) row.push(b);
      } else if (s.rows.path === path) {
        mtx.push(Array((model[s.cols.path] ?? []).length).fill(b));
      }
    }
  }

  function agregar(p?: P, lado: 'f' | 'c' = 'f') {
    let sitio: P = p ?? [W / 2 + (Math.random() - 0.5) * 120, H / 2 + (Math.random() - 0.5) * 80];
    if (g.bipartita && p) lado = p[0] < W / 2 ? 'f' : 'c';
    if (g.bipartita && !p) sitio = [lado === 'f' ? M + 20 : W - M - 20, H / 2 + (Math.random() - 0.5) * 120];
    const ax = lado === 'c' ? g.matriz.cols : g.matriz.rows;
    // los demás nodos quedan donde se ven
    fijarPosiciones(new Map());
    const item = nuevoElemento(ax.path, g.bipartita ? ax.label : etiquetaNodo, ax.label_key);
    const [x, y] = aMundo(limitar(sitio));
    item.x = x;
    item.y = y;
    model[ax.path].push(item);
    crecer(ax.path);
    onchange();
    const nf = model[g.matriz.rows.path].length;
    abrir({ tipo: 'nodo', i: lado === 'c' ? nf + model[g.matriz.cols.path].length - 1 : nf - 1 });
  }

  function quitarNodo(k: number) {
    const n = nodos[k];
    if (!n) return;
    const ax = n.lado === 'c' ? g.matriz.cols : g.matriz.rows;
    const list = model[ax.path] as any[];
    if (list.length <= 1) {
      ui.avisar(g.bipartita ? t('La red necesita al menos un origen y un destino') : t('La red necesita al menos un nodo'), 'error');
      return;
    }
    const idx = n.lado === 'c' ? k - ns : k;
    list.splice(idx, 1);
    for (const s of g.matrices) {
      const mtx = (model[s.path] ?? []) as any[][];
      if (s.rows.path === ax.path) mtx.splice(idx, 1);
      if (s.cols.path === ax.path) for (const row of mtx) row.splice(idx, 1);
    }
    // referencias por nombre: nodo inicial y final, cotas de los arcos
    for (const f of [g.inicio, g.fin]) if (f && String(model[f.key] ?? '').toUpperCase() === n.nombre.toUpperCase()) model[f.key] = null;
    if (g.cotas && Array.isArray(model[g.cotas.path])) model[g.cotas.path] = model[g.cotas.path].filter((c: any) => c.from !== n.nombre && c.to !== n.nombre);
    cerrar();
    onchange();
    ui.avisar(t('Nodo {0} eliminado', n.nombre), 'info');
  }

  function renombrar(k: number, v: string) {
    const n = nodos[k];
    const nuevo = v.trim();
    if (!n || !nuevo || nuevo === n.nombre) return;
    const mismos = nodos.filter((x) => x.lado === n.lado && x.i !== k).map((x) => x.nombre.toUpperCase());
    if (mismos.includes(nuevo.toUpperCase())) {
      ui.avisar(t('Ya hay un nodo llamado {0}', nuevo), 'error');
      return;
    }
    for (const f of [g.inicio, g.fin]) if (f && String(model[f.key] ?? '').toUpperCase() === n.nombre.toUpperCase()) model[f.key] = nuevo;
    if (g.cotas && Array.isArray(model[g.cotas.path])) {
      for (const c of model[g.cotas.path]) {
        if (c.from === n.nombre) c.from = nuevo;
        if (c.to === n.nombre) c.to = nuevo;
      }
    }
    n.obj[n.lado === 'c' ? kc : kf] = nuevo;
    onchange();
  }

  function marcar(f: Field | null, k: number) {
    if (!f) return;
    const actual = String(model[f.key] ?? '').toUpperCase();
    model[f.key] = actual === nombres[k].toUpperCase() ? null : nombres[k];
    onchange();
  }

  // ---------------------------------------------------------------- arcos

  function fijarValor(a: number, b: number, v: unknown) {
    const m = model[g.matriz.path];
    if (g.bipartita) {
      m[a][b - ns] = v;
    } else if (!dirigido) {
      const ab = hayArco(g, m[a][b]);
      const ba = hayArco(g, m[b][a]);
      if (ab) m[a][b] = v;
      if (ba) m[b][a] = v;
      if (!ab && !ba) m[a][b] = v;
    } else {
      m[a][b] = v;
    }
    onchange();
  }

  function quitarArco(a: number, b: number) {
    const m = model[g.matriz.path];
    const vacio = sinArco(g);
    if (g.bipartita) m[a][b - ns] = vacio;
    else {
      m[a][b] = vacio;
      if (!dirigido) m[b][a] = vacio;
    }
    if (g.cotas && Array.isArray(model[g.cotas.path])) model[g.cotas.path] = model[g.cotas.path].filter((c: any) => !(c.from === nombres[a] && c.to === nombres[b]));
    cerrar();
    onchange();
  }

  function invertir(a: number, b: number) {
    if (existe(b, a)) return;
    const v = leer(a, b);
    const m = model[g.matriz.path];
    m[b][a] = v;
    m[a][b] = sinArco(g);
    onchange();
    abrir({ tipo: 'arco', a: b, b: a }, false);
  }

  // cotas de flujo del arco (hoja «Cotas de flujo»)
  const camposCota = $derived(g.cotas ? g.cotas.columns.filter((c) => c.key !== 'from' && c.key !== 'to') : []);
  function cota(a: number, b: number): any | undefined {
    if (!g.cotas) return undefined;
    return (model[g.cotas.path] ?? []).find((c: any) => c.from === nombres[a] && c.to === nombres[b]);
  }
  function fijarCota(a: number, b: number, f: Field, v: unknown) {
    if (!g.cotas) return;
    const list = (model[g.cotas.path] ??= []) as any[];
    let c = cota(a, b);
    if (!c) {
      c = { ...defaults(g.cotas.columns), from: nombres[a], to: nombres[b] };
      list.push(c);
      c = list[list.length - 1];
    }
    c[f.key] = v;
    // sin cotas (mínimo 0 y sin máximo) no hace falta la fila
    if (camposCota.every((x) => c[x.key] === (x.optional ? null : (x.default ?? 0)))) model[g.cotas.path] = list.filter((x) => x !== c);
    onchange();
  }

  // ---------------------------------------------------------------- teclado

  function teclaNodo(e: KeyboardEvent, k: number) {
    const paso = e.shiftKey ? 1 : 10;
    const mov: Record<string, P> = { ArrowLeft: [-paso, 0], ArrowRight: [paso, 0], ArrowUp: [0, -paso], ArrowDown: [0, paso] };
    if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      if (desde !== null) clicEnNodo(k, false);
      else abrir({ tipo: 'nodo', i: k });
    } else if (e.key === 'Delete' || e.key === 'Backspace') {
      e.preventDefault();
      quitarNodo(k);
    } else if (e.key.toLowerCase() === 'c') {
      e.preventDefault();
      if (desde !== null && desde !== k) clicEnNodo(k, false);
      else {
        desde = k;
        ui.avisar(t('Elige el nodo de llegada del arco y pulsa Intro'), 'info');
      }
    } else if (mov[e.key]) {
      e.preventDefault();
      if (g.coords === 'plano') vistaFija = vista;
      const [dx, dy] = mov[e.key];
      fijarPosiciones(new Map([[k, limitar([pos[k][0] + dx, pos[k][1] + dy])]]));
      vistaFija = null;
    }
  }

  function teclaArco(e: KeyboardEvent, a: number, b: number) {
    if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      abrir({ tipo: 'arco', a, b });
    } else if (e.key === 'Delete' || e.key === 'Backspace') {
      e.preventDefault();
      quitarArco(a, b);
    }
  }

  function teclaCaja(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      if (desde !== null || enlace) {
        desde = null;
        enlace = null;
      } else if (recuadro) {
        const s = sel;
        cerrar();
        if (s?.tipo === 'nodo') caja?.querySelector<SVGElement>(`[data-nodo="${s.i}"]`)?.focus();
      }
    }
  }

  function quitarSeleccion() {
    if (sel?.tipo === 'nodo') quitarNodo(sel.i);
    else if (sel?.tipo === 'arco') quitarArco(sel.a, sel.b);
  }

  // ---------------------------------------------------------------- recuadro

  const ancla = $derived.by<P | null>(() => {
    if (!sel) return null;
    if (sel.tipo === 'nodo') return pos[sel.i] ?? null;
    const geo = geometria(sel.a, sel.b, curvaDe(sel.a, sel.b));
    return geo ? geo.en(0.5) : null;
  });
  const etiquetaNodo = $derived(
    g.registros[0]?.columns.find((c) => c.key === (g.registros[0]?.path === g.matriz.rows.path ? kf : kc))?.label ?? t('Nodo')
  );
  const campoX = $derived(g.coords === 'plano' ? g.registros.flatMap((r) => r.columns).find((c) => c.key === 'x') : undefined);
  const campoY = $derived(g.coords === 'plano' ? g.registros.flatMap((r) => r.columns).find((c) => c.key === 'y') : undefined);

  const uid = Math.random().toString(36).slice(2, 8);
  const ladoTxt = (n: Nodo) => (g.bipartita ? (n.lado === 'f' ? g.matriz.rows.label : g.matriz.cols.label) : etiquetaNodo);
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="editor-red" bind:this={caja} onkeydown={teclaCaja}>
  <div class="barra" role="toolbar" aria-label={t('Herramientas del editor visual')}>
    <div class="seg" role="group" aria-label={t('Modo')}>
      <button class:on={modo === 'mover'} aria-pressed={modo === 'mover'} onclick={() => ((modo = 'mover'), (desde = null))} title={t('Arrastra los nodos para ubicarlos')}>{t('Mover')}</button>
      <button class:on={modo === 'conectar'} aria-pressed={modo === 'conectar'} onclick={() => (modo = 'conectar')} title={t('Arrastra de un nodo a otro para crear un arco')}>{t('Conectar')}</button>
    </div>
    {#if g.bipartita}
      <button class="btn" onclick={() => agregar(undefined, 'f')}>+ {g.matriz.rows.label}</button>
      <button class="btn" onclick={() => agregar(undefined, 'c')}>+ {g.matriz.cols.label}</button>
    {:else}
      <button class="btn" onclick={() => agregar()}>+ {etiquetaNodo}</button>
    {/if}
    <label class="ord">
      <span class="sr">{t('Ordenar la red')}</span>
      <select
        aria-label={t('Ordenar la red')}
        onchange={(e) => {
          const v = (e.currentTarget as HTMLSelectElement).value;
          (e.currentTarget as HTMLSelectElement).value = '';
          if (v) ordenar(v as 'circulo' | 'capas' | 'cuadricula' | 'columnas');
        }}
      >
        <option value="">{t('Ordenar…')}</option>
        {#if g.bipartita}<option value="columnas">{t('En dos columnas')}</option>{:else}<option value="capas">{t('Por capas (de izquierda a derecha)')}</option>{/if}
        <option value="circulo">{t('En círculo')}</option>
        <option value="cuadricula">{t('En cuadrícula')}</option>
      </select>
    </label>
    <button class="btn" disabled={!sel} onclick={quitarSeleccion} title={t('Eliminar lo seleccionado (Supr)')}>{t('Eliminar')}</button>
    <span class="esp"></span>
    {#if sol}
      <label class="chk-sol" title={sol.titulo ?? ''}>
        <input type="checkbox" bind:checked={verSolucion} />
        {t('Mostrar la solución')}
      </label>
    {/if}
  </div>

  {#if sol && verSolucion && !stale && sol.titulo && desde === null}
    <p class="estado sol-t">{t('Resaltado: {0}', sol.titulo)}</p>
  {/if}
  {#if desde !== null}
    <p class="estado" role="status">{t('Arco desde {0}: haz clic en el nodo de llegada (Esc para cancelar)', nombres[desde])}</p>
  {:else if sol && verSolucion && stale}
    <p class="estado aviso" role="status">{t('La solución dibujada es de antes de los últimos cambios: resuelve de nuevo para actualizarla.')}</p>
  {/if}

  <div class="lienzo" bind:clientWidth={anchoLienzo}>
    <svg
      bind:this={svg}
      viewBox={`0 0 ${W} ${H}`}
      role="application"
      aria-label={t('Editor visual de la red')}
      aria-roledescription={t('editor de red')}
      class:conectando={modo === 'conectar' || desde !== null}
      onpointermove={mover}
      onpointerup={soltar}
      onpointercancel={() => ((arrastre = null), (enlace = null), (vistaFija = null))}
    >
      <defs>
        {#each ['arco', 'sol', 'sel'] as tn}
          <marker id={`fl-${uid}-${tn}`} viewBox="0 0 10 10" refX="9" refY="5" markerUnits="userSpaceOnUse" markerWidth={tn === 'arco' ? 10 : 13} markerHeight={tn === 'arco' ? 10 : 13} orient="auto-start-reverse">
            <path d="M0 0L10 5L0 10z" class={`pf-${tn}`} />
          </marker>
        {/each}
        {#each sol?.rutas ?? [] as _, k}
          <marker id={`fl-${uid}-r${k}`} viewBox="0 0 10 10" refX="9" refY="5" markerUnits="userSpaceOnUse" markerWidth="13" markerHeight="13" orient="auto-start-reverse">
            <path d="M0 0L10 5L0 10z" style:fill={colorRuta(k)} />
          </marker>
        {/each}
        <pattern id={`cuad-${uid}`} width="24" height="24" patternUnits="userSpaceOnUse">
          <path d="M24 0H0V24" class="cuad" />
        </pattern>
      </defs>
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <rect class="fondo" x="0" y="0" width={W} height={H} fill={`url(#cuad-${uid})`} onpointerdown={fondoAbajo} ondblclick={dobleClic} />

      {#if g.bipartita}
        <text class="col-t" x={M + 14} y="22" text-anchor="middle">{g.matriz.rows.label}</text>
        <text class="col-t" x={W - M - 14} y="22" text-anchor="middle">{g.matriz.cols.label}</text>
      {/if}

      <!-- arcos del modelo -->
      {#each arcos as e (e.a + '-' + e.b)}
        {@const geo = geometria(e.a, e.b, curvaDe(e.a, e.b))}
        {@const on = sel?.tipo === 'arco' && sel.a === e.a && sel.b === e.b}
        {#if geo}
          {@const lt = texto(e.v)}
          <g class="arco" class:on>
            <path class="trazo" d={geo.d} marker-end={dirigido || g.bipartita ? `url(#fl-${uid}-${on ? 'sel' : 'arco'})` : undefined} />
            <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
            <path
              class="toque"
              d={geo.d}
              tabindex="0"
              role="button"
              aria-label={t('Arco {0} {1} {2}: {3}', nombres[e.a], dirigido || g.bipartita ? '→' : '—', nombres[e.b], lt)}
              onpointerdown={(ev) => {
                ev.stopPropagation();
                abrir({ tipo: 'arco', a: e.a, b: e.b }, false);
              }}
              onkeydown={(ev) => teclaArco(ev, e.a, e.b)}
            />
          </g>
        {/if}
      {/each}

      <!-- solución del informe -->
      {#if sol && verSolucion}
        <g class="solucion" class:vieja={stale}>
          {#each sol.arcos as e, k (k)}
            {@const geo = geometria(e.a, e.b, curvaDe(e.a, e.b))}
            {#if geo}
              <path
                class="trazo-sol"
                d={geo.d}
                style:stroke={e.ruta !== undefined ? colorRuta(e.ruta) : undefined}
                marker-end={dirigido || g.bipartita || e.ruta !== undefined ? `url(#fl-${uid}-${e.ruta !== undefined ? 'r' + e.ruta : 'sol'})` : undefined}
              />
              {#if e.etiqueta && !existe(e.a, e.b)}
                {@const lp = geo.en(0.5)}
                <g class="et-sol" transform={`translate(${lp[0]} ${lp[1]})`}>
                  <rect x={-(e.etiqueta.length * 3.5) - 6} y="-9.5" width={e.etiqueta.length * 7 + 12} height="19" rx="9.5" />
                  <text y="4" text-anchor="middle">{e.etiqueta}</text>
                </g>
              {/if}
            {/if}
          {/each}
        </g>
      {/if}

      <!-- valores de los arcos (encima de los trazos) -->
      {#each arcos as e (e.a + '-' + e.b)}
        {@const lp = lugares.get(`${e.a}-${e.b}`)}
        {#if lp}
          {@const et = etiquetaArco(e)}
          <g class="et" class:sol={et.sol} class:vieja={et.sol && stale} transform={`translate(${lp[0]} ${lp[1]})`}>
            <rect x={-(et.txt.length * 3.5) - 6} y="-9.5" width={et.txt.length * 7 + 12} height="19" rx="9.5" style:stroke={et.ruta !== undefined ? colorRuta(et.ruta) : undefined} />
            <text y="4" text-anchor="middle">{et.txt}</text>
          </g>
        {/if}
      {/each}

      <!-- arco que se está creando -->
      {#if enlace}
        {@const A = pos[enlace.a]}
        <line class="enlace" x1={A[0]} y1={A[1]} x2={enlace.sobre !== null ? pos[enlace.sobre][0] : enlace.x} y2={enlace.sobre !== null ? pos[enlace.sobre][1] : enlace.y} />
      {/if}

      <!-- nodos -->
      {#each nodos as n, k (n.lado + n.i)}
        {@const [x, y] = pos[k]}
        {@const w = ancho(k)}
        {@const nt = nota(n)}
        {@const on = sel?.tipo === 'nodo' && sel.i === k}
        <g
          class="nodo"
          class:on
          class:origen={desde === k || enlace?.a === k}
          class:destino={enlace?.sobre === k}
          class:en-sol={!!(sol && verSolucion && sol.nodos.has(k))}
          class:arrastrando={arrastre?.i === k && arrastre.movido}
        >
          <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
          <g
            class="cuerpo"
            data-nodo={k}
            tabindex="0"
            role="button"
            aria-label={t('{0} {1}. Intro: editar; flechas: mover; C: crear un arco; Supr: eliminar', ladoTxt(n), n.nombre)}
            onpointerdown={(ev) => bajarEnNodo(ev, k)}
            onkeydown={(ev) => teclaNodo(ev, k)}
            ondblclick={(ev) => {
              ev.stopPropagation();
              abrir({ tipo: 'nodo', i: k });
            }}
          >
            <rect x={x - w / 2} y={y - ALTO / 2} width={w} height={ALTO} rx={g.bipartita ? 8 : 16} />
            <text class="nom" x={x} y={y + 4.5} text-anchor="middle">{n.nombre}</text>
          </g>
          {#if esInicio(k) || esFin(k)}
            <text class="marca" x={x} y={y - ALTO / 2 - 6} text-anchor="middle">{esInicio(k) && esFin(k) ? t('inicio y fin') : esInicio(k) ? t('inicio') : t('fin')}</text>
          {/if}
          {#if nt}
            <text class="nota" x={x} y={y + ALTO / 2 + 14} text-anchor="middle">{nt}</text>
          {/if}
          <!-- asa para crear arcos arrastrando -->
          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <circle class="asa" cx={x + w / 2} cy={y} r="6.5" onpointerdown={(ev) => bajarEnNodo(ev, k, true)}>
            <title>{t('Arrastra hasta otro nodo para crear un arco')}</title>
          </circle>
        </g>
      {/each}
    </svg>

    {#if recuadro && sel && ancla}
      {@const k = anchoLienzo / W}
      {@const alLado = ancla[0] / W > 0.58}
      {@const dx = (sel.tipo === 'nodo' ? ancho(sel.i) / 2 : 6) * k + 14}
      {@const izq = alLado ? ancla[0] * k - dx - anchoPop : ancla[0] * k + dx}
      {@const arriba = ancla[1] * k - altoPop * 0.35}
      <div
        class="pop"
        role="dialog"
        aria-label={sel.tipo === 'nodo' ? t('Datos del nodo') : t('Datos del arco')}
        bind:clientHeight={altoPop}
        bind:clientWidth={anchoPop}
        style:left={`${Math.min(Math.max(izq, 4), Math.max(4, anchoLienzo - anchoPop - 4))}px`}
        style:top={`${Math.max(4, Math.min(arriba, (H * k) - altoPop - 4))}px`}
      >
        {#if sel.tipo === 'nodo'}
          {@const n = nodos[sel.i]}
          {#if n}
            {@const campos = (n.lado === 'c' ? g.camposCol : g.camposFila).filter((f) => shown(f, (k) => n.obj?.[k] ?? model[k]))}
            <div class="ph">
              <strong>{ladoTxt(n)}</strong>
              <button class="x" aria-label={t('Cerrar')} onclick={cerrar}>×</button>
            </div>
            <label class="pc">
              <span>{t('Nombre')}</span>
              <input class="nombre" value={n.nombre} onchange={(e) => renombrar(n.i, (e.currentTarget as HTMLInputElement).value)} />
            </label>
            {#each campos as f (f.key)}
              <label class="pc">
                <span>{f.label}</span>
                <FieldInput field={f} value={n.obj[f.key]} onchange={(v) => { n.obj[f.key] = v; onchange(); }} />
              </label>
            {/each}
            {#if campoX && campoY}
              <div class="dos">
                <label class="pc"><span>{campoX.label}</span><FieldInput field={campoX} value={n.obj.x} onchange={(v) => { n.obj.x = v; onchange(); }} /></label>
                <label class="pc"><span>{campoY.label}</span><FieldInput field={campoY} value={n.obj.y} onchange={(v) => { n.obj.y = v; onchange(); }} /></label>
              </div>
            {/if}
            <div class="acc">
              {#if g.inicio}
                <button class="btn" aria-pressed={esInicio(sel.i) && !!nombreInicio} onclick={() => sel?.tipo === 'nodo' && marcar(g.inicio, sel.i)}>{t('Nodo inicial')}</button>
              {/if}
              {#if g.fin}
                <button class="btn" aria-pressed={esFin(sel.i) && !!nombreFin} onclick={() => sel?.tipo === 'nodo' && marcar(g.fin, sel.i)}>{t('Nodo final')}</button>
              {/if}
              <button
                class="btn"
                onclick={() => {
                  if (sel?.tipo !== 'nodo') return;
                  desde = sel.i;
                  cerrar();
                }}>{t('Crear arco desde aquí')}</button
              >
              <button class="btn peligro" onclick={() => sel?.tipo === 'nodo' && quitarNodo(sel.i)}>{t('Eliminar nodo')}</button>
            </div>
          {/if}
        {:else}
          {@const a = sel.a}
          {@const b = sel.b}
          {@const ct = cota(a, b)}
          <div class="ph">
            <strong>{nombres[a]} {dirigido || g.bipartita ? '→' : '—'} {nombres[b]}</strong>
            <button class="x" aria-label={t('Cerrar')} onclick={cerrar}>×</button>
          </div>
          <label class="pc">
            <span>{g.matriz.cell.label}</span>
            <FieldInput field={g.matriz.cell} value={valor(a, b)} onchange={(v) => (hayArco(g, v) ? fijarValor(a, b, v) : quitarArco(a, b))} />
          </label>
          {#if camposCota.length && dirigido}
            <div class="dos">
              {#each camposCota as f (f.key)}
                <label class="pc" title={f.help}>
                  <span>{t('Flujo {0}', f.label.toLowerCase())}</span>
                  <FieldInput field={f} value={ct ? ct[f.key] : f.optional ? null : (f.default ?? 0)} onchange={(v) => fijarCota(a, b, f, v)} />
                </label>
              {/each}
            </div>
          {/if}
          <div class="acc">
            {#if dirigido && a !== b && !existe(b, a)}
              <button class="btn" onclick={() => invertir(a, b)}>{t('Invertir sentido')}</button>
            {/if}
            <button class="btn peligro" onclick={() => quitarArco(a, b)}>{t('Eliminar arco')}</button>
          </div>
        {/if}
      </div>
    {/if}
  </div>

  {#if sol && verSolucion && sol.rutas.length}
    <div class="leyenda">
      {#each sol.rutas as r, k}
        <span><i style:background={colorRuta(k)}></i><b>{r.nombre}</b>{#if r.nota}<em>{r.nota}</em>{/if}</span>
      {/each}
    </div>
  {/if}
  <p class="ayuda">
    {t('Doble clic en un espacio vacío: nodo nuevo · Arrastra desde el punto de un nodo (o Mayús + arrastrar) hasta otro: arco nuevo · Clic: editar valores · Supr: eliminar · Con el teclado: Tab para elegir, Intro para editar, flechas para mover y C para conectar.')}
  </p>
</div>

<style>
  .nombre {
    width: 100%;
    height: 34px;
    border: 1px solid var(--rule);
    border-radius: var(--r-sm);
    background: var(--field);
    color: var(--ink);
    padding: 0 10px;
  }
  .nombre:focus {
    outline: none;
    border-color: var(--focus);
    box-shadow: 0 0 0 3px var(--solver-tint);
  }
  .editor-red {
    display: flex;
    flex-direction: column;
    gap: 8px;
    max-width: 1100px;
  }
  .barra {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
  }
  .seg {
    display: inline-flex;
    border: 1px solid var(--rule);
    border-radius: var(--r-sm);
    overflow: hidden;
    background: var(--sheet);
  }
  .seg button {
    border: 0;
    background: none;
    padding: 5px 12px;
    font-size: 12.5px;
    font-weight: 550;
    color: var(--ink-2);
  }
  .seg button + button {
    border-left: 1px solid var(--rule);
  }
  .seg button.on {
    background: var(--solver-tint);
    color: var(--solver);
  }
  .ord select {
    height: 30px;
    padding: 0 8px;
    font-size: 12.5px;
    color: var(--ink);
    border: 1px solid var(--rule);
    border-radius: var(--r-sm);
    background-color: var(--sheet);
  }
  .sr {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip: rect(0 0 0 0);
  }
  .esp {
    flex: 1;
  }
  .chk-sol {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: 12.5px;
    color: var(--ink-2);
  }
  .estado {
    margin: 0;
    font-size: 12.5px;
    color: var(--solver);
  }
  .estado.sol-t {
    color: var(--ink-2);
  }
  .estado.aviso {
    color: var(--binding);
  }
  .lienzo {
    position: relative;
  }
  svg {
    display: block;
    width: 100%;
    height: auto;
    background: var(--sheet);
    border: 1px solid var(--rule);
    border-radius: var(--r-lg);
    font-size: 12.5px;
    touch-action: none;
    user-select: none;
  }
  svg:focus-visible {
    outline: 2px solid var(--focus);
  }
  .fondo {
    cursor: crosshair;
  }
  .cuad {
    fill: none;
    stroke: var(--rule-soft);
    stroke-width: 1;
  }
  .col-t {
    fill: var(--ink-3);
    font-size: 11.5px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
  .arco .trazo {
    fill: none;
    stroke: color-mix(in oklab, var(--ink-3) 70%, transparent);
    stroke-width: 1.6;
  }
  .arco.on .trazo {
    stroke: var(--focus);
    stroke-width: 2.6;
  }
  .toque {
    fill: none;
    stroke: transparent;
    stroke-width: 14;
    cursor: pointer;
    outline: none;
  }
  .toque:focus-visible {
    stroke: color-mix(in oklab, var(--focus) 30%, transparent);
  }
  .pf-arco {
    fill: var(--ink-3);
  }
  .pf-sel {
    fill: var(--focus);
  }
  .pf-sol {
    fill: var(--binding);
  }
  .et rect {
    fill: var(--sheet);
    stroke: var(--rule);
  }
  .et text {
    fill: var(--ink-2);
    font-size: 11.5px;
    font-variant-numeric: tabular-nums;
    pointer-events: none;
  }
  .et {
    pointer-events: none;
  }
  .solucion {
    pointer-events: none;
  }
  .solucion.vieja {
    opacity: 0.45;
  }
  .trazo-sol {
    fill: none;
    stroke: var(--binding);
    stroke-width: 4;
    stroke-linecap: round;
    stroke-opacity: 0.85;
  }
  .et.sol rect {
    fill: var(--binding-tint);
    stroke: var(--binding);
  }
  .et.sol text {
    fill: var(--ink);
    font-weight: 700;
  }
  .et.vieja {
    opacity: 0.6;
  }
  .et-sol rect {
    fill: var(--binding-tint);
    stroke: var(--binding);
  }
  .et-sol text {
    fill: var(--ink);
    font-weight: 700;
    font-size: 11.5px;
  }
  .enlace {
    stroke: var(--solver);
    stroke-width: 2;
    stroke-dasharray: 6 5;
    pointer-events: none;
  }
  .nodo .cuerpo {
    cursor: grab;
    outline: none;
  }
  .conectando .nodo .cuerpo {
    cursor: copy;
  }
  .nodo.arrastrando .cuerpo {
    cursor: grabbing;
  }
  .nodo rect {
    fill: var(--sheet);
    stroke: color-mix(in oklab, var(--ink-3) 60%, transparent);
    stroke-width: 1.4;
  }
  .nodo.en-sol rect {
    fill: var(--solver-tint);
    stroke: var(--solver);
    stroke-width: 2;
  }
  .nodo.on rect,
  .nodo .cuerpo:focus-visible rect {
    stroke: var(--focus);
    stroke-width: 2.6;
  }
  .nodo.origen rect,
  .nodo.destino rect {
    stroke: var(--solver);
    stroke-width: 2.6;
    stroke-dasharray: 4 3;
  }
  .nodo .nom {
    fill: var(--ink);
    font-weight: 600;
    pointer-events: none;
  }
  .nodo .nota,
  .nodo .marca {
    paint-order: stroke;
    stroke: var(--sheet);
    stroke-width: 4px;
    stroke-linejoin: round;
    fill: var(--ink-2);
    font-size: 11px;
    pointer-events: none;
  }
  .nodo .marca {
    fill: var(--solver);
    font-weight: 700;
    text-transform: uppercase;
    font-size: 10px;
    letter-spacing: 0.04em;
  }
  .asa {
    fill: var(--solver);
    stroke: var(--sheet);
    stroke-width: 2;
    opacity: 0;
    cursor: crosshair;
    transition: opacity 0.12s;
  }
  .nodo:hover .asa,
  .nodo.on .asa,
  .conectando .asa {
    opacity: 1;
  }
  .pop {
    position: absolute;
    z-index: 5;
    width: 270px;
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 10px 12px 12px;
    background: var(--sheet);
    border: 1px solid var(--rule);
    border-radius: var(--r-md);
    box-shadow: var(--shadow-pop);
  }
  .ph {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 8px;
    font-size: 13px;
  }
  .ph strong {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .x {
    border: 0;
    background: none;
    color: var(--ink-3);
    font-size: 17px;
    width: 24px;
    height: 24px;
    border-radius: 4px;
  }
  .x:hover {
    background: var(--rule-soft);
    color: var(--ink);
  }
  .pc {
    display: flex;
    flex-direction: column;
    gap: 3px;
    font-size: 12px;
    color: var(--ink-2);
    font-weight: 600;
  }
  .pc :global(.inp),
  .pc :global(.sel) {
    width: 100%;
  }
  .dos {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 8px;
  }
  .acc {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin-top: 2px;
  }
  .acc .btn {
    font-size: 12px;
    padding: 3px 9px;
    height: auto;
    min-height: 26px;
  }
  .acc .btn[aria-pressed='true'] {
    background: var(--solver-tint);
    color: var(--solver);
    border-color: var(--solver);
  }
  .btn.peligro {
    color: var(--bad);
  }
  .btn.peligro:hover {
    background: var(--bad-tint);
  }
  .leyenda {
    display: flex;
    flex-wrap: wrap;
    gap: 6px 16px;
    font-size: 12.5px;
  }
  .leyenda span {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }
  .leyenda i {
    width: 14px;
    height: 4px;
    border-radius: 2px;
  }
  .leyenda em {
    color: var(--ink-3);
    font-style: normal;
  }
  .ayuda {
    margin: 0;
    font-size: 12px;
    color: var(--ink-3);
    max-width: 110ch;
  }
</style>
