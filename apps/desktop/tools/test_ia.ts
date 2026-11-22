// Pruebas del formato compatible con OpenAI del asistente (src/lib/ia_openai.ts) y de
// las utilidades de proveedores, sin dependencias.
//
// Uso (Node 22.18 o posterior, que entiende TypeScript):
//   node tools/test_ia.ts

import {
  Acumulador,
  aOpenAI,
  corregirEntrada,
  deRespuesta,
  conItems,
  esquemaSimple,
  id9,
  eventosSSE,
  fin,
  herramientasOpenAI,
  idValido,
  sinSoporteHerramientas,
  type MensajeApi,
  type Vista
} from '../src/lib/ia_openai.ts';

let ok = 0;
let mal = 0;
function prueba(nombre: string, cond: boolean, detalle?: unknown) {
  if (cond) ok++;
  else {
    mal++;
    console.log('✗', nombre, detalle === undefined ? '' : JSON.stringify(detalle));
  }
}

function vistaQueAnota() {
  const log: string[] = [];
  const v: Vista = {
    texto: (d) => log.push('t:' + d),
    herramienta: (i, id, n) => log.push(`h:${i}:${id}:${n}`),
    argumentos: (i, p) => log.push(`a:${i}:${p}`)
  };
  return { v, log };
}

// ---------------------------------------------------------------- historial → OpenAI
const hist: MensajeApi[] = [
  { role: 'user', content: [{ type: 'text', text: '<ctx>datos</ctx>' }, { type: 'text', text: '¿Cuánto da?' }] },
  {
    role: 'assistant',
    content: [
      { type: 'text', text: 'Voy a resolver.' },
      { type: 'tool_use', id: 'toolu_01A', name: 'solve', input: { module: 'lp', model: { a: 1 } } },
      { type: 'tool_use', id: 'call:raro/1', name: 'list_modules', input: {} }
    ]
  },
  {
    role: 'user',
    content: [
      { type: 'tool_result', tool_use_id: 'toolu_01A', content: 'Z = 36' },
      { type: 'tool_result', tool_use_id: 'call:raro/1', content: 'falló', is_error: true },
      { type: 'text', text: 'Y explícalo' }
    ]
  },
  { role: 'assistant', content: [{ type: 'thinking', thinking: '…' }, { type: 'text', text: 'Z vale 36.' }] }
];
const m = aOpenAI(hist, 'SISTEMA') as any[];
prueba('sistema primero', m[0].role === 'system' && m[0].content === 'SISTEMA');
prueba('usuario junta los textos', m[1].role === 'user' && m[1].content === '<ctx>datos</ctx>\n\n¿Cuánto da?', m[1]);
prueba('asistente con tool_calls', m[2].role === 'assistant' && m[2].tool_calls.length === 2 && m[2].content === 'Voy a resolver.', m[2]);
prueba('argumentos en JSON', m[2].tool_calls[0].function.arguments === '{"module":"lp","model":{"a":1}}');
prueba('id raro se limpia igual en la llamada y en la respuesta', m[2].tool_calls[1].id === 'call_raro_1' && m[4].tool_call_id === 'call_raro_1', [m[2].tool_calls[1].id, m[4]]);
prueba('tool después del asistente', m[3].role === 'tool' && m[3].tool_call_id === 'toolu_01A' && m[3].content === 'Z = 36');
prueba('error de herramienta marcado', m[4].content === 'Error: falló');
prueba('texto del usuario después de los tool', m[5].role === 'user' && m[5].content === 'Y explícalo');
prueba('bloques thinking se omiten', m[6].role === 'assistant' && m[6].content === 'Z vale 36.' && m.length === 7, m[6]);

const s = aOpenAI(hist, 'S', false) as any[];
prueba('sin herramientas: no hay tool_calls ni role tool', s.every((x) => !x.tool_calls && x.role !== 'tool'), s);
prueba('sin herramientas: la llamada queda como texto', s[2].content.includes('[solve {"module":"lp"'), s[2]);
prueba('sin herramientas: el resultado va en el mensaje del usuario', s[3].role === 'user' && s[3].content.includes('Z = 36') && s[3].content.includes('Y explícalo'), s[3]);

// ---------------------------------------------------------------- herramientas
const defs = [
  { name: 'list_modules', description: 'Lista', input_schema: { type: 'object', properties: {} } },
  { name: 'solve', description: 'Resuelve', input_schema: { type: 'object', properties: { module: { type: 'string' }, model: { type: 'object', description: 'Modelo' }, values: { type: 'array', items: { type: 'number' } } }, required: ['module', 'model'] } }
];
const h = herramientasOpenAI(defs) as any[];
prueba('formato function', h[1].type === 'function' && h[1].function.name === 'solve' && h[1].function.parameters.required.length === 2);
const hs = herramientasOpenAI(defs, true) as any[];
prueba('esquemas simples: sin parámetros si no hay propiedades', hs[0].function.parameters === undefined, hs[0]);
prueba('esquemas simples: objeto libre → texto JSON', hs[1].function.parameters.properties.model.type === 'string', hs[1]);
prueba('esquemas simples: lo demás igual', hs[1].function.parameters.properties.values.type === 'array');
prueba('esquemaSimple no toca objetos con propiedades', esquemaSimple({ type: 'object', properties: { x: { type: 'number' } } }).properties.x.type === 'number');
const ce = corregirEntrada(defs[1], { module: 'lp', model: '{"a":2}', values: '[1,2]' });
prueba('corregirEntrada lee los objetos que llegaron como texto', (ce.model as any).a === 2 && Array.isArray(ce.values), ce);
prueba('corregirEntrada deja el texto que no es JSON', corregirEntrada(defs[1], { model: 'no json' }).model === 'no json');

// ---------------------------------------------------------------- ids y fin
prueba('id válido se conserva', idValido('call_abc-1') === 'call_abc-1');
prueba('id vacío se genera (9 caracteres)', /^[a-zA-Z0-9]{9}$/.test(idValido(undefined)));
prueba('fin tool_calls', fin('tool_calls', true) === 'tool_use' && fin('stop', true) === 'tool_use' && fin('stop', false) === 'end_turn');
prueba('fin length y filtro', fin('length', true) === 'max_tokens' && fin('content_filter', false) === 'refusal');

// ---------------------------------------------------------------- streaming
{
  const { v, log } = vistaQueAnota();
  const a = new Acumulador(v);
  a.evento({ choices: [{ delta: { role: 'assistant', content: '' } }] });
  a.evento({ choices: [{ delta: { content: 'Hola' } }] });
  a.evento({ choices: [{ delta: { tool_calls: [{ index: 0, id: 'call_1', type: 'function', function: { name: 'solve', arguments: '' } }] } }] });
  a.evento({ choices: [{ delta: { tool_calls: [{ index: 0, function: { arguments: '{"module":' } }] } }] });
  a.evento({ choices: [{ delta: { tool_calls: [{ index: 0, function: { arguments: '"lp"}' } }, { index: 1, id: 'call_2', function: { name: 'list_modules', arguments: '{}' } }] } }] });
  a.evento({ choices: [{ delta: {}, finish_reason: 'tool_calls' }] });
  a.evento({ choices: [], usage: { total_tokens: 10 } });
  a.evento('[DONE]');
  const r = a.resultado();
  prueba('stream: texto + dos llamadas', r.bloques.length === 3 && r.stop === 'tool_use', r);
  prueba('stream: argumentos armados por partes', (r.bloques[1] as any).input.module === 'lp' && (r.bloques[2] as any).name === 'list_modules', r.bloques);
  prueba('stream: la vista ve texto, herramientas y argumentos', log.includes('t:Hola') && log.includes('h:0:call_1:solve') && log.includes('a:0:{"module":"lp"}') && log.includes('h:1:call_2:list_modules'), log);
  prueba('stream: completo', a.completo);
}
{
  // Gemini y otros: llamadas completas sin índice, finish_reason «stop»
  const { v } = vistaQueAnota();
  const a = new Acumulador(v);
  a.evento({ choices: [{ delta: { tool_calls: [{ id: '', type: 'function', function: { name: 'solve', arguments: '{"module":"qa"}' } }] }, finish_reason: 'stop' }] });
  const r = a.resultado();
  prueba('sin índice ni id: se genera el id y se usa la herramienta', r.stop === 'tool_use' && /^[a-zA-Z0-9]{9}$/.test((r.bloques[0] as any).id) && (r.bloques[0] as any).input.module === 'qa', r);
}
{
  // argumentos que ya vienen como objeto (algunos servidores locales)
  const { v } = vistaQueAnota();
  const a = new Acumulador(v);
  a.evento({ choices: [{ delta: { tool_calls: [{ index: 0, id: 'x1', function: { name: 'solve', arguments: { module: 'net' } } }] }, finish_reason: 'tool_calls' }] });
  prueba('argumentos como objeto', (a.resultado().bloques[0] as any).input.module === 'net');
}
{
  const { v } = vistaQueAnota();
  const a = new Acumulador(v);
  let lanzo = false;
  try {
    a.evento({ error: { message: 'Rate limit', code: 429 } });
  } catch (e) {
    lanzo = (e as Error).message === 'Rate limit' && (e as any).codigo === 429;
  }
  prueba('error dentro del streaming', lanzo);
}
{
  const { v, log } = vistaQueAnota();
  const r = deRespuesta(
    { choices: [{ message: { role: 'assistant', content: 'Listo', tool_calls: [{ id: 'c1', type: 'function', function: { name: 'solve', arguments: '{"module":"lp"}' } }] }, finish_reason: 'tool_calls' }] },
    v
  );
  prueba('respuesta completa (sin streaming)', r.stop === 'tool_use' && r.bloques.length === 2 && log.includes('t:Listo'), r);
  const r2 = deRespuesta({ choices: [{ message: { content: [{ type: 'text', text: 'a' }, { type: 'text', text: 'b' }] }, finish_reason: 'length' }] }, vistaQueAnota().v);
  prueba('contenido en partes y largo máximo', (r2.bloques[0] as any).text === 'ab' && r2.stop === 'max_tokens', r2);
}

// ---------------------------------------------------------------- SSE
async function sse() {
  const enc = new TextEncoder();
  const trozos = [': OPENROUTER PROCESSING\n\n', 'data: {"a":', '1}\r\n\r\ndata: {"b":2}\n\n', 'data: [DONE]'];
  const cuerpo = new ReadableStream<Uint8Array>({
    start(c) {
      for (const t of trozos) c.enqueue(enc.encode(t));
      c.close();
    }
  });
  const out: string[] = [];
  for await (const d of eventosSSE(cuerpo)) out.push(d);
  prueba('SSE: comentarios fuera, eventos partidos, CRLF y el último sin línea en blanco', JSON.stringify(out) === JSON.stringify(['{"a":1}', '{"b":2}', '[DONE]']), out);
}

// ---------------------------------------------------------------- sin herramientas
prueba('OpenRouter sin endpoints con herramientas', sinSoporteHerramientas(404, 'No endpoints found that support tool use. Try disabling "solve".'));
prueba('Ollama: el modelo no admite herramientas', sinSoporteHerramientas(400, 'registry.ollama.ai/library/gemma:2b does not support tools'));
prueba('otro 400 no es por herramientas', !sinSoporteHerramientas(400, 'Invalid value for temperature'));
prueba('404 de modelo inexistente no es por herramientas', !sinSoporteHerramientas(404, 'The model `gpt-9` does not exist'));

// ---------------------------------------------------------------- revisión: listas, razonamiento, Mistral
{
  const sinItems = [{ name: 'compare_methods', description: 'x', input_schema: { type: 'object', properties: { values: { type: 'array' }, m: { type: 'object', properties: { l: { type: 'array' } } } } } }];
  const hh = herramientasOpenAI(sinItems) as any[];
  prueba('toda lista lleva items (OpenAI y Gemini la exigen)', hh[0].function.parameters.properties.values.items?.type === 'string' && hh[0].function.parameters.properties.m.properties.l.items?.type === 'string', hh[0]);
  prueba('conItems no toca items existentes', conItems({ type: 'array', items: { type: 'number' } }).items.type === 'number');

  const conExtra: MensajeApi[] = [
    { role: 'user', content: [{ type: 'text', text: 'P1' }] },
    { role: 'assistant', content: [{ type: 'tool_use', id: 'c1', name: 'solve', input: {} }], extra: { reasoning_content: 'pensé 1', llamadas: { c1: { google: { thought_signature: 'SIG1' } } } } },
    { role: 'user', content: [{ type: 'tool_result', tool_use_id: 'c1', content: 'ok' }] },
    { role: 'assistant', content: [{ type: 'text', text: 'R1' }] },
    { role: 'user', content: [{ type: 'text', text: 'P2' }] },
    { role: 'assistant', content: [{ type: 'tool_use', id: 'c2', name: 'solve', input: {} }], extra: { reasoning_content: 'pensé 2', reasoning_details: [{ type: 'reasoning.text', text: 'x' }], llamadas: { c2: { google: { thought_signature: 'SIG2' } } } } },
    { role: 'user', content: [{ type: 'tool_result', tool_use_id: 'c2', content: 'ok' }] }
  ];
  const mx = aOpenAI(conExtra, 'S') as any[];
  const a1 = mx.find((x) => x.tool_calls?.[0]?.id === 'c1');
  const a2 = mx.find((x) => x.tool_calls?.[0]?.id === 'c2');
  prueba('firma de Gemini devuelta en cada llamada', a1.tool_calls[0].extra_content.google.thought_signature === 'SIG1' && a2.tool_calls[0].extra_content.google.thought_signature === 'SIG2');
  prueba('razonamiento solo dentro de la pregunta en curso', a1.reasoning_content === undefined && a2.reasoning_content === 'pensé 2', [a1, a2]);
  prueba('reasoning_details devueltos (OpenRouter)', Array.isArray(a2.reasoning_details));

  const { v } = vistaQueAnota();
  const ac = new Acumulador(v);
  ac.evento({ choices: [{ delta: { reasoning_content: 'pien' } }] });
  ac.evento({ choices: [{ delta: { reasoning_content: 'so', tool_calls: [{ index: 0, id: 'g1', function: { name: 'solve', arguments: '{}' }, extra_content: { google: { thought_signature: 'S' } } }] }, finish_reason: 'tool_calls' }] });
  const rr = ac.resultado();
  prueba('el acumulador guarda razonamiento y firma', rr.extra?.reasoning_content === 'pienso' && (rr.extra?.llamadas?.g1 as any)?.google?.thought_signature === 'S', rr.extra);

  const mi = aOpenAI(
    [
      { role: 'user', content: [{ type: 'text', text: 'P' }] },
      { role: 'assistant', content: [{ type: 'tool_use', id: 'toolu_01XYZ', name: 'solve', input: {} }] },
      { role: 'user', content: [{ type: 'tool_result', tool_use_id: 'toolu_01XYZ', content: 'Cancelled' }, { type: 'text', text: 'otra' }] }
    ],
    'S',
    { mistral: true }
  ) as any[];
  const idm = mi[2].tool_calls[0].id;
  prueba('Mistral: id de 9 caracteres, el mismo en la llamada y en el resultado', /^[a-zA-Z0-9]{9}$/.test(idm) && mi[3].tool_call_id === idm, mi);
  prueba('Mistral: nunca usuario justo después de tool', mi[4].role === 'assistant' && mi[5].role === 'user', mi.map((x) => x.role));
  prueba('id9 estable', id9('toolu_01XYZ') === id9('toolu_01XYZ') && id9('abcDEF123') === 'abcDEF123');
}

// SSE: si quien lee se detiene a mitad, se cancela la lectura (y con ella la conexión)
async function sseCancela() {
  let cancelado = false;
  const enc = new TextEncoder();
  const cuerpo = new ReadableStream<Uint8Array>({
    pull(c) {
      c.enqueue(enc.encode('data: {"x":1}\n\n'));
    },
    cancel() {
      cancelado = true;
    }
  });
  try {
    for await (const d of eventosSSE(cuerpo)) {
      if (d) throw new Error('error a mitad');
    }
  } catch {
    /* esperado */
  }
  await new Promise((r) => setTimeout(r, 10));
  prueba('SSE: cancela la lectura al salir antes', cancelado);
}

// adjuntos: imagen y PDF (bloques de Anthropic) → partes de un mensaje de OpenAI
{
  const h: MensajeApi[] = [
    {
      role: 'user',
      content: [
        { type: 'text', text: '<attached_file name="ej.png" kind="image" />' },
        { type: 'image', source: { type: 'base64', media_type: 'image/png', data: 'AAAA' } },
        { type: 'document', source: { type: 'base64', media_type: 'application/pdf', data: 'JVBE' }, title: 'taller.pdf' },
        { type: 'text', text: 'Resuélvelo' }
      ]
    }
  ];
  const m: any[] = aOpenAI(h, 'S');
  const c = m[1].content;
  prueba('adjuntos: el mensaje lleva partes', Array.isArray(c) && c.length === 3, m[1]);
  prueba('adjuntos: primero el texto', c[0].type === 'text' && c[0].text.includes('Resuélvelo') && c[0].text.includes('ej.png'), c[0]);
  prueba('adjuntos: imagen como image_url con data URL', c[1].type === 'image_url' && c[1].image_url.url === 'data:image/png;base64,AAAA', c[1]);
  prueba('adjuntos: PDF como file con nombre', c[2].type === 'file' && c[2].file.filename === 'taller.pdf' && c[2].file.file_data === 'data:application/pdf;base64,JVBE', c[2]);
  // sin adjuntos el contenido sigue siendo texto plano (los proveedores sencillos lo prefieren)
  const m2: any[] = aOpenAI([{ role: 'user', content: [{ type: 'text', text: 'hola' }] }], 'S');
  prueba('sin adjuntos: contenido de texto', m2[1].content === 'hola', m2[1]);
}

await sse();
await sseCancela();
console.log(`\n${ok} bien, ${mal} fallas`);
if (mal) process.exit(1);
