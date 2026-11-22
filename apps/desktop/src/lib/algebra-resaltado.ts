// Resaltado de sintaxis del lenguaje algebraico de IAQSB (sin dependencias).
//
// Sigue al lector del núcleo (crates/iaqsb-lp/src/algebra/parse.rs): palabras
// clave en español, inglés y portugués sin importar mayúsculas ni tildes,
// comentarios #, // y /* */, textos entre comillas, números y operadores. Los
// nombres declarados con set/param/var se pintan según su clase.

export type Clase = 'com' | 'str' | 'num' | 'kw' | 'fn' | 'set' | 'par' | 'var' | 'name' | 'op' | 'pun' | 'id' | 'ws';

export interface Pieza {
  ini: number;
  fin: number;
  clase: Clase;
}

/** Igual que norm() del lector: minúsculas y sin tildes. */
export function normalizar(s: string): string {
  return s
    .normalize('NFD')
    .replace(/[̀-ͯ]/g, '')
    .toLowerCase();
}

// palabra clave → su forma canónica (la misma tabla que kw() del lector)
const GRUPOS: Record<string, string[]> = {
  set: ['set', 'conjunto', 'conj'],
  param: ['param', 'parametro', 'parametros'],
  var: ['var', 'variable', 'variables', 'variavel', 'variaveis'],
  max: ['maximize', 'maximise', 'max', 'maximizar', 'maximice', 'maximo'],
  min: ['minimize', 'minimise', 'min', 'minimizar', 'minimice', 'minimo'],
  st: ['subject', 'sujeto', 'sujeta', 'sujeito', 'st', 'sa', 'restriccion', 'restricciones', 'restricao', 'restricoes', 'constraint', 'constraints'],
  sum: ['sum', 'suma', 'soma', 'sumatoria', 'somatorio'],
  in: ['in', 'en', 'em'],
  and: ['and'],
  or: ['or', 'ou'],
  not: ['not', 'no', 'nao'],
  if: ['if', 'si', 'se'],
  then: ['then', 'entonces', 'entao'],
  else: ['else', 'sino', 'senao'],
  tipo: [
    'binary', 'binaria', 'binario', 'integer', 'entera', 'entero', 'inteira', 'inteiro', 'int',
    'continuous', 'continua', 'continuo', 'free', 'libre', 'livre', 'unrestricted',
    'semicontinuous', 'semicontinua', 'semicontinuo', 'semiinteger', 'semientera', 'semientero', 'semiinteira', 'semiinteiro'
  ],
  default: ['default', 'defecto', 'omision', 'padrao'],
  conj: ['union', 'uniao', 'inter', 'interseccion', 'intersecao', 'diff', 'minus', 'menos', 'diferencia', 'diferenca'],
  option: ['option', 'opcion', 'opcao'],
  to: ['to']
};
const KW = new Map<string, string>();
for (const [k, ws] of Object.entries(GRUPOS)) for (const w of ws) KW.set(w, k);

// palabras de una letra que son «and», «or» o «to» solo según el contexto
const CORTAS: Record<string, string> = { y: 'and', e: 'and', o: 'or', a: 'to' };

const FUNCIONES = new Set([
  'abs', 'sqrt', 'raiz', 'exp', 'log', 'ln', 'log10', 'floor', 'piso', 'ceil', 'techo', 'teto', 'round', 'redondear', 'arredondar',
  'card', 'tamano', 'tamanho', 'pwl', 'tramos', 'portramos', 'max', 'maximo', 'min', 'minimo', 'and', 'or', 'ou', 'not', 'no', 'nao',
  'atmost', 'alomas', 'nomasde', 'nomaximo', 'atleast', 'almenos', 'pelomenos', 'exactly', 'exactamente', 'exatamente', 'implies', 'implica'
]);

const RE = {
  bloque: /\/\*[\s\S]*?(?:\*\/|$)/y,
  linea: /(?:#|\/\/)[^\n]*/y,
  texto: /"[^"\n]*"?|'[^'\n]*'?|«[^»\n]*»?|“[^”\n]*”?/y,
  numero: /\d+(?:\.(?!\.)\d*)?(?:[eE][+-]?\d+)?|\.\d+(?:[eE][+-]?\d+)?/y,
  st: /[sS]\.[tTaA]\./y,
  nombre: /[\p{L}_][\p{L}\p{N}_']*/uy,
  op: /<=|>=|=<|=>|==|!=|<>|:=|->|\.\.|&&|\|\||[+\-−*·×/^=<>≤≥≠:!|→⇒]/y,
  kwSimbolo: /[∈Σ∑]/y,
  pun: /[(){}[\],;]/y,
  blanco: /\s+/y
};

function probar(re: RegExp, s: string, i: number): number {
  re.lastIndex = i;
  return re.test(s) ? re.lastIndex : -1;
}

/** Corta el texto en piezas (sin clasificar aún los nombres). */
function cortar(s: string): { ini: number; fin: number; clase: Clase | 'nombre' }[] {
  const out: { ini: number; fin: number; clase: Clase | 'nombre' }[] = [];
  let i = 0;
  const n = s.length;
  while (i < n) {
    let f: number;
    if ((f = probar(RE.blanco, s, i)) > i) out.push({ ini: i, fin: f, clase: 'ws' });
    else if ((f = probar(RE.bloque, s, i)) > i) out.push({ ini: i, fin: f, clase: 'com' });
    else if ((f = probar(RE.linea, s, i)) > i) out.push({ ini: i, fin: f, clase: 'com' });
    else if ((f = probar(RE.st, s, i)) > i) out.push({ ini: i, fin: f, clase: 'kw' });
    else if ((f = probar(RE.numero, s, i)) > i) out.push({ ini: i, fin: f, clase: 'num' });
    else if ((f = probar(RE.nombre, s, i)) > i) out.push({ ini: i, fin: f, clase: 'nombre' });
    else if ((f = probar(RE.texto, s, i)) > i) out.push({ ini: i, fin: f, clase: 'str' });
    else if ((f = probar(RE.op, s, i)) > i) out.push({ ini: i, fin: f, clase: 'op' });
    else if ((f = probar(RE.kwSimbolo, s, i)) > i) out.push({ ini: i, fin: f, clase: 'kw' });
    else if ((f = probar(RE.pun, s, i)) > i) out.push({ ini: i, fin: f, clase: 'pun' });
    else {
      // carácter suelto (el lector dirá si sobra); se pinta como texto normal
      const cp = s.codePointAt(i) ?? 0;
      f = i + (cp > 0xffff ? 2 : 1);
      out.push({ ini: i, fin: f, clase: 'id' });
    }
    i = f;
  }
  return out;
}

export interface Declarados {
  sets: Set<string>;
  params: Set<string>;
  vars: Set<string>;
  /** Sin ninguna declaración var: los nombres sueltos son variables (modo rápido). */
  rapido: boolean;
}

/** Clasifica cada pieza del texto. */
export function piezas(s: string): { piezas: Pieza[]; declarados: Declarados } {
  const raw = cortar(s);
  const sig = raw.map((p, k) => k).filter((k) => raw[k].clase !== 'ws' && raw[k].clase !== 'com');
  const palabra = (k: number) => normalizar(s.slice(raw[k].ini, raw[k].fin));
  const decl: Declarados = { sets: new Set(), params: new Set(), vars: new Set(), rapido: true };
  // 1) nombres declarados: «set X», «param X», «var X» (también «var x{I}, y{J}» no existe: uno por sentencia)
  for (let q = 0; q < sig.length - 1; q++) {
    const k = sig[q];
    if (raw[k].clase !== 'nombre') continue;
    const c = KW.get(palabra(k));
    if (c !== 'set' && c !== 'param' && c !== 'var') continue;
    // solo al empezar una sentencia
    const prev = q > 0 ? sig[q - 1] : -1;
    if (prev >= 0 && s.slice(raw[prev].ini, raw[prev].fin) !== ';') continue;
    const sgte = sig[q + 1];
    if (raw[sgte].clase !== 'nombre') continue;
    const nom = s.slice(raw[sgte].ini, raw[sgte].fin);
    if (c === 'set') decl.sets.add(nom);
    else if (c === 'param') decl.params.add(nom);
    else {
      decl.vars.add(nom);
      decl.rapido = false;
    }
  }
  // 2) clase de cada nombre según su contexto
  const pos = new Map<number, number>();
  sig.forEach((k, q) => pos.set(k, q));
  const out: Pieza[] = [];
  for (let k = 0; k < raw.length; k++) {
    const p = raw[k];
    if (p.clase !== 'nombre') {
      out.push(p as Pieza);
      continue;
    }
    const texto = s.slice(p.ini, p.fin);
    const w = normalizar(texto);
    const q = pos.get(k)!;
    const prevK = q > 0 ? sig[q - 1] : -1;
    const nextK = q + 1 < sig.length ? sig[q + 1] : -1;
    const prevTxt = prevK >= 0 ? s.slice(raw[prevK].ini, raw[prevK].fin) : ';';
    const prevW = prevK >= 0 && raw[prevK].clase === 'nombre' ? normalizar(prevTxt) : '';
    const nextTxt = nextK >= 0 ? s.slice(raw[nextK].ini, raw[nextK].fin) : '';
    let clase: Clase;
    if (decl.sets.has(texto)) clase = 'set';
    else if (decl.params.has(texto)) clase = 'par';
    else if (decl.vars.has(texto)) clase = 'var';
    else if (nextTxt === '(' && FUNCIONES.has(w)) clase = 'fn';
    else if (KW.has(w)) {
      // «sujeto a», «subject to», etc.
      clase = 'kw';
    } else if (w in CORTAS) {
      const canon = CORTAS[w];
      if (canon === 'to') clase = KW.get(prevW) === 'st' ? 'kw' : decl.rapido ? 'var' : 'id';
      else clase = decl.rapido ? 'var' : 'kw';
    } else {
      // nombre de restricción u objetivo: al empezar una sentencia, antes de «:» o «{»
      const inicio = prevTxt === ';' || ['st', 'to', 'max', 'min'].includes(KW.get(prevW) ?? (prevW === 'a' ? 'to' : ''));
      if (inicio && (nextTxt === ':' || nextTxt === '{')) clase = 'name';
      else clase = decl.rapido ? 'var' : 'id';
    }
    out.push({ ini: p.ini, fin: p.fin, clase });
  }
  return { piezas: out, declarados: decl };
}

function escapar(s: string): string {
  return s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
}

/** Posición (0-based, en unidades de JavaScript) de la línea y columna (1-based) que da el lector. */
export function offsetDe(s: string, linea: number, col: number): number {
  let i = 0;
  for (let l = 1; l < linea; l++) {
    const j = s.indexOf('\n', i);
    if (j < 0) return s.length;
    i = j + 1;
  }
  // la columna cuenta caracteres (puntos de código), no unidades UTF-16
  let c = 1;
  while (c < col && i < s.length && s[i] !== '\n') {
    const cp = s.codePointAt(i) ?? 0;
    i += cp > 0xffff ? 2 : 1;
    c++;
  }
  return i;
}

/**
 * HTML resaltado del texto. `error`: posición (offset) del error, que se
 * subraya en la pieza que empieza ahí o en la siguiente.
 */
export function resaltar(s: string, error: number | null = null): string {
  const { piezas: ps } = piezas(s);
  let marcada = error === null;
  let html = '';
  for (const p of ps) {
    const txt = escapar(s.slice(p.ini, p.fin));
    let cls = p.clase === 'ws' || p.clase === 'id' ? '' : `c-${p.clase}`;
    if (!marcada && error !== null && p.clase !== 'ws' && p.clase !== 'com' && p.fin > error) {
      cls += ' c-err';
      marcada = true;
    }
    html += cls ? `<span class="${cls.trim()}">${txt}</span>` : txt;
  }
  if (!marcada) html += '<span class="c-err c-fin"> </span>';
  // un salto final necesita una línea más para que el <pre> mida lo mismo que el textarea
  return html + '\n';
}
