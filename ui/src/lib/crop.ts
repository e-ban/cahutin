import type { AssetMeta, Crop, Frame } from "./types";

export interface SourceRect {
  x: number;
  y: number;
  w: number;
  h: number;
}

/** Même calcul que `carnet_core::crop::source_rect` (vecteurs partagés dans fixtures/crop_vectors.json). */
export function sourceRect(imgW: number, imgH: number, frameW: number, frameH: number, crop: Crop): SourceRect {
  const s0 = Math.max(frameW / imgW, frameH / imgH);
  const w = frameW / (s0 * crop.scale);
  const h = frameH / (s0 * crop.scale);
  const clamp = (v: number, lo: number, hi: number) => Math.min(Math.max(v, lo), hi);
  return {
    x: clamp(crop.center_x * imgW - w / 2, 0, imgW - w),
    y: clamp(crop.center_y * imgH - h / 2, 0, imgH - h),
    w,
    h,
  };
}

/** Taille et position (en mm, relatives au cadre) de l'image entière pour n'en montrer que la partie recadrée. */
export function cropToCss(meta: AssetMeta, frame: Frame, crop: Crop) {
  const r = sourceRect(meta.width_px, meta.height_px, frame.w_mm, frame.h_mm, crop);
  const k = frame.w_mm / r.w;
  return { widthMm: meta.width_px * k, heightMm: meta.height_px * k, leftMm: -r.x * k, topMm: -r.y * k };
}
