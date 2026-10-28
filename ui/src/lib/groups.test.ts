import { expect, test } from "vitest";
import { alignableIds, boundsOf, elementsInRect, expandSelection, groupElements, isLockedSelection, rotateElements, scaleElements, ungroup } from "./groups";
import type { Element, Page } from "./types";

const shape = (id: string, x = 0, extra: Partial<Element> = {}): Element => ({
  id,
  frame: { x_mm: x, y_mm: 10, w_mm: 10, h_mm: 10, rotation_deg: 0 },
  locked: false,
  zone: false,
  kind: { type: "Shape", shape: "Rect", fill: "#ffffff", stroke: { width_mm: 1, color: "#000000" }, corner_radius_mm: 2 },
  ...extra,
});

const text = (id: string, size: number): Element => ({
  id,
  frame: { x_mm: 10, y_mm: 10, w_mm: 40, h_mm: 20, rotation_deg: 0 },
  locked: false,
  zone: false,
  kind: {
    type: "Text",
    content: { paragraphs: [{ align: null, runs: [{ text: "x", bold: false, italic: false, underline: false, color: null, font: null, size_pt: size }] }] },
    style: { font: "Andika", size_pt: size, color: "#222222", align: "Left" },
    prompt: null,
  },
});

const page = (elements: Element[], cells: string[] = []): Page => ({
  id: "p",
  size: { width_mm: 210, height_mm: 297 },
  elements,
  grid: cells.length ? { cols: cells.length, rows: 1, margin_mm: 10, gap_mm: 4, cells: cells.map((element_id, col) => ({ element_id, col, row: 0, col_span: 1, row_span: 1 })) } : null,
});

test("groupElements merges touched groups and keeps members contiguous", () => {
  const p = page([shape("a"), shape("b", 0, { group: "old" }), shape("c"), shape("d", 0, { group: "old" }), shape("e")]);
  const r = groupElements(p, ["a", "b"]);
  expect(r.group).not.toBeNull();
  expect(r.page.elements.map((e) => e.id)).toEqual(["c", "a", "b", "d", "e"]);
  expect(r.page.elements.filter((e) => e.group === r.group).map((e) => e.id)).toEqual(["a", "b", "d"]);
});

test("grid cells are never grouped", () => {
  const p = page([shape("z1"), shape("z2"), shape("free")], ["z1", "z2"]);
  expect(groupElements(p, ["z1", "z2"]).group).toBeNull();
  expect(groupElements(p, ["z1", "z2", "free"]).group).toBeNull();
});

test("ungroup clears the whole group", () => {
  const p = page([shape("a", 0, { group: "g" }), shape("b", 0, { group: "g" }), shape("c", 0, { group: "h" }), shape("d", 0, { group: "h" })]);
  expect(ungroup(p, ["a"]).elements.map((e) => e.group ?? null)).toEqual([null, null, "h", "h"]);
});

test("a group with a locked member is locked", () => {
  const p = page([shape("a", 0, { group: "g", locked: true }), shape("b", 0, { group: "g" }), shape("c")]);
  expect(isLockedSelection(p, ["b"])).toBe(true);
  expect(isLockedSelection(p, ["c"])).toBe(false);
});

test("expandSelection adds the other members", () => {
  const p = page([shape("a", 0, { group: "g" }), shape("b"), shape("c", 0, { group: "g" })]);
  expect(expandSelection(p, ["c"])).toEqual(["a", "c"]);
});

test("scaleElements doubles frames and text sizes from the origin", () => {
  const [s] = scaleElements([shape("a", 20)], 2, { x_mm: 10, y_mm: 10 });
  expect(s.frame).toEqual({ x_mm: 30, y_mm: 10, w_mm: 20, h_mm: 20, rotation_deg: 0 });
  expect(s.kind.type === "Shape" && [s.kind.stroke?.width_mm, s.kind.corner_radius_mm]).toEqual([2, 4]);
  const [t1] = scaleElements([text("t", 14)], 1.5, { x_mm: 0, y_mm: 0 });
  expect(t1.kind.type === "Text" && [t1.kind.style.size_pt, t1.kind.content.paragraphs[0].runs[0].size_pt]).toEqual([21, 21]);
  const [t2] = scaleElements([text("t", 13)], 1.1, { x_mm: 0, y_mm: 0 });
  expect(t2.kind.type === "Text" && t2.kind.style.size_pt).toBe(14.5);
});

test("rotateElements turns positions around the centre and adds to rotation", () => {
  const [r] = rotateElements([shape("a", 20)], 90, { x_mm: 15, y_mm: 15 });
  // Centre (25, 15) tourné de 90° (sens horaire, y vers le bas) autour de (15, 15) : (15, 25).
  expect(r.frame.x_mm).toBeCloseTo(10);
  expect(r.frame.y_mm).toBeCloseTo(20);
  expect(r.frame.rotation_deg).toBe(90);
});

test("boundsOf includes rotated corners", () => {
  const b = boundsOf([{ x_mm: 0, y_mm: 0, w_mm: 10, h_mm: 10, rotation_deg: 45 }]);
  expect(b.w_mm).toBeCloseTo(14.142, 2);
  expect(b.x_mm).toBeCloseTo(-2.071, 2);
});

test("elementsInRect returns only fully covered elements", () => {
  const p = page([shape("in", 20), shape("half", 45), shape("out", 100)]);
  expect(elementsInRect(p, { x_mm: 15, y_mm: 5, w_mm: 35, h_mm: 20 })).toEqual(["in"]);
});

test("grid cells are left out of a group of free elements", () => {
  const p = page([shape("z1"), shape("z2"), shape("f1"), shape("f2")], ["z1", "z2"]);
  const r = groupElements(p, ["z1", "z2", "f1", "f2"]);
  expect(r.group).not.toBeNull();
  expect(r.page.elements.filter((e) => e.group === r.group).map((e) => e.id)).toEqual(["f1", "f2"]);
  expect(r.page.elements.filter((e) => e.id.startsWith("z")).every((e) => !e.group)).toBe(true);
});

test("alignableIds is empty when a group member is locked", () => {
  const p = page([shape("a", 0, { group: "g", locked: true }), shape("b", 10, { group: "g" }), shape("c", 20)]);
  expect(alignableIds(p, ["a", "b"])).toEqual([]);
  expect(alignableIds(p, ["c", "b"])).toEqual([]);
  const q = page([shape("a"), shape("b", 10), shape("z", 20)], ["z"]);
  expect(alignableIds(q, ["a", "b", "z"])).toEqual(["a", "b"]);
});
