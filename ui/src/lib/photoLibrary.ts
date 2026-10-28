// Photothèque de la session : filtres, pastilles par dossier, photos utilisées, chemins à mémoriser.
import type { AssetMeta, Document } from "./types";

export interface LibraryPhoto {
  meta: AssetMeta;
  /** Chemin du fichier importé ; vide pour une photo venue d'un document ouvert. */
  path: string;
  folder: string;
}

export type PhotoFilter = { kind: "all" } | { kind: "unused" } | { kind: "folder"; folder: string };

export function usedAssetIds(doc: Document): Set<string> {
  const ids = new Set<string>();
  for (const p of doc.pages) for (const e of p.elements) if (e.kind.type === "Photo" && e.kind.asset) ids.add(e.kind.asset);
  return ids;
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

/** Chemins des photos importées mais pas placées, à retrouver à la réouverture du document. */
export function pendingPaths(photos: LibraryPhoto[], doc: Document): string[] {
  const used = usedAssetIds(doc);
  const out: string[] = [];
  for (const p of photos) if (p.path && !used.has(p.meta.id) && !out.includes(p.path)) out.push(p.path);
  return out;
}
