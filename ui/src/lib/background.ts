// Motifs de fond (Seyès, petits carreaux, lignes, pointillés) : même géométrie que le moteur
// (carnet_core::background), vérifiée contre fixtures/background_marks.json.
import type { PageSize, Pattern } from "./types";

export const KRAFT = "#d8bc8a";

export type Mark =
  | { type: "Line"; x1: number; y1: number; x2: number; y2: number; width_mm: number; color: string }
  | { type: "Dot"; x: number; y: number; d_mm: number; color: string };

const SEYES_THIN = "#c9c3e6";
const SEYES_STRONG = "#9d93d4";
const SEYES_MARGIN = "#e8a2a2";
const SEYES_MARGIN_MM = 25;
const SQUARES = "#b9d3ee";
const LINES = "#c8c8c8";
const DOTS = "#bdbdbd";

/** Multiples de `step` strictement à l'intérieur de `limit`, avec leur rang (1, 2, …). */
function steps(step: number, limit: number): [number, number][] {
  const out: [number, number][] = [];
  for (let k = 1; ; k++) {
    const v = Math.round(k * step * 1000) / 1000;
    if (v >= limit) return out;
    out.push([k, v]);
  }
}

const h = (y: number, w: number, width_mm: number, color: string): Mark => ({ type: "Line", x1: 0, y1: y, x2: w, y2: y, width_mm, color });
const v = (x: number, ht: number, width_mm: number, color: string): Mark => ({ type: "Line", x1: x, y1: 0, x2: x, y2: ht, width_mm, color });

export function marks(size: PageSize, pattern: Pattern): Mark[] {
  const { width_mm: w, height_mm: ht } = size;
  switch (pattern) {
    case "Seyes":
      return [
        ...steps(2, ht).map(([k, y]) => (k % 4 === 0 ? h(y, w, 0.25, SEYES_STRONG) : h(y, w, 0.15, SEYES_THIN))),
        ...steps(8, w).map(([, x]) => v(x, ht, 0.25, SEYES_STRONG)),
        v(SEYES_MARGIN_MM, ht, 0.3, SEYES_MARGIN),
      ];
    case "SmallSquares":
      return [...steps(5, ht).map(([, y]) => h(y, w, 0.15, SQUARES)), ...steps(5, w).map(([, x]) => v(x, ht, 0.15, SQUARES))];
    case "Lines":
      return steps(8, ht).map(([, y]) => h(y, w, 0.2, LINES));
    case "Dots":
      return steps(10, ht).flatMap(([, y]) => steps(10, w).map(([, x]): Mark => ({ type: "Dot", x, y, d_mm: 0.6, color: DOTS })));
  }
}
