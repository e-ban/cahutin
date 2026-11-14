// Les nœuds que l'éditeur de texte connaît : tout contenu que `richToTiptap` produit doit s'y lire, sinon
// TipTap ouvre une zone vide.
import { Node, type JSONContent } from "@tiptap/core";
import type { Schema } from "@tiptap/pm/model";
import { Plugin, TextSelection, type EditorState, type Transaction } from "@tiptap/pm/state";
import { Decoration, DecorationSet } from "@tiptap/pm/view";
import Document from "@tiptap/extension-document";
import Paragraph from "@tiptap/extension-paragraph";
import Text from "@tiptap/extension-text";
import { ClassField } from "./fieldNode";
import { firstNodePt, listBackspace, listEnter } from "./lists";
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
      // Dans un élément de liste aussi : la ligne suivante reste dans l'élément, sous son texte.
      "Shift-Enter": () => {
        this.editor.view.dispatch(this.editor.state.tr.replaceSelectionWith(this.editor.schema.nodes.hardBreak.create(), true));
        return true;
      },
    };
  },
});

/**
 * Paragraphe qui peut porter une puce (`Paragraph.list`, lib/lists.ts) : un attribut, pas une liste imbriquée, pour
 * que le texte garde un paragraphe par ligne. Affiché avec la classe `li` (dessin de la puce : TextEl). Entrée garde
 * la puce (la liste continue) ; Entrée sur un élément vide et Retour arrière à son début la retirent.
 */
const ListParagraph = Paragraph.extend({
  addAttributes() {
    return {
      ...this.parent?.(),
      list: {
        default: null,
        parseHTML: (el: HTMLElement) => (el.classList.contains("li") ? "Bullet" : null),
        renderHTML: (a: { list?: string | null }) => (a.list === "Bullet" ? { class: "li" } : {}),
      },
    };
  },
  addKeyboardShortcuts() {
    const run = (rule: (s: EditorState) => Transaction | null) => () => {
      const tr = rule(this.editor.state);
      if (tr) this.editor.view.dispatch(tr);
      return !!tr;
    };
    return { ...this.parent?.(), Enter: run(listEnter), Backspace: run(listBackspace) };
  },
  // Un premier mot d'une autre taille que le bloc : sa taille sur le paragraphe, pour placer la puce comme
  // LibreOffice (TextEl, `--li-first`), comme à l'affichage.
  addProseMirrorPlugins() {
    return [
      new Plugin({
        props: {
          decorations: (state) => {
            const decos: Decoration[] = [];
            state.doc.forEach((node, pos) => {
              const pt = node.attrs.list === "Bullet" ? firstNodePt(node) : null;
              if (pt != null) decos.push(Decoration.node(pos, pos + node.nodeSize, { style: `--li-first: ${pt}pt` }));
            });
            return DecorationSet.create(state.doc, decos);
          },
        },
      }),
    ];
  },
});

export const textNodes = [Document, ListParagraph, Text, ClassField, HardBreak];

/** Texte brut collé ou déposé en `pos` (par défaut à la place de la sélection), avec la mise en forme du curseur :
 *  chaque ligne est séparée par un retour à la ligne de l'éditeur, que la frappe ne change pas en espace. Dans un
 *  élément de liste, chaque ligne devient un élément, avec sa puce. */
export function insertLines(state: EditorState, text: string, pos?: number): Transaction {
  const tr = state.tr;
  if (pos != null) tr.setSelection(TextSelection.create(tr.doc, pos));
  if (tr.selection.$from.parent.attrs.list === "Bullet") {
    const { $from, $to, empty } = tr.selection;
    const marks = tr.storedMarks ?? (empty ? $from.marks() : $from.marksAcross($to)) ?? [];
    tr.deleteSelection();
    sanitizePlainText(text)
      .split("\n")
      .forEach((line, i) => {
        // Un élément coupé en deux garde sa puce et son alignement.
        if (i > 0) tr.split(tr.selection.from);
        if (line) tr.replaceSelectionWith(state.schema.text(line, marks), false);
      });
    return tr;
  }
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
