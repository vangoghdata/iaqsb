<script lang="ts">
  // Práctica de branch and bound: el estudiante decide en cada nodo si poda
  // (no factible, por cota, solución entera) o ramifica, y sobre qué variable.
  import { untrack } from 'svelte';
  import { app } from '../../lib/state.svelte';
  import { practica, huella } from '../../lib/practica_sesion.svelte';
  import * as P from '../../lib/practica';
  import { t } from '../../lib/i18n.svelte';
  import Icon from '../Icon.svelte';
  import ArbolBnb from './ArbolBnb.svelte';
  import Notas from './Notas.svelte';
  import Resumen from './Resumen.svelte';

  const s = practica.bnb;
  const snap = () => $state.snapshot(app.model) as typeof app.model;
  const actual = $derived(huella(snap()));
  const cambiado = $derived(s.key !== null && s.key !== actual);

  $effect(() => {
    untrack(() => {
      if (s.key === null && !s.loading) s.start(snap(), app.intTolerance);
    });
  });

  const n = $derived(s.nodos.find((x) => x.id === s.actual) ?? null);
  const model = $derived(s.model);
  const ramas = $derived.by(() => {
    if (!n || !model || s.varRama === null) return null;
    const v = n.values[s.varRama];
    const name = model.variables[s.varRama].name;
    return { down: `${name} ≤ ${Math.floor(v)}`, up: `${name} ≥ ${Math.floor(v) + 1}` };
  });
  const cadena = (x: P.BNode): string[] => {
    const out: string[] = [];
    let cur: P.BNode | undefined = x;
    while (cur && cur.branch) {
      out.unshift(cur.branch);
      cur = s.nodos.find((y) => y.id === cur!.parent);
    }
    return out;
  };
  const explorados = $derived(s.nodos.filter((x) => x.order).length);
  const coincide = $derived.by(() => {
    if (s.fase !== 'done' || !s.ref) return null;
    if (s.incumbente) return s.ref.status === 'optimal' && P.sameObjective(s.incumbente.z, s.ref.objective_value);
    return s.ref.status === 'infeasible' || (s.raizNoAcotada && s.ref.status === 'unbounded');
  });
  const tipo = (v: { type?: string }) => (v.type === 'binary' ? t('binaria') : v.type === 'integer' ? t('entera') : t('continua'));

  let caja: HTMLElement | undefined = $state();
  $effect(() => {
    s.fase;
    s.actual;
    queueMicrotask(() => {
      if (!caja) return;
      const sel = s.fase === 'decide' ? '.dec' : s.fase === 'var' ? '.vpk' : s.fase === 'dir' ? '.dir' : null;
      const el = sel ? caja.querySelector<HTMLElement>(sel) : null;
      const a = document.activeElement;
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
    s.fase === 'decide'
      ? t('¿Qué haces con el nodo {0}?', s.actual ?? '')
      : s.fase === 'var'
        ? t('¿Sobre qué variable ramificas?')
        : s.fase === 'dir'
          ? t('¿Qué rama exploras primero?')
          : s.fase === 'solving'
            ? t('Resolviendo los dos subproblemas…')
            : ''
  );
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="bp" bind:this={caja} onkeydown={teclas}>
  {#if s.loading}
    <p class="muted">{t('Resolviendo la relajación lineal…')}</p>
  {:else if s.error}
    <div class="aviso card">
      <Icon name="alerta" size={18} />
      <div>
        <p>{s.error}</p>
        <button class="btn" onclick={() => s.start(snap(), app.intTolerance)}>{t('Intentar de nuevo')}</button>
      </div>
    </div>
  {:else if model && s.nodos.length}
    {#if cambiado}
      <div class="stale">
        <Icon name="recargar" size={17} />{t('El modelo cambió desde que empezaste la práctica.')}
        <button class="link" onclick={() => s.start(snap(), app.intTolerance)}>{t('Empezar con el modelo actual')}</button>
      </div>
    {/if}

    <div class="barra">
      <div class="where">
        <strong>{t('Branch and bound')}</strong>
        <span class="muted">
          {model.objective === 'max' ? t('Maximizar') : t('Minimizar')} ·
          {s.incumbente ? t('mejor solución entera: z = {0}', P.fmtNum(s.incumbente.z)) : t('aún no hay solución entera')}
        </span>
      </div>
      <span class="fill"></span>
      <span class="muted small">{t('Aciertos: {0} de {1}', s.marcador.aciertos, s.marcador.intentos)} · {t('Tolerancia entera: {0}', s.tol)}</span>
      <button class="btn" onclick={() => s.restart()}><Icon name="recargar" size={14} />{t('Reiniciar')}</button>
    </div>

    <ArbolBnb nodos={s.nodos} {model} actual={s.actual} pila={s.pila} mejor={s.incumbente?.node ?? null} />

    {#if n && s.fase !== 'done'}
      <div class="dos">
        <section class="nodo card" aria-label={t('Nodo actual')}>
          <h3>{t('Nodo {0}', n.id)}</h3>
          <p class="muted cad">
            {#if cadena(n).length}{t('Ramas desde la raíz:')} {cadena(n).join(', ')}{:else}{t('Relajación lineal del problema (sin ramas).')}{/if}
          </p>
          {#if n.status === 'optimal'}
            <p class="z">{t('Óptimo de la relajación: z = {0}', P.fmtNum(n.z))}</p>
            <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
            <table
              class="vals"
              onkeydown={(e) => {
                const el = e.target as HTMLElement;
                if (!el.classList.contains('vpk') || (e.key !== 'ArrowDown' && e.key !== 'ArrowUp')) return;
                const all = [...(el.closest('table')?.querySelectorAll<HTMLElement>('.vpk') ?? [])];
                const nx = all[all.indexOf(el) + (e.key === 'ArrowDown' ? 1 : -1)];
                if (nx) {
                  e.preventDefault();
                  nx.focus();
                }
              }}
            >
              <thead><tr><th class="l">{t('Variable')}</th><th class="l">{t('Tipo')}</th><th>{t('Valor')}</th><th>{t('Cotas del nodo')}</th></tr></thead>
              <tbody>
                {#each model.variables as v, j}
                  {@const frac = P.isIntVar(v) && !P.isIntegral(n.values[j], s.tol)}
                  <tr class:frac class:mal={s.malVars.includes(j)} class:elegida={s.varRama === j}>
                    <td class="l b">
                      {#if s.fase === 'var'}
                        <button class="vpk" onclick={() => s.elegirVariable(j)} aria-label={t('Ramificar sobre {0}', v.name)}>{v.name}</button>
                      {:else}{v.name}{/if}
                    </td>
                    <td class="l muted">{tipo(v)}</td>
                    <td>{P.fmtNum(n.values[j])}</td>
                    <td class="muted">[{P.fmtNum(n.bounds[j][0])}, {n.bounds[j][1] === Infinity ? '∞' : P.fmtNum(n.bounds[j][1])}]</td>
                  </tr>
                {/each}
              </tbody>
            </table>
          {:else}
            <p class="z">{n.status === 'unbounded' ? t('La relajación no está acotada.') : t('El subproblema no tiene solución factible.')}</p>
          {/if}
          {#if s.pila.length}
            <p class="muted small">{t('Por explorar después: {0}', [...s.pila].reverse().map((id) => t('nodo {0}', id)).join(', '))}</p>
          {/if}
        </section>

        <section class="pregunta card" aria-labelledby="bp-q">
          <div class="qhead">
            <span class="paso">{t('Paso {0}', s.marcador.decisiones + 1)}</span>
            <h3 id="bp-q">{titulo}</h3>
          </div>
          {#if s.fase === 'decide'}
            <div class="acciones col">
              <button class="btn dec" onclick={() => s.decidir('branch')}>{t('Ramificar')}</button>
              <button class="btn" onclick={() => s.decidir('prune_infeasible')}>{t('Podar: no factible')}</button>
              <button class="btn" onclick={() => s.decidir('prune_bound')}>{t('Podar por cota (no puede mejorar)')}</button>
              <button class="btn" onclick={() => s.decidir('incumbent')}>{t('Solución entera: nueva mejor')}</button>
            </div>
          {:else if s.fase === 'var'}
            <p class="muted ayuda">{t('Pulsa el nombre de la variable en la tabla del nodo.')}</p>
          {:else if s.fase === 'dir' && ramas}
            <p class="muted ayuda">{t('Se crearán los dos subproblemas; elige cuál se explora primero.')}</p>
            <div class="acciones">
              <button class="btn dir" onclick={() => s.elegirRama('down')}>{t('Primero {0}', ramas.down)}</button>
              <button class="btn" onclick={() => s.elegirRama('up')}>{t('Primero {0}', ramas.up)}</button>
            </div>
          {:else if s.fase === 'solving'}
            <p class="muted">{t('Un momento…')}</p>
          {/if}
          {#if s.fase !== 'solving'}
            <div class="acciones">
              <button class="btn ghost" onclick={() => s.pista()} disabled={!s.hayPista} aria-keyshortcuts="P" title={t('Pista (tecla P)')}><Icon name="chispa" size={14} />{t('Pista')}</button>
              <button class="btn ghost" onclick={() => s.mostrarRespuesta()}>{t('Mostrar respuesta')}</button>
            </div>
          {/if}
          <Notas notas={s.notas} pistas={s.pistas} />
          {#if s.notasPrevias.length && !s.notas.length}
            <div class="prev">
              <span class="muted small">{t('Paso anterior')}</span>
              <Notas notas={s.notasPrevias} />
            </div>
          {/if}
        </section>
      </div>
    {/if}

    {#if s.fase === 'done'}
      <section class="fin card" aria-live="polite">
        {#if s.raizNoAcotada}
          <h3>{t('La relajación lineal no está acotada')}</h3>
          <p class="muted">{t('Branch and bound necesita cotas: con la relajación no acotada no se puede podar por cota. Revisa el modelo.')}</p>
        {:else if s.incumbente}
          <h3>{t('Solución entera óptima: z = {0}', P.fmtNum(s.incumbente.z))}</h3>
          <p class="sol">
            {#each model.variables as v, j}<span><strong>{v.name}</strong> = {P.fmtNum(s.incumbente.values[j])}</span>{/each}
          </p>
        {:else}
          <h3>{t('El problema entero no es factible')}</h3>
        {/if}
        <p class="muted">{t('No quedan nodos por explorar: se exploraron {0} nodos.', explorados)}</p>
        {#if coincide === true}
          <p class="ok"><Icon name="check" size={15} stroke={2} />{s.incumbente ? t('Coincide con el resultado del programa (z = {0}).', P.fmtNum(s.ref?.objective_value)) : t('Coincide con el resultado del programa.')}</p>
        {:else if coincide === false}
          <p class="bad"><Icon name="alerta" size={15} />{t('El programa obtiene otro resultado: {0}', s.ref?.message ?? '')}</p>
        {/if}
        {#if s.ref?.branch_and_bound}
          <p class="muted">{t('El programa (rama «≤» primero, primera variable fraccionaria) explora {0} nodos.', s.ref.branch_and_bound.nodes_explored)}</p>
        {/if}
        <Resumen m={s.marcador} pasos={explorados} pasosLabel={t('Nodos explorados')} />
        <div class="acciones">
          <button class="btn primary" onclick={() => s.restart()}><Icon name="recargar" size={14} />{t('Practicar de nuevo')}</button>
        </div>
        <Notas notas={s.notasPrevias} />
      </section>
    {/if}
    <p class="muted small">{t('Teclado: Tab y las flechas para moverse, Intro para elegir, P para una pista. Se explora en profundidad: después de cada nodo sigue el último creado.')}</p>
  {/if}
</div>

<style>
  .bp {
    display: flex;
    flex-direction: column;
    gap: 14px;
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
  .fill {
    flex: 1;
  }
  .small {
    font-size: 12.5px;
    margin: 0;
  }
  .dos {
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
    gap: 14px;
    align-items: start;
    max-width: 1100px;
  }
  @media (max-width: 900px) {
    .dos {
      grid-template-columns: minmax(0, 1fr);
    }
  }
  .nodo,
  .pregunta,
  .fin {
    padding: 14px 18px;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .fin {
    max-width: 920px;
  }
  h3 {
    margin: 0;
    font-size: 16px;
  }
  .nodo p,
  .fin p {
    margin: 0;
  }
  .cad {
    font-size: 12.5px;
  }
  .z {
    font-weight: 650;
  }
  .vals {
    width: 100%;
  }
  .vals th,
  .vals td {
    text-align: right;
    padding: 5px 10px;
    border-bottom: 1px solid var(--rule-soft);
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }
  .vals th {
    font-size: 12px;
    color: var(--ink-2);
    font-weight: 600;
  }
  .vals .l {
    text-align: left;
  }
  .vals .b {
    font-weight: 650;
  }
  tr.frac td:nth-child(3) {
    color: var(--solver);
    font-weight: 650;
  }
  tr.mal td {
    background: var(--bad-tint);
  }
  tr.elegida td {
    background: var(--solver-tint);
  }
  .vpk {
    border: 1px solid var(--rule);
    background: var(--field);
    color: var(--ink);
    font: inherit;
    font-weight: 650;
    padding: 2px 9px;
    border-radius: var(--r-xs);
  }
  .vpk:hover {
    border-color: var(--solver);
    background: var(--solver-tint);
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
  .ayuda {
    margin: 0;
    font-size: 13px;
  }
  .acciones {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }
  .acciones.col {
    flex-direction: column;
    align-items: stretch;
  }
  .acciones.col .btn {
    justify-content: flex-start;
  }
  .prev {
    display: flex;
    flex-direction: column;
    gap: 4px;
    opacity: 0.85;
  }
  .sol {
    display: flex;
    flex-wrap: wrap;
    gap: 6px 18px;
    font-variant-numeric: tabular-nums;
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
</style>
