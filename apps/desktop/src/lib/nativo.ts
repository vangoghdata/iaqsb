// Nativo, el robot de IA Nativa, en el asistente de IAQSB. Las animaciones las hace
// ia-nativa-videos/personajes/panel_iaqsb.py (Blender) y iaqsb.mjs las pasa a webp animado con
// transparencia en public/nativo/ (<toma>.webp, y <toma>-0.webp quieto para quien prefiere menos
// movimiento).

export type TomaNativo = 'reposo' | 'pensando' | 'calculando' | 'explicando' | 'celebrando' | 'confundido' | 'saludo' | 'chat';

/** Duración de cada toma (ms) y si se repite sola (las demás se muestran una vez y vuelve al reposo). */
export const TOMAS: Record<TomaNativo, { ms: number; bucle: boolean }> = {
  reposo: { ms: 6000, bucle: true },
  pensando: { ms: 4000, bucle: true },
  calculando: { ms: 4000, bucle: true },
  explicando: { ms: 4000, bucle: true },
  confundido: { ms: 4000, bucle: true },
  celebrando: { ms: 2500, bucle: false },
  saludo: { ms: 4500, bucle: false },
  chat: { ms: 14000, bucle: false }
};

export function srcNativo(toma: TomaNativo, quieto = false): string {
  return `/nativo/${toma}${quieto ? '-0' : ''}.webp`;
}

/** Con «reducir movimiento» del sistema, Nativo queda quieto. */
export function prefiereQuieto(): boolean {
  try {
    return window.matchMedia?.('(prefers-reduced-motion: reduce)').matches ?? false;
  } catch {
    return false;
  }
}
