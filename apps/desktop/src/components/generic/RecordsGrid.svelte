<script lang="ts">
  import { shown, type Field, type Widget } from '../../lib/gtypes';
  import FieldInput from './FieldInput.svelte';
  import { t } from '../../lib/i18n.svelte';
  import { freshName, gridKeys } from './gridnav';
  import { defaults } from '../../lib/shell.svelte';
  import { pegarEnRegistros } from '../../lib/excel';

  type RecordsW = Extract<Widget, { type: 'records' }>;
  let { w, model, onchange }: { w: RecordsW; model: any; onchange: () => void } = $props();

  const list = $derived((model[w.path] ?? []) as any[]);

  function add() {
    if (!model[w.path]) model[w.path] = [];
    const item: any = defaults(w.columns);
    const nameCol = w.columns.find((c) => c.key === 'name');
    if (nameCol) item.name = freshName(list.map((x) => x.name ?? ''), t('Elemento'));
    model[w.path].push(item);
    onchange();
  }

  function remove(i: number) {
    model[w.path].splice(i, 1);
    onchange();
  }

  function visible(f: Field, row: any) {
    return shown(f, (k) => row?.[k] ?? model[k]);
  }

  // Una columna se muestra si alguna fila la usa (o, sin filas, si el modelo la pide).
  const cols = $derived(
    w.columns.map((f, k) => ({ f, k })).filter(({ f }) => (list.length ? list.some((row) => visible(f, row)) : visible(f, {})))
  );
</script>

{#if w.title}<h3 class="wt">{w.title}</h3>{/if}
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="scroller" onkeydown={gridKeys} onpaste={(e) => pegarEnRegistros(e, w, cols.map((x) => x.f), model, onchange)}>
  <table class="grid">
    <thead>
      <tr>
        <th class="n">#</th>
        {#each cols as { f: c } (c.key + c.label)}<th class:r={['number', 'integer', 'bound'].includes(c.kind.type)} title={c.help}>{c.label}</th>{/each}
        <th></th>
      </tr>
    </thead>
    <tbody>
      {#each list as row, i (i)}
        <tr>
          <td class="n">{i + 1}</td>
          {#each cols as { f, k }, ci (f.key + f.label)}
            {@const on = visible(f, row)}
            <td class:wide={f.kind.type === 'text' || f.kind.type === 'list' || f.kind.type === 'expr'} class:na={!on} class:expr={f.kind.type === 'expr'} title={on ? undefined : t('No se usa en esta fila')}>
              {#if on}
                <FieldInput field={f} value={row[f.key]} cell r={i} c={ci} onchange={(v) => { row[f.key] = v; onchange(); }} />
              {/if}
            </td>
          {/each}
          <td class="del"><button class="x" title={t('Quitar fila')} aria-label={t('Quitar fila {0}', i + 1)} onclick={() => remove(i)}>×</button></td>
        </tr>
      {/each}
      {#if w.add_label}
        <tr class="addr">
          <td colspan={cols.length + 2}><button class="add" onclick={add}>+ {w.add_label.replace(/^(Añadir|Add) /, '')}</button></td>
        </tr>
      {/if}
    </tbody>
  </table>
</div>

<style>
  .wt {
    font-size: 15px;
    margin: 0 0 8px;
  }
  .scroller {
    width: max-content;
    max-width: 100%;
    overflow: auto;
    background: var(--sheet);
    border: 1px solid var(--rule);
    border-radius: var(--r-md);
  }
  th,
  td {
    padding: 0;
    border-bottom: 1px solid var(--rule-soft);
    height: 36px;
    white-space: nowrap;
  }
  thead th {
    background: var(--sheet-2);
    border-bottom: 1px solid var(--rule);
    font-size: 12.5px;
    font-weight: 600;
    color: var(--ink-2);
    height: 40px;
    padding: 0 10px;
    text-align: left;
  }
  thead th.r {
    text-align: right;
  }
  .n {
    color: var(--ink-3);
    font-size: 12px;
    padding: 0 8px 0 12px;
    text-align: right;
  }
  td.wide :global(.inp) {
    min-width: 150px;
  }
  td.expr :global(.inp) {
    min-width: 360px;
    font-family: var(--font-mono, ui-monospace, monospace);
  }
  td.na {
    background: repeating-linear-gradient(135deg, transparent 0 6px, var(--rule-soft) 6px 7px);
  }
  .del {
    width: 30px;
  }
  .x {
    border: 0;
    background: none;
    color: var(--ink-3);
    width: 24px;
    height: 24px;
    border-radius: 4px;
    font-size: 16px;
    opacity: 0;
  }
  tr:hover .x,
  .x:focus-visible {
    opacity: 1;
  }
  .x:hover {
    color: var(--bad);
    background: var(--bad-tint);
  }
  .add {
    border: 1px dashed var(--rule);
    background: none;
    color: var(--ink-2);
    border-radius: var(--r-sm);
    padding: 4px 10px;
    font-size: 12.5px;
    font-weight: 550;
    margin: 4px 10px;
  }
  .add:hover {
    border-color: var(--solver);
    color: var(--solver);
  }
  .addr td {
    border-bottom: 0;
    height: 44px;
  }
</style>
