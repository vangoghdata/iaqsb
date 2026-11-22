<script lang="ts">
  // Panel lateral del asistente de IA (el modelo que se conecte en Ajustes): mensajes con Markdown, herramientas que
  // corren con el motor local, confirmaciones, errores y el cuadro para escribir.
  import Icon from '../Icon.svelte';
  import Markdown from './Markdown.svelte';
  import { chat, etiquetaHerramienta, type Mensaje, type Parte } from '../../lib/chat.svelte';
  import ConexionIA from '../ajustes/ConexionIA.svelte';
  import { ajustes } from '../../lib/ajustes.svelte';
  import { shell } from '../../lib/shell.svelte';
  import { app } from '../../lib/state.svelte';
  import { ui } from '../../lib/ui.svelte';
  import { t } from '../../lib/i18n.svelte';
  import { tick } from 'svelte';
  import { ACEPTA, tamano } from '../../lib/adjuntos';

  let lista: HTMLElement | undefined = $state();
  let entrada: HTMLTextAreaElement | undefined = $state();
  let pegado = true;
  let selector: HTMLInputElement | undefined = $state();
  let soltando = $state(false);

  // adjuntar: botón, arrastrar y soltar, o pegar una captura con Ctrl+V
  function alElegir(e: Event) {
    const inp = e.currentTarget as HTMLInputElement;
    if (inp.files?.length) chat.adjuntarArchivos([...inp.files]);
    inp.value = '';
    entrada?.focus();
  }
  function alSoltar(e: DragEvent) {
    e.preventDefault();
    soltando = false;
    if (e.dataTransfer?.files?.length) chat.adjuntarArchivos([...e.dataTransfer.files]);
  }
  function alArrastrar(e: DragEvent) {
    if (!e.dataTransfer?.types.includes('Files')) return;
    e.preventDefault();
    soltando = true;
  }
  function alPegar(e: ClipboardEvent) {
    const items = [...(e.clipboardData?.items ?? [])];
    const imagenes = items.filter((i) => i.kind === 'file' && i.type.startsWith('image/'));
    const archivos = items.filter((i) => i.kind === 'file' && !i.type.startsWith('image/'));
    if (!imagenes.length && !archivos.length) return;
    e.preventDefault();
    for (const i of imagenes) {
      const b = i.getAsFile();
      if (b) chat.adjuntarPegada(b);
    }
    const fs = archivos.map((i) => i.getAsFile()).filter((f): f is File => !!f);
    if (fs.length) chat.adjuntarArchivos(fs);
  }
  const textoUsuario = (m: Mensaje) => (m.partes.find((p) => p.tipo === 'texto') as { texto: string } | undefined)?.texto ?? '';
  const adjuntosDe = (m: Mensaje) => (m.partes.find((p) => p.tipo === 'adjuntos') as Extract<Parte, { tipo: 'adjuntos' }> | undefined)?.items ?? [];
  const resuelto = $derived(shell.module === 'lp' ? !!app.report : !!shell.get(shell.module).report);
  const sugerencias = $derived(
    [
      resuelto ? t('Explícame el resultado del problema abierto') : t('¿Cómo planteo el problema que tengo abierto?'),
      t('¿Qué módulo uso para decidir cuántas unidades pedir de cada producto?'),
      t('Plantea un problema de transporte pequeño, resuélvelo y explícamelo'),
      resuelto ? t('¿Qué pasa con el resultado si cambia uno de los datos?') : t('¿Qué diferencia hay entre el símplex y branch and bound?')
    ]
  );

  // los avisos de la app se corren a la izquierda del panel (ver el estilo global de abajo)
  $effect(() => {
    document.documentElement.style.setProperty('--chat-abierto-w', chat.abierto ? `${chat.ancho}px` : '0px');
  });

  // enfoca el cuadro de texto cuando se abre el panel o se pide desde otro lado
  $effect(() => {
    chat.foco;
    if (chat.abierto) tick().then(() => entrada?.focus());
  });

  // baja hasta el final mientras llega la respuesta (si el usuario no subió a leer)
  $effect(() => {
    const ult = chat.mensajes.at(-1);
    void chat.mensajes.length;
    void ult?.partes.map((p) => (p.tipo === 'texto' ? p.texto.length : p.tipo === 'herramienta' ? p.estado : p.tipo === 'aviso' ? p.texto : p.items.length)).join();
    void chat.confirmacion;
    void chat.error;
    if (pegado) tick().then(() => lista?.scrollTo({ top: lista.scrollHeight }));
  });

  function alDesplazar() {
    if (!lista) return;
    pegado = lista.scrollHeight - lista.scrollTop - lista.clientHeight < 40;
  }

  // el cuadro de texto crece con lo que se escribe
  $effect(() => {
    void chat.borrador;
    if (!entrada) return;
    entrada.style.height = 'auto';
    entrada.style.height = Math.min(entrada.scrollHeight, 200) + 'px';
  });

  function teclas(e: KeyboardEvent) {
    if (e.key === 'Enter' && !e.shiftKey && !e.isComposing) {
      e.preventDefault();
      enviar();
    }
  }

  function enviar(txt?: string) {
    pegado = true;
    if (chat.tieneClave) conectando = false;
    chat.enviar(txt ?? chat.borrador);
  }

  // la tarjeta para conectar sigue a la vista mientras se escribe la clave o el modelo
  // (no desaparece con la primera letra); se cierra al probar la conexión o al preguntar
  let conectando = $state(!chat.tieneClave);
  $effect(() => {
    if (!chat.tieneClave) conectando = true;
  });

  function conectado() {
    conectando = false;
    ui.avisar(t('Asistente conectado: {0}', ajustes.iaNombre), 'ok');
    tick().then(() => entrada?.focus());
  }

  /** ¿Ya se ve algo en marcha (texto o una herramienta)? Si no, se muestran los puntos de «pensando». */
  function escribiendo(m: Mensaje): boolean {
    return m.partes.some((p) => p.tipo === 'texto' || (p.tipo === 'herramienta' && (p.estado === 'preparando' || p.estado === 'corriendo')));
  }

  function textoDe(m: Mensaje): string {
    return m.partes
      .filter((p): p is Extract<Parte, { tipo: 'texto' }> => p.tipo === 'texto')
      .map((p) => p.texto)
      .join('\n\n');
  }

  async function copiar(m: Mensaje) {
    try {
      await navigator.clipboard.writeText(textoDe(m));
      ui.avisar(t('Respuesta copiada'), 'ok');
    } catch {
      ui.avisar(t('No se pudo copiar al portapapeles'), 'error');
    }
  }

  // cambiar el ancho arrastrando el borde izquierdo
  function arrastrar(e: PointerEvent) {
    e.preventDefault();
    const el = e.currentTarget as HTMLElement;
    el.setPointerCapture(e.pointerId);
    const mover = (ev: PointerEvent) => chat.fijarAncho(Math.min(window.innerWidth - 360, window.innerWidth - ev.clientX));
    const soltar = () => {
      el.removeEventListener('pointermove', mover);
      el.removeEventListener('pointerup', soltar);
    };
    el.addEventListener('pointermove', mover);
    el.addEventListener('pointerup', soltar);
  }

  function teclasBorde(e: KeyboardEvent) {
    if (e.key === 'ArrowLeft') chat.fijarAncho(chat.ancho + 24);
    else if (e.key === 'ArrowRight') chat.fijarAncho(chat.ancho - 24);
  }
</script>

{#if chat.abierto}
  <aside class="chat" style:--chat-w="{chat.ancho}px" aria-label={t('Asistente de IA')}>
    <!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
    <div
      class="borde"
      role="separator"
      aria-orientation="vertical"
      aria-label={t('Cambiar el ancho del panel')}
      aria-valuenow={chat.ancho}
      tabindex="0"
      onpointerdown={arrastrar}
      onkeydown={teclasBorde}
    ></div>

    <header class="cab">
      <span class="logo"><Icon name="chispa" size={15} /></span>
      <div class="ctit">
        <strong>{t('Asistente IA')}</strong>
        <button class="modelo" title={t('Cambiar el proveedor o el modelo')} onclick={() => ajustes.abrir('ia')}>
          {ajustes.iaLista ? ajustes.iaNombre : t('Sin conectar')}<Icon name="abajo" size={11} />
        </button>
      </div>
      <button class="btn ghost icon" title={t('Conversación nueva')} aria-label={t('Conversación nueva')} onclick={() => chat.nueva()} disabled={!chat.mensajes.length && !chat.adjunto}>
        <Icon name="nuevo" size={16} />
      </button>
      <button class="btn ghost icon" title={t('Ajustes de IA')} aria-label={t('Ajustes de IA')} onclick={() => ajustes.abrir('ia')}>
        <Icon name="ajustes" size={16} />
      </button>
      <button class="btn ghost icon" title={t('Cerrar el panel')} aria-label={t('Cerrar el panel')} onclick={() => chat.cerrar()}>
        <Icon name="cerrar" size={16} />
      </button>
    </header>

    <div class="lista" bind:this={lista} onscroll={alDesplazar} aria-live="polite">
      {#if conectando}
        <section class="clave card">
          <h3><Icon name="chispa" size={16} />{t('Conecta un modelo de IA')}</h3>
          <p>{t('Elige el proveedor y el modelo: Claude, ChatGPT, Gemini, Qwen, DeepSeek, Llama y cientos más (con OpenRouter), o un modelo gratis en tu propio equipo con Ollama o LM Studio.')}</p>
          <ConexionIA compacto alConectar={conectado} />
          {#if chat.tieneClave}<button class="btn primary listo" onclick={() => { conectando = false; tick().then(() => entrada?.focus()); }}>{t('Listo, empezar a conversar')}</button>{/if}
          <p class="muted peq">{t('¿Sin clave? Usa «Explicar con IA → Copiar para otra IA» en la solución y pégalo en ChatGPT, Gemini o Claude.ai.')}</p>
        </section>
      {/if}

      {#if !chat.mensajes.length && chat.tieneClave && !conectando}
        <div class="vacio">
          <span class="grande"><Icon name="chispa" size={26} stroke={1.5} /></span>
          <h3>{t('¿En qué te ayudo?')}</h3>
          <p class="muted">{t('Pregunta por el problema abierto, pide que plantee uno nuevo o que compare métodos. Con los modelos que usan herramientas, el asistente resuelve con el motor de IAQSB, así que los números son los del programa.')}</p>
          <div class="sug">
            {#each sugerencias as s (s)}
              <button class="opc" onclick={() => enviar(s)}>{s}</button>
            {/each}
          </div>
        </div>
      {/if}

      {#each chat.mensajes as m (m.id)}
        {#if m.rol === 'usuario'}
          <div class="usuario">
            {#if m.contexto}
              <span class="ctx" title={t('Se adjuntaron el informe y el modelo')}><Icon name="tabla" size={13} />{m.contexto.titulo}</span>
            {/if}
            {#if adjuntosDe(m).length}
              <div class="adjs enviados">
                {#each adjuntosDe(m) as a, k (k)}
                  <span class="adjf" title={a.nombre}>
                    {#if a.vista}<img src={a.vista} alt="" />{:else}<span class="tipo">{a.clase === 'pdf' ? 'PDF' : a.clase === 'modelo' ? (a.modulo ?? '').toUpperCase() : 'TXT'}</span>{/if}
                    <span class="an">{a.nombre}</span>
                  </span>
                {/each}
              </div>
            {/if}
            <div class="burbuja">{textoUsuario(m)}</div>
          </div>
        {:else}
          <div class="asistente">
            {#each m.partes as p, k (k)}
              {#if p.tipo === 'texto'}
                <Markdown texto={p.texto} />
              {:else if p.tipo === 'herramienta'}
                <details class="herr" class:err={p.estado === 'error'}>
                  <summary>
                    {#if p.estado === 'preparando' || p.estado === 'corriendo'}
                      <span class="spinner" aria-hidden="true"></span>
                    {:else if p.estado === 'ok'}
                      <Icon name="motor" size={14} />
                    {:else}
                      <Icon name="alerta" size={14} />
                    {/if}
                    <span class="hl">{etiquetaHerramienta(p.nombre, p.entrada, p.estado)}</span>
                    {#if p.estado === 'cancelada'}<span class="muted">· {t('cancelada')}</span>{/if}
                    {#if p.estado === 'error'}<span class="muted">· {t('error')}</span>{/if}
                    <span class="chev"><Icon name="abajo" size={13} /></span>
                  </summary>
                  <div class="hdet">
                    <span class="hk">{t('Entrada')} · <code>{p.nombre}</code></span>
                    <pre>{p.entrada || '{}'}</pre>
                    {#if p.salida !== undefined}
                      <span class="hk">{t('Resultado')}</span>
                      <pre>{p.salida}</pre>
                    {/if}
                  </div>
                </details>
              {:else if p.tipo === 'aviso'}
                <p class="aviso"><Icon name="info" size={14} />{p.texto}</p>
              {/if}
            {/each}
            {#if chat.ocupado && m === chat.mensajes.at(-1) && !escribiendo(m)}
              <span class="pensando" aria-label={t('El asistente está pensando')}><i></i><i></i><i></i></span>
            {/if}
            {#if m.detenido}<p class="aviso"><Icon name="info" size={14} />{t('Respuesta detenida.')}</p>{/if}
            {#if !(chat.ocupado && m === chat.mensajes.at(-1)) && textoDe(m).trim()}
              <div class="macc">
                <button class="btn ghost mini" onclick={() => copiar(m)} title={t('Copiar la respuesta en Markdown')}><Icon name="copiar" size={13} />{t('Copiar')}</button>
              </div>
            {/if}
          </div>
        {/if}
      {/each}

      {#if chat.confirmacion}
        {@const c = chat.confirmacion}
        <div class="confirmar card" role="alertdialog" aria-labelledby="chat-conf-t">
          <h4 id="chat-conf-t"><Icon name="alerta" size={15} />{c.titulo}</h4>
          <p>{c.texto}</p>
          <div class="acc">
            <button class="btn" onclick={() => c.responder(false)}>{t('Cancelar')}</button>
            <button class="btn primary" onclick={() => c.responder(true)}>{c.aceptar}</button>
          </div>
        </div>
      {/if}

      {#if chat.error}
        <div class="error" role="alert">
          <span><Icon name="alerta" size={15} />{chat.error}</span>
          <div class="acc">
            {#if chat.reintentable && chat.tieneClave}<button class="btn mini" onclick={() => chat.reintentar()}><Icon name="recargar" size={13} />{t('Reintentar')}</button>{/if}
            <button class="btn ghost mini" onclick={() => (chat.error = null)}>{t('Cerrar')}</button>
          </div>
        </div>
      {/if}
    </div>

    <footer class="pie">
      {#if chat.adjunto}
        <div class="adj">
          <Icon name="tabla" size={14} />
          <span class="at" title={t('Se enviarán el informe (en Markdown) y el modelo (JSON) de este problema')}>{t('Adjunto: {0} (informe y modelo)', chat.adjunto.titulo)}</span>
          <button class="btn ghost icon mini" aria-label={t('Quitar el adjunto')} title={t('Quitar el adjunto')} onclick={() => (chat.adjunto = null)}><Icon name="cerrar" size={13} /></button>
        </div>
      {/if}
      {#if chat.adjuntos.length || chat.leyendo}
        <div class="adjs">
          {#each chat.adjuntos as a (a.id)}
            <span class="adjf" title={`${a.nombre} · ${tamano(a.bytes)}`}>
              {#if a.vista}<img src={a.vista} alt="" />{:else}<span class="tipo">{a.clase === 'pdf' ? 'PDF' : a.clase === 'modelo' ? (a.modulo ?? '').toUpperCase() : 'TXT'}</span>{/if}
              <span class="an">{a.nombre}</span>
              <button class="quitar" aria-label={t('Quitar {0}', a.nombre)} title={t('Quitar')} onclick={() => chat.quitarAdjunto(a.id)}><Icon name="cerrar" size={11} /></button>
            </span>
          {/each}
          {#if chat.leyendo}<span class="adjf leyendo"><span class="spinner" aria-hidden="true"></span>{t('Leyendo…')}</span>{/if}
        </div>
      {/if}
      <div class="caja" class:off={!chat.tieneClave} class:soltando ondragover={alArrastrar} ondragleave={() => (soltando = false)} ondrop={alSoltar} role="group" aria-label={t('Mensaje para el asistente')}>
        <input bind:this={selector} type="file" multiple accept={ACEPTA} hidden onchange={alElegir} />
        <button class="btn ghost icon clip" title={t('Adjuntar una foto, captura, PDF, Excel o archivo de WinQSB (también puedes arrastrarlo o pegarlo con Ctrl+V)')} aria-label={t('Adjuntar archivos')} onclick={() => selector?.click()} disabled={!chat.tieneClave}>
          <Icon name="clip" size={16} />
        </button>
        <textarea
          onpaste={alPegar}
          bind:this={entrada}
          bind:value={chat.borrador}
          onkeydown={teclas}
          rows="1"
          placeholder={chat.tieneClave ? t('Pregunta al asistente… (Mayús+Intro: nueva línea)') : t('Primero conecta un modelo de IA')}
          disabled={!chat.tieneClave}
          aria-label={t('Mensaje para el asistente')}
        ></textarea>
        {#if chat.ocupado}
          <button class="btn icon enviar parar" title={t('Detener')} aria-label={t('Detener')} onclick={() => chat.detener()}><span class="cuadro"></span></button>
        {:else}
          <button class="btn primary icon enviar" title={t('Enviar (Intro)')} aria-label={t('Enviar')} onclick={() => enviar()} disabled={!chat.tieneClave || chat.leyendo > 0 || (!chat.borrador.trim() && !chat.adjunto && !chat.adjuntos.length)}>
            <Icon name="derecha" size={16} stroke={2.2} />
          </button>
        {/if}
      </div>
      <p class="legal">{t('La IA puede equivocarse: los números confiables son los que calcula el motor de IAQSB.')}</p>
    </footer>
  </aside>
{/if}

<style>
  :global(.avisos) {
    right: calc(max(0px, min(var(--chat-abierto-w, 0px), 100vw - 420px)) + 20px) !important;
  }
  .chat {
    position: fixed;
    top: 0;
    right: 0;
    bottom: 0;
    z-index: 40;
    width: min(var(--chat-w), 100vw);
    display: grid;
    grid-template-rows: var(--bar-h) minmax(0, 1fr) auto;
    /* una sola columna que no crece con el contenido (nombres de modelos largos) */
    grid-template-columns: minmax(0, 1fr);
    background: var(--paper);
    border-left: 0.5px solid var(--rule);
    box-shadow: -10px 0 30px rgba(0, 0, 0, 0.08);
    animation: entrar 0.18s ease-out;
  }
  @keyframes entrar {
    from {
      transform: translateX(16px);
      opacity: 0;
    }
  }
  .borde {
    position: absolute;
    left: -3px;
    top: 0;
    bottom: 0;
    width: 7px;
    cursor: col-resize;
    z-index: 2;
  }
  .borde:hover,
  .borde:focus-visible {
    background: linear-gradient(to right, transparent 2px, var(--solver) 2px, var(--solver) 4px, transparent 4px);
    outline: none;
  }
  .cab {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 0 8px 0 12px;
    background: var(--barra-bg);
    -webkit-backdrop-filter: var(--vidrio);
    backdrop-filter: var(--vidrio);
    color: var(--barra-fg);
    border-bottom: 0.5px solid var(--rule);
  }
  .cab :global(.btn.ghost) {
    color: var(--barra-fg-2);
  }
  .logo {
    display: grid;
    place-items: center;
    width: 24px;
    height: 24px;
    border-radius: 6px;
    color: #fff;
    background: linear-gradient(135deg, var(--solver), var(--accent-2));
    flex: none;
    margin-right: 6px;
  }
  .ctit {
    display: flex;
    align-items: center;
    gap: 8px;
    flex: 1;
    min-width: 0;
  }
  .ctit strong {
    font-size: 13.5px;
    white-space: nowrap;
  }
  .modelo {
    display: inline-flex;
    flex: 0 1 auto;
    align-items: center;
    gap: 4px;
    min-width: 0;
    max-width: 240px;
    height: 24px;
    padding: 0 8px;
    font-size: 11.5px;
    color: var(--barra-fg-2);
    background-color: color-mix(in oklab, var(--barra-fg) 6%, transparent);
    border: 0;
    border-radius: 6px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .modelo :global(svg) {
    flex: none;
  }
  .lista {
    overflow: auto;
    padding: 16px 16px 8px;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }
  .clave {
    padding: 16px 16px 12px;
  }
  .clave h3,
  .confirmar h4 {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 15px;
    margin: 0 0 8px;
  }
  .clave h3 :global(svg) {
    color: var(--solver);
  }
  .listo {
    margin-top: 10px;
  }
  .clave p {
    margin: 0 0 8px;
  }
  .acc {
    display: flex;
    align-items: center;
    gap: 12px;
    flex-wrap: wrap;
    margin-top: 6px;
  }
  .peq {
    margin-top: 12px !important;
    font-size: 12px;
  }
  .vacio {
    text-align: center;
    padding: 24px 6px 6px;
  }
  .grande {
    display: inline-grid;
    place-items: center;
    width: 48px;
    height: 48px;
    border-radius: 14px;
    color: var(--solver);
    background: var(--solver-tint);
  }
  .vacio h3 {
    font-size: 18px;
    margin: 12px 0 6px;
  }
  .vacio p {
    margin: 0 auto 16px;
    max-width: 44ch;
    font-size: 12.5px;
  }
  .sug {
    display: grid;
    gap: 6px;
    text-align: left;
  }
  .opc {
    border: 0.5px solid var(--rule);
    background: var(--sheet);
    border-radius: var(--r-md);
    padding: 9px 12px;
    font-size: 12.5px;
    text-align: left;
    color: var(--ink);
    transition: background-color 0.12s, color 0.12s, border-color 0.12s;
  }
  .opc:hover,
  .opc:focus-visible {
    background: var(--solver-tint);
    color: var(--solver);
    border-color: transparent;
  }
  .usuario {
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    gap: 4px;
    margin-left: 32px;
  }
  .burbuja {
    padding: 8px 12px;
    border-radius: 14px 14px 4px 14px;
    background: var(--accent-btn);
    color: var(--accent-btn-ink);
    white-space: pre-wrap;
    overflow-wrap: anywhere;
    line-height: 1.45;
  }
  .ctx {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    max-width: 100%;
    font-size: 11.5px;
    color: var(--ink-2);
    padding: 2px 8px;
    border-radius: 999px;
    background: var(--rule-soft);
    border: 0.5px solid var(--rule);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .asistente {
    display: flex;
    flex-direction: column;
    gap: 8px;
    min-width: 0;
  }
  .herr {
    border: 0.5px solid var(--rule);
    border-radius: var(--r-md);
    background: var(--sheet);
    font-size: 12.5px;
  }
  .herr summary {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 10px;
    cursor: pointer;
    list-style: none;
    color: var(--ink-2);
  }
  .herr summary::-webkit-details-marker {
    display: none;
  }
  .herr summary :global(svg) {
    color: var(--solver);
    flex: none;
  }
  .herr.err summary :global(svg) {
    color: var(--bad);
  }
  .hl {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-weight: 550;
    color: var(--ink);
  }
  .chev {
    display: inline-flex;
    transition: transform 0.15s;
  }
  .herr[open] .chev {
    transform: rotate(180deg);
  }
  .hdet {
    padding: 0 10px 10px;
    display: grid;
    gap: 4px;
  }
  .hk {
    font-size: 11px;
    font-weight: 600;
    color: var(--ink-3);
    margin-top: 4px;
  }
  .hdet pre {
    margin: 0;
    max-height: 220px;
    overflow: auto;
    padding: 8px;
    font-size: 11.5px;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
    background: var(--sheet-2);
    border-radius: var(--r-sm);
  }
  .spinner {
    width: 13px;
    height: 13px;
    border-width: 2px;
    color: var(--solver);
  }
  .aviso {
    display: flex;
    align-items: center;
    gap: 6px;
    margin: 0;
    font-size: 12px;
    color: var(--ink-2);
  }
  .pensando {
    display: inline-flex;
    gap: 4px;
    padding: 6px 2px;
  }
  .pensando i {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--ink-3);
    animation: latido 1.2s infinite ease-in-out;
  }
  .pensando i:nth-child(2) {
    animation-delay: 0.15s;
  }
  .pensando i:nth-child(3) {
    animation-delay: 0.3s;
  }
  @keyframes latido {
    0%,
    80%,
    100% {
      opacity: 0.25;
      transform: scale(0.85);
    }
    40% {
      opacity: 1;
      transform: scale(1);
    }
  }
  .macc {
    display: flex;
    gap: 4px;
    margin-top: -2px;
  }
  .btn.mini {
    height: 24px;
    padding: 0 8px;
    font-size: 11.5px;
  }
  .btn.icon.mini {
    width: 24px;
    padding: 0;
  }
  .confirmar {
    padding: 14px 16px;
    border-color: color-mix(in oklab, var(--binding) 45%, transparent);
    background: color-mix(in oklab, var(--binding-tint) 60%, var(--sheet));
  }
  .confirmar h4 :global(svg) {
    color: var(--binding);
  }
  .confirmar p {
    margin: 0 0 6px;
  }
  .confirmar .acc {
    justify-content: flex-end;
  }
  .error {
    display: grid;
    gap: 6px;
    padding: 10px 12px;
    border: 1px solid color-mix(in oklab, var(--bad) 45%, transparent);
    background: var(--bad-tint);
    border-radius: var(--r-md);
    font-size: 12.5px;
  }
  .error > span {
    display: flex;
    gap: 8px;
    align-items: flex-start;
  }
  .error > span :global(svg) {
    color: var(--bad);
    flex: none;
    margin-top: 2px;
  }
  .error .acc {
    margin: 0;
    justify-content: flex-end;
    gap: 6px;
  }
  .pie {
    padding: 8px 12px 10px;
    border-top: 0.5px solid var(--rule);
    background: var(--paper);
  }
  .adj {
    display: flex;
    align-items: center;
    gap: 6px;
    margin-bottom: 6px;
    padding: 3px 4px 3px 10px;
    border-radius: var(--r-md);
    background: var(--solver-tint);
    color: var(--solver);
    font-size: 12px;
    font-weight: 550;
  }
  .adj .at {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .adj :global(.btn.ghost) {
    color: inherit;
  }
  .caja {
    display: flex;
    align-items: flex-end;
    gap: 6px;
    padding: 5px 5px 5px 12px;
    border: 0.5px solid var(--rule);
    border-radius: 14px;
    background: var(--sheet);
    box-shadow: var(--shadow);
  }
  .caja:focus-within {
    border-color: color-mix(in oklab, var(--focus) 60%, transparent);
    box-shadow: 0 0 0 3px color-mix(in oklab, var(--focus) 18%, transparent);
  }
  .caja.off {
    opacity: 0.6;
  }
  .adjs {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin-bottom: 8px;
  }
  .adjs.enviados {
    justify-content: flex-end;
    margin-bottom: 4px;
  }
  .adjf {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    max-width: 220px;
    padding: 3px 6px 3px 3px;
    border: 0.5px solid var(--rule);
    border-radius: var(--r-sm);
    background: var(--sheet);
    font-size: 12px;
  }
  .adjf img {
    width: 34px;
    height: 34px;
    object-fit: cover;
    border-radius: 4px;
  }
  .adjf .tipo {
    display: grid;
    place-items: center;
    min-width: 34px;
    height: 34px;
    padding: 0 4px;
    border-radius: 4px;
    background: var(--rule-soft);
    font-size: 10px;
    font-weight: 650;
    color: var(--ink-2);
  }
  .adjf .an {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .adjf .quitar {
    display: grid;
    place-items: center;
    width: 18px;
    height: 18px;
    border: 0;
    border-radius: 50%;
    background: transparent;
    color: var(--ink-2);
    cursor: pointer;
  }
  .adjf .quitar:hover {
    background: var(--rule-soft);
  }
  .adjf.leyendo {
    padding: 6px 10px;
    color: var(--ink-2);
  }
  .caja.soltando {
    outline: 2px dashed var(--accent, #0a84ff);
    outline-offset: 2px;
  }
  .clip {
    align-self: flex-end;
  }
  textarea {
    flex: 1;
    min-width: 0;
    resize: none;
    border: 0;
    outline: none;
    background: none;
    padding: 5px 0;
    max-height: 200px;
    line-height: 1.45;
  }
  .enviar {
    width: 30px;
    height: 30px;
    border-radius: 10px;
    flex: none;
  }
  .parar {
    background: var(--ink);
    border-color: transparent;
  }
  .cuadro {
    width: 10px;
    height: 10px;
    border-radius: 2px;
    background: var(--paper);
  }
  .legal {
    margin: 6px 2px 0;
    font-size: 11px;
    color: var(--ink-3);
    text-align: center;
  }
</style>
