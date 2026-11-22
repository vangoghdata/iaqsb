<script lang="ts">
  // Solución de un modelo algebraico por índices: x[i,j] como tabla de doble
  // entrada, familias de restricciones con su precio sombra y el tamaño del modelo.
  import TableView from '../blocks/TableView.svelte';
  import AlgebraStats from './AlgebraStats.svelte';
  import type { AlgReport } from '../../lib/algebra.svelte';
  import type { TableBlock } from '../../lib/gtypes';
  import { t } from '../../lib/i18n.svelte';

  let { r }: { r: AlgReport } = $props();

  const FILAS = 100;
  let todas = $state<Record<number, boolean>>({});

  // los índices a la izquierda; en las tablas de doble entrada se resaltan los valores distintos de cero
  function preparar(tb: TableBlock): TableBlock {
    const columns = tb.columns.map((c) => ({ ...c, align: c.align ?? 'left' }));
    const marks: NonNullable<TableBlock['marks']> = [];
    if (tb.rows.some((x) => x.tone === 'total')) {
      tb.rows.forEach((row, i) => {
        if (row.tone === 'total') return;
        row.cells.forEach((v, j) => {
          if (j > 0 && j < row.cells.length - 1 && typeof v === 'number' && Math.abs(v) > 1e-9) marks.push({ row: i, col: j, tone: 'accent' });
        });
      });
    }
    return { ...tb, columns, marks: [...(tb.marks ?? []), ...marks] };
  }

  // las tablas llegan en orden: una por familia de variables (las de r.algebra.families
  // con valores) y después una por familia de restricciones. Las que no tienen índices
  // (una sola fila) se juntan en una tabla para no llenar la pantalla de tablas pequeñas.
  const partes = $derived.by(() => {
    const ts = r.tables ?? [];
    const fams = (r.algebra?.families ?? []).filter((f) => f.count > 0);
    const nVar = Math.min(fams.length, ts.length);
    const escalarCons = (tb: TableBlock) =>
      tb.rows.length === 1 && tb.columns.length === 3 && typeof tb.rows[0].cells[0] === 'string' && !!tb.title?.endsWith(' ' + tb.rows[0].cells[0]);
    const juntar = (ts: TableBlock[], title: string): TableBlock[] =>
      ts.length === 0 ? [] : ts.length === 1 ? ts : [{ ...ts[0], title, rows: ts.flatMap((x) => x.rows), note: undefined }];
    const varsIdx = ts.slice(0, nVar).filter((_, k) => fams[k].indices.length > 0);
    const varsEsc = ts.slice(0, nVar).filter((_, k) => fams[k].indices.length === 0);
    const cons = ts.slice(nVar);
    const consIdx = cons.filter((x) => !escalarCons(x));
    const consEsc = cons.filter(escalarCons);
    // sin ningún índice, las tablas de abajo (Variables de decisión, Restricciones) ya lo dicen todo
    const conIndices = varsIdx.length > 0 || consIdx.length > 0;
    const lista = conIndices
      ? [...varsIdx, ...juntar(varsEsc, t('Variables sin índices')), ...consIdx, ...juntar(consEsc, t('Restricciones sin índices'))]
      : [];
    return { conIndices, tablas: lista.map(preparar) };
  });
  const tablas = $derived(partes.tablas);
  const indexado = $derived(!!r.algebra && (r.algebra.sets.length > 0 || r.algebra.families.some((f) => f.indices.length > 0)));

  function recortar(tb: TableBlock, k: number): TableBlock {
    if (todas[k] || tb.rows.length <= FILAS + 10) return tb;
    return { ...tb, rows: tb.rows.slice(0, FILAS), marks: tb.marks?.filter((m) => m.row < FILAS) };
  }
</script>

{#if tablas.length || r.algebra}
  <section class="asol">
    <h2>{indexado ? t('Solución por índices') : t('Modelo algebraico')}</h2>
    {#if r.algebra}
      <div class="res card"><AlgebraStats s={r.algebra} compacto /></div>
    {/if}
    {#if tablas.length}
      <div class="tablas">
        {#each tablas as tb, k (k)}
          <div class="una">
            <TableView t={recortar(tb, k)} />
            {#if !todas[k] && tb.rows.length > FILAS + 10}
              <button class="link mas" onclick={() => (todas[k] = true)}>{t('Mostrar las {0} filas', tb.rows.length.toLocaleString())}</button>
            {/if}
          </div>
        {/each}
      </div>
    {:else if indexado && r.status !== 'optimal' && r.status !== 'feasible'}
      <p class="muted">{t('Sin solución no hay valores por índices.')}</p>
    {/if}
  </section>
{/if}

<style>
  .asol {
    display: grid;
    gap: 14px;
  }
  h2 {
    font-size: 17px;
    margin: 0;
  }
  .res {
    padding: 12px 16px;
    width: max-content;
    max-width: 100%;
  }
  .tablas {
    display: flex;
    flex-wrap: wrap;
    gap: 22px 28px;
    align-items: flex-start;
  }
  .una {
    min-width: 0;
    max-width: 100%;
    display: grid;
    gap: 6px;
  }
  .mas {
    justify-self: start;
    font-size: 12.5px;
  }
</style>
