<script lang="ts">
  // Texto en Markdown del chat. El HTML sale de lib/markdown.ts, que escapa todo lo
  // que viene del modelo antes de agregar sus propias etiquetas.
  import { markdownHtml } from '../../lib/markdown';
  import { openExternal } from '../../lib/backend';
  import { t } from '../../lib/i18n.svelte';
  import { ui } from '../../lib/ui.svelte';

  let { texto }: { texto: string } = $props();
  const html = $derived(markdownHtml(texto, { copiar: t('Copiar') }));

  /** Tabla del Markdown → texto separado por tabulaciones (se pega bien en Excel). */
  function tsv(tabla: HTMLTableElement): string {
    return [...tabla.rows].map((r) => [...r.cells].map((c) => (c.textContent ?? '').trim()).join('\t')).join('\n');
  }

  async function clic(e: MouseEvent) {
    const el = e.target as HTMLElement;
    const a = el.closest('a');
    if (a) {
      // los enlaces se abren en el navegador del sistema (no dentro de la app)
      e.preventDefault();
      const href = a.getAttribute('href') ?? '';
      if (/^https?:\/\//.test(href)) openExternal(href);
      return;
    }
    const b = el.closest('button.md-copy') as HTMLButtonElement | null;
    if (!b) return;
    const caja = b.parentElement?.parentElement;
    let txt = '';
    if (b.dataset.copy === 'code') txt = caja?.querySelector('code')?.textContent ?? '';
    else {
      const tabla = b.parentElement?.querySelector('table');
      if (tabla) txt = tsv(tabla);
    }
    try {
      await navigator.clipboard.writeText(txt);
      const antes = b.textContent;
      b.textContent = t('Copiado');
      setTimeout(() => (b.textContent = antes), 1400);
    } catch {
      ui.avisar(t('No se pudo copiar al portapapeles'), 'error');
    }
  }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
<div class="md" onclick={clic}>{@html html}</div>

<style>
  .md {
    font-size: 13.5px;
    line-height: 1.55;
    overflow-wrap: anywhere;
    min-width: 0;
  }
  .md :global(> :first-child) {
    margin-top: 0;
  }
  .md :global(> :last-child) {
    margin-bottom: 0;
  }
  .md :global(p) {
    margin: 0 0 9px;
  }
  .md :global(h3),
  .md :global(h4),
  .md :global(h5),
  .md :global(h6) {
    font-family: var(--font-brand);
    letter-spacing: -0.015em;
    margin: 14px 0 6px;
    line-height: 1.3;
  }
  .md :global(h3) {
    font-size: 16px;
  }
  .md :global(h4) {
    font-size: 14.5px;
  }
  .md :global(h5),
  .md :global(h6) {
    font-size: 13.5px;
  }
  .md :global(ul),
  .md :global(ol) {
    margin: 0 0 9px;
    padding-left: 22px;
  }
  .md :global(li) {
    margin: 2px 0;
  }
  .md :global(li > ul),
  .md :global(li > ol) {
    margin: 2px 0 0;
  }
  .md :global(strong) {
    font-weight: 650;
  }
  .md :global(a) {
    color: var(--solver);
    text-decoration: none;
  }
  .md :global(a:hover) {
    text-decoration: underline;
  }
  .md :global(hr) {
    border: 0;
    border-top: 1px solid var(--rule);
    margin: 12px 0;
  }
  .md :global(blockquote) {
    margin: 0 0 9px;
    padding: 4px 12px;
    border-left: 3px solid var(--rule);
    color: var(--ink-2);
  }
  .md :global(code) {
    font-family: ui-monospace, 'SF Mono', 'Cascadia Code', Menlo, Consolas, monospace;
    font-size: 12.5px;
    padding: 1px 5px;
    border-radius: 5px;
    background: var(--rule-soft);
    border: 0.5px solid var(--rule);
  }
  .md :global(.md-code) {
    margin: 0 0 10px;
    border: 0.5px solid var(--rule);
    border-radius: var(--r-md);
    background: var(--sheet-2);
    overflow: hidden;
  }
  .md :global(.md-code-h) {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 3px 6px 3px 10px;
    font-size: 11px;
    color: var(--ink-3);
    border-bottom: 0.5px solid var(--rule);
  }
  .md :global(.md-code pre) {
    margin: 0;
    padding: 10px 12px;
    overflow: auto;
    max-height: 360px;
  }
  .md :global(.md-code code) {
    padding: 0;
    border: 0;
    background: none;
    white-space: pre;
  }
  .md :global(.md-copy) {
    border: 0;
    background: none;
    font-size: 11px;
    font-weight: 600;
    color: var(--ink-3);
    padding: 2px 6px;
    border-radius: 5px;
  }
  .md :global(.md-copy:hover) {
    color: var(--solver);
    background: var(--solver-tint);
  }
  .md :global(.md-table) {
    position: relative;
    margin: 0 0 10px;
  }
  .md :global(.md-table > .md-copy) {
    position: absolute;
    right: 4px;
    top: 3px;
    z-index: 1;
    background: var(--sheet-2);
    opacity: 0;
    transition: opacity 0.15s;
  }
  .md :global(.md-table:hover > .md-copy),
  .md :global(.md-table > .md-copy:focus-visible) {
    opacity: 1;
  }
  .md :global(.md-tw) {
    max-width: 100%;
    overflow: auto;
    border: 0.5px solid var(--rule);
    border-radius: var(--r-md);
    background: var(--sheet);
  }
  .md :global(table) {
    font-size: 12.5px;
    width: max-content;
    min-width: 100%;
  }
  .md :global(th),
  .md :global(td) {
    padding: 5px 10px;
    border-bottom: 1px solid var(--rule-soft);
    text-align: left;
    white-space: nowrap;
  }
  .md :global(th) {
    font-weight: 600;
    color: var(--ink-2);
    background: var(--sheet-2);
    border-bottom: 1px solid var(--rule);
  }
  .md :global(tr:last-child td) {
    border-bottom: 0;
  }
</style>
