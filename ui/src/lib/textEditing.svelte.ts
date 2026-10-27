// L'éditeur TipTap actif, pour que le panneau de droite mette en forme la sélection en cours.
import type { Editor } from "@tiptap/core";

class TextEditing {
  current = $state.raw<Editor | null>(null);
  /** Incrémenté à chaque changement de sélection ou de contenu, pour rafraîchir le panneau. */
  tick = $state(0);
}

export const textEditing = new TextEditing();
