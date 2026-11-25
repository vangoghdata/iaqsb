// Asistente de creación de cursos: los borradores (cursos a medio hacer en la carpeta de datos),
// el archivo abierto en el editor, la vista previa (el borrador instalado como curso) y la
// exportación a .ncourse. El asistente de IA trabaja sobre el mismo borrador con sus herramientas
// (ver chat.svelte.ts, modo «creador»).

import { b64Blob, bytesToBase64, call, download, inTauri } from './backend';
import { shell } from './shell.svelte';
import { cursos } from './cursos.svelte';
import { ui } from './ui.svelte';
import { t } from './i18n.svelte';

export interface ResumenBorrador {
  id: string;
  titulo: string;
  curso: string;
  temas: number;
  modificado: number;
  instalado: boolean;
}

export interface TemaBorrador {
  id: string;
  titulo: string;
  resumen: string;
  tipo: string;
  lenguaje: string | null;
  ejemplos: number;
  lecciones: number;
}

export interface Borrador {
  id: string;
  carpeta: string;
  curso: any;
  temas: TemaBorrador[];
  archivos: { ruta: string; bytes: number }[];
  validacion: { ok: boolean; errores: string[]; avisos: string[] };
  instalado: boolean;
  instalado_como: string | null;
  modificado: number;
}

export interface Prueba {
  tema: string;
  modulo: string;
  ejemplo?: string;
  ok: boolean;
  headline?: string;
  error?: string;
  metricas?: { label: string; value: unknown }[];
}

/** Herramientas que el profesor permite usar en los temas (se las dice al asistente). */
export interface Herramientas {
  leccion: boolean;
  python: boolean;
  r: boolean;
  algebra: boolean;
}

const CLAVE_HERR = 'iaqsb.creador.herramientas';

function leerHerramientas(): Herramientas {
  const d: Herramientas = { leccion: true, python: true, r: true, algebra: true };
  try {
    return { ...d, ...JSON.parse(localStorage.getItem(CLAVE_HERR) ?? '{}') };
  } catch {
    return d;
  }
}

/** Extensiones de los archivos que se editan como texto. */
export const TEXTO = ['json', 'md', 'txt', 'py', 'r', 'iaqsb', 'csv', 'tsv', 'svg'];

export function extension(ruta: string): string {
  return ruta.split('.').pop()?.toLowerCase() ?? '';
}

class Creador {
  /** La página del creador está a la vista. */
  abierto = $state(false);
  borradores = $state<ResumenBorrador[]>([]);
  carpeta = $state('');
  /** El borrador abierto (con el que trabaja el asistente). */
  actual = $state<Borrador | null>(null);
  /** Archivo abierto en el editor. */
  archivo = $state<{ ruta: string; texto: string; original: string; binario?: boolean } | null>(null);
  pruebas = $state<Prueba[] | null>(null);
  ocupado = $state<string | null>(null);
  herramientas = $state<Herramientas>(leerHerramientas());
  /** Sube cada vez que el asistente cambia algo del borrador (para refrescar la vista). */
  version = $state(0);

  get sinGuardar(): boolean {
    return !!this.archivo && !this.archivo.binario && this.archivo.texto !== this.archivo.original;
  }

  fijarHerramienta(k: keyof Herramientas, v: boolean) {
    this.herramientas = { ...this.herramientas, [k]: v };
    try {
      localStorage.setItem(CLAVE_HERR, JSON.stringify(this.herramientas));
    } catch {
      /* solo esta sesión */
    }
  }

  private async conAviso<T>(que: string, f: () => Promise<T>): Promise<T | null> {
    this.ocupado = que;
    try {
      return await f();
    } catch (e) {
      ui.avisar(e instanceof Error ? e.message : String(e), 'error');
      return null;
    } finally {
      this.ocupado = null;
    }
  }

  /** Abre la página (y el asistente en modo creador si hay un borrador abierto). */
  async mostrar() {
    this.abierto = true;
    ui.lateral = false;
    await this.listar();
    if (this.actual) {
      const { chat } = await import('./chat.svelte');
      await chat.entrarCreador(this.actual.id);
    }
  }

  /** Vuelve a los módulos; el borrador sigue abierto para el asistente. */
  ocultar() {
    this.abierto = false;
  }

  /** Cierra el borrador: el asistente vuelve a ser el tutor. */
  async cerrar() {
    if (this.sinGuardar && !confirm(t('«{0}» tiene cambios sin guardar. ¿Cerrar de todos modos?', this.archivo!.ruta))) return;
    this.actual = null;
    this.archivo = null;
    this.pruebas = null;
    const { chat } = await import('./chat.svelte');
    await chat.salirCreador();
    await this.listar();
  }

  async listar() {
    try {
      const r = await call<{ carpeta: string; borradores: ResumenBorrador[] }>('draft_list');
      this.borradores = r.borradores;
      this.carpeta = r.carpeta;
    } catch {
      /* sin núcleo */
    }
  }

  /** desde: «vacio», «plantilla» o «curso:<id>» (modificar un curso instalado). */
  async nuevo(titulo: string, desde = 'vacio') {
    const r = await this.conAviso(t('Creando…'), () => call<{ id: string }>('draft_new', { title: titulo, from: desde }));
    if (r) await this.abrir(r.id);
  }

  /** Abre un .ncourse como borrador para modificarlo. */
  importar() {
    const inp = document.createElement('input');
    inp.type = 'file';
    inp.accept = '.ncourse';
    inp.onchange = async () => {
      const f = inp.files?.[0];
      if (!f) return;
      const r = await this.conAviso(t('Abriendo…'), async () => call<{ id: string }>('draft_import', { base64: bytesToBase64(new Uint8Array(await f.arrayBuffer())) }));
      if (r) await this.abrir(r.id);
    };
    inp.click();
  }

  async abrir(id: string) {
    if (this.actual?.id !== id && this.sinGuardar && !confirm(t('«{0}» tiene cambios sin guardar. ¿Cerrar de todos modos?', this.archivo!.ruta))) return;
    const r = await this.conAviso(t('Abriendo…'), () => call<Borrador>('draft_read', { id }));
    if (!r) return;
    if (this.actual?.id !== id) {
      this.archivo = null;
      this.pruebas = null;
    }
    this.actual = r;
    this.abierto = true;
    if (!this.archivo) await this.abrirArchivo('curso.json');
    const { chat } = await import('./chat.svelte');
    await chat.entrarCreador(id);
    await this.listar();
  }

  /** Vuelve a leer el borrador (después de que el asistente escribió algo). */
  async refrescar() {
    if (!this.actual) return;
    try {
      this.actual = await call<Borrador>('draft_read', { id: this.actual.id });
      this.version++;
      // el archivo abierto, si no tiene cambios del usuario, se recarga
      if (this.archivo && !this.sinGuardar) {
        if (this.actual.archivos.some((a) => a.ruta === this.archivo!.ruta)) await this.abrirArchivo(this.archivo.ruta, true);
        else this.archivo = null;
      }
    } catch {
      /* se borró el borrador */
    }
  }

  async abrirArchivo(ruta: string, silencioso = false) {
    if (!this.actual) return;
    if (!silencioso && this.archivo?.ruta !== ruta && this.sinGuardar && !confirm(t('«{0}» tiene cambios sin guardar. ¿Cerrar de todos modos?', this.archivo!.ruta))) return;
    try {
      const r = await call<{ ruta: string; texto?: string; base64?: string }>('draft_read', { id: this.actual.id, path: ruta });
      if (typeof r.texto === 'string') this.archivo = { ruta, texto: r.texto, original: r.texto };
      else this.archivo = { ruta, texto: r.base64 ?? '', original: r.base64 ?? '', binario: true };
    } catch (e) {
      if (!silencioso) ui.avisar(e instanceof Error ? e.message : String(e), 'error');
    }
  }

  async guardarArchivo() {
    const a = this.archivo;
    if (!this.actual || !a || a.binario) return;
    const ok = await this.conAviso(t('Guardando…'), () => call('draft_write', { id: this.actual!.id, path: a.ruta, content: a.texto }));
    if (ok) {
      a.original = a.texto;
      await this.refrescar();
    }
  }

  /** Agrega un tema con una plantilla que funciona (lección, Python, R o modelo algebraico). */
  async nuevoTema(titulo: string, tipo: 'leccion' | 'python' | 'r' | 'algebra') {
    if (!this.actual) return;
    if (this.sinGuardar) await this.guardarArchivo();
    const r = await this.conAviso(t('Creando…'), () => call<{ id: string; carpeta: string }>('draft_add_topic', { id: this.actual!.id, title: titulo, kind: tipo }));
    if (!r) return;
    await this.refrescar();
    await this.abrirArchivo(`${r.carpeta}/leccion.md`, true);
  }

  /** Agrega archivos (imágenes, datos, PDF) al tema elegido o a la raíz del curso. */
  async agregarArchivos(archivos: File[], carpeta: string) {
    if (!this.actual) return;
    for (const f of archivos) {
      const ruta = (carpeta ? carpeta.replace(/\/$/, '') + '/' : '') + f.name.replace(/[\\/:]/g, '_');
      const base64 = bytesToBase64(new Uint8Array(await f.arrayBuffer()));
      await this.conAviso(t('Guardando…'), () => call('draft_write', { id: this.actual!.id, path: ruta, base64 }));
    }
    await this.refrescar();
  }

  async borrarArchivo(ruta: string) {
    if (!this.actual) return;
    const carpeta = !/\.[a-z0-9]+$/i.test(ruta);
    if (!confirm(carpeta ? t('¿Borrar el tema «{0}» con todos sus archivos?', ruta) : t('¿Borrar «{0}» del borrador?', ruta))) return;
    const ok = await this.conAviso(t('Borrando…'), () => call('draft_delete', { id: this.actual!.id, path: ruta }));
    if (ok) {
      if (this.archivo && (this.archivo.ruta === ruta || this.archivo.ruta.startsWith(ruta + '/'))) this.archivo = null;
      await this.refrescar();
    }
  }

  /** Instala el borrador como curso para verlo en la app; con un tema, lo abre. */
  async vistaPrevia(tema?: string) {
    if (!this.actual) return;
    if (this.sinGuardar) await this.guardarArchivo();
    const r = await this.conAviso(t('Instalando…'), () => call<{ id: string; avisos: string[] }>('draft_install', { id: this.actual!.id }));
    if (!r) return;
    await this.trasInstalar();
    const c = cursos.lista.find((x) => x.id === r.id);
    const destino = (tema && c?.temas.find((x) => x.id === tema)) || c?.temas[0];
    if (destino) {
      shell.module = destino.modulo;
      this.abierto = false;
    }
    ui.avisar(t('Vista previa instalada: los temas están en la barra lateral'), 'ok');
  }

  async trasInstalar() {
    await shell.init();
    await cursos.cargar();
    await this.refrescar();
  }

  async probar(tema?: string) {
    if (!this.actual) return;
    if (this.sinGuardar) await this.guardarArchivo();
    const r = await this.conAviso(t('Probando los ejemplos…'), () => call<{ resultados: Prueba[] }>('draft_test', { id: this.actual!.id, ...(tema ? { topic: tema } : {}) }));
    if (!r) return;
    this.pruebas = r.resultados;
    await this.trasInstalar();
    const malos = r.resultados.filter((x) => !x.ok).length;
    ui.avisar(malos ? t('{0} de {1} ejemplos fallaron', malos, r.resultados.length) : t('Todos los ejemplos funcionan ({0})', r.resultados.length), malos ? 'error' : 'ok');
  }

  /** Guarda el curso como .ncourse para compartirlo. */
  async exportar() {
    const b = this.actual;
    if (!b) return;
    if (this.sinGuardar) await this.guardarArchivo();
    const nombre = `${b.curso?.id || b.id}.ncourse`;
    await this.conAviso(t('Exportando…'), async () => {
      if (inTauri) {
        const { save } = await import('@tauri-apps/plugin-dialog');
        const path = await save({ defaultPath: nombre, filters: [{ name: t('Curso de Nativo (.ncourse)'), extensions: ['ncourse'] }] });
        if (!path) return;
        const { invoke } = await import('@tauri-apps/api/core');
        await invoke('save_export', { path, op: 'draft_pack', args: { id: b.id } });
        ui.avisar(t('Curso exportado: {0}', path), 'ok');
      } else {
        const r = await call<{ base64: string }>('draft_pack', { id: b.id });
        download(b64Blob(r.base64, 'application/zip'), nombre);
      }
    });
  }

  async quitar(id: string) {
    const b = this.borradores.find((x) => x.id === id);
    if (!confirm(t('¿Borrar el borrador «{0}»? No se puede deshacer.', b?.titulo || id))) return;
    const quitarVista = !!b?.instalado && confirm(t('¿Quitar también la vista previa instalada de la barra lateral?'));
    const ok = await this.conAviso(t('Borrando…'), () => call('draft_remove', { id, with_preview: quitarVista }));
    if (!ok) return;
    if (this.actual?.id === id) await this.cerrar();
    if (quitarVista) await this.trasInstalar();
    await this.listar();
  }
}

export const creador = new Creador();
