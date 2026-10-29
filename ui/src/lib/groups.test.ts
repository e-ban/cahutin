import { expect, test } from "vitest";
import { alignFrames, distributeFrames } from "./align";
import { alignableIds, alignUnits, arrangeUnits, boundsOf, elementsInRect, expandSelection, groupElements, isLockedSelection, rotateElements, scaleElements, ungroup } from "./groups";
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

test("scaleElements scales a set padding and the text frame, not the default padding", () => {
  const base = text("t", 14);
  if (base.kind.type !== "Text") throw new Error();
  const framed: Element = {
    ...base,
    kind: { ...base.kind, style: { ...base.kind.style, padding_mm: 4 }, frame: { fill: "#ffeeaa", stroke: { width_mm: 1, color: "#333333", style: "Dashed" }, corner_radius_mm: 3, image: null } },
  };
  const [t] = scaleElements([framed], 2, { x_mm: 0, y_mm: 0 });
  expect(t.kind.type === "Text" && [t.kind.style.padding_mm, t.kind.frame?.stroke?.width_mm, t.kind.frame?.corner_radius_mm, t.kind.frame?.stroke?.style]).toEqual([8, 2, 6, "Dashed"]);
  const [plain] = scaleElements([base], 2, { x_mm: 0, y_mm: 0 });
  expect(plain.kind.type === "Text" && (plain.kind.style.padding_mm ?? null)).toBeNull();
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

// Aligner et Répartir : un groupe bouge d'un bloc, ses membres gardent leur place entre eux.
const at = (id: string, x: number, y: number, w: number, h: number, group?: string): Element => ({
  ...shape(id),
  frame: { x_mm: x, y_mm: y, w_mm: w, h_mm: h, rotation_deg: 0 },
  group: group ?? null,
});
const pageWith = (elements: Element[]): Page => ({ id: "p", size: { width_mm: 210, height_mm: 297 }, elements, grid: null, background: null }) as Page;
const frameOf = (p: Page, id: string) => p.elements.find((e) => e.id === id)!.frame;

test("a group is one block for align and distribute", () => {
  // Photo et légende groupées, entre deux formes seules.
  const page = pageWith([at("a", 0, 0, 10, 10), at("photo", 40, 5, 30, 20, "g"), at("legende", 45, 26, 20, 5, "g"), at("b", 150, 0, 10, 10)]);
  expect(alignUnits(page, ["a", "photo", "b"])).toEqual([["a"], ["photo", "legende"], ["b"]]);
  const out = arrangeUnits(page, ["a", "photo", "b"], (f) => distributeFrames(f, "h"));
  // Blocs de 10, 30 et 10 mm entre 0 et 160 : 55 mm d'espace de part et d'autre du groupe.
  expect(frameOf(out, "photo").x_mm).toBe(65);
  expect(frameOf(out, "legende").x_mm).toBe(70);
  expect(frameOf(out, "legende").y_mm).toBe(26);
  expect(frameOf(out, "a")).toEqual(frameOf(page, "a"));
});

test("aligning moves each group as a whole", () => {
  const page = pageWith([at("a", 20, 0, 10, 10), at("photo", 40, 5, 30, 20, "g"), at("legende", 45, 26, 20, 5, "g")]);
  const out = arrangeUnits(page, ["a", "legende"], (f) => alignFrames(f, "left"));
  expect(frameOf(out, "photo").x_mm).toBe(20);
  expect(frameOf(out, "legende").x_mm).toBe(25);
});

test("one group alone has nothing to align or distribute", () => {
  const border = Array.from({ length: 20 }, (_, i) => at(`m${i}`, i * 8, 0, 8, 8, "bordure"));
  const page = pageWith(border);
  expect(alignUnits(page, ["m3"])).toEqual([border.map((e) => e.id)]);
  expect(arrangeUnits(page, ["m3"], (f) => distributeFrames(f, "h"))).toEqual(page);
});

test("a locked member freezes align and distribute", () => {
  const page = pageWith([at("a", 0, 0, 10, 10), at("b", 40, 0, 10, 10, "g"), { ...at("c", 60, 0, 10, 10, "g"), locked: true }, at("d", 150, 0, 10, 10)]);
  expect(alignUnits(page, ["a", "b", "d"])).toEqual([]);
});
