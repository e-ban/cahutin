// Aides de typographie française pendant l'écriture : l'espace tapé avant « : ; ! ? » ou contre un guillemet
// devient insécable (le signe ne part pas seul à la ligne), et la palette insère les lettres difficiles à taper.
// Rien n'est ajouté sans espace tapé : « Hello! » reste tel quel. Les guillemets droits ne sont jamais changés.
import { Extension } from "@tiptap/core";
import { closeHistory } from "@tiptap/pm/history";
import { Plugin, TextSelection, type EditorState, type Transaction } from "@tiptap/pm/state";

/** Espace insécable ordinaire (U+00A0) : certaines polices n'ont pas l'espace fine. */
export const NBSP = " ";

/** Les caractères de la palette, dans l'ordre où ils s'affichent. Les guillemets ont leur propre bouton. */
export const PALETTE_CHARS = ["É", "È", "Ê", "À", "Â", "Ç", "Ô", "Î", "Ù", "Œ", "Æ", "œ", "æ", "…"];

/** Signes précédés d'une espace insécable quand on a tapé une espace avant eux. */
const AFTER_SPACE = new Set([":", ";", "!", "?", "»"]);

/** Le caractère juste avant ou juste après `pos` dans son paragraphe ; une pastille de champ ou un retour à la
 *  ligne ne compte pas comme du texte. */
function charAt(state: EditorState, pos: number, side: -1 | 1): string {
  const $pos = state.doc.resolve(pos);
  const off = $pos.parentOffset;
  const start = side < 0 ? off - 1 : off;
  if (start < 0 || start + 1 > $pos.parent.content.size) return "";
  return $pos.parent.textBetween(start, start + 1, undefined, "￼");
}

/**
 * La frappe de `text` entre `from` et `to`, avec l'espace insécable : la frappe telle quelle d'abord, puis l'espace
 * changée dans un pas d'annulation à part (Ctrl+Z rend exactement ce qui a été tapé). `dispatch` applique une
 * transaction et rend l'état suivant. Faux si la règle ne s'applique pas : la frappe suit alors son cours normal.
 */
export function typeWithSpacing(state: EditorState, from: number, to: number, text: string, dispatch: (tr: Transaction) => EditorState, typed?: () => Transaction): boolean {
  let space: number | null = null;
  if (AFTER_SPACE.has(text) && charAt(state, from, -1) === " ") space = from - 1;
  else if (text === " " && (charAt(state, from, -1) === "«" || charAt(state, to, 1) === "»")) space = from;
  if (space === null) return false;
  const next = dispatch(typed ? typed() : state.tr.insertText(text, from, to));
  const $s = next.doc.resolve(space);
  const marks = $s.marksAcross(next.doc.resolve(space + 1)) ?? $s.marks();
  const changed = dispatch(closeHistory(next.tr.replaceWith(space, space + 1, next.schema.text(NBSP, marks))));
  // La frappe suivante commence elle aussi un nouveau pas : Ctrl+Z n'annule que l'espace changée.
  dispatch(closeHistory(changed.tr));
  return true;
}

/** Les positions de texte de la sélection (une sélection de tout le texte commence avant le premier paragraphe). */
function textRange(state: EditorState): { from: number; to: number } {
  const { from, to } = state.selection;
  const sel = TextSelection.between(state.doc.resolve(from), state.doc.resolve(to));
  return { from: sel.from, to: sel.to };
}

/** Un caractère de la palette au curseur, à la place du passage choisi. */
export function insertChar(state: EditorState, ch: string): Transaction {
  const { from, to } = textRange(state);
  return state.tr.insertText(ch, from, to).scrollIntoView();
}

/** Des guillemets « » avec leurs espaces insécables : autour du passage choisi, qui reste choisi ; sans passage,
 *  une paire avec le curseur au milieu. Un seul pas d'annulation. */
export function insertGuillemets(state: EditorState): Transaction {
  const { from, to } = textRange(state);
  const tr = state.tr;
  if (from === to) {
    tr.insertText(`«${NBSP}${NBSP}»`, from);
    tr.setSelection(TextSelection.create(tr.doc, from + 2));
  } else {
    tr.insertText(`${NBSP}»`, to);
    tr.insertText(`«${NBSP}`, from);
    tr.setSelection(TextSelection.create(tr.doc, from + 2, to + 2));
  }
  return tr.scrollIntoView();
}

/** L'espace insécable pendant la frappe, pour l'éditeur de texte. */
export const FrenchSpacing = Extension.create({
  name: "frenchSpacing",
  addProseMirrorPlugins() {
    return [
      new Plugin({
        props: {
          handleTextInput(view, from, to, text, deflt) {
            return typeWithSpacing(
              view.state,
              from,
              to,
              text,
              (tr) => {
                view.dispatch(tr);
                return view.state;
              },
              deflt,
            );
          },
        },
      }),
    ];
  },
});
