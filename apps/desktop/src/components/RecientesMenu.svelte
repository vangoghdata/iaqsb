<script lang="ts">
  // Archivos recientes: volver a abrir con un clic (app de escritorio).
  import Icon from './Icon.svelte';
  import { recientes, nombreArchivo } from '../lib/recientes.svelte';
  import { inTauri, type Imported } from '../lib/backend';
  import { shell } from '../lib/shell.svelte';
  import { ui } from '../lib/ui.svelte';
  import { i18n, t } from '../lib/i18n.svelte';

  let abierto = $state(false);
  let caja: HTMLElement | undefined = $state();

  function fuera(e: MouseEvent) {
    if (abierto && caja && !caja.contains(e.target as Node)) abierto = false;
  }

  const nombreModulo = (id: string) => (id === 'lp' ? 'LP-ILP' : (shell.modules.find((m) => m.id === id)?.winqsb ?? id));

  function cuando(ts: number): string {
    const d = new Date(ts);
    return d.toLocaleString(i18n.lang === 'en' ? 'en-US' : i18n.lang === 'pt' ? 'pt-BR' : 'es-CO', { dateStyle: 'medium', timeStyle: 'short' });
  }

  async function abrir(path: string, module: string) {
    abierto = false;
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const existe = await invoke<boolean>('file_exists', { path });
      if (!existe) {
        recientes.quitar(path);
        ui.avisar(t('Ese archivo ya no existe: {0}', nombreArchivo(path)), 'error');
        return;
      }
      const res = await invoke<Imported>('read_file', { path, module, problemType: null });
      shell.openImported({ ...res, path });
    } catch (e) {
      ui.avisar(e instanceof Error ? e.message : String(e), 'error');
    }
  }
</script>

<svelte:window onclick={fuera} onkeydown={(e) => e.key === 'Escape' && (abierto = false)} />

{#if inTauri}
  <div class="rec" bind:this={caja}>
    <button class="btn ghost icon" aria-haspopup="menu" aria-expanded={abierto} onclick={() => (abierto = !abierto)} title={t('Archivos recientes')} aria-label={t('Archivos recientes')}>
      <Icon name="reloj" size={16} />
    </button>
    {#if abierto}
      <div class="popover lista" role="menu">
        <p class="ptit">{t('Archivos recientes')}</p>
        {#each recientes.lista as r (r.path)}
          <button role="menuitem" class="opc" onclick={() => abrir(r.path, r.module)} title={r.path}>
            <span class="el">{nombreArchivo(r.path)}</span>
            <span class="ef">{nombreModulo(r.module)} · {cuando(r.ts)}{r.name ? ` · ${r.name}` : ''}</span>
          </button>
        {:else}
          <p class="vacio">{t('Todavía no hay archivos recientes.')}</p>
        {/each}
        {#if recientes.lista.length}
          <button class="limpiar" onclick={() => recientes.limpiar()}>{t('Borrar la lista')}</button>
        {/if}
      </div>
    {/if}
  </div>
{/if}

<style>
  .rec {
    position: relative;
  }
  .lista {
    left: 0;
    top: calc(100% + 6px);
    width: 340px;
    max-height: 60vh;
    overflow: auto;
    padding: 6px;
  }
  .ptit {
    margin: 4px 8px 6px;
    font-size: 10.5px;
    font-weight: 650;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--ink-3);
  }
  .opc {
    display: block;
    width: 100%;
    text-align: left;
    border: 0;
    background: none;
    padding: 7px 10px;
    border-radius: 9px;
  }
  .opc:hover,
  .opc:focus-visible {
    background: var(--solver-tint);
    outline: none;
  }
  .el {
    display: block;
    font-size: 13px;
  }
  .ef {
    display: block;
    font-size: 11.5px;
    color: var(--ink-3);
  }
  .vacio {
    margin: 6px 10px 10px;
    font-size: 12.5px;
    color: var(--ink-3);
  }
  .limpiar {
    margin: 4px 6px 2px;
    border: 0;
    background: none;
    font-size: 12px;
    color: var(--ink-3);
  }
</style>
