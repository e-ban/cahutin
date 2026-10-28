// Actions d'édition sur la sélection, communes aux raccourcis, au clic droit et à la barre flottante.
import { copyElements, duplicateElements, pasteElements, removeElements, reorder } from "./clipboard";
import { editor } from "./docStore.svelte";
import type { Element } from "./types";

let clipboard: Element[] = [];

export const hasClipboard = () => clipboard.length > 0;

export function copySelection() {
  if (editor.selection.length) clipboard = copyElements(editor.page, editor.selection);
}

export function cutSelection() {
  copySelection();
  deleteSelection();
}

export function paste() {
  const { page, ids } = pasteElements(editor.page, clipboard);
  if (ids.length === 0) return;
  editor.updatePage(() => page);
  editor.selection = ids;
  // Coller encore décale encore : chaque collage part du dernier.
  clipboard = copyElements(page, ids);
}

export function duplicateSelection() {
  if (editor.selection.length === 0) return;
  const { page, ids } = duplicateElements(editor.page, editor.selection);
  editor.updatePage(() => page);
  editor.selection = ids;
}

export function bringSelection(where: "front" | "back") {
  if (editor.selection.length) editor.updatePage((p) => reorder(p, editor.selection, where));
}

export function toggleLockSelection() {
  const ids = editor.selection;
  const lock = !editor.page.elements.filter((e) => ids.includes(e.id)).every((e) => e.locked);
  editor.updatePage((p) => ({ ...p, elements: p.elements.map((e) => (ids.includes(e.id) ? { ...e, locked: lock } : e)) }));
}

/** Les éléments verrouillés restent. */
export function deleteSelection() {
  const next = removeElements(editor.page, editor.selection);
  if (next !== editor.page) editor.updatePage(() => next);
}
