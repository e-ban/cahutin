// Pointe d'une bulle de parole, ou petits ronds d'une bulle de pensée (nuage) : `ShapeProps.tail` est le point visé, en
// fractions du cadre (avant rotation et miroir), souvent hors du cadre. Le cadre est alors le corps de la bulle.
// Même calcul que `shapes.rs` (`tailed_path`), au caractère près (un test de chaque côté compare les tracés) :
// additions, multiplications et racines carrées seulement, arrondi `floor(v + 0.5)`.
import { SHAPE_PATHS, SHAPE_VIEWBOX } from "./shapePaths";
import type { Element, Frame, ShapeProps } from "./types";

type Pt = { x: number; y: number };

/** Quart de cercle en courbe de Bézier. */
const KAPPA = 0.5522847498;
/** Pointe hors de ces bornes (fractions du cadre) : document abîmé, pas de pointe. */
const TAIL_LIMIT = 20;
/** Corps d'une ancienne bulle sans pointe réglable : le haut de son cadre (le reste est la pointe fixe). */
export const LEGACY_BODY = 0.78;
/** Pointe fixe d'une ancienne bulle, en fractions de son cadre. */
export const LEGACY_TIP: [number, number] = [0.18, 1];

/** Bulle du tiroir : corps de 70 × 35 mm, pointe en bas à gauche. */
export const BUBBLE_TAIL: [number, number] = [9 / 70, 45 / 35];
export const BUBBLE_BODY_H = 35;
/** Bulle de pensée du tiroir : nuage de 70 × 40 mm, petits ronds vers le bas à gauche. */
export const THOUGHT_TAIL: [number, number] = [0.15, 1.45];
export const THOUGHT_BODY_H = 40;

/** La pointe d'une bulle ou d'un nuage, si elle est lisible. */
export function tailOf(s: ShapeProps): [number, number] | null {
  if (s.shape !== "Bubble" && s.shape !== "Cloud") return null;
  const t = s.tail;
  if (!Array.isArray(t) || t.length !== 2 || !t.every((v) => typeof v === "number" && Number.isFinite(v) && Math.abs(v) <= TAIL_LIMIT)) return null;
  return [t[0], t[1]];
}

export const hasTail = (e: Element): boolean => e.kind.type === "Shape" && tailOf(e.kind) !== null;

const n = (v: number) => String(Math.floor(v + 0.5));

/** Tracé en mm du cadre, écrit dans le carré de 1000 étiré au cadre (comme les autres formes). */
class PathOut {
  parts: string[] = [];
  constructor(
    private w: number,
    private h: number,
  ) {}
  p(q: Pt) {
    return `${n((q.x * SHAPE_VIEWBOX) / this.w)} ${n((q.y * SHAPE_VIEWBOX) / this.h)}`;
  }
  m(q: Pt) {
    this.parts.push(`M ${this.p(q)}`);
  }
  l(q: Pt) {
    this.parts.push(`L ${this.p(q)}`);
  }
  c(a: Pt, b: Pt, q: Pt) {
    this.parts.push(`C ${this.p(a)} ${this.p(b)} ${this.p(q)}`);
  }
  z() {
    this.parts.push("Z");
  }
  toString() {
    return this.parts.join(" ");
  }
}

const add = (a: Pt, b: Pt, k = 1): Pt => ({ x: a.x + b.x * k, y: a.y + b.y * k });
const sub = (a: Pt, b: Pt): Pt => ({ x: a.x - b.x, y: a.y - b.y });

/** Bulle de parole de `w` × `h` mm, coins de 9 mm au plus (ronds quelle que soit la forme du cadre), pointe vers
 *  `tail` : elle part du côté qui fait face au point visé, un peu ramenée vers son milieu, en deux courbes. Point
 *  visé dans le corps : pas de pointe. */
export function bubblePath(w: number, h: number, tail: [number, number]): string {
  const r = Math.min(9, w * 0.25, h * 0.25);
  const k = r * KAPPA;
  const t: Pt = { x: tail[0] * w, y: tail[1] * h };
  const d: Pt = { x: t.x - w / 2, y: t.y - h / 2 };
  const inside = t.x >= 0 && t.x <= w && t.y >= 0 && t.y <= h;
  // Côté de la pointe : 0 haut, 1 droite, 2 bas, 3 gauche.
  const side = inside ? -1 : Math.abs(d.y) * w >= Math.abs(d.x) * h ? (d.y > 0 ? 2 : 0) : d.x > 0 ? 1 : 3;
  // Chaque côté : début et fin de sa partie droite, sens de parcours, normale vers l'extérieur.
  const sides = [
    { a: { x: r, y: 0 }, b: { x: w - r, y: 0 }, e: { x: 1, y: 0 }, nrm: { x: 0, y: -1 } },
    { a: { x: w, y: r }, b: { x: w, y: h - r }, e: { x: 0, y: 1 }, nrm: { x: 1, y: 0 } },
    { a: { x: w - r, y: h }, b: { x: r, y: h }, e: { x: -1, y: 0 }, nrm: { x: 0, y: 1 } },
    { a: { x: 0, y: h - r }, b: { x: 0, y: r }, e: { x: 0, y: -1 }, nrm: { x: -1, y: 0 } },
  ];
  const corners: [Pt, Pt, Pt][] = [
    [{ x: w - r + k, y: 0 }, { x: w, y: r - k }, { x: w, y: r }],
    [{ x: w, y: h - r + k }, { x: w - r + k, y: h }, { x: w - r, y: h }],
    [{ x: r - k, y: h }, { x: 0, y: h - r + k }, { x: 0, y: h - r }],
    [{ x: 0, y: r - k }, { x: r - k, y: 0 }, { x: r, y: 0 }],
  ];
  const out = new PathOut(w, h);
  out.m(sides[0].a);
  for (let i = 0; i < 4; i++) {
    const s = sides[i];
    if (i === side) tailOn(out, s, i, w, h, r, t, d);
    out.l(s.b);
    out.c(...corners[i]);
  }
  out.z();
  return out.toString();
}

/** La pointe insérée dans la partie droite d'un côté (rien si le côté est trop court). */
function tailOn(out: PathOut, s: { e: Pt; nrm: Pt }, side: number, w: number, h: number, r: number, t: Pt, d: Pt) {
  const vertical = side === 0 || side === 2;
  const len = vertical ? w : h;
  const depth = side === 0 ? -t.y : side === 1 ? t.x - w : side === 2 ? t.y - h : -t.x;
  // Base de 11 mm, plus large pour une longue pointe (sinon une aiguille), jamais plus que le côté.
  const bw = Math.min(Math.max(11, depth * 0.5), len * 0.35, len - 2 * r);
  if (bw <= 0.5) return;
  // Où la droite du centre au point visé croise ce côté, ramené d'un quart vers le milieu du côté.
  const cross = vertical ? w / 2 + (d.x * (h / 2)) / Math.abs(d.y) : h / 2 + (d.y * (w / 2)) / Math.abs(d.x);
  const m0 = cross + (len / 2 - cross) * 0.25;
  const m = Math.min(len - r - bw / 2, Math.max(r + bw / 2, m0));
  const mid: Pt = vertical ? { x: m, y: side === 0 ? 0 : h } : { x: side === 3 ? 0 : w, y: m };
  const b1 = add(mid, s.e, -bw / 2);
  const b2 = add(mid, s.e, bw / 2);
  const a = add(b1, s.nrm, depth * 0.5);
  const b = add(b2, s.nrm, depth * 0.35);
  out.l(b1);
  out.c(a, add(t, sub(a, t), 0.5), t);
  out.c(add(t, sub(b, t), 0.4), b, b2);
}

/** Bulle de pensée : le nuage, et trois ronds de plus en plus petits de son bord vers le point visé (le dernier
 *  centré dessus). Point visé dans le cadre, ou trop près : le nuage seul. */
export function thoughtPath(w: number, h: number, tail: [number, number]): string {
  const cloud = SHAPE_PATHS.Cloud!;
  const t: Pt = { x: tail[0] * w, y: tail[1] * h };
  if (t.x >= 0 && t.x <= w && t.y >= 0 && t.y <= h) return cloud;
  const d: Pt = { x: t.x - w / 2, y: t.y - h / 2 };
  const ax = Math.abs(d.x);
  const ay = Math.abs(d.y);
  // Sortie du cadre sur la droite du centre au point visé.
  const f = ax * h >= ay * w ? w / 2 / ax : h / 2 / ay;
  const e: Pt = add({ x: w / 2, y: h / 2 }, d, f);
  const full = Math.sqrt(d.x * d.x + d.y * d.y);
  const len = full * (1 - f);
  const u: Pt = { x: d.x / full, y: d.y / full };
  const big = Math.min(0.12 * Math.min(w, h), len / 5.25);
  if (big < 0.4) return cloud;
  const s1 = 1.6 * big;
  const s2 = (2.6 * big + len - 0.45 * big) / 2;
  const out = new PathOut(w, h);
  for (const [s, rad] of [[s1, big], [s2, 0.7 * big], [len, 0.45 * big]]) {
    const c = add(e, u, s);
    const kk = rad * KAPPA;
    out.m({ x: c.x, y: c.y - rad });
    out.c({ x: c.x + kk, y: c.y - rad }, { x: c.x + rad, y: c.y - kk }, { x: c.x + rad, y: c.y });
    out.c({ x: c.x + rad, y: c.y + kk }, { x: c.x + kk, y: c.y + rad }, { x: c.x, y: c.y + rad });
    out.c({ x: c.x - kk, y: c.y + rad }, { x: c.x - rad, y: c.y + kk }, { x: c.x - rad, y: c.y });
    out.c({ x: c.x - rad, y: c.y - kk }, { x: c.x - kk, y: c.y - rad }, { x: c.x, y: c.y - rad });
    out.z();
  }
  return `${cloud} ${out}`;
}

/** Le tracé d'une forme dessinée (dans le carré de 1000 étiré au cadre) : bulle ou nuage avec pointe, sinon le tracé
 *  fixe de sa sorte. */
export function shapePath(frame: Frame, s: ShapeProps): string | undefined {
  const tail = tailOf(s);
  if (tail && s.shape === "Bubble") return bubblePath(frame.w_mm, frame.h_mm, tail);
  if (tail && s.shape === "Cloud") return thoughtPath(frame.w_mm, frame.h_mm, tail);
  return SHAPE_PATHS[s.shape];
}

/** La boîte du dessin, en fractions du cadre : le cadre, agrandi jusqu'à la pointe (et ses ronds) s'il y en a une. */
export function tailBox(s: ShapeProps): { x0: number; y0: number; x1: number; y1: number } {
  const t = tailOf(s);
  if (!t) return { x0: 0, y0: 0, x1: 1, y1: 1 };
  const pad = 0.08;
  return { x0: Math.min(0, t[0] - pad), y0: Math.min(0, t[1] - pad), x1: Math.max(1, t[0] + pad), y1: Math.max(1, t[1] + pad) };
}

/** Un point en fractions du cadre, sur la page (miroir puis rotation, comme l'écran). */
export function tailPagePoint(frame: Frame, f: [number, number], flipH: boolean, flipV: boolean): Pt {
  const a = (frame.rotation_deg * Math.PI) / 180;
  const dx = (flipH ? 1 - f[0] : f[0]) * frame.w_mm - frame.w_mm / 2;
  const dy = (flipV ? 1 - f[1] : f[1]) * frame.h_mm - frame.h_mm / 2;
  return { x: frame.x_mm + frame.w_mm / 2 + dx * Math.cos(a) - dy * Math.sin(a), y: frame.y_mm + frame.h_mm / 2 + dx * Math.sin(a) + dy * Math.cos(a) };
}

/** Le point de la page `p` en fractions du cadre (l'inverse de `tailPagePoint`), borné. */
export function tipFraction(frame: Frame, p: Pt, flipH: boolean, flipV: boolean): [number, number] {
  const a = (-frame.rotation_deg * Math.PI) / 180;
  const [x, y] = [p.x - frame.x_mm - frame.w_mm / 2, p.y - frame.y_mm - frame.h_mm / 2];
  const fx = (x * Math.cos(a) - y * Math.sin(a) + frame.w_mm / 2) / frame.w_mm;
  const fy = (x * Math.sin(a) + y * Math.cos(a) + frame.h_mm / 2) / frame.h_mm;
  const clamp = (v: number) => Math.max(-TAIL_LIMIT / 2, Math.min(TAIL_LIMIT / 2, v));
  return [clamp(flipH ? 1 - fx : fx), clamp(flipV ? 1 - fy : fy)];
}

/** Une ancienne bulle (pointe fixe dans son cadre) devenue bulle à pointe réglable : le cadre se réduit au corps, la
 *  pointe va où elle était ; rien ne bouge sur la page, sinon l'arrondi des coins. */
export function legacyBubbleToTailed(e: Element): Element {
  if (e.kind.type !== "Shape") return e;
  const f = e.frame;
  const fh = !!e.flip_h;
  const fv = !!e.flip_v;
  // Centre du corps sur la page, puis cadre du corps autour de lui.
  const c = tailPagePoint(f, [0.5, LEGACY_BODY / 2], fh, fv);
  const h = f.h_mm * LEGACY_BODY;
  const frame: Frame = { ...f, h_mm: h, x_mm: c.x - f.w_mm / 2, y_mm: c.y - h / 2 };
  const tip = tipFraction(frame, tailPagePoint(f, LEGACY_TIP, fh, fv), fh, fv);
  return { ...e, frame, kind: { ...e.kind, tail: tip } };
}
