<script lang="ts">
  // Mapa real con Leaflet y los mosaicos de OpenStreetMap: puntos en latitud y
  // longitud (el depósito con otro marcador), rutas como polilíneas de un color
  // por vehículo y, si se puede editar, clic para añadir y arrastre para mover.
  // Sin conexión no hay mosaicos: se avisa y los marcadores siguen funcionando
  // sobre un fondo liso.
  import { onMount } from 'svelte';
  import 'leaflet/dist/leaflet.css';
  import type * as LT from 'leaflet';
  import { t } from '../../lib/i18n.svelte';
  import { openExternal } from '../../lib/backend';
  import { COLORES_RUTA, type PuntoGeo, type RutaGeo } from '../../lib/grafo';

  let {
    puntos,
    rutas = [],
    editable = false,
    alto = 460,
    onmapa,
    onmover,
    onelegir
  }: {
    puntos: PuntoGeo[];
    rutas?: RutaGeo[];
    editable?: boolean;
    alto?: number;
    onmapa?: (lat: number, lon: number) => void;
    onmover?: (i: number, lat: number, lon: number) => void;
    onelegir?: (i: number) => void;
  } = $props();

  const OSM = 'https://tile.openstreetmap.org/{z}/{x}/{y}.png';
  const ATRIBUCION = '© <a href="https://www.openstreetmap.org/copyright">OpenStreetMap</a> contributors';

  let el: HTMLDivElement | undefined = $state();
  let L: typeof LT | null = $state(null);
  let mapa: LT.Map | null = null;
  let capaPuntos: LT.LayerGroup | null = null;
  let capaRutas: LT.LayerGroup | null = null;
  let marcadores: { m: LT.Marker; clave: string }[] = [];
  let sinMosaicos = $state(false);
  let error = $state<string | null>(null);

  const esc = (s: string) => s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;');

  /** Color real de una variable CSS (Leaflet pinta con atributos SVG, que no entienden var()). */
  function color(k: number | undefined): string {
    if (k === undefined) return '#8e8e93';
    const c = COLORES_RUTA[k % COLORES_RUTA.length];
    const m = /^var\((--[\w-]+)\)$/.exec(c);
    if (!m || !el) return c;
    return getComputedStyle(el).getPropertyValue(m[1]).trim() || '#007aff';
  }

  function icono(p: PuntoGeo): { icon: LT.DivIcon; clave: string } {
    const nota = p.nota ? `<span class="gm-nota">${esc(p.nota)}</span>` : '';
    const lab = `<span class="gm-lab">${esc(p.nombre)}${nota}</span>`;
    const html = p.deposito
      ? `<span class="gm-dep${p.sel ? ' sel' : ''}"><svg viewBox="0 0 16 16" aria-hidden="true"><path d="M3 8.5 8 4l5 4.5M4.5 7.5V12h7V7.5" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linejoin="round"/></svg></span>${lab}`
      : `<span class="gm-pin${p.sel ? ' sel' : ''}" style="--c:${color(p.ruta)}"></span>${lab}`;
    const clave = html;
    const icon = L!.divIcon({ className: 'gm-icono', html, iconSize: p.deposito ? [26, 26] : [18, 18], iconAnchor: p.deposito ? [13, 13] : [9, 9] });
    return { icon, clave };
  }

  function dibujarPuntos() {
    if (!L || !mapa || !capaPuntos) return;
    if (marcadores.length !== puntos.length) {
      capaPuntos.clearLayers();
      marcadores = [];
    }
    puntos.forEach((p, i) => {
      if (!Number.isFinite(p.lat) || !Number.isFinite(p.lon)) return;
      const ll = L!.latLng(p.lat, p.lon);
      const actual = marcadores[i];
      if (actual) {
        if (!actual.m.getLatLng().equals(ll)) actual.m.setLatLng(ll);
        const ic = icono(p);
        if (ic.clave !== actual.clave) {
          actual.m.setIcon(ic.icon);
          actual.clave = ic.clave;
        }
        actual.m.options.title = p.nombre;
        return;
      }
      const ic = icono(p);
      const m = L!.marker(ll, { icon: ic.icon, draggable: editable, keyboard: true, title: p.nombre, alt: p.nombre, autoPan: true, riseOnHover: true });
      m.on('click', () => onelegir?.(i));
      m.on('dragend', () => {
        const q = m.getLatLng();
        onmover?.(i, q.lat, q.lng);
      });
      m.addTo(capaPuntos!);
      marcadores[i] = { m, clave: ic.clave };
    });
  }

  function dibujarRutas() {
    if (!L || !mapa || !capaRutas) return;
    capaRutas.clearLayers();
    rutas.forEach((r, k) => {
      if (r.coords.length < 2) return;
      L!.polyline(r.coords, {
        color: color(k),
        weight: 5,
        opacity: r.vieja ? 0.45 : 0.85,
        dashArray: r.vieja ? '8 8' : undefined,
        lineJoin: 'round',
        interactive: false
      }).addTo(capaRutas!);
    });
  }

  /** Encuadra todos los puntos. */
  export function ajustar() {
    if (!L || !mapa) return;
    const ok = puntos.filter((p) => Number.isFinite(p.lat) && Number.isFinite(p.lon));
    if (!ok.length) {
      mapa.setView([4.6097, -74.0817], 12);
      return;
    }
    if (ok.length === 1) {
      mapa.setView([ok[0].lat, ok[0].lon], 14);
      return;
    }
    mapa.fitBounds(L.latLngBounds(ok.map((p) => [p.lat, p.lon] as [number, number])), { padding: [36, 36], maxZoom: 16 });
  }

  /** Centra el mapa en un punto sin cambiar el zoom. */
  export function centrar(i: number) {
    const p = puntos[i];
    if (mapa && p) mapa.panTo([p.lat, p.lon]);
  }

  onMount(() => {
    let vivo = true;
    let reloj: ReturnType<typeof setTimeout> | undefined;
    const enlaces = (ev: MouseEvent) => {
      // los enlaces de la atribución se abren en el navegador del sistema
      const a = (ev.target as HTMLElement | null)?.closest?.('a');
      if (a && a.closest('.leaflet-control-attribution') && a.href) {
        ev.preventDefault();
        ev.stopPropagation();
        openExternal(a.href);
      }
    };
    import('leaflet')
      .then((mod) => {
        if (!vivo || !el) return;
        const Lm = ((mod as unknown as { default?: typeof LT }).default ?? mod) as typeof LT;
        L = Lm;
        mapa = Lm.map(el, { zoomControl: true, attributionControl: true, keyboard: true, zoomSnap: 0.5 });
        mapa.attributionControl.setPrefix('<a href="https://leafletjs.com">Leaflet</a>');
        Lm.control.scale({ metric: true, imperial: false }).addTo(mapa);
        const capa = Lm.tileLayer(OSM, { maxZoom: 19, attribution: ATRIBUCION, referrerPolicy: 'strict-origin-when-cross-origin' } as LT.TileLayerOptions);
        let cargados = 0;
        let fallidos = 0;
        capa.on('tileload', () => {
          cargados++;
          sinMosaicos = false;
        });
        capa.on('tileerror', () => {
          fallidos++;
          if (!cargados && fallidos >= 2) sinMosaicos = true;
        });
        reloj = setTimeout(() => {
          if (!cargados) sinMosaicos = true;
        }, 8000);
        if (typeof navigator !== 'undefined' && navigator.onLine === false) sinMosaicos = true;
        capa.addTo(mapa);
        capaRutas = Lm.layerGroup().addTo(mapa);
        capaPuntos = Lm.layerGroup().addTo(mapa);
        mapa.on('click', (e: LT.LeafletMouseEvent) => {
          if (editable) onmapa?.(e.latlng.lat, e.latlng.lng);
        });
        el.addEventListener('click', enlaces, true);
        ajustar();
        dibujarRutas();
        dibujarPuntos();
      })
      .catch((e) => {
        error = String(e instanceof Error ? e.message : e);
      });
    // el contenedor cambia de tamaño (barra lateral, ventana): Leaflet debe saberlo
    const ro = new ResizeObserver(() => mapa?.invalidateSize());
    if (el) ro.observe(el);
    return () => {
      vivo = false;
      clearTimeout(reloj);
      ro.disconnect();
      el?.removeEventListener('click', enlaces, true);
      mapa?.remove();
      mapa = null;
    };
  });

  // cada cambio de los puntos o de las rutas se lleva al mapa
  $effect(() => {
    void JSON.stringify(puntos);
    if (L) dibujarPuntos();
  });
  $effect(() => {
    void JSON.stringify(rutas);
    if (L) dibujarRutas();
  });
</script>

<div class="geo" class:liso={sinMosaicos}>
  {#if error}<p class="aviso" role="alert">{t('No se pudo abrir el mapa: {0}', error)}</p>{/if}
  <div class="mapa" class:editable bind:this={el} style:height={`${alto}px`} role="region" aria-label={t('Mapa de los puntos')}></div>
  {#if sinMosaicos}
    <p class="aviso flota" role="status">
      {t('No se pudieron cargar los mapas de OpenStreetMap (¿sin conexión a internet?). Los puntos y las rutas se siguen viendo y editando sobre un fondo liso.')}
    </p>
  {/if}
</div>

<style>
  .geo {
    position: relative;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  /* el aviso va sobre el mapa, abajo, sin mover nada */
  .aviso.flota {
    position: absolute;
    z-index: 700;
    left: 50%;
    bottom: 30px;
    transform: translateX(-50%);
    width: max-content;
    max-width: min(640px, calc(100% - 24px));
    box-shadow: var(--shadow);
    background: color-mix(in oklab, var(--binding) 16%, var(--sheet));
    pointer-events: none;
  }
  .aviso {
    margin: 0;
    padding: 6px 10px;
    font-size: 12.5px;
    color: var(--ink);
    background: var(--binding-tint);
    border: 1px solid color-mix(in oklab, var(--binding) 45%, transparent);
    border-radius: var(--r-sm);
  }
  .mapa {
    width: 100%;
    border: 1px solid var(--rule);
    border-radius: var(--r-lg);
    overflow: hidden;
    background: var(--sheet-2);
    z-index: 0;
  }
  .mapa.editable :global(.leaflet-container),
  .mapa.editable:global(.leaflet-container) {
    cursor: crosshair;
  }
  .liso .mapa {
    background-color: var(--sheet-2);
    background-image:
      linear-gradient(var(--rule-soft) 1px, transparent 1px),
      linear-gradient(90deg, var(--rule-soft) 1px, transparent 1px);
    background-size: 32px 32px;
  }
  .liso .mapa :global(.leaflet-tile-pane) {
    display: none;
  }
  /* tema oscuro: los mosaicos claros se invierten para no deslumbrar */
  :global(html:not([data-tema='claro'])) .mapa :global(.leaflet-tile-pane) {
    filter: invert(1) hue-rotate(180deg) brightness(0.92) contrast(0.88) saturate(0.55);
  }
  .mapa :global(.leaflet-control-attribution) {
    background: color-mix(in oklab, var(--sheet) 85%, transparent);
    color: var(--ink-2);
    font-size: 10.5px;
  }
  .mapa :global(.leaflet-control-attribution a) {
    color: var(--solver);
  }
  .mapa :global(.leaflet-bar a) {
    background: var(--sheet);
    color: var(--ink);
    border-bottom-color: var(--rule);
  }
  .mapa :global(.leaflet-bar a:hover) {
    background: var(--sheet-2);
  }
  .mapa :global(.leaflet-control-scale-line) {
    background: color-mix(in oklab, var(--sheet) 80%, transparent);
    color: var(--ink-2);
    border-color: var(--ink-3);
  }
  .mapa :global(.gm-icono) {
    background: none;
    border: 0;
  }
  .mapa :global(.gm-icono:focus-visible) {
    outline: 2px solid var(--focus);
    outline-offset: 3px;
    border-radius: 50%;
  }
  .mapa :global(.gm-pin) {
    display: block;
    width: 18px;
    height: 18px;
    box-sizing: border-box;
    border-radius: 50%;
    background: #fff;
    border: 4px solid var(--c, var(--solver));
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.45);
  }
  .mapa :global(.gm-dep) {
    display: grid;
    place-items: center;
    width: 26px;
    height: 26px;
    box-sizing: border-box;
    border-radius: 7px;
    background: var(--solver);
    color: #fff;
    border: 2px solid #fff;
    box-shadow: 0 1px 4px rgba(0, 0, 0, 0.5);
  }
  .mapa :global(.gm-dep svg) {
    width: 16px;
    height: 16px;
  }
  .mapa :global(.gm-pin.sel),
  .mapa :global(.gm-dep.sel) {
    outline: 3px solid var(--focus);
    outline-offset: 2px;
  }
  .mapa :global(.gm-lab) {
    position: absolute;
    left: 50%;
    bottom: calc(100% + 3px);
    transform: translateX(-50%);
    white-space: nowrap;
    font: 600 11.5px/1.2 var(--font-ui);
    color: #1d1d1f;
    text-shadow:
      0 0 3px #fff,
      0 0 3px #fff,
      0 0 2px #fff;
    pointer-events: none;
  }
  .liso .mapa :global(.gm-lab),
  :global(html:not([data-tema='claro'])) .mapa :global(.gm-lab) {
    color: var(--ink);
    text-shadow:
      0 0 3px var(--sheet),
      0 0 3px var(--sheet),
      0 0 2px var(--sheet);
  }
  /* el nombre del depósito va debajo, para no chocar con los clientes cercanos */
  .mapa :global(.gm-dep + .gm-lab) {
    bottom: auto;
    top: calc(100% + 3px);
  }
  .mapa :global(.gm-nota) {
    display: block;
    text-align: center;
    font-weight: 500;
    font-size: 10.5px;
    opacity: 0.8;
  }
</style>
