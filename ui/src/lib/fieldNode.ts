// La pastille d'un champ de la classe dans l'éditeur : un seul caractère (un retour arrière l'efface d'un
// coup, le curseur n'entre pas dedans), qui montre le nom du champ et s'enregistre en accolades.
import { Node } from "@tiptap/core";
import type { Fragment } from "@tiptap/pm/model";
import { fieldLabel, fieldShown } from "./classFields";

export const ClassField = Node.create({
  name: "classField",
  group: "inline",
  inline: true,
  atom: true,
  selectable: true,
  addAttributes() {
    return {
      key: {
        default: "prenom",
        parseHTML: (el: HTMLElement) => el.getAttribute("data-field"),
        renderHTML: (a: { key: string }) => ({ "data-field": a.key }),
      },
    };
  },
  parseHTML() {
    return [{ tag: "span[data-field]" }];
  },
  renderHTML({ node, HTMLAttributes }) {
    const key: string = node.attrs.key;
    return ["span", { ...HTMLAttributes, class: "class-field", contenteditable: "false", title: fieldLabel(key) ?? key }, fieldShown(key) ?? key];
  },
  // Copié en texte brut : ses accolades, pour qu'il redevienne un champ là où on le colle.
  renderText({ node }) {
    return `{${node.attrs.key}}`;
  },
});

/** Le texte brut d'un passage : chaque pastille en accolades, chaque fin de paragraphe en retour à la ligne. */
export function plainText(fragment: Fragment): string {
  return fragment.textBetween(0, fragment.size, "\n", (n) => (n.type.name === "classField" ? `{${n.attrs.key}}` : "\n"));
}
