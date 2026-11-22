<script lang="ts">
  // Práctica del símplex: el estudiante elige la variable que entra, la que sale,
  // (opcionalmente) calcula la nueva fila pivote, y declara la conclusión.
  import { untrack } from 'svelte';
  import { app } from '../../lib/state.svelte';
  import { practica, huella } from '../../lib/practica_sesion.svelte';
  import * as P from '../../lib/practica';
  import { num } from '../../lib/format';
  import { t } from '../../lib/i18n.svelte';
  import Icon from '../Icon.svelte';
  import TablaPractica from './TablaPractica.svelte';
  import Notas from './Notas.svelte';
  import Resumen from './Resumen.svelte';

  const s = practica.simplex;
  const actual = $derived(huella($state.snapshot(app.model) as typeof app.model));
  const cambiado = $derived(s.key !== null && s.key !== actual);

  // primera vez: empieza con el modelo de la hoja
  $effect(() => {
    untrack(() => {
      if (s.key === null && !s.loading) s.start($state.snapshot(app.model) as typeof app.model);
    });
  });

  const tb = $derived(s.tb);
  const obj = $derived(tb ? P.objective(tb) : null);
  const nombre = (j: number | null) => (tb && j !== null ? tb.columns[j] : '');
  const basica = (r: number | null) => (tb && r !== null ? tb.columns[tb.basis[r]] : '');
  const refIter = $derived((s.ref?.simplex_trace?.length ?? 1) - 1);
  const valores = $derived(s.fin && s.model && s.fin.kind === 'optimal' ? P.variableValues(s.model, s.fin.tb) : []);
  const zFin = $derived(s.fin ? P.objective(s.fin.tb) : null);
  const coincide = $derived.by(() => {
    if (!s.fin || !s.ref) return null;
    if (s.fin.kind === 'optimal') return s.ref.status === 'optimal' && P.sameObjective(zFin!.c.toNumber(), s.ref.objective_value) && zFin!.m.isZero();
    return s.ref.status === s.fin.kind;
  });
  const grande = $derived(!!tb && tb.columns.length > 16);

  let caja: HTMLElement | undefined = $state();
  // tras cada cambio de fase, el foco va al primer control de la pregunta
  $effect(() => {
    s.fase;
    s.tb;
    queueMicrotask(() => {
      if (!caja) return;
      const sel = s.fase === 'enter' ? '[data-nav="col"]' : s.fase === 'leave' ? '[data-nav="row"]' : s.fase === 'row' ? 'input.cell' : s.fase === 'confirm' ? '.pivotar' : null;
      const el = sel ? caja.querySelector<HTMLElement>(sel) : null;
      const a = document.activeElement;
      // solo si el foco ya estaba en la práctica (no se lo quita a otra parte de la ventana)
      if (el && (!a || a === document.body || caja.contains(a))) el.focus({ preventScroll: true });
    });
  });

  function teclas(e: KeyboardEvent) {
    const tag = (e.target as HTMLElement).tagName;
    if (tag === 'INPUT' || tag === 'SELECT' || tag === 'TEXTAREA' || e.ctrlKey || e.metaKey || e.altKey) return;
    if (e.key === 'p' || e.key === 'P') {
      e.preventDefault();
      s.pista();
    }
  }

  const titulo = $derived(
    !tb
      ? ''
      : s.fase === 'enter'
        ? t('¿Qué variable entra a la base?')
        : s.fase === 'leave'
          ? t('Entra {0}. ¿Qué variable sale de la base?', nombre(s.entra))
          : s.fase === 'row'
            ? t('Calcula la nueva fila pivote: {0} reemplaza a {1}', nombre(s.entra), basica(s.sale))
            : s.fase === 'confirm'
              ? t('Pivote: columna {0}, fila de {1}', nombre(s.entra), basica(s.sale))
              : ''
  );
  const ayuda = $derived(
    s.fase === 'enter'
      ? t('Pulsa el nombre de una columna (o usa Tab y las flechas, e Intro). Si ninguna columna mejora el objetivo, declara la conclusión.')
      : s.fase === 'leave'
        ? t('Pulsa la variable básica de la fila que sale, según la prueba del cociente mínimo.')
        : s.fase === 'row'
          ? t('Escribe cada valor de la fila pivote dividida entre el pivote ({0}). Se aceptan fracciones (1/2) y decimales (0,5).', s.tb && s.entra !== null && s.sale !== null ? P.fmtFrac(s.tb.rows[s.sale][s.entra], true) : '')
          : s.fase === 'confirm'
            ? t('Con el pivoteo, {0} entra a la base en lugar de {1}: la fila pivote se divide entre el pivote y las demás filas se ajustan para dejar ceros en la columna.', nombre(s.entra), basica(s.sale))
            : ''
  );
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="sp" bind:this={caja} onkeydown={teclas}>
  {#if s.loading}
    <p class="muted"><span class="spinner" aria-hidden="true"></span> {t('Preparando la tabla inicial…')}</p>
  {:else if s.error}
    <div class="aviso card">
      <Icon name="alerta" size={18} />
      <div>
        {#if s.error === 'sin tablas'}
          <p>{t('Este modelo no se puede practicar con el símplex de tablas: es demasiado grande o necesita el motor de optimización.')}</p>
        {:else}
          <p>{s.error}</p>
        {/if}
        <button class="btn" onclick={() => s.start($state.snapshot(app.model) as typeof app.model)}>{t('Intentar de nuevo')}</button>
      </div>
    </div>
  {:else if tb}
    {#if cambiado}
      <div class="stale">
        <Icon name="recargar" size={17} />{t('El modelo cambió desde que empezaste la práctica.')}
        <button class="link" onclick={() => s.start($state.snapshot(app.model) as typeof app.model)}>{t('Empezar con el modelo actual')}</button>
      </div>
    {/if}

    <div class="barra">
      <div class="where">
        <strong>{tb.iteration === 0 ? t('Tabla inicial') : t('Iteración {0}', tb.iteration)}</strong>
        <span class="muted">{s.model?.objective === 'max' ? t('Maximizar') : t('Minimizar')} · z = {obj ? P.fmtMV(obj, s.fracciones) : ''}</span>
      </div>
      <div class="seg" role="radiogroup" aria-label={t('Formato de los números')}>
        <button role="radio" aria-checked={s.fracciones} class:on={s.fracciones} onclick={() => (s.fracciones = true)}>{t('Fracciones')}</button>
        <button role="radio" aria-checked={!s.fracciones} class:on={!s.fracciones} onclick={() => (s.fracciones = false)}>{t('Decimales')}</button>
      </div>
      <label class="chk"><input type="checkbox" bind:checked={s.practicarFila} /> {t('Calcular la nueva fila pivote')}</label>
      <span class="sp-fill"></span>
      <span class="muted small">{t('Aciertos: {0} de {1}', s.marcador.aciertos, s.marcador.intentos)}</span>
      <button class="btn" onclick={() => s.restart()} title={t('Volver a la tabla inicial')}><Icon name="recargar" size={14} />{t('Reiniciar')}</button>
    </div>

    {#if s.fase !== 'done'}
      <section class="pregunta card" aria-labelledby="sp-q">
        <div class="qhead">
          <span class="paso">{t('Paso {0}', s.marcador.decisiones + 1)}</span>
          <h3 id="sp-q">{titulo}</h3>
        </div>
        <p class="muted ayuda">{ayuda}</p>
        <div class="acciones">
          {#if s.fase === 'enter'}
            <button class="btn" onclick={() => s.declarar('optimal')}><Icon name="check" size={14} />{t('La tabla es óptima')}</button>
            <button class="btn" onclick={() => s.declarar('infeasible')}>{t('El problema no es factible')}</button>
          {:else if s.fase === 'leave'}
            <button class="btn" onclick={() => s.declarar('unbounded')}>{t('Ninguna fila limita: no acotado')}</button>
            <button class="btn ghost" onclick={() => s.cambiarColumna()}>{t('Cambiar la variable que entra')}</button>
          {:else if s.fase === 'row'}
            <button class="btn primary" onclick={() => s.comprobarFila()}>{t('Comprobar')}</button>
            <button class="btn ghost" onclick={() => s.omitirFila()}>{t('Omitir este ejercicio')}</button>
          {:else if s.fase === 'confirm'}
            <button class="btn primary pivotar" onclick={() => s.pivotar()}><Icon name="jugar" size={13} stroke={2} />{t('Pivotar y ver la nueva tabla')}</button>
          {/if}
          <span class="sp-fill"></span>
          {#if s.fase !== 'confirm'}
            <button class="btn ghost" onclick={() => s.pista()} disabled={!s.hayPista} aria-keyshortcuts="P" title={t('Pista (tecla P)')}><Icon name="chispa" size={14} />{t('Pista')}</button>
            <button class="btn ghost" onclick={() => s.mostrarRespuesta()}>{t('Mostrar respuesta')}</button>
          {/if}
        </div>
        <Notas notas={s.notas} pistas={s.pistas} />
      </section>
    {/if}

    {#if s.fin}
      <section class="fin card" aria-live="polite">
        <h3>
          {#if s.fin.kind === 'optimal'}
            {t('Solución óptima: z = {0}', P.fmtMV(zFin!, s.fracciones))}{#if s.fracciones && !zFin!.c.isInt()}<span class="muted dec"> ({P.fmtFrac(zFin!.c, false)})</span>{/if}
          {:else if s.fin.kind === 'unbounded'}
            {t('El problema no está acotado')}
          {:else}
            {t('El problema no es factible')}
          {/if}
        </h3>
        {#if valores.length && s.model}
          <p class="sol">
            {#each s.model.variables as v, j}<span><strong>{v.name}</strong> = {P.fmtFrac(valores[j], s.fracciones)}</span>{/each}
          </p>
        {/if}
        {#if coincide === true}
          <p class="ok"><Icon name="check" size={15} stroke={2} />
            {s.fin.kind === 'optimal' ? t('Coincide con el resultado del programa (z = {0}).', num(s.ref?.objective_value)) : t('Coincide con el resultado del programa.')}
          </p>
        {:else if coincide === false}
          <p class="bad"><Icon name="alerta" size={15} />{t('El programa obtiene otro resultado: {0}', s.ref?.message ?? '')}</p>
        {/if}
        <p class="muted">
          {s.historial.length === refIter
            ? s.historial.length === 1
              ? t('Hiciste 1 iteración, como la regla de Dantzig.')
              : t('Hiciste {0} iteraciones, las mismas que la regla de Dantzig.', s.historial.length)
            : refIter === 1
              ? t('Hiciste {0} iteraciones; con la regla de Dantzig el programa hace 1.', s.historial.length)
              : t('Hiciste {0} iteraciones; con la regla de Dantzig el programa hace {1}.', s.historial.length, refIter)}
        </p>
        <Resumen m={s.marcador} pasos={s.historial.length} />
        <div class="acciones">
          <button class="btn primary" onclick={() => s.restart()}><Icon name="recargar" size={14} />{t('Practicar de nuevo')}</button>
        </div>
        <Notas notas={s.notas} />
      </section>
    {/if}

    {#if grande}<p class="muted small">{t('La tabla es grande: la práctica es más cómoda con modelos pequeños.')}</p>{/if}

    <TablaPractica
      {tb}
      frac={s.fracciones}
      pick={s.fase === 'enter' ? 'cols' : s.fase === 'leave' ? 'rows' : 'none'}
      entra={s.entra}
      sale={s.sale}
      malCols={s.malCols}
      malFilas={s.malFilas}
      descartadas={s.descartadas}
      cocientes={s.verCocientes}
      fila={s.fase === 'row' ? s.fila : null}
      filaOk={s.filaOk}
      onpickcol={(j) => s.elegirColumna(j)}
      onpickrow={(r) => s.elegirFila(r)}
      onrowinput={(k, v) => (s.fila[k] = v)}
      onsubmitrow={() => s.comprobarFila()}
    />
    <p class="muted small">{t('Teclado: Tab y las flechas para moverse, Intro para elegir, P para una pista.')}</p>

    {#if s.historial.length}
      <section class="hist">
        <h4>{t('Pasos anteriores')}</h4>
        {#each s.historial as h, k (k)}
          <details>
            <summary>
              <strong>{h.tb.iteration === 0 ? t('Tabla inicial') : t('Iteración {0}', h.tb.iteration)}</strong>:
              {t('entra {0}, sale {1}', h.tb.columns[h.e], h.tb.columns[h.tb.basis[h.r]])}
              {#if !h.dantzig}<span class="pill plain">{t('no es la de Dantzig')}</span>{/if}
              <span class="muted">· z = {P.fmtMV(P.objective(h.tb), s.fracciones)}</span>
            </summary>
            <TablaPractica tb={h.tb} frac={s.fracciones} entra={h.e} sale={h.r} cocientes compacta />
          </details>
        {/each}
      </section>
    {/if}
  {/if}
</div>

<style>
  .sp {
    display: flex;
    flex-direction: column;
    gap: 14px;
    outline: none;
  }
  .barra {
    display: flex;
    align-items: center;
    gap: 10px 14px;
    flex-wrap: wrap;
  }
  .where strong {
    font-family: var(--font-brand);
    font-size: 17px;
    margin-right: 8px;
  }
  .sp-fill {
    flex: 1;
  }
  .chk {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 12.5px;
  }
  .small {
    font-size: 12.5px;
    margin: 0;
  }
  .pregunta,
  .fin {
    padding: 14px 18px;
    display: flex;
    flex-direction: column;
    gap: 10px;
    max-width: 920px;
  }
  .qhead {
    display: flex;
    align-items: baseline;
    gap: 10px;
  }
  .paso {
    font-size: 11.5px;
    font-weight: 650;
    color: var(--solver);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
  h3 {
    margin: 0;
    font-size: 16px;
  }
  .ayuda {
    margin: 0;
    font-size: 13px;
  }
  .acciones {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    align-items: center;
  }
  .fin .sol {
    display: flex;
    flex-wrap: wrap;
    gap: 6px 18px;
    margin: 0;
    font-variant-numeric: tabular-nums;
  }
  .fin p {
    margin: 0;
  }
  .fin .dec {
    font-weight: 500;
  }
  .fin .ok,
  .fin .bad {
    display: flex;
    gap: 8px;
    align-items: center;
  }
  .fin .ok :global(svg) {
    color: var(--good);
  }
  .fin .bad :global(svg) {
    color: var(--bad);
  }
  .aviso {
    display: flex;
    gap: 12px;
    padding: 14px 18px;
    max-width: 720px;
  }
  .aviso p {
    margin: 0 0 10px;
  }
  .aviso :global(svg) {
    color: var(--binding);
    flex: none;
  }
  .hist {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .hist h4 {
    margin: 6px 0 2px;
    font-size: 13px;
    color: var(--ink-2);
  }
  details summary {
    cursor: pointer;
    padding: 4px 0;
    font-size: 13px;
  }
  details[open] summary {
    margin-bottom: 6px;
  }
  .pill {
    margin-left: 6px;
  }
  .spinner {
    display: inline-block;
    width: 12px;
    height: 12px;
    border-radius: 50%;
    border: 2px solid currentColor;
    border-right-color: transparent;
    animation: girar 0.7s linear infinite;
    vertical-align: -1px;
  }
</style>
