// Taper pour écrire : sur un texte sélectionné (ou un groupe qui n'en a qu'un, une bulle de parole), une lettre
// tapée ouvre l'écriture et s'y écrit, Entrée ou F2 l'ouvre sans rien écrire, comme dans les autres logiciels.
// Avant, la frappe était perdue tant qu'on n'avait pas double-cliqué.
import type { Page } from "./types";

/** Le texte où écrire pour cette sélection : un texte seul, ou le seul texte d'un groupe ; jamais verrouillé. */
export function textToWrite(page: Page, selection: string[]): string | null {
  const els = page.elements.filter((e) => selection.includes(e.id));
  if (els.length === 0) return null;
  if (els.length > 1) {
    const group = els[0].group;
    if (!group || els.some((e) => e.group !== group)) return null;
  }
  const texts = els.filter((e) => e.kind.type === "Text" && !e.locked);
  return texts.length === 1 ? texts[0].id : null;
}

type KeyInfo = Pick<KeyboardEvent, "key" | "ctrlKey" | "metaKey" | "altKey">;

/** La touche ouvre-t-elle l'écriture, et avec quoi : un caractère tapé, ou rien (Entrée, F2). */
export function startsWriting(ev: KeyInfo): { insert: string } | null {
  // Ctrl+Alt, c'est AltGr sous Windows (€, @, #...) : un caractère, pas un raccourci.
  const shortcut = ev.metaKey || (ev.ctrlKey && !ev.altKey);
  if (shortcut) return null;
  if (ev.key === "Enter" || ev.key === "F2") return ev.ctrlKey || ev.altKey ? null : { insert: "" };
  return [...ev.key].length === 1 ? { insert: ev.key } : null;
}
