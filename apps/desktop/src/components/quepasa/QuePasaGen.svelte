<script lang="ts">
  // ¿Qué pasa si…? en cualquier módulo: se elige un dato numérico del modelo,
  // un rango y una métrica del informe; el núcleo resuelve el modelo con cada
  // valor (en paralelo) y aquí se grafica la métrica. Además, un deslizador
  // vuelve a resolver en vivo con el valor elegido.
  import { untrack, tick } from 'svelte';
  import BlockView from '../blocks/BlockView.svelte';
  import Deslizador from './Deslizador.svelte';
  import Icon from '../Icon.svelte';
  import { call } from '../../lib/backend';
  import { num, parseNum } from '../../lib/format';
  import { t } from '../../lib/i18n.svelte';
  import { motor } from '../../lib/motor.svelte';
  import type { GenState } from '../../lib/shell.svelte';
  import type { ChartBlock, Report, Series } from '../../lib/gtypes';
  import {
    Ultima,
    barrer,
    datosDelModelo,
    genq,
    mensaje,
    metricaArg,
    metricaDe,
    metricasDe,
    niceStep,
    primerasMetricas,
    rangoPorOmision,
    setAt,
    signo,
    snap,
    valoresBarrido
  } from '../../lib/quepasa.svelte';

  let { st }: { st: GenState } = $props();
  const g = $derived(genq(st.id));
  const panelId = $derived(`qp-gen-${st.id}`);

  // ---- datos del modelo y métricas del informe
  const datos = $derived(datosDelModelo(st.ptype, st.model));
  const dato = $derived(datos.find((d) => d.path === g.path) ?? datos[0] ?? null);
  const coinciden = $derived.by(() => {
    const q = g.filtro.trim().toLowerCase();
    return q ? datos.filter((d) => `${d.grupo} ${d.label}`.toLowerCase().includes(q)) : datos;
  });
  const filtrados = $derived.by(() => {
    const l = coinciden;
    // el elegido siempre está en la lista
    return dato && !l.includes(dato) ? [dato, ...l] : l;
  });
  const grupos = $derived.by(() => {
    const m = new Map<string, typeof datos>();
    for (const d of filtrados) {
      if (!m.has(d.grupo)) m.set(d.grupo, []);
      m.get(d.grupo)!.push(d);
    }
    return [...m.entries()];
  });

  // informe de referencia: el de la app si está al día; si no, uno propio del modelo actual
  const firma = $derived(JSON.stringify(st.model) + '|' + JSON.stringify(st.options) + '|' + JSON.stringify(motor.params()));
  const ref = $derived<Report | null>(st.report && !st.stale ? st.report : g.firmaBase === firma ? g.base : null);
  const metricas = $derived(metricasDe(ref ?? st.report));
  const metrica = $derived(g.metrica && metricas.includes(g.metrica) ? g.metrica : (metricas[0] ?? null));
  const unidad = $derived(metricaDe(ref ?? st.report, metrica)?.unit);
  const mBase = $derived(metricaDe(ref, metrica)?.value ?? null);

  function opciones() {
    const o: Record<string, unknown> = { ...$state.snapshot(st.options) };
    const p = motor.params();
    if (Object.keys(p).length) o.engine = p;
    return o;
  }

  // al cambiar de dato: rango por omisión (±50 %) y deslizador en el valor del modelo
  // (si el valor del modelo queda fuera del rango, por ejemplo con otro ejemplo, también)
  // (g.path puede cambiar antes que este efecto: se compara con el último dato visto)
  let ultimo: string | null = null;
  $effect(() => {
    const d = dato;
    untrack(() => {
      if (!d) return;
      const otro = ultimo === null ? g.path !== d.path : ultimo !== d.path;
      ultimo = d.path;
      if (otro || d.valor < Math.min(g.from, g.to) || d.valor > Math.max(g.from, g.to)) {
        const r = rangoPorOmision(d.valor, d.entero);
        g.from = r.from;
        g.to = r.to;
      }
      if (otro) {
        g.path = d.path;
        g.vivo = null;
        g.vivoRep = null;
        g.barrido = null;
      }
    });
  });

  // informe propio cuando el de la app quedó desactualizado
  const baseUlt = new Ultima();
  $effect(() => {
    const f = firma;
    const necesita = !!st.stale || !st.report;
    untrack(() => {
      if (!necesita || g.firmaBase === f) return;
      const m = $state.snapshot(st.model);
      baseUlt.pedir(
        () => call<Report>('solve', { module: st.id, model: m, options: opciones() }),
        (r) => {
          g.base = r;
          g.firmaBase = f;
        },
        () => {
          g.base = null;
          g.firmaBase = f;
        },
        150
      );
    });
  });

  // ---- barrido
  const barridoUlt = new Ultima();
  const claveBarrido = $derived(dato && metricas.length ? JSON.stringify([firma, dato.path, g.from, g.to, g.pasos, metrica]) : '');
  let hecho = '';
  $effect(() => {
    const k = claveBarrido;
    const auto = g.auto;
    if (!k) return;
    untrack(() => {
      if (auto && k !== hecho) calcular(k, 300);
    });
  });

  function calcular(k = claveBarrido, espera = 0) {
    const d = dato;
    if (!d || !(g.to > g.from) || !st.model) return;
    hecho = k;
    const m = $state.snapshot(st.model);
    const vals = valoresBarrido(g.from, g.to, g.pasos, d.entero);
    const met = metricaArg(ref ?? st.report, metrica);
    g.cargando = true;
    const t0 = performance.now();
    barridoUlt.pedir(
      () => barrer({ module: st.id, model: m, options: opciones(), path: d.path, values: vals, metric: met }),
      (r) => {
        g.barrido = r;
        g.error = null;
        // si el barrido tarda, se deja de recalcular solo
        if (performance.now() - t0 > 4000 && g.auto) {
          g.auto = false;
          st.flash(t('El barrido tardó {0} s: se desactivó el cálculo automático', num((performance.now() - t0) / 1000, 1)));
        }
      },
      (e) => (g.error = mensaje(e)),
      espera,
      () => (g.cargando = false)
    );
  }

  // ---- en vivo
  const vivoUlt = new Ultima();
  const step = $derived(dato?.entero ? Math.max(1, Math.round(niceStep(g.to - g.from))) : niceStep(g.to - g.from));
  const valorVivo = $derived(g.vivo ?? dato?.valor ?? 0);
  const lento = $derived(g.ms > 1200);

  function mover(v: number) {
    const d = dato;
    if (!d || !Number.isFinite(v)) return;
    v = d.entero ? Math.round(v) : snap(v, step);
    if (Math.abs(v - d.valor) < 1e-12) {
      g.vivo = null;
      g.vivoRep = null;
      vivoUlt.cancelar();
      g.vivoCargando = false;
      return;
    }
    g.vivo = v;
    const m = $state.snapshot(st.model);
    setAt(m, d.path, v);
    g.vivoCargando = true;
    vivoUlt.pedir(
      async () => {
        const t0 = performance.now();
        const r = await call<Report>('solve', { module: st.id, model: m, options: opciones() });
        g.ms = performance.now() - t0;
        return r;
      },
      (r) => {
        g.vivoRep = r;
        g.vivoError = null;
      },
      (e) => (g.vivoError = mensaje(e)),
      lento ? 600 : 120,
      () => (g.vivoCargando = false)
    );
  }

  const mVivo = $derived(g.vivoRep ? (metricaDe(g.vivoRep, metrica)?.value ?? null) : mBase);
  const metricasVivo = $derived(g.vivoRep ? primerasMetricas(g.vivoRep) : null);

  async function aplicar() {
    const d = dato;
    const v = g.vivo;
    if (!d || v === null) return;
    setAt(st.model, d.path, v);
    st.changed();
    g.vivo = null;
    g.vivoRep = null;
    await st.solve();
    st.flash(t('Se aplicó {0} = {1} al modelo', d.label, num(v)));
  }

  // al buscar, si el dato elegido ya no coincide, se elige el primero que sí
  function buscar() {
    const q = g.filtro.trim().toLowerCase();
    if (!q) return;
    const ok = datos.filter((d) => `${d.grupo} ${d.label}`.toLowerCase().includes(q));
    if (ok.length && !ok.some((d) => d.path === dato?.path)) g.path = ok[0].path;
  }

  let textoVivo = $state('');
  let editando = $state(false);

  // ---- gráfica y tabla
  const okSt = (s: string) => s === 'optimal' || s === 'feasible' || s === 'solved';
  const chart = $derived.by((): ChartBlock | null => {
    const b = g.barrido;
    if (!b || !dato || b.path !== dato.path) return null;
    const pts = b.points.map((p) => (okSt(p.status) && p.metric !== null ? [p.value, p.metric] : [NaN, NaN]) as [number, number]);
    const series: Series[] = [{ name: metrica ?? t('Métrica'), kind: 'line', tone: 'accent', points: pts }];
    if (g.vivo !== null && mVivo !== null) series.push({ name: t('En vivo ({0})', num(g.vivo)), kind: 'scatter', tone: 'good', points: [[g.vivo, mVivo]] });
    if (mBase !== null) series.push({ name: t('Modelo actual ({0})', num(dato.valor)), kind: 'scatter', tone: 'muted', points: [[dato.valor, mBase]] });
    return {
      type: 'chart',
      title: t('{0} según {1}', metrica ?? t('Métrica'), dato.label),
      x_label: dato.label,
      y_label: unidad ? `${metrica} (${unidad})` : (metrica ?? ''),
      series,
      vlines: dato.valor >= Math.min(g.from, g.to) && dato.valor <= Math.max(g.from, g.to) ? [{ value: dato.valor, label: t('actual'), tone: 'muted' }] : []
    };
  });
  const sinDatos = $derived(!!g.barrido && g.barrido.points.every((p) => p.metric === null));
  const cerca = (a: number, b: number) => Math.abs(a - b) <= 1e-9 * Math.max(1, Math.abs(b));

  const estadoTxt: Record<string, string> = {
    optimal: 'Óptimo',
    feasible: 'Solución factible',
    infeasible: 'No factible',
    unbounded: 'No acotado',
    solved: 'Resuelto',
    error: 'Error'
  };

  // al abrir: el panel a la vista y el foco en su título
  let titulo: HTMLElement | undefined = $state();
  let estaba = false;
  $effect(() => {
    const ab = g.abierto;
    if (ab && !estaba) tick().then(() => {
      titulo?.scrollIntoView({ behavior: 'smooth', block: 'start' });
      titulo?.focus({ preventScroll: true });
    });
    estaba = ab;
  });

  function numIn(e: Event, fn: (v: number) => void) {
    const s = (e.currentTarget as HTMLInputElement).value;
    const v = parseNum(s);
    if (v !== null && s.trim() !== '') fn(v);
  }
</script>

{#if g.abierto}
  <section class="qg card" id={panelId} aria-labelledby={`${panelId}-t`}>
    <header class="qh">
      <div>
        <h3 id={`${panelId}-t`} tabindex="-1" bind:this={titulo}><Icon name="deslizadores" size={18} />{t('¿Qué pasa si…?')}</h3>
        <p class="muted">{t('Elige un dato del modelo y un rango: se resuelve el problema con cada valor y se grafica la métrica elegida. El modelo no cambia hasta que pulses «Aplicar al modelo».')}</p>
      </div>
      <button class="btn ghost icon" onclick={() => (g.abierto = false)} title={t('Cerrar')} aria-label={t('Cerrar ¿Qué pasa si…?')}><Icon name="cerrar" size={16} /></button>
    </header>

    {#if !datos.length}
      <p class="muted">{t('Este modelo no tiene datos numéricos que se puedan variar.')}</p>
    {:else}
      <div class="form">
        <div class="campo dato">
          <span class="et">{t('Dato')}</span>
          <div class="dsel">
            <label class="bq">
              <Icon name="buscar" size={14} />
              <input type="search" placeholder={t('Buscar…')} aria-label={t('Buscar dato')} bind:value={g.filtro} oninput={buscar} />
            </label>
            <select aria-label={t('Dato a variar')} value={dato?.path ?? ''} onchange={(e) => (g.path = (e.currentTarget as HTMLSelectElement).value)}>
              {#each grupos as [gr, ds] (gr)}
                <optgroup label={gr}>
                  {#each ds as d (d.path)}<option value={d.path}>{d.label} ({num(d.valor)})</option>{/each}
                </optgroup>
              {/each}
            </select>
          </div>
          {#if g.filtro.trim() && !coinciden.length}<span class="muted ayuda">{t('Ningún dato coincide con la búsqueda.')}</span>{/if}
        </div>
        <label class="campo">
          <span class="et">{t('Desde')}</span>
          <input class="num" inputmode="decimal" value={num(g.from, 6)} onchange={(e) => numIn(e, (v) => (g.from = v))} />
        </label>
        <label class="campo">
          <span class="et">{t('Hasta')}</span>
          <input class="num" inputmode="decimal" value={num(g.to, 6)} onchange={(e) => numIn(e, (v) => (g.to = v))} />
        </label>
        <label class="campo">
          <span class="et">{t('Pasos')}</span>
          <input class="num corto" type="number" min="2" max="400" value={g.pasos} onchange={(e) => numIn(e, (v) => (g.pasos = Math.max(2, Math.min(400, Math.round(v)))))} />
        </label>
        <label class="campo">
          <span class="et">{t('Métrica')}</span>
          <select value={metrica ?? ''} onchange={(e) => (g.metrica = (e.currentTarget as HTMLSelectElement).value)} disabled={!metricas.length}>
            {#each metricas as m (m)}<option value={m}>{m}</option>{/each}
          </select>
        </label>
        <div class="campo acc">
          <button class="btn primary" onclick={() => calcular()} disabled={g.cargando || !dato || !(g.to > g.from)}>
            {#if g.cargando}<span class="spinner" aria-hidden="true"></span>{t('Calculando…')}{:else}<Icon name="jugar" size={13} stroke={2} />{t('Calcular')}{/if}
          </button>
          <label class="auto"><input type="checkbox" bind:checked={g.auto} />{t('Recalcular al cambiar')}</label>
        </div>
      </div>
      {#if !(g.to > g.from)}<p class="err">{t('«Hasta» debe ser mayor que «Desde».')}</p>{/if}
      {#if !metricas.length}<p class="muted">{t('El informe de este módulo no tiene métricas numéricas para graficar.')}</p>{/if}

      {#if dato}
        <div class="vivo">
          <div class="vc">
            <div class="vt">
              <span class="et">{t('En vivo')}</span>
              <span class="vn">{dato.label}</span>
            </div>
            <div class="vfila">
              <Deslizador value={valorVivo} min={Math.min(g.from, valorVivo)} max={Math.max(g.to, valorVivo)} {step} base={dato.valor} label={t('Valor de {0}', dato.label)} oninput={mover} />
              <input
                class="num"
                inputmode="decimal"
                aria-label={t('Valor de {0}', dato.label)}
                value={editando ? textoVivo : num(valorVivo, 6)}
                onfocus={(e) => {
                  textoVivo = num(valorVivo, 6);
                  editando = true;
                  (e.currentTarget as HTMLInputElement).select();
                }}
                oninput={(e) => (textoVivo = (e.currentTarget as HTMLInputElement).value)}
                onblur={(e) => {
                  numIn(e, mover);
                  editando = false;
                }}
                onkeydown={(e) => e.key === 'Enter' && (e.currentTarget as HTMLInputElement).blur()}
              />
              <button class="btn ghost icon" onclick={() => mover(dato.valor)} disabled={g.vivo === null} title={t('Volver al valor del modelo ({0})', num(dato.valor))} aria-label={t('Restablecer')}><Icon name="recargar" size={15} /></button>
              <button class="btn" onclick={aplicar} disabled={g.vivo === null || st.busy} title={t('Escribe este valor en el modelo y lo resuelve')}><Icon name="check" size={14} stroke={2.2} />{t('Aplicar al modelo')}</button>
            </div>
            {#if lento}<p class="muted ayuda">{t('Cada resolución tarda {0} s: el resultado se actualiza al dejar de mover el control.', num(g.ms / 1000, 1))}</p>{/if}
          </div>
          <div class="vr" aria-live="polite">
            {#if g.vivo === null}
              <p class="muted">{t('Mueve el control para resolver con otro valor.')}</p>
            {:else if g.vivoError}
              <p class="err"><Icon name="alerta" size={15} />{g.vivoError}</p>
            {:else if g.vivoRep}
              <div class="vm" class:atenuado={g.vivoCargando}>
                <span class="pill" class:good={okSt(g.vivoRep.status)} class:bad={!okSt(g.vivoRep.status)}>{t(estadoTxt[g.vivoRep.status] ?? g.vivoRep.status)}</span>
                {#if metrica}
                  <span class="ml">{metrica}</span>
                  <span class="mv">{num(mVivo)}{#if unidad}<small> {unidad}</small>{/if}</span>
                  {#if mVivo !== null && mBase !== null}
                    <span class="dv">Δ {signo(mVivo - mBase, (v) => num(v))}</span>
                    <span class="muted">{t('antes {0}', num(mBase))}</span>
                  {/if}
                {/if}
              </div>
              <p class="hl">{g.vivoRep.headline}</p>
              {#if metricasVivo}<div class="mets"><BlockView block={metricasVivo} /></div>{/if}
            {:else}
              <p class="muted cargando"><span class="spinner" aria-hidden="true"></span>{t('Resolviendo…')}</p>
            {/if}
          </div>
        </div>
      {/if}

      {#if g.error}
        <p class="err" role="alert"><Icon name="alerta" size={15} />{g.error}</p>
      {:else if sinDatos}
        <p class="muted">{t('Ninguno de los puntos dio un valor numérico para «{0}».', metrica ?? '')}</p>
      {:else if chart && g.barrido}
        <div class="graf" class:atenuado={g.cargando}><BlockView block={chart} /></div>
        <details class="puntos" open={g.barrido.points.length <= 25}>
          <summary>{t('Tabla de los {0} puntos', g.barrido.points.length)}</summary>
          <div class="tw">
            <table>
              <thead>
                <tr>
                  <th>{dato?.label}</th>
                  <th class="l">{t('Estado')}</th>
                  <th>{metrica}</th>
                  <th class="l">{t('Resultado')}</th>
                </tr>
              </thead>
              <tbody>
                {#each g.barrido.points as p, k (k)}
                  {@const prev = g.barrido.points[k - 1]}
                  <tr class:actual={dato && cerca(p.value, dato.valor)} class:mal={!okSt(p.status)} class:salto={prev && prev.status !== p.status}>
                    <td>{num(p.value)}</td>
                    <td class="l">{t(estadoTxt[p.status] ?? p.status)}</td>
                    <td class="b">{num(p.metric)}</td>
                    <td class="l hlc">{p.error ?? p.headline ?? ''}</td>
                  </tr>
                {/each}
              </tbody>
            </table>
          </div>
        </details>
      {:else if g.cargando}
        <p class="muted cargando"><span class="spinner" aria-hidden="true"></span>{t('Calculando…')}</p>
      {/if}
    {/if}
  </section>
{/if}

<style>
  .qg {
    display: grid;
    gap: 14px;
    padding: 14px 18px 18px;
    margin-bottom: 18px;
    min-width: 0;
    border-color: color-mix(in oklab, var(--solver) 30%, var(--rule));
  }
  .qh {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: 12px;
  }
  .qh h3 {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 15px;
    outline: none;
  }
  .qh h3 :global(svg) {
    color: var(--solver);
  }
  .qh p {
    margin: 4px 0 0;
    font-size: 12.5px;
    max-width: 80ch;
  }
  .form {
    display: flex;
    flex-wrap: wrap;
    align-items: flex-end;
    gap: 10px 14px;
  }
  .campo {
    display: grid;
    gap: 4px;
  }
  .campo.dato {
    flex: 1 1 360px;
    min-width: 0;
  }
  .et {
    font-size: 12px;
    font-weight: 600;
    color: var(--ink-2);
  }
  .dsel {
    display: grid;
    grid-template-columns: minmax(110px, 0.6fr) minmax(0, 1.4fr);
    gap: 6px;
  }
  .bq {
    display: flex;
    align-items: center;
    gap: 6px;
    height: 30px;
    padding: 0 8px;
    border: 1px solid var(--rule);
    border-radius: var(--r-sm);
    background: var(--field);
    color: var(--ink-3);
  }
  .bq input {
    border: 0;
    background: none;
    outline: none;
    min-width: 0;
    flex: 1;
    color: var(--ink);
  }
  select {
    height: 30px;
    padding: 0 8px;
    border: 1px solid var(--rule);
    background-color: var(--field);
    min-width: 0;
    max-width: 100%;
  }
  .campo select {
    max-width: 280px;
  }
  .dsel select {
    max-width: none;
  }
  .num {
    width: 104px;
    height: 30px;
    padding: 0 8px;
    text-align: right;
    border: 1px solid var(--rule);
    border-radius: var(--r-xs);
    background: var(--field);
    font-variant-numeric: tabular-nums;
  }
  .num.corto {
    width: 72px;
  }
  .acc {
    display: flex;
    align-items: center;
    gap: 12px;
  }
  .auto {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 12.5px;
    color: var(--ink-2);
  }
  .ayuda {
    font-size: 11.5px;
    margin: 0;
  }
  .err {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 0;
    color: var(--bad);
  }
  .vivo {
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
    gap: 14px 20px;
    padding: 12px 14px;
    border-radius: var(--r-md);
    background: var(--rule-soft);
    border: 1px solid var(--rule-soft);
  }
  @media (max-width: 1000px) {
    .vivo {
      grid-template-columns: minmax(0, 1fr);
    }
  }
  .vc {
    display: grid;
    gap: 6px;
    align-content: start;
    min-width: 0;
  }
  .vt {
    display: flex;
    align-items: baseline;
    gap: 8px;
    min-width: 0;
  }
  .vn {
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .vfila {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
  }
  .vfila > :global(.dz) {
    flex: 1 1 180px;
  }
  .vr {
    min-width: 0;
  }
  .vr p {
    margin: 0;
  }
  .vm {
    display: flex;
    align-items: baseline;
    flex-wrap: wrap;
    gap: 6px 10px;
    transition: opacity 0.12s;
  }
  .vm .pill {
    align-self: center;
  }
  .atenuado {
    opacity: 0.6;
  }
  .ml {
    color: var(--ink-2);
    font-size: 12.5px;
  }
  .mv {
    font-family: var(--font-brand);
    font-size: 26px;
    font-weight: 700;
    color: var(--solver);
    font-variant-numeric: tabular-nums;
    letter-spacing: -0.02em;
  }
  .mv small {
    font-size: 12px;
    color: var(--ink-2);
    font-weight: 500;
  }
  .dv {
    font-weight: 700;
    font-variant-numeric: tabular-nums;
  }
  .hl {
    margin-top: 6px !important;
    font-size: 12.5px;
    color: var(--ink-2);
  }
  .mets {
    margin-top: 10px;
  }
  .mets :global(.metric.main .mv) {
    font-size: 22px;
  }
  .cargando {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .graf {
    transition: opacity 0.15s;
  }
  .puntos summary {
    cursor: pointer;
    font-size: 12.5px;
    color: var(--ink-2);
    margin-bottom: 8px;
  }
  .tw {
    width: max-content;
    max-width: 100%;
    max-height: 360px;
    overflow: auto;
    background: var(--sheet);
    border: 1px solid var(--rule);
    border-radius: var(--r-md);
  }
  th,
  td {
    text-align: right;
    padding: 6px 12px;
    border-bottom: 1px solid var(--rule-soft);
    white-space: nowrap;
  }
  th {
    position: sticky;
    top: 0;
    font-size: 12px;
    font-weight: 600;
    color: var(--ink-2);
    background: var(--sheet-2);
    border-bottom: 1px solid var(--rule);
  }
  .l {
    text-align: left;
  }
  .b {
    font-weight: 700;
  }
  .hlc {
    white-space: normal;
    min-width: 260px;
    max-width: 560px;
    color: var(--ink-2);
    font-size: 12.5px;
  }
  tr.actual td {
    background: var(--solver-tint);
  }
  tr.actual td:first-child {
    box-shadow: inset 3px 0 0 var(--solver);
  }
  tr.mal td {
    color: var(--bad);
  }
  tr.salto td {
    border-top: 2px solid var(--binding);
  }
</style>
