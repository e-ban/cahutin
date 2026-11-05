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

/** Glisser la photo dans son cadre : la partie visible se déplace en sens inverse, sans sortir de l'image. Le
 *  glisser à l'écran est ramené dans le cadre : une photo tournée (ou en miroir) suit la souris. */
export function dragCrop(crop: Crop, screenDx: number, screenDy: number, meta: AssetMeta, frame: Frame, zoom: number, mirrored = false, flippedV = false): Crop {
  const t = (-frame.rotation_deg * Math.PI) / 180;
  const dxPx = (screenDx * Math.cos(t) - screenDy * Math.sin(t)) * (mirrored ? -1 : 1);
  const dyPx = (screenDx * Math.sin(t) + screenDy * Math.cos(t)) * (flippedV ? -1 : 1);
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

/** Plus petit côté, en mm, d'une photo rognée : en dessous, on ne la saisit plus. */
export const MIN_TRIMMED_MM = 5;

/** Ce que chaque bord du cadre cache de la photo, en mm, du côté où on le voit. */
export interface Trims {
  left: number;
  top: number;
  right: number;
  bottom: number;
}

/**
 * Rognage d'une photo : ce que chaque bord de son cadre cache, tel qu'on le voit à l'écran. `flipH` :
 * photo retournée de gauche à droite (miroir), `flipV` : de haut en bas.
 */
export function photoTrims(meta: AssetMeta, frame: Frame, crop: Crop, flipH = false, flipV = false): Trims {
  const c = cropToCss(meta, frame, crop);
  const left = -c.leftMm;
  const top = -c.topMm;
  const right = c.widthMm - frame.w_mm - left;
  const bottom = c.heightMm - frame.h_mm - top;
  return { left: flipH ? right : left, right: flipH ? left : right, top: flipV ? bottom : top, bottom: flipV ? top : bottom };
}

/**
 * Rogne la photo : chaque bord du cadre va là où il cache `trims` de la photo, et la photo ne bouge pas
 * sur la page (même tournée ou en miroir). Un bord ne dépasse pas la photo ; le cadre garde au moins
 * `MIN_TRIMMED_MM` de côté, en retenant le bord que l'on vient de déplacer.
 */
export function withTrims(meta: AssetMeta, frame: Frame, crop: Crop, trims: Trims, flipH = false, flipV = false): { frame: Frame; crop: Crop } {
  const c = cropToCss(meta, frame, crop);
  const old = photoTrims(meta, frame, crop, flipH, flipV);
  const fit = (a: number, b: number, oldA: number, total: number): [number, number] => {
    a = Math.max(0, a);
    b = Math.max(0, b);
    if (a + b > total - MIN_TRIMMED_MM) {
      if (a !== oldA) a = Math.max(0, total - MIN_TRIMMED_MM - b);
      else b = Math.max(0, total - MIN_TRIMMED_MM - a);
    }
    return [a, b];
  };
  const [left, right] = fit(trims.left, trims.right, old.left, c.widthMm);
  const [top, bottom] = fit(trims.top, trims.bottom, old.top, c.heightMm);
  // Dans la photo elle-même (sans miroir).
  const li = flipH ? right : left;
  const ti = flipV ? bottom : top;
  const w = c.widthMm - left - right;
  const h = c.heightMm - top - bottom;
  // La photo garde sa taille à l'écran : mêmes millimètres par pixel.
  const mmPerPx = c.widthMm / meta.width_px;
  const cover = Math.max(w / meta.width_px, h / meta.height_px);
  // Le centre du cadre se déplace de la moitié de ce que chaque bord a bougé, dans le sens du cadre tourné.
  const dx = (left - old.left - (right - old.right)) / 2;
  const dy = (top - old.top - (bottom - old.bottom)) / 2;
  const t = (frame.rotation_deg * Math.PI) / 180;
  const cx = frame.x_mm + frame.w_mm / 2 + dx * Math.cos(t) - dy * Math.sin(t);
  const cy = frame.y_mm + frame.h_mm / 2 + dx * Math.sin(t) + dy * Math.cos(t);
  return {
    frame: { ...frame, x_mm: cx - w / 2, y_mm: cy - h / 2, w_mm: w, h_mm: h },
    crop: { scale: Math.max(1, mmPerPx / cover), center_x: (li + w / 2) / c.widthMm, center_y: (ti + h / 2) / c.heightMm },
  };
}

/**
 * Photo entière : plus rien n'est rogné, la photo garde sa taille et sa place, le cadre s'élargit jusqu'à
 * ses bords. Plus grande que `maxW` × `maxH` (la page), elle est réduite pour y tenir, autour de son centre.
 */
export function wholePhoto(meta: AssetMeta, frame: Frame, crop: Crop, flipH: boolean, flipV: boolean, maxW: number, maxH: number): { frame: Frame; crop: Crop } {
  const r = withTrims(meta, frame, crop, { left: 0, top: 0, right: 0, bottom: 0 }, flipH, flipV);
  const f = Math.min(1, maxW / r.frame.w_mm, maxH / r.frame.h_mm);
  if (f === 1) return r;
  const { x_mm, y_mm, w_mm, h_mm } = r.frame;
  return { ...r, frame: { ...r.frame, x_mm: x_mm + (w_mm * (1 - f)) / 2, y_mm: y_mm + (h_mm * (1 - f)) / 2, w_mm: w_mm * f, h_mm: h_mm * f } };
}

/**
 * Position de la partie visible dans la photo, de 0 (à gauche, en haut, à l'écran) à 1 ; null quand la
 * photo n'a rien de caché dans ce sens.
 */
export function cropPosition(meta: AssetMeta, frame: Frame, crop: Crop, flipH = false, flipV = false): { x: number | null; y: number | null } {
  const r = sourceRect(meta.width_px, meta.height_px, frame.w_mm, frame.h_mm, crop);
  const along = (start: number, visible: number, total: number, flipped: boolean) => {
    const slack = total - visible;
    if (slack < 1e-6) return null;
    const p = start / slack;
    return flipped ? 1 - p : p;
  };
  return { x: along(r.x, r.w, meta.width_px, flipH), y: along(r.y, r.h, meta.height_px, flipV) };
}

/** Place la partie visible à cette position (voir `cropPosition`), le cadre et le zoom restant les mêmes. */
export function withPosition(meta: AssetMeta, frame: Frame, crop: Crop, pos: { x?: number; y?: number }, flipH = false, flipV = false): Crop {
  const r = sourceRect(meta.width_px, meta.height_px, frame.w_mm, frame.h_mm, crop);
  const center = (p: number, flipped: boolean, visible: number, total: number) => (((flipped ? 1 - p : p) * (total - visible) + visible / 2) / total);
  return {
    ...crop,
    ...(pos.x === undefined ? {} : { center_x: center(pos.x, flipH, r.w, meta.width_px) }),
    ...(pos.y === undefined ? {} : { center_y: center(pos.y, flipV, r.h, meta.height_px) }),
  };
}

/** Cadre arrondi à 0,1 mm, la précision des autres réglages : l'écart sur la photo ne se voit pas. */
export function roundedFrame(f: Frame): Frame {
  const r = (v: number) => Math.round(v * 10) / 10;
  return { ...f, x_mm: r(f.x_mm), y_mm: r(f.y_mm), w_mm: r(f.w_mm), h_mm: r(f.h_mm) };
}
