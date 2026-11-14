// Listes à puces : une puce devant un paragraphe (`Paragraph.list`), un seul niveau, sans numéros. La puce n'est
// pas un caractère du texte : l'écran la dessine en CSS (TextEl), l'ODG en vraie liste de LibreOffice.
import { closeHistory } from "@tiptap/pm/history";
import type { Node as PmNode } from "@tiptap/pm/model";
import { TextSelection, type EditorState, type Transaction } from "@tiptap/pm/state";
import { parsePt } from "./richtext";
import type { FontMetrics, Paragraph, RichText, Run, TextProps } from "./types";

/** Retrait d'un élément de liste, en em de la taille du bloc : la puce y tient, le texte et ses lignes suivantes
 *  commencent après (`carnet_core::model::LIST_INDENT_EM`). */
export const LIST_INDENT_EM = 1.2;

/** La puce, dans la police, la couleur et à la taille du bloc (`carnet_core::model::LIST_BULLET`). */
export const LIST_BULLET = "•";

/** Le paragraphe a du texte (des espaces seules ne comptent pas, comme pour un bloc vide). */
const hasText = (p: Paragraph) => p.runs.some((r) => r.text.trim() !== "");

/** Le paragraphe sans sa puce : la clé disparaît, comme dans le fichier (jamais `list: null`). */
function withoutBullet(p: Paragraph): Paragraph {
  const { list: _, ...rest } = p;
  return rest;
}

/** Les paragraphes non vides du bloc portent tous une puce (et il y en a au moins un). */
export function bulletsOn(content: RichText): boolean {
  const full = content.paragraphs.filter(hasText);
  return full.length > 0 && full.every((p) => p.list === "Bullet");
}

/** Le bouton « Puces » hors écriture : des puces à tous les paragraphes non vides du bloc, ou aucune s'ils en ont
 *  tous. Null : le bloc n'a pas de texte, rien à changer. */
export function toggledBullets(t: TextProps): TextProps | null {
  if (!t.content.paragraphs.some(hasText)) return null;
  const on = !bulletsOn(t.content);
  const paragraphs = t.content.paragraphs.map((p) => (on ? (hasText(p) ? { ...p, list: "Bullet" as const } : p) : withoutBullet(p)));
  return { ...t, content: { paragraphs } };
}

/** Un bloc sans texte ne garde pas de puce : l'export ne l'écrit pas, et elle ne changerait que la version du fichier. */
export function blankWithoutBullets(content: RichText): RichText {
  if (content.paragraphs.some(hasText) || !content.paragraphs.some((p) => p.list)) return content;
  return { paragraphs: content.paragraphs.map(withoutBullet) };
}

/** Taille du premier mot d'un élément de liste (le premier passage non vide, comme l'export), s'il a la sienne. */
export function firstWordPt(runs: Run[]): number | null {
  return runs.find((r) => r.text !== "")?.size_pt ?? null;
}

/**
 * Un premier mot plus grand que le bloc : LibreOffice remonte la puce de (ascendante - descendante) / 2 de la
 * police du bloc, par point d'écart entre les deux tailles. Ce facteur (en em), que le CSS multiplie par l'écart
 * (`max(0, premier mot - taille du bloc)`, TextEl) ; police inconnue : 0, la puce ne bouge pas.
 */
export function bulletLiftFactor(metrics: FontMetrics | null | undefined): number {
  if (!metrics || !(metrics.units_per_em > 0)) return 0;
  return (metrics.ascent - metrics.descent) / (2 * metrics.units_per_em);
}

// --- Pendant l'écriture (ProseMirror) ---

/** Taille du premier morceau non vide d'un paragraphe de l'éditeur, s'il a la sienne (comme `firstWordPt`). */
export function firstNodePt(node: PmNode): number | null {
  let pt: number | null = null;
  let seen = false;
  node.forEach((child) => {
    if (seen || (child.isText && !child.text)) return;
    seen = true;
    pt = parsePt(child.marks.find((m) => m.type.name === "textStyle")?.attrs.fontSize);
  });
  return pt;
}

/** Le paragraphe de l'éditeur a du texte ; une pastille de champ compte, un retour à la ligne non. */
const nodeHasText = (node: PmNode) => node.textBetween(0, node.content.size, undefined, (leaf) => (leaf.type.name === "classField" ? "x" : "")).trim() !== "";

/** Les paragraphes que vise le bouton : celui du curseur ; une sélection sur plusieurs, ceux qui ont du texte. */
function targets(state: EditorState): { pos: number; node: PmNode }[] {
  const { from, to } = state.selection;
  const all: { pos: number; node: PmNode }[] = [];
  state.doc.nodesBetween(from, to, (node, pos) => {
    if (node.type.name !== "paragraph") return true;
    all.push({ pos, node });
    return false;
  });
  if (all.length <= 1) return all;
  const full = all.filter((p) => nodeHasText(p.node));
  return full.length ? full : all;
}

/** Les paragraphes visés portent tous une puce. */
export function listActive(state: EditorState): boolean {
  const t = targets(state);
  return t.length > 0 && t.every((p) => p.node.attrs.list === "Bullet");
}

/** Le bouton « Puces » pendant l'écriture : un pas d'annulation à lui, même juste après une frappe. */
export function toggleList(state: EditorState): Transaction {
  const tr = closeHistory(state.tr);
  // Un texte tout neuf n'a pas encore de paragraphe : le premier élément de la liste est créé, le curseur dedans.
  if (state.doc.childCount === 0) {
    tr.insert(0, state.schema.nodes.paragraph.create({ list: "Bullet" }));
    return tr.setSelection(TextSelection.create(tr.doc, 1));
  }
  const on = !listActive(state);
  for (const { pos, node } of targets(state)) tr.setNodeMarkup(pos, undefined, { ...node.attrs, list: on ? "Bullet" : null });
  return tr;
}

/** L'élément de liste du curseur (sélection vide), s'il y en a un. */
function bulletAtCursor(state: EditorState) {
  const { $from, empty } = state.selection;
  const node = $from.parent;
  return empty && node.type.name === "paragraph" && node.attrs.list === "Bullet" ? { $from, node, pos: $from.before() } : null;
}

/** La puce retirée, dans un pas d'annulation à part : Ctrl+Z la rend sans effacer ce qui vient d'être tapé. */
const dropBullet = (state: EditorState, pos: number, node: PmNode) => closeHistory(state.tr).setNodeMarkup(pos, undefined, { ...node.attrs, list: null });

/** Entrée sur un élément vide : il perd sa puce, on quitte la liste. Null : Entrée suit son cours (un élément
 *  coupé en deux garde sa puce, la liste continue). */
export function listEnter(state: EditorState): Transaction | null {
  const b = bulletAtCursor(state);
  return b && b.node.content.size === 0 ? dropBullet(state, b.pos, b.node) : null;
}

/** Retour arrière au début d'un élément : il perd sa puce, sans se fusionner avec le paragraphe d'avant. */
export function listBackspace(state: EditorState): Transaction | null {
  const b = bulletAtCursor(state);
  return b && b.$from.parentOffset === 0 ? dropBullet(state, b.pos, b.node) : null;
}
