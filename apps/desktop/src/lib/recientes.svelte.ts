// Archivos abiertos o guardados hace poco (solo en la app de escritorio, que
// puede volver a abrirlos por su ruta).

import { inTauri } from './backend';

const CLAVE = 'iaqsb.recientes';
const MAX = 12;

export interface Reciente {
  path: string;
  module: string;
  /** Nombre del problema (si se conoce). */
  name: string;
  /** Milisegundos desde 1970. */
  ts: number;
}

function leer(): Reciente[] {
  try {
    const v = JSON.parse(localStorage.getItem(CLAVE) ?? '[]');
    return Array.isArray(v) ? v.filter((x) => x && typeof x.path === 'string') : [];
  } catch {
    return [];
  }
}

class Recientes {
  lista = $state<Reciente[]>(leer());

  private guardar() {
    try {
      localStorage.setItem(CLAVE, JSON.stringify(this.lista));
    } catch {
      /* sin almacenamiento */
    }
  }

  /** Anota un archivo (sube al principio si ya estaba). */
  anotar(path: string | null | undefined, module: string, name = '') {
    if (!path || !inTauri) return;
    const norm = path.replace(/\\/g, '/').toLowerCase();
    this.lista = [{ path, module, name, ts: Date.now() }, ...this.lista.filter((r) => r.path.replace(/\\/g, '/').toLowerCase() !== norm)].slice(0, MAX);
    this.guardar();
  }

  quitar(path: string) {
    this.lista = this.lista.filter((r) => r.path !== path);
    this.guardar();
  }

  limpiar() {
    this.lista = [];
    this.guardar();
  }
}

export const recientes = new Recientes();

/** Nombre del archivo sin la carpeta. */
export function nombreArchivo(path: string): string {
  return path.split(/[\\/]/).pop() ?? path;
}
