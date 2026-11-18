// Chemin à coudes ou courbe : un trait ou une flèche qui a des `points` (fractions de son cadre, avant rotation et
// miroir). Les gestes travaillent sur les points visibles en mm de page (`pagePoints`) ; le résultat redevient un
// cadre et des fractions (`fromPagePoints`), sans rotation ni miroir. Mêmes règles que `shapes.rs` (valid_points,
// curve_beziers).
import { dragEnd, frameFromEndpoints, headWidthMm, lineEndpoints, type Pt } from "./lineEnds";
import type { Frame, ShapeProps } from "./types";

export type { Pt };

/** Plus grand nombre de points d'un chemin. */
export const MAX_PATH_POINTS = 200;

/** Points acceptables : 3 à 200, finis, dans le cadre (fractions de 0 à 1). */
export function validPoints(p: [number, number][] | undefined | null): p is [number, number][] {
  const EPS = 1e-6;
  return !!p && p.length >= 3 && p.length <= MAX_PATH_POINTS && p.every((xy) => xy.every((v) => Number.isFinite(v) && v >= -EPS && v <= 1 + EPS));
}

/** Un trait qui a des coudes. */
export const isPath = (s: ShapeProps): boolean => validPoints(s.points);

/** Points d'un chemin en mm de son cadre (avant rotation et miroir) ; vide s'il n'en a pas. */
export const pathPointsMm = (frame: Frame, s: ShapeProps): Pt[] => (isPath(s) ? s.points!.map(([x, y]) => ({ x: x * frame.w_mm, y: y * frame.h_mm })) : []);

/** Zone cliquable autour du tracé : la plus grande terminaison, 4 mm au moins. */
export const hitMm = (s: ShapeProps) => Math.max(4, headWidthMm(s));

/** Points visibles en mm de page (rotation et miroir compris) ; un trait droit : ses deux bouts. */
export function pagePoints(frame: Frame, s: ShapeProps, flipH: boolean, flipV: boolean): Pt[] {
  if (!isPath(s)) return lineEndpoints(frame, flipH);
  const cx = frame.x_mm + frame.w_mm / 2;
  const cy = frame.y_mm + frame.h_mm / 2;
  const a = (frame.rotation_deg * Math.PI) / 180;
  const [cos, sin] = [Math.cos(a), Math.sin(a)];
  return s.points!.map(([fx, fy]) => {
    const dx = (flipH ? 1 - fx : fx) * frame.w_mm - frame.w_mm / 2;
    const dy = (flipV ? 1 - fy : fy) * frame.h_mm - frame.h_mm / 2;
    return { x: cx + dx * cos - dy * sin, y: cy + dx * sin + dy * cos };
  });
}

/** Cadre et champs d'un trait qui passe par `pts` (mm de page) : deux points, un trait droit ; plus, un chemin dont
 *  le cadre est la boîte des points agrandie de la moitié de la zone cliquable, sans rotation ni miroir. */
export function fromPagePoints(pts: Pt[], s: ShapeProps): { frame: Frame; patch: Pick<ShapeProps, "points" | "curve" | "closed">; flipH: false; flipV: false } {
  const hit = hitMm(s);
  if (pts.length <= 2) return { frame: frameFromEndpoints(pts[0], pts[pts.length - 1], hit, false), patch: { points: undefined, curve: undefined, closed: undefined }, flipH: false, flipV: false };
  const xs = pts.map((p) => p.x);
  const ys = pts.map((p) => p.y);
  const m = hit / 2;
  const x0 = Math.min(...xs) - m;
  const y0 = Math.min(...ys) - m;
  const w = Math.max(...xs) + m - x0;
  const h = Math.max(...ys) + m - y0;
  return {
    frame: { x_mm: x0, y_mm: y0, w_mm: w, h_mm: h, rotation_deg: 0 },
    patch: { points: pts.map((p) => [(p.x - x0) / w, (p.y - y0) / h] as [number, number]), curve: s.curve || undefined, closed: s.closed || undefined },
    flipH: false,
    flipV: false,
  };
}

/** Un point ajouté après l'indice `after` ; inchangé à 200 points. */
export const insertPoint = (pts: Pt[], after: number, p: Pt): Pt[] => (pts.length >= MAX_PATH_POINTS ? pts : [...pts.slice(0, after + 1), p, ...pts.slice(after + 1)]);

/** Le point `i` retiré ; jamais moins de deux points. */
export const removePoint = (pts: Pt[], i: number): Pt[] => (pts.length <= 2 ? pts : pts.filter((_, j) => j !== i));

/** Le chemin prolongé par l'un de ses bouts ; inchangé à 200 points. */
export function extend(pts: Pt[], from: "start" | "end", p: Pt): Pt[] {
  if (pts.length >= MAX_PATH_POINTS) return pts;
  return from === "start" ? [p, ...pts] : [...pts, p];
}

/** Courbe passant par les points : Catmull-Rom centripète, une cubique par segment [contrôle 1, contrôle 2,
 *  arrivée] ; aux bouts, la tangente va vers le voisin (comme `curve_beziers`, shapes.rs). */
export function curveBeziers(pts: Pt[]): [Pt, Pt, Pt][] {
  const n = pts.length;
  return pts.slice(0, -1).map((p, i) => crSegment(i === 0 ? p : pts[i - 1], p, pts[i + 1], i + 2 < n ? pts[i + 2] : pts[i + 1]));
}

/** Courbe d'un chemin fermé : la même, qui revient du dernier point au premier sans angle (comme
 *  `closed_curve_beziers`, shapes.rs). */
export function closedCurveBeziers(pts: Pt[]): [Pt, Pt, Pt][] {
  const n = pts.length;
  return pts.map((p, i) => crSegment(pts[(i + n - 1) % n], p, pts[(i + 1) % n], pts[(i + 2) % n]));
}

/** Un segment de Catmull-Rom centripète de `p1` à `p2` (voisins `p0`, `p3`) en cubique. */
function crSegment(p0: Pt, p1: Pt, p2: Pt, p3: Pt): [Pt, Pt, Pt] {
  const EPS = 1e-12;
  const dist = (a: Pt, b: Pt) => Math.sqrt(Math.hypot(b.x - a.x, b.y - a.y));
  const third = (a: Pt, b: Pt): Pt => ({ x: a.x + (b.x - a.x) / 3, y: a.y + (b.y - a.y) / 3 });
  const [d1, d2, d3] = [dist(p0, p1), dist(p1, p2), dist(p2, p3)];
  if (d2 < EPS) return [p1, p2, p2];
  const ctl = (da: number, pa: Pt, pb: Pt, pc: Pt): Pt => {
    // Contrôle près de `pb`, entre `pa` (voisin de l'autre côté) et `pc` (l'autre bout du segment).
    const k = 3 * da * (da + d2);
    const a = 2 * da * da + 3 * da * d2 + d2 * d2;
    return { x: (da * da * pc.x - d2 * d2 * pa.x + a * pb.x) / k, y: (da * da * pc.y - d2 * d2 * pa.y + a * pb.y) / k };
  };
  return [d1 < EPS ? third(p1, p2) : ctl(d1, p0, p1, p2), d3 < EPS ? third(p2, p1) : ctl(d3, p3, p2, p1), p2];
}

/** Milieu de chaque segment (fermé : celui qui revient au premier point aussi) ; sur la courbe, son point à
 *  mi-parcours. */
export function segmentMids(pts: Pt[], curve: boolean, closed = false): Pt[] {
  const ends = closed ? pts.map((_, i) => pts[(i + 1) % pts.length]) : pts.slice(1);
  if (!curve) return ends.map((p, i) => ({ x: (pts[i].x + p.x) / 2, y: (pts[i].y + p.y) / 2 }));
  return (closed ? closedCurveBeziers(pts) : curveBeziers(pts)).map(([c1, c2, p], i) => ({ x: (pts[i].x + 3 * c1.x + 3 * c2.x + p.x) / 8, y: (pts[i].y + 3 * c1.y + 3 * c2.y + p.y) / 8 }));
}

const angle = (from: Pt, to: Pt) => (Math.atan2(to.y - from.y, to.x - from.x) * 180) / Math.PI;
const same = (a: Pt, b: Pt) => Math.hypot(a.x - b.x, a.y - b.y) < 1e-9;

/** Direction d'arrivée à chaque bout, vers l'extérieur (degrés) : le dernier segment, ou la tangente de la courbe.
 *  Points confondus : le premier point distinct ; tous confondus : 0 et 180. */
export function endDirections(pts: Pt[], curve: boolean): { start: number; end: number } {
  const toward = (tip: Pt, candidates: Pt[], fallback: number) => {
    const from = candidates.find((c) => !same(c, tip));
    return from ? angle(from, tip) : fallback;
  };
  const n = pts.length;
  if (!curve) return { start: toward(pts[0], pts.slice(1), 180), end: toward(pts[n - 1], pts.slice(0, -1).reverse(), 0) };
  const bz = curveBeziers(pts);
  return {
    start: toward(pts[0], [bz[0]?.[0], ...pts.slice(1)].filter(Boolean) as Pt[], 180),
    end: toward(pts[n - 1], [bz[bz.length - 1]?.[1], ...pts.slice(0, -1).reverse()].filter(Boolean) as Pt[], 0),
  };
}

/** Le point `i` tiré jusqu'à `to`, avec les règles d'un bout de trait (`dragEnd`) : son voisin précédent reste fixe
 *  (le suivant pour le premier), et il s'accroche aussi aux x et y des autres points du chemin. */
export function dragPoint(o: { pts: Pt[]; i: number; to: Pt; shift: boolean; lines: { xs: number[]; ys: number[] }; tolMm: number }): Pt[] {
  const fixed = o.pts[o.i === 0 ? 1 : o.i - 1];
  const moving = o.pts[o.i];
  const others = o.pts.filter((_, j) => j !== o.i);
  const lines = { xs: [...o.lines.xs, ...others.map((p) => p.x)], ys: [...o.lines.ys, ...others.map((p) => p.y)] };
  // Le segment voisin, vu comme un trait droit dont on tire la fin.
  const seg = frameFromEndpoints(fixed, same(fixed, moving) ? { x: fixed.x + 1, y: fixed.y } : moving, 1, false);
  const [, p] = lineEndpoints(dragEnd({ frame: seg, flipH: false, which: "end", to: o.to, shift: o.shift, lines, tolMm: o.tolMm }), false);
  return o.pts.map((q, j) => (j === o.i ? p : q));
}

/** Un chemin devenu forme ordinaire (`Path`) : son tracé dans la boîte 0..1000 étirée sur un cadre sans rotation,
 *  qui contient ses points (et les contrôles de sa courbe). La forme se redimensionne et se tourne comme les autres ;
 *  ses points ne se tirent plus. Un chemin tout plat garde 1 mm d'épaisseur de cadre. */
export function toShapePath(frame: Frame, s: ShapeProps, flipH: boolean, flipV: boolean): { frame: Frame; path: string } {
  const pts = pagePoints(frame, s, flipH, flipV);
  const curve = s.curve ? (s.closed ? closedCurveBeziers(pts) : curveBeziers(pts)) : null;
  const all = [...pts, ...(curve ?? []).flatMap(([c1, c2]) => [c1, c2])];
  const box = (vs: number[]) => {
    const [lo, hi] = [Math.min(...vs), Math.max(...vs)];
    return hi - lo >= 1 ? [lo, hi - lo] : [(lo + hi) / 2 - 0.5, 1];
  };
  const [x0, w] = box(all.map((p) => p.x));
  const [y0, h] = box(all.map((p) => p.y));
  const n = (v: number) => String(Math.round(v * 10) / 10 + 0);
  const at = (p: Pt) => `${n(((p.x - x0) / w) * 1000)} ${n(((p.y - y0) / h) * 1000)}`;
  const body = curve ? curve.map(([c1, c2, p]) => ` C ${at(c1)} ${at(c2)} ${at(p)}`).join("") : pts.slice(1).map((p) => ` L ${at(p)}`).join("");
  return { frame: { x_mm: x0, y_mm: y0, w_mm: w, h_mm: h, rotation_deg: 0 }, path: `M ${at(pts[0])}${body}${s.closed ? " Z" : ""}` };
}

/** Une forme tracée (`Path`) relue en chemin : ses points en mm de page (rotation et miroir compris), si elle est
 *  faite d'un seul morceau de segments droits (2 à 200 points), ou de cubiques qui sont, à 2 % de sa taille près, la
 *  courbe de Cahutin passant par leurs extrémités (3 points au moins). Sinon `null` : la forme ne se règle pas point
 *  par point sans changer d'allure. */
export function fromShapePath(frame: Frame, path: string, flipH: boolean, flipV: boolean): { pts: Pt[]; curve: boolean; closed: boolean } | null {
  const tokens = path.trim().split(/\s+/);
  const num = (i: number) => Number(tokens[i]);
  const cx = frame.x_mm + frame.w_mm / 2;
  const cy = frame.y_mm + frame.h_mm / 2;
  const a = (frame.rotation_deg * Math.PI) / 180;
  const page = (bx: number, by: number): Pt => {
    const dx = ((flipH ? 1000 - bx : bx) / 1000) * frame.w_mm - frame.w_mm / 2;
    const dy = ((flipV ? 1000 - by : by) / 1000) * frame.h_mm - frame.h_mm / 2;
    return { x: cx + dx * Math.cos(a) - dy * Math.sin(a), y: cy + dx * Math.sin(a) + dy * Math.cos(a) };
  };
  if (tokens[0] !== "M") return null;
  const pts: Pt[] = [page(num(1), num(2))];
  const ctrls: [Pt, Pt][] = [];
  let lines = 0;
  let closed = false;
  for (let i = 3; i < tokens.length; ) {
    if (closed) return null;
    const c = tokens[i];
    if (c === "L") {
      pts.push(page(num(i + 1), num(i + 2)));
      lines++;
      i += 3;
    } else if (c === "C") {
      ctrls.push([page(num(i + 1), num(i + 2)), page(num(i + 3), num(i + 4))]);
      pts.push(page(num(i + 5), num(i + 6)));
      i += 7;
    } else if (c === "Z") {
      closed = true;
      i += 1;
    } else return null;
  }
  if (pts.some((p) => !Number.isFinite(p.x) || !Number.isFinite(p.y)) || (lines && ctrls.length)) return null;
  const xs = pts.map((p) => p.x);
  const ys = pts.map((p) => p.y);
  const tol = 0.02 * Math.hypot(Math.max(...xs) - Math.min(...xs), Math.max(...ys) - Math.min(...ys));
  const near = (p: Pt, q: Pt) => Math.hypot(p.x - q.x, p.y - q.y) <= Math.max(tol, 1e-6);
  // Fermé : un dernier point revenu sur le premier n'en est pas un de plus.
  if (closed && pts.length > 1 && near(pts[0], pts[pts.length - 1])) pts.pop();
  if (pts.length > MAX_PATH_POINTS || pts.length < (closed || ctrls.length ? 3 : 2)) return null;
  if (!ctrls.length) return { pts, curve: false, closed };
  const ours = closed ? closedCurveBeziers(pts) : curveBeziers(pts);
  if (ours.length !== ctrls.length || !ours.every(([c1, c2], k) => near(c1, ctrls[k][0]) && near(c2, ctrls[k][1]))) return null;
  return { pts, curve: true, closed };
}
