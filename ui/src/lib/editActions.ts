// Actions d'édition sur la sélection, communes aux raccourcis, au clic droit et à la barre flottante.
// Copié en partie pour l'assistant dans crates/cahutin-mcp/src/decor.rs (légende d'une photo) : changer les deux ensemble.
import { warnConsent } from "./phototheque.svelte";
import { copyElements, duplicateElements, pasteElements, removeElements, reorder, reorderStep } from "./clipboard";
import { editor } from "./docStore.svelte";
import { markCopy, plainTextOf } from "./systemClipboard";
import { boundsOf, existingIds, flipped, expandSelection, groupElements, pathOf, isLockedSelection, mirrorElements, rotateElements, scaleElements, ungroup, withPath } from "./groups";
import { newId } from "./factory";
import { elementAssets, usedAssetIds } from "./photoLibrary";
import { ask } from "./ask";
import { fr } from "./i18n/fr";
import { textFrame } from "./elementEdit";
import { tableDissolve, tableSetCell } from "./api";
import { clearTextCell, inTableGroup, isTableCell } from "./table";
import type { AssetMeta, Background, Element, Frame, Page } from "./types";

let clipboard: Element[] = [];
// Images des photos copiées : collées dans un autre document, elles doivent rejoindre ses images.
let clipboardAssets: Record<string, AssetMeta> = {};

export const hasClipboard = () => clipboard.length > 0;

function assetsOf(elements: Element[]): Record<string, AssetMeta> {
  const out: Record<string, AssetMeta> = {};
  for (const id of elements.flatMap(elementAssets)) {
    const meta = editor.assetMeta(id);
    if (meta) out[meta.id] = meta;
  }
  return out;
}

/** La sélection, groupes complétés ; l'élément modifié seul après un double-clic reste seul. */
const selected = () => (editor.focusedMember ? editor.selection : expandSelection(editor.page, editor.selection, editor.inside));

export function copySelection() {
  if (editor.selection.length === 0) return;
  clipboard = copyElements(editor.page, selected());
  clipboardAssets = assetsOf(clipboard);
  markCopy(plainTextOf(clipboard));
}

export function cutSelection() {
  copySelection();
  deleteSelection();
}

export function paste() {
  const { page, ids } = pasteElements(editor.page, clipboard);
  if (ids.length === 0) return;
  editor.commit({
    ...editor.doc,
    assets: { ...clipboardAssets, ...editor.doc.assets },
    pages: editor.doc.pages.map((p, i) => (i === editor.pageIndex ? page : p)),
  });
  editor.selection = ids;
  // Coller encore décale encore : chaque collage part du dernier.
  clipboard = copyElements(page, ids);
}

export function duplicateSelection() {
  if (editor.selection.length === 0) return;
  const { page, ids } = duplicateElements(editor.page, selected());
  editor.updatePage(() => page);
  editor.selection = ids;
}

export function bringSelection(where: "front" | "back" | "forward" | "backward") {
  if (!editor.selection.length) return;
  if (where === "front" || where === "back") editor.updatePage((p) => reorder(p, editor.selection, where));
  // Déjà tout devant ou tout derrière : rien ne bouge, et pas d'étape d'annulation vide.
  else if (canBringSelection(where)) editor.updatePage((p) => reorderStep(p, editor.selection, where, editor.inside));
}

/** Avancer ou reculer d'un cran changerait l'ordre (sinon le bouton est grisé). */
export function canBringSelection(where: "forward" | "backward"): boolean {
  return editor.selection.length > 0 && reorderStep(editor.page, editor.selection, where, editor.inside) !== editor.page;
}

/** Toute la sélection est verrouillée : le bouton propose de la déverrouiller (barre et Réglages). */
export function selectionLocked(): boolean {
  const ids = selected();
  return editor.page.elements.filter((e) => ids.includes(e.id)).every((e) => e.locked);
}

/** Verrouille la sélection, ou la déverrouille si elle l'est toute (voir `selectionLocked`). */
export function toggleLockSelection() {
  const ids = selected();
  const lock = !selectionLocked();
  editor.updatePage((p) => ({ ...p, elements: p.elements.map((e) => (ids.includes(e.id) ? { ...e, locked: lock } : e)) }));
}

/** Les éléments verrouillés restent. Une case de tableau modifiée seule est vidée, pas retirée : on retire
 *  sa ligne ou sa colonne. */
export function deleteSelection() {
  const ids = selected();
  const cell = ids.length === 1 && isTableCell(editor.page, ids[0]) ? editor.page.elements.find((e) => e.id === ids[0]) : undefined;
  if (cell) {
    if (cell.locked) return;
    if (cell.kind.type === "Text") editor.updatePage((p) => clearTextCell(p, cell.id));
    else runOnPage((p) => tableSetCell(p, cell.id, "Text"));
    return;
  }
  const next = removeElements(editor.page, ids, editor.templateMode);
  if (next !== editor.page) editor.updatePage(() => next);
  // Rien n'est parti à cause d'un verrou : on dit pourquoi, et où l'enlever.
  else if (!editor.templateMode && editor.page.elements.some((e) => ids.includes(e.id) && e.locked)) editor.notice = fr.lockedNoDelete;
}

/** Opération du moteur sur la page courante ; son erreur s'affiche dans le bandeau. */
export async function runOnPage(op: (p: Page) => Promise<Page>) {
  const id = editor.page.id;
  try {
    const page = await op(editor.page);
    editor.updatePageById(id, () => page);
    // Une case retirée (ligne ou colonne) ne reste pas sélectionnée.
    if (editor.page.id === id) {
      editor.selection = existingIds(page, editor.selection);
      if (editor.focusedMember && !editor.selection.includes(editor.focusedMember)) editor.focusedMember = null;
    }
  } catch (e) {
    editor.notice = String(e);
  }
}

/** Tourne la sélection d'un quart de tour (`deg` : 90 à droite, -90 à gauche), un groupe autour de son centre ;
 *  rien si un élément est verrouillé, dans une grille ou dans un tableau. */
export function rotateSelection(deg: number) {
  const ids = selected();
  const page = editor.page;
  if (!ids.length || isLockedSelection(page, ids, editor.inside) || ids.some((id) => inTableGroup(page, id) || !!page.grid?.cells.some((c) => c.element_id === id))) return;
  const els = page.elements.filter((e) => ids.includes(e.id));
  const b = boundsOf(els.map((e) => e.frame));
  const turned = new Map(rotateElements(els, deg, { x_mm: b.x_mm + b.w_mm / 2, y_mm: b.y_mm + b.h_mm / 2 }).map((e) => [e.id, e]));
  editor.updatePage((p) => ({ ...p, elements: p.elements.map((e) => turned.get(e.id) ?? e) }));
}

/** Miroir de la sélection, de gauche à droite (`h`) ou de haut en bas (`v`) : un élément se retourne dans son
 *  cadre, un groupe tout entier autour de son centre ; mêmes limites que `rotateSelection`. */
export function mirrorSelection(axis: "h" | "v") {
  const ids = selected();
  const page = editor.page;
  if (!ids.length || isLockedSelection(page, ids, editor.inside) || ids.some((id) => inTableGroup(page, id) || !!page.grid?.cells.some((c) => c.element_id === id))) return;
  const els = page.elements.filter((e) => ids.includes(e.id));
  const b = boundsOf(els.map((e) => e.frame));
  const done = els.length === 1 ? [flipped(els[0], axis)] : mirrorElements(els, axis, { x_mm: b.x_mm + b.w_mm / 2, y_mm: b.y_mm + b.h_mm / 2 });
  const byId = new Map(done.map((e) => [e.id, e]));
  editor.updatePage((p) => ({ ...p, elements: p.elements.map((e) => byId.get(e.id) ?? e) }));
}

/** Grouper la sélection (Ctrl+G) : un seul pas d'annulation. */
export function groupSelection() {
  const { page, group } = groupElements(editor.page, editor.selection, editor.inside);
  if (!group) return;
  editor.updatePage(() => page);
  editor.focusedMember = null;
  editor.selection = page.elements.filter((e) => pathOf(e).includes(group)).map((e) => e.id);
}

/** Dissocier (Ctrl+Maj+G) : les éléments restent sélectionnés. */
export function ungroupSelection() {
  if (!editor.page.elements.some((e) => editor.selection.includes(e.id) && e.group)) return;
  // Un tableau dissocié rend ses cases et ses traits, devenus des formes « trait » ; les autres groupes de
  // la sélection sont dissous comme d'habitude.
  const ids = selected();
  const tables = editor.page.elements.filter((e) => e.kind.type === "Table" && expandSelection(editor.page, ids).includes(e.id));
  if (tables.length) {
    const groups = new Set(tables.map((t) => t.group));
    const others = ids.filter((id) => !inTableGroup(editor.page, id));
    runOnPage(async (p) => {
      for (const t of tables) p = await tableDissolve(p, t.id);
      return ungroup(p, others);
    }).then(() => {
      editor.selection = editor.page.elements.filter((e) => ids.includes(e.id) || (e.group && groups.has(e.group))).map((e) => e.id);
    });
    return;
  }
  const inside = editor.inside;
  editor.updatePage((p) => ungroup(p, ids, inside));
  editor.selection = ids;
}

export interface GroupTransform {
  dx_mm: number;
  dy_mm: number;
  /** Agrandissement proportionnel depuis `origin`. */
  factor: number;
  origin: { x_mm: number; y_mm: number };
  /** Rotation autour de `center` (sens horaire). */
  deg: number;
  center?: { x_mm: number; y_mm: number };
}

/** Fin d'un geste sur plusieurs éléments (groupe ou sélection) : un seul pas d'annulation. */
export function applyGroupTransform(ids: string[], t: GroupTransform) {
  editor.updatePage((p) => {
    let moved = p.elements.filter((e) => ids.includes(e.id));
    if (t.factor !== 1) moved = scaleElements(moved, t.factor, t.origin);
    if (t.deg !== 0 && t.center) moved = rotateElements(moved, t.deg, t.center);
    if (t.dx_mm !== 0 || t.dy_mm !== 0) {
      const r = (v: number) => Math.round(v * 10) / 10;
      moved = moved.map((e) => ({ ...e, frame: { ...e.frame, x_mm: r(e.frame.x_mm + t.dx_mm), y_mm: r(e.frame.y_mm + t.dy_mm) } }));
    }
    const byId = new Map(moved.map((e) => [e.id, e]));
    return { ...p, elements: p.elements.map((e) => byId.get(e.id) ?? e) };
  });
}

/** Valeur tapée dans le panneau de droite ; une image de la banque reste carrée. Avec `keepRatio`, changer la
 *  largeur change aussi la hauteur (et l'inverse), aux proportions de l'élément. */
export function setSelectedFrame(key: keyof Frame, value: number, keepRatio = false) {
  const e = editor.selected;
  if (!e || !Number.isFinite(value)) return;
  const min = key === "w_mm" || key === "h_mm" ? 1 : -10000;
  const v = Math.max(min, Math.round(value * 10) / 10);
  const r = (n: number) => Math.max(1, Math.round(n * 10) / 10);
  const ratio = e.kind.type === "Image" ? 1 : e.frame.h_mm / e.frame.w_mm;
  const linked = keepRatio || e.kind.type === "Image";
  editor.updateElement(e.id, (el) => {
    const frame = { ...el.frame, [key]: v };
    if (linked && key === "w_mm") frame.h_mm = r(v * ratio);
    if (linked && key === "h_mm") frame.w_mm = r(v / ratio);
    return { ...el, frame };
  });
}

/** Taille d'une sélection de plusieurs éléments (un groupe), à ses proportions : tout s'agrandit depuis le coin
 *  haut gauche de l'ensemble, textes et traits compris. */
export function setSelectionSize(key: "w_mm" | "h_mm", value: number) {
  const ids = selected();
  const page = editor.page;
  if (ids.length < 2 || !Number.isFinite(value) || value < 1 || isLockedSelection(page, ids, editor.inside)) return;
  const b = boundsOf(page.elements.filter((e) => ids.includes(e.id)).map((e) => e.frame));
  const factor = value / (key === "w_mm" ? b.w_mm : b.h_mm);
  if (!Number.isFinite(factor) || Math.abs(factor - 1) < 0.001) return;
  applyGroupTransform(ids, { dx_mm: 0, dy_mm: 0, factor, origin: { x_mm: b.x_mm, y_mm: b.y_mm }, deg: 0 });
}

/** Fond de la page courante (ou de toutes les pages) : un seul pas d'annulation. */
export function setBackground(bg: Background | null, allPages: boolean) {
  editor.commit({
    ...editor.doc,
    pages: editor.doc.pages.map((p, i) => (allPages || i === editor.pageIndex ? { ...p, background: bg && { ...bg } } : p)),
  });
}

export const CAPTION_GAP_MM = 2;
export const CAPTION_HEIGHT_MM = 10;

/**
 * Légende de la photo : un texte juste sous elle (même après un agrandissement du groupe) qui la
 * recouvre en largeur ; du même groupe, ou, pour une photo sans groupe (case de grille), une légende
 * posée par ce bouton (invite « Légende… »).
 */
function captionOf(photo: Element): Element | undefined {
  // La photo telle qu'elle s'affiche : une photo tournée n'occupe pas son cadre d'origine.
  const f = boundsOf([photo.frame]);
  const bottom = f.y_mm + f.h_mm;
  return editor.page.elements.find((e) => {
    if (e.kind.type !== "Text" || e.id === photo.id) return false;
    const sameOwner = photo.group ? pathOf(e).join("/") === pathOf(photo).join("/") : !e.group && e.kind.prompt === CAPTION_PROMPT;
    const below = e.frame.y_mm >= bottom - CAPTION_HEIGHT_MM - CAPTION_GAP_MM && e.frame.y_mm <= bottom + 2 * CAPTION_HEIGHT_MM;
    const overlap = Math.min(e.frame.x_mm + e.frame.w_mm, f.x_mm + f.w_mm) - Math.max(e.frame.x_mm, f.x_mm);
    return sameOwner && below && overlap > e.frame.w_mm / 2;
  });
}

const CAPTION_PROMPT = "Légende…";

/**
 * Légende d'une photo : un texte sous la photo, groupé avec elle (dans son groupe s'il existe), ouvert
 * à l'écriture. S'il y en a déjà une, elle est sélectionnée. Une case de grille n'est jamais groupée.
 */
export function addCaption(photoId: string) {
  const photo = editor.page.elements.find((e) => e.id === photoId);
  if (!photo || photo.kind.type !== "Photo") return;
  const existing = captionOf(photo);
  if (existing) {
    editor.focusedMember = existing.id;
    editor.selection = [existing.id];
    return;
  }
  const inGrid = !!editor.page.grid?.cells.some((c) => c.element_id === photo.id);
  // La légende rejoint le plus petit groupe de la photo (tout son chemin), ou un groupe nouveau avec elle.
  const path = inGrid ? [] : photo.group ? pathOf(photo) : [newId()];
  const group = path[0] ?? null;
  // Sous la photo telle qu'elle s'affiche (tournée, elle n'occupe pas son cadre d'origine), aussi large qu'elle.
  const f = boundsOf([photo.frame]);
  // Toujours sur la page (marge de 2 mm en bas), quitte à mordre sur le bas de la photo.
  const maxY = editor.page.size.height_mm - CAPTION_HEIGHT_MM - CAPTION_GAP_MM;
  const caption: Element = withPath({
    id: newId(),
    frame: { x_mm: Math.round(f.x_mm * 10) / 10, y_mm: Math.round(Math.min(f.y_mm + f.h_mm + CAPTION_GAP_MM, maxY) * 10) / 10, w_mm: Math.round(f.w_mm * 10) / 10, h_mm: CAPTION_HEIGHT_MM, rotation_deg: 0 },
    locked: false,
    zone: false,
    kind: { type: "Text", content: { paragraphs: [] }, style: { font: "Andika", size_pt: 12, color: "#333333", align: "Center" }, prompt: CAPTION_PROMPT },
  }, path);
  editor.updatePage((p) => {
    const elements = p.elements.map((e) => (e.id === photo.id && group && !e.group ? withPath(e, path) : e));
    // Groupée : juste après sa photo (le groupe reste d'un seul tenant). Case de grille : au-dessus
    // de toutes les cases, sinon la case du dessous la recouvrirait.
    if (group) elements.splice(p.elements.findIndex((e) => e.id === photo.id) + 1, 0, caption);
    else elements.push(caption);
    return { ...p, elements };
  });
  editor.focusedMember = group ? caption.id : null;
  editor.selection = [caption.id];
  editor.editingText = caption.id;
}

export const NEW_PHOTO_WIDTH_MM = 80;

/** Nouvelle photo libre centrée sur (x, y), de 80 mm de large ; l'image rejoint celles du document. Dans un
 *  modèle, elle est fixe (un logo, un dessin), ce qui se change dans sa barre. */
export function addPhotoAt(meta: AssetMeta, xMm: number, yMm: number) {
  const w = NEW_PHOTO_WIDTH_MM;
  const h = Math.round(((w * meta.height_px) / meta.width_px) * 10) / 10;
  const id = newId();
  const el: Element = {
    id,
    frame: { x_mm: Math.round((xMm - w / 2) * 10) / 10, y_mm: Math.round((yMm - h / 2) * 10) / 10, w_mm: w, h_mm: h, rotation_deg: 0 },
    locked: editor.templateMode,
    zone: false,
    kind: { type: "Photo", asset: meta.id, crop: { scale: 1, center_x: 0.5, center_y: 0.5 }, corner_radius_mm: 0, border: null, shadow: false },
  };
  editor.commit({
    ...editor.doc,
    assets: { ...editor.doc.assets, [meta.id]: meta },
    pages: editor.doc.pages.map((p, i) => (i === editor.pageIndex ? { ...p, elements: [...p.elements, el] } : p)),
  });
  editor.selection = [id];
}

/** Plusieurs images d'un coup (vue agrandie du tiroir Images) : la première remplit la case choisie (ou
 *  remplace la photo, après « Remplacer ») ; les suivantes sont posées à côté, sans rien remplacer. Un seul
 *  avis pour les enfants sans autorisation photo de toutes ces images. */
export function useImages(metas: AssetMeta[]) {
  metas.forEach((m, i) => {
    usePhoto(m, { warn: false });
    if (i === 0) {
      editor.selection = [];
      editor.replacingPhoto = false;
    }
  });
  warnConsent(metas.map((m) => m.id));
}

/** Image de fond du cadre du texte sélectionné (une photo de la photothèque), ou aucune. */
export function setFrameImage(meta: AssetMeta | null) {
  const e = editor.selected;
  if (!e || e.kind.type !== "Text") return;
  const page = { ...editor.page, elements: editor.page.elements.map((x) => (x.id === e.id ? textFrame({ image: meta?.id ?? null })(x) : x)) };
  const pages = editor.doc.pages.map((p, i) => (i === editor.pageIndex ? page : p));
  editor.commit({ ...editor.doc, pages, assets: meta ? { ...editor.doc.assets, [meta.id]: meta } : editor.doc.assets });
}

/** Clic sur une vignette du tiroir Photos : remplit la case photo choisie (ou remplace la photo, après
 *  « Remplacer »), sinon pose la photo au centre de la page, à ses proportions. */
export function usePhoto(meta: AssetMeta, { warn = true }: { warn?: boolean } = {}) {
  if (warn) warnConsent(meta.id);
  const e = editor.selected;
  // Une case photo à remplir (vide, ou case d'un modèle) se remplit ; une photo déjà posée n'est remplacée que sur
  // demande (« Remplacer ») : sinon un clic pose une photo de plus.
  const fills = e?.kind.type === "Photo" && (e.zone || !e.kind.asset || editor.replacingPhoto);
  if (e && e.kind.type === "Photo" && !e.locked && fills) {
    editor.placePhoto(e.id, meta);
    return;
  }
  // Clic après clic, chaque photo est un peu décalée : posée sur la précédente, on ne la verrait pas.
  let [x, y] = [editor.page.size.width_mm / 2, editor.page.size.height_mm / 2];
  const taken = (x: number, y: number) => editor.page.elements.some((e) => Math.abs(e.frame.x_mm + e.frame.w_mm / 2 - x) < 1 && Math.abs(e.frame.y_mm + e.frame.h_mm / 2 - y) < 1);
  for (let i = 0; i < 20 && taken(x, y); i++) [x, y] = [x + 8, y + 8];
  addPhotoAt(meta, x, y);
}

/** Les photos du panneau posées nulle part dans le document. */
export function unusedPhotoIds(): Set<string> {
  const used = usedAssetIds(editor.doc);
  return new Set(editor.photos.filter((p) => !used.has(p.meta.id)).map((p) => p.meta.id));
}

/** Combien de photos non placées retirerait `askRemoveUnusedPhotos(only)`. */
export function unusedPhotoCountIn(only?: Set<string>): number {
  const ids = unusedPhotoIds();
  return only ? [...ids].filter((id) => only.has(id)).length : ids.size;
}

/** Retire du panneau les photos non placées (`only` : la vue d'un dossier, seulement celles-là), après
 *  confirmation ; « Annuler » les remet. Vrai si retirées. */
export async function askRemoveUnusedPhotos(only?: Set<string>): Promise<boolean> {
  const n = unusedPhotoCountIn(only);
  if (!n || !(await ask(fr.photosRemoveUnusedConfirm(n), fr.photosRemoveUnused(n)))) return false;
  const gone = editor.removeUnusedPhotos(only);
  editor.notify(fr.photosRemovedUnused(gone.photos.length), { label: fr.undo, run: () => editor.restorePhotos(gone) });
  return true;
}
