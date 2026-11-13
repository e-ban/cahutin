// L'éditeur TipTap actif, pour que le panneau de droite mette en forme la sélection en cours.
import type { Editor } from "@tiptap/core";

class TextEditing {
  current = $state.raw<Editor | null>(null);
  /** Incrémenté à chaque changement de sélection ou de contenu, pour rafraîchir le panneau. */
  tick = $state(0);
  /** Point du double-clic qui ouvre un texte (son id), pour y poser le curseur ; pris une seule fois. */
  click: { id: string; x: number; y: number } | null = null;
}

export const textEditing = new TextEditing();

/** Le point du double-clic qui a ouvert le texte `id`, s'il y en a un ; oublié ensuite. */
export function takeClick(id: string): { x: number; y: number } | null {
  const c = textEditing.click;
  textEditing.click = null;
  return c && c.id === id ? { x: c.x, y: c.y } : null;
}

/** Un clic dans le panneau de droite, la barre flottante (police, couleur...) ou la liste des champs de la classe
 *  ne termine pas l'écriture : la mise en forme s'applique au passage choisi, le champ s'insère au curseur. */
export function keepsEditing(next: { closest(selector: string): unknown } | null): boolean {
  return !!next?.closest(".right, .ctx, .fields");
}
