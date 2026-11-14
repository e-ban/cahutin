// Police, taille ou couleur données au bloc de texte entier : les mots qui avaient la leur la perdent,
// comme pour un style nommé (`applyStyle`) et pour l'assistant (`set_text_style`).
import type { TextProps, TextStyle } from "./types";

export type WholeLook = Partial<Pick<TextStyle, "font" | "size_pt" | "color">>;

export function lookOnWholeText(t: TextProps, look: WholeLook): TextProps {
  const cleared = Object.fromEntries(Object.keys(look).map((k) => [k, null]));
  return {
    ...t,
    style: { ...t.style, ...look },
    content: { paragraphs: t.content.paragraphs.map((p) => ({ ...p, runs: p.runs.map((r) => ({ ...r, ...cleared })) })) },
  };
}

/** La sélection de l'éditeur couvre-t-elle tout le texte (de `from` à `to`, dans un document de `docSize`) ? */
export function coversWholeText(from: number, to: number, docSize: number): boolean {
  return from < to && from <= 1 && to >= docSize - 1;
}
