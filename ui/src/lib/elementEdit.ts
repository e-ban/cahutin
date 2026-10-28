// Modifications de l'élément sélectionné, partagées par la barre flottante et le panneau de droite.
import { editor } from "./docStore.svelte";
import type { Element, PhotoProps, ShapeProps } from "./types";

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
