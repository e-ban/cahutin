import { isPlacedCell } from "./table";
import { editor } from "./docStore.svelte";
import { copySelection, cutSelection, deleteSelection, duplicateSelection, groupSelection, paste, ungroupSelection } from "./editActions";
import { newDoc, openDoc, print, save } from "./fileActions";
import { groupOf, isLockedSelection } from "./groups";
import { modal } from "./modal.svelte";
import { nudge, type ArrowKey } from "./geometry";
import { tour } from "./tour.svelte";

function typing(ev: KeyboardEvent): boolean {
  const t = ev.target as HTMLElement | null;
  return !!t && (t.isContentEditable || ["INPUT", "TEXTAREA", "SELECT"].includes(t.tagName));
}

export type ShortcutAction = "new" | "open" | "save" | "print" | "duplicate" | "undo" | "redo" | "copy" | "cut" | "paste" | "group" | "ungroup";

type KeyInfo = Pick<KeyboardEvent, "key" | "ctrlKey" | "metaKey" | "shiftKey" | "repeat">;

/** Raccourci Ctrl+touche reconnu ; une touche maintenue ne relance pas une commande de fichier. */
export function shortcutAction(ev: KeyInfo): ShortcutAction | null {
  if (!(ev.ctrlKey || ev.metaKey)) return null;
  const k = ev.key.toLowerCase();
  if (k === "z") return ev.shiftKey ? "redo" : "undo";
  if (k === "y") return "redo";
  if (k === "g") return ev.repeat ? null : ev.shiftKey ? "ungroup" : "group";
  if (ev.repeat) return null;
  const actions: Record<string, ShortcutAction> = { n: "new", o: "open", s: "save", p: "print", d: "duplicate", c: "copy", x: "cut", v: "paste" };
  return actions[k] ?? null;
}

/** Raccourcis globaux : fichiers, annuler, rétablir, supprimer, déplacer au clavier. */
export function handleShortcut(ev: KeyboardEvent) {
  // Une boîte de dialogue attend une réponse, ou la visite guidée est ouverte : aucun raccourci.
  if (modal.message !== null || tour.step !== null) return;
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
  // Sous l'écran d'accueil ou le panneau Astuces, la page n'est pas visible : rien ne doit la modifier.
  if (typing(ev) || editor.showWelcome || editor.showTips) return;
  const editActions: Partial<Record<ShortcutAction, () => void>> = {
    undo: () => editor.undo(),
    redo: () => editor.redo(),
    duplicate: duplicateSelection,
    copy: copySelection,
    cut: cutSelection,
    paste,
    group: groupSelection,
    ungroup: ungroupSelection,
  };
  if (action && editActions[action]) {
    ev.preventDefault();
    editActions[action]!();
  } else if (ev.key === "Delete" || ev.key === "Backspace") {
    if (editor.selection.length === 0) return;
    ev.preventDefault();
    deleteSelection();
  } else if (ev.key === "Escape" && editor.focusedMember) {
    // Retour au groupe de l'élément modifié seul.
    const id = editor.focusedMember;
    editor.focusedMember = null;
    editor.selection = groupOf(editor.page, id);
  } else if (ev.key.startsWith("Arrow")) {
    const ids = editor.selection;
    const inGrid = ids.some((id) => isPlacedCell(editor.page, id));
    if (ids.length === 0 || inGrid || isLockedSelection(editor.page, ids)) return;
    ev.preventDefault();
    editor.updatePage((p) => ({ ...p, elements: p.elements.map((el) => (ids.includes(el.id) ? { ...el, frame: nudge(el.frame, ev.key as ArrowKey, ev.shiftKey) } : el)) }));
  }
}
