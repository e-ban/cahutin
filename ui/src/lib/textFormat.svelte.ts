// Mise en forme du texte sélectionné, partagée par la barre flottante et le panneau de droite.
// Pendant l'écriture, elle s'applique à la sélection TipTap ; sinon à toute la zone.
import type { Editor } from "@tiptap/core";
// Déclarations des commandes de mise en forme (setFontFamily, setColor, setTextAlign…) ajoutées par ces extensions.
import type {} from "@tiptap/extension-text-align";
import type {} from "@tiptap/extension-text-style";
import { editor } from "./docStore.svelte";
import { alignToCss, parsePt } from "./richtext";
import { textEditing } from "./textEditing.svelte";
import { coversWholeText, lookOnWholeText, type WholeLook } from "./textLook";
import type { Align, Element, Run, TextProps, TextStyle, VAlign } from "./types";

export type Mark = "bold" | "italic" | "underline";

/** L'éditeur TipTap de l'élément sélectionné, s'il est en cours d'écriture. */
export function activeTip(): Editor | null {
  const e = editor.selected;
  return e && editor.editingText === e.id ? textEditing.current : null;
}

function updateText(fn: (t: TextProps) => TextProps) {
  const e = editor.selected;
  if (e) editor.updateElement(e.id, (el: Element) => (el.kind.type === "Text" ? { ...el, kind: { type: "Text", ...fn(el.kind) } } : el));
}

function setStyle(patch: Partial<TextStyle>) {
  updateText((t) => ({ ...t, style: { ...t.style, ...patch } }));
}

/** Insère du texte au curseur pendant l'écriture (une date déjà écrite, par exemple). */
export function insertText(text: string) {
  activeTip()?.chain().focus().insertContent(text).run();
}

/** Insère la pastille d'un champ de la classe au curseur pendant l'écriture. */
export function insertField(key: string) {
  activeTip()?.chain().focus().insertContent({ type: "classField", attrs: { key } }).run();
}

/**
 * Police, taille ou couleur : pendant l'écriture, sur le passage sélectionné. Sans écriture, ou quand tout le
 * texte est sélectionné, elle devient celle du bloc, y compris des mots qui avaient la leur.
 */
function setLook(look: WholeLook, onTip: (tip: Editor) => void) {
  const tip = activeTip();
  if (tip) {
    onTip(tip);
    const { from, to } = tip.state.selection;
    // Pendant l'écriture, les mots sont dans l'éditeur (qui vient de les changer) : seul le bloc change ici.
    if (coversWholeText(from, to, tip.state.doc.content.size)) setStyle(look);
  } else updateText((t) => lookOnWholeText(t, look));
}

export function setFont(font: string) {
  setLook({ font }, (tip) => tip.chain().focus().setFontFamily(font).run());
}

export function setSize(pt: number) {
  setLook({ size_pt: pt }, (tip) => tip.chain().focus().setFontSize(`${pt}pt`).run());
}

export function setColor(color: string | null) {
  if (!color) return;
  setLook({ color }, (tip) => tip.chain().focus().setColor(color).run());
}

export function setAlign(align: Align) {
  const tip = activeTip();
  if (tip) tip.chain().focus().setTextAlign(alignToCss(align)).run();
  else updateText((t) => ({ ...t, style: { ...t.style, align }, content: { paragraphs: t.content.paragraphs.map((p) => ({ ...p, align: null })) } }));
}

/** Position du texte dans son cadre : réglage du cadre entier, même pendant l'écriture. */
export function setVAlign(valign: VAlign) {
  setStyle({ valign: valign === "Top" ? null : valign });
}

/** Interligne : réglage du bloc entier, même pendant l'écriture ; null : celui de la police. */
export function setLineSpacing(spacing: number | null) {
  setStyle({ line_spacing: spacing });
}

export function toggleMark(mark: Mark) {
  const tip = activeTip();
  if (tip) {
    tip.chain().focus().toggleMark(mark).run();
    return;
  }
  updateText((t) => {
    const all = t.content.paragraphs.flatMap((p) => p.runs);
    const on = !all.every((r: Run) => r[mark]);
    return { ...t, content: { paragraphs: t.content.paragraphs.map((p) => ({ ...p, runs: p.runs.map((r) => ({ ...r, [mark]: on })) })) } };
  });
}

/** Police du passage sélectionné pendant l'écriture, sinon celle de la zone. */
export function currentFont(t: TextProps): string {
  void textEditing.tick;
  const f = activeTip()?.getAttributes("textStyle").fontFamily;
  return typeof f === "string" && f ? f : t.style.font;
}

/** Taille du passage sélectionné pendant l'écriture, sinon celle de la zone. */
export function currentSize(t: TextProps): number {
  void textEditing.tick;
  return parsePt(activeTip()?.getAttributes("textStyle").fontSize) ?? t.style.size_pt;
}

export function isMarkActive(mark: Mark, t: TextProps): boolean {
  void textEditing.tick;
  const tip = activeTip();
  if (tip) return tip.isActive(mark);
  const runs = t.content.paragraphs.flatMap((p) => p.runs);
  return runs.length > 0 && runs.every((r) => r[mark]);
}

export function updatePrompt(prompt: string | null) {
  updateText((t) => ({ ...t, prompt }));
}

export const ALIGNS: Align[] = ["Left", "Center", "Right", "Justify"];
export const VALIGNS: VAlign[] = ["Top", "Middle", "Bottom"];
