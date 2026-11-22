<script lang="ts">
  // Botón «Explicar con IA» de las vistas de solución: arma un prompt con el informe
  // (Markdown del núcleo) y el modelo (JSON), para copiarlo a cualquier IA o
  // preguntarlo en el chat de IAQSB con el contexto adjunto.
  import Icon from './Icon.svelte';
  import { chat, fuenteActual, promptParaCopiar } from '../lib/chat.svelte';
  import { ui } from '../lib/ui.svelte';
  import { t } from '../lib/i18n.svelte';

  let { modulo }: { modulo: string } = $props();

  let abierto = $state(false);
  let pasos = $state(false);
  let ocupado = $state(false);
  let caja: HTMLElement | undefined = $state();

  const detalle = $derived(pasos ? 'steps' : 'summary') as 'summary' | 'steps';

  async function preguntar() {
    abierto = false;
    ocupado = true;
    try {
      await chat.explicar(fuenteActual(modulo), detalle);
    } catch (e) {
      ui.avisar(e instanceof Error ? e.message : String(e), 'error');
    } finally {
      ocupado = false;
    }
  }

  async function copiar() {
    ocupado = true;
    try {
      const txt = await promptParaCopiar(fuenteActual(modulo), detalle);
      await escribir(txt);
      abierto = false;
      ui.avisar(t('Prompt copiado: pégalo en ChatGPT, Gemini o Claude.ai'), 'ok');
    } catch (e) {
      ui.avisar(t('No se pudo copiar al portapapeles') + (e instanceof Error ? `: ${e.message}` : ''), 'error');
    } finally {
      ocupado = false;
    }
  }

  /** Portapapeles, con reserva para ventanas sin permiso de la API moderna. */
  async function escribir(txt: string) {
    try {
      await navigator.clipboard.writeText(txt);
    } catch {
      const ta = document.createElement('textarea');
      ta.value = txt;
      ta.style.position = 'fixed';
      ta.style.opacity = '0';
      document.body.appendChild(ta);
      ta.select();
      const ok = document.execCommand('copy');
      ta.remove();
      if (!ok) throw new Error(t('el sistema no lo permitió'));
    }
  }

  function fuera(e: MouseEvent) {
    if (abierto && caja && !caja.contains(e.target as Node)) abierto = false;
  }
</script>

<svelte:window onclick={fuera} onkeydown={(e) => e.key === 'Escape' && (abierto = false)} />

<div class="explicar" bind:this={caja}>
  <button class="btn ia" aria-haspopup="menu" aria-expanded={abierto} onclick={() => (abierto = !abierto)} disabled={ocupado} title={t('Que una IA te explique este resultado')}>
    {#if ocupado}<span class="spinner" aria-hidden="true"></span>{:else}<Icon name="chispa" size={15} />{/if}
    {t('Explicar con IA')}
    <Icon name="abajo" size={14} />
  </button>
  {#if abierto}
    <div class="popover menu" role="menu">
      <button role="menuitem" class="opc" onclick={preguntar}>
        <span class="oi"><Icon name="chat" size={16} /></span>
        <span class="ot"><b>{t('Preguntar en el chat')}</b><small>{t('El asistente lo explica aquí, con el informe y el modelo adjuntos')}</small></span>
      </button>
      <button role="menuitem" class="opc" onclick={copiar}>
        <span class="oi"><Icon name="copiar" size={16} /></span>
        <span class="ot"><b>{t('Copiar para otra IA')}</b><small>{t('Prompt listo para pegar en ChatGPT, Gemini o Claude.ai')}</small></span>
      </button>
      <label class="pasos">
        <input type="checkbox" bind:checked={pasos} />
        <span>{t('Incluir el paso a paso del método')}</span>
      </label>
    </div>
  {/if}
</div>

<style>
  .explicar {
    position: relative;
    flex: none;
  }
  .ia :global(svg:first-child) {
    color: var(--accent-2);
  }
  .ia .spinner {
    width: 13px;
    height: 13px;
    color: var(--accent-2);
  }
  .menu {
    right: 0;
    top: calc(100% + 6px);
    width: 320px;
    padding: 6px;
  }
  .opc {
    display: flex;
    align-items: flex-start;
    gap: 10px;
    width: 100%;
    text-align: left;
    border: 0;
    background: none;
    padding: 8px 10px;
    border-radius: 9px;
  }
  .opc:hover,
  .opc:focus-visible {
    background: var(--solver-tint);
    outline: none;
  }
  .oi {
    display: grid;
    place-items: center;
    width: 28px;
    height: 28px;
    border-radius: 8px;
    flex: none;
    color: var(--solver);
    background: var(--solver-tint);
  }
  .ot {
    display: grid;
    gap: 1px;
    font-size: 13px;
  }
  .ot b {
    font-weight: 600;
  }
  .ot small {
    font-size: 11.5px;
    color: var(--ink-2);
    line-height: 1.35;
  }
  .pasos {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 4px 0 2px;
    padding: 8px 10px 4px;
    border-top: 0.5px solid var(--rule);
    font-size: 12.5px;
    color: var(--ink-2);
  }
</style>
