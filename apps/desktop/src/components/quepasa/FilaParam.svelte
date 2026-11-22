<script lang="ts">
  // Un coeficiente del programa lineal: deslizador con su banda de sensibilidad,
  // casilla para escribir el valor, restablecer y elegirlo para la gráfica.
  import Deslizador from './Deslizador.svelte';
  import Icon from '../Icon.svelte';
  import { num, parseNum } from '../../lib/format';
  import { t } from '../../lib/i18n.svelte';
  import { dentro, niceStep, snap, type ParamLp } from '../../lib/quepasa.svelte';

  let {
    p,
    value,
    from,
    to,
    elegido = false,
    onvalue,
    onreset,
    onelegir
  }: {
    p: ParamLp;
    value: number;
    from: number;
    to: number;
    elegido?: boolean;
    onvalue: (v: number) => void;
    onreset: () => void;
    onelegir: () => void;
  } = $props();

  const step = $derived(niceStep(to - from));
  const cambiado = $derived(Math.abs(value - p.base) > 1e-12);
  const estado = $derived(dentro(p, value));
  const idDz = $derived(`qp-dz-${p.key.replace(':', '-')}`);
  const rangoTxt = $derived.by(() => {
    if (!p.ranged) return '';
    const a = p.lo !== null ? num(p.lo) : p.infLo ? '−∞' : '?';
    const b = p.hi !== null ? num(p.hi) : p.infHi ? '∞' : '?';
    return `[${a}, ${b}]`;
  });

  let texto = $state('');
  let editando = $state(false);
  const mostrado = $derived(editando ? texto : num(value, 6));

  function escribir(s: string) {
    const v = parseNum(s);
    if (v === null || s.trim() === '') return;
    onvalue(v);
  }
</script>

<div class="fila" class:elegido class:cambiado>
  <div class="cab">
    <button class="nom" onclick={onelegir} aria-pressed={elegido} title={t('Ver la gráfica de Z según {0}', p.label)}>
      <span class="et">{p.kind === 'cost' ? 'c' : 'b'}</span>
      <span class="nm">{p.name}</span>
    </button>
    <input
      class="val"
      inputmode="decimal"
      aria-label={t('Valor de {0}', p.label)}
      value={mostrado}
      onfocus={(e) => {
        texto = num(value, 6);
        editando = true;
        (e.currentTarget as HTMLInputElement).select();
      }}
      oninput={(e) => (texto = (e.currentTarget as HTMLInputElement).value)}
      onblur={() => {
        escribir(texto);
        editando = false;
      }}
      onkeydown={(e) => {
        if (e.key === 'Enter') {
          escribir(texto);
          (e.currentTarget as HTMLInputElement).blur();
        } else if (e.key === 'Escape') {
          editando = false;
          (e.currentTarget as HTMLInputElement).blur();
        }
      }}
    />
    <button class="btn ghost icon mini" onclick={onreset} disabled={!cambiado} title={t('Volver al valor original ({0})', num(p.base))} aria-label={t('Restablecer {0}', p.label)}>
      <Icon name="recargar" size={14} />
    </button>
    <button class="btn ghost icon mini graf" class:on={elegido} onclick={onelegir} aria-pressed={elegido} title={t('Ver la gráfica de Z según {0}', p.label)} aria-label={t('Gráfica de {0}', p.label)}>
      <Icon name="fc" size={14} />
    </button>
  </div>
  <Deslizador
    id={idDz}
    {value}
    min={from}
    max={to}
    {step}
    base={p.base}
    lo={p.lo}
    hi={p.hi}
    banda={p.ranged}
    label={p.label}
    oninput={(v) => onvalue(snap(v, step))}
  />
  <div class="pie">
    <span>{num(from)}</span>
    {#if p.ranged}
      <span class="rg" class:fuera={estado === false} title={t('Dentro de este rango la solución actual sigue siendo óptima')}>
        {#if estado === false}<Icon name="alerta" size={12} />{/if}
        {p.kind === 'cost' ? t('Misma solución en {0}', rangoTxt) : t('Misma base en {0}', rangoTxt)}
      </span>
    {:else}
      <span class="rg muted">{t('original {0}', num(p.base))}</span>
    {/if}
    <span>{num(to)}</span>
  </div>
</div>

<style>
  .fila {
    display: grid;
    gap: 2px;
    padding: 8px 10px 6px;
    border-radius: var(--r-md);
    border: 1px solid transparent;
  }
  .fila.elegido {
    background: var(--solver-tint);
    border-color: color-mix(in oklab, var(--solver) 30%, transparent);
  }
  .cab {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .nom {
    display: inline-flex;
    align-items: baseline;
    gap: 6px;
    min-width: 0;
    flex: 1;
    border: 0;
    background: none;
    padding: 2px 4px;
    margin-left: -4px;
    border-radius: 5px;
    text-align: left;
  }
  .nom:hover {
    background: var(--rule-soft);
  }
  .et {
    font-size: 11px;
    font-weight: 700;
    color: var(--ink-3);
    font-style: italic;
  }
  .nm {
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .cambiado .nm::after {
    content: '';
    display: inline-block;
    width: 6px;
    height: 6px;
    margin-left: 6px;
    border-radius: 50%;
    background: var(--solver);
    vertical-align: middle;
  }
  .val {
    width: 92px;
    height: 26px;
    padding: 0 8px;
    text-align: right;
    border: 1px solid var(--rule);
    border-radius: var(--r-xs);
    background: var(--field);
    font-variant-numeric: tabular-nums;
    font-weight: 600;
  }
  .cambiado .val {
    border-color: color-mix(in oklab, var(--solver) 55%, transparent);
    color: var(--solver);
  }
  .mini {
    width: 26px;
    height: 26px;
  }
  .graf.on {
    color: var(--solver);
    background: var(--solver-tint);
  }
  .pie {
    display: flex;
    justify-content: space-between;
    gap: 8px;
    font-size: 11px;
    color: var(--ink-3);
    font-variant-numeric: tabular-nums;
  }
  .rg {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    color: var(--good);
    font-weight: 550;
    text-align: center;
  }
  .rg.fuera {
    color: var(--binding);
  }
  .rg.muted {
    color: var(--ink-3);
    font-weight: 400;
  }
</style>
