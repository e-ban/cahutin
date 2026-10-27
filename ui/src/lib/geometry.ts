import { pxToMm } from "./units";
import type { Frame } from "./types";

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

export type ArrowKey = "ArrowUp" | "ArrowDown" | "ArrowLeft" | "ArrowRight";

export function nudge(frame: Frame, key: ArrowKey, shift: boolean): Frame {
  const step = shift ? 5 : 1;
  const [dx, dy] = { ArrowUp: [0, -step], ArrowDown: [0, step], ArrowLeft: [-step, 0], ArrowRight: [step, 0] }[key];
  return { ...frame, x_mm: frame.x_mm + dx, y_mm: frame.y_mm + dy };
}
