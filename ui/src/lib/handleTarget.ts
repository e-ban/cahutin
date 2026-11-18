// L'élément qui reçoit des poignées : le seul sélectionné, ni verrouillé (hors mode modèle), ni recadré, ni en
// cours d'écriture, ni case placée par une grille ou un tableau. Partagé par les poignées du cadre et celles
// des bouts d'un trait.
import { editor } from "./docStore.svelte";
import { isPlacedCell } from "./table";
import type { Element } from "./types";

export function handleTarget(): Element | null {
  const e = editor.selected;
  if (!e || (e.locked && !editor.templateMode) || editor.cropping === e.id || editor.editingText === e.id) return null;
  // Une case de grille ou de tableau reste à sa place : la grille la place.
  return isPlacedCell(editor.page, e.id) ? null : e;
}
