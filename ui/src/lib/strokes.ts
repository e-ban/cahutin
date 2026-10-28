// Tirets et pointillés : mêmes longueurs que les `draw:stroke-dash` de l'export ODG
// (carnet_core::odg::styles), en multiples de l'épaisseur du trait.
import type { Stroke } from "./types";

/** `stroke-dasharray` d'un trait d'épaisseur `widthPx` à l'écran ; aucun pour un trait plein. */
export function dashArray(stroke: Stroke | null | undefined, widthPx: number): string | undefined {
  switch (stroke?.style) {
    case "Dashed":
      return `${3 * widthPx} ${2 * widthPx}`;
    case "Dotted":
      return `${widthPx} ${widthPx}`;
    default:
      return undefined;
  }
}

export const isSolid = (stroke: Stroke | null | undefined) => !stroke?.style || stroke.style === "Solid";
