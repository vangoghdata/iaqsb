/**
 * Tema de IAQSB: claro, oscuro, el del sistema o uno personalizado (colores
 * del menú, de la barra de arriba, del acento y un fondo con color, dibujo o
 * foto). El mismo sistema del panel de IA Nativa.
 *
 * Se guarda en este equipo (localStorage). Se aplica con html[data-tema]
 * (+ data-personal y variables CSS en el personalizado); las reglas están en
 * app.css. index.html trae una copia mínima (SCRIPT_TEMA) que lo aplica antes
 * de pintar, para que no se vea un destello de otro tema al abrir.
 */

export type PreferenciaTema = 'claro' | 'oscuro' | 'sistema' | 'personalizado';
export type Tema = 'claro' | 'oscuro';

export const CLAVE_TEMA = 'iaqsb_tema';
export const CLAVE_PERSONAL = 'iaqsb_tema_personal';
export const CLAVE_IMAGEN = 'iaqsb_tema_imagen';
export const TEMA_POR_DEFECTO: PreferenciaTema = 'sistema';

/** Lo que se elige en el editor. Se guarda para poder volver a editarlo. */
export type DisenoTema = {
  base: Tema;
  menu: string;
  barra: string;
  acento: string;
  fondo: string;
  /** dibujo de public/temas ("" = sin dibujo; "propia" = foto del equipo) */
  dibujo: string;
  /** 0 a 0.9: cuánto se suaviza el dibujo para que se lea el texto */
  velo: number;
};

export type TemaPersonal = {
  id: string;
  diseno: DisenoTema;
  base: Tema;
  vars: Record<string, string>;
  acentoClaro: boolean;
  fondoPropio: boolean;
};

export const VELO_POR_DEFECTO = 0.8;

export const VARS_PERSONAL = [
  '--menu-bg', '--menu-fg', '--menu-fg-2', '--menu-fg-3', '--menu-hover', '--menu-on-bg',
  '--barra-bg', '--barra-fg', '--barra-fg-2',
  '--fondo-color', '--fondo-img', '--fondo-tam', '--fondo-rep', '--fondo-velo',
  '--solver', '--solver-tint', '--accent-btn', '--accent-btn-ink', '--accent-2', '--focus'
];

const IMAGEN_VALIDA = /^data:image\/(jpeg|png|webp);base64,[A-Za-z0-9+/=]+$/;

/** Corre antes de pintar (va en línea en index.html). Pequeño y sin dependencias. */
export const SCRIPT_TEMA = `(function(){var d=document.documentElement;try{var p=localStorage.getItem("${CLAVE_TEMA}")||"${TEMA_POR_DEFECTO}",t,c=null,V=${JSON.stringify(VARS_PERSONAL)};if(p==="personalizado"){try{c=JSON.parse(localStorage.getItem("${CLAVE_PERSONAL}")||"null")}catch(e){}if(!c||!c.vars)p="sistema"}if(c&&p==="personalizado"){t=c.base;for(var k=0;k<V.length;k++){var v=c.vars[V[k]];if(typeof v==="string")d.style.setProperty(V[k],v)}if(c.fondoPropio){var i=localStorage.getItem("${CLAVE_IMAGEN}");if(i)d.style.setProperty("--fondo-img",'url("'+i+'")')}d.setAttribute("data-personal","");if(c.acentoClaro)d.setAttribute("data-acento-claro","")}else{t=p==="sistema"?(matchMedia("(prefers-color-scheme: light)").matches?"claro":"oscuro"):(p==="claro"?"claro":"oscuro")}d.setAttribute("data-tema",t)}catch(e){d.setAttribute("data-tema","oscuro")}})();`;

// ------------------------------------------------------------- memoria

let enMemoria: PreferenciaTema | null = null;
let personalEnMemoria: TemaPersonal | null = null;
let imagenEnMemoria: string | null | undefined;

export function leerPreferencia(): PreferenciaTema {
  if (enMemoria) return enMemoria;
  try {
    const p = localStorage.getItem(CLAVE_TEMA);
    if (p === 'claro' || p === 'oscuro' || p === 'sistema' || p === 'personalizado') return p;
  } catch {
    /* sin almacenamiento */
  }
  return TEMA_POR_DEFECTO;
}

export function leerPersonal(): TemaPersonal | null {
  if (personalEnMemoria) return personalEnMemoria;
  try {
    const c = JSON.parse(localStorage.getItem(CLAVE_PERSONAL) || 'null') as TemaPersonal | null;
    if (c && c.vars && c.diseno) return c;
  } catch {
    /* sin almacenamiento */
  }
  return null;
}

export function leerImagenPropia(): string | null {
  if (imagenEnMemoria !== undefined) return imagenEnMemoria;
  try {
    const i = localStorage.getItem(CLAVE_IMAGEN);
    imagenEnMemoria = i && IMAGEN_VALIDA.test(i) ? i : null;
  } catch {
    imagenEnMemoria = null;
  }
  return imagenEnMemoria;
}

function temaDelSistema(): Tema {
  try {
    return window.matchMedia('(prefers-color-scheme: light)').matches ? 'claro' : 'oscuro';
  } catch {
    return 'oscuro';
  }
}

export function temaDe(p: PreferenciaTema): Tema {
  if (p === 'claro' || p === 'oscuro') return p;
  if (p === 'personalizado') return leerPersonal()?.base ?? temaDelSistema();
  return temaDelSistema();
}

function quitarPersonal() {
  const d = document.documentElement;
  for (const k of VARS_PERSONAL) d.style.removeProperty(k);
  d.removeAttribute('data-personal');
  d.removeAttribute('data-acento-claro');
}

function ponerPersonal(t: TemaPersonal) {
  const d = document.documentElement;
  const nuevas: Record<string, string> = {};
  for (const k of VARS_PERSONAL) if (typeof t.vars[k] === 'string') nuevas[k] = t.vars[k];
  if (t.fondoPropio) {
    const img = leerImagenPropia();
    if (img) nuevas['--fondo-img'] = `url("${img}")`;
  }
  for (const k of VARS_PERSONAL) {
    if (!(k in nuevas)) d.style.removeProperty(k);
    else if (d.style.getPropertyValue(k) !== nuevas[k]) d.style.setProperty(k, nuevas[k]);
  }
  d.setAttribute('data-personal', '');
  if (t.acentoClaro) d.setAttribute('data-acento-claro', '');
  else d.removeAttribute('data-acento-claro');
}

export function aplicarPreferencia(p: PreferenciaTema) {
  const d = document.documentElement;
  const personal = p === 'personalizado' ? leerPersonal() : null;
  let t: Tema;
  if (personal) {
    t = personal.base;
    ponerPersonal(personal);
  } else {
    quitarPersonal();
    t = temaDe(p === 'personalizado' ? 'sistema' : p);
  }
  if (d.getAttribute('data-tema') !== t) d.setAttribute('data-tema', t);
}

function avisar(p: PreferenciaTema) {
  window.dispatchEvent(new CustomEvent('iaqsb-tema', { detail: p }));
}

export function guardarPreferencia(p: PreferenciaTema) {
  enMemoria = p;
  try {
    localStorage.setItem(CLAVE_TEMA, p);
  } catch {
    /* vale para esta ventana */
  }
  aplicarPreferencia(p);
  avisar(p);
}

/** Pone un tema personalizado sin guardarlo (mientras se arrastra un color). */
export function probarPersonal(t: TemaPersonal) {
  personalEnMemoria = t;
  enMemoria = 'personalizado';
  aplicarPreferencia('personalizado');
}

export function guardarPersonal(t: TemaPersonal) {
  personalEnMemoria = t;
  try {
    localStorage.setItem(CLAVE_PERSONAL, JSON.stringify(t));
  } catch {
    /* vale para esta ventana */
  }
  guardarPreferencia('personalizado');
}

/** Guarda la foto de fondo propia. false si no cabe (unos 5 MB por sitio). */
export function guardarImagenPropia(dataUrl: string | null): boolean {
  if (dataUrl && !IMAGEN_VALIDA.test(dataUrl)) return false;
  try {
    if (dataUrl) localStorage.setItem(CLAVE_IMAGEN, dataUrl);
    else localStorage.removeItem(CLAVE_IMAGEN);
    imagenEnMemoria = dataUrl;
    return true;
  } catch {
    return false;
  }
}

/** Mantiene el tema al día (modo noche automático del sistema). */
export function seguirTema(alCambiar?: (p: PreferenciaTema) => void): () => void {
  const sincronizar = () => {
    const p = leerPreferencia();
    aplicarPreferencia(p);
    alCambiar?.(p);
  };
  sincronizar();
  const mq = window.matchMedia('(prefers-color-scheme: light)');
  const enSistema = () => {
    const p = leerPreferencia();
    if (p === 'sistema' || (p === 'personalizado' && !leerPersonal())) sincronizar();
  };
  const enEsta = () => alCambiar?.(leerPreferencia());
  mq.addEventListener?.('change', enSistema);
  window.addEventListener('iaqsb-tema', enEsta);
  return () => {
    mq.removeEventListener?.('change', enSistema);
    window.removeEventListener('iaqsb-tema', enEsta);
  };
}

// ------------------------------------------------- colores: hex ↔ OKLCH

type RGB = [number, number, number];

function hexARgb(hex: string): RGB {
  let h = hex.replace('#', '').trim();
  if (h.length === 3) h = h.split('').map((c) => c + c).join('');
  const n = parseInt(h.slice(0, 6), 16);
  if (!Number.isFinite(n)) return [0, 0, 0];
  return [(n >> 16) & 255, (n >> 8) & 255, n & 255];
}
const aHex = ([r, g, b]: RGB) => '#' + [r, g, b].map((v) => Math.round(Math.min(255, Math.max(0, v))).toString(16).padStart(2, '0')).join('');
const aLineal = (c: number) => {
  const v = c / 255;
  return v <= 0.04045 ? v / 12.92 : Math.pow((v + 0.055) / 1.055, 2.4);
};
const aSrgb = (v: number) => 255 * (v <= 0.0031308 ? 12.92 * v : 1.055 * Math.pow(v, 1 / 2.4) - 0.055);

function rgbAOklch(rgb: RGB): [number, number, number] {
  const [r, g, b] = rgb.map(aLineal);
  const l = Math.cbrt(0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b);
  const m = Math.cbrt(0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b);
  const s = Math.cbrt(0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b);
  const L = 0.2104542553 * l + 0.793617785 * m - 0.0040720468 * s;
  const A = 1.9779984951 * l - 2.428592205 * m + 0.4505937099 * s;
  const B = 0.0259040371 * l + 0.7827717662 * m - 0.808675766 * s;
  return [L, Math.sqrt(A * A + B * B), ((Math.atan2(B, A) * 180) / Math.PI + 360) % 360];
}

function oklchARgb(L: number, C: number, H: number): { rgb: RGB; dentro: boolean } {
  const a = C * Math.cos((H * Math.PI) / 180);
  const b = C * Math.sin((H * Math.PI) / 180);
  const l = Math.pow(L + 0.3963377774 * a + 0.2158037573 * b, 3);
  const m = Math.pow(L - 0.1055613458 * a - 0.0638541728 * b, 3);
  const s = Math.pow(L - 0.0894841775 * a - 1.291485548 * b, 3);
  const lin = [4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s, -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s, -0.0041960863 * l - 0.7034186147 * m + 1.707614701 * s];
  const dentro = lin.every((v) => v >= -0.0005 && v <= 1.0005);
  return { rgb: lin.map(aSrgb) as RGB, dentro };
}

function enGama(L: number, C: number, H: number): string {
  let c = C;
  for (let i = 0; i < 30; i++) {
    const r = oklchARgb(L, c, H);
    if (r.dentro) return aHex(r.rgb);
    c *= 0.9;
  }
  return aHex(oklchARgb(L, 0, H).rgb);
}

function luminancia(hex: string): number {
  const [r, g, b] = hexARgb(hex).map(aLineal);
  return 0.2126 * r + 0.7152 * g + 0.0722 * b;
}

export function contraste(a: string, b: string): number {
  const [x, y] = [luminancia(a), luminancia(b)].sort((p, q) => q - p);
  return (x + 0.05) / (y + 0.05);
}

/** Blanco o negro: lo que mejor se lee sobre ese fondo. */
export function textoSobre(fondo: string): string {
  return contraste(fondo, '#ffffff') >= contraste(fondo, '#000000') ? '#ffffff' : '#000000';
}

function conAlfa(hex: string, a: number): string {
  const [r, g, b] = hexARgb(hex);
  return `rgba(${r}, ${g}, ${b}, ${a})`;
}

function escala(hex: string): Record<number, string> {
  const [, C, H] = rgbAOklch(hexARgb(hex));
  const luz: Record<number, number> = { 200: 0.885, 300: 0.81, 400: 0.715, 500: 0.63, 600: 0.555, 700: 0.495, 800: 0.43 };
  const croma = Math.max(C, 0.02);
  const out: Record<number, string> = {};
  for (const [k, L] of Object.entries(luz)) out[Number(k)] = enGama(L, croma * (Number(k) <= 300 ? 0.55 : 1), H);
  return out;
}

/** Oscurece (o aclara) un tono hasta que se lea sobre otro, sin cambiar su matiz. */
function hastaContraste(hex: string, contra: string, minimo: number, haciaOscuro: boolean): string {
  let [L, C, H] = rgbAOklch(hexARgb(hex));
  let actual = hex;
  for (let i = 0; i < 60 && contraste(actual, contra) < minimo; i++) {
    L = Math.min(0.98, Math.max(0.05, L + (haciaOscuro ? -0.01 : 0.01)));
    actual = enGama(L, C, H);
  }
  return actual;
}

// ------------------------------------------------- dibujos de fondo

export type Dibujo = { id: string; nombre: string; archivo: string; mosaico?: number };

export const DIBUJOS: Dibujo[] = [
  { id: 'cuadricula', nombre: 'Cuadrícula', archivo: '/temas/cuadricula-v1.svg', mosaico: 120 },
  { id: 'puntos', nombre: 'Puntos', archivo: '/temas/puntos-v1.svg', mosaico: 28 },
  { id: 'mar', nombre: 'Mar', archivo: '/temas/mar-v1.svg', mosaico: 240 },
  { id: 'noche', nombre: 'Noche', archivo: '/temas/noche-v1.svg', mosaico: 260 }
];

// ------------------------------------------------- armar el tema

export function armarTema(diseno: DisenoTema, id = 'propio'): TemaPersonal {
  const d = { ...diseno, velo: Math.min(0.9, Math.max(0, diseno.velo || 0)) };
  const claro = d.base === 'claro';
  const menuFg = textoSobre(d.menu);
  const barraFg = textoSobre(d.barra);
  const acento = escala(d.acento);
  // Botón: texto blanco si se puede; si el color es muy claro (amarillo), texto oscuro.
  const acentoClaro = contraste(acento[600], '#ffffff') < 3;
  const boton = acentoClaro ? hastaContraste(acento[600], '#1a2030', 4.6, false) : hastaContraste(acento[600], '#ffffff', 4.6, true);
  // Acento como texto y líneas sobre las tarjetas (blancas o casi negras).
  const texto = claro ? hastaContraste(acento[600], '#ffffff', 4.6, true) : hastaContraste(acento[400], '#111827', 4.6, false);
  const [L, C, H] = rgbAOklch(hexARgb(d.acento));
  const segundo = enGama(claro ? Math.min(L, 0.6) : Math.max(L, 0.72), C, (H + 28) % 360);

  const vars: Record<string, string> = {
    '--menu-bg': d.menu,
    '--menu-fg': menuFg,
    '--menu-fg-2': conAlfa(menuFg, 0.78),
    '--menu-fg-3': conAlfa(menuFg, 0.55),
    '--menu-hover': conAlfa(menuFg, 0.08),
    '--menu-on-bg': conAlfa(menuFg, 0.16),
    '--barra-bg': d.barra,
    '--barra-fg': barraFg,
    '--barra-fg-2': conAlfa(barraFg, 0.72),
    '--fondo-color': d.fondo,
    '--fondo-velo': conAlfa(claro ? '#f4f5f7' : '#030712', d.velo),
    '--solver': texto,
    '--solver-tint': conAlfa(acento[500], claro ? 0.1 : 0.16),
    '--accent-btn': boton,
    '--accent-btn-ink': acentoClaro ? '#1a2030' : '#ffffff',
    '--accent-2': segundo,
    '--focus': texto
  };
  const dibujo = DIBUJOS.find((x) => x.id === d.dibujo);
  if (d.dibujo === 'propia') {
    vars['--fondo-tam'] = 'cover';
    vars['--fondo-rep'] = 'no-repeat';
  } else if (dibujo) {
    vars['--fondo-img'] = `url("${dibujo.archivo}")`;
    vars['--fondo-tam'] = dibujo.mosaico ? `${dibujo.mosaico}px` : 'cover';
    vars['--fondo-rep'] = dibujo.mosaico ? 'repeat' : 'no-repeat';
  } else {
    vars['--fondo-img'] = 'none';
  }
  return { id, diseno: d, base: d.base, vars, acentoClaro, fondoPropio: d.dibujo === 'propia' };
}

// ------------------------------------------------- temas listos

export type TemaListo = { id: string; nombre: string; grupo: 'Colores' | 'Con fondo'; diseno: DisenoTema };

const claro = (menu: string, barra: string, acento: string, fondo: string, dibujo = ''): DisenoTema => ({
  base: 'claro',
  menu,
  barra,
  acento,
  fondo,
  dibujo,
  velo: dibujo ? VELO_POR_DEFECTO : 0
});
const oscuro = (menu: string, barra: string, acento: string, fondo: string, dibujo = ''): DisenoTema => ({
  base: 'oscuro',
  menu,
  barra,
  acento,
  fondo,
  dibujo,
  velo: dibujo ? VELO_POR_DEFECTO : 0
});

export const TEMAS_LISTOS: TemaListo[] = [
  { id: 'azul', nombre: 'Azul', grupo: 'Colores', diseno: claro('#1e40af', '#eff6ff', '#2563eb', '#dbeafe') },
  { id: 'verde', nombre: 'Verde', grupo: 'Colores', diseno: claro('#15803d', '#f0fdf4', '#16a34a', '#dcfce7') },
  { id: 'morado', nombre: 'Morado', grupo: 'Colores', diseno: claro('#6d28d9', '#f5f3ff', '#7c3aed', '#ede9fe') },
  { id: 'rosa', nombre: 'Rosa', grupo: 'Colores', diseno: claro('#be185d', '#fdf2f8', '#db2777', '#fce7f3') },
  { id: 'rojo', nombre: 'Rojo', grupo: 'Colores', diseno: claro('#b91c1c', '#fef2f2', '#dc2626', '#fee2e2') },
  { id: 'amarillo', nombre: 'Amarillo', grupo: 'Colores', diseno: claro('#facc15', '#fefce8', '#ca8a04', '#fef9c3') },
  { id: 'grafito', nombre: 'Grafito', grupo: 'Colores', diseno: oscuro('#0b0f19', '#111827', '#14b8a6', '#030712') },
  { id: 'cuadricula', nombre: 'Cuaderno', grupo: 'Con fondo', diseno: claro('#1e293b', '#ffffff', '#4f46e5', '#f8fafc', 'cuadricula') },
  { id: 'puntos', nombre: 'Pizarra', grupo: 'Con fondo', diseno: oscuro('#0f172a', '#111827', '#818cf8', '#020617', 'puntos') },
  { id: 'mar', nombre: 'Mar', grupo: 'Con fondo', diseno: claro('#0e7490', '#ecfeff', '#0891b2', '#cffafe', 'mar') },
  { id: 'noche', nombre: 'Noche', grupo: 'Con fondo', diseno: oscuro('#0b1026', '#0f1530', '#8b5cf6', '#070b1f', 'noche') }
];
