// Modifications de l'élément sélectionné, partagées par la barre flottante et le panneau de droite.
import { editor } from "./docStore.svelte";
import { TEXT_PADDING_MM } from "./richtext";
import { endsOf, isLineTarget, normalizeEnds, withHitHeight } from "./lineEnds";
import { fromPagePoints, isPath, pagePoints, removePoint, toShapePath, type Pt } from "./linePath";
import type { Element, HeadSize, LineEnd, LineEnds, PhotoProps, ShapeProps, TableProps, TextFrame, TextStyle } from "./types";

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
/** Champs absents plutôt que `undefined` : le document reste le même qu'avant l'ajout des chemins. */
function clean<T extends object>(o: T): T {
  return Object.fromEntries(Object.entries(o).filter(([, v]) => v !== undefined)) as T;
}

/** Un trait aux réglages `kind` qui passe par `pts` (mm de page) : chemin ou trait droit, sans rotation ni miroir.
 *  Ne touche pas l'historique (les poignées s'en servent pour l'aperçu). */
export function throughPoints(e: Element, kind: ShapeProps, pts: Pt[]): Element {
  const r = fromPagePoints(pts, kind);
  return { ...e, frame: r.frame, flip_h: false, flip_v: false, kind: { type: "Shape", ...clean({ ...kind, ...r.patch }) } };
}

/** Un trait dont les réglages deviennent `kind` : sa zone cliquable suit ses pointes ; un chemin garde ses points
 *  en place (son cadre est recalculé, pas étiré). */
function refit(e: Element, kind: ShapeProps): Element {
  if (e.kind.type !== "Shape") return e;
  if (isPath(kind)) return throughPoints(e, kind, pagePoints(e.frame, e.kind, !!e.flip_h, !!e.flip_v));
  return { ...e, frame: withHitHeight(e.frame, kind), kind: { type: "Shape", ...kind } };
}

const shape = (patch: Partial<ShapeProps>) => (e: Element): Element => {
  if (e.kind.type !== "Shape") return e;
  const kind = { ...e.kind, ...patch };
  // Un trait plus épais a des pointes plus grandes : sa zone cliquable les suit.
  return isLineTarget(e) ? refit(e, kind) : { ...e, kind };
};

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
  const f = { fill: null, stroke: null, corner_radius_mm: 0, image: null, pattern: null, ...e.kind.frame, ...patch };
  const empty = !f.fill && !f.stroke && !f.image && !f.pattern;
  return { ...e, kind: { ...e.kind, frame: empty ? null : f } };
};

export const updateTextStyle = (patch: Partial<TextStyle>) => updateSelected(textStyle(patch));
export const previewTextStyle = (patch: Partial<TextStyle>) => previewSelected(textStyle(patch));
export const updateTextFrame = (patch: Partial<TextFrame>) => updateSelected(textFrame(patch));
export const previewTextFrame = (patch: Partial<TextFrame>) => previewSelected(textFrame(patch));

/** Applique `fn` à chaque trait ou flèche sélectionné, en un seul pas d'annulation ; les autres éléments ne changent pas. */
export function updateLines(fn: (e: Element) => Element) {
  const ids = new Set(editor.selection);
  editor.updatePage((p) => ({ ...p, elements: p.elements.map((e) => (ids.has(e.id) && isLineTarget(e) ? fn(e) : e)) }));
}

/** Terminaisons d'un trait changées par `patch` : enregistrées seulement si elles diffèrent de son genre, et le cadre
 *  assez haut pour la plus grande. */
const lineEnds = (patch: (ends: LineEnds) => LineEnds) => (e: Element): Element => {
  if (e.kind.type !== "Shape") return e;
  return refit(e, { ...e.kind, ends: normalizeEnds(e.kind.shape, patch(endsOf(e.kind))) ?? undefined });
};

export const setLineEnd = (which: "start" | "end", end: LineEnd) => updateLines(lineEnds((x) => ({ ...x, [which]: end })));
export const setHeadSize = (size: HeadSize) => updateLines(lineEnds((x) => ({ ...x, size })));
/** Les terminaisons changent de bout ; le trait (ou le chemin) ne bouge pas. */
export const swapLineEnds = () => updateLines(lineEnds((x) => ({ ...x, start: x.end, end: x.start })));

/** Le trait `id` passe désormais par `pts` (mm de page), en un pas d'annulation : deux points, un trait droit. */
export function setPathPoints(id: string, pts: Pt[]) {
  editor.updateElement(id, (e) => (e.kind.type === "Shape" ? throughPoints(e, e.kind, pts) : e));
}

/** Retire le point `i` du trait `id` (rien sur un trait droit). */
export function removePathPoint(id: string, i: number) {
  const e = editor.page.elements.find((o) => o.id === id);
  if (!e || e.kind.type !== "Shape" || !isPath(e.kind)) return;
  setPathPoints(id, removePoint(pagePoints(e.frame, e.kind, !!e.flip_h, !!e.flip_v), i));
}

/** Les chemins sélectionnés deviennent des formes ordinaires (tracé libre) : elles se redimensionnent et se tournent
 *  comme un rectangle, mais leurs points ne se tirent plus. Un chemin fermé garde son fond ; un chemin ouvert perd
 *  ses pointes. Un pas d'annulation. */
export function convertToShape() {
  updateLines((e) => {
    if (e.kind.type !== "Shape" || !isPath(e.kind)) return e;
    const k = e.kind;
    const out = toShapePath(e.frame, k, !!e.flip_h, !!e.flip_v);
    return { ...e, frame: out.frame, flip_h: false, flip_v: false, kind: { type: "Shape", shape: "Path", fill: k.closed ? k.fill : null, stroke: k.stroke, corner_radius_mm: 0, path: out.path } };
  });
}

/** Ferme (ou rouvre) les chemins sélectionnés : un segment revient du dernier point au premier ; fermé, un chemin
 *  n'a plus de terminaisons et `fill` le remplit. Sans effet ni pas d'annulation sur des traits droits. */
export function setClosed(on: boolean) {
  const ids = new Set(editor.selection);
  if (!editor.page.elements.some((e) => ids.has(e.id) && e.kind.type === "Shape" && isPath(e.kind) && !!e.kind.closed !== on)) return;
  updateLines((e) => (e.kind.type === "Shape" && isPath(e.kind) ? { ...e, kind: clean({ ...e.kind, closed: on || undefined }) } : e));
}

/** Adoucit (ou redonne leurs angles à) les chemins sélectionnés ; sans effet ni pas d'annulation sur des traits droits. */
export function setCurve(on: boolean) {
  const ids = new Set(editor.selection);
  if (!editor.page.elements.some((e) => ids.has(e.id) && e.kind.type === "Shape" && isPath(e.kind) && !!e.kind.curve !== on)) return;
  updateLines((e) => (e.kind.type === "Shape" && isPath(e.kind) ? { ...e, kind: clean({ ...e.kind, curve: on || undefined }) } : e));
}
