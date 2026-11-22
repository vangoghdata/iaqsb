<script lang="ts">
  import type { GenState } from '../../lib/shell.svelte';
  import { defaults } from '../../lib/shell.svelte';
  import FieldInput from './FieldInput.svelte';
  import { t } from '../../lib/i18n.svelte';

  let { st }: { st: GenState } = $props();
  let dlg: HTMLDialogElement;
  let type = $state('');
  let dims = $state<Record<string, unknown>>({});
  let err = $state<string | null>(null);

  const pt = $derived(st.spec?.problem_types.find((p) => p.id === type));

  $effect(() => {
    if (st.newOpen) {
      type = st.model?.problem_type ?? st.spec?.problem_types[0]?.id ?? '';
      err = null;
      dlg?.showModal();
    } else {
      dlg?.close();
    }
  });

  $effect(() => {
    if (pt) dims = defaults(pt.dims);
  });

  async function create() {
    try {
      await st.newProblem(type, $state.snapshot(dims));
      st.newOpen = false;
    } catch (e) {
      err = e instanceof Error ? e.message : String(e);
    }
  }
</script>

<dialog bind:this={dlg} onclose={() => (st.newOpen = false)} aria-labelledby="np-title">
  <h2 id="np-title">{t('Problema nuevo')}</h2>
  <div class="types" role="radiogroup" aria-label={t('Tipo de problema')}>
    {#each st.spec?.problem_types ?? [] as p (p.id)}
      <button role="radio" aria-checked={type === p.id} class:on={type === p.id} onclick={() => (type = p.id)}>
        <span class="tl">{p.label}</span>
        <span class="td">{p.description}</span>
      </button>
    {/each}
  </div>
  {#if pt && pt.dims.length}
    <div class="dims">
      {#each pt.dims as f (f.key)}
        <label>
          <span>{f.label}</span>
          <FieldInput field={f} value={dims[f.key]} onchange={(v) => (dims[f.key] = v)} />
        </label>
      {/each}
    </div>
  {/if}
  {#if err}<p class="err">{err}</p>{/if}
  <div class="actions">
    <button class="btn" onclick={() => (st.newOpen = false)}>{t('Cancelar')}</button>
    <button class="btn primary" onclick={create}>{t('Crear problema')}</button>
  </div>
</dialog>

<style>
  dialog {
    width: min(680px, calc(100vw - 32px));
    border: 1px solid var(--rule);
    border-radius: var(--r-lg);
    box-shadow: var(--shadow-pop);
    padding: 24px 26px 20px;
    background: var(--sheet);
    color: var(--ink);
  }
  dialog::backdrop {
    background: var(--overlay);
  }
  h2 {
    font-size: 21px;
    margin-bottom: 14px;
  }
  .types {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(190px, 1fr));
    gap: 8px;
    max-height: 46vh;
    overflow: auto;
  }
  .types button {
    display: flex;
    flex-direction: column;
    gap: 3px;
    text-align: left;
    padding: 10px 12px;
    border: 1px solid var(--rule);
    border-radius: var(--r-md);
    background: var(--rule-soft);
  }
  .types button:hover {
    background: var(--rule);
  }
  .types button.on {
    border-color: var(--accent-btn);
    background: var(--solver-tint);
  }
  .tl {
    font-weight: 650;
  }
  .td {
    font-size: 12.5px;
    color: var(--ink-2);
  }
  .dims {
    display: flex;
    flex-wrap: wrap;
    gap: 14px;
    margin-top: 16px;
  }
  .dims label {
    display: flex;
    flex-direction: column;
    gap: 4px;
    width: 180px;
    font-size: 12.5px;
    font-weight: 600;
    color: var(--ink-2);
  }
  .err {
    color: var(--bad);
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 18px;
  }
</style>
