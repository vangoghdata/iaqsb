<script lang="ts">
  import type { TableBlock } from '../../lib/gtypes';
  import { cell } from './fmt';
  import { copiable } from '../../lib/excel';

  let { t }: { t: TableBlock } = $props();

  const marks = $derived(new Map((t.marks ?? []).map((m) => [`${m.row}:${m.col}`, m.tone])));
  const colMarks = $derived(new Map((t.col_marks ?? []).map(([c, tone]) => [c, tone])));
  const align = (j: number) => (t.columns[j]?.align === 'left' ? 'l' : 'r');
</script>

<div class="tb">
  {#if t.title}<h3>{t.title}</h3>{/if}
  <div class="tw" use:copiable>
    <table>
      <thead>
        <tr>
          {#each t.columns as c, j}
            <th class:l={c.align === 'left'} class={colMarks.get(j) ? `c-${colMarks.get(j)}` : ''}>{c.label}</th>
          {/each}
        </tr>
      </thead>
      <tbody>
        {#each t.rows as r, i}
          <tr class={`r-${r.tone ?? 'normal'}`}>
            {#each r.cells as v, j}
              {@const m = marks.get(`${i}:${j}`) ?? (colMarks.get(j) ? `col-${colMarks.get(j)}` : '')}
              <td class={`${align(j)} ${m ? 'm-' + m : ''}`}>{cell(v)}</td>
            {/each}
          </tr>
        {/each}
      </tbody>
    </table>
  </div>
  {#if t.note}<p class="note">{t.note}</p>{/if}
</div>

<style>
  h3 {
    font-size: 15px;
    margin: 0 0 8px;
  }
  .tw {
    width: max-content;
    max-width: 100%;
    overflow: auto;
    background: var(--sheet);
    border: 0.5px solid var(--rule);
    border-radius: var(--r-lg);
  }
  th,
  td {
    padding: 7px 14px;
    border-bottom: 1px solid var(--rule-soft);
    white-space: nowrap;
    text-align: right;
    font-variant-numeric: tabular-nums;
  }
  th {
    font-size: 12.5px;
    font-weight: 600;
    color: var(--ink-2);
    background: var(--sheet-2);
    border-bottom: 1px solid var(--rule);
  }
  th.l,
  td.l {
    text-align: left;
  }
  td.l:first-child {
    font-weight: 600;
  }
  tr:last-child td {
    border-bottom: 0;
  }
  tr.r-accent td {
    background: var(--solver-tint);
  }
  tr.r-accent td:first-child {
    box-shadow: inset 3px 0 0 var(--solver);
  }
  tr.r-highlight td {
    background: var(--binding-tint);
  }
  tr.r-highlight td:first-child {
    box-shadow: inset 3px 0 0 var(--binding);
  }
  tr.r-good td:first-child {
    box-shadow: inset 3px 0 0 var(--good);
  }
  tr.r-bad td {
    background: var(--bad-tint);
  }
  tr.r-muted td {
    color: var(--ink-3);
  }
  tr.r-total td {
    font-weight: 700;
    border-top: 1px solid var(--rule);
  }
  td.m-accent {
    background: var(--solver-tint);
    color: var(--solver);
    font-weight: 600;
  }
  td.m-highlight {
    background: var(--binding-tint);
    outline: 2px solid var(--binding);
    outline-offset: -3px;
    font-weight: 700;
  }
  td.m-bad {
    background: var(--bad-tint);
    color: var(--bad);
    font-weight: 600;
  }
  td.m-good {
    color: var(--good);
    font-weight: 600;
  }
  td.m-muted {
    color: var(--ink-3);
  }
  td.m-col-accent,
  th.c-accent {
    background: var(--solver-tint);
  }
  td.m-col-highlight,
  th.c-highlight {
    background: var(--binding-tint);
  }
  .note {
    margin: 6px 2px 0;
    font-size: 12.5px;
    color: var(--ink-2);
    max-width: 90ch;
  }
</style>
