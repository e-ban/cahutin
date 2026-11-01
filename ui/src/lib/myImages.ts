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

/** Les images dont le nom contient chaque mot cherché, sans tenir compte des accents ni des majuscules. */
export function searchMyImages(images: MyImage[], query: string): MyImage[] {
  const words = fold(query).split(/\s+/).filter(Boolean);
  return images.filter((i) => words.every((w) => fold(displayName(i.name)).includes(w)));
}

/** Vrai si l'image montre des enfants de la classe : une photo de la photothèque où des élèves sont notés, ou
 *  un portrait de la liste de classe. Elle ne va pas dans « Mes images », hors des autorisations. */
export function showsChildren(assetId: string, library: ClassPhoto[], portraits: Record<string, unknown>): boolean {
  return assetId in portraits || library.some((p) => p.id === assetId && p.students.length > 0);
}
