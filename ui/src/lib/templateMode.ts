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

/**
 * Cases couvertes en glissant de `fromId` à `toId` : le rectangle qui englobe les deux cases.
 * `null` s'il coupe une case fusionnée (la fusion serait refusée).
 */
export function cellsCoveredBy(page: Page, fromId: string, toId: string): string[] | null {
  const cells = page.grid?.cells ?? [];
  const a = cells.find((c) => c.element_id === fromId);
  const b = cells.find((c) => c.element_id === toId);
  if (!a || !b) return null;
  const c0 = Math.min(a.col, b.col);
  const r0 = Math.min(a.row, b.row);
  const c1 = Math.max(a.col + a.col_span, b.col + b.col_span);
  const r1 = Math.max(a.row + a.row_span, b.row + b.row_span);
  const covered: string[] = [];
  for (const c of cells) {
    const inside = c.col >= c0 && c.col + c.col_span <= c1 && c.row >= r0 && c.row + c.row_span <= r1;
    const outside = c.col + c.col_span <= c0 || c.col >= c1 || c.row + c.row_span <= r0 || c.row >= r1;
    if (inside) covered.push(c.element_id);
    else if (!outside) return null;
  }
  return covered;
}
