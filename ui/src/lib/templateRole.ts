// Le rôle d'un texte ou d'une photo dans un modèle : à remplir (une zone, vidée quand le modèle est
// enregistré, son texte d'origine gardé comme invite), ou fixe (verrouillé, gardé tel quel : un titre, un
// logo). Sans l'un ni l'autre, il n'est pas gardé dans le modèle : une photo ou un texte libre peut montrer un
// enfant ou le nommer (voir `templates::from_page` dans le moteur).
import type { Element } from "./types";

export type TemplateRole = "fill" | "fixed" | "none";

/** Les éléments qui ont un rôle à choisir : textes et photos (formes et pictogrammes sont toujours gardés). */
export const hasTemplateRole = (e: Element) => e.kind.type === "Text" || e.kind.type === "Photo";

export const templateRole = (e: Element): TemplateRole => (e.zone ? "fill" : e.locked ? "fixed" : "none");

const plain = (e: Element) =>
  e.kind.type === "Text" ? e.kind.content.paragraphs.map((p) => p.runs.map((r) => r.text).join("")).join(" ").trim() : "";

/** L'élément avec ce rôle. Un texte à remplir garde ses mots comme invite (« Titre »), sauf s'il en a déjà une. */
export function withTemplateRole(e: Element, role: Exclude<TemplateRole, "none">): Element {
  if (role === "fixed") return { ...e, zone: false, locked: true };
  const k = e.kind;
  if (k.type !== "Text") return { ...e, zone: true, locked: false };
  return { ...e, zone: true, locked: false, kind: { ...k, prompt: k.prompt || plain(e) || "Texte" } };
}
