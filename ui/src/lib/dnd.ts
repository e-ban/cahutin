/** Type de donnée du glisser-déposer d'une vignette de photo vers la page. */
export const ASSET_MIME = "application/x-carnet-asset";

/** Type de donnée du glisser-déposer d'une image de la banque OpenMoji (son code). */
export const PICTO_MIME = "application/x-carnet-picto";
/** Une image de « Mes images » glissée sur la page : son nom de fichier. */
export const MY_IMAGE_MIME = "application/x-carnet-my-image";
/** Une photo de la photothèque de la classe glissée sur la page : son identifiant. */
export const CLASS_PHOTO_MIME = "application/x-carnet-class-photo";
/** Une forme du tiroir Formes glissée sur la page : sa tuile, en JSON (`ShapeTile`). */
export const SHAPE_MIME = "application/x-carnet-shape";

import type { LineEnds, ShapeKind } from "./types";

/** Une tuile du tiroir Formes : la forme, ses coins, pour un trait ses terminaisons, pour une bulle sa pointe. */
export type ShapeTile = { shape: ShapeKind; radius: number; ends?: LineEnds; tail?: [number, number] };

const KINDS: ShapeKind[] = ["Rect", "Ellipse", "Line", "Arrow", "Star", "Heart", "Bubble", "Cloud"];

export const shapeTile = (t: ShapeTile) => JSON.stringify(t);

/** La tuile glissée, ou `null` si ce n'en est pas une. */
export function parseShapeTile(raw: string): ShapeTile | null {
  try {
    const t = JSON.parse(raw) as Partial<ShapeTile>;
    if (!t || !KINDS.includes(t.shape as ShapeKind) || typeof t.radius !== "number") return null;
    const tail = Array.isArray(t.tail) && t.tail.length === 2 && t.tail.every((v) => typeof v === "number" && Number.isFinite(v)) ? (t.tail as [number, number]) : undefined;
    return { shape: t.shape as ShapeKind, radius: t.radius, ...(t.ends ? { ends: t.ends } : {}), ...(tail ? { tail } : {}) };
  } catch {
    return null;
  }
}
