// Décors de page : mention de licence OpenMoji (même cadre que l'export ODG), cadres et frises.
import { newId } from "./factory";
import { normalizeGroups, pathOf, rotateElements, withPath } from "./groups";
import { pictoElement } from "./pictos.svelte";
import type { AssetMeta, Element, Frame, Page, PageSize } from "./types";

/** Mention en bas des pages qui contiennent une image OpenMoji (carnet_core::odg::content::credit_frame). */
export const CREDIT_SIZE_PT = 6;
export const CREDIT_MARGIN_MM = 8;

export function creditFrame(size: PageSize): Frame {
  return { x_mm: CREDIT_MARGIN_MM, y_mm: size.height_mm - CREDIT_MARGIN_MM, w_mm: size.width_mm - 2 * CREDIT_MARGIN_MM, h_mm: 5, rotation_deg: 0 };
}

const MARGIN_MM = 8;
const FRAME_COLOR = "#555555";

export type FrameStyle = "thin" | "thick" | "rounded" | "double";

function border(page: Page, inset: number, width_mm: number, corner_radius_mm: number): Element {
  const { width_mm: w, height_mm: h } = page.size;
  return {
    id: newId(),
    frame: { x_mm: inset, y_mm: inset, w_mm: w - 2 * inset, h_mm: h - 2 * inset, rotation_deg: 0 },
    locked: false,
    zone: false,
    kind: { type: "Shape", shape: "Rect", fill: null, stroke: { width_mm, color: FRAME_COLOR }, corner_radius_mm },
  };
}

/** Cadre le long de la marge de 8 mm ; le double est un groupe de deux rectangles. */
export function pageFrame(page: Page, style: FrameStyle): Element[] {
  switch (style) {
    case "thin":
      return [border(page, MARGIN_MM, 0.5, 0)];
    case "thick":
      return [border(page, MARGIN_MM, 2, 0)];
    case "rounded":
      return [border(page, MARGIN_MM, 1, 8)];
    case "double": {
      const group = newId();
      return [border(page, MARGIN_MM, 0.5, 0), border(page, MARGIN_MM + 3, 1.5, 0)].map((e) => ({ ...e, group }));
    }
  }
}

/** Rectangle vide qui entoure presque toute la page : un cadre de page, posé derrière tout. Son intérieur laisse
 *  passer les clics vers la page, sinon il serait choisi à chaque clic dans le vide. */
export function isPageFrame(e: Element, size: PageSize): boolean {
  const k = e.kind;
  return k.type === "Shape" && k.shape === "Rect" && k.fill === null && e.frame.w_mm >= 0.8 * size.width_mm && e.frame.h_mm >= 0.8 * size.height_mm;
}

export type FriezeSide = "top" | "bottom" | "around";
/** Taille des motifs d'une frise et écart minimal entre eux, par défaut. */
export const FRIEZE_SIZE_MM = 15;
export const FRIEZE_GAP_MM = 3;
const round = (v: number) => Math.round(v * 10) / 10;

/** Positions régulières de `size` le long d'une longueur utile, la première au début, la dernière à la fin, au
 *  moins `gap` entre deux. */
function spread(start: number, length: number, size: number, gap: number): number[] {
  const n = Math.max(1, Math.floor((length + gap) / (size + gap)));
  if (n === 1) return [round(start + (length - size) / 2)];
  const step = (length - size) / (n - 1);
  return Array.from({ length: n }, (_, i) => round(start + i * step));
}

type Rect = { x_mm: number; y_mm: number; w_mm: number; h_mm: number };

/**
 * Positions des copies le long d'un rectangle. Tout autour : une seule boucle dans le sens horaire
 * (haut, droite, bas, gauche), sans doublon aux coins, pour que la suite des motifs ne s'interrompe pas.
 */
function friezePositions(r: Rect, side: FriezeSide, size: number, gap: number): [number, number][] {
  const xs = spread(r.x_mm, r.w_mm, size, gap);
  const top = r.y_mm;
  const bottom = round(r.y_mm + r.h_mm - size);
  if (side === "top") return xs.map((x) => [x, top]);
  if (side === "bottom") return xs.map((x) => [x, bottom]);
  const ys = spread(r.y_mm, r.h_mm, size, gap).slice(1, -1);
  const right = round(r.x_mm + r.w_mm - size);
  return [
    ...xs.map((x): [number, number] => [x, top]),
    ...ys.map((y): [number, number] => [right, y]),
    ...[...xs].reverse().map((x): [number, number] => [x, bottom]),
    ...[...ys].reverse().map((y): [number, number] => [r.x_mm, y]),
  ];
}

/** Motif d'une frise : un pictogramme, ou une image de la collection (déjà dans les images de l'application). */
export type Motif = { code: string } | { asset: AssetMeta };

/** Le cadre d'une image de proportions `w` × `h` dans une case carrée de `size` : entière, centrée, son plus grand
 *  côté à la taille de la case (comme un pictogramme). */
function fitInSlot(x: number, y: number, size: number, w: number, h: number): Frame {
  const k = size / Math.max(w, h);
  const [fw, fh] = [round(w * k), round(h * k)];
  return { x_mm: round(x + (size - fw) / 2), y_mm: round(y + (size - fh) / 2), w_mm: fw, h_mm: fh, rotation_deg: 0 };
}

function motifElement(page: Page, m: Motif, x: number, y: number, size: number): Element {
  if ("code" in m) {
    const e = pictoElement(page, m.code, undefined, size);
    return { ...e, frame: { ...e.frame, x_mm: x, y_mm: y } };
  }
  return {
    id: newId(),
    frame: fitInSlot(x, y, size, m.asset.width_px, m.asset.height_px),
    locked: false,
    zone: false,
    kind: { type: "Photo", asset: m.asset.id, crop: { scale: 1, center_x: 0.5, center_y: 0.5 }, corner_radius_mm: 0, border: null, shadow: false },
  };
}

/** Copies régulièrement espacées des motifs, répétés dans l'ordre, groupées en une frise. */
export function friezeInRect(page: Page, r: Rect, motifs: Motif[], side: FriezeSide, size_mm = FRIEZE_SIZE_MM, gap_mm = FRIEZE_GAP_MM): Element[] {
  const group = newId();
  return friezePositions(r, side, size_mm, gap_mm).map(([x, y], i) => ({ ...motifElement(page, motifs[i % motifs.length], x, y, size_mm), group }));
}

/** Frise le long des marges de la page. */
export function frieze(page: Page, motifs: Motif[], side: FriezeSide, size_mm = FRIEZE_SIZE_MM, gap_mm = FRIEZE_GAP_MM): Element[] {
  const { width_mm: w, height_mm: h } = page.size;
  return friezeInRect(page, { x_mm: MARGIN_MM, y_mm: MARGIN_MM, w_mm: w - 2 * MARGIN_MM, h_mm: h - 2 * MARGIN_MM }, motifs, side, size_mm, gap_mm);
}

const isPicture = (e: Element) => (e.kind.type === "Image" || (e.kind.type === "Photo" && !!e.kind.asset)) && !e.zone && !e.locked && !e.frame.rotation_deg;
/** Taille de la case d'un motif : son plus grand côté. */
const slotOf = (e: Element) => Math.max(e.frame.w_mm, e.frame.h_mm);

/** Les éléments d'une frise dans la sélection : au moins trois images (pictogrammes ou images de la collection)
 *  d'un même groupe, à la même taille, sans rotation. Une frise posée autour d'un bloc est trouvée dans son
 *  groupe. */
export function friezeIn(page: Page, ids: string[]): string[] | null {
  const byPath = new Map<string, Element[]>();
  for (const e of page.elements) {
    if (!e.group) continue;
    const key = pathOf(e).join("/");
    byPath.set(key, [...(byPath.get(key) ?? []), e]);
  }
  const found = [...byPath.values()].filter((els) => els.some((e) => ids.includes(e.id)) && els.length >= 3 && els.every(isPicture) && els.every((e) => Math.abs(slotOf(e) - slotOf(els[0])) < 0.3));
  return found.length === 1 ? found[0].map((e) => e.id) : null;
}

/** Taille et écart des motifs d'une frise posée. */
export function friezeSettings(page: Page, ids: string[]): { size: number; gap: number } {
  const els = page.elements.filter((e) => ids.includes(e.id));
  const size = round(slotOf(els[0]));
  const cx = els.map((e) => e.frame.x_mm + e.frame.w_mm / 2).sort((a, b) => a - b);
  const steps = cx.slice(1).map((x, i) => x - cx[i]).filter((d) => d > 0.5);
  return { size, gap: steps.length ? Math.max(0, Math.round(Math.min(...steps) - size)) : FRIEZE_GAP_MM };
}

/** Refait la frise à une autre taille ou un autre écart, à la même place (bord haut d'une frise du haut, bord bas
 *  d'une frise du bas, tour extérieur d'une frise tout autour), avec les mêmes motifs dans le même ordre et dans
 *  ses groupes. */
export function resizeFrieze(page: Page, ids: string[], size: number, gap: number): { page: Page; ids: string[] } {
  const els = page.elements.filter((e) => ids.includes(e.id));
  const s0 = slotOf(els[0]);
  const centers = els.map((e) => [e.frame.x_mm + e.frame.w_mm / 2, e.frame.y_mm + e.frame.h_mm / 2]);
  const [minX, maxX] = [Math.min(...centers.map((c) => c[0])) - s0 / 2, Math.max(...centers.map((c) => c[0])) + s0 / 2];
  const [minY, maxY] = [Math.min(...centers.map((c) => c[1])) - s0 / 2, Math.max(...centers.map((c) => c[1])) + s0 / 2];
  const oneRow = maxY - minY < s0 + 0.5;
  const side: FriezeSide = !oneRow ? "around" : (minY + maxY) / 2 < page.size.height_mm / 2 ? "top" : "bottom";
  const rect = { x_mm: minX, y_mm: minY, w_mm: maxX - minX, h_mm: maxY - minY };
  // La suite des motifs : le plus court début qui se répète.
  const key = (e: Element) => (e.kind.type === "Image" ? `${e.kind.code}/${e.kind.outline ?? false}` : e.kind.type === "Photo" ? `${e.kind.asset}` : "");
  const keys = els.map(key);
  const repeats = (p: number) => p > 0 && keys.every((k, i) => k === keys[i % p]);
  const found = keys.findIndex((_, p) => repeats(p));
  const period = found > 0 ? found : keys.length;
  const path = pathOf(els[0]);
  const fresh = friezePositions(rect, side, size, gap).map(([x, y], i) => {
    const t = els[i % period];
    const frame = t.kind.type === "Image" ? { x_mm: x, y_mm: y, w_mm: size, h_mm: size, rotation_deg: 0 } : fitInSlot(x, y, size, t.frame.w_mm, t.frame.h_mm);
    return withPath({ ...t, id: newId(), frame }, path);
  });
  const at = page.elements.findIndex((e) => ids.includes(e.id));
  const rest = page.elements.filter((e) => !ids.includes(e.id));
  const before = page.elements.slice(0, at).filter((e) => !ids.includes(e.id)).length;
  return { page: normalizeGroups({ ...page, elements: [...rest.slice(0, before), ...fresh, ...rest.slice(before)] }), ids: fresh.map((e) => e.id) };
}

/** Motifs d'une frise : un clic ajoute l'image à la fin de la suite, ou l'en retire. */
export function toggleMotif(codes: string[], code: string): string[] {
  return codes.includes(code) ? codes.filter((c) => c !== code) : [...codes, code];
}

/**
 * Frise tout autour d'un bloc (photo, forme, texte) : motifs centrés sur ses bords, du quart de son plus petit
 * côté (6 à 15 mm), tournés avec lui et groupés avec lui (dans son groupe s'il en a un). `null` pour une case
 * de grille, jamais groupée, ou sans motif.
 */
export function blockFrieze(page: Page, id: string, motifs: Motif[]): Page | null {
  const target = page.elements.find((e) => e.id === id);
  if (!target || motifs.length === 0 || page.grid?.cells.some((c) => c.element_id === id)) return null;
  const f = target.frame;
  const size = round(Math.min(15, Math.max(6, Math.min(f.w_mm, f.h_mm) / 4)));
  const rect = { x_mm: f.x_mm - size / 2, y_mm: f.y_mm - size / 2, w_mm: f.w_mm + size, h_mm: f.h_mm + size };
  // Un groupe nouveau réunit le bloc et sa frise, qui reste un groupe à elle : dissocier rend l'un et l'autre
  // intacts.
  const group = newId();
  let pics: Element[] = friezeInRect(page, rect, motifs, "around", size).map((e) => withPath(e, [group, e.group!]));
  if (f.rotation_deg) pics = rotateElements(pics, f.rotation_deg, { x_mm: f.x_mm + f.w_mm / 2, y_mm: f.y_mm + f.h_mm / 2 });
  const block = new Set(target.group ? page.elements.filter((e) => e.group === target.group).map((e) => e.id) : [id]);
  const wrapped = page.elements.map((e) => (block.has(e.id) ? withPath(e, [group, ...pathOf(e)]) : e));
  const at = wrapped.findIndex((e) => e.id === id);
  return normalizeGroups({ ...page, elements: [...wrapped.slice(0, at + 1), ...pics, ...wrapped.slice(at + 1)] });
}
