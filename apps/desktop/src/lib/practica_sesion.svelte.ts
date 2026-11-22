// Estado del modo práctica (símplex y branch and bound). Vive fuera de los
// componentes para que el avance no se pierda al cambiar de pestaña. Las
// comprobaciones están en practica.ts (funciones puras).

import { call } from './backend';
import * as P from './practica';
import { msg as t } from './practica';
import type { LpModel, LpReport } from './types';

export type Modo = 'simplex' | 'bnb';

/** Una línea de comentario para el estudiante. */
export interface Nota {
  level: P.Level;
  msgs: P.Msg[];
}

/** Contadores de la sesión. */
export interface Marcador {
  /** Preguntas resueltas (decisiones). */
  decisiones: number;
  /** Respuestas enviadas. */
  intentos: number;
  aciertos: number;
  /** Preguntas acertadas al primer intento, sin pistas ni respuesta. */
  primera: number;
  pistas: number;
  respuestas: number;
}

const marcador0 = (): Marcador => ({ decisiones: 0, intentos: 0, aciertos: 0, primera: 0, pistas: 0, respuestas: 0 });

/** Huella del modelo, para saber si cambió desde que empezó la práctica. */
export function huella(model: LpModel): string {
  return JSON.stringify(model);
}

function errorText(e: unknown): string {
  return e instanceof Error ? e.message : String(e);
}

/** Lleva la cuenta de una pregunta (intentos, pistas) y la cierra en el marcador. */
class Pregunta {
  intentos = 0;
  pistas = 0;
  mostrada = false;
  reset() {
    this.intentos = 0;
    this.pistas = 0;
    this.mostrada = false;
  }
}

// ------------------------------------------------------------ símplex

export type FaseSimplex = 'enter' | 'leave' | 'row' | 'confirm' | 'done';

export interface PasoHecho {
  tb: P.PTableau;
  e: number;
  r: number;
  dantzig: boolean;
}

export class PracticaSimplex {
  key = $state<string | null>(null);
  model = $state.raw<LpModel | null>(null);
  loading = $state(false);
  error = $state<string | null>(null);
  /** Respuesta del núcleo (relajación lineal) para comparar al final. */
  ref = $state.raw<LpReport | null>(null);
  inicial = $state.raw<P.PTableau | null>(null);
  tb = $state.raw<P.PTableau | null>(null);
  fase = $state<FaseSimplex>('enter');
  entra = $state<number | null>(null);
  sale = $state<number | null>(null);
  notas = $state.raw<Nota[]>([]);
  pistas = $state.raw<P.Msg[]>([]);
  malCols = $state.raw<number[]>([]);
  malFilas = $state.raw<number[]>([]);
  /** Columnas descartadas en esta tabla (entran pero no llevan a ninguna conclusión). */
  descartadas = $state.raw<number[]>([]);
  verCocientes = $state(false);
  fila = $state<string[]>([]);
  filaOk = $state.raw<(boolean | null)[]>([]);
  historial = $state.raw<PasoHecho[]>([]);
  fin = $state.raw<{ kind: P.Conclusion; tb: P.PTableau } | null>(null);
  marcador = $state<Marcador>(marcador0());
  /** Mostrar fracciones (exactas) o decimales. */
  fracciones = $state(true);
  /** Pedir la nueva fila pivote antes de pivotar. */
  practicarFila = $state(false);
  private q = new Pregunta();

  async start(model: LpModel) {
    this.key = huella(model);
    this.model = model;
    this.loading = true;
    this.error = null;
    this.tb = null;
    try {
      const rep = await call<LpReport>('solve', { module: 'lp', model, options: { trace: true, relax_integers: true, diagnose: false } });
      const s0 = rep.simplex_trace?.[0];
      if (!s0) throw new Error('sin tablas');
      this.ref = rep;
      this.inicial = P.fromSnapshot(s0, model.objective);
      this.restart();
    } catch (e) {
      const msg = errorText(e);
      this.error = msg === 'sin tablas' ? 'sin tablas' : msg;
    } finally {
      this.loading = false;
    }
  }

  restart() {
    if (!this.inicial) return;
    this.tb = this.inicial;
    this.historial = [];
    this.fin = null;
    this.marcador = marcador0();
    this.nuevaTabla();
  }

  private nuevaTabla() {
    this.fase = 'enter';
    this.entra = null;
    this.sale = null;
    this.notas = [];
    this.pistas = [];
    this.malCols = [];
    this.malFilas = [];
    this.descartadas = [];
    this.verCocientes = false;
    this.fila = [];
    this.filaOk = [];
    this.q.reset();
  }

  /** Cambia de pregunta dentro de la misma tabla (conserva las notas). */
  private siguientePregunta() {
    this.pistas = [];
    this.q.reset();
  }

  private intento(v: P.Verdict) {
    this.q.intentos++;
    this.marcador.intentos++;
    if (v.ok) this.marcador.aciertos++;
  }

  private cerrar() {
    this.marcador.decisiones++;
    if (this.q.intentos === 1 && !this.q.pistas && !this.q.mostrada) this.marcador.primera++;
    // al cerrar la pregunta quedan solo las respuestas aceptadas (los errores ya se vieron)
    this.notas = this.notas.filter((n) => n.level !== 'bad');
    this.malCols = [];
    this.malFilas = [];
  }

  /** El estudiante hace clic en una columna (variable que entra). */
  elegirColumna(j: number) {
    const tb = this.tb;
    if (!tb || this.fase !== 'enter') return;
    const v = P.checkEntering(tb, j);
    this.intento(v);
    if (v.ok && this.descartadas.includes(j)) {
      this.notas = [...this.notas, { level: 'bad', msgs: [t('Esa columna ya se descartó en esta tabla: elige otra.')] }];
      return;
    }
    this.notas = [...this.notas, v];
    if (!v.ok) {
      this.malCols = [...this.malCols, j];
      return;
    }
    this.cerrar();
    this.entra = j;
    this.fase = 'leave';
    this.malFilas = [];
    this.siguientePregunta();
  }

  /** Vuelve a elegir la variable que entra (sin penalización). */
  cambiarColumna() {
    if (this.fase !== 'leave') return;
    this.entra = null;
    this.fase = 'enter';
    this.malFilas = [];
    this.siguientePregunta();
  }

  /** El estudiante hace clic en una fila (variable que sale). */
  elegirFila(r: number) {
    const tb = this.tb;
    if (!tb || this.fase !== 'leave' || this.entra === null) return;
    const v = P.checkLeaving(tb, this.entra, r);
    this.intento(v);
    this.notas = [...this.notas, v];
    if (!v.ok) {
      this.malFilas = [...this.malFilas, r];
      return;
    }
    this.cerrar();
    this.sale = r;
    this.verCocientes = true;
    this.trasSalida();
  }

  private trasSalida() {
    const tb = this.tb!;
    this.siguientePregunta();
    if (this.practicarFila) {
      this.fase = 'row';
      this.fila = Array(tb.columns.length + 1).fill('');
      this.filaOk = [];
    } else this.fase = 'confirm';
  }

  /** Comprueba la nueva fila pivote escrita. */
  comprobarFila() {
    const tb = this.tb;
    if (!tb || this.fase !== 'row' || this.entra === null || this.sale === null) return;
    const res = P.checkPivotRow(tb, this.sale, this.entra, this.fila);
    this.filaOk = res.cells;
    const v: P.Verdict = res.ok
      ? { ok: true, level: 'ok', msgs: [t('Correcto: la nueva fila pivote está bien calculada.')] }
      : res.cells.some((c) => c === null)
        ? { ok: false, level: 'bad', msgs: [t('Hay casillas vacías o que no se entienden. Escribe números como 3, −1/2 o 0,75.')] }
        : res.cells.filter((c) => c === false).length === 1
          ? { ok: false, level: 'bad', msgs: [t('Una casilla no coincide. Revisa la marcada en rojo.')] }
          : { ok: false, level: 'bad', msgs: [t('{0} casillas no coinciden. Revisa las marcadas en rojo.', res.cells.filter((c) => c === false).length)] };
    this.intento(v);
    // solo el último intento de la fila pivote
    this.notas = [...this.notas.filter((n) => n.level !== 'bad'), v];
    if (res.ok) {
      this.cerrar();
      this.fase = 'confirm';
      this.siguientePregunta();
    }
  }

  /** Salta el ejercicio de la fila pivote. */
  omitirFila() {
    if (this.fase !== 'row') return;
    this.fase = 'confirm';
    this.siguientePregunta();
  }

  /** Hace el pivoteo y pasa a la tabla siguiente. */
  pivotar() {
    const tb = this.tb;
    if (!tb || this.fase !== 'confirm' || this.entra === null || this.sale === null) return;
    const dantzig = P.dantzig(tb).includes(this.entra);
    this.historial = [...this.historial, { tb, e: this.entra, r: this.sale, dantzig }];
    this.tb = P.pivot(tb, this.sale, this.entra);
    this.nuevaTabla();
  }

  /** Declara la conclusión de la tabla. */
  declarar(kind: P.Conclusion) {
    const tb = this.tb;
    if (!tb || (this.fase !== 'enter' && this.fase !== 'leave')) return;
    if (this.fase === 'leave' && kind === 'unbounded' && this.entra !== null) {
      this.declararNoAcotado(tb, this.entra);
      return;
    }
    const v = P.checkDeclaration(tb, kind);
    this.intento(v);
    this.notas = [...this.notas, v];
    if (v.ok) this.terminar(kind);
  }

  /** «No acotado» con la columna elegida: ninguna fila tiene elemento positivo. */
  private declararNoAcotado(tb: P.PTableau, e: number) {
    if (!P.noPositive(tb, e)) {
      const v: P.Verdict = { ok: false, level: 'bad', msgs: [t('La columna {0} tiene elementos positivos: la prueba del cociente sí limita su crecimiento.', tb.columns[e])] };
      this.intento(v);
      this.notas = [...this.notas, v];
      return;
    }
    if (!P.positiveArtificials(tb).length) {
      const v = P.checkDeclaration(tb, 'unbounded');
      this.intento(v);
      this.notas = [...this.notas, v];
      this.terminar('unbounded');
      return;
    }
    if (P.provablyInfeasible(tb)) {
      // ninguna fila limita, pero la solución ni siquiera es factible
      const v: P.Verdict = {
        ok: false,
        level: 'warn',
        msgs: [t('Ninguna fila limita, pero la artificial {0} sigue positiva y la parte M ya no mejora: el problema no es factible.', tb.columns[tb.basis[P.positiveArtificials(tb)[0]]])]
      };
      this.intento(v);
      this.notas = [...this.notas, v];
      this.terminar('infeasible');
      return;
    }
    // caso raro: la columna no limita pero aún quedan artificiales que pueden salir
    this.notas = [...this.notas, { level: 'info', msgs: [t('Ninguna fila limita a {0}, pero aún hay columnas que reducen la parte M. Elige una de ellas primero.', tb.columns[e])] }];
    this.descartadas = [...this.descartadas, e];
    this.entra = null;
    this.fase = 'enter';
    this.siguientePregunta();
  }

  private terminar(kind: P.Conclusion) {
    this.cerrar();
    this.fin = { kind, tb: this.tb! };
    this.fase = 'done';
    this.pistas = [];
  }

  /** Muestra la siguiente pista de la pregunta actual. */
  pista() {
    const tb = this.tb;
    if (!tb) return;
    const lvl = this.pistas.length;
    let m: P.Msg | null = null;
    if (this.fase === 'enter') m = lvl < 2 ? P.hintEntering(tb, lvl) : null;
    else if (this.fase === 'leave' && this.entra !== null) m = lvl < 2 ? P.hintLeaving(tb, this.entra, lvl) : null;
    else if (this.fase === 'row' && this.entra !== null && this.sale !== null) m = lvl < 2 ? P.hintPivotRow(tb, this.sale, this.entra, lvl) : null;
    if (!m) return;
    this.pistas = [...this.pistas, m];
    this.q.pistas++;
    this.marcador.pistas++;
  }

  /** ¿Queda otra pista para esta pregunta? */
  get hayPista(): boolean {
    return ['enter', 'leave', 'row'].includes(this.fase) && this.pistas.length < 2;
  }

  /** Muestra (y aplica) la respuesta de la pregunta actual. */
  mostrarRespuesta() {
    const tb = this.tb;
    if (!tb) return;
    this.q.mostrada = true;
    this.marcador.respuestas++;
    if (this.fase === 'enter') {
      const end = P.expectedConclusion(tb);
      if (end) {
        const v = P.checkDeclaration(tb, end);
        this.notas = [...this.notas, { level: 'info', msgs: [t('Respuesta:'), ...v.msgs] }];
        this.terminar(end);
        return;
      }
      const cand = P.dantzig(tb).filter((j) => !this.descartadas.includes(j));
      const j = cand[0] ?? P.improving(tb).find((x) => !this.descartadas.includes(x));
      if (j === undefined) return;
      this.notas = [...this.notas, { level: 'info', msgs: [t('Respuesta:'), ...P.checkEntering(tb, j).msgs] }];
      this.cerrar();
      this.entra = j;
      this.fase = 'leave';
      this.siguientePregunta();
    } else if (this.fase === 'leave' && this.entra !== null) {
      const rows = P.minRatioRows(tb, this.entra);
      if (!rows.length) {
        // no hay fila: la respuesta es la conclusión
        this.declararNoAcotado(tb, this.entra);
        return;
      }
      this.notas = [...this.notas, { level: 'info', msgs: [t('Respuesta:'), ...P.checkLeaving(tb, this.entra, rows[0]).msgs] }];
      this.cerrar();
      this.sale = rows[0];
      this.verCocientes = true;
      this.trasSalida();
    } else if (this.fase === 'row' && this.entra !== null && this.sale !== null) {
      const ex = P.newPivotRow(tb, this.sale, this.entra);
      this.fila = ex.map((x) => P.fmtFrac(x, true));
      this.filaOk = ex.map(() => true);
      this.cerrar();
      this.fase = 'confirm';
      this.siguientePregunta();
    }
  }
}

// ------------------------------------------------------------ branch and bound

export type FaseBnb = 'decide' | 'var' | 'dir' | 'solving' | 'done';

export class PracticaBnb {
  key = $state<string | null>(null);
  model = $state.raw<LpModel | null>(null);
  tol = $state(0.01);
  loading = $state(false);
  error = $state<string | null>(null);
  ref = $state.raw<LpReport | null>(null);
  nodos = $state.raw<P.BNode[]>([]);
  /** Nodos por explorar (el último es el siguiente). */
  pila = $state.raw<number[]>([]);
  actual = $state<number | null>(null);
  fase = $state<FaseBnb>('decide');
  varRama = $state<number | null>(null);
  incumbente = $state.raw<{ z: number; values: number[]; node: number } | null>(null);
  notas = $state.raw<Nota[]>([]);
  pistas = $state.raw<P.Msg[]>([]);
  malVars = $state.raw<number[]>([]);
  marcador = $state<Marcador>(marcador0());
  /** La relajación de la raíz no está acotada. */
  raizNoAcotada = $state(false);
  private orden = 0;
  private q = new Pregunta();

  get nodo(): P.BNode | null {
    return this.nodos.find((n) => n.id === this.actual) ?? null;
  }

  private async resolver(bounds: [number, number][]): Promise<Pick<P.BNode, 'status' | 'z' | 'values'>> {
    const model = this.model!;
    if (bounds.some(([l, u]) => l > u + 1e-9)) return { status: 'infeasible', z: null, values: [] };
    const rep = await call<LpReport>('solve', { module: 'lp', model: P.modelWithBounds(model, bounds), options: { relax_integers: true, diagnose: false } });
    const status: P.NodeStatus = rep.status === 'optimal' ? 'optimal' : rep.status === 'infeasible' ? 'infeasible' : rep.status === 'unbounded' ? 'unbounded' : 'error';
    if (status === 'error') throw new Error(rep.message);
    return { status, z: status === 'optimal' ? rep.objective_value : null, values: rep.variables.map((v) => v.value) };
  }

  async start(model: LpModel, tol: number) {
    this.key = huella(model);
    this.model = model;
    this.tol = tol > 0 ? tol : 0.01;
    this.loading = true;
    this.error = null;
    this.nodos = [];
    try {
      const [ref, root] = await Promise.all([
        call<LpReport>('solve', { module: 'lp', model, options: { bnb_trace: true, integer_tolerance: this.tol, diagnose: false } }),
        this.resolver(model.variables.map(P.effBounds))
      ]);
      this.ref = ref;
      this.raiz = root;
      this.restart();
    } catch (e) {
      this.error = errorText(e);
    } finally {
      this.loading = false;
    }
  }

  private raiz: Pick<P.BNode, 'status' | 'z' | 'values'> | null = null;

  restart() {
    const model = this.model;
    if (!model || !this.raiz) return;
    this.orden = 1;
    this.nodos = [{ id: 1, parent: null, depth: 0, branch: null, bounds: model.variables.map(P.effBounds), ...this.raiz, order: 1 }];
    this.pila = [];
    this.actual = 1;
    this.incumbente = null;
    this.marcador = marcador0();
    this.notasPrevias = [];
    this.raizNoAcotada = this.raiz.status === 'unbounded';
    this.nuevoNodo();
    if (this.raizNoAcotada) this.fase = 'done';
  }

  private nuevoNodo() {
    this.fase = 'decide';
    this.varRama = null;
    this.notas = [];
    this.pistas = [];
    this.malVars = [];
    this.q.reset();
  }

  private intento(v: P.Verdict) {
    this.q.intentos++;
    this.marcador.intentos++;
    if (v.ok) this.marcador.aciertos++;
  }

  private cerrar() {
    this.marcador.decisiones++;
    if (this.q.intentos === 1 && !this.q.pistas && !this.q.mostrada) this.marcador.primera++;
    this.pistas = [];
    this.q.reset();
    this.notas = this.notas.filter((n) => n.level !== 'bad');
    this.malVars = [];
  }

  private marcar(id: number, patch: Partial<P.BNode>) {
    this.nodos = this.nodos.map((n) => (n.id === id ? { ...n, ...patch } : n));
  }

  /** Pasa al siguiente nodo de la pila (búsqueda en profundidad). */
  private siguiente() {
    const pila = [...this.pila];
    const id = pila.pop();
    this.pila = pila;
    if (id === undefined) {
      this.actual = null;
      this.fase = 'done';
      this.pistas = [];
      return;
    }
    this.marcar(id, { order: ++this.orden });
    this.actual = id;
    this.nuevoNodo();
  }

  /** El estudiante decide qué hacer con el nodo actual. */
  decidir(d: P.Decision) {
    const n = this.nodo;
    const model = this.model;
    if (!n || !model || this.fase !== 'decide') return;
    const v = P.checkDecision(model, n, this.incumbente?.z ?? null, this.tol, d);
    this.intento(v);
    this.notas = [...this.notas, v];
    if (v.ok) this.aplicar(n, d);
  }

  private aplicar(n: P.BNode, d: P.Decision) {
    this.cerrar();
    if (d === 'branch') {
      this.fase = 'var';
      return;
    }
    if (d === 'incumbent' && n.z !== null) this.incumbente = { z: n.z, values: n.values, node: n.id };
    this.marcar(n.id, { decision: d });
    this.notasPrevias = this.notas;
    this.siguiente();
  }

  /** Comentarios del nodo anterior (se muestran un momento tras pasar al siguiente). */
  notasPrevias = $state.raw<Nota[]>([]);

  /** Elige la variable sobre la que se ramifica. */
  elegirVariable(j: number) {
    const n = this.nodo;
    const model = this.model;
    if (!n || !model || this.fase !== 'var') return;
    const v = P.checkBranchVar(model, n, j, this.tol);
    this.intento(v);
    this.notas = [...this.notas, v];
    if (!v.ok) {
      this.malVars = [...this.malVars, j];
      return;
    }
    this.cerrar();
    this.varRama = j;
    this.fase = 'dir';
  }

  /** Elige la rama que se explora primero; crea y resuelve los dos hijos. */
  async elegirRama(first: 'down' | 'up') {
    const n = this.nodo;
    const model = this.model;
    const j = this.varRama;
    if (!n || !model || j === null || this.fase !== 'dir') return;
    this.fase = 'solving';
    const { down, up, fl, ce } = P.childBounds(n.bounds, j, n.values[j]);
    const name = model.variables[j].name;
    try {
      const [a, b] = await Promise.all([this.resolver(down), this.resolver(up)]);
      const base = Math.max(...this.nodos.map((x) => x.id));
      const hd: P.BNode = { id: base + 1, parent: n.id, depth: n.depth + 1, branch: `${name} ≤ ${fl}`, bounds: down, ...a };
      const hu: P.BNode = { id: base + 2, parent: n.id, depth: n.depth + 1, branch: `${name} ≥ ${ce}`, bounds: up, ...b };
      this.nodos = [...this.nodos.map((x) => (x.id === n.id ? { ...x, decision: 'branch' as const, branchVar: j } : x)), hd, hu];
      // el que se explora primero queda arriba de la pila
      this.pila = first === 'down' ? [...this.pila, hu.id, hd.id] : [...this.pila, hd.id, hu.id];
      this.notasPrevias = [
        ...this.notas,
        {
          level: 'info',
          msgs: [t('Se crean los nodos {0} ({1}) y {2} ({3}); se explora primero {4}.', hd.id, hd.branch!, hu.id, hu.branch!, first === 'down' ? hd.branch! : hu.branch!)]
        }
      ];
      this.siguiente();
    } catch (e) {
      this.error = errorText(e);
      this.fase = 'dir';
    }
  }

  pista() {
    const n = this.nodo;
    const model = this.model;
    if (!n || !model) return;
    const lvl = this.pistas.length;
    if (lvl >= 2) return;
    let m: P.Msg | null = null;
    if (this.fase === 'decide') {
      m =
        lvl === 0
          ? t('Primero mira si el subproblema es factible; después compara su z con la mejor solución entera; por último revisa si las variables enteras ya son enteras.')
          : P.correctDecision(model, n, this.incumbente?.z ?? null, this.tol).why;
    } else if (this.fase === 'var') {
      const fr = P.fractionalVars(model, n.values, this.tol);
      m = lvl === 0 ? t('Se ramifica sobre una variable entera (o binaria) cuyo valor no es entero.') : t('Variables enteras fraccionarias: {0}.', fr.map((j) => `${model.variables[j].name} = ${P.fmtNum(n.values[j])}`).join(', '));
    } else if (this.fase === 'dir') {
      m = t('Cualquiera de las dos ramas sirve: al final se exploran (o se podan) ambas. WinQSB explora primero la rama «≤».');
    }
    if (!m) return;
    this.pistas = [...this.pistas, m];
    if (this.fase !== 'dir') {
      this.q.pistas++;
      this.marcador.pistas++;
    }
  }

  get hayPista(): boolean {
    return (this.fase === 'decide' || this.fase === 'var') ? this.pistas.length < 2 : this.fase === 'dir' && this.pistas.length < 1;
  }

  mostrarRespuesta() {
    const n = this.nodo;
    const model = this.model;
    if (!n || !model) return;
    if (this.fase === 'decide') {
      const right = P.correctDecision(model, n, this.incumbente?.z ?? null, this.tol);
      this.q.mostrada = true;
      this.marcador.respuestas++;
      this.notas = [...this.notas, { level: 'info', msgs: [t('Respuesta:'), right.why] }];
      this.aplicar(n, right.decision);
    } else if (this.fase === 'var') {
      const j = P.fractionalVars(model, n.values, this.tol)[0];
      if (j === undefined) return;
      this.q.mostrada = true;
      this.marcador.respuestas++;
      this.notas = [...this.notas, { level: 'info', msgs: [t('Respuesta:'), ...P.checkBranchVar(model, n, j, this.tol).msgs] }];
      this.cerrar();
      this.varRama = j;
      this.fase = 'dir';
    } else if (this.fase === 'dir') {
      void this.elegirRama('down');
    }
  }
}

class Practica {
  modo = $state<Modo>('simplex');
  simplex = new PracticaSimplex();
  bnb = new PracticaBnb();
}

export const practica = new Practica();
