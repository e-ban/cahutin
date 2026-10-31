import { expect, test } from "vitest";
import { applyStyle, BUILTIN_STYLES, followsStyle, lookOf, newStyleFrom, redefineStyle, stylesOf } from "./namedStyles";
import type { Document, Element } from "./types";

const text = (id: string, size = 14): Element => ({
  id,
  frame: { x_mm: 0, y_mm: 0, w_mm: 100, h_mm: 20, rotation_deg: 0 },
  locked: false,
  zone: false,
  group: null,
  kind: {
    type: "Text",
    content: { paragraphs: [{ align: "Right", runs: [{ text: "Bonjour", bold: true, italic: false, underline: false, color: "#ff0000", font: "Arial", size_pt: 30 }] }] },
    style: { font: "Andika", size_pt: size, color: "#000000", align: "Left" },
    prompt: null,
  },
});
const doc = (els: Element[]): Document => ({ format_version: 5, title: "", pages: [{ id: "p", size: { width_mm: 210, height_mm: 297 }, elements: els, grid: null, background: null }], assets: {}, pending_photos: [] });
const titre = BUILTIN_STYLES[0];

test("a style gives its look and clears word-level settings, bold kept", () => {
  const t = applyStyle(text("a"), titre);
  expect(t.kind.type === "Text" && [t.kind.style.font, t.kind.style.size_pt, t.kind.style.named]).toEqual(["Playwrite FR Moderne", 28, "titre"]);
  const run = t.kind.type === "Text" ? t.kind.content.paragraphs[0].runs[0] : null;
  expect([run?.font, run?.size_pt, run?.color, run?.bold]).toEqual([null, null, null, true]);
  expect(followsStyle(t, titre)).toBe(true);
});

test("redefining a style changes every text that follows it, on every page", () => {
  const d0 = doc([applyStyle(text("a"), titre), text("b"), applyStyle(text("c"), titre)]);
  const bigger = { ...titre, size_pt: 36 };
  const d = redefineStyle(d0, bigger);
  const sizes = d.pages[0].elements.map((e) => (e.kind.type === "Text" ? e.kind.style.size_pt : 0));
  expect(sizes).toEqual([36, 14, 36]);
  expect(stylesOf(d)[0].size_pt).toBe(36);
  expect(stylesOf(d)).toHaveLength(BUILTIN_STYLES.length);
});

test("a new style comes from a text", () => {
  const made = newStyleFrom(doc([]), text("a", 18), "Date");
  expect(made && stylesOf(made.doc).at(-1)).toMatchObject({ name: "Date", size_pt: 18 });
  expect(newStyleFrom(doc([]), text("a"), "  ")).toBeNull();
  expect(lookOf(text("a"), "x", "X")?.font).toBe("Andika");
});
