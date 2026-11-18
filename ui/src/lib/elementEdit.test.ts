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

import { updateShape } from "./elementEdit";

test("a thicker stroke grows the hit area of a line with a large head", () => {
  const { a } = threeShapes();
  editor.selection = [a.id];
  setHeadSize("Large");
  updateShape({ stroke: { width_mm: 3, color: "#333333", style: null } });
  expect(shapeOf(a.id).e.frame.h_mm).toBeCloseTo(18, 9);
});

// Chemins : points, courbe, inversion.
import { removePathPoint, setCurve, setPathPoints } from "./elementEdit";
import { pagePoints } from "./linePath";

const pts3 = [{ x: 20, y: 20 }, { x: 60, y: 20 }, { x: 60, y: 60 }];
const pointsOf = (id: string) => {
  const { e, s } = shapeOf(id);
  return pagePoints(e.frame, s, !!e.flip_h, !!e.flip_v);
};

test("setPathPoints is one undo step", () => {
  const { a } = threeShapes();
  const before = shapeOf(a.id).e;
  setPathPoints(a.id, pts3);
  expect(shapeOf(a.id).s.points).toHaveLength(3);
  pointsOf(a.id).forEach((p, i) => {
    expect(p.x).toBeCloseTo(pts3[i].x, 9);
    expect(p.y).toBeCloseTo(pts3[i].y, 9);
  });
  editor.undo();
  expect(shapeOf(a.id).e).toEqual(before);
});

test("swapLineEnds reverses the points", () => {
  const { a } = threeShapes();
  setPathPoints(a.id, pts3);
  editor.selection = [a.id];
  swapLineEnds();
  const got = pointsOf(a.id);
  [...pts3].reverse().forEach((p, i) => {
    expect(got[i].x).toBeCloseTo(p.x, 9);
    expect(got[i].y).toBeCloseTo(p.y, 9);
  });
  expect(shapeOf(a.id).s.ends).toEqual({ start: "Arrow", end: "None", size: "Medium" });
});

test("setCurve ignores straight lines", () => {
  const { a, b } = threeShapes();
  setPathPoints(a.id, pts3);
  editor.selection = [a.id, b.id];
  setCurve(true);
  expect(shapeOf(a.id).s.curve).toBe(true);
  expect(shapeOf(b.id).s.curve).toBeFalsy();
  editor.undo();
  expect(shapeOf(a.id).s.curve).toBeFalsy();
  editor.selection = [b.id];
  const doc = editor.doc;
  setCurve(true);
  expect(editor.doc).toBe(doc);
});

test("removePathPoint down to two points drops points and curve", () => {
  const { a } = threeShapes();
  setPathPoints(a.id, pts3);
  editor.selection = [a.id];
  setCurve(true);
  removePathPoint(a.id, 1);
  const { s } = shapeOf(a.id);
  expect(s.points).toBeUndefined();
  expect(s.curve).toBeFalsy();
  const [p, q] = pointsOf(a.id);
  expect([p.x, p.y, q.x, q.y].map((v) => Math.round(v * 1e6) / 1e6)).toEqual([20, 20, 60, 60]);
});

test("a thicker path keeps its points in place", () => {
  const { a } = threeShapes();
  setPathPoints(a.id, pts3);
  editor.selection = [a.id];
  updateShape({ stroke: { width_mm: 3, color: "#333333", style: null } });
  setHeadSize("Large");
  pointsOf(a.id).forEach((p, i) => {
    expect(p.x).toBeCloseTo(pts3[i].x, 9);
    expect(p.y).toBeCloseTo(pts3[i].y, 9);
  });
});
