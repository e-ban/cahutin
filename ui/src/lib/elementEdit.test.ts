import { expect, test } from "vitest";
import { textFrame, textStyle } from "./elementEdit";
import type { Element } from "./types";

const text: Element = {
  id: "t",
  frame: { x_mm: 0, y_mm: 0, w_mm: 50, h_mm: 20, rotation_deg: 0 },
  locked: false,
  zone: false,
  kind: { type: "Text", content: { paragraphs: [] }, style: { font: "Andika", size_pt: 14, color: "#222222", align: "Left" }, prompt: null },
};

test("a text frame is created by its first setting and removed when nothing is left", () => {
  const filled = textFrame({ fill: "#ffeeaa" })(text);
  expect(filled.kind.type === "Text" && filled.kind.frame).toEqual({ fill: "#ffeeaa", stroke: null, corner_radius_mm: 0, image: null, pattern: null });
  const rounded = textFrame({ corner_radius_mm: 5 })(filled);
  const cleared = textFrame({ fill: null })(rounded);
  expect(cleared.kind.type === "Text" && cleared.kind.frame).toBeNull();
  // Des lignes d'écriture seules font un cadre.
  expect(textFrame({ pattern: "Seyes" })(text).kind).toMatchObject({ frame: { pattern: "Seyes" } });
  // Des coins seuls ne font pas un cadre.
  expect(textFrame({ corner_radius_mm: 5 })(text).kind.type === "Text" && textFrame({ corner_radius_mm: 5 })(text).kind).toMatchObject({ frame: null });
});

test("the default margin and top alignment are stored as absent", () => {
  const back = textStyle({ padding_mm: 1.5, valign: "Top" })(textStyle({ padding_mm: 4, valign: "Middle" })(text));
  expect(back.kind.type === "Text" && [back.kind.style.padding_mm, back.kind.style.valign]).toEqual([null, null]);
});

test("hollow letters off are stored as absent, so the format does not change", () => {
  const on = textStyle({ outline: true })(text);
  expect(on.kind.type === "Text" && on.kind.style.outline).toBe(true);
  const off = textStyle({ outline: false })(on);
  expect(off.kind.type === "Text" && off.kind.style.outline).toBeNull();
});

// Terminaisons des traits : plusieurs traits se règlent ensemble, en un seul pas d'annulation.
import { editor } from "./docStore.svelte";
import { newDocument, shapeElement } from "./factory";
import { setHeadSize, setLineEnd, swapLineEnds } from "./elementEdit";

function threeShapes() {
  editor.reset(newDocument(), null);
  const a = shapeElement(editor.page, "Arrow");
  const b = shapeElement(editor.page, "Line");
  const r = shapeElement(editor.page, "Rect");
  editor.updatePage((p) => ({ ...p, elements: [a, b, r] }));
  editor.selection = [a.id, b.id, r.id];
  return { a, b, r };
}
const shapeOf = (id: string) => {
  const e = editor.page.elements.find((o) => o.id === id)!;
  if (e.kind.type !== "Shape") throw new Error("forme attendue");
  return { e, s: e.kind };
};

test("updateLines only touches lines, one undo step", () => {
  const { a, b, r } = threeShapes();
  const rect = shapeOf(r.id).e;
  setLineEnd("start", "Circle");
  expect(shapeOf(a.id).s.ends).toEqual({ start: "Circle", end: "Arrow", size: "Medium" });
  expect(shapeOf(b.id).s.ends).toEqual({ start: "Circle", end: "None", size: "Medium" });
  expect(shapeOf(r.id).e).toEqual(rect);
  editor.undo();
  expect(shapeOf(a.id).s.ends).toBeUndefined();
  expect(shapeOf(b.id).s.ends).toBeUndefined();
});

test("swapLineEnds exchanges ends", () => {
  const { a } = threeShapes();
  editor.selection = [a.id];
  setLineEnd("start", "Circle");
  setHeadSize("Large");
  const before = shapeOf(a.id).e;
  swapLineEnds();
  const after = shapeOf(a.id);
  expect(after.s.ends).toEqual({ start: "Arrow", end: "Circle", size: "Large" });
  expect(after.e.flip_h).toEqual(before.flip_h);
  expect(after.e.frame.x_mm).toBe(before.frame.x_mm);
  expect(after.e.frame.w_mm).toBe(before.frame.w_mm);
});

test("back to default stores nothing", () => {
  const { a } = threeShapes();
  editor.selection = [a.id];
  setLineEnd("start", "Square");
  setLineEnd("start", "None");
  expect(shapeOf(a.id).s.ends ?? null).toBeNull();
});

test("a large head grows the hit area of a thick line", () => {
  const { a } = threeShapes();
  editor.selection = [a.id];
  editor.updateElement(a.id, (e) => (e.kind.type === "Shape" ? { ...e, kind: { ...e.kind, stroke: { width_mm: 3, color: "#333333", style: null } } } : e));
  setHeadSize("Large");
  expect(shapeOf(a.id).e.frame.h_mm).toBeCloseTo(18, 9);
});
