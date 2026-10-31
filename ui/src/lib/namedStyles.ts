// Styles de texte nommés (Titre, Consigne, Texte, Prénom, et ceux de l'enseignant) : appliquer un style
// donne au texte son allure ; redéfinir un style change d'un coup tous les textes du document qui le
// suivent. L'allure reste écrite sur chaque texte : l'affichage et l'export n'ont rien à résoudre.
import { newId } from "./factory";
import type { Document, Element, NamedTextStyle, Page } from "./types";

export const BUILTIN_STYLES: NamedTextStyle[] = [
  { id: "titre", name: "Titre", font: "Playwrite FR Moderne", size_pt: 28, color: "#c0392b", align: "Center", outline: null },
  { id: "consigne", name: "Consigne", font: "Andika", size_pt: 16, color: "#1f2c4a", align: "Left", outline: null },
  { id: "texte", name: "Texte", font: "Andika", size_pt: 14, color: "#222222", align: "Left", outline: null },
  { id: "prenom", name: "Prénom", font: "Playwrite FR Moderne", size_pt: 32, color: "#1f2c4a", align: "Center", outline: null },
];

/** Les styles du document : ceux de Cahutin (tels que le document les a redéfinis), puis les siens. */
export function stylesOf(doc: Document): NamedTextStyle[] {
  const own = doc.text_styles ?? [];
  const builtin = BUILTIN_STYLES.map((b) => own.find((s) => s.id === b.id) ?? b);
  return [...builtin, ...own.filter((s) => !BUILTIN_STYLES.some((b) => b.id === s.id))];
}

/** Le texte prend l'allure du style ; les tailles, polices et couleurs posées sur des mots sont retirées. */
export function applyStyle(e: Element, s: NamedTextStyle): Element {
  if (e.kind.type !== "Text") return e;
  const k = e.kind;
  return {
    ...e,
    kind: {
      ...k,
      style: { ...k.style, font: s.font, size_pt: s.size_pt, color: s.color, align: s.align, outline: s.outline ?? null, named: s.id },
      content: { paragraphs: k.content.paragraphs.map((p) => ({ ...p, align: null, runs: p.runs.map((r) => ({ ...r, font: null, size_pt: null, color: null })) })) },
    },
  };
}

/** Le texte a-t-il gardé l'allure de son style ? */
export function followsStyle(e: Element, s: NamedTextStyle): boolean {
  if (e.kind.type !== "Text") return false;
  const t = e.kind.style;
  return t.font === s.font && t.size_pt === s.size_pt && t.color === s.color && t.align === s.align && !!t.outline === !!s.outline;
}

/** Le style tiré d'un texte (pour redéfinir un style, ou en créer un). */
export function lookOf(e: Element, id: string, name: string): NamedTextStyle | null {
  if (e.kind.type !== "Text") return null;
  const t = e.kind.style;
  return { id, name, font: t.font, size_pt: t.size_pt, color: t.color, align: t.align, outline: t.outline ?? null };
}

const restyle = (pages: Page[], s: NamedTextStyle): Page[] =>
  pages.map((p) => ({ ...p, elements: p.elements.map((e) => (e.kind.type === "Text" && e.kind.style.named === s.id ? applyStyle(e, s) : e)) }));

/** Le style redéfini dans le document, et tous ses textes avec lui. */
export function redefineStyle(doc: Document, s: NamedTextStyle): Document {
  const own = doc.text_styles ?? [];
  const text_styles = own.some((x) => x.id === s.id) ? own.map((x) => (x.id === s.id ? s : x)) : [...own, s];
  return { ...doc, text_styles, pages: restyle(doc.pages, s) };
}

/** Un nouveau style, tiré d'un texte, que ce texte suit aussitôt. */
export function newStyleFrom(doc: Document, e: Element, name: string): { doc: Document; style: NamedTextStyle } | null {
  const style = lookOf(e, newId(), name.trim());
  if (!style || !style.name) return null;
  return { doc: { ...doc, text_styles: [...(doc.text_styles ?? []), style] }, style };
}
