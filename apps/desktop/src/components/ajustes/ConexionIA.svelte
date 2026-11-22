<script lang="ts">
  // «Conectar la API»: proveedor, dirección, clave y modelo del asistente de IA.
  // Sirve para cualquier modelo: Claude (Anthropic), OpenRouter (cientos de modelos),
  // OpenAI, Gemini, Qwen, DeepSeek, Mistral, Groq, xAI, modelos locales (Ollama,
  // LM Studio) o cualquier servidor compatible con la API de OpenAI.
  // Se usa en Ajustes → Inteligencia artificial y en el panel del asistente.
  import Icon from '../Icon.svelte';
  import { ajustes } from '../../lib/ajustes.svelte';
  import { PROVEEDORES, base, esLocal, listarModelos, probar, proveedor, type ModeloDisponible } from '../../lib/proveedores';
  import { openExternal } from '../../lib/backend';
  import { chat } from '../../lib/chat.svelte';
  import { t } from '../../lib/i18n.svelte';

  /** compacto: para el panel del asistente (sin la dirección salvo que haga falta). */
  let { compacto = false, alConectar }: { compacto?: boolean; alConectar?: () => void } = $props();

  const c = $derived(ajustes.conexion);
  const p = $derived(c.p);
  const uid = $props.id();

  let verClave = $state(false);
  let verUrl = $state(false);
  let prueba = $state<{ ok: boolean; texto: string } | null>(null);
  let probando = $state(false);
  // modelos de cada proveedor (se piden una vez por proveedor y dirección)
  let listas = $state<Record<string, ModeloDisponible[]>>({});
  let cargando = $state(false);
  let pedidoLista: AbortController | null = null;
  let errorLista = $state<string | null>(null);
  let soloHerramientas = $state(true);

  const claveLista = $derived(`${p.id}|${base(c.url)}`);
  const modelos = $derived(listas[claveLista] ?? []);
  const conInfoHerr = $derived(modelos.some((m) => m.herramientas !== null));
  const sugerencias = $derived.by(() => {
    const vistos = new Set<string>();
    const out: { id: string; etiqueta: string }[] = [];
    for (const id of p.sugeridos ?? []) {
      vistos.add(id);
      out.push({ id, etiqueta: id });
    }
    for (const m of modelos) {
      if (vistos.has(m.id) || (soloHerramientas && m.herramientas === false)) continue;
      vistos.add(m.id);
      out.push({ id: m.id, etiqueta: m.nombre ? `${m.nombre}${m.herramientas === false ? ' · ' + t('sin herramientas') : ''}` : m.herramientas === false ? t('sin herramientas') : '' });
    }
    return out;
  });
  const elegido = $derived(modelos.find((m) => m.id === c.modelo));
  const httpRemoto = $derived(/^http:\/\//i.test(c.url.trim()) && !esLocal(c.url));
  const mostrarUrl = $derived(verUrl || !!p.editarUrl || p.grupo === 'local' || !!ajustes.urls[p.id]);
  const nube = PROVEEDORES.filter((x) => x.grupo === 'nube');
  const locales = PROVEEDORES.filter((x) => x.grupo === 'local');
  const otros = PROVEEDORES.filter((x) => x.grupo === 'otro');

  function guardar() {
    ajustes.guardar();
    prueba = null;
  }

  function elegirProveedor(id: string) {
    pedidoLista?.abort();
    pedidoLista = null;
    cargando = false;
    ajustes.proveedor = id;
    verUrl = false;
    prueba = null;
    errorLista = null;
    guardar();
    chat.error = null;
  }

  function fijar(campo: 'claves' | 'modelos' | 'urls', v: string) {
    ajustes[campo] = { ...ajustes[campo], [p.id]: v };
    guardar();
  }

  async function cargarModelos(forzar = false) {
    if (cargando || (!forzar && listas[claveLista])) return;
    if (p.clave === 'si' && !c.clave.trim()) {
      errorLista = t('Primero escribe la clave de la API.');
      return;
    }
    if (!c.url.trim()) {
      errorLista = t('Primero escribe la dirección de la API.');
      return;
    }
    pedidoLista?.abort();
    const ac = new AbortController();
    pedidoLista = ac;
    cargando = true;
    errorLista = null;
    const k = claveLista;
    try {
      const l = await listarModelos(c, ac.signal);
      if (ac.signal.aborted) return;
      listas = { ...listas, [k]: l };
      if (!l.length && k === claveLista) errorLista = t('El proveedor no devolvió la lista de modelos: escribe el nombre del modelo.');
    } catch (e) {
      // la respuesta de otro proveedor (se cambió mientras tanto) no se muestra
      if (ac.signal.aborted || k !== claveLista) return;
      errorLista = t('No se pudo traer la lista de modelos ({0}). Puedes escribir el nombre del modelo.', e instanceof Error ? e.message : String(e));
    } finally {
      if (pedidoLista === ac) {
        pedidoLista = null;
        cargando = false;
      }
    }
  }

  async function probarConexion() {
    probando = true;
    prueba = null;
    try {
      await probar(c);
      prueba = { ok: true, texto: t('Conectado: {0} responde.', c.modelo) };
      chat.error = null;
      alConectar?.();
    } catch (e) {
      const m = e instanceof Error ? e.message : String(e);
      prueba = { ok: false, texto: e instanceof TypeError ? t('No se pudo conectar con {0} ({1}).', base(c.url), m) : m };
    } finally {
      probando = false;
    }
  }
</script>

<div class="conexion" class:compacto>
  <label class="campo">
    <span class="et">{t('Proveedor')}</span>
    <select value={p.id} onchange={(e) => elegirProveedor(e.currentTarget.value)}>
      <optgroup label={t('En la nube')}>
        {#each nube as x (x.id)}<option value={x.id}>{t(x.nombre)}</option>{/each}
      </optgroup>
      <optgroup label={t('En este equipo (gratis, sin internet)')}>
        {#each locales as x (x.id)}<option value={x.id}>{t(x.nombre)}</option>{/each}
      </optgroup>
      <optgroup label={t('Otro')}>
        {#each otros as x (x.id)}<option value={x.id}>{t(x.nombre)}</option>{/each}
      </optgroup>
    </select>
  </label>
  <p class="ayuda">{t(p.descripcion)}</p>
  {#if p.nota && !compacto}<p class="nota"><Icon name="info" size={13} />{t(p.nota)}</p>{/if}

  {#if mostrarUrl}
    <label class="campo">
      <span class="et">{t('Dirección de la API')}</span>
      <input
        value={ajustes.urls[p.id] ?? p.url}
        oninput={(e) => fijar('urls', e.currentTarget.value.trim() === p.url ? '' : e.currentTarget.value)}
        placeholder={p.url || 'https://mi-servidor.edu/v1'}
        spellcheck="false"
        autocomplete="off"
      />
    </label>
    {#if httpRemoto}<p class="nota mal"><Icon name="alerta" size={13} />{t('La dirección no está cifrada (http): la clave y la conversación viajan sin protección. Usa https salvo en tu propia red.')}</p>{/if}
  {/if}

  {#if p.clave !== 'no'}
    <label class="campo">
      <span class="et">{p.clave === 'opcional' ? t('Clave de la API (si el servidor la pide)') : t('Clave de la API')}</span>
      <span class="con-boton">
        <input
          type={verClave ? 'text' : 'password'}
          value={ajustes.claves[p.id] ?? ''}
          oninput={(e) => fijar('claves', e.currentTarget.value.trim())}
          placeholder={p.formatoClave ?? ''}
          autocomplete="off"
          spellcheck="false"
        />
        <button type="button" class="btn ghost" onclick={() => (verClave = !verClave)}>{verClave ? t('Ocultar') : t('Mostrar')}</button>
      </span>
    </label>
  {/if}

  <label class="campo">
    <span class="et">{t('Modelo')}</span>
    <span class="con-boton">
      <input
        list="modelos-{uid}"
        value={ajustes.modelos[p.id] ?? p.modelo}
        oninput={(e) => fijar('modelos', e.currentTarget.value.trim())}
        onfocus={() => cargarModelos()}
        placeholder={p.id === 'openrouter' ? t('p. ej. proveedor/modelo') : t('Escribe o elige un modelo')}
        spellcheck="false"
        autocomplete="off"
      />
      <button type="button" class="btn ghost" onclick={() => cargarModelos(true)} disabled={cargando} title={t('Traer la lista de modelos del proveedor')}>
        {#if cargando}<span class="spinner" aria-hidden="true"></span>{:else}<Icon name="recargar" size={14} />{/if}{t('Modelos')}
      </button>
    </span>
    <datalist id="modelos-{uid}">
      {#each sugerencias as m (m.id)}<option value={m.id}>{m.etiqueta}</option>{/each}
    </datalist>
  </label>
  {#if modelos.length}
    <p class="ayuda">
      {t('{0} modelos disponibles: escribe para buscar.', modelos.length)}
      {#if conInfoHerr}
        <label class="chk"><input type="checkbox" bind:checked={soloHerramientas} />{t('Solo los que usan herramientas (para resolver con el motor)')}</label>
      {/if}
    </p>
  {/if}
  {#if elegido?.herramientas === false}<p class="nota mal"><Icon name="alerta" size={13} />{t('Este modelo no usa herramientas: podrá explicar, pero no resolver con el motor de IAQSB.')}</p>{/if}
  {#if errorLista}<p class="nota">{errorLista}</p>{/if}

  <div class="acciones">
    <button type="button" class="btn primary" onclick={probarConexion} disabled={probando || !ajustes.iaLista}>
      {#if probando}<span class="spinner" aria-hidden="true"></span>{t('Probando…')}{:else}<Icon name="chispa" size={14} />{t('Probar la conexión')}{/if}
    </button>
    {#if p.dondeClave}
      <button type="button" class="link" onclick={() => openExternal(p.dondeClave!)}>{p.clave === 'no' ? t('Descargar {0}', t(p.nombre)) : t('Obtener una clave')}</button>
    {/if}
    {#if !mostrarUrl}<button type="button" class="link" onclick={() => (verUrl = true)}>{t('Cambiar la dirección')}</button>{/if}
  </div>
  {#if prueba}<p class="resultado" class:ok={prueba.ok} class:mal={!prueba.ok} role="status">{prueba.texto}</p>{/if}
  {#if !compacto}
    <p class="ayuda">{t('La clave se guarda solo en este equipo y se usa únicamente para hablar con el proveedor elegido; el uso lo cobra cada proveedor según su plan. Puedes guardar una clave por proveedor y cambiar cuando quieras.')}</p>
  {/if}
</div>

<style>
  .conexion {
    display: grid;
    gap: 9px;
    max-width: 560px;
  }
  .campo {
    display: grid;
    gap: 4px;
    font-size: 12.5px;
    color: var(--ink-2);
  }
  .et {
    font-weight: 550;
  }
  .campo input,
  .campo select {
    height: 32px;
    padding: 0 9px;
    border: 1px solid var(--rule);
    border-radius: var(--r-sm);
    background: var(--sheet);
    font-size: 13.5px;
    color: var(--ink);
    min-width: 0;
  }
  .con-boton {
    display: flex;
    gap: 6px;
  }
  .con-boton input {
    flex: 1;
    font-family: ui-monospace, Menlo, Consolas, monospace;
    font-size: 12.5px;
  }
  .ayuda {
    margin: 0;
    font-size: 12.5px;
    color: var(--ink-3);
    line-height: 1.45;
  }
  .nota {
    display: flex;
    gap: 6px;
    align-items: flex-start;
    margin: 0;
    font-size: 12px;
    line-height: 1.45;
    color: var(--ink-2);
  }
  .nota :global(svg) {
    flex: none;
    margin-top: 2px;
  }
  .nota.mal {
    color: var(--bad);
  }
  .chk {
    display: inline-flex;
    gap: 6px;
    align-items: center;
    margin-left: 8px;
    color: var(--ink-2);
  }
  .acciones {
    display: flex;
    gap: 12px;
    align-items: center;
    flex-wrap: wrap;
  }
  .link {
    border: 0;
    background: none;
    padding: 0;
    color: var(--solver);
    font-size: 13px;
    cursor: pointer;
  }
  .link:hover {
    text-decoration: underline;
  }
  .resultado {
    margin: 0;
    font-size: 13px;
    overflow-wrap: anywhere;
  }
  .resultado.ok {
    color: var(--good);
  }
  .resultado.mal {
    color: var(--bad);
  }
  .compacto {
    gap: 8px;
  }
</style>
