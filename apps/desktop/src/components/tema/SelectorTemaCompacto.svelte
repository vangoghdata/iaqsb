<script lang="ts">
  // Cuatro botones juntos para cambiar el tema, como en el menú de cuenta de IA Nativa.
  import Icon from '../Icon.svelte';
  import { ui } from '../../lib/ui.svelte';
  import { t } from '../../lib/i18n.svelte';
  import type { PreferenciaTema } from '../../lib/tema';
  import type { NombreIcono } from '../../lib/iconos';

  const OPCIONES: { valor: PreferenciaTema; titulo: string; icono: NombreIcono }[] = [
    { valor: 'claro', titulo: 'Claro', icono: 'sol' },
    { valor: 'oscuro', titulo: 'Oscuro', icono: 'luna' },
    { valor: 'sistema', titulo: 'Automático', icono: 'pantalla' },
    { valor: 'personalizado', titulo: 'Personalizado', icono: 'paleta' }
  ];
</script>

<div class="grupo" role="radiogroup" aria-label={t('Tema')}>
  {#each OPCIONES as o (o.valor)}
    <button
      role="radio"
      aria-checked={ui.tema === o.valor}
      aria-label={t(o.titulo)}
      title={t(o.titulo)}
      class:on={ui.tema === o.valor}
      onclick={() => ui.elegirTema(o.valor)}
    >
      <Icon name={o.icono} size={16} />
    </button>
  {/each}
</div>

<style>
  .grupo {
    display: inline-flex;
    gap: 2px;
    padding: 3px;
    border-radius: 12px;
    background: color-mix(in oklab, var(--barra-fg) 6%, transparent);
  }
  button {
    display: grid;
    place-items: center;
    width: 32px;
    height: 28px;
    border: 0;
    border-radius: 9px;
    background: none;
    color: var(--barra-fg-2);
    transition: background-color 0.15s, color 0.15s;
  }
  button:hover {
    color: var(--barra-fg);
  }
  button.on {
    background: var(--sheet);
    color: var(--ink);
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.25);
  }
</style>
