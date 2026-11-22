// Proveedores de modelos de IA para el asistente.
//
// Hay dos formas de API:
// - «anthropic»: la API de mensajes de Anthropic (Claude), con herramientas nativas.
// - «openai»: la API de chat de OpenAI (chat/completions), que hoy hablan casi todos:
//   OpenRouter (cientos de modelos), OpenAI, Google Gemini, Qwen, DeepSeek, Mistral,
//   Groq, xAI, servidores locales (Ollama, LM Studio, vLLM) y cualquier otro compatible.
// Las direcciones son las de la documentación de cada proveedor; todas se pueden cambiar
// en Ajustes (por ejemplo, la región de Qwen o un servidor propio de la universidad).

import { fetchIA } from './red';

export type TipoApi = 'anthropic' | 'openai';

export interface Proveedor {
  id: string;
  nombre: string;
  /** Qué ofrece (se traduce con t()). */
  descripcion: string;
  tipo: TipoApi;
  /** Dirección base (con /v1 o lo que corresponda): se le agrega /chat/completions, /messages o /models. */
  url: string;
  grupo: 'nube' | 'local' | 'otro';
  clave: 'si' | 'opcional' | 'no';
  /** Cómo suele empezar la clave (para el ejemplo del campo). */
  formatoClave?: string;
  /** Dónde se consigue la clave. */
  dondeClave?: string;
  /** Modelo inicial ('' = elegirlo de la lista). */
  modelo: string;
  sugeridos?: string[];
  /** Nombre del parámetro del largo máximo de la respuesta. */
  tokens?: 'max_tokens' | 'max_completion_tokens';
  /** false: con herramientas se pide la respuesta completa (sin streaming). */
  streamConHerramientas?: boolean;
  /** La dirección hay que revisarla o escribirla (Qwen, servidor propio). */
  editarUrl?: boolean;
  /** Aviso sobre la configuración (se traduce con t()). */
  nota?: string;
}

export const PROVEEDORES: Proveedor[] = [
  {
    id: 'openrouter',
    nombre: 'OpenRouter',
    descripcion: 'Una sola clave para cientos de modelos: GPT, Gemini, Claude, Qwen, DeepSeek, Llama, Mistral y más.',
    tipo: 'openai',
    url: 'https://openrouter.ai/api/v1',
    grupo: 'nube',
    clave: 'si',
    formatoClave: 'sk-or-…',
    dondeClave: 'https://openrouter.ai/keys',
    modelo: ''
  },
  {
    id: 'anthropic',
    nombre: 'Anthropic (Claude)',
    descripcion: 'Claude con la API de Anthropic.',
    tipo: 'anthropic',
    url: 'https://api.anthropic.com/v1',
    grupo: 'nube',
    clave: 'si',
    formatoClave: 'sk-ant-…',
    dondeClave: 'https://console.anthropic.com/settings/keys',
    modelo: 'claude-sonnet-5-5',
    sugeridos: ['claude-sonnet-5-5', 'claude-opus-5-5', 'claude-haiku-5-5']
  },
  {
    id: 'openai',
    nombre: 'OpenAI (ChatGPT)',
    descripcion: 'Los modelos GPT con la API de OpenAI.',
    tipo: 'openai',
    url: 'https://api.openai.com/v1',
    grupo: 'nube',
    clave: 'si',
    formatoClave: 'sk-…',
    dondeClave: 'https://platform.openai.com/api-keys',
    modelo: '',
    tokens: 'max_completion_tokens'
  },
  {
    id: 'gemini',
    nombre: 'Google Gemini',
    descripcion: 'Gemini con la API de Google AI Studio (compatible con OpenAI).',
    tipo: 'openai',
    url: 'https://generativelanguage.googleapis.com/v1beta/openai',
    grupo: 'nube',
    clave: 'si',
    formatoClave: 'AIza…',
    dondeClave: 'https://aistudio.google.com/apikey',
    modelo: 'gemini-3.8-flash',
    sugeridos: ['gemini-3.8-flash']
  },
  {
    id: 'qwen',
    nombre: 'Qwen (Alibaba Cloud)',
    descripcion: 'Qwen con Alibaba Cloud Model Studio (DashScope), compatible con OpenAI.',
    tipo: 'openai',
    url: 'https://dashscope-intl.aliyuncs.com/compatible-mode/v1',
    grupo: 'nube',
    clave: 'si',
    formatoClave: 'sk-…',
    dondeClave: 'https://modelstudio.console.alibabacloud.com/',
    modelo: 'qwen-plus',
    sugeridos: ['qwen-plus', 'qwen-max'],
    editarUrl: true,
    nota: 'Alibaba Cloud recomienda usar la dirección de tu espacio de trabajo (está en la consola de Model Studio, con la región: Singapur, EE. UU., Japón o China). La clave debe ser de la misma región que la dirección.'
  },
  {
    id: 'deepseek',
    nombre: 'DeepSeek',
    descripcion: 'Los modelos de DeepSeek con su API.',
    tipo: 'openai',
    url: 'https://api.deepseek.com/v1',
    grupo: 'nube',
    clave: 'si',
    formatoClave: 'sk-…',
    dondeClave: 'https://platform.deepseek.com/api_keys',
    modelo: ''
  },
  {
    id: 'mistral',
    nombre: 'Mistral AI',
    descripcion: 'Los modelos de Mistral con su API.',
    tipo: 'openai',
    url: 'https://api.mistral.ai/v1',
    grupo: 'nube',
    clave: 'si',
    dondeClave: 'https://console.mistral.ai/api-keys',
    modelo: ''
  },
  {
    id: 'groq',
    nombre: 'Groq',
    descripcion: 'Modelos abiertos (Llama, Qwen, GPT-OSS…) muy rápidos en Groq.',
    tipo: 'openai',
    url: 'https://api.groq.com/openai/v1',
    grupo: 'nube',
    clave: 'si',
    formatoClave: 'gsk_…',
    dondeClave: 'https://console.groq.com/keys',
    modelo: ''
  },
  {
    id: 'xai',
    nombre: 'xAI (Grok)',
    descripcion: 'Grok con la API de xAI.',
    tipo: 'openai',
    url: 'https://api.x.ai/v1',
    grupo: 'nube',
    clave: 'si',
    formatoClave: 'xai-…',
    dondeClave: 'https://console.x.ai',
    modelo: ''
  },
  {
    id: 'moonshot',
    nombre: 'Moonshot (Kimi)',
    descripcion: 'Kimi con la API de Moonshot AI.',
    tipo: 'openai',
    url: 'https://api.moonshot.ai/v1',
    grupo: 'nube',
    clave: 'si',
    formatoClave: 'sk-…',
    dondeClave: 'https://platform.moonshot.ai',
    modelo: ''
  },
  {
    id: 'ollama',
    nombre: 'Ollama',
    descripcion: 'Modelos abiertos en este equipo, sin internet y sin costo (Qwen, Llama, Gemma, DeepSeek…).',
    tipo: 'openai',
    url: 'http://localhost:11434/v1',
    grupo: 'local',
    clave: 'no',
    dondeClave: 'https://ollama.com/download',
    modelo: '',
    nota: 'Instala Ollama, descarga un modelo que use herramientas (por ejemplo, con «ollama pull» y el nombre del modelo) y elígelo aquí. Los modelos pequeños pueden equivocarse más al usar el motor.'
  },
  {
    id: 'lmstudio',
    nombre: 'LM Studio',
    descripcion: 'Modelos abiertos en este equipo con LM Studio (activa su servidor local).',
    tipo: 'openai',
    url: 'http://localhost:1234/v1',
    grupo: 'local',
    clave: 'no',
    dondeClave: 'https://lmstudio.ai',
    modelo: ''
  },
  {
    id: 'personalizado',
    nombre: 'Otro compatible con OpenAI',
    descripcion: 'Cualquier servidor con la API de chat de OpenAI: vLLM o LocalAI de tu universidad, Together, Fireworks, Azure OpenAI, etc.',
    tipo: 'openai',
    url: '',
    grupo: 'otro',
    clave: 'opcional',
    modelo: '',
    editarUrl: true,
    nota: 'Escribe la dirección base de la API (la que termina antes de /chat/completions, normalmente en /v1).'
  }
];

export function proveedor(id: string): Proveedor {
  return PROVEEDORES.find((p) => p.id === id) ?? PROVEEDORES[0];
}

/** Dirección base sin la barra final. */
export function base(url: string): string {
  return url.trim().replace(/\/+$/, '').replace(/\/(chat\/completions|messages|models)$/, '');
}

/** ¿La dirección es de este equipo o de la red local? (para avisar si va sin cifrar a otra parte) */
export function esLocal(url: string): boolean {
  try {
    const h = new URL(url).hostname;
    return /^(localhost|127\.|0\.0\.0\.0|\[?::1\]?|10\.|192\.168\.|172\.(1[6-9]|2\d|3[01])\.)/.test(h) || h.endsWith('.local');
  } catch {
    return false;
  }
}

export interface Conexion {
  p: Proveedor;
  url: string;
  clave: string;
  modelo: string;
}

export function urlChat(c: Conexion): string {
  return base(c.url) + (c.p.tipo === 'anthropic' ? '/messages' : '/chat/completions');
}

export function cabeceras(c: Conexion): Record<string, string> {
  const h: Record<string, string> = { 'content-type': 'application/json' };
  const k = c.clave.trim();
  if (c.p.tipo === 'anthropic') {
    h['x-api-key'] = k;
    h['anthropic-version'] = '2023-06-01';
    h['anthropic-dangerous-direct-browser-access'] = 'true';
  } else if (k) {
    h['authorization'] = `Bearer ${k}`;
  }
  // OpenRouter: nombre de la aplicación para sus estadísticas
  if (c.p.id === 'openrouter') h['x-title'] = 'IAQSB';
  return h;
}

/** Mensaje de error que trae el cuerpo de una respuesta de cualquier proveedor. */
export function detalleError(j: unknown): { tipo: string; mensaje: string } {
  const x = (Array.isArray(j) ? j[0] : j) as any;
  const e = x?.error ?? x;
  const mensaje = typeof e === 'string' ? e : (e?.message ?? x?.message ?? x?.detail ?? '');
  const tipo = String(e?.type ?? e?.code ?? e?.status ?? '');
  let m = typeof mensaje === 'string' ? mensaje : JSON.stringify(mensaje);
  // OpenRouter pone el error del proveedor de fondo en metadata.raw
  const raw = e?.metadata?.raw;
  if (raw && typeof raw === 'string' && !m.includes(raw.slice(0, 40))) m += ` (${raw.slice(0, 300)})`;
  return { tipo, mensaje: m };
}

export interface ModeloDisponible {
  id: string;
  nombre?: string;
  /** null = no se sabe */
  herramientas: boolean | null;
}

/** Modelos que ofrece el proveedor (GET …/models). */
export async function listarModelos(c: Conexion, signal?: AbortSignal): Promise<ModeloDisponible[]> {
  const h = cabeceras(c);
  delete h['content-type'];
  const res = await fetchIA(base(c.url) + '/models' + (c.p.tipo === 'anthropic' ? '?limit=1000' : ''), { method: 'GET', headers: h, signal });
  const txt = await res.text();
  let j: any = null;
  try {
    j = JSON.parse(txt);
  } catch {
    /* no es JSON */
  }
  if (!res.ok) throw new Error(`${res.status} · ${detalleError(j).mensaje || txt.slice(0, 200)}`);
  const lista: any[] = Array.isArray(j) ? j : Array.isArray(j?.data) ? j.data : Array.isArray(j?.models) ? j.models : [];
  const out: ModeloDisponible[] = lista
    .map((m) => {
      const id = String(m?.id ?? m?.name ?? '').replace(/^models\//, '');
      const sp = m?.supported_parameters;
      return { id, nombre: typeof m?.name === 'string' && m.name !== id ? m.name : (m?.display_name ?? undefined), herramientas: Array.isArray(sp) ? sp.includes('tools') : null };
    })
    .filter((m) => m.id);
  out.sort((a, b) => a.id.localeCompare(b.id));
  return out;
}

/** Prueba la conexión con una pregunta mínima al modelo elegido. */
export async function probar(c: Conexion, signal?: AbortSignal): Promise<void> {
  const cuerpo: Record<string, unknown> =
    c.p.tipo === 'anthropic'
      ? { model: c.modelo, max_tokens: 16, messages: [{ role: 'user', content: 'ok' }] }
      : { model: c.modelo, messages: [{ role: 'user', content: 'Reply with: ok' }], [c.p.tokens ?? 'max_tokens']: 16, stream: false };
  for (let intento = 0; ; intento++) {
    const res = await fetchIA(urlChat(c), { method: 'POST', headers: cabeceras(c), body: JSON.stringify(cuerpo), signal });
    const txt = await res.text();
    if (res.ok) return;
    let j: unknown = null;
    try {
      j = JSON.parse(txt);
    } catch {
      /* no es JSON */
    }
    const m = detalleError(j).mensaje || txt.slice(0, 200);
    // modelos que solo responden por partes (p. ej. Qwen pensando)
    if (intento === 0 && c.p.tipo === 'openai' && /stream/i.test(m)) {
      cuerpo.stream = true;
      continue;
    }
    if (res.status >= 300 && res.status < 400) throw new Error(`${res.status} · ${res.headers.get('location') ?? ''}`);
    throw new Error(`${res.status} · ${m}`);
  }
}
