import { PX_PER_MM } from "./units";
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

const MAX_SCALE = 5;
const WHEEL_STEP = 1.1;

/** Glisser la photo dans son cadre : la partie visible se déplace en sens inverse, sans sortir de l'image. */
export function dragCrop(crop: Crop, dxPx: number, dyPx: number, meta: AssetMeta, frame: Frame, zoom: number): Crop {
  const r = sourceRect(meta.width_px, meta.height_px, frame.w_mm, frame.h_mm, crop);
  const mmPerImgPx = frame.w_mm / r.w;
  const toImg = (px: number) => px / (PX_PER_MM * zoom) / mmPerImgPx;
  const clampCenter = (c: number, visible: number, total: number) => {
    const half = visible / 2 / total;
    return half >= 0.5 ? 0.5 : Math.min(Math.max(c, half), 1 - half);
  };
  // On part du centre réellement affiché (le recadrage peut déjà être bloqué contre un bord).
  const cx = (r.x + r.w / 2) / meta.width_px;
  const cy = (r.y + r.h / 2) / meta.height_px;
  return {
    ...crop,
    center_x: clampCenter(cx - toImg(dxPx) / meta.width_px, r.w, meta.width_px),
    center_y: clampCenter(cy - toImg(dyPx) / meta.height_px, r.h, meta.height_px),
  };
}

/** Molette : zoom dans la photo, par pas de 10 %, entre 1 (remplir le cadre) et 5. */
export function wheelCrop(crop: Crop, deltaY: number): Crop {
  const next = deltaY < 0 ? crop.scale * WHEEL_STEP : crop.scale / WHEEL_STEP;
  return { ...crop, scale: Math.min(Math.max(next, 1), MAX_SCALE) };
}
