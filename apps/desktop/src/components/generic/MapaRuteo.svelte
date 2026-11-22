<script lang="ts">
  // Ruteo con latitud y longitud: los puntos del modelo en un mapa real. Clic en
  // el mapa añade un cliente, arrastrar un marcador lo mueve (cambia la latitud y
  // la longitud del modelo) y, después de resolver, se dibujan las rutas.
  import { tick } from 'svelte';
  import type { Report } from '../../lib/gtypes';
  import { shown } from '../../lib/gtypes';
  import { defaults } from '../../lib/shell.svelte';
  import { num } from '../../lib/format';
  import { t } from '../../lib/i18n.svelte';
  import { ui } from '../../lib/ui.svelte';
  import { colorRuta, mapaDelInforme, type PuntoGeo, type RecordsW, type RutaGeo } from '../../lib/grafo';
  import FieldInput from './FieldInput.svelte';
  import GeoMap from './GeoMap.svelte';

  let { w, model, onchange, report = null, stale = false }: { w: RecordsW; model: any; onchange: () => void; report?: Report | null; stale?: boolean } = $props();

  const lista = $derived((model[w.path] ?? []) as any[]);
  let sel = $state<number | null>(null);
  let anadir = $state(true);
  let verRutas = $state(true);
  let geo: GeoMap | undefined = $state();
  let panel: HTMLDivElement | undefined = $state();

  const r6 = (v: number) => Math.round(v * 1e6) / 1e6;
  const visible = (k: string, row: any) => (f: { key: string; show_if?: [string, string[]][] }) => f.key === k && shown(f, (x) => row?.[x] ?? model[x]);
  // columnas que se ven con la distancia geográfica (Latitud y Longitud, no X e Y)
  const campoLat = $derived(w.columns.find(visible('y', {})));
  const campoLon = $derived(w.columns.find(visible('x', {})));
  const otros = $derived(w.columns.filter((c) => c.key !== 'name' && c.key !== 'x' && c.key !== 'y' && shown(c, (x) => model[x])));
  const campoDemanda = $derived(otros.find((c) => c.key === 'demand'));

  const mapaRep = $derived(mapaDelInforme(report));
  const rutaDe = $derived.by(() => {
    const m = new Map<string, number>();
    if (!mapaRep || stale) return m;
    for (const p of mapaRep.points) if (p.route !== undefined && p.route !== null) m.set(p.label, p.route);
    return m;
  });

  const puntos = $derived<PuntoGeo[]>(
    lista.map((p, i) => ({
      lat: Number(p.y),
      lon: Number(p.x),
      nombre: String(p.name ?? ''),
      deposito: i === 0,
      nota: i > 0 && campoDemanda && p[campoDemanda.key] ? `${num(Number(p[campoDemanda.key]), 2)}` : undefined,
      ruta: rutaDe.get(String(p.name)),
      sel: sel === i
    }))
  );

  const rutas = $derived.by<RutaGeo[]>(() => {
    if (!mapaRep || !verRutas || !mapaRep.geo) return [];
    const by = new Map(mapaRep.points.map((p) => [p.id, p]));
    return mapaRep.routes.map((r) => ({
      nombre: r.name,
      coords: r.stops.map((s) => by.get(s)).filter((p) => !!p).map((p) => [p!.y, p!.x] as [number, number]),
      vieja: stale
    }));
  });

  function nombreNuevo(): string {
    const usados = new Set(lista.map((p) => String(p.name ?? '').toUpperCase()));
    for (let k = lista.length; k < 100000; k++) {
      const n = t('Cliente {0}', k);
      if (!usados.has(n.toUpperCase())) return n;
    }
    return t('Cliente {0}', Date.now());
  }

  async function elegir(i: number | null, foco = false) {
    sel = i;
    if (i === null || !foco) return;
    await tick();
    const el = panel?.querySelector<HTMLInputElement>('input');
    el?.focus();
    el?.select();
  }

  function agregar(lat: number, lon: number) {
    if (!anadir) return;
    if (!model[w.path]) model[w.path] = [];
    const item: any = defaults(w.columns);
    item.name = nombreNuevo();
    item.x = r6(lon);
    item.y = r6(lat);
    model[w.path].push(item);
    onchange();
    elegir(model[w.path].length - 1, true);
  }

  function mover(i: number, lat: number, lon: number) {
    const p = lista[i];
    if (!p) return;
    p.x = r6(lon);
    p.y = r6(lat);
    onchange();
    sel = i;
  }

  function quitar(i: number) {
    if (lista.length <= 2) {
      ui.avisar(t('Hacen falta el depósito y al menos un cliente'), 'error');
      return;
    }
    const n = lista[i]?.name;
    model[w.path].splice(i, 1);
    sel = null;
    onchange();
    ui.avisar(t('{0} eliminado', n), 'info');
  }

  function hacerDeposito(i: number) {
    if (i <= 0) return;
    const [p] = model[w.path].splice(i, 1);
    // el depósito es siempre el primer punto (su demanda no se usa)
    model[w.path].unshift(p);
    sel = 0;
    onchange();
    ui.avisar(t('{0} es ahora el depósito', p.name), 'info');
  }

  function renombrar(i: number, v: string) {
    const nuevo = v.trim();
    if (!nuevo) return;
    if (lista.some((p, k) => k !== i && String(p.name).toUpperCase() === nuevo.toUpperCase())) {
      ui.avisar(t('Ya hay un punto llamado {0}', nuevo), 'error');
      return;
    }
    lista[i].name = nuevo;
    onchange();
  }

  function tecla(e: KeyboardEvent) {
    if (e.key === 'Escape' && sel !== null) {
      sel = null;
    }
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="ruteo" onkeydown={tecla}>
  <div class="barra">
    <button class="btn" class:on={anadir} aria-pressed={anadir} onclick={() => (anadir = !anadir)} title={t('Con esta opción, un clic en el mapa añade un cliente')}>
      {t('Clic para añadir clientes')}
    </button>
    <button class="btn" onclick={() => geo?.ajustar()}>{t('Ajustar la vista')}</button>
    {#if mapaRep?.routes.length}
      <label class="chk"><input type="checkbox" bind:checked={verRutas} />{t('Mostrar las rutas')}</label>
    {/if}
    <span class="esp"></span>
    <span class="cuenta">{t('{0} clientes · depósito: {1}', Math.max(0, lista.length - 1), lista[0]?.name ?? '–')}</span>
  </div>
  {#if rutas.length && stale}
    <p class="estado" role="status">{t('Las rutas dibujadas son de antes de los últimos cambios: resuelve de nuevo para actualizarlas.')}</p>
  {/if}

  <div class="cuerpo">
    <GeoMap bind:this={geo} {puntos} {rutas} editable onmapa={agregar} onmover={mover} onelegir={(i) => elegir(i)} />

    {#if sel !== null && lista[sel]}
      {@const p = lista[sel]}
      {@const i = sel}
      <div class="panel" bind:this={panel} role="dialog" aria-label={t('Datos del punto')}>
        <div class="ph">
          <strong>{i === 0 ? t('Depósito') : t('Cliente')}</strong>
          <button class="x" aria-label={t('Cerrar')} onclick={() => (sel = null)}>×</button>
        </div>
        <label class="pc"><span>{t('Nombre')}</span><input class="nombre" value={p.name} onchange={(e) => renombrar(i, (e.currentTarget as HTMLInputElement).value)} /></label>
        {#each otros.filter((f) => i > 0 || f.key !== 'demand') as f (f.key)}
          <label class="pc"><span>{f.label}</span><FieldInput field={f} value={p[f.key]} onchange={(v) => { p[f.key] = v; onchange(); }} /></label>
        {/each}
        <div class="dos">
          {#if campoLat}<label class="pc"><span>{campoLat.label}</span><FieldInput field={campoLat} value={p.y} onchange={(v) => { p.y = v; onchange(); geo?.centrar(i); }} /></label>{/if}
          {#if campoLon}<label class="pc"><span>{campoLon.label}</span><FieldInput field={campoLon} value={p.x} onchange={(v) => { p.x = v; onchange(); geo?.centrar(i); }} /></label>{/if}
        </div>
        <div class="acc">
          {#if i > 0}<button class="btn" onclick={() => hacerDeposito(i)}>{t('Usar como depósito')}</button>{/if}
          <button class="btn peligro" onclick={() => quitar(i)}>{t('Eliminar')}</button>
        </div>
      </div>
    {/if}
  </div>

  {#if rutas.length}
    <div class="leyenda">
      {#each mapaRep?.routes ?? [] as r, k}
        <span><i style:background={colorRuta(k)}></i><b>{r.name}</b>{#if r.note}<em>{r.note}</em>{/if}</span>
      {/each}
    </div>
  {/if}
  <p class="ayuda">
    {t('Clic en el mapa: cliente nuevo · Arrastra los marcadores para moverlos · Clic en un marcador: editar o eliminar · El cuadrado azul es el depósito. Las distancias son en línea recta sobre la Tierra (km).')}
  </p>
</div>

<style>
  .nombre {
    width: 100%;
    height: 34px;
    border: 1px solid var(--rule);
    border-radius: var(--r-sm);
    background: var(--field);
    color: var(--ink);
    padding: 0 10px;
  }
  .nombre:focus {
    outline: none;
    border-color: var(--focus);
    box-shadow: 0 0 0 3px var(--solver-tint);
  }
  .ruteo {
    display: flex;
    flex-direction: column;
    gap: 8px;
    max-width: 1100px;
  }
  .barra {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
  }
  .btn.on {
    background: var(--solver-tint);
    color: var(--solver);
    border-color: var(--solver);
  }
  .chk {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: 12.5px;
    color: var(--ink-2);
  }
  .esp {
    flex: 1;
  }
  .cuenta {
    font-size: 12.5px;
    color: var(--ink-3);
  }
  .estado {
    margin: 0;
    font-size: 12.5px;
    color: var(--binding);
  }
  .cuerpo {
    position: relative;
  }
  .panel {
    position: absolute;
    top: 12px;
    right: 12px;
    z-index: 800;
    width: 260px;
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 10px 12px 12px;
    background: var(--sheet);
    border: 1px solid var(--rule);
    border-radius: var(--r-md);
    box-shadow: var(--shadow-pop);
  }
  .ph {
    display: flex;
    justify-content: space-between;
    align-items: center;
    font-size: 13px;
  }
  .x {
    border: 0;
    background: none;
    color: var(--ink-3);
    font-size: 17px;
    width: 24px;
    height: 24px;
    border-radius: 4px;
  }
  .x:hover {
    background: var(--rule-soft);
    color: var(--ink);
  }
  .pc {
    display: flex;
    flex-direction: column;
    gap: 3px;
    font-size: 12px;
    font-weight: 600;
    color: var(--ink-2);
  }
  .pc :global(.inp) {
    width: 100%;
  }
  .dos {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 8px;
  }
  .acc {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .acc .btn {
    font-size: 12px;
    padding: 3px 9px;
    height: auto;
    min-height: 26px;
  }
  .btn.peligro {
    color: var(--bad);
  }
  .btn.peligro:hover {
    background: var(--bad-tint);
  }
  .leyenda {
    display: flex;
    flex-wrap: wrap;
    gap: 6px 16px;
    font-size: 12.5px;
  }
  .leyenda span {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }
  .leyenda i {
    width: 14px;
    height: 4px;
    border-radius: 2px;
  }
  .leyenda em {
    color: var(--ink-3);
    font-style: normal;
  }
  .ayuda {
    margin: 0;
    font-size: 12px;
    color: var(--ink-3);
  }
</style>
