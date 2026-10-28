import type { Document } from "./types";

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

/** Familles utilisées par le document et absentes du poste (le texte s'affiche alors en Andika). */
export function missingFamilies(doc: Document): string[] {
  const out: string[] = [];
  const add = (f: string | null | undefined) => {
    if (f && !known.has(f) && !out.includes(f)) out.push(f);
  };
  for (const e of doc.pages.flatMap((p) => p.elements)) {
    if (e.kind.type !== "Text") continue;
    add(e.kind.style.font);
    for (const r of e.kind.content.paragraphs.flatMap((p) => p.runs)) add(r.font);
  }
  return out;
}
