// Les nœuds que l'éditeur de texte connaît : tout contenu que `richToTiptap` produit doit s'y lire, sinon
// TipTap ouvre une zone vide.
import { Node, type JSONContent } from "@tiptap/core";
import type { Schema } from "@tiptap/pm/model";
import { TextSelection, type EditorState, type Transaction } from "@tiptap/pm/state";
import Document from "@tiptap/extension-document";
import Paragraph from "@tiptap/extension-paragraph";
import Text from "@tiptap/extension-text";
import { ClassField } from "./fieldNode";
import { sanitizePlainText } from "./richtext";

/** Retour à la ligne dans un paragraphe (texte collé sur plusieurs lignes, Maj+Entrée), enregistré en « \n ». */
export const HardBreak = Node.create({
  name: "hardBreak",
  group: "inline",
  inline: true,
  selectable: false,
  parseHTML() {
    return [{ tag: "br" }];
  },
  renderHTML() {
    return ["br"];
  },
  renderText() {
    return "\n";
  },
  addKeyboardShortcuts() {
    return {
      "Shift-Enter": () => {
        this.editor.view.dispatch(insertLines(this.editor.state, "\n"));
        return true;
      },
    };
  },
});

export const textNodes = [Document, Paragraph, Text, ClassField, HardBreak];

/** Texte brut collé ou déposé en `pos` (par défaut à la place de la sélection), avec la mise en forme du curseur :
 *  chaque ligne est séparée par un retour à la ligne de l'éditeur, que la frappe ne change pas en espace. */
export function insertLines(state: EditorState, text: string, pos?: number): Transaction {
  const tr = state.tr;
  if (pos != null) tr.setSelection(TextSelection.create(tr.doc, pos));
  sanitizePlainText(text)
    .split("\n")
    .forEach((line, i) => {
      if (i > 0) tr.replaceSelectionWith(state.schema.nodes.hardBreak.create(), true);
      if (line) tr.insertText(line);
    });
  return tr;
}

/** Le contenu se lit dans l'éditeur ; sinon TipTap ouvre une zone vide à la place. */
export function readable(schema: Schema, content: JSONContent): boolean {
  try {
    schema.nodeFromJSON(content);
    return true;
  } catch {
    return false;
  }
}
