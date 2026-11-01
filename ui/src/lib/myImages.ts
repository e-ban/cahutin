// « Mes images » : noms lisibles et recherche. Le fichier garde son nom ; l'enseignant voit un nom propre.
import type { ClassPhoto, MyImage } from "./types";

/** « 960px-Lemmling-Cartoon-cow.svg.png » devient « Lemmling Cartoon cow » : sans extensions, sans taille en
 *  pixels ni numéro de copie, tirets et soulignés en espaces. */
export function displayName(file: string): string {
  const name = file
    .replace(/(\.(png|jpe?g|webp|svg))+$/i, "")
    .replace(/^\d+px-/i, "")
    .replace(/ \(\d+\)$/, "")
    .replace(/[-_]+/g, " ")
    .replace(/\s+/g, " ")
    .trim();
  return name || file;
}

const fold = (s: string) => s.normalize("NFD").replace(/\p{M}/gu, "").toLowerCase();

/** Les images dont le nom ou un thème contient chaque mot cherché, sans tenir compte des accents ni des
 *  majuscules. */
export function searchMyImages(images: MyImage[], query: string): MyImage[] {
  const words = fold(query).split(/\s+/).filter(Boolean);
  return images.filter((i) => {
    const text = fold([displayName(i.name), ...(i.themes ?? [])].join(" "));
    return words.every((w) => text.includes(w));
  });
}

/** Les thèmes de la collection, par ordre alphabétique, avec leur nombre d'images. */
export function themeCounts(images: MyImage[]): { theme: string; count: number }[] {
  const counts = new Map<string, number>();
  for (const i of images) for (const t of i.themes ?? []) counts.set(t, (counts.get(t) ?? 0) + 1);
  return [...counts].map(([theme, count]) => ({ theme, count })).sort((a, b) => a.theme.localeCompare(b.theme, "fr"));
}

/** Filtre de la collection : tout, un thème, ou les images sans thème. */
export type ThemeFilter = { kind: "all" } | { kind: "theme"; theme: string } | { kind: "none" };

export function filterByTheme(images: MyImage[], f: ThemeFilter): MyImage[] {
  if (f.kind === "theme") return images.filter((i) => i.themes?.includes(f.theme));
  if (f.kind === "none") return images.filter((i) => !i.themes?.length);
  return images;
}

/** Vrai si l'image montre des enfants de la classe : une photo de la photothèque où des élèves sont notés, ou
 *  un portrait de la liste de classe. Elle ne va pas dans « Mes images », hors des autorisations. */
export function showsChildren(assetId: string, library: ClassPhoto[], portraits: Record<string, unknown>): boolean {
  return assetId in portraits || library.some((p) => p.id === assetId && p.students.length > 0);
}
