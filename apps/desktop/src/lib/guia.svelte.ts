// «Guíame»: la guía didáctica de cada módulo (crates/iaqsb-core/guias/<id>.json, operación
// `guide` del núcleo). La app la muestra en GuiaPanel.svelte; la IA la pide con
// get_module_guide.

import { call } from './backend';
import { i18n } from './i18n.svelte';

export interface Guia {
  modulo: string;
  titulo: string;
  resumen: string;
  que_es?: string[];
  analogia?: string;
  para_que_sirve?: { sector: string; ejemplo: string }[];
  historia?: { anio: string; hecho: string; citas?: { cita: string; verificada: boolean }[] }[];
  necesitas?: { dato: string; para_que: string }[];
  supuestos?: string[];
  pasos?: { titulo: string; texto: string }[];
  como_resuelve?: string[];
  interpretar?: string[];
  industria?: { nombre: string; tipo: string; uso: string }[];
  alternativas?: string[];
  errores_comunes?: string[];
  profesor?: { objetivos?: string[]; actividades?: string[]; preguntas?: string[] };
  glosario?: { termino: string; definicion: string }[];
  referencias_apa?: { id: string; cita: string; apa: string; verificada: boolean; enlace?: string | null }[];
  /** Lecciones de un tema de curso (Markdown). */
  lecciones?: { titulo: string; markdown: string }[];
  curso?: { id: string; titulo: string };
}

const CLAVE_VISTAS = 'iaqsb.guia.vistas';

function vistasGuardadas(): string[] {
  try {
    const v = JSON.parse(localStorage.getItem(CLAVE_VISTAS) ?? '[]');
    return Array.isArray(v) ? v : [];
  } catch {
    return [];
  }
}

class GuiaEstado {
  abierta = $state(false);
  modulo = $state('');
  datos = $state<Guia | null>(null);
  cargando = $state(false);
  error = $state<string | null>(null);
  /** Módulos cuya guía ya se abrió (para no insistir con el aviso). */
  vistas = $state<string[]>(vistasGuardadas());
  private cache = new Map<string, Guia>();

  async abrir(modulo: string) {
    this.abierta = true;
    this.modulo = modulo;
    this.error = null;
    if (!this.vistas.includes(modulo)) {
      this.vistas = [...this.vistas, modulo];
      try {
        localStorage.setItem(CLAVE_VISTAS, JSON.stringify(this.vistas));
      } catch {
        /* sin almacenamiento */
      }
    }
    const clave = `${i18n.lang}:${modulo}`;
    const c = this.cache.get(clave);
    if (c) {
      this.datos = c;
      return;
    }
    this.datos = null;
    this.cargando = true;
    try {
      const r = await call<{ guide: Guia }>('guide', { module: modulo });
      this.cache.set(clave, r.guide);
      if (this.modulo === modulo) this.datos = r.guide;
    } catch (e) {
      this.error = e instanceof Error ? e.message : String(e);
    } finally {
      this.cargando = false;
    }
  }

  cerrar() {
    this.abierta = false;
  }

  /** El idioma cambió: las guías se vuelven a pedir. */
  olvidar() {
    this.cache.clear();
  }
}

export const guia = new GuiaEstado();
