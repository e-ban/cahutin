// Décors de page : mention de licence OpenMoji (même cadre que l'export ODG), cadres et frises.
import { newId } from "./factory";
import { rotateElements } from "./groups";
import { pictoElement } from "./pictos.svelte";
import type { Element, Frame, Page, PageSize } from "./types";

/** Mention en bas des pages qui contiennent une image OpenMoji (carnet_core::odg::content::credit_frame). */
export const CREDIT_SIZE_PT = 6;
export const CREDIT_MARGIN_MM = 8;

export function creditFrame(size: PageSize): Frame {
  return { x_mm: CREDIT_MARGIN_MM, y_mm: size.height_mm - CREDIT_MARGIN_MM, w_mm: size.width_mm - 2 * CREDIT_MARGIN_MM, h_mm: 5, rotation_deg: 0 };
}

const MARGIN_MM = 8;
const FRAME_COLOR = "#555555";

export type FrameStyle = "thin" | "thick" | "rounded" | "double";

function border(page: Page, inset: number, width_mm: number, corner_radius_mm: number): Element {
  const { width_mm: w, height_mm: h } = page.size;
  return {
    id: newId(),
    frame: { x_mm: inset, y_mm: inset, w_mm: w - 2 * inset, h_mm: h - 2 * inset, rotation_deg: 0 },
    locked: false,
    zone: false,
    kind: { type: "Shape", shape: "Rect", fill: null, stroke: { width_mm, color: FRAME_COLOR }, corner_radius_mm },
  };
}

/** Cadre le long de la marge de 8 mm ; le double est un groupe de deux rectangles. */
export function pageFrame(page: Page, style: FrameStyle): Element[] {
  switch (style) {
    case "thin":
      return [border(page, MARGIN_MM, 0.5, 0)];
    case "thick":
      return [border(page, MARGIN_MM, 2, 0)];
    case "rounded":
      return [border(page, MARGIN_MM, 1, 8)];
    case "double": {
      const group = newId();
      return [border(page, MARGIN_MM, 0.5, 0), border(page, MARGIN_MM + 3, 1.5, 0)].map((e) => ({ ...e, group }));
    }
  }
}

export type FriezeSide = "top" | "bottom" | "around";
const MIN_GAP_MM = 3;
const round = (v: number) => Math.round(v * 10) / 10;

/** Positions régulières de `size` le long d'une longueur utile, la première au début, la dernière à la fin. */
function spread(start: number, length: number, size: number): number[] {
  const n = Math.max(1, Math.floor((length + MIN_GAP_MM) / (size + MIN_GAP_MM)));
  if (n === 1) return [round(start + (length - size) / 2)];
  const step = (length - size) / (n - 1);
  return Array.from({ length: n }, (_, i) => round(start + i * step));
}

type Rect = { x_mm: number; y_mm: number; w_mm: number; h_mm: number };

/**
 * Positions des copies le long d'un rectangle. Tout autour : une seule boucle dans le sens horaire
 * (haut, droite, bas, gauche), sans doublon aux coins, pour que la suite des motifs ne s'interrompe pas.
 */
function friezePositions(r: Rect, side: FriezeSide, size: number): [number, number][] {
  const xs = spread(r.x_mm, r.w_mm, size);
  const top = r.y_mm;
  const bottom = round(r.y_mm + r.h_mm - size);
  if (side === "top") return xs.map((x) => [x, top]);
  if (side === "bottom") return xs.map((x) => [x, bottom]);
  const ys = spread(r.y_mm, r.h_mm, size).slice(1, -1);
  const right = round(r.x_mm + r.w_mm - size);
  return [
    ...xs.map((x): [number, number] => [x, top]),
    ...ys.map((y): [number, number] => [right, y]),
    ...[...xs].reverse().map((x): [number, number] => [x, bottom]),
    ...[...ys].reverse().map((y): [number, number] => [r.x_mm, y]),
  ];
}

/** Copies régulièrement espacées des motifs, répétés dans l'ordre, groupées en une frise. */
export function friezeInRect(page: Page, r: Rect, codes: string[], side: FriezeSide, size_mm = 15): Element[] {
  const group = newId();
  return friezePositions(r, side, size_mm).map(([x, y], i) => {
    const e = pictoElement(page, codes[i % codes.length], undefined, size_mm);
    return { ...e, frame: { ...e.frame, x_mm: x, y_mm: y }, group };
  });
}

/** Frise le long des marges de la page. */
export function frieze(page: Page, codes: string[], side: FriezeSide, size_mm = 15): Element[] {
  const { width_mm: w, height_mm: h } = page.size;
  return friezeInRect(page, { x_mm: MARGIN_MM, y_mm: MARGIN_MM, w_mm: w - 2 * MARGIN_MM, h_mm: h - 2 * MARGIN_MM }, codes, side, size_mm);
}

/** Motifs d'une frise : un clic ajoute l'image à la fin de la suite, ou l'en retire. */
export function toggleMotif(codes: string[], code: string): string[] {
  return codes.includes(code) ? codes.filter((c) => c !== code) : [...codes, code];
}

/**
 * Frise tout autour d'un bloc (photo, forme, texte) : motifs centrés sur ses bords, du quart de son plus petit
 * côté (6 à 15 mm), tournés avec lui et groupés avec lui (dans son groupe s'il en a un). `null` pour une case
 * de grille, jamais groupée, ou sans motif.
 */
export function blockFrieze(page: Page, id: string, codes: string[]): Page | null {
  const target = page.elements.find((e) => e.id === id);
  if (!target || codes.length === 0 || page.grid?.cells.some((c) => c.element_id === id)) return null;
  const f = target.frame;
  const size = round(Math.min(15, Math.max(6, Math.min(f.w_mm, f.h_mm) / 4)));
  const rect = { x_mm: f.x_mm - size / 2, y_mm: f.y_mm - size / 2, w_mm: f.w_mm + size, h_mm: f.h_mm + size };
  const group = target.group ?? newId();
  let pics: Element[] = friezeInRect(page, rect, codes, "around", size).map((e) => ({ ...e, group }));
  if (f.rotation_deg) pics = rotateElements(pics, f.rotation_deg, { x_mm: f.x_mm + f.w_mm / 2, y_mm: f.y_mm + f.h_mm / 2 });
  const at = page.elements.indexOf(target);
  const elements = [...page.elements.slice(0, at), { ...target, group }, ...pics, ...page.elements.slice(at + 1)];
  return { ...page, elements };
}
