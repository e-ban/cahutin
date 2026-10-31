// Répéter un bloc en grille sur la page : planches d'étiquettes ou de cartes sans modèle dédié.
// Les copies gardent leurs zones à remplir, pour que la classe les remplisse ensuite.
import { newId } from "./factory";
import { boundsOf, expandSelection } from "./groups";
import { remapTables } from "./table";
import type { Element, Page } from "./types";

export const REPEAT_GAP_MM = 5;
/** Marge laissée au bord de la page par « Remplir la page ». */
export const REPEAT_MARGIN_MM = 10;

/** Le plus de colonnes et de lignes qui tiennent sur la page, à partir de la place du bloc. */
export function fitCount(page: Page, ids: string[], gap = REPEAT_GAP_MM): { cols: number; rows: number } {
  const b = boundsOf(page.elements.filter((e) => ids.includes(e.id)).map((e) => e.frame));
  const fit = (start: number, size: number, limit: number) => Math.max(1, Math.floor((limit - REPEAT_MARGIN_MM - start + gap) / (size + gap)));
  return { cols: fit(b.x_mm, b.w_mm, page.size.width_mm), rows: fit(b.y_mm, b.h_mm, page.size.height_mm) };
}

/**
 * Le bloc sélectionné répété en `cols` × `rows` (lui compris, en haut à gauche), espacé de `gap`. Chaque
 * exemplaire forme un groupe, pour se déplacer d'un bloc et recevoir un élève. Rend la page et les
 * identifiants de tous les exemplaires.
 */
export function repeatBlock(page: Page, selected: string[], cols: number, rows: number, gap = REPEAT_GAP_MM): { page: Page; ids: string[] } {
  const ids = expandSelection(page, selected);
  const block = page.elements.filter((e) => ids.includes(e.id));
  if (block.length === 0) return { page, ids: [] };
  const b = boundsOf(block.map((e) => e.frame));
  // Plusieurs éléments sans groupe commun : le bloc d'origine devient un groupe, comme ses copies.
  const shared = block.every((e) => e.group && e.group === block[0].group);
  const ownGroup = block.length > 1 && !shared ? newId() : null;
  const original = block.map((e) => (ownGroup ? { ...e, group: ownGroup } : e));
  const copies: Element[] = [];
  for (let r = 0; r < rows; r++) {
    for (let c = 0; c < cols; c++) {
      if (r === 0 && c === 0) continue;
      const groups = new Map<string, string>();
      const regroup = (g: string | null | undefined) => (g ? (groups.get(g) ?? groups.set(g, newId()).get(g)!) : null);
      const newIds = new Map(original.map((e) => [e.id, newId()]));
      const one = original.map((e) => ({
        ...(JSON.parse(JSON.stringify(e)) as Element),
        id: newIds.get(e.id)!,
        group: regroup(e.group),
        locked: false,
        frame: { ...e.frame, x_mm: e.frame.x_mm + c * (b.w_mm + gap), y_mm: e.frame.y_mm + r * (b.h_mm + gap) },
      }));
      copies.push(...remapTables(one, newIds));
    }
  }
  const elements = [...page.elements.map((e) => original.find((o) => o.id === e.id) ?? e), ...copies];
  return { page: { ...page, elements }, ids: [...original, ...copies].map((e) => e.id) };
}
