<script lang="ts">
  import type { Field } from '../../lib/gtypes';
  import { parseNum } from '../../lib/format';
  import { t } from '../../lib/i18n.svelte';

  let {
    field,
    value,
    onchange,
    cell = false,
    disabled = false,
    r,
    c
  }: {
    field: Field;
    value: unknown;
    onchange: (v: unknown) => void;
    cell?: boolean;
    disabled?: boolean;
    r?: number;
    c?: number;
  } = $props();

  let bad = $state(false);

  function show(v: unknown): string {
    if (v === null || v === undefined) return '';
    if (Array.isArray(v)) return v.join(', ');
    if (typeof v === 'number') {
      if (!Number.isFinite(v)) return v > 0 ? 'M' : '-M';
      return String(v);
    }
    return String(v);
  }

  function commit(e: Event) {
    const raw = (e.currentTarget as HTMLInputElement).value;
    const k = field.kind.type;
    if (k === 'number' || k === 'integer' || k === 'bound') {
      if (raw.trim() === '') {
        bad = false;
        onchange(field.optional ? null : 0);
        return;
      }
      const u = raw.trim().toUpperCase();
      if (k === 'bound' && (u === 'M' || u === '-M' || u === '∞')) {
        bad = false;
        onchange(u === '-M' ? '-M' : 'M');
        return;
      }
      const v = parseNum(raw);
      if (v === null || (k === 'integer' && !Number.isInteger(v))) {
        bad = true;
        return;
      }
      bad = false;
      onchange(v);
    } else if (k === 'list') {
      onchange(
        raw
          .split(/[,;]/)
          .map((s) => s.trim())
          .filter(Boolean)
      );
    } else {
      onchange(raw);
    }
  }
</script>

{#if field.kind.type === 'bool'}
  <input
    type="checkbox"
    class="chk"
    checked={!!value}
    {disabled}
    aria-label={field.label}
    data-r={r}
    data-c={c}
    onchange={(e) => onchange((e.currentTarget as HTMLInputElement).checked)}
  />
{:else if field.kind.type === 'choice'}
  <select
    class:cell
    class="sel"
    value={String(value ?? '')}
    {disabled}
    aria-label={field.label}
    data-r={r}
    data-c={c}
    onchange={(e) => onchange((e.currentTarget as HTMLSelectElement).value)}
  >
    {#each field.kind.options as [v, l]}<option value={v}>{l}</option>{/each}
  </select>
{:else}
  <input
    class:cell
    class:num={['number', 'integer', 'bound'].includes(field.kind.type)}
    class:bad
    class:zero={value === 0}
    class:empty={value === null || value === undefined || value === ''}
    class="inp"
    inputmode={['number', 'integer', 'bound'].includes(field.kind.type) ? 'decimal' : undefined}
    value={show(value)}
    placeholder={field.optional && cell ? '' : undefined}
    {disabled}
    aria-label={field.label}
    title={bad ? t('No es un valor válido') : field.help}
    data-r={r}
    data-c={c}
    onchange={commit}
  />
{/if}

<style>
  .inp {
    width: 100%;
    height: 36px;
    border: 1px solid var(--rule);
    border-radius: var(--r-sm);
    background: var(--field);
    padding: 0 11px;
    transition: border-color 0.15s, box-shadow 0.15s;
  }
  .inp:not(.cell):focus {
    outline: none;
    border-color: var(--focus);
    box-shadow: 0 0 0 3px var(--solver-tint);
  }
  .inp.zero {
    color: var(--ink-3);
  }
  .inp.zero:focus {
    color: var(--ink);
  }
  .inp.cell {
    border: 0;
    border-radius: 0;
    background: transparent;
    height: 35px;
    min-width: 92px;
    padding: 0 10px;
  }
  .inp.num {
    text-align: right;
    font-variant-numeric: tabular-nums;
  }
  .inp.cell:hover {
    background: var(--rule-soft);
  }
  .inp.cell:focus {
    background: var(--solver-tint);
    outline: 2px solid var(--solver);
    outline-offset: -2px;
  }
  .inp.bad {
    background: var(--bad-tint);
    outline: 2px solid var(--bad);
    outline-offset: -2px;
  }
  .sel {
    border: 1px solid var(--rule);
    background-color: var(--field);
    height: 36px;
    padding: 0 10px;
  }
  .sel.cell {
    border: 0;
    background-color: transparent;
    height: 35px;
    min-width: 110px;
  }
  .chk {
    width: 16px;
    height: 16px;
    accent-color: var(--accent-btn);
  }
</style>
