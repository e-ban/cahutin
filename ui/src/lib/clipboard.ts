// Copier, coller, dupliquer, ordre d'empilement et suppression d'éléments d'une page.
// Le presse-papiers est interne à l'application : rien ne passe par celui du système.
import { newId } from "./factory";
import { dissolveSmallGroups, expandSelection } from "./groups";
import type { Element, Page } from "./types";

export const PASTE_OFFSET_MM = 5;

/** Copies profondes des éléments désignés, dans l'ordre de la page. */
export function copyElements(page: Page, ids: string[]): Element[] {
  return page.elements.filter((e) => ids.includes(e.id)).map(plainCopy);
}

// Copie profonde par JSON : les éléments peuvent être des proxys d'état Svelte, que structuredClone refuse.
function plainCopy(e: Element): Element {
  return JSON.parse(JSON.stringify(e)) as Element;
}

/** Colle des copies décalées, toujours libres : jamais dans la grille, jamais zone à remplir, jamais verrouillées. */
export function pasteElements(page: Page, copied: Element[], offsetMm = PASTE_OFFSET_MM): { page: Page; ids: string[] } {
  if (copied.length === 0) return { page, ids: [] };
  // Chaque collage crée ses propres groupes : coller deux fois donne deux groupes distincts.
  const groups = new Map<string, string>();
  const regroup = (g: string | null | undefined) => {
    if (!g) return null;
    if (!groups.has(g)) groups.set(g, newId());
    return groups.get(g)!;
  };
  const ids = new Map(copied.map((e) => [e.id, newId()]));
  const pasted = copied.map((e) => ({
    ...plainCopy(e),
    id: ids.get(e.id)!,
    group: regroup(e.group),
    zone: false,
    locked: false,
    frame: { ...e.frame, x_mm: e.frame.x_mm + offsetMm, y_mm: e.frame.y_mm + offsetMm },
  }));
  // Un tableau collé désigne ses cases collées.
  for (const e of pasted) {
    if (e.kind.type === "Table") e.kind = { ...e.kind, cells: e.kind.cells.map((c) => ({ ...c, element_id: ids.get(c.element_id) ?? c.element_id })) };
  }
  return { page: { ...page, elements: [...page.elements, ...pasted] }, ids: pasted.map((e) => e.id) };
}

export function duplicateElements(page: Page, ids: string[]): { page: Page; ids: string[] } {
  return pasteElements(page, copyElements(page, ids));
}

/** Devant (fin de la liste) ou derrière (début), sans changer l'ordre entre les éléments déplacés. */
export function reorder(page: Page, selected: string[], where: "front" | "back"): Page {
  const ids = expandSelection(page, selected);
  const moved = page.elements.filter((e) => ids.includes(e.id));
  const rest = page.elements.filter((e) => !ids.includes(e.id));
  return { ...page, elements: where === "front" ? [...rest, ...moved] : [...moved, ...rest] };
}

/** Les éléments verrouillés restent ; une case de grille supprimée disparaît aussi de la grille. */
export function removeElements(page: Page, ids: string[]): Page {
  const gone = new Set(page.elements.filter((e) => ids.includes(e.id) && !e.locked).map((e) => e.id));
  if (gone.size === 0) return page;
  const cells = page.grid?.cells.filter((c) => !gone.has(c.element_id));
  return dissolveSmallGroups({
    ...page,
    elements: page.elements.filter((e) => !gone.has(e.id)),
    grid: page.grid && cells && cells.length > 0 ? { ...page.grid, cells } : null,
  });
}
