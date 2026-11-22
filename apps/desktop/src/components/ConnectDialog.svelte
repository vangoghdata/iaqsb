<script lang="ts">
  import { app } from '../lib/state.svelte';
  import { mcpInfo, type McpInfo } from '../lib/backend';
  import { t } from '../lib/i18n.svelte';

  let dlg: HTMLDialogElement;
  let info = $state<McpInfo | null>(null);
  let copied = $state(false);

  $effect(() => {
    if (app.connectOpen) {
      mcpInfo().then((i) => (info = i));
      dlg?.showModal();
    } else {
      dlg?.close();
    }
  });

  async function copy() {
    if (!info) return;
    await navigator.clipboard.writeText(info.config);
    copied = true;
    setTimeout(() => (copied = false), 2000);
  }
</script>

<dialog bind:this={dlg} onclose={() => (app.connectOpen = false)} aria-labelledby="connect-title">
  <h2 id="connect-title">{t('Conectar IAQSB con Claude')}</h2>
  <p>
    {t(
      'IAQSB incluye un servidor MCP. Con él, Claude puede plantear problemas, resolverlos con este mismo motor, abrir y guardar archivos de WinQSB y explicarte los resultados.'
    )}
  </p>
  <ol>
    <li>{t('Abre el archivo de servidores MCP de Claude Desktop,')} <code>claude_desktop_config.json</code>.</li>
    <li>{t('Añade este bloque (o solo la entrada')} <code>iaqsb</code> {t('si ya tienes otros servidores) y guárdalo.')}</li>
    <li>{t('Reinicia Claude Desktop. IAQSB aparecerá entre sus herramientas.')}</li>
  </ol>
  <pre>{info?.config ?? '…'}</pre>
  <p class="muted small">
    {t('Otros asistentes pueden usar la línea de comandos:')} <code>{info?.cli_path ?? 'iaqsb'} --json llamar solve</code> {t('con el modelo en JSON.')}
  </p>
  <div class="actions">
    <button class="btn" onclick={() => (app.connectOpen = false)}>{t('Cerrar')}</button>
    <button class="btn primary" onclick={copy}>{copied ? t('Copiado') : t('Copiar configuración')}</button>
  </div>
</dialog>

<style>
  dialog {
    width: min(600px, calc(100vw - 32px));
    border: 1px solid var(--rule);
    border-radius: var(--r-lg);
    box-shadow: var(--shadow-pop);
    padding: 26px 28px 22px;
    background: var(--sheet);
    color: var(--ink);
  }
  dialog::backdrop {
    background: var(--overlay);
  }
  h2 {
    font-size: 21px;
    margin-bottom: 10px;
  }
  p,
  li {
    max-width: 62ch;
  }
  ol {
    padding-left: 20px;
  }
  li {
    margin-bottom: 4px;
  }
  pre {
    background: var(--rule-soft);
    border: 1px solid var(--rule);
    border-radius: var(--r-md);
    padding: 12px 14px;
    overflow: auto;
    font-size: 12.5px;
  }
  code {
    font-size: 12.5px;
    background: var(--rule-soft);
    padding: 1px 5px;
    border-radius: 4px;
  }
  .small {
    font-size: 12.5px;
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 16px;
  }
</style>
