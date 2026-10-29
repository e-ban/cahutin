import { expect, test } from "vitest";
import { newDocument, newPage } from "./factory";
import { importReportText, insertImported, newDocumentFrom } from "./importPages";
import type { AssetMeta, ImportedPages, ImportReport } from "./types";

const report = (r: Partial<ImportReport> = {}): ImportReport => ({ pages: 1, texts: 0, images: 0, shapes: 0, skipped: 0, ...r });
const meta = (id: string): AssetMeta => ({ id, width_px: 10, height_px: 10, mime: "Png" });
const part = (n: number, assets: AssetMeta[] = []): ImportedPages => ({ pages: Array.from({ length: n }, newPage), assets, offered_fonts: [], report: report({ pages: n }) });

test("pages go after the current page in order", () => {
  const doc = newDocument();
  doc.pages.push(newPage());
  const [a, b] = [part(2), part(1)];
  const { doc: next } = insertImported(doc, 0, [a, b]);
  expect(next.pages.map((p) => p.id)).toEqual([doc.pages[0].id, a.pages[0].id, a.pages[1].id, b.pages[0].id, doc.pages[1].id]);
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
});
