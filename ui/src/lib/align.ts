// Alignement et répartition de plusieurs éléments, sur leur boîte englobante (cadres non tournés) ; un bloc seul
// s'aligne sur la page. `arrangeUnits` donne la boîte qu'un élément tourné couvre vraiment.
import { GUIDE_MARGIN_MM } from "./geometry";
import type { Frame, PageSize } from "./types";

export type AlignMode = "left" | "hcenter" | "right" | "top" | "vmiddle" | "bottom";

const round = (v: number) => Math.round(v * 10) / 10;

type Area = { left: number; right: number; top: number; bottom: number };

/** Où aligner un bloc seul : contre les marges de 10 mm (les lignes d'aimantation, la grille des modèles, et
 *  hors de la bande que l'imprimante n'atteint pas) ; centrer reste au centre de la page. */
export function pageArea(size: PageSize): Area {
  const m = GUIDE_MARGIN_MM;
  return { left: m, right: size.width_mm - m, top: m, bottom: size.height_mm - m };
}

/** Aligne les cadres entre eux, ou dans `within` (la page) s'il est donné. */
export function alignFrames(frames: Frame[], mode: AlignMode, within?: Area): Frame[] {
  if (frames.length === 0) return frames;
  const left = within?.left ?? Math.min(...frames.map((f) => f.x_mm));
  const right = within?.right ?? Math.max(...frames.map((f) => f.x_mm + f.w_mm));
  const top = within?.top ?? Math.min(...frames.map((f) => f.y_mm));
  const bottom = within?.bottom ?? Math.max(...frames.map((f) => f.y_mm + f.h_mm));
  return frames.map((f) => {
    switch (mode) {
      case "left":
        return { ...f, x_mm: left };
      case "hcenter":
        return { ...f, x_mm: round((left + right) / 2 - f.w_mm / 2) };
      case "right":
        return { ...f, x_mm: round(right - f.w_mm) };
      case "top":
        return { ...f, y_mm: top };
      case "vmiddle":
        return { ...f, y_mm: round((top + bottom) / 2 - f.h_mm / 2) };
      case "bottom":
        return { ...f, y_mm: round(bottom - f.h_mm) };
    }
  });
}

/** Espaces égaux entre les éléments ; les deux extrêmes ne bougent pas. Moins de 3 éléments : inchangé. */
export function distributeFrames(frames: Frame[], axis: "h" | "v"): Frame[] {
  if (frames.length < 3) return frames;
  const pos = (f: Frame) => (axis === "h" ? f.x_mm : f.y_mm);
  const size = (f: Frame) => (axis === "h" ? f.w_mm : f.h_mm);
  const order = frames.map((_, i) => i).sort((a, b) => pos(frames[a]) - pos(frames[b]));
  const first = frames[order[0]];
  const last = frames[order[order.length - 1]];
  const total = order.reduce((s, i) => s + size(frames[i]), 0);
  const gap = (pos(last) + size(last) - pos(first) - total) / (frames.length - 1);
  const out = [...frames];
  let cursor = pos(first);
  for (const i of order) {
    const f = frames[i];
    out[i] = axis === "h" ? { ...f, x_mm: round(cursor) } : { ...f, y_mm: round(cursor) };
    cursor += size(f) + gap;
  }
  out[order[order.length - 1]] = last;
  return out;
}
