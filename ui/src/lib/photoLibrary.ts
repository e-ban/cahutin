// Photothèque de la session : filtres, pastilles par dossier, photos utilisées, chemins à mémoriser.
import type { AssetMeta, Document, Element, Page } from "./types";

export interface LibraryPhoto {
  meta: AssetMeta;
  /** Chemin du fichier importé ; vide pour une photo venue d'un document ouvert. */
  path: string;
  folder: string;
}

export type PhotoFilter = { kind: "all" } | { kind: "unused" } | { kind: "page" } | { kind: "folder"; folder: string };

/** À partir de ce nombre de photos, le tiroir s'ouvre sur « Non placées » et les aides se taisent. */
export const MANY_PHOTOS = 20;

/** Le filtre à l'ouverture du tiroir : celui qu'on a choisi la dernière fois (sauf un dossier, propre à son
 *  document) ; sinon « Non placées » quand il y a beaucoup de photos. */
export function startFilter(saved: PhotoFilter | null, count: number): PhotoFilter {
  if (saved && (saved.kind === "all" || saved.kind === "unused" || saved.kind === "page")) return saved;
  return count >= MANY_PHOTOS ? { kind: "unused" } : { kind: "all" };
}

/** Les photos par jour de prise de vue, du plus récent au plus ancien ; celles sans date (documents d'avant la
 *  version 10) à la fin. Dans un jour, l'ordre d'import est gardé. */
export function byDay(photos: LibraryPhoto[]): { day: string | null; photos: LibraryPhoto[] }[] {
  const days = new Map<string | null, LibraryPhoto[]>();
  for (const p of photos) {
    const day = p.meta.taken ?? null;
    days.set(day, [...(days.get(day) ?? []), p]);
  }
  return [...days]
    .sort(([a], [b]) => (a === null ? 1 : b === null ? -1 : b.localeCompare(a)))
    .map(([day, photos]) => ({ day, photos }));
}

/** Le badge d'une photo posée : sa page, ou la première suivie de « … » si elle est sur plusieurs. */
export function pageBadge(pages: number[]): string | null {
  if (pages.length === 0) return null;
  return `p. ${pages[0]}${pages.length > 1 ? "…" : ""}`;
}

/** Les images posées sur une page : photos et fonds de cadre. */
export function assetsOnPage(page: Page): Set<string> {
  return new Set(page.elements.flatMap(elementAssets));
}

/** Images de la photothèque utilisées par un élément : la photo, ou l'image de fond d'un cadre de texte. */
export function elementAssets(e: Element): string[] {
  const k = e.kind;
  if (k.type === "Photo") return k.asset ? [k.asset] : [];
  if (k.type === "Text") return k.frame?.image ? [k.frame.image] : [];
  return [];
}

export function usedAssetIds(doc: Document): Set<string> {
  return new Set(doc.pages.flatMap((p) => p.elements.flatMap(elementAssets)));
}

export function filterPhotos(photos: LibraryPhoto[], filter: PhotoFilter, used: Set<string>, onPage: Set<string> = new Set()): LibraryPhoto[] {
  switch (filter.kind) {
    case "all":
      return photos;
    case "unused":
      return photos.filter((p) => !used.has(p.meta.id));
    case "page":
      return photos.filter((p) => onPage.has(p.meta.id));
    case "folder":
      return photos.filter((p) => p.folder === filter.folder);
  }
}

export function folderChips(photos: LibraryPhoto[]): { folder: string; count: number }[] {
  const counts = new Map<string, number>();
  for (const p of photos) if (p.folder) counts.set(p.folder, (counts.get(p.folder) ?? 0) + 1);
  return [...counts].map(([folder, count]) => ({ folder, count }));
}

/**
 * Chemins des photos importées mais pas placées, à retrouver à la réouverture du document.
 * `carried` : les chemins en attente du document ouvert ; ceux qui ne sont pas (encore) dans la
 * photothèque sont gardés (réimport en cours, clé USB absente aujourd'hui).
 */
export function pendingPaths(photos: LibraryPhoto[], doc: Document, carried: string[] = []): string[] {
  const used = usedAssetIds(doc);
  const inLibrary = new Set(photos.map((p) => p.path));
  const out: string[] = [];
  for (const p of photos) if (p.path && !used.has(p.meta.id) && !out.includes(p.path)) out.push(p.path);
  for (const path of carried) if (!inLibrary.has(path) && !out.includes(path)) out.push(path);
  return out;
}

/** Numéros (à partir de 1) des pages où l'image est posée. */
export function pagesUsing(doc: Document, id: string): number[] {
  return doc.pages.flatMap((p, i) => (p.elements.some((e) => elementAssets(e).includes(id)) ? [i + 1] : []));
}

/** Le document sans l'image : ses cases photo restent, vides (la mise en page ne bouge pas), et les cadres de
 *  texte perdent leur image de fond. */
export function withoutAsset(doc: Document, id: string): Document {
  const strip = (e: Element): Element => {
    const k = e.kind;
    if (k.type === "Photo" && k.asset === id) return { ...e, kind: { ...k, asset: null } };
    if (k.type === "Text" && k.frame?.image === id) return { ...e, kind: { ...k, frame: { ...k.frame, image: null } } };
    return e;
  };
  return { ...doc, pages: doc.pages.map((p) => ({ ...p, elements: p.elements.map(strip) })) };
}
