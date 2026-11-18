// Dessin d'un trait et de ses terminaisons dans son cadre (px), à l'écran comme à l'impression. Mêmes proportions
// que les marqueurs de l'export (crates/carnet-core/src/odg/styles.rs) : pointe de base `head` et de longueur
// 1,5 × `head`, chevron dans le même triangle, rond et carré de `DOT_FACTOR` × `head`.
import { DOT_FACTOR } from "./shapePaths";
import { closedCurveBeziers, curveBeziers, endDirections, type Pt } from "./linePath";
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
export function pathSvg(pts: Pt[], curve: boolean, ends: LineEnds, head: number, closed = false): { d: string; parts: LinePart[] } {
  const at = (p: Pt) => `${fmt(p.x)} ${fmt(p.y)}`;
  // Fermé : pas de bouts, donc pas de terminaisons ; le tracé revient au premier point.
  if (closed) {
    const body = curve ? closedCurveBeziers(pts).map(([c1, c2, p]) => ` C ${at(c1)} ${at(c2)} ${at(p)}`).join("") : pts.slice(1).map((p) => ` L ${at(p)}`).join("");
    return { d: `M ${at(pts[0])}${body} Z`, parts: [] };
  }
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
  // Recul d'une flèche pleine : jamais plus de 90 % du segment du bout (il ne dépasse pas le point voisin).
  const share = (by: number, a: Pt, b: Pt) => {
    const len = Math.hypot(b.x - a.x, b.y - a.y);
    return len > 1e-9 ? Math.min(0.9, by / len) : 0;
  };
  const ts = start.x1 ? share(start.x1, pts[0], pts[1]) : 0;
  const te = -end.x2 ? share(-end.x2, pts[n - 1], pts[n - 2]) : 0;
  if (!curve) {
    const lerp = (a: Pt, b: Pt, t: number): Pt => ({ x: a.x + (b.x - a.x) * t, y: a.y + (b.y - a.y) * t });
    const drawn = pts.map((p, i) => (i === 0 ? lerp(p, pts[1], ts) : i === n - 1 ? lerp(p, pts[n - 2], te) : p));
    return { d: `M ${at(drawn[0])}` + drawn.slice(1).map((p) => ` L ${at(p)}`).join(""), parts };
  }
  // Courbe : celle des vrais points (comme à l'export), raccourcie au bout par une coupe de sa première ou dernière
  // cubique.
  const bz = curveBeziers(pts);
  let first = pts[0];
  if (ts) [first, ...bz[0]] = splitAfter([pts[0], ...bz[0]], ts);
  if (te) {
    const k = bz.length - 1;
    const from = k === 0 ? first : bz[k - 1][2];
    const [, c1, c2, p] = splitBefore([from, ...bz[k]], 1 - te);
    bz[k] = [c1, c2, p];
  }
  return { d: `M ${at(first)}` + bz.map(([c1, c2, p]) => ` C ${at(c1)} ${at(c2)} ${at(p)}`).join(""), parts };
}

type Cubic = [Pt, Pt, Pt, Pt];
const mix = (a: Pt, b: Pt, t: number): Pt => ({ x: a.x + (b.x - a.x) * t, y: a.y + (b.y - a.y) * t });
/** De Casteljau : les deux moitiés d'une cubique coupée en `t`. */
function split([p0, p1, p2, p3]: Cubic, t: number): [Cubic, Cubic] {
  const [a, b, c] = [mix(p0, p1, t), mix(p1, p2, t), mix(p2, p3, t)];
  const [d, e] = [mix(a, b, t), mix(b, c, t)];
  const f = mix(d, e, t);
  return [[p0, a, d, f], [f, e, c, p3]];
}
/** La part de la cubique après `t`. */
const splitAfter = (c: Cubic, t: number): Cubic => split(c, t)[1];
/** La part de la cubique avant `t`. */
const splitBefore = (c: Cubic, t: number): Cubic => split(c, t)[0];
