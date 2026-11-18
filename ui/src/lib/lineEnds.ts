// Géométrie d'un trait ou d'une flèche : ses deux bouts, tirés à la souris, et ses terminaisons. Le trait reste
// enregistré comme un cadre tourné (segment du milieu du bord gauche au milieu du bord droit), comme à l'export
// (crates/carnet-core/src/odg/content.rs, `line_ends`).
import { SNAP_MM } from "./geometry";
import { HEAD_FACTORS } from "./shapePaths";
import type { Frame, LineEnds, ShapeKind, ShapeProps } from "./types";

/** Un point de la page, en mm. */
export type Pt = { x: number; y: number };

/** Plus court trait possible : un bout tiré sur l'autre le garde à cette longueur. */
export const MIN_LINE_MM = 3;
/** Un trait à moins de ce nombre de degrés de l'horizontale ou de la verticale s'y cale. */
export const ANGLE_MAGNET_DEG = 3;

/** Terminaisons d'un trait qui n'en précise pas (les mêmes que `shapes::default_ends`). */
export function defaultEnds(kind: ShapeKind): LineEnds {
  return { start: "None", end: kind === "Arrow" ? "Arrow" : "None", size: "Medium" };
}

export const endsOf = (s: ShapeProps): LineEnds => s.ends ?? defaultEnds(s.shape);

/** Les terminaisons à enregistrer : aucune si ce sont celles du genre (le fichier ne change pas sans raison). */
export function normalizeEnds(kind: ShapeKind, ends: LineEnds): LineEnds | null {
  const d = defaultEnds(kind);
  return ends.start === d.start && ends.end === d.end && ends.size === d.size ? null : ends;
}

/** Largeur d'une pointe de flèche, en mm : quatre fois le trait, 4 mm au moins, fois le facteur de la taille. */
export function headWidthMm(s: ShapeProps): number {
  return Math.max(4, (s.stroke?.width_mm ?? 1) * 4) * HEAD_FACTORS[endsOf(s).size];
}

/** Bouts visibles du trait : [début, fin]. Le miroir les échange. */
export function lineEndpoints(f: Frame, flipH: boolean): [Pt, Pt] {
  const cx = f.x_mm + f.w_mm / 2;
  const cy = f.y_mm + f.h_mm / 2;
  const a = (f.rotation_deg * Math.PI) / 180;
  const at = (dx: number): Pt => ({ x: cx + dx * Math.cos(a), y: cy + dx * Math.sin(a) });
  const half = f.w_mm / 2;
  return flipH ? [at(half), at(-half)] : [at(-half), at(half)];
}

/** Cadre d'un trait qui va de `start` à `end` (l'inverse exact de `lineEndpoints`). */
export function frameFromEndpoints(start: Pt, end: Pt, hMm: number, flipH: boolean): Frame {
  const [from, to] = flipH ? [end, start] : [start, end];
  const w = Math.hypot(to.x - from.x, to.y - from.y);
  const cx = (start.x + end.x) / 2;
  const cy = (start.y + end.y) / 2;
  return { x_mm: cx - w / 2, y_mm: cy - hMm / 2, w_mm: w, h_mm: hMm, rotation_deg: (Math.atan2(to.y - from.y, to.x - from.x) * 180) / Math.PI };
}

/** Le point à `len` de `from`, dans la direction `deg`. */
const polar = (from: Pt, len: number, deg: number): Pt => ({ x: from.x + len * Math.cos((deg * Math.PI) / 180), y: from.y + len * Math.sin((deg * Math.PI) / 180) });

/** La valeur de `lines` la plus proche de `v` à `tol` près, sinon `null`. */
function nearest(v: number, lines: number[], tol: number): number | null {
  let best: number | null = null;
  for (const l of lines) if (Math.abs(l - v) <= tol && (best === null || Math.abs(l - v) < Math.abs(best - v))) best = l;
  return best;
}

/**
 * Cadre d'un trait dont un bout est tiré jusqu'à `to`. Dans l'ordre : avec Maj, le trait ne prend que des multiples
 * de 45° ; sinon le bout s'accroche aux lignes (bords, centres, marges) ; sinon le trait se cale à l'horizontale ou
 * à la verticale ; sinon le bout s'accroche à la grille. Le trait garde au moins `MIN_LINE_MM`.
 */
export function dragEnd(o: { frame: Frame; flipH: boolean; which: "start" | "end"; to: Pt; shift: boolean; lines: { xs: number[]; ys: number[] }; tolMm: number }): Frame {
  const [s, e] = lineEndpoints(o.frame, o.flipH);
  const fixed = o.which === "start" ? e : s;
  const moving = o.which === "start" ? s : e;
  const dirDeg = (p: Pt) => (Math.atan2(p.y - fixed.y, p.x - fixed.x) * 180) / Math.PI;
  const len = (p: Pt) => Math.hypot(p.x - fixed.x, p.y - fixed.y);
  let p = o.to;
  if (o.shift) {
    p = polar(fixed, len(p), Math.round(dirDeg(p) / 45) * 45);
  } else {
    const x = nearest(p.x, o.lines.xs, o.tolMm);
    const y = nearest(p.y, o.lines.ys, o.tolMm);
    if (x !== null || y !== null) {
      p = { x: x ?? p.x, y: y ?? p.y };
    } else {
      const deg = dirDeg(p);
      const square = Math.round(deg / 90) * 90;
      if (Math.abs(deg - square) <= ANGLE_MAGNET_DEG) p = polar(fixed, len(p), square);
      else p = { x: nearest(p.x, [Math.round(p.x / SNAP_MM) * SNAP_MM], o.tolMm) ?? p.x, y: nearest(p.y, [Math.round(p.y / SNAP_MM) * SNAP_MM], o.tolMm) ?? p.y };
    }
  }
  if (len(p) < MIN_LINE_MM) {
    // Trop court : dans la direction du geste, ou celle d'avant si le bout est tombé sur l'autre.
    const deg = len(p) > 1e-9 ? dirDeg(p) : dirDeg(moving);
    p = polar(fixed, MIN_LINE_MM, deg);
  }
  const [start, end] = o.which === "start" ? [p, e] : [s, p];
  return frameFromEndpoints(start, end, o.frame.h_mm, o.flipH);
}

/** Cadre assez haut pour la plus grande terminaison : la zone cliquable et l'avis « près du bord » la couvrent. */
export function withHitHeight(f: Frame, s: ShapeProps): Frame {
  const ends = endsOf(s);
  if (ends.start === "None" && ends.end === "None") return f;
  const h = headWidthMm(s);
  if (f.h_mm >= h) return f;
  const cy = f.y_mm + f.h_mm / 2;
  return { ...f, y_mm: cy - h / 2, h_mm: h };
}
