// Ajustes de la aplicación (se guardan en este equipo). La página de Ajustes
// (components/ajustes/AjustesPage.svelte) los edita; el resto de la app los lee.

import { proveedor, type Conexion } from './proveedores';
import { t } from './i18n.svelte';

const CLAVE = 'iaqsb.ajustes';

/** Versión de IAQSB (la del instalador). */
export const VERSION = '0.2.0';

export type Nivel = 'pregrado' | 'posgrado';
export type Separador = 'auto' | 'punto' | 'coma';

interface Datos {
  nivel: Nivel;
  decimales: number;
  separador: Separador;
  autoguardado: boolean;
  /** Proveedor del asistente de IA (ver proveedores.ts). */
  proveedor: string;
  /** Clave de la API de cada proveedor (solo en este equipo). */
  claves: Record<string, string>;
  /** Modelo elegido en cada proveedor. */
  modelos: Record<string, string>;
  /** Dirección propia de cada proveedor (si se cambió). */
  urls: Record<string, string>;
  /** Al resolver, mostrar el paso a paso cuando el método lo tiene. */
  pasos: boolean;
  /** Autor, curso e institución para la portada de los informes. */
  autor: string;
  curso: string;
  institucion: string;
}

const DEF: Datos = {
  nivel: 'pregrado',
  decimales: 4,
  separador: 'auto',
  autoguardado: true,
  proveedor: 'openrouter',
  claves: {},
  modelos: {},
  urls: {},
  pasos: true,
  autor: '',
  curso: '',
  institucion: ''
};

function leer(): Datos {
  try {
    const v = JSON.parse(localStorage.getItem(CLAVE) ?? '{}') ?? {};
    const d: Datos = { ...DEF, ...v, claves: { ...(v.claves ?? {}) }, modelos: { ...(v.modelos ?? {}) }, urls: { ...(v.urls ?? {}) } };
    // ajustes de la versión anterior (solo Claude)
    if (typeof v.claveClaude === 'string' && v.claveClaude.trim() && !d.claves.anthropic) {
      d.claves.anthropic = v.claveClaude.trim();
      if (!v.proveedor) d.proveedor = 'anthropic';
    }
    if (typeof v.modeloClaude === 'string' && v.modeloClaude && !d.modelos.anthropic) d.modelos.anthropic = v.modeloClaude;
    return d;
  } catch {
    return { ...DEF, claves: {}, modelos: {}, urls: {} };
  }
}

class Ajustes {
  private d = leer();
  nivel = $state<Nivel>(this.d.nivel);
  decimales = $state(this.d.decimales);
  separador = $state<Separador>(this.d.separador);
  autoguardado = $state(this.d.autoguardado);
  proveedor = $state(this.d.proveedor);
  claves = $state<Record<string, string>>(this.d.claves);
  modelos = $state<Record<string, string>>(this.d.modelos);
  urls = $state<Record<string, string>>(this.d.urls);
  pasos = $state(this.d.pasos);
  autor = $state(this.d.autor);
  curso = $state(this.d.curso);
  institucion = $state(this.d.institucion);
  /** Página de Ajustes abierta. */
  abierta = $state(false);
  /** Sección de la página que se muestra. */
  seccion = $state<string>('general');

  guardar() {
    const d: Datos = {
      nivel: this.nivel,
      decimales: this.decimales,
      separador: this.separador,
      autoguardado: this.autoguardado,
      proveedor: this.proveedor,
      claves: $state.snapshot(this.claves),
      modelos: $state.snapshot(this.modelos),
      urls: $state.snapshot(this.urls),
      pasos: this.pasos,
      autor: this.autor,
      curso: this.curso,
      institucion: this.institucion
    };
    try {
      localStorage.setItem(CLAVE, JSON.stringify(d));
    } catch {
      /* sin almacenamiento: vale para esta sesión */
    }
  }

  /** Proveedor, dirección, clave y modelo del asistente de IA. */
  get conexion(): Conexion {
    const p = proveedor(this.proveedor);
    return {
      p,
      url: this.urls[p.id]?.trim() || p.url,
      clave: this.claves[p.id] ?? '',
      modelo: (this.modelos[p.id] ?? p.modelo).trim()
    };
  }

  /** ¿El asistente tiene lo necesario para conversar (dirección, clave si hace falta y modelo)? */
  get iaLista(): boolean {
    const c = this.conexion;
    return !!c.url.trim() && !!c.modelo && (c.p.clave !== 'si' || !!c.clave.trim());
  }

  /** «OpenRouter · qwen/qwen3…» */
  get iaNombre(): string {
    const c = this.conexion;
    const n = t(c.p.nombre);
    return c.modelo ? `${n} · ${c.modelo}` : n;
  }

  abrir(seccion = 'general') {
    this.seccion = seccion;
    this.abierta = true;
  }

}

export const ajustes = new Ajustes();
