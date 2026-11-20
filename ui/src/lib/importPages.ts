// Pages importées (autre document Carnet, ou fichier converti par LibreOffice) ajoutées au document.
import { newDocument } from "./factory";
import { carryChapter, chapterEnd } from "./chapters";
import type { AssetMeta, Document, ImportedPages, ImportReport } from "./types";

function mergedAssets(base: Record<string, AssetMeta | undefined>, parts: ImportedPages[]) {
  return { ...base, ...Object.fromEntries(parts.flatMap((p) => p.assets.map((a) => [a.id, a]))) };
}

/** Pages ajoutées après la page `pageIndex` ; `pageIndex` du résultat : la première page importée. Des pages qui ouvrent
 *  leurs propres chapitres (un autre carnet) vont à la fin du chapitre en cours, pour ne pas le couper en deux. */
export function insertImported(doc: Document, pageIndex: number, parts: ImportedPages[]): { doc: Document; pageIndex: number } {
  const added = parts.flatMap((p) => p.pages);
  const at = added.some((p) => p.chapter) ? chapterEnd(doc.pages, pageIndex) : pageIndex + 1;
  const pages = [...doc.pages];
  pages.splice(at, 0, ...added);
  return { doc: { ...doc, pages, assets: mergedAssets(doc.assets, parts) }, pageIndex: at };
}

/** Les pages remplies pour la classe prennent la place de la page modèle `pageIndex` (la première, son chapitre). */
export function replacePage(doc: Document, pageIndex: number, part: ImportedPages): { doc: Document; pageIndex: number } {
  const pages = [...doc.pages];
  pages.splice(pageIndex, 1, ...carryChapter(doc.pages[pageIndex], part.pages));
  return { doc: { ...doc, pages, assets: mergedAssets(doc.assets, [part]) }, pageIndex };
}

/** Document neuf dont les pages sont uniquement les pages importées (import depuis l'accueil). */
export function newDocumentFrom(parts: ImportedPages[]): Document {
  return { ...newDocument(), pages: parts.flatMap((p) => p.pages), assets: mergedAssets({}, parts) };
}

/**
 * Où vont les pages : `startedFrom` est le document au début d'un import lancé depuis l'accueil. S'il
 * n'a pas changé pendant la conversion, il est remplacé par les pages importées ; sinon (on a commencé
 * à travailler entre-temps), les pages s'ajoutent après la page courante, rien n'est perdu.
 */
export function placeImported(current: Document, pageIndex: number, parts: ImportedPages[], startedFrom: Document | null): { doc: Document; pageIndex: number; replaced: boolean } {
  if (startedFrom && current === startedFrom) return { doc: newDocumentFrom(parts), pageIndex: 0, replaced: true };
  return { ...insertImported(current, pageIndex, parts), replaced: false };
}

const count = (n: number, one: string, many: string) => `${n} ${n > 1 ? many : one}`;

/** Nombre de dessins manquants encore dit en chiffre. */
const SKIPPED_SHOWN = 5;

export function importReportText(name: string, r: ImportReport): string {
  const details = [
    r.texts && count(r.texts, "texte", "textes"),
    r.images && count(r.images, "image", "images"),
    r.shapes && count(r.shapes, "forme", "formes"),
  ].filter(Boolean);
  let text = `${name} : ${count(r.pages, "page importée", "pages importées")}${details.length ? ` (${details.join(", ")})` : ""}.`;
  // Un dessin LibreOffice repris par son PDF : tout se voit, mais en morceaux (plus de pictogramme à remplacer).
  if (r.fallback) text += r.pages > 1 ? " Les pages ont été reprises telles qu'elles se voient : leurs images sont en morceaux." : " La page a été reprise telle qu'elle se voit : ses images sont en morceaux.";
  if (r.drawn) text += r.drawn > 1 ? ` ${r.drawn} tracés repris en image fixe, au fond de la page.` : " 1 tracé repris en image fixe, au fond de la page.";
  if (r.shrunk) text += r.shrunk > 1 ? ` ${r.shrunk} lignes écrites plus petit : leur police manque sur cet ordinateur.` : " 1 ligne écrite plus petit : sa police manque sur cet ordinateur.";
  // Les formes toutes faites de LibreOffice, ajoutées une à une, sont toujours dites en chiffre.
  if (r.ready_made)
    text +=
      r.ready_made > 1
        ? ` ${r.ready_made} formes toutes faites de LibreOffice (smiley, flèche de liaison…) n'ont pas été reprises : Cahutin ne sait pas les redessiner.`
        : " 1 forme toute faite de LibreOffice (smiley, flèche de liaison…) n'a pas été reprise : Cahutin ne sait pas la redessiner.";
  // Au-delà de quelques-uns, un nombre (« 1032 dessins ») inquiète sans aider : on dit quoi faire.
  const skipped = r.skipped - r.ready_made;
  if (skipped > SKIPPED_SHOWN) text += " Quelques dessins n'ont pas pu être repris : comparez avec le fichier d'origine.";
  else if (skipped) text += skipped > 1 ? ` ${skipped} dessins n'ont pas pu être repris.` : " 1 dessin n'a pas pu être repris.";
  return text;
}
