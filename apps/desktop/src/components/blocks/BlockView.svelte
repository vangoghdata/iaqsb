<script lang="ts">
  import type { Block } from '../../lib/gtypes';
  import TableView from './TableView.svelte';
  import ChartView from './ChartView.svelte';
  import NetworkView from './NetworkView.svelte';
  import GanttView from './GanttView.svelte';
  import TreeView from './TreeView.svelte';
  import MapView from './MapView.svelte';
  import StepsView from './StepsBlock.svelte';
  import SourcesView from './SourcesView.svelte';
  import LayoutView from './LayoutView.svelte';
  import Figure from './Figure.svelte';
  import { cell } from './fmt';

  let { block }: { block: Block } = $props();
</script>

{#if block.type === 'metrics'}
  <div class="metrics">
    {#each block.items as m}
      <div class="metric" class:main={m.emphasis}>
        <span class="ml">{m.label}</span>
        <span class="mv">{cell(m.value)}{#if m.unit}<span class="mu"> {m.unit}</span>{/if}</span>
      </div>
    {/each}
  </div>
{:else if block.type === 'text'}
  <div class="text" class:bad={block.tone === 'bad'} class:hl={block.tone === 'highlight'}>
    {#if block.title}<h3>{block.title}</h3>{/if}
    <p>{block.body}</p>
  </div>
{:else if block.type === 'table'}
  <TableView t={block} />
{:else if block.type === 'chart'}
  <Figure {block}><ChartView c={block} /></Figure>
{:else if block.type === 'network'}
  <Figure {block}><NetworkView n={block} /></Figure>
{:else if block.type === 'gantt'}
  <Figure {block}><GanttView g={block} /></Figure>
{:else if block.type === 'tree'}
  <Figure {block}><TreeView t={block} /></Figure>
{:else if block.type === 'map'}
  <Figure {block}><MapView m={block} /></Figure>
{:else if block.type === 'layout'}
  <Figure {block}><LayoutView p={block} /></Figure>
{:else if block.type === 'sources'}
  <SourcesView s={block} />
{:else if block.type === 'steps'}
  <StepsView s={block} />
{/if}

<style>
  .metrics {
    display: flex;
    flex-wrap: wrap;
    gap: 10px 36px;
    align-items: baseline;
  }
  .metric {
    display: flex;
    flex-direction: column;
  }
  .ml {
    font-size: 12.5px;
    color: var(--ink-2);
  }
  .mv {
    font-weight: 650;
    font-size: 17px;
    font-variant-numeric: tabular-nums;
  }
  .metric.main .mv {
    font-family: var(--font-brand);
    font-size: 40px;
    font-weight: 700;
    letter-spacing: -0.02em;
    line-height: 1.1;
    color: var(--solver);
  }
  .mu {
    font-size: 13px;
    color: var(--ink-2);
  }
  .text p {
    margin: 0;
    max-width: 78ch;
  }
  .text h3 {
    font-size: 15px;
    margin-bottom: 6px;
  }
  .text.bad {
    padding: 12px 14px;
    border-left: 3px solid var(--bad);
    background: var(--bad-tint);
    border-radius: 0 var(--r-sm) var(--r-sm) 0;
  }
  .text.hl {
    padding: 12px 14px;
    border-left: 3px solid var(--binding);
    background: var(--binding-tint);
    border-radius: 0 var(--r-sm) var(--r-sm) 0;
  }
</style>
