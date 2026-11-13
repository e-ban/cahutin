// Ce que Cahutin met dans le presse-papiers du système quand on copie des éléments de la page : leur texte, et un
// jeton sous un type propre à Cahutin. Au collage, le presse-papiers qui contient encore ce jeton (ou, si le type
// s'est perdu, exactement ce texte) est celui de la dernière copie dans Cahutin ; sinon, on a copié ailleurs depuis.
import { newId } from "./factory";
import type { Element } from "./types";

export const COPY_MIME = "application/x-cahutin-copy";

export interface CopyMark {
  token: string;
  text: string;
}

let mark: CopyMark | null = null;
let pending: CopyMark | null = null;

/** Le texte des éléments copiés, un bloc par ligne ; rien pour une photo ou une forme. */
export function plainTextOf(elements: Element[]): string {
  return elements
    .flatMap((e) => (e.kind.type === "Text" ? [e.kind.content.paragraphs.map((p) => p.runs.map((r) => r.text).join("")).join("\n")] : []))
    .filter((t) => t)
    .join("\n");
}

/** Dernière marque écrite, ou null si l'écriture n'a pas pu se faire (Cahutin ne sait pas qui est le plus récent). */
export const lastMark = () => mark;

/** Écrit le texte et le jeton dans le presse-papiers du système, par l'événement « copy » de la vue web. */
export function markCopy(text: string) {
  mark = null;
  if (typeof document === "undefined") return;
  pending = { token: newId(), text };
  try {
    document.execCommand("copy");
  } catch {
    // Vue web qui refuse : la marque reste inconnue.
  }
  pending = null;
}

function onCopy(ev: ClipboardEvent) {
  if (!pending || !ev.clipboardData) return;
  ev.clipboardData.setData("text/plain", pending.text);
  ev.clipboardData.setData(COPY_MIME, pending.token);
  ev.preventDefault();
  mark = pending;
}

if (typeof window !== "undefined") window.addEventListener("copy", onCopy, true);
