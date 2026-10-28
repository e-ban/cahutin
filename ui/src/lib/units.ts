// Le modèle est en millimètres ; l'écran en pixels CSS (96 par pouce), multipliés par le zoom.
export const PX_PER_MM = 96 / 25.4;
export const PX_PER_PT = 96 / 72;

export function mmToPx(mm: number, zoom: number): number {
  return mm * PX_PER_MM * zoom;
}

/** Arrondi à 0,1 mm, la précision des éditions à la souris. */
export function pxToMm(px: number, zoom: number): number {
  return Math.round((px / (PX_PER_MM * zoom)) * 10) / 10;
}

export function ptToPx(pt: number, zoom: number): number {
  return pt * PX_PER_PT * zoom;
}

export const ZOOM_LEVELS = [0.25, 0.5, 0.75, 1, 1.25, 1.5, 2, 3];

/** Niveau de zoom suivant (+1) ou précédent (-1), depuis n'importe quelle valeur (zoom « ajuster »). */
export function nextZoom(current: number, dir: 1 | -1): number {
  const eps = 1e-6;
  if (dir > 0) return ZOOM_LEVELS.find((z) => z > current + eps) ?? ZOOM_LEVELS[ZOOM_LEVELS.length - 1];
  return [...ZOOM_LEVELS].reverse().find((z) => z < current - eps) ?? ZOOM_LEVELS[0];
}
