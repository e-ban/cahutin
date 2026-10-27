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
