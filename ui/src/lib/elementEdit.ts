// Modifications de l'élément sélectionné, partagées par la barre flottante et le panneau de droite.
import { editor } from "./docStore.svelte";
import { TEXT_PADDING_MM } from "./richtext";
import type { Element, PhotoProps, ShapeProps, TableProps, TextFrame, TextStyle } from "./types";

export function updateSelected(fn: (e: Element) => Element) {
  const e = editor.selected;
  if (e) editor.updateElement(e.id, fn);
}

/** Curseur que l'on fait glisser : aperçu en direct, un seul pas d'annulation à `endPreview`. */
export function previewSelected(fn: (e: Element) => Element) {
  const sel = editor.selected;
  if (sel) previewElement(sel.id, fn);
}

function previewElement(id: string, fn: (e: Element) => Element) {
  const doc = editor.doc;
  editor.preview({
    ...doc,
    pages: doc.pages.map((p, i) => (i === editor.pageIndex ? { ...p, elements: p.elements.map((e) => (e.id === id ? fn(e) : e)) } : p)),
  });
}

export const endPreview = () => editor.endPreview();

const photo = (patch: Partial<PhotoProps>) => (e: Element): Element => (e.kind.type === "Photo" ? { ...e, kind: { ...e.kind, ...patch } } : e);
const shape = (patch: Partial<ShapeProps>) => (e: Element): Element => (e.kind.type === "Shape" ? { ...e, kind: { ...e.kind, ...patch } } : e);

const table = (patch: Partial<TableProps>) => (e: Element): Element => (e.kind.type === "Table" ? { ...e, kind: { ...e.kind, ...patch } } : e);

/** Traits d'un tableau, sélectionné avec toutes ses cases. */
export const updateTable = (id: string, patch: Partial<TableProps>) => editor.updateElement(id, table(patch));
export const previewTable = (id: string, patch: Partial<TableProps>) => previewElement(id, table(patch));

export const updatePhoto = (patch: Partial<PhotoProps>) => updateSelected(photo(patch));
export const previewPhoto = (patch: Partial<PhotoProps>) => previewSelected(photo(patch));
export const updateShape = (patch: Partial<ShapeProps>) => updateSelected(shape(patch));
export const previewShape = (patch: Partial<ShapeProps>) => previewSelected(shape(patch));

/** Réglages du texte ; la marge par défaut, l'alignement en haut et les lettres pleines sont enregistrés
 *  comme absents (format inchangé). */
export const textStyle = (patch: Partial<TextStyle>) => (e: Element): Element => {
  if (e.kind.type !== "Text") return e;
  const style = { ...e.kind.style, ...patch };
  if (style.padding_mm === TEXT_PADDING_MM) style.padding_mm = null;
  if (style.valign === "Top") style.valign = null;
  if (!style.outline) style.outline = null;
  return { ...e, kind: { ...e.kind, style } };
};
/** Cadre d'un texte ; un cadre sans fond, sans bordure et sans image disparaît (le document reste lisible par la 0.4). */
export const textFrame = (patch: Partial<TextFrame>) => (e: Element): Element => {
  if (e.kind.type !== "Text") return e;
  const f = { fill: null, stroke: null, corner_radius_mm: 0, image: null, ...e.kind.frame, ...patch };
  const empty = !f.fill && !f.stroke && !f.image;
  return { ...e, kind: { ...e.kind, frame: empty ? null : f } };
};

export const updateTextStyle = (patch: Partial<TextStyle>) => updateSelected(textStyle(patch));
export const previewTextStyle = (patch: Partial<TextStyle>) => previewSelected(textStyle(patch));
export const updateTextFrame = (patch: Partial<TextFrame>) => updateSelected(textFrame(patch));
export const previewTextFrame = (patch: Partial<TextFrame>) => previewSelected(textFrame(patch));
