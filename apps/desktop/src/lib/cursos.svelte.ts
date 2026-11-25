// Cursos (.ncourse): instalar, quitar y listar. Los temas de los cursos aparecen como módulos
// (id c.<curso>.<tema>) y el núcleo los atiende igual que a los incluidos (ver
// crates/iaqsb-api/src/cursos y docs/CURSOS.md).

import { bytesToBase64, call } from './backend';
import { shell } from './shell.svelte';
import { ui } from './ui.svelte';
import { t } from './i18n.svelte';

export interface CursoInstalado {
  id: string;
  titulo: string;
  version: string;
  descripcion: string;
  autores?: { nombre: string; institucion?: string }[];
  licencia?: string;
  area?: string;
  carpeta: string;
  temas: { id: string; modulo: string; titulo: string; resumen: string; tipo: string; calcula: boolean }[];
}

class Cursos {
  lista = $state<CursoInstalado[]>([]);
  carpeta = $state('');
  instalando = $state(false);

  async cargar() {
    try {
      const r = await call<{ carpeta: string; cursos: CursoInstalado[] }>('list_courses');
      this.lista = r.cursos;
      this.carpeta = r.carpeta;
    } catch {
      /* sin núcleo: nada que listar */
    }
  }

  /** Instala un .ncourse (archivo elegido, arrastrado o con el que se abrió la app). */
  async instalar(origen: File | string) {
    this.instalando = true;
    try {
      const args = typeof origen === 'string' ? { path: origen } : { base64: bytesToBase64(new Uint8Array(await origen.arrayBuffer())) };
      const r = await call<{ id: string; titulo: string; temas: number; actualizado: boolean; avisos: string[] }>('course_install', args);
      await shell.init();
      await this.cargar();
      const c = this.lista.find((x) => x.id === r.id);
      if (c?.temas.length) shell.module = c.temas[0].modulo;
      ui.avisar(r.actualizado ? t('Curso actualizado: {0}', r.titulo) : t('Curso instalado: {0} ({1} temas)', r.titulo, r.temas), 'ok');
      if (r.avisos.length) ui.avisar(r.avisos.join(' · '), 'info');
    } catch (e) {
      ui.avisar(e instanceof Error ? e.message : String(e), 'error');
    } finally {
      this.instalando = false;
    }
  }

  async quitar(id: string) {
    try {
      await call('course_remove', { id });
      if (shell.module.startsWith(`c.${id}.`)) shell.module = 'lp';
      await shell.init();
      await this.cargar();
      ui.avisar(t('Curso quitado'), 'info');
    } catch (e) {
      ui.avisar(e instanceof Error ? e.message : String(e), 'error');
    }
  }

  /** Abre el selector de archivos para instalar un curso. */
  elegir() {
    const inp = document.createElement('input');
    inp.type = 'file';
    inp.accept = '.ncourse';
    inp.onchange = () => {
      const f = inp.files?.[0];
      if (f) void this.instalar(f);
    };
    inp.click();
  }
}

export const cursos = new Cursos();
