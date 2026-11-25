// Idioma de la interfaz (español, inglés o portugués de Brasil).
//
// Los textos se escriben en español en el código y se pasan por t():
//   t('Resolver')                      → «Solve» en inglés
//   t('Gráfica guardada: {0}', nombre)  → los {0}, {1}… se reemplazan por los argumentos
// Las traducciones están en lib/i18n/*.ts (español → inglés y español → portugués). El núcleo en Rust
// traduce por su cuenta los informes, los ejemplos y las fuentes: cada llamada
// le manda el idioma (ver backend.ts).

import { marca } from './marca';
import { en } from './i18n/en';
import { pt } from './i18n/pt';

export type Lang = 'es' | 'en' | 'pt';
export const IDIOMAS: { id: Lang; nombre: string }[] = [
  { id: 'es', nombre: 'Español' },
  { id: 'en', nombre: 'English' },
  { id: 'pt', nombre: 'Português (Brasil)' }
];
const CLAVE = 'iaqsb_idioma';

function inicial(): Lang {
  try {
    const v = localStorage.getItem(CLAVE);
    if (v === 'es' || v === 'en' || v === 'pt') return v;
  } catch {
    /* sin almacenamiento */
  }
  const nav = typeof navigator !== 'undefined' ? navigator.language || 'es' : 'es';
  const l = nav.toLowerCase();
  return l.startsWith('es') ? 'es' : l.startsWith('pt') ? 'pt' : 'en';
}

class I18n {
  lang = $state<Lang>(inicial());

  constructor() {
    if (typeof document !== 'undefined') document.documentElement.lang = this.lang;
  }

  set(l: Lang) {
    this.lang = l;
    try {
      localStorage.setItem(CLAVE, l);
    } catch {
      /* sin almacenamiento */
    }
    if (typeof document !== 'undefined') document.documentElement.lang = l;
  }
}

export const i18n = new I18n();

/** Texto en el idioma activo. La clave es el texto en español. */
export function t(es: string, ...args: unknown[]): string {
  const l = i18n.lang;
  const s = marca(l === 'en' ? (en[es] ?? es) : l === 'pt' ? (pt[es] ?? es) : es);
  return args.length ? s.replace(/\{(\d+)\}/g, (_, k) => String(args[Number(k)] ?? '')) : s;
}

/** Formato de números y fechas según el idioma. */
export function locale(): string {
  return i18n.lang === 'en' ? 'en-US' : i18n.lang === 'pt' ? 'pt-BR' : 'es-CO';
}
