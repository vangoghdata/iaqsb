// Estado de la interfaz: tema, menú plegado, diálogo de apariencia y avisos.

import { guardarPreferencia, leerPersonal, leerPreferencia, seguirTema, type PreferenciaTema } from './tema';

const CLAVE_FIJO = 'iaqsb_menu_fijo';

function leerFijo(): boolean {
  try {
    return localStorage.getItem(CLAVE_FIJO) === '1';
  } catch {
    return false;
  }
}

export type Aviso = { id: number; texto: string; tipo: 'ok' | 'error' | 'info' };

class Ui {
  tema = $state<PreferenciaTema>('sistema');
  apariencia = $state(false);
  /** Barra lateral fija (si no, aparece al llevar el mouse al borde izquierdo). */
  fijado = $state(leerFijo());
  /** Barra lateral flotante visible. */
  lateral = $state(false);
  private tLateral: ReturnType<typeof setTimeout> | undefined;
  avisos = $state<Aviso[]>([]);
  private n = 0;

  iniciar(): () => void {
    this.tema = leerPreferencia();
    return seguirTema((p) => (this.tema = p));
  }

  elegirTema(p: PreferenciaTema) {
    // Sin un tema personalizado armado, se abre el editor (que pone uno listo).
    if (p === 'personalizado' && !leerPersonal()) {
      this.apariencia = true;
      return;
    }
    guardarPreferencia(p);
    this.tema = p;
  }

  mostrarLateral() {
    clearTimeout(this.tLateral);
    this.lateral = true;
  }

  ocultarLateral(ms = 260) {
    clearTimeout(this.tLateral);
    if (ms <= 0) this.lateral = false;
    else this.tLateral = setTimeout(() => (this.lateral = false), ms);
  }

  fijar(v = !this.fijado) {
    this.fijado = v;
    this.lateral = false;
    try {
      localStorage.setItem(CLAVE_FIJO, v ? '1' : '0');
    } catch {
      /* solo esta ventana */
    }
  }

  avisar(texto: string, tipo: Aviso['tipo'] = 'ok') {
    const id = ++this.n;
    this.avisos = [...this.avisos, { id, texto, tipo }].slice(-3);
    setTimeout(() => this.quitar(id), tipo === 'error' ? 6000 : 3200);
  }

  quitar(id: number) {
    this.avisos = this.avisos.filter((a) => a.id !== id);
  }
}

export const ui = new Ui();
