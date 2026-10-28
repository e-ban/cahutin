import { pxToMm } from "./units";
import type { Frame, PageSize } from "./types";

/** Pas de la grille aimantée. */
export const SNAP_MM = 5;
/** Marges de la page sur lesquelles s'aligner (comme la grille des modèles). */
export const GUIDE_MARGIN_MM = 10;

export interface RectPx {
  left: number;
  top: number;
  width: number;
  height: number;
  rotation: number;
}

export function rectPxToFrame(r: RectPx, zoom: number): Frame {
  return {
    x_mm: pxToMm(r.left, zoom),
    y_mm: pxToMm(r.top, zoom),
    w_mm: pxToMm(r.width, zoom),
    h_mm: pxToMm(r.height, zoom),
    rotation_deg: Math.round(r.rotation * 10) / 10,
  };
}

type Box = Pick<Frame, "x_mm" | "y_mm" | "w_mm" | "h_mm">;

/** Lignes d'accrochage (mm) : bords, marges et milieux de la page, puis bords et centres des autres blocs. */
export function snapLines(size: PageSize, others: Box[]): { xs: number[]; ys: number[] } {
  const { width_mm: w, height_mm: h } = size;
  const m = GUIDE_MARGIN_MM;
  return {
    xs: [0, m, w / 2, w - m, w, ...others.flatMap((b) => [b.x_mm, b.x_mm + b.w_mm / 2, b.x_mm + b.w_mm])],
    ys: [0, m, h / 2, h - m, h, ...others.flatMap((b) => [b.y_mm, b.y_mm + b.h_mm / 2, b.y_mm + b.h_mm])],
  };
}

// Moveable mesure les blocs au pixel entier (offsetLeft) : un bloc accroché s'arrête jusqu'à un pixel
// à côté de sa ligne. À la fin du geste, on le pose exactement sur la ligne (ou le pas de grille) qu'il
// touche à `tol` près.

/** Décalage qui amène la plus proche de `edges` sur une ligne ou un pas de grille, 0 si aucune n'est à `tol` près. */
export function settle(edges: number[], lines: number[], tol: number): number {
  let best = 0;
  let dist = tol + 1e-9;
  for (const e of edges) {
    for (const l of [...lines, Math.round(e / SNAP_MM) * SNAP_MM]) {
      if (Math.abs(l - e) < dist) [best, dist] = [l - e, Math.abs(l - e)];
    }
  }
  return best;
}

/** Bloc déplacé : bords et centre posés sur les lignes. Un bloc tourné reste tel quel. */
export function settleMove(f: Frame, lines: { xs: number[]; ys: number[] }, tol: number): Frame {
  if (f.rotation_deg !== 0) return f;
  const dx = settle([f.x_mm, f.x_mm + f.w_mm / 2, f.x_mm + f.w_mm], lines.xs, tol);
  const dy = settle([f.y_mm, f.y_mm + f.h_mm / 2, f.y_mm + f.h_mm], lines.ys, tol);
  return { ...f, x_mm: f.x_mm + dx, y_mm: f.y_mm + dy };
}

/** Bloc agrandi : seuls les bords qui ont bougé sont posés sur les lignes. */
export function settleResize(before: Frame, f: Frame, lines: { xs: number[]; ys: number[] }, tol: number): Frame {
  if (f.rotation_deg !== 0) return f;
  let { x_mm: x, y_mm: y, w_mm: w, h_mm: h } = f;
  if (x !== before.x_mm) {
    const d = settle([x], lines.xs, tol);
    [x, w] = [x + d, w - d];
  }
  if (x + w !== before.x_mm + before.w_mm) w += settle([x + w], lines.xs, tol);
  if (y !== before.y_mm) {
    const d = settle([y], lines.ys, tol);
    [y, h] = [y + d, h - d];
  }
  if (y + h !== before.y_mm + before.h_mm) h += settle([y + h], lines.ys, tol);
  return { ...f, x_mm: x, y_mm: y, w_mm: w, h_mm: h };
}

/** Groupe agrandi depuis un coin (`dir` : coin tiré, comme Moveable) : facteur qui pose sur une ligne le bord mobile le plus proche. */
export function settleScale(box: Box, factor: number, dir: number[], lines: { xs: number[]; ys: number[] }, tol: number): number {
  const ox = dir[0] < 0 ? box.x_mm + box.w_mm : box.x_mm;
  const oy = dir[1] < 0 ? box.y_mm + box.h_mm : box.y_mm;
  const ex = ox + (dir[0] < 0 ? -1 : 1) * box.w_mm * factor;
  const ey = oy + (dir[1] < 0 ? -1 : 1) * box.h_mm * factor;
  const sx = settle([ex], lines.xs, tol);
  const sy = settle([ey], lines.ys, tol);
  if (sx !== 0 && (sy === 0 || Math.abs(sx) <= Math.abs(sy))) return Math.abs(ex + sx - ox) / box.w_mm;
  if (sy !== 0) return Math.abs(ey + sy - oy) / box.h_mm;
  return factor;
}

export type ArrowKey = "ArrowUp" | "ArrowDown" | "ArrowLeft" | "ArrowRight";

export function nudge(frame: Frame, key: ArrowKey, shift: boolean): Frame {
  const step = shift ? 5 : 1;
  const [dx, dy] = { ArrowUp: [0, -step], ArrowDown: [0, step], ArrowLeft: [-step, 0], ArrowRight: [step, 0] }[key];
  return { ...frame, x_mm: frame.x_mm + dx, y_mm: frame.y_mm + dy };
}
