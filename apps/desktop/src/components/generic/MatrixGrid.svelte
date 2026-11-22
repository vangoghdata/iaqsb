<script lang="ts">
  import type { Widget } from '../../lib/gtypes';
  import FieldInput from './FieldInput.svelte';
  import { t } from '../../lib/i18n.svelte';
  import { freshName, gridKeys } from './gridnav';
  import { defaults } from '../../lib/shell.svelte';
  import { pegarEnMatriz } from '../../lib/excel';

  type MatrixW = Extract<Widget, { type: 'matrix' }>;
  let { w, model, onchange, siblings = [] }: { w: MatrixW; model: any; onchange: () => void; siblings?: MatrixW[] } = $props();

  // todas las matrices del editor: al añadir o quitar un elemento de una lista,
  // se ajustan todas las que la usan (filas, columnas o ambas)
  const all = $derived(siblings.length ? siblings : [w]);

  const square = $derived(w.rows.path === w.cols.path);
  const rows = $derived((model[w.rows.path] ?? []) as any[]);
  const cols = $derived((model[w.cols.path] ?? []) as any[]);
  const mat = $derived((model[w.path] ?? []) as any[][]);

  function blankOf(s: MatrixW) {
    return s.cell.optional ? null : (s.cell.default ?? 0);
  }

  function grow(path: string) {
    for (const s of all) {
      const mtx = (model[s.path] ??= []) as any[][];
      const b = blankOf(s);
      if (s.rows.path === path && s.cols.path === path) {
        for (const row of mtx) row.push(b);
        mtx.push(Array(model[path].length).fill(b));
      } else if (s.cols.path === path) {
        for (const row of mtx) row.push(b);
      } else if (s.rows.path === path) {
        mtx.push(Array((model[s.cols.path] ?? []).length).fill(b));
      }
    }
  }

  function shrink(path: string, i: number) {
    for (const s of all) {
      const mtx = (model[s.path] ?? []) as any[][];
      if (s.rows.path === path) mtx.splice(i, 1);
      if (s.cols.path === path) for (const row of mtx) row.splice(i, 1);
    }
  }

  function newItem(path: string, label: string, key: string) {
    const list = (model[path] ??= []) as any[];
    const extras = all.flatMap((s) => [...(s.rows.path === path ? s.rows.extra : []), ...(s.cols.path === path ? s.cols.extra : [])]);
    return { [key]: freshName(list.map((x) => x[key]), label), ...defaults(extras) };
  }

  function addRow() {
    const item = newItem(w.rows.path, w.rows.label, w.rows.label_key);
    model[w.rows.path].push(item);
    grow(w.rows.path);
    onchange();
  }

  function addCol() {
    const item = newItem(w.cols.path, w.cols.label, w.cols.label_key);
    model[w.cols.path].push(item);
    grow(w.cols.path);
    onchange();
  }

  function removeRow(i: number) {
    if (rows.length <= 1) return;
    model[w.rows.path].splice(i, 1);
    shrink(w.rows.path, i);
    onchange();
  }

  function removeCol(j: number) {
    if (cols.length <= 1 || square) return;
    model[w.cols.path].splice(j, 1);
    shrink(w.cols.path, j);
    onchange();
  }

  function rename(list: any[], key: string, i: number, e: Event) {
    const v = (e.currentTarget as HTMLInputElement).value.trim();
    if (!v) return;
    list[i][key] = v;
    onchange();
  }

  const nExtra = $derived(w.rows.extra.length);
</script>

{#if w.title}<h3 class="wt">{w.title}</h3>{/if}
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="scroller" onkeydown={gridKeys} onpaste={(e) => pegarEnMatriz(e, w, all, model, onchange)}>
  <table class="grid">
    <thead>
      <tr>
        <th class="corner">{w.rows.label} \ {w.cols.label}</th>
        {#each cols as col, j (j)}
          <th class="ch">
            <div class="hn">
              <input class="name r" data-hc={j} value={col[w.cols.label_key]} aria-label={t('Nombre de {0} {1}', w.cols.label, j + 1)} onchange={(e) => rename(model[w.cols.path], w.cols.label_key, j, e)} />
              {#if !square && cols.length > 1}
                <button class="x" title={t('Quitar')} aria-label={t('Quitar {0}', col[w.cols.label_key])} onclick={() => removeCol(j)}>×</button>
              {/if}
            </div>
          </th>
        {/each}
        {#if w.cols.add_label && !square}
          <th class="addc"><button class="add" onclick={addCol}>+ {w.cols.add_label.replace(/^(Añadir|Add) /, '')}</button></th>
        {/if}
        {#each w.rows.extra as f}<th class="ex">{f.label}</th>{/each}
      </tr>
    </thead>
    <tbody>
      {#each rows as row, i (i)}
        <tr>
          <th class="rh">
            <div class="hn">
              <input class="name" data-hr={i} value={row[w.rows.label_key]} aria-label={t('Nombre de {0} {1}', w.rows.label, i + 1)} onchange={(e) => rename(model[w.rows.path], w.rows.label_key, i, e)} />
              {#if rows.length > 1 && w.rows.add_label}
                <button class="x" title={t('Quitar')} aria-label={t('Quitar {0}', row[w.rows.label_key])} onclick={() => removeRow(i)}>×</button>
              {/if}
            </div>
          </th>
          {#each cols as _, j (j)}
            <td class:diag={square && i === j && !w.allow_diagonal}>
              <FieldInput
                field={w.cell}
                value={mat[i]?.[j]}
                cell
                r={i}
                c={j}
                disabled={square && i === j && !w.allow_diagonal}
                onchange={(v) => {
                  model[w.path][i][j] = v;
                  onchange();
                }}
              />
            </td>
          {/each}
          {#if w.cols.add_label && !square}<td></td>{/if}
          {#each w.rows.extra as f, k}
            <td class="exc">
              <FieldInput field={f} value={row[f.key]} cell r={i} c={cols.length + 1 + k} onchange={(v) => { row[f.key] = v; onchange(); }} />
            </td>
          {/each}
        </tr>
      {/each}
      {#each w.cols.extra as f, k}
        <tr class="exr">
          <th class="rh">{f.label}</th>
          {#each cols as col, j (j)}
            <td>
              <FieldInput field={f} value={col[f.key]} cell r={rows.length + k} c={j} onchange={(v) => { col[f.key] = v; onchange(); }} />
            </td>
          {/each}
          {#if w.cols.add_label && !square}<td></td>{/if}
          {#each Array(nExtra) as _}<td></td>{/each}
        </tr>
      {/each}
      {#if w.rows.add_label}
        <tr class="addr">
          <th class="rh"><button class="add" onclick={addRow}>+ {w.rows.add_label.replace(/^(Añadir|Add) /, '')}</button></th>
          <td colspan={cols.length + nExtra + 1}></td>
        </tr>
      {/if}
    </tbody>
  </table>
</div>
{#if w.cell.optional}<p class="hint">{t('Una celda vacía significa que no hay conexión.')}</p>{/if}

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
    padding: 0 4px;
  }
  .corner {
    text-align: left;
    padding: 0 10px !important;
    font-weight: 500 !important;
    color: var(--ink-3) !important;
  }
  .rh {
    text-align: left;
    position: sticky;
    left: 0;
    z-index: 1;
    background: var(--sheet);
    padding: 0 4px;
    font-weight: 550;
    font-size: 13px;
    color: var(--ink-2);
  }
  .hn {
    display: flex;
    align-items: center;
  }
  .name {
    border: 0;
    background: transparent;
    padding: 6px;
    border-radius: var(--r-sm);
    font-weight: 650;
    color: var(--ink);
    width: 112px;
  }
  .name.r {
    text-align: right;
    width: 96px;
  }
  .name:hover {
    background: var(--rule-soft);
  }
  .x {
    border: 0;
    background: none;
    color: var(--ink-3);
    width: 22px;
    height: 22px;
    border-radius: 4px;
    opacity: 0;
    font-size: 16px;
  }
  .hn:hover .x,
  .x:focus-visible {
    opacity: 1;
  }
  .x:hover {
    color: var(--bad);
    background: var(--bad-tint);
  }
  .ex {
    color: var(--solver) !important;
    text-align: right;
    padding: 0 10px !important;
    border-left: 1px solid var(--rule);
  }
  .exc {
    border-left: 1px solid var(--rule);
    background: var(--solver-tint);
  }
  .exr td,
  .exr th {
    background: var(--solver-tint);
    border-top: 1px solid var(--rule);
  }
  .exr th {
    color: var(--solver);
    padding: 0 10px;
  }
  td.diag {
    background: var(--rule-soft);
  }
  .add {
    border: 1px dashed var(--rule);
    background: none;
    color: var(--ink-2);
    border-radius: var(--r-sm);
    padding: 4px 10px;
    font-size: 12.5px;
    font-weight: 550;
    margin: 0 6px;
  }
  .add:hover {
    border-color: var(--solver);
    color: var(--solver);
  }
  .addr th,
  .addr td {
    height: 44px;
    border-bottom: 0;
  }
  .hint {
    margin: 6px 2px 0;
    font-size: 12.5px;
    color: var(--ink-3);
  }
</style>
