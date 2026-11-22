<script lang="ts">
  // Avisos breves abajo a la derecha (guardado, archivo abierto, ejemplo cargado).
  import Icon from './Icon.svelte';
  import { ui } from '../lib/ui.svelte';
  import { t } from '../lib/i18n.svelte';
</script>

<div class="avisos" aria-live="polite">
  {#each ui.avisos as a (a.id)}
    <div class="aviso {a.tipo}" role={a.tipo === 'error' ? 'alert' : 'status'}>
      <span class="ico"><Icon name={a.tipo === 'error' ? 'alerta' : a.tipo === 'info' ? 'info' : 'check'} size={16} stroke={2} /></span>
      <span class="txt">{a.texto}</span>
      <button aria-label={t('Cerrar aviso')} onclick={() => ui.quitar(a.id)}><Icon name="cerrar" size={14} /></button>
    </div>
  {/each}
</div>

<style>
  .avisos {
    position: fixed;
    right: 20px;
    bottom: 44px;
    z-index: 80;
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    gap: 8px;
    pointer-events: none;
  }
  .aviso {
    pointer-events: auto;
    display: flex;
    align-items: center;
    gap: 10px;
    max-width: 380px;
    padding: 10px 8px 10px 12px;
    border-radius: 14px;
    background: var(--sheet);
    border: 1px solid var(--rule);
    box-shadow: var(--shadow-pop);
    font-size: 13px;
    font-weight: 550;
    animation: entra 0.2s ease-out;
  }
  @keyframes entra {
    from {
      opacity: 0;
      transform: translateY(8px);
    }
  }
  .ico {
    display: grid;
    place-items: center;
    width: 26px;
    height: 26px;
    border-radius: 8px;
    flex: none;
    background: var(--good-tint);
    color: var(--good);
  }
  .info .ico {
    background: var(--solver-tint);
    color: var(--solver);
  }
  .error .ico {
    background: var(--bad-tint);
    color: var(--bad);
  }
  .txt {
    flex: 1;
    min-width: 0;
  }
  button {
    display: grid;
    place-items: center;
    width: 24px;
    height: 24px;
    border: 0;
    border-radius: 7px;
    background: none;
    color: var(--ink-3);
  }
  button:hover {
    background: var(--rule-soft);
    color: var(--ink);
  }
</style>
