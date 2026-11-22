// API de chat compatible con OpenAI (chat/completions) para el asistente.
//
// El historial del asistente se guarda en el formato de la API de mensajes de Anthropic
// (bloques text / tool_use / tool_result). Aquí se traduce a mensajes de OpenAI y se
// lee la respuesta (por partes o completa) de vuelta a bloques. Sin dependencias de
// Svelte: se prueba con tools/test_ia.ts.

export type Bloque =
  | { type: 'text'; text: string }
  | { type: 'tool_use'; id: string; name: string; input: Record<string, unknown> }
  | { type: 'tool_result'; tool_use_id: string; content: string; is_error?: boolean }
  | { type: string; [k: string]: unknown };

export interface MensajeApi {
  role: 'user' | 'assistant';
  content: Bloque[];
  /**
   * Lo que algunos proveedores compatibles con OpenAI piden que se devuelva tal cual en
   * la ronda siguiente (razonamiento de los modelos que piensan, firmas de Gemini en
   * cada llamada). No se manda a la API de Anthropic.
   */
  extra?: Extra;
}

export interface Extra {
  reasoning_content?: string;
  reasoning_details?: unknown[];
  /** extra_content de cada llamada (Gemini: thought_signature), por id. */
  llamadas?: Record<string, unknown>;
}

export interface OpcionesOpenAI {
  conHerramientas?: boolean;
  /** Mistral: ids de 9 letras o dígitos y nunca un mensaje del usuario justo después de un «tool». */
  mistral?: boolean;
}

export interface Herramienta {
  name: string;
  description: string;
  input_schema: Record<string, any>;
}

/** Motivo de fin con los nombres de Anthropic: tool_use, end_turn, max_tokens, refusal. */
export type Fin = 'tool_use' | 'end_turn' | 'max_tokens' | 'refusal';

const ID_OK = /^[a-zA-Z0-9_-]+$/;

/** Id de llamada válido para todas las API (algunos proveedores no lo mandan o traen otros caracteres). */
export function idValido(id: unknown): string {
  if (typeof id === 'string' && id && ID_OK.test(id)) return id;
  if (typeof id === 'string' && id) return id.replace(/[^a-zA-Z0-9_-]/g, '_');
  const abc = 'abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789';
  let s = '';
  for (let i = 0; i < 9; i++) s += abc[Math.floor(Math.random() * abc.length)];
  return s;
}

/** Mistral exige ids de exactamente 9 caracteres [a-zA-Z0-9]: uno estable a partir del original. */
export function id9(id: string): string {
  if (/^[a-zA-Z0-9]{9}$/.test(id)) return id;
  let h = 2166136261;
  let g = 5381;
  for (const c of id) {
    h = Math.imul(h ^ c.charCodeAt(0), 16777619) >>> 0;
    g = (Math.imul(g, 33) + c.charCodeAt(0)) >>> 0;
  }
  return (h.toString(36) + g.toString(36) + '000000000').replace(/[^a-z0-9]/g, '').slice(0, 9);
}

/** Toda lista necesita «items» (OpenAI y Gemini rechazan una sin ellos). */
export function conItems(s: any): any {
  if (!s || typeof s !== 'object') return s;
  if (Array.isArray(s)) return s.map(conItems);
  const out: any = { ...s };
  if (out.type === 'array' && !out.items) out.items = { type: 'string' };
  for (const k of ['properties']) if (out[k]) out[k] = Object.fromEntries(Object.entries(out[k]).map(([n, v]) => [n, conItems(v)]));
  if (out.items) out.items = conItems(out.items);
  return out;
}

/**
 * Esquemas para modelos con restricciones (Gemini no acepta objetos sin propiedades):
 * un objeto libre pasa a ser texto JSON y se vuelve a leer al ejecutar la herramienta.
 */
export function esquemaSimple(s: any, raiz = true): any {
  if (!s || typeof s !== 'object') return s;
  if (s.type === 'object') {
    const props = s.properties && Object.keys(s.properties).length ? s.properties : null;
    if (!props) return raiz ? { type: 'object', properties: {} } : { type: 'string', description: `${s.description ?? ''} (JSON text)`.trim() };
    return { ...s, properties: Object.fromEntries(Object.entries(props).map(([k, v]) => [k, esquemaSimple(v, false)])) };
  }
  if (s.type === 'array' && s.items) return { ...s, items: esquemaSimple(s.items, false) };
  return s;
}

export function herramientasOpenAI(defs: Herramienta[], simples = false): unknown[] {
  return defs.map((d) => {
    const vacia = !d.input_schema?.properties || !Object.keys(d.input_schema.properties).length;
    const parameters = conItems(simples ? esquemaSimple(d.input_schema) : d.input_schema);
    const f: Record<string, unknown> = { name: d.name, description: d.description };
    // sin parámetros: algunos proveedores rechazan un objeto vacío
    if (!(simples && vacia)) f.parameters = parameters;
    return { type: 'function', function: f };
  });
}

/** Los argumentos que llegaron como texto JSON (esquemas simples) se vuelven objetos. */
export function corregirEntrada(def: Herramienta | undefined, input: Record<string, unknown>): Record<string, unknown> {
  if (!def) return input;
  const out = { ...input };
  for (const [k, v] of Object.entries(out)) {
    const tipo = def.input_schema?.properties?.[k]?.type;
    if (typeof v === 'string' && (tipo === 'object' || tipo === 'array')) {
      try {
        out[k] = JSON.parse(v);
      } catch {
        /* se deja como vino: la herramienta dirá qué falta */
      }
    }
  }
  return out;
}

const textoDe = (bs: Bloque[]) =>
  bs
    .filter((b) => b.type === 'text')
    .map((b) => String((b as { text: string }).text))
    .filter((x) => x.trim())
    .join('\n\n');

/** Imagen o PDF adjunto (bloque de Anthropic) → parte de un mensaje de OpenAI. */
export function parteOpenAI(b: Bloque): Record<string, unknown> | null {
  const s = (b as { source?: { type?: string; media_type?: string; data?: string } }).source;
  if (!s || s.type !== 'base64' || !s.data) return null;
  const url = `data:${s.media_type};base64,${s.data}`;
  if (b.type === 'image') return { type: 'image_url', image_url: { url } };
  if (b.type === 'document') return { type: 'file', file: { filename: String((b as { title?: string }).title || 'documento.pdf'), file_data: url } };
  return null;
}

/**
 * Historial (bloques de Anthropic) → mensajes de OpenAI. Sin herramientas, las llamadas
 * y sus resultados se escriben como texto (para modelos que no usan herramientas).
 */
export function aOpenAI(historial: MensajeApi[], sistema: string, op: OpcionesOpenAI | boolean = {}): unknown[] {
  const o: OpcionesOpenAI = typeof op === 'boolean' ? { conHerramientas: op } : op;
  const conHerramientas = o.conHerramientas ?? true;
  const id = (x: string) => (o.mistral ? id9(idValido(x)) : idValido(x));
  // la pregunta en curso empieza en el último mensaje del usuario con texto (no solo resultados)
  let turno = 0;
  historial.forEach((m, k) => {
    if (m.role === 'user' && m.content.some((b) => b.type === 'text')) turno = k;
  });
  const out: unknown[] = [{ role: 'system', content: sistema }];
  historial.forEach((m, k) => {
    if (m.role === 'assistant') {
      const usos = m.content.filter((b) => b.type === 'tool_use') as Extract<Bloque, { type: 'tool_use' }>[];
      const txt = textoDe(m.content);
      if (conHerramientas && usos.length) {
        const msg: Record<string, unknown> = {
          role: 'assistant',
          content: txt || null,
          tool_calls: usos.map((u) => {
            const tc: Record<string, unknown> = { id: id(u.id), type: 'function', function: { name: u.name, arguments: JSON.stringify(u.input ?? {}) } };
            const ex = m.extra?.llamadas?.[u.id];
            if (ex !== undefined) tc.extra_content = ex;
            return tc;
          })
        };
        // razonamiento: se devuelve dentro de la pregunta en curso (DeepSeek, Kimi, Qwen…)
        if (m.extra?.reasoning_content && k > turno) msg.reasoning_content = m.extra.reasoning_content;
        if (m.extra?.reasoning_details?.length) msg.reasoning_details = m.extra.reasoning_details;
        out.push(msg);
      } else {
        const extra = usos.map((u) => `[${u.name} ${JSON.stringify(u.input ?? {})}]`).join('\n');
        const c = [txt, extra].filter(Boolean).join('\n\n');
        if (c) out.push({ role: 'assistant', content: c });
      }
    } else {
      const res = m.content.filter((b) => b.type === 'tool_result') as Extract<Bloque, { type: 'tool_result' }>[];
      if (conHerramientas) {
        for (const r of res) out.push({ role: 'tool', tool_call_id: id(r.tool_use_id), content: (r.is_error ? 'Error: ' : '') + (r.content || '(ok)') });
      }
      const txt = textoDe(m.content);
      const extra = conHerramientas ? '' : res.map((r) => `[result${r.is_error ? ' (error)' : ''}]\n${r.content}`).join('\n\n');
      const c = [extra, txt].filter(Boolean).join('\n\n');
      // imágenes y PDF adjuntos: partes del mensaje (image_url y file, como en la API de OpenAI)
      const medios = m.content.filter((b) => b.type === 'image' || b.type === 'document').map(parteOpenAI).filter(Boolean);
      if (c || medios.length) {
        if (o.mistral && (out.at(-1) as { role?: string })?.role === 'tool') out.push({ role: 'assistant', content: '(…)' });
        out.push({ role: 'user', content: medios.length ? [...(c ? [{ type: 'text', text: c }] : []), ...medios] : c });
      }
    }
  });
  // dos mensajes seguidos del mismo rol (user/assistant) se juntan: algunos proveedores no lo aceptan
  const juntos: any[] = [];
  for (const m of out as any[]) {
    const prev = juntos.at(-1);
    if (prev && prev.role === m.role && (m.role === 'user' || (m.role === 'assistant' && !prev.tool_calls && !m.tool_calls)) && typeof prev.content === 'string' && typeof m.content === 'string') {
      prev.content += '\n\n' + m.content;
    } else juntos.push({ ...m });
  }
  return juntos;
}

/** finish_reason de OpenAI → motivo de Anthropic. */
export function fin(finish: string | null | undefined, hayLlamadas: boolean): Fin {
  if (hayLlamadas && finish !== 'length') return 'tool_use';
  if (finish === 'tool_calls' || finish === 'function_call') return 'tool_use';
  if (finish === 'length' || finish === 'max_tokens') return 'max_tokens';
  if (finish === 'content_filter' || finish === 'refusal' || finish === 'safety') return 'refusal';
  return 'end_turn';
}

function contenido(c: unknown): string {
  if (typeof c === 'string') return c;
  if (Array.isArray(c)) return c.map((x) => (typeof x === 'string' ? x : (x?.text ?? ''))).join('');
  return '';
}

/** Lo que se va viendo mientras llega la respuesta. */
export interface Vista {
  texto(delta: string): void;
  herramienta(i: number, id: string, nombre: string): void;
  argumentos(i: number, parcial: string): void;
}

/** Junta los trozos (chunks) de una respuesta por partes. */
export class Acumulador {
  private texto = '';
  private razonamiento = '';
  private detalles: unknown[] = [];
  private llamadas: { id: string; nombre: string; args: string; extra?: unknown }[] = [];
  private finish: string | null = null;
  terminado = false;
  private vista: Vista;

  constructor(vista: Vista) {
    this.vista = vista;
  }

  /** Un evento «data:» ya leído como JSON (o [DONE]). */
  evento(d: any): void {
    if (d === '[DONE]') {
      this.terminado = true;
      return;
    }
    if (d?.error) {
      const e = d.error;
      throw Object.assign(new Error(typeof e === 'string' ? e : (e.message ?? JSON.stringify(e))), { codigo: e.code ?? e.type ?? '' });
    }
    for (const ch of d?.choices ?? []) {
      const dl = ch.delta ?? ch.message ?? {};
      const t = contenido(dl.content);
      if (t) {
        this.texto += t;
        this.vista.texto(t);
      }
      if (typeof dl.reasoning_content === 'string') this.razonamiento += dl.reasoning_content;
      if (Array.isArray(dl.reasoning_details)) this.detalles.push(...dl.reasoning_details);
      const tcs: any[] = Array.isArray(dl.tool_calls) ? dl.tool_calls : [];
      tcs.forEach((tc, k) => {
        const i = typeof tc.index === 'number' ? tc.index : this.llamadas.length > k && !tc.id ? k : this.llamadas.length;
        let ll = this.llamadas[i];
        if (!ll) {
          ll = this.llamadas[i] = { id: idValido(tc.id), nombre: tc.function?.name ?? '', args: '' };
          this.vista.herramienta(i, ll.id, ll.nombre);
        } else if (tc.function?.name && !ll.nombre) ll.nombre = tc.function.name;
        if (tc.extra_content !== undefined) ll.extra = tc.extra_content;
        const a = tc.function?.arguments;
        if (a !== undefined && a !== null) {
          ll.args += typeof a === 'string' ? a : JSON.stringify(a);
          this.vista.argumentos(i, ll.args);
        }
      });
      if (ch.finish_reason) this.finish = ch.finish_reason;
    }
  }

  /** ¿Llegó el final (finish_reason o [DONE])? */
  get completo(): boolean {
    return this.terminado || this.finish !== null;
  }

  /** Lo que hay que devolver en la ronda siguiente (ver MensajeApi.extra). */
  extra(): Extra | undefined {
    const e: Extra = {};
    if (this.razonamiento) e.reasoning_content = this.razonamiento;
    if (this.detalles.length) e.reasoning_details = this.detalles;
    const ll = this.llamadas.filter((l) => l && l.extra !== undefined);
    if (ll.length) e.llamadas = Object.fromEntries(ll.map((l) => [l.id, l.extra]));
    return Object.keys(e).length ? e : undefined;
  }

  resultado(): { bloques: Bloque[]; stop: Fin; extra?: Extra } {
    const bloques: Bloque[] = [];
    if (this.texto.trim()) bloques.push({ type: 'text', text: this.texto });
    const llamadas = this.llamadas.filter((l) => l && l.nombre);
    for (const l of llamadas) {
      let input: Record<string, unknown> = {};
      try {
        const v = l.args.trim() ? JSON.parse(l.args) : {};
        input = v && typeof v === 'object' && !Array.isArray(v) ? v : {};
      } catch {
        input = {};
      }
      bloques.push({ type: 'tool_use', id: l.id, name: l.nombre, input });
    }
    return { bloques, stop: fin(this.finish, llamadas.length > 0), extra: this.extra() };
  }
}

/** Respuesta completa (sin streaming). */
export function deRespuesta(j: any, vista: Vista): { bloques: Bloque[]; stop: Fin; extra?: Extra } {
  const a = new Acumulador(vista);
  if (j?.error) a.evento(j);
  const ch = j?.choices?.[0];
  const msg = ch?.message ?? {};
  if (ch)
    a.evento({
      choices: [
        {
          delta: { content: msg.content, reasoning_content: msg.reasoning_content, reasoning_details: msg.reasoning_details, tool_calls: (msg.tool_calls ?? []).map((tc: any, i: number) => ({ index: i, ...tc })) },
          finish_reason: ch.finish_reason ?? 'stop'
        }
      ]
    });
  return a.resultado();
}

/** Lee los eventos «data:» de un cuerpo SSE. */
export async function* eventosSSE(cuerpo: ReadableStream<Uint8Array>): AsyncGenerator<string> {
  const lector = cuerpo.getReader();
  const dec = new TextDecoder();
  let buf = '';
  const sacar = function* (final: boolean) {
    let k: number;
    while ((k = buf.indexOf('\n\n')) >= 0 || (final && buf.trim())) {
      const crudo = k >= 0 ? buf.slice(0, k) : buf;
      buf = k >= 0 ? buf.slice(k + 2) : '';
      const datos = crudo
        .split('\n')
        .filter((l) => l.startsWith('data:'))
        .map((l) => l.slice(5).replace(/^ /, ''))
        .join('\n');
      if (datos) yield datos;
      if (k < 0) break;
    }
  };
  let completo = false;
  try {
    for (;;) {
      const { done, value } = await lector.read();
      if (done) break;
      buf += dec.decode(value, { stream: true }).replace(/\r\n?/g, '\n');
      yield* sacar(false);
    }
    buf += dec.decode();
    yield* sacar(true);
    completo = true;
  } finally {
    // si quien lee se detuvo antes (un error a mitad), se corta la conexión
    if (!completo) lector.cancel().catch(() => {});
  }
}

/** El modelo o el proveedor no admite herramientas (para reintentar sin ellas). */
export function sinSoporteHerramientas(status: number, detalle: string): boolean {
  const d = detalle.toLowerCase();
  if (!/tool|function/.test(d)) return false;
  return (
    status === 404 ||
    /not support|unsupported|does not support|doesn't support|no endpoints found that support|not available|not enabled|is not allowed/.test(d)
  );
}
