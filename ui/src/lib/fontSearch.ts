// Recherche et groupes du sélecteur de police (fournies, ajoutées, sur cet ordinateur).
import type { FontFamilyInfo } from "./types";

/** Minuscules, sans accents ni espaces autour : pour comparer ce que l'on tape à des noms. */
export const normalize = (s: string) =>
  s
    .normalize("NFD")
    .replace(/\p{Diacritic}/gu, "")
    .toLowerCase()
    .trim();

/** Familles dont le nom contient la recherche, sans tenir compte des majuscules ni des accents. */
export function searchFamilies(families: FontFamilyInfo[], query: string): FontFamilyInfo[] {
  const q = normalize(query);
  return q ? families.filter((f) => normalize(f.name).includes(q)) : families;
}

export function groupFamilies(families: FontFamilyInfo[]) {
  return {
    builtin: families.filter((f) => !f.user && !f.system),
    user: families.filter((f) => f.user),
    system: families.filter((f) => f.system),
  };
}
