// Dessin d'un trait et de ses terminaisons dans son cadre (px), à l'écran comme à l'impression. Mêmes proportions
// que les marqueurs de l'export (crates/carnet-core/src/odg/styles.rs) : pointe de base `head` et de longueur
// 1,5 × `head`, chevron dans le même triangle, rond et carré de `DOT_FACTOR` × `head`.
import { DOT_FACTOR } from "./shapePaths";
import { curveBeziers, endDirections, type Pt } from "./linePath";
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

const fmt = (v: number) => String(Math.round(v * 1e4) / 1e4 + 0);

/** Tracé d'un chemin (mm de son cadre) et ses terminaisons, orientées selon la direction d'arrivée à chaque bout :
 *  mêmes dessins que `lineSvg`, posés sur le bout par `translate` puis `rotate`. Une flèche pleine recule le tracé
 *  de la moitié de sa longueur, le long de cette direction. */
export function pathSvg(pts: Pt[], curve: boolean, ends: LineEnds, head: number): { d: string; parts: LinePart[] } {
  const dir = endDirections(pts, curve);
  const parts: LinePart[] = [];
  const place = (one: LinePart[], tip: Pt, deg: number) => {
    for (const p of one) parts.push({ kind: p.kind, attrs: { ...p.attrs, transform: `translate(${fmt(tip.x)} ${fmt(tip.y)}) rotate(${fmt(deg)})` } });
  };
  // Une terminaison seule, pointe en (0, 0) : le début regarde vers -x, la fin vers +x.
  const start = lineSvg(0, 0, { start: ends.start, end: "None", size: ends.size }, head);
  const end = lineSvg(0, 0, { start: "None", end: ends.end, size: ends.size }, head);
  const n = pts.length;
  place(start.parts, pts[0], dir.start - 180);
  place(end.parts, pts[n - 1], dir.end);
  const back = (p: Pt, deg: number, by: number): Pt => (by ? { x: p.x - by * Math.cos((deg * Math.PI) / 180), y: p.y - by * Math.sin((deg * Math.PI) / 180) } : p);
  const drawn = pts.map((p, i) => (i === 0 ? back(p, dir.start, start.x1) : i === n - 1 ? back(p, dir.end, -end.x2) : p));
  const at = (p: Pt) => `${fmt(p.x)} ${fmt(p.y)}`;
  const d = curve
    ? `M ${at(drawn[0])}` + curveBeziers(drawn).map(([c1, c2, p]) => ` C ${at(c1)} ${at(c2)} ${at(p)}`).join("")
    : `M ${at(drawn[0])}` + drawn.slice(1).map((p) => ` L ${at(p)}`).join("");
  return { d, parts };
}
