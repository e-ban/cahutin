// Dessin à main levée : un tracé (les positions du pointeur, en mm de page) devient un chemin du palier 2, lissé
// (Ramer-Douglas-Peucker) et en courbe passant par les points gardés. On peut ensuite le modifier comme tout chemin.
import { shapeElement } from "./factory";
import { fromPagePoints, MAX_PATH_POINTS, type Pt } from "./linePath";
import type { Element, Page } from "./types";

/** Écart toléré entre le tracé et sa version lissée, en mm. */
export const STROKE_TOLERANCE_MM = 0.8;
/** Un tracé plus petit que ça (un clic, un tremblement) ne dessine rien. */
const MIN_STROKE_MM = 2;

/** Distance du point `p` au segment `a`-`b`. */
function toSegment(p: Pt, a: Pt, b: Pt): number {
  const [dx, dy] = [b.x - a.x, b.y - a.y];
  const len2 = dx * dx + dy * dy;
  const t = len2 ? Math.max(0, Math.min(1, ((p.x - a.x) * dx + (p.y - a.y) * dy) / len2)) : 0;
  return Math.hypot(p.x - (a.x + t * dx), p.y - (a.y + t * dy));
}

/** Ramer-Douglas-Peucker : les points qui s'écartent de plus de `tol` du segment qui les saute. */
function rdp(pts: Pt[], tol: number): Pt[] {
  const keep = new Array<boolean>(pts.length).fill(false);
  keep[0] = keep[pts.length - 1] = true;
  const stack: [number, number][] = [[0, pts.length - 1]];
  while (stack.length) {
    const [i, j] = stack.pop()!;
    let [best, at] = [-1, -1];
    for (let k = i + 1; k < j; k++) {
      const d = toSegment(pts[k], pts[i], pts[j]);
      if (d > best) [best, at] = [d, k];
    }
    if (at > 0 && best > tol) {
      keep[at] = true;
      stack.push([i, at], [at, j]);
    }
  }
  return pts.filter((_, k) => keep[k]);
}

/** Le tracé lissé : ses bouts, et les points qui s'écartent de plus de `tol` mm ; jamais plus de 200 points (la
 *  tolérance grandit jusqu'à tenir). */
export function simplifyStroke(pts: Pt[], tol = STROKE_TOLERANCE_MM): Pt[] {
  if (pts.length <= 2) return pts.slice();
  let t = tol;
  let out = rdp(pts, t);
  while (out.length > MAX_PATH_POINTS) {
    t *= 1.5;
    out = rdp(pts, t);
  }
  return out;
}

/** Le chemin d'un tracé à main levée (trait de 1 mm, comme le tiroir Formes), ou `null` pour un clic ou un tracé de
 *  moins de 2 mm. Deux points gardés : un trait droit ; plus : un chemin en courbe. */
export function strokeElement(page: Page, pts: Pt[]): Element | null {
  if (pts.length < 2) return null;
  const xs = pts.map((p) => p.x);
  const ys = pts.map((p) => p.y);
  if (Math.hypot(Math.max(...xs) - Math.min(...xs), Math.max(...ys) - Math.min(...ys)) < MIN_STROKE_MM) return null;
  const kept = simplifyStroke(pts);
  const base = shapeElement(page, "Line");
  if (base.kind.type !== "Shape") return null;
  const kind = { ...base.kind, stroke: base.kind.stroke ?? { width_mm: 1, color: "#333333", style: null }, curve: kept.length >= 3 || undefined };
  const r = fromPagePoints(kept, kind);
  const all = { ...kind, ...r.patch };
  const clean = Object.fromEntries(Object.entries(all).filter(([, v]) => v !== undefined)) as typeof all;
  return { ...base, frame: r.frame, kind: clean };
}
