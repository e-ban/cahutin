import { isPlacedCell } from "./table";
import { editor } from "./docStore.svelte";
import { copySelection, cutSelection, deleteSelection, duplicateSelection, groupSelection, ungroupSelection } from "./editActions";
import { armPaste } from "./incoming";
import { newDoc, openDoc, print, save } from "./fileActions";
import { boundsOf, groupOf, isLockedSelection, selectAllIds } from "./groups";
import { modal } from "./modal.svelte";
import { nudgeOnPage, type ArrowKey } from "./geometry";
import { tour } from "./tour.svelte";
import { nextZoom } from "./units";

function typing(ev: KeyboardEvent): boolean {
  const t = ev.target as HTMLElement | null;
  return !!t && (t.isContentEditable || ["INPUT", "TEXTAREA", "SELECT"].includes(t.tagName));
}

export type ShortcutAction = "new" | "open" | "save" | "print" | "duplicate" | "undo" | "redo" | "copy" | "cut" | "paste" | "group" | "ungroup" | "selectAll" | "zoomIn" | "zoomOut" | "zoomFit" | "movePageBefore" | "movePageAfter";

type KeyInfo = Pick<KeyboardEvent, "key" | "ctrlKey" | "metaKey" | "shiftKey" | "repeat">;

/** Raccourci Ctrl+touche reconnu ; une touche maintenue ne relance pas une commande de fichier. */
export function shortcutAction(ev: KeyInfo): ShortcutAction | null {
  if (!(ev.ctrlKey || ev.metaKey)) return null;
  const k = ev.key.toLowerCase();
  // Ctrl+Maj+Pg.Préc. ou Pg.Suiv. : la page change de place (Pg.Préc. et Pg.Suiv. seuls changent de page).
  if (ev.shiftKey && (k === "pageup" || k === "pagedown")) return k === "pageup" ? "movePageBefore" : "movePageAfter";
  if (k === "z") return ev.shiftKey ? "redo" : "undo";
  if (k === "y") return "redo";
  if (k === "g") return ev.repeat ? null : ev.shiftKey ? "ungroup" : "group";
  // Zoom : Ctrl et +, -, 0 (« à » sur un clavier français), comme dans les autres logiciels.
  if (k === "+" || k === "=") return "zoomIn";
  if (k === "-") return "zoomOut";
  if (k === "0" || k === "à") return ev.repeat ? null : "zoomFit";
  // Maintenue, la touche choisit encore tout : la vue web ne surligne pas l'interface.
  if (k === "a") return "selectAll";
  if (ev.repeat) return null;
  const actions: Record<string, ShortcutAction> = { n: "new", o: "open", s: "save", p: "print", d: "duplicate", c: "copy", x: "cut", v: "paste" };
  return actions[k] ?? null;
}

// Sous l'écran d'accueil, le panneau Astuces, la classe, la photothèque ou la vue agrandie du tiroir Images, la
// page n'est pas visible : rien ne doit la modifier.
function pageHidden(): boolean {
  return editor.showWelcome || editor.showTips || editor.showClassPage || editor.showPhototheque || editor.showCollection || editor.showPreferences || editor.imagesWide;
}

// Une fenêtre, un menu ou une liste ouverts (ils se ferment eux-mêmes à Échap), ou un recadrage en cours.
function somethingToClose(): boolean {
  return !!editor.cropping || !!globalThis.document?.querySelector('[role="dialog"], [role="menu"], [role="listbox"]');
}

/** Ctrl+A : tous les éléments de la page affichée, ou du groupe où l'on est entré. */
function selectAll() {
  if (editor.cropping || editor.editingText) return;
  const inside = editor.inside;
  editor.focusedMember = null;
  editor.insideGroup = inside;
  editor.selection = selectAllIds(editor.page, inside, editor.templateMode);
}

/** Pixels de molette pour un cran de zoom : le pincement du pavé tactile envoie beaucoup de petits pas. */
export const WHEEL_ZOOM_STEP = 40;

/**
 * Ctrl+molette : ajoute ce pas au cumul et dit s'il faut zoomer (1), dézoomer (-1) ou attendre (0).
 * Molette vers le haut : on zoome. Un changement de sens repart de zéro.
 */
export function wheelZoomStep(acc: number, deltaY: number, deltaMode: number): { acc: number; dir: 1 | -1 | 0 } {
  const px = deltaMode === 1 ? deltaY * 40 : deltaMode === 2 ? deltaY * 800 : deltaY;
  const sum = Math.sign(px) === Math.sign(acc) ? acc + px : px;
  if (sum <= -WHEEL_ZOOM_STEP) return { acc: 0, dir: 1 };
  if (sum >= WHEEL_ZOOM_STEP) return { acc: 0, dir: -1 };
  return { acc: sum, dir: 0 };
}

let wheelAcc = 0;

/**
 * Ctrl+molette, n'importe où dans la fenêtre : zoom de la page. Renvoie le nouveau zoom, ou null.
 * La vue web ne zoome jamais toute l'interface (sa taille se règle dans les Préférences).
 */
export function handleWheelZoom(ev: WheelEvent): number | null {
  if (!(ev.ctrlKey || ev.metaKey)) return null;
  ev.preventDefault();
  if (modal.message !== null || tour.step !== null || pageHidden()) return null;
  const step = wheelZoomStep(wheelAcc, ev.deltaY, ev.deltaMode);
  wheelAcc = step.acc;
  if (step.dir === 0) return null;
  editor.zoom = nextZoom(editor.effectiveZoom, step.dir);
  return editor.zoom;
}

// Rafale de déplacements aux flèches : un seul pas d'annulation, comme un curseur que l'on fait glisser. Elle se
// termine au relâchement de la flèche, à un changement de sélection, à toute autre touche ou avant d'enregistrer.
let nudging: string[] | null = null;

export function endNudge() {
  if (!nudging) return;
  nudging = null;
  editor.endPreview();
}

editor.onFlush(endNudge);

export function handleKeyUp(ev: KeyboardEvent) {
  if (ev.key.startsWith("Arrow")) endNudge();
}

/** Raccourcis globaux : fichiers, annuler, rétablir, supprimer, déplacer au clavier. */
export function handleShortcut(ev: KeyboardEvent) {
  if (nudging && !(ev.key.startsWith("Arrow") && nudging.join() === editor.selection.join())) endNudge();
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
  if (typing(ev) || pageHidden()) return;
  const editActions: Partial<Record<ShortcutAction, () => void>> = {
    undo: () => editor.undo(),
    redo: () => editor.redo(),
    duplicate: duplicateSelection,
    copy: copySelection,
    cut: cutSelection,
    group: groupSelection,
    ungroup: ungroupSelection,
    selectAll,
    zoomIn: () => (editor.zoom = nextZoom(editor.effectiveZoom, 1)),
    zoomOut: () => (editor.zoom = nextZoom(editor.effectiveZoom, -1)),
    zoomFit: () => (editor.zoom = "fit"),
    movePageBefore: () => editor.movePage(editor.pageIndex, editor.pageIndex - 1),
    movePageAfter: () => editor.movePage(editor.pageIndex, editor.pageIndex + 1),
  };
  // Coller : sans empêcher la touche, la vue web envoie l'événement « paste » avec le presse-papiers du système.
  if (action === "paste") {
    armPaste();
    return;
  }
  if (action && editActions[action]) {
    ev.preventDefault();
    editActions[action]!();
  } else if ((ev.key === "PageDown" || ev.key === "PageUp") && !ev.ctrlKey && !ev.metaKey) {
    ev.preventDefault();
    const next = editor.pageIndex + (ev.key === "PageDown" ? 1 : -1);
    if (next < 0 || next >= editor.doc.pages.length) return;
    editor.pageIndex = next;
    editor.selection = [];
  } else if (ev.key === "Delete" || ev.key === "Backspace") {
    if (editor.selection.length === 0) return;
    ev.preventDefault();
    deleteSelection();
  } else if (ev.key === "Escape" && ev.defaultPrevented) {
    // Échap déjà pris par l'éditeur de texte, qui vient de fermer l'écriture (et a pu quitter la page avant que la
    // touche arrive ici) : l'élément reste sélectionné, l'Échap suivant remonte d'un niveau.
  } else if (ev.key === "Escape" && (editor.focusedMember || editor.inside.length)) {
    // Retour d'un niveau : de l'élément modifié seul à son groupe, d'un sous-groupe au groupe qui le contient.
    const id = editor.focusedMember ?? editor.selection[0];
    editor.focusedMember = null;
    editor.insideGroup = editor.inside.slice(0, -1);
    editor.selection = groupOf(editor.page, id, editor.insideGroup);
  } else if (ev.key === "Escape" && editor.selection.length && !editor.editingText && !somethingToClose()) {
    // Plus rien à quitter : Échap désélectionne, comme un clic sur la page vide.
    editor.selection = [];
    editor.insideGroup = [];
  } else if (ev.key.startsWith("Arrow")) {
    const ids = editor.selection;
    const inGrid = ids.some((id) => isPlacedCell(editor.page, id));
    if (ids.length === 0 || inGrid || isLockedSelection(editor.page, ids, editor.inside)) return;
    ev.preventDefault();
    // La sélection bouge d'un bloc, et s'arrête avant de quitter la page.
    const box = boundsOf(editor.page.elements.filter((el) => ids.includes(el.id)).map((el) => el.frame));
    const { dx, dy } = nudgeOnPage(box, editor.page.size, ev.key as ArrowKey, ev.shiftKey);
    if (dx === 0 && dy === 0) return;
    nudging = [...ids];
    const pages = editor.doc.pages.map((p, i) => (i === editor.pageIndex ? { ...p, elements: p.elements.map((el) => (ids.includes(el.id) ? { ...el, frame: { ...el.frame, x_mm: el.frame.x_mm + dx, y_mm: el.frame.y_mm + dy } } : el)) } : p));
    editor.preview({ ...editor.doc, pages });
  }
}
