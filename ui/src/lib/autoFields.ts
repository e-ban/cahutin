// Les champs de la date et de la classe ({date}, {ecole}…) s'écrivent une fois : à la création d'une page depuis
// un modèle, ou à la pose d'un champ dans un document. En mode modèle, ils restent des pastilles.
import { isAutoField, isDateField, needsDate, splitFields } from "./classFields";
import type { Class, Page } from "./types";

export interface AutoDeps {
  /** La classe choisie, ou `null`. */
  cls: Class | null;
  /** Demande la date (« 2026-11-09 ») ; `null` si l'on a annulé. */
  ask: (cancellable: boolean) => Promise<string | null>;
  /** Le moteur : écrit les champs de la date et de la classe d'une page. */
  fill: (page: Page, date: string | null, cls: Class | null) => Promise<Page>;
}

/** Un champ de la classe reste à écrire : aucune classe n'est choisie. */
function hasClassField(page: Page): boolean {
  return page.elements.some((e) => e.kind.type === "Text" && e.kind.content.paragraphs.some((p) => p.runs.some((r) => splitFields(r.text).some((x) => "field" in x && isAutoField(x.field) && !isDateField(x.field)))));
}

/** Une page qui vient d'un modèle, ses champs de la date et de la classe écrits : une seule date demandée, et
 *  seulement s'il y en a. `missingClass` : des champs de la classe restent, faute de classe choisie. */
export async function completePage(page: Page, deps: AutoDeps & { templateMode: boolean }): Promise<{ page: Page; missingClass: boolean }> {
  if (deps.templateMode) return { page, missingClass: false };
  const date = needsDate(page) ? await deps.ask(false) : null;
  const done = await deps.fill(page, date, deps.cls);
  return { page: done, missingClass: hasClassField(done) };
}

/** Le texte d'un champ de la date ou de la classe posé dans un document : sa valeur, `null` si l'on a annulé la
 *  date (rien n'est posé), ou ses accolades s'il faut une classe et qu'aucune n'est choisie. */
export async function autoText(key: string, deps: AutoDeps): Promise<string | null> {
  const date = isDateField(key) ? await deps.ask(true) : null;
  if (isDateField(key) && date === null) return null;
  const one: Page = {
    id: "champ",
    size: { width_mm: 10, height_mm: 10 },
    grid: null,
    elements: [
      {
        id: "t",
        frame: { x_mm: 0, y_mm: 0, w_mm: 10, h_mm: 10, rotation_deg: 0 },
        locked: false,
        zone: false,
        kind: { type: "Text", content: { paragraphs: [{ align: null, runs: [{ text: `{${key}}`, bold: false, italic: false, underline: false, color: null, font: null, size_pt: null }] }] }, style: { font: "Andika", size_pt: 14, color: "#222222", align: "Left" }, prompt: null },
      },
    ],
  };
  const filled = await deps.fill(one, date, deps.cls);
  const e = filled.elements[0];
  return e.kind.type === "Text" ? e.kind.content.paragraphs.map((p) => p.runs.map((r) => r.text).join("")).join("\n") : null;
}
