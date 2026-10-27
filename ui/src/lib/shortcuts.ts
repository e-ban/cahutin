import { editor } from "./docStore.svelte";
import { nudge, type ArrowKey } from "./geometry";

function typing(ev: KeyboardEvent): boolean {
  const t = ev.target as HTMLElement | null;
  return !!t && (t.isContentEditable || ["INPUT", "TEXTAREA", "SELECT"].includes(t.tagName));
}

/** Raccourcis globaux : annuler, rétablir, supprimer, déplacer au clavier. */
export function handleShortcut(ev: KeyboardEvent) {
  if (typing(ev)) return;
  const ctrl = ev.ctrlKey || ev.metaKey;
  if (ctrl && ev.key.toLowerCase() === "z" && !ev.shiftKey) {
    ev.preventDefault();
    editor.undo();
  } else if (ctrl && (ev.key.toLowerCase() === "y" || (ev.key.toLowerCase() === "z" && ev.shiftKey))) {
    ev.preventDefault();
    editor.redo();
  } else if (ev.key === "Delete" || ev.key === "Backspace") {
    const ids = editor.selection.filter((id) => !editor.page.elements.find((e) => e.id === id)?.locked);
    if (ids.length === 0) return;
    ev.preventDefault();
    editor.updatePage((p) => ({ ...p, elements: p.elements.filter((e) => !ids.includes(e.id)) }));
  } else if (ev.key.startsWith("Arrow")) {
    const e = editor.selected;
    const inGrid = e && editor.page.grid?.cells.some((c) => c.element_id === e.id);
    if (!e || e.locked || inGrid) return;
    ev.preventDefault();
    editor.updateElement(e.id, (el) => ({ ...el, frame: nudge(el.frame, ev.key as ArrowKey, ev.shiftKey) }));
  }
}
