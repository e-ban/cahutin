// Police, taille ou couleur données au bloc de texte entier : les mots qui avaient la leur la perdent,
// comme pour un style nommé (`applyStyle`) et pour l'assistant (`set_text_style`).
import type { RichText, TextProps, TextStyle } from "./types";

export type WholeLook = Partial<Pick<TextStyle, "font" | "size_pt" | "color">>;

export function lookOnWholeText(t: TextProps, look: WholeLook): TextProps {
  const cleared = Object.fromEntries(Object.keys(look).map((k) => [k, null]));
  return {
    ...t,
    style: { ...t.style, ...look },
    content: { paragraphs: t.content.paragraphs.map((p) => ({ ...p, runs: p.runs.map((r) => ({ ...r, ...cleared })) })) },
  };
}

const LOOK_KEYS = ["font", "size_pt", "color"] as const;

/**
 * À la fin de l'écriture : une police, une taille ou une couleur que portent tous les passages du texte devient
 * celle du bloc, et les passages la perdent. Le texte s'affiche pareil ; le panneau montre la vraie valeur du
 * bloc, et un changement de tout le texte pendant l'écriture s'annule en un seul pas avec le contenu.
 */
export function liftUniformLook(t: TextProps): TextProps {
  const runs = t.content.paragraphs.flatMap((p) => p.runs);
  const look: WholeLook = {};
  for (const k of LOOK_KEYS) {
    const v = runs[0]?.[k];
    if (v != null && runs.every((r) => r[k] === v)) Object.assign(look, { [k]: v });
  }
  return Object.keys(look).length ? lookOnWholeText(t, look) : t;
}

/** Le texte à la fin de l'écriture, avec `content` tapé ; null si le contenu n'a pas changé (rien à enregistrer). */
export function finishedText(t: TextProps, content: RichText): TextProps | null {
  return JSON.stringify(t.content) === JSON.stringify(content) ? null : liftUniformLook({ ...t, content });
}
