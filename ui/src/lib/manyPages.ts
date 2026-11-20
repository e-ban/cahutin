// Actions sur plusieurs pages choisies dans la vue « Toutes les pages » : chacune rend le document d'un coup, pour un
// seul pas d'annulation. Les chapitres des autres pages restent à leur place.
import { duplicatePage } from "./pageOrder";
import type { Document, Page } from "./types";

/** Les pages `ids` retirées du carnet : un chapitre dont la première page part passe à sa première page restante, un
 *  chapitre dont toutes les pages partent disparaît. `taken` : les pages retirées, dans l'ordre, sans chapitre. */
function takeOut(pages: Page[], ids: string[]): { rest: Page[]; taken: Page[] } {
  const rest: Page[] = [];
  const taken: Page[] = [];
  let pending: Page["chapter"] = null;
  for (const p of pages) {
    if (ids.includes(p.id)) {
      if (p.chapter) pending = p.chapter;
      taken.push({ ...p, chapter: null });
    } else {
      rest.push(pending && !p.chapter ? { ...p, chapter: pending } : p);
      pending = null;
    }
  }
  return { rest, taken };
}

/** Ce qui reste d'un chapitre si les pages `ids` partent : faux s'il n'en garde aucune. */
export function keepsPages(pages: Page[], start: number, ids: string[]): boolean {
  for (let i = start; i < pages.length && (i === start || !pages[i].chapter); i++) if (!ids.includes(pages[i].id)) return true;
  return false;
}

/** Les pages `ids` vont à la fin du chapitre qui commence à `start`, ou, avec `start` à -1, avant le premier chapitre.
 *  Le même document si ce chapitre n'a plus de page à lui. */
export function movePagesTo(doc: Document, ids: string[], start: number): Document {
  const mark = start >= 0 ? doc.pages[start]?.chapter : null;
  if (start >= 0 && (!mark || !keepsPages(doc.pages, start, ids))) return doc;
  const { rest, taken } = takeOut(doc.pages, ids);
  if (!taken.length) return doc;
  let at: number;
  if (mark) {
    const k = rest.findIndex((p) => p.chapter === mark);
    const next = rest.findIndex((p, i) => i > k && p.chapter);
    at = next < 0 ? rest.length : next;
  } else {
    const first = rest.findIndex((p) => p.chapter);
    at = first < 0 ? rest.length : first;
  }
  return { ...doc, pages: [...rest.slice(0, at), ...taken, ...rest.slice(at)] };
}

/** Chaque page `ids` suivie de sa copie. */
export function duplicatePages(doc: Document, ids: string[]): Document {
  return { ...doc, pages: doc.pages.flatMap((p) => (ids.includes(p.id) ? [p, duplicatePage(p)] : [p])) };
}

/** Le carnet sans les pages `ids` ; le même document s'il n'en resterait aucune. */
export function deletePages(doc: Document, ids: string[]): Document {
  const { rest } = takeOut(doc.pages, ids);
  return rest.length && rest.length < doc.pages.length ? { ...doc, pages: rest } : doc;
}

/** La place de la page affichée après une action : la même page, ou, supprimée, celle qui a pris sa place. */
export function followPage(before: Page[], after: Page[], index: number): number {
  const id = before[index]?.id;
  const found = after.findIndex((p) => p.id === id);
  if (found >= 0) return found;
  // La page a disparu : la première page restante après elle, sinon la dernière.
  const later = before.slice(index + 1).map((p) => after.findIndex((q) => q.id === p.id)).find((i) => i >= 0);
  return later ?? Math.max(0, after.length - 1);
}
