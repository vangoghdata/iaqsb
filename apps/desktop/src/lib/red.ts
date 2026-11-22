// fetch para el asistente de IA.
//
// En la aplicación de escritorio la petición la hace Rust (comando ia_fetch): así se
// puede hablar con cualquier proveedor —OpenRouter, OpenAI, Gemini, Qwen, un servidor
// local como Ollama— sin chocar con la política de seguridad de la ventana ni con
// CORS. La respuesta llega por partes y se entrega como un Response normal, con su
// cuerpo en streaming. En el navegador (modo de desarrollo) se usa el fetch de siempre.

import { inTauri } from './backend';

type Trozo =
  | { t: 'cabecera'; status: number; headers: [string, string][] }
  | { t: 'datos'; b: string }
  | { t: 'fin' }
  | { t: 'error'; m: string };

// ids distintos aunque la ventana se recargue (Rust guarda las peticiones en curso por id)
let siguiente = 1 + Math.floor(Math.random() * 0x7fff0000);

function abortado(): DOMException {
  return new DOMException('abort', 'AbortError');
}

export interface PedidoIA {
  method?: string;
  headers?: Record<string, string>;
  body?: string;
  signal?: AbortSignal;
}

export async function fetchIA(url: string, init: PedidoIA = {}): Promise<Response> {
  if (!inTauri) return fetch(url, { method: init.method ?? 'POST', headers: init.headers, body: init.body, signal: init.signal });
  const { Channel, invoke } = await import('@tauri-apps/api/core');
  const signal = init.signal;
  if (signal?.aborted) throw abortado();
  const id = siguiente++;
  const enc = new TextEncoder();
  let ctrl: ReadableStreamDefaultController<Uint8Array> | null = null;
  let cerrado = false;
  const cuerpo = new ReadableStream<Uint8Array>({
    start(c) {
      ctrl = c;
    },
    cancel() {
      cerrado = true;
      void invoke('ia_cancel', { id });
    }
  });
  return new Promise<Response>((resolver, rechazar) => {
    let resuelto = false;
    const alAbortar = () => {
      void invoke('ia_cancel', { id });
      if (!resuelto) rechazar(abortado());
      else if (!cerrado) {
        cerrado = true;
        ctrl?.error(abortado());
      }
    };
    signal?.addEventListener('abort', alAbortar, { once: true });
    const canal = new Channel<Trozo>();
    canal.onmessage = (m) => {
      if (m.t === 'cabecera') {
        resuelto = true;
        // un Response no admite cuerpo con 204/205/304
        const sinCuerpo = m.status === 204 || m.status === 205 || m.status === 304;
        resolver(new Response(sinCuerpo ? null : cuerpo, { status: m.status, headers: m.headers }));
      } else if (m.t === 'datos') {
        if (!cerrado) ctrl?.enqueue(enc.encode(m.b));
      } else if (m.t === 'fin') {
        signal?.removeEventListener('abort', alAbortar);
        if (!cerrado) {
          cerrado = true;
          ctrl?.close();
        }
      } else if (m.t === 'error') {
        signal?.removeEventListener('abort', alAbortar);
        // como el fetch del navegador: TypeError si no hubo respuesta
        if (!resuelto) rechazar(new TypeError(m.m));
        else if (!cerrado) {
          cerrado = true;
          ctrl?.error(new TypeError(m.m));
        }
      }
    };
    const cabeceras = Object.entries(init.headers ?? {});
    invoke('ia_fetch', { id, url, metodo: init.method ?? 'POST', cabeceras, cuerpo: init.body ?? null, canal }).catch((e) => {
      signal?.removeEventListener('abort', alAbortar);
      if (!resuelto) rechazar(new TypeError(String(e)));
    });
  });
}
