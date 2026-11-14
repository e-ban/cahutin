// Ce que l'imprimante risque de couper. Les pages s'impriment sans marge (print.css), mais une imprimante de bureau
// n'imprime pas les derniers millimètres de la feuille, et l'application ne peut pas savoir combien. On prévient,
// sans bloquer, et la page n'est jamais réduite.
import { isCutMark } from "./cutMarks";
import { friezeIn, isPageFrame } from "./decor";
import { boundsOf } from "./groups";
import { fr } from "./i18n/fr";
import { a3Tiles, formatOf } from "./pageFormat";
import type { Document, Element, Frame, Page, PageSize } from "./types";

/** Sous cette distance au bord (mm), un contenu risque d'être coupé ; les tuiles A3 se recouvrent du double. */
export const SAFE_MARGIN_MM = 5;

/** Ce qui se perd si l'imprimante coupe. */
export type RiskKind = "text" | "photo" | "table" | "image" | "cut" | "graphisme";

const blankText = (e: Element) => e.kind.type === "Text" && e.kind.content.paragraphs.every((p) => p.runs.every((r) => r.text.trim() === ""));

/** Ce que l'élément perdrait, coupé ; null pour ce qui se rogne sans dommage (décor) ou ne s'imprime pas. */
function kindOf(page: Page, e: Element, friezes: Set<string>): RiskKind | null {
  const k = e.kind;
  // D'abord les repères de découpe : les ciseaux sont une image, un contour de découpe ressemble à un cadre de page.
  if (isCutMark(e)) return "cut";
  if (isPageFrame(e, page.size) || friezes.has(e.id)) return null;
  switch (k.type) {
    case "Text":
      // Un texte vide ne s'imprime pas, sauf son cadre (fond, bordure, lignes d'écriture).
      return blankText(e) && !k.frame ? null : "text";
    case "Photo":
      // Une zone sans photo ne s'imprime pas.
      return k.asset ? "photo" : null;
    case "Table":
      return "table";
    case "Image":
      return "image";
    case "Shape":
      // Les tracés de graphisme (et leur ligne de base, qui porte la recette) ; les autres formes sont des bandeaux.
      return k.shape === "Path" || k.graphisme ? "graphisme" : null;
  }
}

/** La surface dessinée : un trait n'est que sa ligne, pas la bande de clic autour. */
function drawn(e: Element): Frame {
  const f = e.frame;
  if (e.kind.type !== "Shape" || (e.kind.shape !== "Line" && e.kind.shape !== "Arrow")) return f;
  return { ...f, y_mm: f.y_mm + f.h_mm / 2, h_mm: 0 };
}

/** Distance au bord le plus proche, au centième de mm (les rotations laissent des restes : 4,9999…). */
function edgeDistance(f: Frame, size: PageSize): number {
  const b = boundsOf([f]);
  const d = Math.min(b.x_mm, b.y_mm, size.width_mm - b.x_mm - b.w_mm, size.height_mm - b.y_mm - b.h_mm);
  return Math.round(d * 100) / 100;
}

function friezesOf(page: Page): Set<string> {
  // Une frise reste une frise une fois verrouillée (dans un modèle).
  const unlocked = { ...page, elements: page.elements.map((e) => (e.locked ? { ...e, locked: false } : e)) };
  const found = new Set<string>();
  for (const e of page.elements) if (e.group && !found.has(e.id) && (e.kind.type === "Image" || e.kind.type === "Photo")) for (const id of friezeIn(unlocked, [e.id]) ?? []) found.add(id);
  return found;
}

function risks(page: Page): { e: Element; kind: RiskKind }[] {
  const friezes = friezesOf(page);
  return page.elements.flatMap((e) => {
    const kind = kindOf(page, e, friezes);
    return kind && edgeDistance(drawn(e), page.size) < SAFE_MARGIN_MM ? [{ e, kind }] : [];
  });
}

/** Les éléments de contenu qui passent à moins de SAFE_MARGIN_MM d'un bord (boîte englobante, rotation comprise). */
export function atRisk(page: Page): Element[] {
  return risks(page).map((r) => r.e);
}

/** Les pages (indices à partir de 0) qui ont un élément à risque. */
export function pagesAtRisk(doc: Document): number[] {
  return doc.pages.flatMap((p, i) => (atRisk(p).length ? [i] : []));
}

/** Les feuilles qu'il faut à l'impression ; en tuiles, une affiche A3 prend deux A4 de l'autre orientation. */
function sheets(doc: Document, tiles: boolean): PageSize[] {
  return doc.pages.map((p) => (tiles && formatOf(p.size) === "A3" ? a3Tiles(p.size)[0].sheet : p.size));
}

/** Le document demande des papiers différents (formats ou orientations mêlés). */
export function mixedSheets(doc: Document, tiles: boolean): boolean {
  const [first, ...rest] = sheets(doc, tiles);
  return rest.some((s) => Math.abs(s.width_mm - first.width_mm) > 0.5 || Math.abs(s.height_mm - first.height_mm) > 0.5);
}

/**
 * L'avis avant d'imprimer : ce qui risque d'être coupé (la première page en cause et ses éléments, pour le
 * bouton) et, sous Linux, les formats mêlés qui sortiront tous sur le même papier. null : rien à dire.
 */
export function printWarning(doc: Document, opts: { linux: boolean; pdf: boolean; tiles: boolean }): { message: string; page: number; ids: string[] } | null {
  const found = doc.pages.map(risks);
  const pages = found.flatMap((r, i) => (r.length ? [i] : []));
  // Un cadre de découpe, une ligne de graphisme comptent pour un, quel que soit le nombre de leurs traits.
  const units = found.flatMap((r) => [...new Map(r.map(({ e, kind }) => [kind === "cut" || kind === "graphisme" ? `${kind}:${e.group ?? e.id}` : e.id, kind])).values()]);
  const parts: string[] = [];
  if (pages.length) parts.push(fr.edgeRisk(pages.map((i) => i + 1), units) + (opts.pdf ? " " + fr.edgeRiskPdf : ""));
  if (opts.linux && !opts.pdf && mixedSheets(doc, opts.tiles)) parts.push(fr.mixedSheets);
  if (!parts.length) return null;
  return { message: parts.join("\n\n"), page: pages[0] ?? -1, ids: pages.length ? found[pages[0]].map((r) => r.e.id) : [] };
}
