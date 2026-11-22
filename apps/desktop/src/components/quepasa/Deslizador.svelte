<script lang="ts">
  // Deslizador con la banda del rango de sensibilidad: dentro de la banda la
  // solución base sigue siendo óptima. Marca también el valor original y pinta
  // el tramo recorrido desde él. Es un <input type="range"> nativo (teclado:
  // flechas, Re Pág / Av Pág, Inicio y Fin).
  import { num } from '../../lib/format';
  import { t } from '../../lib/i18n.svelte';

  let {
    value,
    min,
    max,
    step,
    base,
    lo = null,
    hi = null,
    banda = false,
    label,
    id,
    oninput
  }: {
    value: number;
    min: number;
    max: number;
    step: number;
    base: number;
    /** Extremos de la banda (null = sin límite por ese lado). */
    lo?: number | null;
    hi?: number | null;
    banda?: boolean;
    label: string;
    id?: string;
    oninput: (v: number) => void;
  } = $props();

  const pct = (x: number) => (max > min ? Math.min(100, Math.max(0, ((x - min) / (max - min)) * 100)) : 0);
  const bA = $derived(pct(lo ?? min));
  const bB = $derived(pct(hi ?? max));
  const fuera = $derived(banda && ((lo !== null && value < lo - 1e-9) || (hi !== null && value > hi + 1e-9)));
  const dA = $derived(Math.min(pct(base), pct(value)));
  const dB = $derived(Math.max(pct(base), pct(value)));
  const texto = $derived(
    banda ? (fuera ? t('{0}, fuera del rango de optimalidad', num(value)) : t('{0}, dentro del rango de optimalidad', num(value))) : num(value)
  );
</script>

<div class="dz" class:fuera>
  <div class="pista" aria-hidden="true">
    <div class="riel"></div>
    {#if banda}
      <div class="banda" style:left={`${bA}%`} style:width={`${Math.max(0, bB - bA)}%`} class:abiertaI={lo === null} class:abiertaD={hi === null}></div>
    {/if}
    <div class="delta" style:left={`${dA}%`} style:width={`${dB - dA}%`}></div>
    <div class="base" style:left={`${pct(base)}%`} title={t('Valor original: {0}', num(base))}></div>
  </div>
  <input
    {id}
    type="range"
    {min}
    {max}
    {step}
    {value}
    aria-label={label}
    aria-valuetext={texto}
    oninput={(e) => oninput((e.currentTarget as HTMLInputElement).valueAsNumber)}
  />
</div>

<style>
  .dz {
    --th: 16px;
    position: relative;
    height: 26px;
    min-width: 120px;
  }
  .pista {
    position: absolute;
    inset: 0 calc(var(--th) / 2);
    pointer-events: none;
  }
  .riel,
  .banda,
  .delta {
    position: absolute;
    top: 50%;
    height: 6px;
    margin-top: -3px;
    border-radius: 999px;
  }
  .riel {
    left: calc(var(--th) / -2);
    right: calc(var(--th) / -2);
    background: var(--rule);
  }
  .banda {
    height: 12px;
    margin-top: -6px;
    background: var(--good-tint);
    box-shadow: inset 0 0 0 1px color-mix(in oklab, var(--good) 55%, transparent);
    border-radius: 4px;
  }
  .banda.abiertaI {
    border-top-left-radius: 0;
    border-bottom-left-radius: 0;
  }
  .banda.abiertaD {
    border-top-right-radius: 0;
    border-bottom-right-radius: 0;
  }
  .delta {
    height: 4px;
    margin-top: -2px;
    border-radius: 0;
    background: var(--solver);
    opacity: 0.85;
  }
  .fuera .delta {
    background: var(--binding);
  }
  .base {
    position: absolute;
    top: 50%;
    width: 2px;
    height: 16px;
    margin: -8px 0 0 -1px;
    border-radius: 1px;
    background: var(--ink-2);
  }
  input {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    margin: 0;
    background: transparent;
    -webkit-appearance: none;
    appearance: none;
    cursor: pointer;
  }
  input:focus-visible {
    outline: 2px solid var(--focus);
    outline-offset: 2px;
    border-radius: 6px;
  }
  input::-webkit-slider-runnable-track {
    height: 26px;
    background: transparent;
  }
  input::-moz-range-track {
    height: 26px;
    background: transparent;
  }
  input::-webkit-slider-thumb {
    -webkit-appearance: none;
    width: var(--th);
    height: var(--th);
    margin-top: 5px;
    border-radius: 50%;
    background: var(--sheet);
    border: 2px solid var(--solver);
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.3);
  }
  input::-moz-range-thumb {
    width: 12px;
    height: 12px;
    border-radius: 50%;
    background: var(--sheet);
    border: 2px solid var(--solver);
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.3);
  }
  .fuera input::-webkit-slider-thumb {
    border-color: var(--binding);
  }
  .fuera input::-moz-range-thumb {
    border-color: var(--binding);
  }
</style>
