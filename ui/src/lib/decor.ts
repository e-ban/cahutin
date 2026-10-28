// Décors de page : mention de licence OpenMoji (même cadre que l'export ODG), cadres et frises.
import { newId } from "./factory";
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

/** Copies régulièrement espacées d'une image le long des marges, groupées en une frise. */
export function frieze(page: Page, code: string, side: FriezeSide, size_mm = 15): Element[] {
  const { width_mm: w, height_mm: h } = page.size;
  const xs = spread(MARGIN_MM, w - 2 * MARGIN_MM, size_mm);
  const top = MARGIN_MM;
  const bottom = round(h - MARGIN_MM - size_mm);
  const positions: [number, number][] = [];
  if (side !== "bottom") positions.push(...xs.map((x): [number, number] => [x, top]));
  if (side !== "top") positions.push(...xs.map((x): [number, number] => [x, bottom]));
  if (side === "around") {
    // Côtés : sans les coins, déjà posés par les lignes du haut et du bas.
    const ys = spread(MARGIN_MM, h - 2 * MARGIN_MM, size_mm).slice(1, -1);
    const right = round(w - MARGIN_MM - size_mm);
    for (const y of ys) positions.push([MARGIN_MM, y], [right, y]);
  }
  const group = newId();
  return positions.map(([x, y]) => {
    const e = pictoElement(page, code, undefined, size_mm);
    return { ...e, frame: { ...e.frame, x_mm: x, y_mm: y }, group };
  });
}
