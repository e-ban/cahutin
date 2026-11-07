// Les champs de la date et de la classe ({date}, {ecole}…) s'écrivent une fois : à la création d'une page depuis
// un modèle, à la pose d'un champ dans un document, ou à la fin de l'écriture d'un texte où on l'a tapé. En mode
// modèle, ils restent des pastilles. Un champ de la classe qui attend (pas de classe choisie, ou une école pas encore
// renseignée) s'écrit quand la classe est choisie ou complétée.
import { isAutoField, isDateField, needsDate, splitFields } from "./classFields";
import type { Class, Page, RichText } from "./types";

export interface AutoDeps {
  /** La classe choisie, ou `null`. */
  cls: Class | null;
  /** Demande la date (« 2026-11-09 ») ; `null` si l'on a annulé. */
  ask: (cancellable: boolean) => Promise<string | null>;
  /** Le moteur : écrit les champs de la date et de la classe d'une page. */
  fill: (page: Page, date: string | null, cls: Class | null) => Promise<Page>;
}

const autoKeys = (c: RichText) => c.paragraphs.flatMap((p) => p.runs.flatMap((r) => splitFields(r.text).flatMap((x) => ("field" in x && isAutoField(x.field) ? [x.field] : []))));
const hasClassFieldIn = (c: RichText) => autoKeys(c).some((k) => !isDateField(k));
const textsOf = (page: Page) => page.elements.flatMap((e) => (e.kind.type === "Text" ? [e.kind.content] : []));

/** Pourquoi des champs de la classe restent : aucune classe choisie, ou une information vide sur « Ma classe ». */
export type Waiting = "noClass" | "empty" | null;
const waiting = (left: boolean, cls: Class | null): Waiting => (!left ? null : cls ? "empty" : "noClass");

/** Une page qui vient d'un modèle, ses champs de la date et de la classe écrits : une seule date demandée, et
 *  seulement s'il y en a. */
export async function completePage(page: Page, deps: AutoDeps & { templateMode: boolean }): Promise<{ page: Page; notice: Waiting }> {
  if (deps.templateMode) return { page, notice: null };
  const date = needsDate(page) ? await deps.ask(false) : null;
  const done = await deps.fill(page, date, deps.cls);
  return { page: done, notice: waiting(textsOf(done).some(hasClassFieldIn), deps.cls) };
}

/** Un texte seul sur une page, pour que le moteur l'écrive. */
const pageWith = (content: RichText): Page => ({
  id: "champ",
  size: { width_mm: 10, height_mm: 10 },
  grid: null,
  elements: [
    {
      id: "t",
      frame: { x_mm: 0, y_mm: 0, w_mm: 10, h_mm: 10, rotation_deg: 0 },
      locked: false,
      zone: false,
      kind: { type: "Text", content, style: { font: "Andika", size_pt: 14, color: "#222222", align: "Left" }, prompt: null },
    },
  ],
});
const run = (text: string) => ({ text, bold: false, italic: false, underline: false, color: null, font: null, size_pt: null });

/** Le texte d'un champ de la date ou de la classe posé dans un document : sa valeur, `null` si l'on a annulé la
 *  date (rien n'est posé), ou ses accolades s'il attend une classe ou une information de la classe. */
export async function autoText(key: string, deps: AutoDeps): Promise<string | null> {
  const date = isDateField(key) ? await deps.ask(true) : null;
  if (isDateField(key) && date === null) return null;
  const filled = await deps.fill(pageWith({ paragraphs: [{ align: null, runs: [run(`{${key}}`)] }] }), date, deps.cls);
  const e = filled.elements[0];
  return e.kind.type === "Text" ? e.kind.content.paragraphs.map((p) => p.runs.map((r) => r.text).join("")).join("\n") : null;
}

/** Ce que fait la pose d'un champ : une pastille (champ d'élève, mode modèle, ou champ de la classe qui attend :
 *  `noClass`, `empty`), son texte, ou rien (date annulée). */
export type Placing = { kind: "pastille"; notice?: "noClass" | "empty" } | { kind: "text"; text: string } | { kind: "cancel" };

export async function placing(key: string, deps: AutoDeps & { templateMode: boolean }): Promise<Placing> {
  if (!isAutoField(key) || deps.templateMode) return { kind: "pastille" };
  const t = await autoText(key, deps);
  if (t === null) return { kind: "cancel" };
  if (t === `{${key}}`) return { kind: "pastille", notice: deps.cls ? "empty" : "noClass" };
  return { kind: "text", text: t };
}

/** Un texte qu'on vient d'écrire contient un champ de la date ou de la classe de plus qu'avant. */
export function addsAutoFields(before: RichText, after: RichText): boolean {
  return autoKeys(after).length > autoKeys(before).length;
}

/** Un texte où l'on a tapé un champ de la date ou de la classe, écrit comme un champ posé : la date demandée (on
 *  peut annuler : ses champs restent), la classe écrite. */
export async function completeText(content: RichText, deps: AutoDeps): Promise<{ content: RichText; notice: Waiting }> {
  const date = needsDate(pageWith(content)) ? await deps.ask(true) : null;
  const e = (await deps.fill(pageWith(content), date, deps.cls)).elements[0];
  const done = e.kind.type === "Text" ? e.kind.content : content;
  return { content: done, notice: waiting(hasClassFieldIn(done), deps.cls) };
}

/** Les pages, leurs champs de la classe écrits avec cette classe (jamais la date) ; `null` si rien n'a changé. */
export async function fillClassFields(pages: Page[], cls: Class, fill: AutoDeps["fill"]): Promise<Page[] | null> {
  let changed = false;
  const out: Page[] = [];
  for (const p of pages) {
    if (!textsOf(p).some(hasClassFieldIn)) {
      out.push(p);
      continue;
    }
    const done = await fill(p, null, cls);
    if (JSON.stringify(done) !== JSON.stringify(p)) changed = true;
    out.push(done);
  }
  return changed ? out : null;
}
