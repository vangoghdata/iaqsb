<script lang="ts">
  import Sidebar from './components/Sidebar.svelte';
  import TopBar from './components/TopBar.svelte';
  import LpWorkbench from './components/LpWorkbench.svelte';
  import GenericWorkbench from './components/GenericWorkbench.svelte';
  import ConnectDialog from './components/ConnectDialog.svelte';
  import AparienciaDialog from './components/tema/AparienciaDialog.svelte';
  import Avisos from './components/Avisos.svelte';
  import ChatPanel from './components/chat/ChatPanel.svelte';
  import AjustesPage from './components/ajustes/AjustesPage.svelte';
  import { app } from './lib/state.svelte';
  import { shell } from './lib/shell.svelte';
  import { ui } from './lib/ui.svelte';
  import { chat } from './lib/chat.svelte';
  import { inTauri } from './lib/backend';
  import { sesion } from './lib/sesion.svelte';
  import { t } from './lib/i18n.svelte';
  import { onMount } from 'svelte';
  import { escucharPedidosR, motoresWeb } from './lib/motores_web.svelte';

  onMount(() => ui.iniciar());
  // el núcleo puede pedir el R de la app (series de tiempo sin R instalado)
  onMount(() => void escucharPedidosR());

  // si se abrió la app con doble clic en un archivo de WinQSB, cargarlo
  onMount(async () => {
    await shell.init();
    let f: any = null;
    if (inTauri) {
      const { invoke } = await import('@tauri-apps/api/core');
      f = await invoke<{ module: string; layout: 'matrix' | 'normal'; model: any; path: string } | null>('initial_file');
    }
    // el trabajo de la sesión anterior (aunque no se haya guardado)
    const habia = await sesion.recuperar();
    if (f) {
      // openImported sabe abrir la hoja, un modelo grande, un modelo algebraico o cualquier módulo
      shell.openImported(f);
    } else if (habia) {
      ui.avisar(t('Se recuperó el trabajo sin guardar de la sesión anterior.'), 'info');
    }
  });
</script>

<div class="shell" class:fijo={ui.fijado} class:con-chat={chat.abierto} style:--chat-espacio="{chat.ancho}px">
  {#if !ui.fijado}
    <!-- borde izquierdo: al llegar con el mouse aparece la barra lateral -->
    <div class="borde-lateral" role="presentation" onmouseenter={() => ui.mostrarLateral()}></div>
  {/if}
  <Sidebar />
  <div class="main-col">
    <TopBar />
    {#if shell.module === 'lp'}
      <LpWorkbench />
    {:else}
      {#key shell.module}
        <GenericWorkbench id={shell.module} />
      {/key}
    {/if}
  </div>
</div>

<ConnectDialog />
<AjustesPage />
<AparienciaDialog />
<Avisos />
{#if motoresWeb.detalle && !chat.abierto}
  <div class="motorweb" role="status"><span class="spinner" aria-hidden="true"></span>{motoresWeb.detalle}</div>
{/if}
<ChatPanel />

<style>
  .motorweb {
    position: fixed;
    left: 50%;
    bottom: 18px;
    transform: translateX(-50%);
    z-index: 60;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 14px;
    border-radius: 999px;
    background: var(--panel, #fff);
    border: 1px solid var(--border);
    box-shadow: 0 4px 16px rgba(0, 0, 0, 0.15);
    font-size: 0.9em;
  }
  .borde-lateral {
    position: fixed;
    left: 0;
    top: 0;
    bottom: 0;
    width: 6px;
    z-index: 55;
  }
  /* con el asistente abierto, el contenido se corre para que el resultado siga a la vista */
  @media (min-width: 1000px) {
    .shell.con-chat {
      padding-right: min(var(--chat-espacio), 100vw - 560px);
    }
  }
</style>
