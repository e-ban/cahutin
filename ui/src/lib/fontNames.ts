/** Familles fournies (carnet_core::fonts). */
export const EMBEDDED_FAMILIES = ["Andika", "Atkinson Hyperlegible", "OpenDyslexic", "Playwrite FR Moderne"];

// Familles connues du moteur (fournies et ajoutées) : les seules qu'un texte peut utiliser.
let known = new Set(EMBEDDED_FAMILIES);

export function setKnownFamilies(names: string[]) {
  known = new Set(names);
}

export function isKnownFamily(name: string): boolean {
  return known.has(name);
}
