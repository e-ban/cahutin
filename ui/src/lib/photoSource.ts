import { mmToPx } from "./units";

/** Plus grand côté de la vignette d'une photo (`photo::THUMB_LONG_EDGE_PX` côté moteur). */
export const THUMB_LONG_EDGE_PX = 256;

/**
 * Vrai quand la photo, affichée à cette taille, tient dans sa vignette (en pixels de l'écran) : les aperçus
 * (bande des pages, accueil, vis-à-vis) chargent alors la vignette, pas l'image entière. À l'impression (et à
 * l'export PDF, qui passe par elle), toujours l'image entière : le papier a bien plus de points que l'écran.
 */
export function useThumb(widthMm: number, heightMm: number, zoom: number, pixelRatio: number, printing = false): boolean {
  return !printing && Math.max(mmToPx(widthMm, zoom), mmToPx(heightMm, zoom)) * pixelRatio <= THUMB_LONG_EDGE_PX;
}
