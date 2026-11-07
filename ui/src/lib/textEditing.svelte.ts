// L'éditeur TipTap actif, pour que le panneau de droite mette en forme la sélection en cours.
import type { Editor } from "@tiptap/core";

class TextEditing {
  current = $state.raw<Editor | null>(null);
  /** Incrémenté à chaque changement de sélection ou de contenu, pour rafraîchir le panneau. */
  tick = $state(0);
}

export const textEditing = new TextEditing();

/** Un clic dans le panneau de droite, la barre flottante (police, couleur...) ou la liste des champs de la classe
 *  ne termine pas l'écriture : la mise en forme s'applique au passage choisi, le champ s'insère au curseur. */
export function keepsEditing(next: { closest(selector: string): unknown } | null): boolean {
  return !!next?.closest(".right, .ctx, .fields");
}
