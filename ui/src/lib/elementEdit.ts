// Modifications de l'élément sélectionné, partagées par la barre flottante et le panneau de droite.
import { editor } from "./docStore.svelte";
import type { Element, PhotoProps, ShapeProps, TextFrame, TextStyle } from "./types";

export function updateSelected(fn: (e: Element) => Element) {
  const e = editor.selected;
  if (e) editor.updateElement(e.id, fn);
}

/** Curseur que l'on fait glisser : aperçu en direct, un seul pas d'annulation à `endPreview`. */
export function previewSelected(fn: (e: Element) => Element) {
  const sel = editor.selected;
  if (!sel) return;
  const doc = editor.doc;
  editor.preview({
    ...doc,
    pages: doc.pages.map((p, i) => (i === editor.pageIndex ? { ...p, elements: p.elements.map((e) => (e.id === sel.id ? fn(e) : e)) } : p)),
  });
}

export const endPreview = () => editor.endPreview();

const photo = (patch: Partial<PhotoProps>) => (e: Element): Element => (e.kind.type === "Photo" ? { ...e, kind: { ...e.kind, ...patch } } : e);
const shape = (patch: Partial<ShapeProps>) => (e: Element): Element => (e.kind.type === "Shape" ? { ...e, kind: { ...e.kind, ...patch } } : e);

export const updatePhoto = (patch: Partial<PhotoProps>) => updateSelected(photo(patch));
export const previewPhoto = (patch: Partial<PhotoProps>) => previewSelected(photo(patch));
export const updateShape = (patch: Partial<ShapeProps>) => updateSelected(shape(patch));
export const previewShape = (patch: Partial<ShapeProps>) => previewSelected(shape(patch));

const textStyle = (patch: Partial<TextStyle>) => (e: Element): Element =>
  e.kind.type === "Text" ? { ...e, kind: { ...e.kind, style: { ...e.kind.style, ...patch } } } : e;
/** Cadre d'un texte ; un cadre sans fond, sans bordure et sans image disparaît (le document reste lisible par la 0.4). */
const textFrame = (patch: Partial<TextFrame>) => (e: Element): Element => {
  if (e.kind.type !== "Text") return e;
  const f = { fill: null, stroke: null, corner_radius_mm: 0, image: null, ...e.kind.frame, ...patch };
  const empty = !f.fill && !f.stroke && !f.image;
  return { ...e, kind: { ...e.kind, frame: empty ? null : f } };
};

export const updateTextStyle = (patch: Partial<TextStyle>) => updateSelected(textStyle(patch));
export const previewTextStyle = (patch: Partial<TextStyle>) => previewSelected(textStyle(patch));
export const updateTextFrame = (patch: Partial<TextFrame>) => updateSelected(textFrame(patch));
export const previewTextFrame = (patch: Partial<TextFrame>) => previewSelected(textFrame(patch));
