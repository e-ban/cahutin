import type { Page } from "./types";

const isCell = (page: Page, id: string) => !!page.grid?.cells.some((c) => c.element_id === id);

/** Deux cases de la grille sélectionnées (Maj+clic) : la paire à fusionner. */
export function mergePair(selected: string[], page: Page): [string, string] | null {
  if (selected.length !== 2 || !selected.every((id) => isCell(page, id))) return null;
  return [selected[0], selected[1]];
}

/** Une seule case sélectionnée, et fusionnée : celle qu'on peut scinder. */
export function spannedCell(selected: string[], page: Page): string | null {
  if (selected.length !== 1) return null;
  const c = page.grid?.cells.find((x) => x.element_id === selected[0]);
  return c && (c.col_span > 1 || c.row_span > 1) ? c.element_id : null;
}
