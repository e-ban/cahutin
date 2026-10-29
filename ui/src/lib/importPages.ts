// Pages importées (autre document Carnet, ou fichier converti par LibreOffice) ajoutées au document.
import { newDocument } from "./factory";
import type { AssetMeta, Document, ImportedPages, ImportReport } from "./types";

function mergedAssets(base: Record<string, AssetMeta | undefined>, parts: ImportedPages[]) {
  return { ...base, ...Object.fromEntries(parts.flatMap((p) => p.assets.map((a) => [a.id, a]))) };
}

/** Pages ajoutées après la page `pageIndex` ; `pageIndex` du résultat : la première page importée. */
export function insertImported(doc: Document, pageIndex: number, parts: ImportedPages[]): { doc: Document; pageIndex: number } {
  const pages = [...doc.pages];
  pages.splice(pageIndex + 1, 0, ...parts.flatMap((p) => p.pages));
  return { doc: { ...doc, pages, assets: mergedAssets(doc.assets, parts) }, pageIndex: pageIndex + 1 };
}

/** Document neuf dont les pages sont uniquement les pages importées (import depuis l'accueil). */
export function newDocumentFrom(parts: ImportedPages[]): Document {
  return { ...newDocument(), pages: parts.flatMap((p) => p.pages), assets: mergedAssets({}, parts) };
}

const count = (n: number, one: string, many: string) => `${n} ${n > 1 ? many : one}`;

export function importReportText(name: string, r: ImportReport): string {
  const details = [
    r.texts && count(r.texts, "texte", "textes"),
    r.images && count(r.images, "image", "images"),
    r.shapes && count(r.shapes, "forme", "formes"),
  ].filter(Boolean);
  let text = `${name} : ${count(r.pages, "page importée", "pages importées")}${details.length ? ` (${details.join(", ")})` : ""}.`;
  if (r.skipped) text += r.skipped > 1 ? ` ${r.skipped} dessins n'ont pas pu être repris.` : " 1 dessin n'a pas pu être repris.";
  return text;
}
