// Photothèque de la session : filtres, pastilles par dossier, photos utilisées, chemins à mémoriser.
import type { AssetMeta, Document, Element } from "./types";

export interface LibraryPhoto {
  meta: AssetMeta;
  /** Chemin du fichier importé ; vide pour une photo venue d'un document ouvert. */
  path: string;
  folder: string;
}

export type PhotoFilter = { kind: "all" } | { kind: "unused" } | { kind: "folder"; folder: string };

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

export function filterPhotos(photos: LibraryPhoto[], filter: PhotoFilter, used: Set<string>): LibraryPhoto[] {
  switch (filter.kind) {
    case "all":
      return photos;
    case "unused":
      return photos.filter((p) => !used.has(p.meta.id));
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
