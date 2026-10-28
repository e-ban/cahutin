// Décors de page : mention de licence OpenMoji (même cadre que l'export ODG), cadres et frises.
import type { Frame, PageSize } from "./types";

/** Mention en bas des pages qui contiennent une image OpenMoji (carnet_core::odg::content::credit_frame). */
export const CREDIT_SIZE_PT = 6;
export const CREDIT_MARGIN_MM = 8;

export function creditFrame(size: PageSize): Frame {
  return { x_mm: CREDIT_MARGIN_MM, y_mm: size.height_mm - CREDIT_MARGIN_MM, w_mm: size.width_mm - 2 * CREDIT_MARGIN_MM, h_mm: 5, rotation_deg: 0 };
}
