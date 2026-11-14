import { describe, expect, it } from "vitest";
import pages from "../../../fixtures/print_safety_pages.json";
import { cutMarks } from "./cutMarks";
import { frieze, pageFrame } from "./decor";
import { graphismeRow } from "./graphisme";
import { a3Tiles, FORMATS } from "./pageFormat";
import { SAFE_MARGIN_MM, atRisk, mixedSheets, pagesAtRisk, printWarning } from "./printSafety";
import type { Document, Element, Frame, Page } from "./types";

const frame = (x: number, y: number, w = 40, h = 20, rotation_deg = 0): Frame => ({ x_mm: x, y_mm: y, w_mm: w, h_mm: h, rotation_deg });
const text = (id: string, f: Frame, words = "Bonjour"): Element => ({
  id,
  frame: f,
  locked: false,
  zone: false,
  kind: {
    type: "Text",
    content: { paragraphs: [{ runs: [{ text: words, bold: false, italic: false, underline: false, color: null, font: null, size_pt: null }] }] },
    style: { font: "Andika", size_pt: 16, color: "#000000", align: "Left" },
    prompt: null,
  } as Element["kind"],
});
const photo = (id: string, f: Frame, asset: string | null, zone = false): Element => ({
  id,
  frame: f,
  locked: false,
  zone,
  kind: { type: "Photo", asset, crop: { scale: 1, center_x: 0.5, center_y: 0.5 }, corner_radius_mm: 0, border: null, shadow: false },
});
const band = (id: string, f: Frame): Element => ({ id, frame: f, locked: false, zone: false, kind: { type: "Shape", shape: "Rect", fill: "#ffcc00", stroke: null, corner_radius_mm: 0 } });
const page = (elements: Element[], size = { width_mm: 210, height_mm: 297 }): Page => ({ id: "p", size, grid: null, elements });
const ids = (p: Page) => atRisk(p).map((e) => e.id);
const doc = (ps: Page[]): Document => ({ format_version: 1, title: "", pages: ps, assets: {}, pending_photos: [] });

describe("atRisk", () => {
  it("warns for a text at 4.9 mm from an edge, not at 5.0 mm", () => {
    expect(ids(page([text("a", frame(4.9, 50))]))).toEqual(["a"]);
    expect(ids(page([text("a", frame(5, 50))]))).toEqual([]);
    // Bords droit et bas, à 5,0 mm tout juste, avec des largeurs qui ne tombent pas juste en binaire.
    expect(ids(page([text("a", frame(210 - 5 - 13.3, 297 - 5 - 7.7, 13.3, 7.7))]))).toEqual([]);
    expect(ids(page([text("a", frame(210 - 4.9 - 13.3, 100, 13.3, 7.7))]))).toEqual(["a"]);
  });

  it("judges a turned element on its bounding box", () => {
    // 40 × 20 tourné d'un quart de tour : sa boîte fait 20 × 40, centrée au même point.
    const turned = (x: number) => text("t", frame(x, 100, 40, 20, 90));
    expect(ids(page([turned(4.9 - 10)]))).toEqual(["t"]);
    expect(ids(page([turned(5 - 10)]))).toEqual([]);
    // Le cadre lui-même est à 0 mm du bord, mais une fois tourné il en est à 10 mm.
    expect(ids(page([text("t", frame(0, 100, 40, 20, 90))]))).toEqual([]);
    // À 45°, les coins s'approchent du bord.
    expect(ids(page([text("t", frame(6, 100, 40, 20))]))).toEqual([]);
    expect(ids(page([text("t", frame(6, 100, 40, 20, 45))]))).toEqual(["t"]);
  });

  it("lets the decor be trimmed: background, page frame, frieze, plain band", () => {
    const base = { ...page([]), background: { color: "#d8bc8a", pattern: null } };
    const framed = { ...base, elements: [...pageFrame(base, "thin").map((e) => ({ ...e, frame: frame(1, 1, 208, 295) }))] };
    expect(ids(framed)).toEqual([]);
    const f = frieze(base, [{ code: "1F33C" }], "top");
    const atZero = { ...base, elements: f.map((e) => ({ ...e, frame: { ...e.frame, y_mm: 0 } })) };
    expect(atZero.elements.length).toBeGreaterThanOrEqual(3);
    expect(ids(atZero)).toEqual([]);
    expect(ids(page([band("b", frame(0, 0, 210, 30))]))).toEqual([]);
  });

  it("warns for a lone pictogram, a photo, a table", () => {
    const picto: Element = { id: "i", frame: frame(2, 50, 15, 15), locked: false, zone: false, kind: { type: "Image", code: "1F33C" } };
    const table: Element = { id: "t", frame: frame(20, 293, 100, 30), locked: false, zone: false, kind: { type: "Table", cols: [1], rows: [1], cells: [], border: null } };
    expect(ids(page([picto, photo("p", frame(100, 2), "asset"), table]))).toEqual(["i", "p", "t"]);
  });

  it("warns for a cut mark at 3 mm", () => {
    // Carte à 10 mm du bord gauche : ses traits de coin horizontaux, à gauche, vont de 3 à 8 mm.
    const p = cutMarks(page([band("c", frame(10, 40, 60, 40))]), ["c"], "corners");
    const risky = atRisk(p);
    expect(risky).toHaveLength(2);
    expect(risky.every((e) => e.kind.type === "Shape" && e.kind.shape === "Line" && e.frame.x_mm === 3)).toBe(true);
    // Le trait lui-même compte, pas la bande de clic autour : un trait horizontal à 5 mm du bord du haut n'alerte pas.
    const line = risky[0];
    expect(ids(page([{ ...line, id: "l", frame: { ...line.frame, x_mm: 50, y_mm: 5 - line.frame.h_mm / 2 } }]))).toEqual([]);
    expect(ids(page([{ ...line, id: "l", frame: { ...line.frame, x_mm: 50, y_mm: 4.9 - line.frame.h_mm / 2 } }]))).toEqual(["l"]);
  });

  it("ignores empty template zones, but not filled ones", () => {
    expect(ids(page([photo("z", frame(0, 0), null, true), text("e", frame(0, 50), " ")]))).toEqual([]);
    expect(ids(page([photo("z", frame(0, 0), "asset", true)]))).toEqual(["z"]);
    // Un cadre à lignes vide s'imprime : ses lignes peuvent être coupées.
    const lined = text("l", frame(0, 50), "");
    if (lined.kind.type === "Text") lined.kind.frame = { fill: null, stroke: null, corner_radius_mm: 0, pattern: "Seyes" };
    expect(ids(page([lined]))).toEqual(["l"]);
  });

  it("warns for graphisme lines", () => {
    const row = graphismeRow("ponts", "MS", 2, 100, 200);
    expect(atRisk(page(row)).length).toBeGreaterThan(0);
  });

  it("finds nothing in the examples and templates provided", () => {
    for (const { id, page: p } of pages as unknown as { id: string; page: Page }[]) expect(ids(p), id).toEqual([]);
  });
});

describe("pagesAtRisk", () => {
  it("lists the pages with something at risk", () => {
    expect(pagesAtRisk(doc([page([]), page([text("a", frame(1, 50))]), page([text("b", frame(50, 50))])]))).toEqual([1]);
  });
});

describe("A3 posters on two A4 sheets", () => {
  it("overlap by at least twice the safe margin, so the printed parts meet", () => {
    for (const size of [
      { width_mm: 297, height_mm: 420 },
      { width_mm: 420, height_mm: 297 },
    ]) {
      const [t] = a3Tiles(size);
      const overlap = 2 * FORMATS.A4[0] - Math.max(size.width_mm, size.height_mm) * t.scale;
      expect(overlap).toBeGreaterThanOrEqual(2 * SAFE_MARGIN_MM - 1e-9);
    }
  });
});

describe("mixedSheets", () => {
  const a4 = page([]);
  const a4l = page([], { width_mm: 297, height_mm: 210 });
  const a3 = page([], { width_mm: 297, height_mm: 420 });
  it("tells when the pages need different paper", () => {
    expect(mixedSheets(doc([a4, a4]), false)).toBe(false);
    expect(mixedSheets(doc([a4, a4l]), false)).toBe(true);
    expect(mixedSheets(doc([a4, a3]), false)).toBe(true);
    // En deux feuilles A4, une affiche A3 portrait sort sur des A4 paysage ; une A3 paysage, sur des A4 portrait.
    expect(mixedSheets(doc([a4l, a3]), true)).toBe(false);
    expect(mixedSheets(doc([a4, a3]), true)).toBe(true);
  });
});

describe("printWarning", () => {
  const risky = (...els: Element[]) => page(els);
  it("says nothing when all is well", () => {
    expect(printWarning(doc([page([text("a", frame(50, 50))])]), { linux: true, pdf: false, tiles: false })).toBeNull();
  });

  it("names the page and what is at risk, in plain words", () => {
    const w = printWarning(doc([page([]), page([]), risky(text("a", frame(1, 50)))]), { linux: false, pdf: false, tiles: false })!;
    expect(w.message).toBe("Page 3 : un texte est tout près du bord. Beaucoup d'imprimantes n'impriment pas jusqu'au bord : il risque d'être coupé.");
    expect(w.page).toBe(2);
    expect(w.ids).toEqual(["a"]);
  });

  it("agrees with what is at risk, and names several pages briefly", () => {
    const w = printWarning(doc([risky(photo("a", frame(1, 50), "x")), page([]), risky(photo("b", frame(1, 50), "y"))]), { linux: false, pdf: false, tiles: false })!;
    expect(w.message).toBe("Pages 1 et 3 : des photos sont tout près du bord. Beaucoup d'imprimantes n'impriment pas jusqu'au bord : elles risquent d'être coupées.");
    const mixed = printWarning(doc([risky(photo("a", frame(1, 50), "x"), text("b", frame(1, 100)))]), { linux: false, pdf: false, tiles: false })!;
    expect(mixed.message).toContain("Page 1 : des éléments sont tout près du bord.");
    expect(mixed.message).toContain("ils risquent d'être coupés.");
    const many = doc(Array.from({ length: 9 }, () => risky(text("a", frame(1, 50)))));
    expect(printWarning(many, { linux: false, pdf: false, tiles: false })!.message.startsWith("Pages 1, 2, 3, 4 et 5 autres : des textes")).toBe(true);
  });

  it("counts one cut frame for its marks", () => {
    const p = cutMarks(page([band("c", frame(0, 40, 60, 40))]), ["c"], "dashed");
    expect(printWarning(doc([p]), { linux: false, pdf: false, tiles: false })!.message).toContain("un trait de coupe est tout près du bord");
  });

  it("advises « Taille réelle » for a PDF", () => {
    const w = printWarning(doc([risky(text("a", frame(1, 50)))]), { linux: false, pdf: true, tiles: false })!;
    expect(w.message).toContain("À l'impression du PDF, choisissez « Taille réelle »");
  });

  it("on Linux, warns that mixed formats come out on the same paper", () => {
    const d = doc([page([]), page([], { width_mm: 297, height_mm: 210 })]);
    const w = printWarning(d, { linux: true, pdf: false, tiles: false })!;
    expect(w.message).toContain("même papier");
    expect(w.message).toContain("une impression par format");
    expect(w.message).not.toContain("LibreOffice");
    expect(w.ids).toEqual([]);
    expect(printWarning(d, { linux: false, pdf: false, tiles: false })).toBeNull();
  });

  it("uses no long dashes", () => {
    const d = doc([risky(text("a", frame(1, 50))), page([], { width_mm: 297, height_mm: 210 })]);
    expect(printWarning(d, { linux: true, pdf: true, tiles: false })!.message).not.toMatch(/[–—]/);
  });
});
