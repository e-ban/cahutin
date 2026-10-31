// Ce que l'interface doit savoir de la classe sur la page courante : y a-t-il des prénoms à remplir, et
// quelle photo va à quel enfant d'après le nom de son fichier.
import { normalize } from "./fontSearch";
import type { Element, Page, Student } from "./types";

const NAME_PROMPTS = ["Prénom", "PRÉNOM", "Nom"];
const NAME_MARKERS = ["{prénom}", "{PRÉNOM}", "{nom}"];

const isNameZone = (e: Element) => e.zone && e.kind.type === "Text" && NAME_PROMPTS.includes(e.kind.prompt ?? "");
const hasNameMarker = (e: Element) => e.kind.type === "Text" && e.kind.content.paragraphs.some((p) => p.runs.some((r) => NAME_MARKERS.some((m) => r.text.includes(m))));
const written = (e: Element) => e.kind.type === "Text" && e.kind.content.paragraphs.some((p) => p.runs.some((r) => r.text.trim() !== ""));

/** La page attend des prénoms : une zone « Prénom » (ou « Nom ») ou le mot {prénom}. */
export function hasNameFields(page: Page): boolean {
  return page.elements.some((e) => isNameZone(e) || hasNameMarker(e));
}

/** Une zone « Prénom » est déjà écrite : la page a été remplie (ou commencée à la main). */
export function nameFieldsFilled(page: Page): boolean {
  return page.elements.some((e) => isNameZone(e) && written(e));
}

/** Nom du fichier sans dossier ni extension, en mots sans accents ni capitales (« lea_durand.JPG » → « lea durand »). */
function words(path: string): string {
  const base = path.split(/[\\/]/).pop() ?? "";
  return normalize(base.replace(/\.[^.]+$/, "").replace(/[_\s]+/g, " ").replace(/\s*\d+$/, "")).trim();
}

/**
 * Photos attribuées aux enfants d'après le nom du fichier : le prénom (« Noé.jpg »), ou le prénom et le nom
 * dans un sens ou dans l'autre. Un prénom porté par plusieurs enfants demande le nom ; une photo qui ne
 * désigne personne, ou plusieurs enfants, reste sans attribution.
 */
export function matchPhotos(students: Student[], paths: string[]): { matched: { student: string; path: string }[]; unmatched: string[] } {
  const first = (s: Student) => normalize(s.first_name).trim();
  const full = (s: Student) => [normalize(`${s.first_name} ${s.last_name}`).trim(), normalize(`${s.last_name} ${s.first_name}`).trim()];
  const matched: { student: string; path: string }[] = [];
  const unmatched: string[] = [];
  for (const path of paths) {
    const w = words(path);
    let found = students.filter((s) => s.last_name && full(s).includes(w));
    if (found.length === 0) found = students.filter((s) => first(s) === w);
    if (found.length === 1 && !matched.some((m) => m.student === found[0].id)) matched.push({ student: found[0].id, path });
    else unmatched.push(path);
  }
  return { matched, unmatched };
}
