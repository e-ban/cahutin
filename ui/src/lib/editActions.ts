// Actions d'édition sur la sélection, communes aux raccourcis, au clic droit et à la barre flottante.
import { copyElements, duplicateElements, pasteElements, removeElements, reorder } from "./clipboard";
import { editor } from "./docStore.svelte";
import { expandSelection, groupElements, rotateElements, scaleElements, ungroup } from "./groups";
import { newId } from "./factory";
import type { AssetMeta, Element } from "./types";

let clipboard: Element[] = [];
// Images des photos copiées : collées dans un autre document, elles doivent rejoindre ses images.
let clipboardAssets: Record<string, AssetMeta> = {};

export const hasClipboard = () => clipboard.length > 0;

function assetsOf(elements: Element[]): Record<string, AssetMeta> {
  const out: Record<string, AssetMeta> = {};
  for (const e of elements) {
    const meta = e.kind.type === "Photo" && e.kind.asset ? editor.assetMeta(e.kind.asset) : undefined;
    if (meta) out[meta.id] = meta;
  }
  return out;
}

/** La sélection, groupes complétés. */
const selected = () => expandSelection(editor.page, editor.selection);

export function copySelection() {
  if (editor.selection.length === 0) return;
  clipboard = copyElements(editor.page, selected());
  clipboardAssets = assetsOf(clipboard);
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

export function bringSelection(where: "front" | "back") {
  if (editor.selection.length) editor.updatePage((p) => reorder(p, editor.selection, where));
}

export function toggleLockSelection() {
  const ids = selected();
  const lock = !editor.page.elements.filter((e) => ids.includes(e.id)).every((e) => e.locked);
  editor.updatePage((p) => ({ ...p, elements: p.elements.map((e) => (ids.includes(e.id) ? { ...e, locked: lock } : e)) }));
}

/** Les éléments verrouillés restent. */
export function deleteSelection() {
  const next = removeElements(editor.page, selected());
  if (next !== editor.page) editor.updatePage(() => next);
}

/** Grouper la sélection (Ctrl+G) : un seul pas d'annulation. */
export function groupSelection() {
  const { page, group } = groupElements(editor.page, editor.selection);
  if (!group) return;
  editor.updatePage(() => page);
  editor.selection = page.elements.filter((e) => e.group === group).map((e) => e.id);
}

/** Dissocier (Ctrl+Maj+G) : les éléments restent sélectionnés. */
export function ungroupSelection() {
  if (!editor.page.elements.some((e) => editor.selection.includes(e.id) && e.group)) return;
  const ids = selected();
  editor.updatePage((p) => ungroup(p, ids));
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

export const NEW_PHOTO_WIDTH_MM = 80;

/** Nouvelle photo libre centrée sur (x, y), de 80 mm de large ; l'image rejoint celles du document. */
export function addPhotoAt(meta: AssetMeta, xMm: number, yMm: number) {
  const w = NEW_PHOTO_WIDTH_MM;
  const h = Math.round(((w * meta.height_px) / meta.width_px) * 10) / 10;
  const id = newId();
  const el: Element = {
    id,
    frame: { x_mm: Math.round((xMm - w / 2) * 10) / 10, y_mm: Math.round((yMm - h / 2) * 10) / 10, w_mm: w, h_mm: h, rotation_deg: 0 },
    locked: false,
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

/** Clic sur une vignette : remplit (ou remplace) la photo sélectionnée, sinon pose la photo au centre de la page. */
export function usePhoto(meta: AssetMeta) {
  const e = editor.selected;
  if (e && e.kind.type === "Photo" && !e.locked) {
    editor.placePhoto(e.id, meta);
    return;
  }
  addPhotoAt(meta, editor.page.size.width_mm / 2, editor.page.size.height_mm / 2);
}
