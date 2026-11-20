import { expect, test } from "vitest";
import { newDocument, newPage } from "./factory";
import { importReportText, insertImported, newDocumentFrom, placeImported, replacePage } from "./importPages";
import type { AssetMeta, ImportedPages, ImportReport } from "./types";

const report = (r: Partial<ImportReport> = {}): ImportReport => ({ pages: 1, texts: 0, images: 0, shapes: 0, skipped: 0, ready_made: 0, drawn: 0, shrunk: 0, fallback: false, ...r });
const meta = (id: string): AssetMeta => ({ id, width_px: 10, height_px: 10, mime: "Png" });
const part = (n: number, assets: AssetMeta[] = []): ImportedPages => ({ pages: Array.from({ length: n }, newPage), assets, offered_fonts: [], report: report({ pages: n }) });

test("pages go after the current page in order", () => {
  const doc = newDocument();
  doc.pages.push(newPage());
  const [a, b] = [part(2), part(1)];
  const { doc: next } = insertImported(doc, 0, [a, b]);
  expect(next.pages.map((p) => p.id)).toEqual([doc.pages[0].id, a.pages[0].id, a.pages[1].id, b.pages[0].id, doc.pages[1].id]);
});

test("replacePage puts the filled pages where the template page was", () => {
  const doc = newDocument();
  doc.pages.push(newPage(), newPage());
  const filled = part(3, [meta("photo")]);
  const { doc: next, pageIndex } = replacePage(doc, 1, filled);
  expect(next.pages.map((p) => p.id)).toEqual([doc.pages[0].id, ...filled.pages.map((p) => p.id), doc.pages[2].id]);
  expect(pageIndex).toBe(1);
  expect(Object.keys(next.assets)).toEqual(["photo"]);
});

test("assets are merged", () => {
  const doc = { ...newDocument(), assets: { old: meta("old") } };
  const { doc: next } = insertImported(doc, 0, [part(1, [meta("neuf")])]);
  expect(Object.keys(next.assets).sort()).toEqual(["neuf", "old"]);
});

test("page index points at the first imported page", () => {
  const doc = newDocument();
  doc.pages.push(newPage(), newPage());
  expect(insertImported(doc, 1, [part(2)]).pageIndex).toBe(2);
});

test("a document started from an import holds only the imported pages", () => {
  const a = part(2, [meta("x")]);
  const doc = newDocumentFrom([a]);
  expect(doc.pages.map((p) => p.id)).toEqual(a.pages.map((p) => p.id));
  expect(Object.keys(doc.assets)).toEqual(["x"]);
});

test("report text with skipped drawings", () => {
  expect(importReportText("fiche.docx", report({ pages: 2, texts: 14, images: 1, shapes: 6, skipped: 3 }))).toBe(
    "fiche.docx : 2 pages importées (14 textes, 1 image, 6 formes). 3 dessins n'ont pas pu être repris.",
  );
});

test("report text singular and zero counts", () => {
  expect(importReportText("a.pdf", report({ texts: 1, shapes: 1, skipped: 1 }))).toBe("a.pdf : 1 page importée (1 texte, 1 forme). 1 dessin n'a pas pu être repris.");
  expect(importReportText("vide.pdf", report())).toBe("vide.pdf : 1 page importée.");
  expect(importReportText("graphisme.pdf", report({ texts: 2, drawn: 31 }))).toBe("graphisme.pdf : 1 page importée (2 textes). 31 tracés repris en image fixe, au fond de la page.");
  expect(importReportText("mot.docx", report({ texts: 3, shrunk: 2 }))).toBe("mot.docx : 1 page importée (3 textes). 2 lignes écrites plus petit : leur police manque sur cet ordinateur.");
});

test("a welcome import replaces the untouched document", () => {
  const start = newDocument();
  const a = part(2);
  const r = placeImported(start, 0, [a], start);
  expect(r.replaced).toBe(true);
  expect(r.doc.pages.map((p) => p.id)).toEqual(a.pages.map((p) => p.id));
  expect(r.pageIndex).toBe(0);
});

test("a welcome import keeps work done during the conversion", () => {
  const start = newDocument();
  const edited = { ...start, pages: [...start.pages, newPage()] };
  const r = placeImported(edited, 1, [part(1)], start);
  expect(r.replaced).toBe(false);
  expect(r.doc.pages.length).toBe(3);
  expect(r.doc.pages.slice(0, 2).map((p) => p.id)).toEqual(edited.pages.map((p) => p.id));
});

test("many drawings left out are not counted out loud", () => {
  expect(importReportText("a.odg", report({ texts: 1, skipped: 1032 }))).toBe("a.odg : 1 page importée (1 texte). Quelques dessins n'ont pas pu être repris : comparez avec le fichier d'origine.");
  expect(importReportText("a.odg", report({ texts: 1, skipped: 5 }))).toBe("a.odg : 1 page importée (1 texte). 5 dessins n'ont pas pu être repris.");
});

test("ready-made LibreOffice shapes left out are named and counted", () => {
  expect(importReportText("fiche.odg", report({ texts: 2, images: 2, shapes: 1, skipped: 7, ready_made: 7 }))).toBe(
    "fiche.odg : 1 page importée (2 textes, 2 images, 1 forme). 7 formes toutes faites de LibreOffice (smiley, flèche de liaison…) n'ont pas été reprises : Cahutin ne sait pas les redessiner.",
  );
  expect(importReportText("fiche.odg", report({ texts: 1, skipped: 1, ready_made: 1 }))).toBe(
    "fiche.odg : 1 page importée (1 texte). 1 forme toute faite de LibreOffice (smiley, flèche de liaison…) n'a pas été reprise : Cahutin ne sait pas la redessiner.",
  );
  // Beaucoup de formes : toujours dites en chiffre ; les autres dessins manquants gardent leur message.
  expect(importReportText("fiche.odg", report({ texts: 1, skipped: 42, ready_made: 40 }))).toBe(
    "fiche.odg : 1 page importée (1 texte). 40 formes toutes faites de LibreOffice (smiley, flèche de liaison…) n'ont pas été reprises : Cahutin ne sait pas les redessiner. 2 dessins n'ont pas pu être repris.",
  );
});

test("a drawing taken through its PDF says it is shown as it looks", () => {
  expect(importReportText("fiche.odg", report({ texts: 3, shapes: 46, fallback: true }))).toBe(
    "fiche.odg : 1 page importée (3 textes, 46 formes). La page a été reprise telle qu'elle se voit : ses images sont en morceaux.",
  );
  expect(importReportText("fiche.odg", report({ pages: 2, texts: 3, drawn: 12, fallback: true }))).toBe(
    "fiche.odg : 2 pages importées (3 textes). Les pages ont été reprises telles qu'elles se voient : leurs images sont en morceaux. 12 tracés repris en image fixe, au fond de la page.",
  );
});

test("un carnet importé avec ses chapitres arrive à la fin du chapitre en cours, sans le couper", () => {
  const doc = newDocument();
  doc.pages.push(newPage(), newPage(), newPage());
  doc.pages[1] = { ...doc.pages[1], chapter: { title: "Septembre", color: "#2e86de" } };
  doc.pages[3] = { ...doc.pages[3], chapter: { title: "Octobre", color: "#e67e22" } };
  const withChapter = part(2);
  withChapter.pages[0] = { ...withChapter.pages[0], chapter: { title: "Ferme", color: "#27ae60" } };
  const placed = insertImported(doc, 1, [withChapter]);
  expect(placed.pageIndex).toBe(3);
  expect(placed.doc.pages.map((p) => p.chapter?.title ?? "")).toEqual(["", "Septembre", "", "Ferme", "", "Octobre"]);
  // Sans chapitre, les pages vont juste après la page affichée, dans son chapitre.
  expect(insertImported(doc, 1, [part(1)]).pageIndex).toBe(2);
});
