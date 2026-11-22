<script lang="ts">
  // Línea con lo que hizo el motor: cuál, cuánto tardó, tamaño, nodos y brecha.
  import Icon from './Icon.svelte';
  import { num } from '../lib/format';
  import { t } from '../lib/i18n.svelte';
  import type { EngineInfo } from '../lib/types';

  let { engine }: { engine: EngineInfo } = $props();

  const partes = $derived.by(() => {
    const e = engine;
    const p: string[] = [];
    if (e.runs && e.runs > 1) p.push(t('{0} resoluciones', e.runs));
    if (typeof e.time === 'number') p.push(e.time < 0.001 ? '< 1 ms' : e.time < 1 ? `${Math.round(e.time * 1000)} ms` : `${num(Math.round(e.time * 100) / 100)} s`);
    if (e.cols != null && e.rows != null) p.push(t('{0} variables × {1} restricciones', e.cols.toLocaleString(), e.rows.toLocaleString()));
    if (e.is_mip && e.nodes != null) p.push(t('{0} nodos', e.nodes.toLocaleString()));
    if (e.gap != null && (e.is_mip || e.runs)) p.push(t('brecha {0} %', num(Math.round(e.gap * 1e6) / 1e4)));
    if (e.is_mip && e.bound != null) p.push(t('cota {0}', num(e.bound)));
    return p;
  });
  const limite = $derived(engine.limited || ['time_limit', 'node_limit', 'solution_limit', 'iteration_limit', 'interrupted'].includes(engine.status ?? ''));
</script>

<p class="motor" class:lim={limite}>
  <Icon name="motor" size={14} />
  <b>{engine.solver}</b>
  {#each partes as p}<span>· {p}</span>{/each}
  {#if limite}<span class="aviso">· {t('se alcanzó un límite: es la mejor solución encontrada, puede no ser la óptima')}</span>{/if}
</p>

<style>
  .motor {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 4px 6px;
    margin: 6px 0 0;
    font-size: 12px;
    color: var(--ink-3);
    font-variant-numeric: tabular-nums;
  }
  .motor b {
    font-weight: 600;
    color: var(--ink-2);
  }
  .aviso {
    color: color-mix(in srgb, var(--binding) 85%, var(--ink));
  }
</style>
