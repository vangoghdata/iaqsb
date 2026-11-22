// Deshacer y rehacer: fotos del modelo (en JSON) después de cada cambio. Las
// ediciones seguidas (escribir en varias celdas en un momento) se agrupan.

export class Historial {
  private pila: string[] = [];
  private i = -1;
  private espera: ReturnType<typeof setTimeout> | undefined;
  private pendiente: (() => string) | null = null;
  /** Cambia cuando cambian las posibilidades de deshacer o rehacer (para los botones). */
  version = $state(0);

  constructor(private max = 120) {}

  /** Empieza de cero con el estado actual (al abrir o crear un problema). */
  reiniciar(foto: string) {
    clearTimeout(this.espera);
    this.pendiente = null;
    this.pila = [foto];
    this.i = 0;
    this.version++;
  }

  /** Registra el estado después de un cambio (agrupado: espera un momento). */
  registrar(foto: () => string, ms = 350) {
    this.pendiente = foto;
    clearTimeout(this.espera);
    this.espera = setTimeout(() => this.confirmar(), ms);
  }

  /** Guarda ya lo que esté pendiente. */
  confirmar() {
    clearTimeout(this.espera);
    const f = this.pendiente;
    this.pendiente = null;
    if (!f) return;
    const s = f();
    if (this.pila[this.i] === s) return;
    this.pila = this.pila.slice(0, this.i + 1);
    this.pila.push(s);
    if (this.pila.length > this.max) this.pila.shift();
    this.i = this.pila.length - 1;
    this.version++;
  }

  get puedeDeshacer(): boolean {
    void this.version;
    return this.i > 0 || this.pendiente !== null;
  }

  get puedeRehacer(): boolean {
    void this.version;
    return this.i < this.pila.length - 1;
  }

  deshacer(): string | null {
    this.confirmar();
    if (this.i <= 0) return null;
    this.i--;
    this.version++;
    return this.pila[this.i];
  }

  rehacer(): string | null {
    this.confirmar();
    if (this.i >= this.pila.length - 1) return null;
    this.i++;
    this.version++;
    return this.pila[this.i];
  }
}
