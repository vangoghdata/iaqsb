<script lang="ts">
  // Comentarios del modo práctica (acierto, aviso, error, pista).
  import type { Nota } from '../../lib/practica_sesion.svelte';
  import type { Msg } from '../../lib/practica';
  import Icon from '../Icon.svelte';
  import { t } from '../../lib/i18n.svelte';

  let { notas = [], pistas = [] }: { notas?: Nota[]; pistas?: Msg[] } = $props();
  const icono = { ok: 'check', warn: 'alerta', bad: 'cerrar', info: 'info' } as const;
  const tx = (m: Msg) => t(m.key, ...(m.args ?? []));
</script>

{#if notas.length || pistas.length}
  <div class="notas" aria-live="polite">
    {#each notas as n, i (i)}
      <div class="nota {n.level}">
        <span class="ic"><Icon name={icono[n.level]} size={15} stroke={2} /></span>
        <div>
          {#each n.msgs as m}<p>{tx(m)}</p>{/each}
        </div>
      </div>
    {/each}
    {#each pistas as m, i (i)}
      <div class="nota pista">
        <span class="ic"><Icon name="chispa" size={15} stroke={2} /></span>
        <div><p><strong>{t('Pista {0}:', i + 1)}</strong> {tx(m)}</p></div>
      </div>
    {/each}
  </div>
{/if}

<style>
  .notas {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .nota {
    display: flex;
    gap: 10px;
    align-items: flex-start;
    padding: 8px 12px;
    border-radius: var(--r-sm);
    border: 1px solid var(--rule);
    background: var(--sheet);
    font-size: 13px;
  }
  .nota p {
    margin: 0;
  }
  .nota p + p {
    margin-top: 3px;
    color: var(--ink-2);
  }
  .ic {
    display: inline-flex;
    margin-top: 1px;
  }
  .ok {
    background: var(--good-tint);
    border-color: color-mix(in oklab, var(--good) 35%, transparent);
  }
  .ok .ic {
    color: var(--good);
  }
  .warn {
    background: var(--binding-tint);
    border-color: color-mix(in oklab, var(--binding) 35%, transparent);
  }
  .warn .ic {
    color: var(--binding);
  }
  .bad {
    background: var(--bad-tint);
    border-color: color-mix(in oklab, var(--bad) 35%, transparent);
  }
  .bad .ic {
    color: var(--bad);
  }
  .info .ic,
  .pista .ic {
    color: var(--solver);
  }
  .pista {
    background: var(--solver-tint);
    border-color: color-mix(in oklab, var(--solver) 30%, transparent);
  }
</style>
