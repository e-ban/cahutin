// Mise en forme du texte sélectionné, partagée par la barre flottante et le panneau de droite.
// Pendant l'écriture, elle s'applique à la sélection TipTap ; sinon à toute la zone.
import type { Editor } from "@tiptap/core";
// Déclarations des commandes de mise en forme (setFontFamily, setColor, setTextAlign…) ajoutées par ces extensions.
import type {} from "@tiptap/extension-text-align";
import type {} from "@tiptap/extension-text-style";
import { editor } from "./docStore.svelte";
import { alignToCss } from "./richtext";
import { textEditing } from "./textEditing.svelte";
import type { Align, Element, Run, TextProps, TextStyle } from "./types";

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

export function setFont(font: string) {
  const tip = activeTip();
  if (tip) tip.chain().focus().setFontFamily(font).run();
  else setStyle({ font });
}

export function setSize(pt: number) {
  const tip = activeTip();
  if (tip) tip.chain().focus().setFontSize(`${pt}pt`).run();
  else setStyle({ size_pt: pt });
}

export function setColor(color: string | null) {
  if (!color) return;
  const tip = activeTip();
  if (tip) tip.chain().focus().setColor(color).run();
  else setStyle({ color });
}

export function setAlign(align: Align) {
  const tip = activeTip();
  if (tip) tip.chain().focus().setTextAlign(alignToCss(align)).run();
  else updateText((t) => ({ ...t, style: { ...t.style, align }, content: { paragraphs: t.content.paragraphs.map((p) => ({ ...p, align: null })) } }));
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

export const ALIGNS: [Align, string][] = [
  ["Left", "⯇"],
  ["Center", "≡"],
  ["Right", "⯈"],
  ["Justify", "☰"],
];
