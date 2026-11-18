// Dessin d'un trait et de ses terminaisons dans son cadre (px), à l'écran comme à l'impression. Mêmes proportions
// que les marqueurs de l'export (crates/carnet-core/src/odg/styles.rs) : pointe de base `head` et de longueur
// 1,5 × `head`, chevron dans le même triangle, rond et carré de `DOT_FACTOR` × `head`.
import { DOT_FACTOR } from "./shapePaths";
import type { LineEnd, LineEnds } from "./types";

export type LinePart = { kind: "polygon" | "polyline" | "circle" | "rect"; attrs: Record<string, number | string> };

/** Le segment (de `x1` à `x2`, sur la ligne médiane) et les terminaisons d'un trait de `w` × `h` px. */
export function lineSvg(w: number, h: number, ends: LineEnds, head: number): { x1: number; x2: number; parts: LinePart[] } {
  const y = h / 2;
  const len = head * 1.5;
  const dot = head * DOT_FACTOR;
  const parts: LinePart[] = [];
  // `tip` : le bout ; `dir` : 1 si le trait part vers la droite depuis ce bout, -1 sinon. Rend le recul du segment.
  const draw = (end: LineEnd, tip: number, dir: 1 | -1): number => {
    const base = tip + dir * len;
    switch (end) {
      case "Arrow":
        parts.push({ kind: "polygon", attrs: { points: `${tip},${y} ${base},${y - head / 2} ${base},${y + head / 2}` } });
        return len / 2;
      case "OpenArrow":
        parts.push({ kind: "polyline", attrs: { points: `${base},${y - head / 2} ${tip},${y} ${base},${y + head / 2}`, "stroke-width": head / 5 } });
        return 0;
      case "Circle":
        parts.push({ kind: "circle", attrs: { cx: tip, cy: y, r: dot / 2 } });
        return 0;
      case "Square":
        parts.push({ kind: "rect", attrs: { x: tip - dot / 2, y: y - dot / 2, width: dot, height: dot } });
        return 0;
      default:
        return 0;
    }
  };
  const x1 = draw(ends.start, 0, 1);
  const x2 = w - draw(ends.end, w, -1);
  return { x1, x2, parts };
}
