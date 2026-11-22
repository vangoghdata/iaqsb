// Prueba de punta a punta (modo navegador) del asistente con distintos proveedores,
// contra servidores simulados (sin claves reales):
// - una API compatible con OpenAI (como OpenRouter, OpenAI, Gemini, Qwen, Ollama…):
//   streaming con comentarios, llamadas a herramientas armadas por partes, lista de
//   modelos, prueba de conexión, modelo sin herramientas y clave equivocada;
// - la API de mensajes de Anthropic (Claude).
//
// Uso:
//   npm run build
//   python3 tools/dev_server.py ../../target/debug/iaqsb 1491 &
//   node tools/test_ia_ui.mjs [http://127.0.0.1:1491] [carpeta de salida]
//
// Necesita Playwright (npm i -D playwright, o PLAYWRIGHT_PATH=/ruta/a/playwright).

import { createRequire } from 'node:module';
import { mkdirSync } from 'node:fs';
import { join } from 'node:path';
import http from 'node:http';

const require = createRequire(import.meta.url);
let pw;
try {
  pw = require('playwright');
} catch {
  pw = require(process.env.PLAYWRIGHT_PATH ?? '/opt/npm-tools/node_modules/playwright');
}
const URL_APP = process.argv[2] ?? 'http://127.0.0.1:1491/';
const OUT = process.argv[3] ?? join(process.cwd(), 'test-ia-salida');
mkdirSync(OUT, { recursive: true });

let fallas = 0;
let bien = 0;
function ok(cond, que) {
  if (cond) bien++;
  else {
    fallas++;
    console.error('  FALLA: ' + que);
  }
}

// ---------------------------------------------------------------- servidor simulado
const CLAVE = 'clave-prueba';
const pedidos = [];
const cortes = [];
const sse = (res) => {
  res.writeHead(200, { 'content-type': 'text/event-stream', 'access-control-allow-origin': '*' });
};
const dormir = (ms) => new Promise((r) => setTimeout(r, ms));
async function enviar(res, eventos) {
  for (const e of eventos) {
    res.write(e);
    await dormir(15);
  }
  res.end();
}
const chunk = (delta, finish = null) => `data: ${JSON.stringify({ id: 'x', object: 'chat.completion.chunk', choices: [{ index: 0, delta, finish_reason: finish }] })}\n\n`;

const servidor = http.createServer(async (req, res) => {
  const cors = { 'access-control-allow-origin': '*', 'access-control-allow-headers': '*', 'access-control-allow-methods': 'GET, POST, OPTIONS' };
  if (req.method === 'OPTIONS') {
    res.writeHead(204, cors);
    return res.end();
  }
  let cuerpo = '';
  for await (const c of req) cuerpo += c;
  const json = (status, v) => {
    res.writeHead(status, { 'content-type': 'application/json', ...cors });
    res.end(JSON.stringify(v));
  };
  const b = cuerpo ? JSON.parse(cuerpo) : null;
  pedidos.push({ url: req.url, auth: req.headers.authorization ?? req.headers['x-api-key'] ?? '', cuerpo: b });

  // --- API de mensajes de Anthropic
  if (req.url === '/anthropic/v1/messages') {
    if (req.headers['x-api-key'] !== CLAVE) return json(401, { type: 'error', error: { type: 'authentication_error', message: 'invalid x-api-key' } });
    sse(res);
    const ev = (tipo, d) => `event: ${tipo}\ndata: ${JSON.stringify({ type: tipo, ...d })}\n\n`;
    return enviar(res, [
      ev('message_start', { message: { id: 'm', role: 'assistant', content: [] } }),
      ev('content_block_start', { index: 0, content_block: { type: 'text', text: '' } }),
      ev('content_block_delta', { index: 0, delta: { type: 'text_delta', text: 'Hola desde ' } }),
      ev('content_block_delta', { index: 0, delta: { type: 'text_delta', text: 'CLAUDE-SIMULADO.' } }),
      ev('content_block_stop', { index: 0 }),
      ev('message_delta', { delta: { stop_reason: 'end_turn' } }),
      ev('message_stop', {})
    ]);
  }

  // --- API compatible con OpenAI
  if (req.headers.authorization !== `Bearer ${CLAVE}`) return json(401, { error: { message: 'Incorrect API key provided', type: 'invalid_request_error', code: 'invalid_api_key' } });
  if (req.url === '/v1/models') {
    return json(200, {
      data: [
        { id: 'mock/herramientas', name: 'Simulado con herramientas', supported_parameters: ['tools', 'max_tokens'] },
        { id: 'mock/sin-herramientas', name: 'Simulado sin herramientas', supported_parameters: ['max_tokens'] }
      ]
    });
  }
  if (req.url !== '/v1/chat/completions') return json(404, { error: { message: 'no existe' } });
  // prueba de conexión (sin streaming)
  if (b.stream === false && !b.tools) {
    return json(200, { choices: [{ index: 0, message: { role: 'assistant', content: 'ok' }, finish_reason: 'stop' }] });
  }
  if (b.model === 'mock/sin-herramientas') {
    if (b.tools) return json(404, { error: { message: 'No endpoints found that support tool use. To learn more about provider routing, visit: https://openrouter.ai/docs/provider-routing', code: 404 } });
    sse(res);
    return enviar(res, [chunk({ role: 'assistant', content: '' }), chunk({ content: 'Respuesta ' }), chunk({ content: 'SIN-HERRAMIENTAS.' }), chunk({}, 'stop'), 'data: [DONE]\n\n']);
  }
  // como OpenAI: una lista sin «items» en el esquema de una herramienta es un error
  const sinItems = (x) => x && typeof x === 'object' && ((x.type === 'array' && !x.items) || Object.values(x.properties ?? {}).some(sinItems) || (x.items && sinItems(x.items)));
  const mala = (b.tools ?? []).find((t) => sinItems(t.function?.parameters));
  if (mala) return json(400, { error: { message: `Invalid schema for function '${mala.function.name}': array schema missing items.`, type: 'invalid_request_error' } });
  // una llamada sin su resultado en el historial (como OpenAI)
  const msgs = b.messages ?? [];
  for (let i = 0; i < msgs.length; i++) {
    const m = msgs[i];
    if (m.role === 'assistant' && m.tool_calls?.length) {
      const ids = new Set(msgs.slice(i + 1).filter((x) => x.role === 'tool').map((x) => x.tool_call_id));
      if (m.tool_calls.some((tc) => !ids.has(tc.id))) return json(400, { error: { message: "An assistant message with 'tool_calls' must be followed by tool messages responding to each 'tool_call_id'." } });
    }
  }
  if (b.model === 'mock/cortado') {
    sse(res);
    if (msgs.at(-1)?.content === 'cortar')
      return enviar(res, [chunk({ tool_calls: [{ index: 0, id: 'call_cut', function: { name: 'solve', arguments: '{"module":"lp","mod' } }] }), chunk({}, 'length'), 'data: [DONE]\n\n']);
    return enviar(res, [chunk({ content: 'SIGO-SIN-ERROR.' }), chunk({}, 'stop'), 'data: [DONE]\n\n']);
  }
  if (b.model === 'mock/bucle') {
    // un modelo que nunca deja de pedir herramientas (no respeta tool_choice)
    sse(res);
    if (!b.tools) return enviar(res, [chunk({ content: 'FINAL-SIN-HERRAMIENTAS.' }), chunk({}, 'stop'), 'data: [DONE]\n\n']);
    return enviar(res, [chunk({ tool_calls: [{ index: 0, id: 'call_' + msgs.length, function: { name: 'list_modules', arguments: '{}' } }] }), chunk({}, 'tool_calls'), 'data: [DONE]\n\n']);
  }
  if (b.model === 'mock/lento') {
    // respuesta larga: se detiene desde la app
    sse(res);
    let cerrada = false;
    res.on('close', () => {
      cerrada = true;
      cortes.push(Date.now());
    });
    for (let i = 0; i < 40 && !cerrada; i++) {
      res.write(chunk({ content: `parte${i} ` }));
      await dormir(150);
    }
    if (!cerrada) res.end();
    return;
  }
  const ultimo = b.messages.at(-1);
  sse(res);
  if (ultimo.role === 'tool') {
    const n = (() => {
      try {
        return JSON.parse(ultimo.content).length;
      } catch {
        return (ultimo.content.match(/\n- /g) ?? []).length || '?';
      }
    })();
    return enviar(res, [chunk({ content: 'Revisé los módulos ' }), chunk({ content: `(${n}). TERMINADO.` }), chunk({}, 'stop'), 'data: [DONE]\n\n']);
  }
  // primera ronda: texto y una llamada a list_modules con los argumentos por partes
  return enviar(res, [
    ': OPENROUTER PROCESSING\n\n',
    chunk({ role: 'assistant', content: 'Voy a consultar los módulos.' }),
    chunk({ tool_calls: [{ index: 0, id: 'call_abc123', type: 'function', function: { name: 'list_modules', arguments: '' } }] }),
    chunk({ tool_calls: [{ index: 0, function: { arguments: '{' } }] }),
    chunk({ tool_calls: [{ index: 0, function: { arguments: '}' } }] }),
    chunk({}, 'tool_calls'),
    'data: [DONE]\n\n'
  ]);
});
await new Promise((r) => servidor.listen(0, '127.0.0.1', r));
const BASE = `http://127.0.0.1:${servidor.address().port}`;

// ---------------------------------------------------------------- navegador
const browser = await pw.chromium.launch();

/**
 * Simula la app de escritorio dentro del navegador: window.__TAURI_INTERNALS__ con
 * invoke y los canales de Tauri. ia_fetch hace la petición y manda los trozos por el
 * canal (cabecera, datos, fin, error) como lo hace red.rs; api_call va al servidor de
 * desarrollo. Así se prueba red.ts con el Channel real de @tauri-apps/api.
 */
function simularTauri() {
  const cbs = new Map();
  let n = 1;
  const cancelar = {};
  window.__TAURI_INTERNALS__ = {
    transformCallback(cb) {
      const id = n++;
      cbs.set(id, cb);
      return id;
    },
    unregisterCallback(id) {
      cbs.delete(id);
    },
    async invoke(cmd, args) {
      if (cmd === 'api_call') {
        const r = await fetch('/api/call', { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ op: args.op, args: args.args }) });
        const j = await r.json();
        if (!j.ok) throw j.error?.message ?? 'error';
        return j.result;
      }
      if (cmd === 'initial_file') return null;
      if (cmd === 'file_exists') return false;
      if (cmd === 'ia_cancel') {
        cancelar[args.id]?.abort();
        return null;
      }
      if (cmd === 'ia_fetch') {
        sessionStorage.setItem('pedidosIA', String(Number(sessionStorage.getItem('pedidosIA') ?? 0) + 1));
        const cb = cbs.get(args.canal.id);
        let i = 0;
        const enviar = (m) => cb({ index: i++, message: m });
        const ac = new AbortController();
        cancelar[args.id] = ac;
        (async () => {
          try {
            const r = await fetch(args.url, { method: args.metodo, headers: Object.fromEntries(args.cabeceras), body: args.cuerpo ?? undefined, signal: ac.signal });
            enviar({ t: 'cabecera', status: r.status, headers: [...r.headers] });
            const rd = r.body.getReader();
            const dec = new TextDecoder();
            for (;;) {
              const { done, value } = await rd.read();
              if (done) break;
              enviar({ t: 'datos', b: dec.decode(value, { stream: true }) });
            }
            enviar({ t: 'fin' });
          } catch (e) {
            if (!ac.signal.aborted) enviar({ t: 'error', m: String(e) });
          }
        })();
        return null;
      }
      throw new Error('comando no simulado: ' + cmd);
    }
  };
}

for (const modo of ['navegador', 'escritorio (Tauri simulado)']) {
console.log(`\n=== ${modo}`);
pedidos.length = 0;
const ctx = await browser.newContext({ viewport: { width: 1400, height: 900 }, locale: 'es-CO' });
const page = await ctx.newPage();
const errores = [];
page.on('pageerror', (e) => errores.push(e.message));
if (modo !== 'navegador') await page.addInitScript(simularTauri);
await page.addInitScript(
  ({ base, clave }) => {
    if (sessionStorage.getItem('iniciado')) return;
    sessionStorage.setItem('iniciado', '1');
    localStorage.clear();
    localStorage.setItem('iaqsb_menu_fijo', '1');
    localStorage.setItem('iaqsb_idioma', 'es');
    localStorage.setItem(
      'iaqsb.ajustes',
      JSON.stringify({ proveedor: 'personalizado', urls: { personalizado: base + '/v1' }, claves: { personalizado: clave }, modelos: { personalizado: 'mock/herramientas' } })
    );
  },
  { base: BASE, clave: CLAVE }
);
await page.goto(URL_APP);
await page.waitForSelector('[data-r="1"][data-c="1"]');

async function preguntar(texto) {
  const caja = page.locator('aside.chat textarea');
  await caja.fill(texto);
  await caja.press('Enter');
}
async function esperarTexto(texto, ms = 15000) {
  await page.locator('aside.chat .lista').getByText(texto).first().waitFor({ timeout: ms });
}

console.log('· API compatible con OpenAI, con herramientas');
await page.getByRole('button', { name: 'Asistente' }).first().click();
await page.waitForSelector('aside.chat');
ok(await page.locator('aside.chat .modelo').innerText().then((s) => s.includes('mock/herramientas')), 'el panel muestra el proveedor y el modelo');
await preguntar('¿Qué módulos hay?');
await esperarTexto('TERMINADO', 20000).catch(() => {});
const respuesta = await page.locator('aside.chat .asistente').last().innerText();
ok(respuesta.includes('Voy a consultar los módulos.') && respuesta.includes('TERMINADO'), 'texto antes y después de la herramienta: ' + respuesta.slice(0, 200));
ok((await page.locator('aside.chat details.herr').count()) === 1, 'se ve la tarjeta de la herramienta');
const chats = pedidos.filter((p) => p.url === '/v1/chat/completions');
ok(chats.length === 2, `dos rondas (${chats.length})`);
const r1 = chats[0]?.cuerpo;
ok(r1?.messages?.[0]?.role === 'system' && r1.stream === true && Array.isArray(r1.tools) && r1.tools[0].type === 'function', 'primera ronda: sistema, streaming y herramientas en formato OpenAI');
ok(r1?.max_tokens > 0, 'max_tokens');
const r2 = chats[1]?.cuerpo?.messages ?? [];
const llamada = r2.find((m) => m.role === 'assistant' && m.tool_calls);
const resultado = r2.find((m) => m.role === 'tool');
ok(llamada?.tool_calls?.[0]?.id === 'call_abc123' && resultado?.tool_call_id === 'call_abc123', 'segunda ronda: la llamada y su resultado con el mismo id');
ok(resultado?.content?.length > 20, 'el resultado de list_modules (motor local) va en el mensaje tool');
await page.screenshot({ path: join(OUT, `${modo[0]}_1_chat_openai.png`) });

console.log('· Ajustes → Conectar la API: modelos y prueba de conexión');
await page.locator('aside.chat .modelo').click();
await page.waitForSelector('.capa .conexion');
await page.locator('.conexion').getByRole('button', { name: 'Modelos', exact: true }).click();
await page.locator('.conexion').getByText('modelos disponibles').waitFor({ timeout: 10000 });
const opciones = await page.locator('.conexion datalist option').evaluateAll((os) => os.map((o) => o.value));
ok(opciones.includes('mock/herramientas') && !opciones.includes('mock/sin-herramientas'), 'lista de modelos (solo con herramientas): ' + opciones.join(', '));
await page.locator('.conexion .chk input').uncheck();
const todas = await page.locator('.conexion datalist option').evaluateAll((os) => os.map((o) => o.value));
ok(todas.includes('mock/sin-herramientas'), 'lista completa al quitar el filtro');
await page.getByRole('button', { name: 'Probar la conexión' }).click();
await page.locator('.conexion .resultado.ok').waitFor({ timeout: 10000 });
ok((await page.locator('.conexion .resultado').innerText()).includes('Conectado'), 'prueba de conexión correcta');
await page.screenshot({ path: join(OUT, `${modo[0]}_2_ajustes_conexion.png`) });

console.log('· Modelo sin herramientas');
const modelo = page.locator('.conexion input[list]');
await modelo.fill('mock/sin-herramientas');
ok(await page.locator('.conexion .nota.mal').count().then((n) => n > 0), 'aviso: el modelo no usa herramientas');
await page.keyboard.press('Escape');
await page.locator('aside.chat button[title="Conversación nueva"]').click();
await preguntar('Hola');
await esperarTexto('SIN-HERRAMIENTAS', 15000).catch(() => {});
const r3 = await page.locator('aside.chat .asistente').last().innerText();
ok(r3.includes('SIN-HERRAMIENTAS') && r3.includes('no admite herramientas'), 'responde sin herramientas y lo avisa: ' + r3.slice(0, 200));
const ultimo = pedidos.filter((p) => p.url === '/v1/chat/completions').at(-1)?.cuerpo;
ok(ultimo && !ultimo.tools, 'el reintento va sin herramientas');

console.log('· Detener una respuesta');
await page.evaluate(() => {
  const a = JSON.parse(localStorage.getItem('iaqsb.ajustes'));
  a.modelos.personalizado = 'mock/lento';
  localStorage.setItem('iaqsb.ajustes', JSON.stringify(a));
});
await page.reload();
await page.waitForSelector('[data-r="1"][data-c="1"]');
await page.getByRole('button', { name: 'Asistente' }).first().click();
await page.locator('aside.chat button[title="Conversación nueva"]').click().catch(() => {});
const antes = cortes.length;
await preguntar('Escribe mucho');
await esperarTexto('parte2', 10000).catch(() => {});
await page.locator('aside.chat button[title="Detener"]').click();
await page.locator('aside.chat').getByText('Respuesta detenida.').waitFor({ timeout: 5000 }).catch(() => {});
ok(await page.locator('aside.chat').getByText('Respuesta detenida.').isVisible(), 'la respuesta se detiene');
await dormir(400);
ok(cortes.length > antes, 'la conexión con el proveedor se cerró al detener');
ok(!(await page.locator('aside.chat .asistente').last().innerText()).includes('parte39'), 'no siguió escribiendo');

console.log('· Llamada cortada por el largo y después «continúa»');
async function usarModelo(m) {
  await page.evaluate((m) => {
    const a = JSON.parse(localStorage.getItem('iaqsb.ajustes'));
    a.modelos.personalizado = m;
    a.claves.personalizado = 'clave-prueba';
    localStorage.setItem('iaqsb.ajustes', JSON.stringify(a));
  }, m);
  await page.reload();
  await page.waitForSelector('[data-r="1"][data-c="1"]');
  await page.getByRole('button', { name: 'Asistente' }).first().click();
  await page.locator('aside.chat button[title="Conversación nueva"]').click().catch(() => {});
}
await usarModelo('mock/cortado');
await preguntar('cortar');
await page.locator('aside.chat').getByText('se cortó por su longitud').waitFor({ timeout: 10000 }).catch(() => {});
await preguntar('continúa');
await esperarTexto('SIGO-SIN-ERROR', 10000).catch(() => {});
ok(await page.locator('aside.chat').getByText('SIGO-SIN-ERROR').isVisible(), 'la conversación sigue después de una llamada cortada');
ok(!(await page.locator('aside.chat .error').count()), 'sin error de historial');

console.log('· Modelo que no deja de pedir herramientas');
await usarModelo('mock/bucle');
const antesBucle = pedidos.filter((p) => p.url === '/v1/chat/completions').length;
await preguntar('hola');
await esperarTexto('FINAL-SIN-HERRAMIENTAS', 30000).catch(() => {});
const rondas = pedidos.filter((p) => p.url === '/v1/chat/completions').length - antesBucle;
ok(await page.locator('aside.chat').getByText('FINAL-SIN-HERRAMIENTAS').isVisible(), 'termina con una respuesta sin herramientas');
ok(rondas === 9, `rondas limitadas (${rondas})`);

console.log('· Clave equivocada');
await page.evaluate(() => {
  const a = JSON.parse(localStorage.getItem('iaqsb.ajustes'));
  a.claves.personalizado = 'otra';
  a.modelos.personalizado = 'mock/herramientas';
  localStorage.setItem('iaqsb.ajustes', JSON.stringify(a));
});
await page.reload();
await page.waitForSelector('[data-r="1"][data-c="1"]');
await page.getByRole('button', { name: 'Asistente' }).first().click();
await page.locator('aside.chat button[title="Conversación nueva"]').click().catch(() => {});
await preguntar('Hola');
await page.locator('aside.chat .error').waitFor({ timeout: 10000 }).catch(() => {});
const err = await page.locator('aside.chat .error').innerText().catch(() => '');
ok(err.includes('no aceptó la clave'), 'error de clave claro: ' + err);

console.log('· API de mensajes de Anthropic (Claude)');
await page.evaluate(
  ({ base, clave }) => {
    const a = JSON.parse(localStorage.getItem('iaqsb.ajustes'));
    a.proveedor = 'anthropic';
    a.urls = { ...a.urls, anthropic: base + '/anthropic/v1' };
    a.claves = { ...a.claves, anthropic: clave };
    localStorage.setItem('iaqsb.ajustes', JSON.stringify(a));
  },
  { base: BASE, clave: CLAVE }
);
await page.reload();
await page.waitForSelector('[data-r="1"][data-c="1"]');
await page.getByRole('button', { name: 'Asistente' }).first().click();
await page.locator('aside.chat button[title="Conversación nueva"]').click().catch(() => {});
await preguntar('Hola');
await esperarTexto('CLAUDE-SIMULADO', 15000).catch(() => {});
ok((await page.locator('aside.chat .asistente').last().innerText().catch(() => '')).includes('CLAUDE-SIMULADO'), 'responde por la API de Anthropic');
const pa = pedidos.filter((p) => p.url === '/anthropic/v1/messages').at(-1)?.cuerpo;
ok(pa?.model === 'claude-sonnet-5-5' && Array.isArray(pa.tools) && pa.tools[0].input_schema, 'pedido en formato de Anthropic');
await page.screenshot({ path: join(OUT, `${modo[0]}_3_chat_anthropic.png`) });

console.log('· Conectar escribiendo letra por letra');
await page.evaluate(() => {
  const a = JSON.parse(localStorage.getItem('iaqsb.ajustes'));
  a.proveedor = 'openrouter';
  a.claves = {};
  localStorage.setItem('iaqsb.ajustes', JSON.stringify(a));
});
await page.reload();
await page.waitForSelector('[data-r="1"][data-c="1"]');
await page.getByRole('button', { name: 'Asistente' }).first().click();
ok(await page.locator('aside.chat .clave .conexion').isVisible(), 'el panel ofrece conectar un modelo');
await page.locator('aside.chat .conexion input[type="password"]').pressSequentially('sk-or-abc', { delay: 10 });
await page.locator('aside.chat .conexion input[list]').pressSequentially('qwen/qwen3-max', { delay: 10 });
ok(await page.locator('aside.chat .clave .conexion').isVisible(), 'la tarjeta sigue a la vista mientras se escribe');
const guardado = await page.evaluate(() => JSON.parse(localStorage.getItem('iaqsb.ajustes')));
ok(guardado.modelos.openrouter === 'qwen/qwen3-max' && guardado.claves.openrouter === 'sk-or-abc', 'se guardó lo escrito completo: ' + JSON.stringify(guardado.modelos));
await page.getByRole('button', { name: 'Listo, empezar a conversar' }).click();
ok(!(await page.locator('aside.chat .clave').count()), 'la tarjeta se cierra con «Listo»');
await page.screenshot({ path: join(OUT, `${modo[0]}_4_sin_conectar.png`) });

ok(!errores.length, 'sin errores en la página: ' + errores.join(' | '));
if (modo !== 'navegador') ok((await page.evaluate(() => Number(sessionStorage.getItem('pedidosIA') ?? 0))) >= 6, 'las peticiones pasaron por ia_fetch (Rust)');
await ctx.close();
}
await browser.close();
servidor.close();
console.log(`${bien} bien, ${fallas} fallas · capturas en ${OUT}`);
process.exit(fallas ? 1 : 0);
