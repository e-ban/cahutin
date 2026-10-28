import { editor } from "./docStore.svelte";
import { copySelection, cutSelection, deleteSelection, duplicateSelection, paste } from "./editActions";
import { newDoc, openDoc, print, save } from "./fileActions";
import { nudge, type ArrowKey } from "./geometry";

function typing(ev: KeyboardEvent): boolean {
  const t = ev.target as HTMLElement | null;
  return !!t && (t.isContentEditable || ["INPUT", "TEXTAREA", "SELECT"].includes(t.tagName));
}

export type ShortcutAction = "new" | "open" | "save" | "print" | "duplicate" | "undo" | "redo" | "copy" | "cut" | "paste";

type KeyInfo = Pick<KeyboardEvent, "key" | "ctrlKey" | "metaKey" | "shiftKey" | "repeat">;

/** Raccourci Ctrl+touche reconnu ; une touche maintenue ne relance pas une commande de fichier. */
export function shortcutAction(ev: KeyInfo): ShortcutAction | null {
  if (!(ev.ctrlKey || ev.metaKey)) return null;
  const k = ev.key.toLowerCase();
  if (k === "z") return ev.shiftKey ? "redo" : "undo";
  if (k === "y") return "redo";
  if (ev.repeat) return null;
  const actions: Record<string, ShortcutAction> = { n: "new", o: "open", s: "save", p: "print", d: "duplicate", c: "copy", x: "cut", v: "paste" };
  return actions[k] ?? null;
}

/** Raccourcis globaux : fichiers, annuler, rétablir, supprimer, déplacer au clavier. */
export function handleShortcut(ev: KeyboardEvent) {
  const action = shortcutAction(ev);
  // Commandes de fichier : même pendant la saisie d'un texte (le texte en cours est validé avant).
  const fileActions: Partial<Record<ShortcutAction, () => unknown>> = { new: newDoc, open: openDoc, save, print };
  if (action && fileActions[action]) {
    ev.preventDefault();
    fileActions[action]!();
    return;
  }
  // Touche maintenue : ne pas laisser passer l'action du navigateur (enregistrer la page, imprimer…).
  if ((ev.ctrlKey || ev.metaKey) && ["n", "o", "s", "p"].includes(ev.key.toLowerCase())) {
    ev.preventDefault();
    return;
  }
  if (typing(ev)) return;
  const editActions: Partial<Record<ShortcutAction, () => void>> = {
    undo: () => editor.undo(),
    redo: () => editor.redo(),
    duplicate: duplicateSelection,
    copy: copySelection,
    cut: cutSelection,
    paste,
  };
  if (action && editActions[action]) {
    ev.preventDefault();
    editActions[action]!();
  } else if (ev.key === "Delete" || ev.key === "Backspace") {
    if (editor.selection.length === 0) return;
    ev.preventDefault();
    deleteSelection();
  } else if (ev.key.startsWith("Arrow")) {
    const e = editor.selected;
    const inGrid = e && editor.page.grid?.cells.some((c) => c.element_id === e.id);
    if (!e || e.locked || inGrid) return;
    ev.preventDefault();
    editor.updateElement(e.id, (el) => ({ ...el, frame: nudge(el.frame, ev.key as ArrowKey, ev.shiftKey) }));
  }
}
