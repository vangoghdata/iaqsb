// Motor de optimización: cuál usar (HiGHS incluido; Gurobi, CPLEX, SCIP, CBC o
// GLPK si están instalados) y sus parámetros. Vale para todos los módulos.

import { call } from './backend';

export interface SolverInfo {
  id: string;
  name: string;
  via: string;
  problems: string;
  note: string;
}

export type Metodo = 'auto' | 'dual_simplex' | 'primal_simplex' | 'barrier' | 'pdlp';

const CLAVE = 'iaqsb.motor';

interface Guardado {
  solver: string;
  tiempo: number | null;
  brecha: number | null;
  hilos: number | null;
  metodo: Metodo;
  registro: boolean;
  extra?: Record<string, unknown>;
}

function leer(): Partial<Guardado> {
  try {
    return JSON.parse(localStorage.getItem(CLAVE) ?? '{}') ?? {};
  } catch {
    return {};
  }
}

class Motor {
  solvers = $state<SolverInfo[]>([]);
  cargando = $state(false);
  /** id del motor ('highs' = el incluido). */
  solver = $state('highs');
  /** Segundos como máximo (null = sin límite). */
  tiempo = $state<number | null>(null);
  /** Brecha MIP aceptada, en porcentaje (null = la del motor). */
  brecha = $state<number | null>(null);
  hilos = $state<number | null>(null);
  metodo = $state<Metodo>('auto');
  registro = $state(false);
  /** Otros parámetros (presolve, semilla, opciones propias del motor), p. ej. los que deja el ajuste automático. */
  extra = $state<Record<string, unknown>>({});
  /** Resumen del último ajuste automático aplicado. */
  ajuste = $state<string | null>(null);

  constructor() {
    const g = leer();
    if (typeof g.solver === 'string') this.solver = g.solver;
    if (typeof g.tiempo === 'number') this.tiempo = g.tiempo;
    if (typeof g.brecha === 'number') this.brecha = g.brecha;
    if (typeof g.hilos === 'number') this.hilos = g.hilos;
    if (g.metodo) this.metodo = g.metodo;
    if (g.registro) this.registro = true;
    if (g.extra && typeof g.extra === 'object') this.extra = g.extra;
  }

  /** Detecta los motores instalados (una vez). */
  async init() {
    if (this.solvers.length || this.cargando) return;
    this.cargando = true;
    try {
      const r = await call<{ solvers: SolverInfo[] }>('solver_info');
      this.solvers = r.solvers;
      if (!this.solvers.some((s) => s.id === this.solver)) this.solver = 'highs';
    } catch {
      this.solvers = [];
    } finally {
      this.cargando = false;
    }
  }

  guardar() {
    const g: Guardado = { solver: this.solver, tiempo: this.tiempo, brecha: this.brecha, hilos: this.hilos, metodo: this.metodo, registro: this.registro, extra: $state.snapshot(this.extra) };
    try {
      localStorage.setItem(CLAVE, JSON.stringify(g));
    } catch {
      /* sin almacenamiento: vale para esta sesión */
    }
  }

  restablecer() {
    this.solver = 'highs';
    this.tiempo = null;
    this.brecha = null;
    this.hilos = null;
    this.metodo = 'auto';
    this.registro = false;
    this.extra = {};
    this.ajuste = null;
    this.guardar();
  }

  /** ¿Se cambió algo respecto a lo predeterminado? */
  personalizado = $derived(
    this.solver !== 'highs' || this.tiempo !== null || this.brecha !== null || this.hilos !== null || this.metodo !== 'auto' || this.registro || Object.keys(this.extra).length > 0
  );

  nombre = $derived(this.solvers.find((s) => s.id === this.solver)?.name ?? (this.solver === 'highs' ? 'HiGHS' : this.solver));

  /** Parámetros para el núcleo (solo los que se cambiaron). */
  params(): Record<string, unknown> {
    const p: Record<string, unknown> = { ...$state.snapshot(this.extra) };
    if (this.solver !== 'highs') p.solver = this.solver;
    if (this.tiempo !== null && this.tiempo > 0) p.time_limit = this.tiempo;
    if (this.brecha !== null && this.brecha >= 0) p.mip_gap = this.brecha / 100;
    if (this.hilos !== null && this.hilos > 0) p.threads = Math.round(this.hilos);
    if (this.metodo !== 'auto') p.method = this.metodo;
    if (this.registro) p.log = true;
    return p;
  }

  /** Aplica los parámetros que encontró el ajuste automático (`tune`). */
  aplicar(best: Record<string, unknown>, resumen: string | null = null) {
    const b = { ...best };
    if (typeof b.method === 'string') this.metodo = b.method as Metodo;
    else this.metodo = 'auto';
    this.hilos = typeof b.threads === 'number' ? b.threads : null;
    for (const k of ['method', 'threads', 'solver', 'time_limit', 'mip_gap', 'log']) delete b[k];
    this.extra = b;
    this.ajuste = resumen;
    this.guardar();
  }
}

export const motor = new Motor();
