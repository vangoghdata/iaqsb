<script lang="ts">
  // Botón de la barra de arriba: motor de optimización y sus parámetros (para todos los módulos).
  import Icon from './Icon.svelte';
  import { motor, type Metodo } from '../lib/motor.svelte';
  import { t } from '../lib/i18n.svelte';
  import { call } from '../lib/backend';
  import { shell } from '../lib/shell.svelte';
  import { app } from '../lib/state.svelte';
  import { algebra } from '../lib/algebra.svelte';
  import { ui } from '../lib/ui.svelte';
  import { num } from '../lib/format';

  interface Prueba {
    name: string;
    params: Record<string, unknown>;
    time: number;
    status: string;
    same_result: boolean;
    speedup: number;
  }
  interface Ajuste {
    is_mip: boolean;
    cols: number;
    rows: number;
    elapsed: number;
    baseline: Prueba;
    trials: Prueba[];
    best: Record<string, unknown>;
    best_name: string;
    best_speedup: number;
    summary: string;
  }

  let ajustando = $state(false);
  let ajuste = $state<Ajuste | null>(null);
  let presupuesto = $state(20);

  /** Prueba configuraciones del motor en el modelo abierto y propone la más rápida que da el mismo resultado. */
  async function ajustar() {
    ajustando = true;
    ajuste = null;
    try {
      const model = algebra.on ? algebra.payload() : app.payload();
      const base = motor.params();
      delete base.log;
      ajuste = await call<Ajuste>('tune', { model, options: { engine: base }, budget: presupuesto, repeats: 2 });
    } catch (e) {
      ui.avisar(String(e), 'error');
    } finally {
      ajustando = false;
    }
  }

  function aplicar() {
    if (!ajuste) return;
    motor.aplicar(ajuste.best, ajuste.summary);
    ui.avisar(t('Se aplicó la configuración «{0}».', ajuste.best_name), 'ok');
    ajuste = null;
  }

  let abierto = $state(false);
  let caja: HTMLElement | undefined = $state();

  function alternar() {
    abierto = !abierto;
    if (abierto) motor.init();
  }

  function fuera(e: MouseEvent) {
    if (abierto && caja && !caja.contains(e.target as Node)) abierto = false;
  }

  function numero(v: string, entero = false): number | null {
    const x = entero ? parseInt(v, 10) : parseFloat(v.replace(',', '.'));
    return Number.isFinite(x) && x > 0 ? x : null;
  }

  const METODOS: { id: Metodo; label: string }[] = $derived([
    { id: 'auto', label: t('Automático') },
    { id: 'dual_simplex', label: t('Símplex dual') },
    { id: 'primal_simplex', label: t('Símplex primal') },
    { id: 'barrier', label: t('Punto interior (barrera)') },
    { id: 'pdlp', label: t('PDLP (primer orden, modelos enormes)') }
  ]);

  const actual = $derived(motor.solvers.find((s) => s.id === motor.solver));
</script>

<svelte:window onclick={fuera} onkeydown={(e) => e.key === 'Escape' && (abierto = false)} />

<div class="motor" bind:this={caja}>
  <button
    class="btn ghost"
    class:on={motor.personalizado}
    aria-haspopup="dialog"
    aria-expanded={abierto}
    title={t('Motor de optimización y sus parámetros')}
    onclick={alternar}
  >
    <Icon name="motor" size={17} />
    <span class="txt">{motor.nombre.split(' ')[0]}</span>
    {#if motor.personalizado}<span class="punto" aria-hidden="true"></span>{/if}
  </button>

  {#if abierto}
    <div class="popover panel" role="dialog" aria-label={t('Motor de optimización')}>
      <p class="ptit">{t('Motor de optimización')}</p>
      <p class="ayuda">{t('Vale para todos los módulos. HiGHS viene incluido; Gurobi, CPLEX, SCIP, CBC y GLPK se usan si están instalados en este equipo (con su propia licencia).')}</p>

      <label class="fila">
        <span>{t('Motor')}</span>
        <select bind:value={motor.solver} onchange={() => motor.guardar()} disabled={motor.cargando}>
          {#if !motor.solvers.length}<option value="highs">HiGHS</option>{/if}
          {#each motor.solvers as s (s.id)}
            <option value={s.id}>{s.name}</option>
          {/each}
        </select>
      </label>
      {#if motor.cargando}
        <p class="nota">{t('Buscando motores instalados…')}</p>
      {:else if actual}
        <p class="nota">{actual.problems} · {actual.via}{actual.note ? ` · ${actual.note}` : ''}</p>
      {/if}

      <label class="fila">
        <span>{t('Tiempo máximo (s)')}</span>
        <input type="number" min="0" step="1" placeholder={t('sin límite')} value={motor.tiempo ?? ''}
          oninput={(e) => { motor.tiempo = numero(e.currentTarget.value); motor.guardar(); }} />
      </label>
      <label class="fila">
        <span>{t('Brecha MIP (%)')}</span>
        <input type="number" min="0" step="0.01" placeholder={t('la del motor')} value={motor.brecha ?? ''}
          oninput={(e) => { const v = e.currentTarget.value.trim(); motor.brecha = v === '' ? null : Math.max(0, parseFloat(v.replace(',', '.')) || 0); motor.guardar(); }} />
      </label>
      <label class="fila">
        <span>{t('Hilos')}</span>
        <input type="number" min="1" step="1" placeholder={t('automático')} value={motor.hilos ?? ''}
          oninput={(e) => { motor.hilos = numero(e.currentTarget.value, true); motor.guardar(); }} />
      </label>
      <label class="fila">
        <span>{t('Método (LP)')}</span>
        <select bind:value={motor.metodo} onchange={() => motor.guardar()}>
          {#each METODOS as m (m.id)}<option value={m.id}>{m.label}</option>{/each}
        </select>
      </label>
      <label class="check">
        <input type="checkbox" bind:checked={motor.registro} onchange={() => motor.guardar()} />
        <span>{t('Mostrar el registro del motor')}</span>
      </label>

      <p class="nota">{t('Con los valores predeterminados, los modelos pequeños de LP-ILP se resuelven con el símplex de tablas y el branch and bound de WinQSB (para ver el paso a paso); al cambiar algo, se usa el motor elegido.')}</p>

      {#if shell.module === 'lp'}
        <div class="ajuste">
          <p class="ptit">{t('Ajuste automático')}</p>
          <p class="ayuda">{t('Prueba varias configuraciones del motor en el modelo abierto y propone la más rápida que da el mismo resultado (como el «tuning» de Gurobi o CPLEX).')}</p>
          <label class="fila">
            <span>{t('Tiempo para probar (s)')}</span>
            <input type="number" min="2" max="600" step="1" bind:value={presupuesto} />
          </label>
          <button class="btn" onclick={ajustar} disabled={ajustando}>
            {#if ajustando}<span class="spinner" aria-hidden="true"></span>{t('Probando configuraciones…')}{:else}<Icon name="motor" size={14} />{t('Ajustar para este modelo')}{/if}
          </button>
          {#if ajuste}
            <p class="nota">{ajuste.summary}</p>
            {#if ajuste.trials.length}
              <table class="pruebas">
                <thead><tr><th>{t('Configuración')}</th><th>{t('Tiempo (ms)')}</th><th>{t('Aceleración')}</th></tr></thead>
                <tbody>
                  <tr class="base"><td>{ajuste.baseline.name}</td><td>{num(ajuste.baseline.time * 1000, 2)}</td><td>1×</td></tr>
                  {#each ajuste.trials.filter((x) => x.name !== ajuste?.baseline.name) as p (p.name)}
                    <tr class:mejor={p.name === ajuste.best_name} class:distinto={!p.same_result}>
                      <td>{p.name}{#if !p.same_result} <span title={t('Dio otro resultado: no se tiene en cuenta')}>⚠</span>{/if}</td>
                      <td>{num(p.time * 1000, 2)}</td>
                      <td>{num(p.speedup, 2)}×</td>
                    </tr>
                  {/each}
                </tbody>
              </table>
            {/if}
            {#if ajuste.best_speedup >= 1.1}
              <button class="btn primary" onclick={aplicar}>{t('Aplicar «{0}»', ajuste.best_name)}</button>
            {/if}
          {:else if motor.ajuste}
            <p class="nota">{t('Configuración aplicada')}: {motor.ajuste}</p>
          {/if}
        </div>
      {/if}

      <div class="pie">
        <button class="btn ghost" onclick={() => motor.restablecer()} disabled={!motor.personalizado}>{t('Restablecer')}</button>
      </div>
    </div>
  {/if}
</div>

<style>
  .motor {
    position: relative;
  }
  .btn.on {
    color: var(--barra-fg) !important;
  }
  .punto {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--solver);
  }
  .panel {
    right: 0;
    top: calc(100% + 6px);
    width: 340px;
    padding: 12px 14px 10px;
    display: grid;
    gap: 8px;
  }
  .ptit {
    margin: 0;
    font-size: 10.5px;
    font-weight: 650;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--ink-3);
  }
  .ayuda,
  .nota {
    margin: 0;
    font-size: 11.5px;
    line-height: 1.4;
    color: var(--ink-3);
  }
  .fila {
    display: grid;
    grid-template-columns: 1fr 150px;
    align-items: center;
    gap: 10px;
    font-size: 13px;
  }
  .fila input,
  .fila select {
    width: 100%;
    height: 30px;
    padding: 0 8px;
    border: 1px solid var(--rule);
    border-radius: var(--r-sm);
    background: var(--sheet);
    font-size: 13px;
  }
  .check {
    display: flex;
    gap: 8px;
    align-items: center;
    font-size: 13px;
  }
  .ajuste {
    display: grid;
    gap: 8px;
    padding-top: 8px;
    border-top: 0.5px solid var(--rule);
  }
  .pruebas {
    border-collapse: collapse;
    font-size: 12px;
    font-variant-numeric: tabular-nums;
  }
  .pruebas th,
  .pruebas td {
    padding: 3px 6px;
    text-align: right;
    border-bottom: 0.5px solid var(--rule);
  }
  .pruebas th:first-child,
  .pruebas td:first-child {
    text-align: left;
  }
  .pruebas .base {
    color: var(--ink-3);
  }
  .pruebas .mejor {
    color: var(--solver);
    font-weight: 600;
  }
  .pruebas .distinto {
    color: var(--ink-3);
    text-decoration: line-through;
  }
  .panel {
    max-height: calc(100vh - 80px);
    overflow: auto;
  }
  .pie {
    display: flex;
    justify-content: flex-end;
  }
  @media (max-width: 640px) {
    .txt {
      display: none;
    }
  }
</style>
