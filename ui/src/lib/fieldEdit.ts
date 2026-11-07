// Poser et changer un champ de la classe, hors de l'éditeur de texte : depuis le tiroir Texte, ou depuis le
// bouton « Champ » de la barre d'un texte choisi.
import { splitFields } from "./classFields";
import { centeredFrame, textElement } from "./factory";
import type { Element, Page, RichText } from "./types";

const run = (text: string) => ({ text, bold: false, italic: false, underline: false, color: null, font: null, size_pt: null });

/** Les champs du texte, dans l'ordre. */
export function fieldsOf(content: RichText): string[] {
  return content.paragraphs.flatMap((p) => p.runs.flatMap((r) => splitFields(r.text).flatMap((x) => ("field" in x ? [x.field] : []))));
}

/** Un texte posé comme ceux du tiroir, qui contient le champ ; jamais une zone ni verrouillé, même en mode
 *  modèle : un texte à champ est gardé tel quel dans un modèle. */
export function fieldText(page: Page, key: string): Element {
  return textWith(page, `{${key}}`);
}

/** Un texte posé comme ceux du tiroir, qui contient `words` (un champ, ou une date déjà écrite). */
export function textWith(page: Page, words: string): Element {
  const e = textElement(centeredFrame(page, 80, 20));
  if (e.kind.type === "Text") e.kind = { ...e.kind, content: { paragraphs: [{ align: null, runs: [run(words)] }] } };
  return { ...e, zone: false, locked: false };
}

/** `words` ajouté à la fin du dernier paragraphe, après une espace s'il en faut une. */
export function appendText(content: RichText, words: string): RichText {
  const paragraphs = content.paragraphs.length ? content.paragraphs : [{ align: null, runs: [] }];
  const last = paragraphs[paragraphs.length - 1];
  const text = last.runs.map((r) => r.text).join("");
  const added = `${text && !/\s$/.test(text) ? " " : ""}${words}`;
  const end = last.runs.at(-1);
  const runs = end ? [...last.runs.slice(0, -1), { ...end, text: end.text + added }] : [run(added)];
  return { paragraphs: [...paragraphs.slice(0, -1), { ...last, runs }] };
}

/** Le texte avec ce champ : un seul champ est remplacé ; sinon (aucun, ou plusieurs), le champ s'ajoute à la
 *  fin du dernier paragraphe. */
export function setField(content: RichText, key: string): RichText {
  const fields = fieldsOf(content);
  if (fields.length === 1) {
    const old = `{${fields[0]}}`;
    return { paragraphs: content.paragraphs.map((p) => ({ ...p, runs: p.runs.map((r) => (r.text.includes(old) ? { ...r, text: r.text.replace(old, `{${key}}`) } : r)) })) };
  }
  return appendText(content, `{${key}}`);
}
