<script lang="ts">
  // ¿Qué pasa si…? en LP-ILP: mueve costos y lados derechos y mira al instante
  // cómo cambian Z y la solución. Trabaja sobre una copia: el modelo solo cambia
  // con «Aplicar al modelo».
  import { untrack } from 'svelte';
  import FilaParam from './FilaParam.svelte';
  import CurvaZ from './CurvaZ.svelte';
  import Icon from '../Icon.svelte';
  import { app } from '../../lib/state.svelte';
  import { call } from '../../lib/backend';
  import { num } from '../../lib/format';
  import { t } from '../../lib/i18n.svelte';
  import { motor } from '../../lib/motor.svelte';
  import type { LpModel, LpReport } from '../../lib/types';
  import {
    Ultima,
    conCambios,
    curvaExacta,
    curvaMuestreada,
    dentro,
    lpOptions,
    lpq,
    mensaje,
    paramsLp,
    signo,
    soporte,
    type Barrido,
    type ParamLp,
    type Parametrico
  } from '../../lib/quepasa.svelte';

  /** A partir de cuántos coeficientes se pasa a «buscar y un deslizador». */
  const MUCHOS = 24;

  const firma = $derived(JSON.stringify(app.model) + '|' + app.intTolerance + '|' + JSON.stringify(motor.params()));
  const modelo = (): LpModel => $state.snapshot(app.model) as LpModel;
  const opciones = () => lpOptions(app.intTolerance);

  const params = $derived(paramsLp(app.model, lpq.base));
  const porKey = $derived(new Map(params.map((p) => [p.key, p])));
  const elegido = $derived(porKey.get(lpq.elegido) ?? params[0] ?? null);
  const muchos = $derived(params.length > MUCHOS);
  const costos = $derived(params.filter((p) => p.kind === 'cost'));
  const rhs = $derived(params.filter((p) => p.kind === 'rhs'));
  const nCambios = $derived(Object.keys(lpq.cambios).length);

  const valorDe = (p: ParamLp) => lpq.cambios[p.key] ?? p.base;
  const rangoDe = (p: ParamLp) => lpq.rangos[p.key] ?? { from: p.rango.min, to: p.rango.max };

  // ---- base: el modelo tal como está (se recalcula si el modelo cambia)
  const solver = new Ultima();
  const curvas = new Ultima();

  $effect(() => {
    const f = firma;
    untrack(() => {
      if (f !== lpq.firma || !lpq.base) cargarBase(f);
    });
  });

  async function cargarBase(f: string) {
    solver.cancelar();
    curvas.cancelar();
    lpq.firma = f;
    lpq.cambios = {};
    lpq.rangos = {};
    lpq.curva = null;
    lpq.actual = null;
    lpq.error = null;
    lpq.cargando = true;
    try {
      const rep = await call<LpReport>('solve', { module: 'lp', model: modelo(), options: opciones() });
      if (lpq.firma !== f) return;
      lpq.base = rep;
      lpq.actual = rep;
      if (!porKey.has(lpq.elegido)) lpq.elegido = params[0]?.key ?? '';
    } catch (e) {
      if (lpq.firma === f) lpq.error = mensaje(e);
    } finally {
      if (lpq.firma === f) lpq.cargando = false;
    }
  }

  // ---- escenario: se vuelve a resolver con cada cambio (espera corta y solo vale la última respuesta)
  function resolver() {
    if (!nCambios) {
      solver.cancelar();
      lpq.actual = lpq.base;
      lpq.resolviendo = false;
      lpq.error = null;
      return;
    }
    lpq.resolviendo = true;
    const m = conCambios(modelo(), $state.snapshot(lpq.cambios));
    solver.pedir(
      () => call<LpReport>('solve', { module: 'lp', model: m, options: opciones() }),
      (rep) => {
        lpq.actual = rep;
        lpq.error = null;
      },
      (e) => (lpq.error = mensaje(e)),
      120,
      () => (lpq.resolviendo = false)
    );
  }

  function poner(p: ParamLp, v: number) {
    if (!Number.isFinite(v)) return;
    if (Math.abs(v - p.base) < 1e-12) delete lpq.cambios[p.key];
    else lpq.cambios[p.key] = v;
    // un valor escrito fuera del recorrido lo amplía
    const r = rangoDe(p);
    if (v < r.from || v > r.to) {
      const w = Math.max(r.to - r.from, 1e-9) * 0.15;
      lpq.rangos[p.key] = { from: Math.min(r.from, v - (v < r.from ? w : 0)), to: Math.max(r.to, v + (v > r.to ? w : 0)) };
    }
    resolver();
  }

  function restablecer(p: ParamLp) {
    delete lpq.cambios[p.key];
    resolver();
  }

  function restablecerTodo() {
    lpq.cambios = {};
    lpq.rangos = {};
    resolver();
  }

  async function aplicar() {
    const n = nCambios;
    if (!n) return;
    for (const p of params) {
      const v = lpq.cambios[p.key];
      if (v === undefined) continue;
      if (p.kind === 'cost') app.model.variables[p.index].cost = v;
      else app.model.constraints[p.index].rhs = v;
    }
    app.changed();
    await app.solve();
    app.tab = 'quepasa';
    app.flash(n === 1 ? t('Se aplicó 1 cambio al modelo') : t('Se aplicaron {0} cambios al modelo', n));
  }

  // ---- curva del coeficiente elegido (con los demás cambios aplicados)
  const claveCurva = $derived.by(() => {
    if (!elegido || !lpq.base) return '';
    const otros = Object.entries(lpq.cambios).filter(([k]) => k !== elegido.key);
    const r = rangoDe(elegido);
    return JSON.stringify([lpq.firma, elegido.key, otros, r.from, r.to]);
  });

  $effect(() => {
    const k = claveCurva;
    if (!k) return;
    untrack(() => cargarCurva(k));
  });

  function cargarCurva(k: string) {
    const p = elegido;
    const base = lpq.base;
    if (!p || !base) return;
    const { from, to } = rangoDe(p);
    const m = conCambios(modelo(), $state.snapshot(lpq.cambios), p.key);
    const exacta = !!base.sensitivity;
    lpq.curvaCargando = true;
    curvas.pedir(
      async () => {
        if (exacta) {
          const r = await call<Parametrico>('parametric', { model: m, kind: p.kind, index: p.index, from, to, options: opciones() });
          return curvaExacta(r, k);
        }
        const r = await call<Barrido>('sweep', { module: 'lp', model: m, options: opciones(), path: p.path, values: { from, to, steps: 40 } });
        return curvaMuestreada(r, k, from, to);
      },
      (c) => {
        lpq.curva = c;
        lpq.curvaError = null;
      },
      (e) => {
        lpq.curva = null;
        lpq.curvaError = mensaje(e);
      },
      lpq.curva ? 220 : 0,
      () => (lpq.curvaCargando = false)
    );
  }

  // ---- lo que se muestra del escenario
  const base = $derived(lpq.base);
  const actual = $derived(lpq.actual ?? lpq.base);
  const zb = $derived(base?.objective_value ?? null);
  const za = $derived(actual?.objective_value ?? null);
  const dz = $derived(zb !== null && za !== null ? za - zb : null);
  const dzPct = $derived(dz !== null && zb !== null && Math.abs(zb) > 1e-12 ? (dz / Math.abs(zb)) * 100 : null);
  /** +1 si el escenario mejora el objetivo, −1 si lo empeora. */
  const mejora = $derived(dz === null || Math.abs(dz) <= 1e-9 ? 0 : (app.model.objective === 'max') === dz > 0 ? 1 : -1);
  const okA = $derived(actual?.status === 'optimal' || actual?.status === 'feasible');

  const sopB = $derived(soporte(base));
  const sopA = $derived(soporte(actual));
  const entran = $derived([...sopA].filter((x) => !sopB.has(x)));
  const salen = $derived([...sopB].filter((x) => !sopA.has(x)));
  const activas = (rep: LpReport | null) => new Set((rep?.constraints ?? []).filter((c) => Math.abs(c.slack_or_surplus) < 1e-9).map((c) => c.name));
  const actB = $derived(activas(base));
  const actA = $derived(activas(actual));
  const nuevasAct = $derived([...actA].filter((x) => !actB.has(x)));
  const yaNoAct = $derived([...actB].filter((x) => !actA.has(x)));
  const mismaEstructura = $derived(okA && !entran.length && !salen.length && !nuevasAct.length && !yaNoAct.length);
  const mismosValores = $derived(
    okA && !!base && !!actual && base.variables.every((v, i) => Math.abs(v.value - (actual.variables[i]?.value ?? NaN)) < 1e-7)
  );

  const fuera = $derived(params.filter((p) => lpq.cambios[p.key] !== undefined && dentro(p, lpq.cambios[p.key]) === false));

  /** Regla del 100 %: suma de la fracción del margen que usa cada cambio. */
  function regla(kind: 'cost' | 'rhs'): number | null {
    const ps = params.filter((p) => p.kind === kind && lpq.cambios[p.key] !== undefined);
    if (ps.length < 2 || !ps.every((p) => p.ranged)) return null;
    let s = 0;
    for (const p of ps) {
      const d = lpq.cambios[p.key] - p.base;
      const margen = d > 0 ? (p.hi === null ? Infinity : p.hi - p.base) : p.lo === null ? Infinity : p.base - p.lo;
      s += margen === Infinity ? 0 : margen <= 1e-12 ? Infinity : Math.abs(d) / margen;
    }
    return s;
  }
  const reglaC = $derived(regla('cost'));
  const reglaB = $derived(regla('rhs'));

  const statusText = $derived({
    optimal: t('Óptimo'),
    infeasible: t('No factible'),
    unbounded: t('No acotado'),
    iteration_limit: t('Límite de iteraciones'),
    node_limit: t('Límite de nodos'),
    feasible: t('Factible'),
    time_limit: t('Se acabó el tiempo'),
    error: t('Error del motor')
  } as Record<string, string>);

  // tablas: en modelos grandes, solo lo que importa
  let todo = $state(false);
  const filasV = $derived.by(() => {
    if (!base) return [];
    const av = new Map((actual?.variables ?? []).map((v) => [v.name, v]));
    const rows = base.variables.map((v) => {
      const a = av.get(v.name);
      const nv = okA && a ? a.value : null;
      return { name: v.name, antes: v.value, ahora: nv, cambia: nv === null || Math.abs(nv - v.value) > 1e-7 };
    });
    return rows.length > 30 && !todo ? rows.filter((r) => r.cambia || Math.abs(r.antes) > 1e-9) : rows;
  });
  const filasR = $derived.by(() => {
    if (!base) return [];
    const ac = new Map((actual?.constraints ?? []).map((c) => [c.name, c]));
    const rows = base.constraints.map((c) => {
      const a = ac.get(c.name);
      const h = okA && a ? a.slack_or_surplus : null;
      const activaB = Math.abs(c.slack_or_surplus) < 1e-9;
      const activaA = h !== null && Math.abs(h) < 1e-9;
      return { name: c.name, antes: c.slack_or_surplus, ahora: h, sombra: okA && a ? a.shadow_price : null, sombraB: c.shadow_price, activaA, cambia: activaA !== activaB };
    });
    return rows.length > 30 && !todo ? rows.filter((r) => r.cambia || r.activaA) : rows;
  });
  const recortado = $derived(!!base && (base.variables.length > 30 || base.constraints.length > 30));

  // modo «muchos coeficientes»: buscar y elegir uno
  const filtrados = $derived.by(() => {
    const q = lpq.filtro.trim().toLowerCase();
    return q ? params.filter((p) => p.label.toLowerCase().includes(q)) : params;
  });
  const elegir = (p: ParamLp) => (lpq.elegido = p.key);
  const rangoElegido = $derived(elegido ? rangoDe(elegido) : { from: 0, to: 1 });
</script>

<div class="qp">
  <header class="qp-head card">
    <div class="qh-t">
      <h2><Icon name="deslizadores" size={19} />{t('¿Qué pasa si…?')}</h2>
      <p class="muted">
        {t('Mueve un costo o un lado derecho y el modelo se vuelve a resolver al instante. Trabajas sobre una copia: el modelo no cambia hasta que pulses «Aplicar al modelo».')}
      </p>
    </div>
    <div class="qh-a">
      <button class="btn" onclick={restablecerTodo} disabled={!nCambios}><Icon name="recargar" size={15} />{t('Restablecer todo')}</button>
      <button class="btn primary" onclick={aplicar} disabled={!nCambios || app.busy} title={t('Copia los valores del escenario al modelo y lo resuelve')}>
        <Icon name="check" size={15} stroke={2.2} />{nCambios ? t('Aplicar al modelo ({0})', nCambios) : t('Aplicar al modelo')}
      </button>
    </div>
  </header>

  {#if app.isInteger && base && !base.sensitivity}
    <p class="aviso"><Icon name="info" size={16} />{t('Modelo entero: no hay rangos de sensibilidad (las bandas verdes no aplican) y la curva de Z se obtiene muestreando.')}</p>
  {/if}

  {#if lpq.cargando && !base}
    <p class="muted cargando"><span class="spinner" aria-hidden="true"></span>{t('Resolviendo el modelo base…')}</p>
  {:else if !base}
    {#if lpq.error}<p class="err" role="alert"><Icon name="alerta" size={16} />{lpq.error}</p>{/if}
  {:else}
    <div class="qp-grid">
      <section class="ctl card" aria-label={t('Coeficientes')}>
        <!-- en pantallas angostas el resultado queda abajo: aquí un resumen fijo -->
        <div class="zmini" aria-hidden="true">
          <span>Z</span>{#if okA}<b class:atenuado={lpq.resolviendo}>{num(za)}</b>{/if}
          {#if nCambios && okA && dz !== null}<span class="dv" class:sube={mejora > 0} class:baja={mejora < 0}>Δ {signo(dz, (v) => num(v))}</span>{/if}
          {#if !okA}<span class="pill bad">{statusText[actual?.status ?? ''] ?? actual?.status}</span>{/if}
        </div>
        <p class="leyenda">
          {#if base.sensitivity}<span class="li"><span class="sw" aria-hidden="true"></span><span>{t('Dentro de la banda verde la solución actual sigue siendo óptima (un coeficiente a la vez).')}</span></span>{/if}
          <span class="li"><span class="sw base" aria-hidden="true"></span><span>{t('valor original')}</span></span>
        </p>
        {#if muchos}
          <div class="buscar">
            <label class="bq">
              <Icon name="buscar" size={15} />
              <input type="search" placeholder={t('Buscar variable o restricción…')} aria-label={t('Buscar coeficiente')} bind:value={lpq.filtro} />
            </label>
            <select aria-label={t('Coeficiente')} value={elegido?.key ?? ''} onchange={(e) => (lpq.elegido = (e.currentTarget as HTMLSelectElement).value)}>
              {#if filtrados.some((p) => p.kind === 'cost')}
                <optgroup label={t('Costos c(j)')}>
                  {#each filtrados.filter((p) => p.kind === 'cost') as p (p.key)}<option value={p.key}>{p.label}{lpq.cambios[p.key] !== undefined ? ' •' : ''}</option>{/each}
                </optgroup>
              {/if}
              {#if filtrados.some((p) => p.kind === 'rhs')}
                <optgroup label={t('Lados derechos')}>
                  {#each filtrados.filter((p) => p.kind === 'rhs') as p (p.key)}<option value={p.key}>{p.label}{lpq.cambios[p.key] !== undefined ? ' •' : ''}</option>{/each}
                </optgroup>
              {/if}
            </select>
            <span class="muted cuenta">{t('{0} de {1}', filtrados.length, params.length)}</span>
          </div>
          {#if elegido}
            {@const r = rangoDe(elegido)}
            <FilaParam p={elegido} value={valorDe(elegido)} from={r.from} to={r.to} elegido onvalue={(v) => poner(elegido, v)} onreset={() => restablecer(elegido)} onelegir={() => {}} />
          {/if}
          {#if nCambios}
            <div class="chips" aria-label={t('Cambios del escenario')}>
              <span class="muted">{t('Cambios:')}</span>
              {#each params.filter((p) => lpq.cambios[p.key] !== undefined) as p (p.key)}
                <span class="chip" class:on={p.key === elegido?.key}>
                  <button class="cb" onclick={() => elegir(p)} title={t('Elegir {0}', p.label)}>{p.label} = {num(lpq.cambios[p.key])}</button>
                  <button class="cx" onclick={() => restablecer(p)} aria-label={t('Restablecer {0}', p.label)} title={t('Restablecer {0}', p.label)}><Icon name="cerrar" size={11} stroke={2.2} /></button>
                </span>
              {/each}
            </div>
          {/if}
        {:else}
          {#each [{ titulo: t('Costos c(j)'), lista: costos }, { titulo: t('Lados derechos'), lista: rhs }] as g (g.titulo)}
            {#if g.lista.length}
              <h3 class="gt">{g.titulo}</h3>
              <div class="lista">
                {#each g.lista as p (p.key)}
                  {@const r = rangoDe(p)}
                  <FilaParam {p} value={valorDe(p)} from={r.from} to={r.to} elegido={p.key === elegido?.key} onvalue={(v) => poner(p, v)} onreset={() => restablecer(p)} onelegir={() => elegir(p)} />
                {/each}
              </div>
            {/if}
          {/each}
        {/if}
      </section>

      <section class="res card" aria-label={t('Resultado del escenario')}>
        <div class="zrow" aria-live="polite">
          <div>
            <span class="pill" class:good={okA} class:bad={!okA}>
              <Icon name={okA ? 'check' : 'alerta'} size={13} stroke={2.2} />{statusText[actual?.status ?? ''] ?? actual?.status}
            </span>
            {#if lpq.resolviendo}<span class="spinner peq" aria-label={t('Resolviendo…')}></span>{/if}
          </div>
          <div class="zline">
            <span class="zlab">{app.model.objective === 'max' ? t('Máximo') : t('Mínimo')} Z</span>
            {#if okA}<span class="zval" class:atenuado={lpq.resolviendo}>{num(za)}</span>{:else}<span class="znada">{t('sin solución')}</span>{/if}
          </div>
          {#if nCambios}
            <div class="delta">
              {#if okA && dz !== null}
                <span class="dv" class:sube={mejora > 0} class:baja={mejora < 0}>Δ {signo(dz, (v) => num(v))}</span>
                {#if dzPct !== null && Math.abs(dz) > 1e-9}<span class="muted">({signo(dzPct, (v) => num(v, 2))} %)</span>{/if}
              {/if}
              <span class="muted">{t('antes {0}', num(zb))}</span>
            </div>
          {:else}
            <p class="muted sin">{t('Sin cambios: este es el modelo original.')}</p>
          {/if}
        </div>

        {#if lpq.error}<p class="err" role="alert"><Icon name="alerta" size={15} />{lpq.error}</p>{/if}

        {#if nCambios}
          <div class="estr" class:cambio={!mismaEstructura && !mismosValores}>
            {#if !okA}
              <Icon name="alerta" size={16} />
              <span>{actual?.message ?? ''}</span>
            {:else if mismosValores}
              <Icon name="check" size={16} stroke={2.2} />
              <span>{t('La solución óptima no cambia: las variables conservan sus valores.')}</span>
            {:else if mismaEstructura}
              <Icon name="check" size={16} stroke={2.2} />
              <span>{t('Misma estructura (las mismas variables positivas y restricciones activas): solo cambian los valores.')}</span>
            {:else}
              <Icon name="alerta" size={16} />
              <span>
                <b>{t('Cambió la estructura de la solución.')}</b>
                {#if entran.length}{' '}{t('Entran: {0}.', entran.join(', '))}{/if}
                {#if salen.length}{' '}{t('Salen: {0}.', salen.join(', '))}{/if}
                {#if nuevasAct.length}{' '}{t('Ahora activas: {0}.', nuevasAct.join(', '))}{/if}
                {#if yaNoAct.length}{' '}{t('Dejan de estar activas: {0}.', yaNoAct.join(', '))}{/if}
              </span>
            {/if}
          </div>
          {#if fuera.length}
            <p class="fuera">{t('Fuera del rango de sensibilidad: {0}. Ahí la solución base deja de ser óptima.', fuera.map((p) => p.label).join(', '))}</p>
          {/if}
          {#if reglaC !== null}
            <p class="regla">{t('Regla del 100 % (costos): los cambios usan el {0} % del margen permitido.', num(reglaC * 100, 1))} {reglaC <= 1 ? t('La solución sigue siendo óptima.') : t('Ya no se puede asegurar que siga siendo óptima.')}</p>
          {/if}
          {#if reglaB !== null}
            <p class="regla">{t('Regla del 100 % (lados derechos): los cambios usan el {0} % del margen permitido.', num(reglaB * 100, 1))} {reglaB <= 1 ? t('Los precios sombra siguen valiendo.') : t('Ya no se puede asegurar que los precios sombra sigan valiendo.')}</p>
          {/if}
        {/if}

        <div class="tablas">
          <div class="tw">
            <table>
              <caption class="cap">{t('Variables')}</caption>
              <thead><tr><th class="l">{t('Variable')}</th><th>{t('Original')}</th><th>{t('Escenario')}</th><th>Δ</th></tr></thead>
              <tbody>
                {#each filasV as v (v.name)}
                  <tr class:cambia={nCambios > 0 && okA && v.cambia}>
                    <td class="l nm">{v.name}</td>
                    <td>{num(v.antes)}</td>
                    <td class="b">{num(v.ahora)}</td>
                    <td class="d">{v.ahora === null ? '–' : signo(v.ahora - v.antes, (x) => num(x))}</td>
                  </tr>
                {/each}
              </tbody>
            </table>
          </div>
          <div class="tw">
            <table>
              <caption class="cap">{t('Restricciones')}</caption>
              <thead><tr><th class="l">{t('Restricción')}</th><th>{t('Holgura original')}</th><th>{t('Holgura')}</th><th>{t('Precio sombra')}</th></tr></thead>
              <tbody>
                {#each filasR as c (c.name)}
                  <tr class:cambia={nCambios > 0 && okA && c.cambia} class:activa={c.activaA}>
                    <td class="l nm">{c.name}</td>
                    <td>{num(c.antes)}</td>
                    <td class="b">{num(c.ahora)}</td>
                    <td>{num(c.sombra)}</td>
                  </tr>
                {/each}
              </tbody>
            </table>
          </div>
        </div>
        {#if recortado}
          <label class="todo"><input type="checkbox" bind:checked={todo} />{t('Mostrar todas las filas (si no, solo las que cambian o son distintas de cero)')}</label>
        {/if}
        <p class="pie muted">{t('Filas resaltadas: lo que cambió respecto del modelo original. Restricciones activas en naranja.')}</p>
      </section>
    </div>

    {#if elegido}
      <section class="graf card" aria-label={t('Z según el parámetro')}>
        <header class="gh">
          <h3>{t('Z según el parámetro')}</h3>
          <label class="gsel">
            <span class="muted">{t('Coeficiente')}</span>
            <select value={elegido.key} onchange={(e) => (lpq.elegido = (e.currentTarget as HTMLSelectElement).value)}>
              <optgroup label={t('Costos c(j)')}>
                {#each costos as p (p.key)}<option value={p.key}>{p.label}</option>{/each}
              </optgroup>
              {#if rhs.length}
                <optgroup label={t('Lados derechos')}>
                  {#each rhs as p (p.key)}<option value={p.key}>{p.label}</option>{/each}
                </optgroup>
              {/if}
            </select>
          </label>
        </header>
        {#if nCambios && Object.keys(lpq.cambios).some((k) => k !== elegido.key)}
          <p class="muted otros">{t('La curva usa los demás cambios del escenario.')}</p>
        {/if}
        <CurvaZ
          p={elegido}
          curva={lpq.curva}
          cargando={lpq.curvaCargando}
          error={lpq.curvaError}
          valor={valorDe(elegido)}
          from={rangoElegido.from}
          to={rangoElegido.to}
          onrango={(a, b) => (lpq.rangos[elegido.key] = { from: a, to: b })}
          onsugerido={() => delete lpq.rangos[elegido.key]}
        />
      </section>
    {/if}
  {/if}
</div>

<style>
  .qp {
    display: grid;
    gap: 16px;
    min-width: 0;
  }
  .qp-head {
    display: flex;
    justify-content: space-between;
    align-items: center;
    flex-wrap: wrap;
    gap: 12px 20px;
    padding: 14px 18px;
    background:
      radial-gradient(120% 140% at 100% 0%, var(--solver-tint), transparent 55%),
      var(--sheet);
  }
  .qh-t h2 {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 17px;
  }
  .qh-t h2 :global(svg) {
    color: var(--solver);
  }
  .qh-t p {
    margin: 4px 0 0;
    max-width: 78ch;
    font-size: 12.5px;
  }
  .qh-a {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
  }
  .aviso,
  .err {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 0;
  }
  .aviso {
    padding: 10px 14px;
    border-radius: var(--r-md);
    background: var(--solver-tint);
    font-size: 12.5px;
  }
  .aviso :global(svg) {
    color: var(--solver);
    flex: none;
  }
  .err {
    color: var(--bad);
  }
  .cargando {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .qp-grid {
    display: grid;
    grid-template-columns: minmax(300px, 420px) minmax(0, 1fr);
    gap: 16px;
    align-items: start;
  }
  @media (max-width: 1000px) {
    .qp-grid {
      grid-template-columns: minmax(0, 1fr);
    }
  }
  .ctl {
    padding: 12px 10px;
    display: grid;
    gap: 4px;
    min-width: 0;
  }
  .leyenda {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 4px 16px;
    margin: 0 10px 6px;
    font-size: 11.5px;
    color: var(--ink-2);
  }
  .li {
    display: inline-flex;
    align-items: center;
    gap: 7px;
  }
  .sw {
    display: inline-block;
    width: 18px;
    height: 10px;
    border-radius: 3px;
    background: var(--good-tint);
    box-shadow: inset 0 0 0 1px color-mix(in oklab, var(--good) 55%, transparent);
    flex: none;
  }
  .sw.base {
    width: 2px;
    height: 12px;
    background: var(--ink-2);
    box-shadow: none;
  }
  .zmini {
    display: none;
    position: sticky;
    top: 0;
    z-index: 2;
    align-items: baseline;
    gap: 10px;
    margin: -12px -10px 6px;
    padding: 8px 18px;
    border-bottom: 1px solid var(--rule);
    border-radius: var(--r-lg) var(--r-lg) 0 0;
    background: color-mix(in oklab, var(--sheet) 92%, transparent);
    -webkit-backdrop-filter: var(--vidrio);
    backdrop-filter: var(--vidrio);
    font-variant-numeric: tabular-nums;
  }
  .zmini b {
    font-family: var(--font-brand);
    font-size: 22px;
    color: var(--solver);
  }
  .zmini .atenuado {
    opacity: 0.6;
  }
  @media (max-width: 1000px) {
    .zmini {
      display: flex;
    }
  }
  .gt {
    font-size: 12px;
    font-weight: 650;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--ink-3);
    margin: 10px 10px 2px;
  }
  .lista {
    display: grid;
    gap: 2px;
  }
  .buscar {
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(0, 1fr) auto;
    gap: 8px;
    align-items: center;
    padding: 0 10px 6px;
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
  .buscar select,
  .gsel select {
    height: 30px;
    padding: 0 8px;
    border: 1px solid var(--rule);
    background-color: var(--field);
    min-width: 0;
  }
  .cuenta {
    font-size: 11.5px;
  }
  .chips {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px;
    padding: 8px 10px 2px;
    font-size: 12px;
  }
  .chip {
    display: inline-flex;
    align-items: center;
    border-radius: 999px;
    background: var(--rule-soft);
    border: 1px solid var(--rule);
  }
  .chip.on {
    background: var(--solver-tint);
    border-color: color-mix(in oklab, var(--solver) 40%, transparent);
  }
  .chip button {
    border: 0;
    background: none;
    padding: 2px 4px 2px 9px;
    font-size: 12px;
    font-variant-numeric: tabular-nums;
  }
  .chip .cx {
    padding: 2px 7px 2px 3px;
    color: var(--ink-3);
    display: inline-flex;
  }
  .chip .cx:hover {
    color: var(--bad);
  }
  .res {
    position: sticky;
    top: 0;
    padding: 14px 16px;
    display: grid;
    gap: 10px;
    min-width: 0;
  }
  @media (max-width: 1000px) {
    .res {
      position: static;
    }
  }
  .zrow {
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: 6px 18px;
  }
  .zrow > div:first-child {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-basis: 100%;
  }
  .zline {
    display: flex;
    align-items: baseline;
    gap: 10px;
  }
  .zlab {
    color: var(--ink-2);
  }
  .zval {
    font-family: var(--font-brand);
    font-size: 40px;
    font-weight: 700;
    letter-spacing: -0.02em;
    line-height: 1.05;
    color: var(--solver);
    font-variant-numeric: tabular-nums;
    transition: opacity 0.12s;
  }
  .znada {
    font-family: var(--font-brand);
    font-size: 22px;
    font-weight: 650;
    color: var(--bad);
  }
  .zval.atenuado {
    opacity: 0.6;
  }
  .delta {
    display: flex;
    align-items: baseline;
    gap: 8px;
    font-variant-numeric: tabular-nums;
  }
  .dv {
    font-size: 17px;
    font-weight: 700;
  }
  .dv.sube {
    color: var(--good);
  }
  .dv.baja {
    color: var(--bad);
  }
  .sin {
    margin: 0;
    font-size: 12.5px;
  }
  .spinner.peq {
    width: 12px;
    height: 12px;
    color: var(--ink-3);
  }
  .estr {
    display: flex;
    align-items: flex-start;
    gap: 8px;
    padding: 8px 12px;
    border-radius: var(--r-sm);
    background: var(--good-tint);
    font-size: 12.5px;
  }
  .estr :global(svg) {
    color: var(--good);
    flex: none;
    margin-top: 1px;
  }
  .estr.cambio {
    background: var(--binding-tint);
  }
  .estr.cambio :global(svg) {
    color: var(--binding);
  }
  .fuera,
  .regla {
    margin: 0;
    font-size: 12.5px;
    color: var(--ink-2);
  }
  .fuera {
    color: var(--binding);
    font-weight: 550;
  }
  .tablas {
    display: flex;
    flex-wrap: wrap;
    gap: 12px;
    align-items: flex-start;
  }
  .tw {
    max-width: 100%;
    max-height: 420px;
    overflow: auto;
    border: 1px solid var(--rule);
    border-radius: var(--r-md);
  }
  .cap {
    caption-side: top;
    text-align: left;
    padding: 6px 10px;
    font-size: 12px;
    font-weight: 650;
    color: var(--ink-2);
    background: var(--sheet);
  }
  th,
  td {
    text-align: right;
    padding: 5px 10px;
    border-bottom: 1px solid var(--rule-soft);
    white-space: nowrap;
  }
  th {
    font-size: 11.5px;
    font-weight: 600;
    color: var(--ink-2);
    background: var(--sheet-2);
    border-bottom: 1px solid var(--rule);
  }
  .l {
    text-align: left;
  }
  .nm {
    font-weight: 600;
  }
  .b {
    font-weight: 700;
  }
  .d {
    color: var(--ink-2);
  }
  tr.activa td:first-child {
    box-shadow: inset 3px 0 0 var(--binding);
  }
  tr.cambia td {
    background: var(--solver-tint);
  }
  tr.cambia .b,
  tr.cambia .d {
    color: var(--solver);
  }
  .todo {
    display: flex;
    gap: 8px;
    align-items: center;
    font-size: 12.5px;
    color: var(--ink-2);
  }
  .pie {
    margin: 0;
    font-size: 11.5px;
  }
  .graf {
    padding: 14px 18px 18px;
    display: grid;
    gap: 10px;
    min-width: 0;
  }
  .gh {
    display: flex;
    align-items: center;
    justify-content: space-between;
    flex-wrap: wrap;
    gap: 10px;
  }
  .gh h3 {
    font-size: 15px;
  }
  .gsel {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 12.5px;
  }
  .otros {
    margin: 0;
    font-size: 12px;
  }
</style>
