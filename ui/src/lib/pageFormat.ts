// Formats de page (A5, A4, A3) et impression d'une affiche A3 sur deux feuilles A4.
import { scaleElements } from "./groups";
import type { Page, PageSize } from "./types";

export type Format = "A5" | "A4" | "A3";

/** Petit et grand côté, en mm. */
export const FORMATS: Record<Format, [number, number]> = { A5: [148, 210], A4: [210, 297], A3: [297, 420] };

const near = (a: number, b: number) => Math.abs(a - b) < 0.5;

/** Le format de la page, s'il est connu (en portrait ou en paysage). */
export function formatOf(size: PageSize): Format | null {
  const short = Math.min(size.width_mm, size.height_mm);
  const long = Math.max(size.width_mm, size.height_mm);
  return (Object.keys(FORMATS) as Format[]).find((f) => near(FORMATS[f][0], short) && near(FORMATS[f][1], long)) ?? null;
}

export const isLandscape = (size: PageSize) => size.width_mm > size.height_mm;

/** Taille de la page dans un autre format, dans la même orientation. */
export function sizeIn(format: Format, landscape: boolean): PageSize {
  const [s, l] = FORMATS[format];
  return landscape ? { width_mm: l, height_mm: s } : { width_mm: s, height_mm: l };
}

/** La page dans un autre format : son contenu est agrandi ou réduit d'autant (les formats A ont les mêmes
 *  proportions). La grille éventuelle est recalculée ensuite par le moteur. */
export function changeFormat(page: Page, format: Format): Page {
  const size = sizeIn(format, isLandscape(page.size));
  const factor = size.width_mm / page.size.width_mm;
  const gridIds = new Set(page.grid?.cells.map((c) => c.element_id) ?? []);
  const scaled = scaleElements(page.elements.filter((e) => !gridIds.has(e.id)), factor, { x_mm: 0, y_mm: 0 });
  const byId = new Map(scaled.map((e) => [e.id, e]));
  return { ...page, size, elements: page.elements.map((e) => byId.get(e.id) ?? e) };
}

/** Nom de la page CSS d'impression (`@page a3p` dans print.css). */
export function printPageName(size: PageSize): string {
  return `${(formatOf(size) ?? "A4").toLowerCase()}${isLandscape(size) ? "l" : "p"}`;
}

/** Recouvrement entre les deux moitiés d'une affiche A3 imprimée sur A4, pour les coller (mm sur l'A4). */
export const TILE_OVERLAP_MM = 10;

export interface Tile {
  /** Feuille A4 (paysage pour une affiche en portrait, et inversement). */
  sheet: PageSize;
  /** Réduction de l'affiche pour que les deux moitiés se recouvrent. */
  scale: number;
  /** Décalage de l'affiche réduite sur la feuille, en mm. */
  dx_mm: number;
  dy_mm: number;
}

/** Les deux feuilles A4 d'une page A3 : la moitié haute puis la basse (ou gauche puis droite en paysage). */
export function a3Tiles(size: PageSize): Tile[] {
  const landscape = isLandscape(size);
  const sheet = sizeIn("A4", !landscape);
  // Côté découpé : deux demi-feuilles plus le recouvrement couvrent toute l'affiche réduite.
  const long = Math.max(size.width_mm, size.height_mm);
  const scale = (2 * FORMATS.A4[0] - TILE_OVERLAP_MM) / long;
  const across = (Math.min(size.width_mm, size.height_mm) * scale - FORMATS.A4[1]) / -2;
  const second = -(long * scale - FORMATS.A4[0]);
  return landscape
    ? [
        { sheet, scale, dx_mm: 0, dy_mm: across },
        { sheet, scale, dx_mm: second, dy_mm: across },
      ]
    : [
        { sheet, scale, dx_mm: across, dy_mm: 0 },
        { sheet, scale, dx_mm: across, dy_mm: second },
      ];
}
