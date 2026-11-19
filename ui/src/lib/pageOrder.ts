import { remapTables } from "./table";
import { remapAttach } from "./connectors";
// Ordre des pages : déplacer une page, et la place où tombe une miniature glissée dans la bande des pages.
import type { Document, Page } from "./types";

/** `doc` avec la page `from` à la place `to` (sa place finale) ; le même document s'il n'y a rien à changer. */
export function movePage(doc: Document, from: number, to: number): Document {
  const n = doc.pages.length;
  if (from === to || from < 0 || from >= n || to < 0 || to >= n) return doc;
  const pages = [...doc.pages];
  const [page] = pages.splice(from, 1);
  pages.splice(to, 0, page);
  return { ...doc, pages };
}

/** La fente sous le pointeur (0 : avant la première miniature), d'après le milieu de chaque miniature. */
export function dropSlot(mids: number[], x: number): number {
  const k = mids.findIndex((m) => x < m);
  return k < 0 ? mids.length : k;
}

/** La place finale de la page `from` lâchée dans la fente `slot` ; `null` si elle ne bouge pas. */
export function slotTarget(from: number, slot: number): number | null {
  const to = slot > from ? slot - 1 : slot;
  return to === from ? null : to;
}

/** La page déplacée d'un ordre à l'autre (mêmes pages, une seule a changé de place), ou `null`. Deux voisines
 *  échangées : `prefer` si c'est l'une d'elles. */
export function movedPage(before: Page[], after: Page[], prefer?: string): string | null {
  const a = before.map((p) => p.id);
  const b = after.map((p) => p.id);
  if (a.length !== b.length || a.every((id, i) => id === b[i]) || !a.every((id) => b.includes(id))) return null;
  const without = (list: string[], id: string) => list.filter((x) => x !== id);
  const fits = a.filter((id) => without(a, id).every((x, i) => x === without(b, id)[i]));
  return prefer && fits.includes(prefer) ? prefer : (fits[0] ?? null);
}

/** La copie d'une page : nouveaux identifiants partout ; ses tableaux, sa grille et ses flèches accrochées désignent
 *  les copies. */
export function duplicatePage(p: Page): Page {
  const ids = new Map(p.elements.map((e) => [e.id, crypto.randomUUID()]));
  return {
    ...structuredClone(p),
    id: crypto.randomUUID(),
    elements: remapAttach(remapTables(p.elements.map((e) => ({ ...structuredClone(e), id: ids.get(e.id)! })), ids), ids),
    grid: p.grid ? { ...p.grid, cells: p.grid.cells.map((c) => ({ ...c, element_id: ids.get(c.element_id)! })) } : null,
  };
}

/** Une page où rien n'est posé (ni élément, ni grille, ni fond) : un modèle choisi y prend sa place, plutôt que
 *  de laisser une page blanche en trop dans le document. */
export function isBlankPage(p: Page): boolean {
  return p.elements.length === 0 && !p.grid && !p.background;
}
