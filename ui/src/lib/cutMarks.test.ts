import { expect, test } from "vitest";
import { CUT_GAP_MM, CUT_LEN_MM, cutMarks, isCutMark } from "./cutMarks";
import type { Element, Page } from "./types";

const block = (id: string, x: number, extra: Partial<Element> = {}): Element => ({
  id,
  frame: { x_mm: x, y_mm: 20, w_mm: 40, h_mm: 30, rotation_deg: 0 },
  locked: false,
  zone: false,
  kind: { type: "Shape", shape: "Rect", fill: "#ffffff", stroke: null, corner_radius_mm: 0 },
  ...extra,
});
const page = (elements: Element[]): Page => ({ id: "p", size: { width_mm: 210, height_mm: 297 }, grid: null, elements });
const marksOf = (p: Page) => p.elements.filter(isCutMark);

test("corner marks: two short lines per corner, outside the block, grouped with it", () => {
  const p = cutMarks(page([block("a", 10)]), ["a"], "corners");
  const marks = marksOf(p);
  expect(marks).toHaveLength(8);
  const a = p.elements.find((e) => e.id === "a")!;
  expect(a.group).toBeTruthy();
  expect(marks.every((m) => m.group === a.group && m.kind.type === "Shape" && m.kind.shape === "Line")).toBe(true);
  // Coin haut gauche, trait horizontal : il finit à CUT_GAP_MM du coin, sur le prolongement du bord du haut.
  const h = marks.find((m) => m.frame.rotation_deg === 0 && m.frame.x_mm < 10)!;
  expect(h.frame.x_mm + h.frame.w_mm).toBeCloseTo(10 - CUT_GAP_MM);
  expect(h.frame.w_mm).toBeCloseTo(CUT_LEN_MM);
  expect(h.frame.y_mm + h.frame.h_mm / 2).toBeCloseTo(20);
});

test("dashed marks: a dashed outline around the block and scissors", () => {
  const p = cutMarks(page([block("a", 10)]), ["a"], "dashed");
  const marks = marksOf(p);
  const rect = marks.find((m) => m.kind.type === "Shape")!;
  expect(rect.kind.type === "Shape" && rect.kind.shape === "Rect" && rect.kind.fill === null && rect.kind.stroke?.style).toBe("Dashed");
  expect(rect.frame).toMatchObject({ x_mm: 10 - CUT_GAP_MM, y_mm: 20 - CUT_GAP_MM, w_mm: 40 + 2 * CUT_GAP_MM, h_mm: 30 + 2 * CUT_GAP_MM });
  expect(marks.some((m) => m.kind.type === "Image" && m.kind.code === "2702")).toBe(true);
});

test("each selected block gets its own marks; a group is one block", () => {
  const p = cutMarks(page([block("a", 10), block("b", 100, { group: "g" }), block("c", 150, { group: "g" })]), ["a", "b"], "corners");
  expect(marksOf(p)).toHaveLength(16);
  expect(marksOf(p).filter((m) => m.group === "g")).toHaveLength(8);
});

test("choosing again replaces the marks; none removes them and the lone block's group", () => {
  let p = cutMarks(page([block("a", 10)]), ["a"], "corners");
  p = cutMarks(p, ["a"], "dashed");
  expect(marksOf(p).filter((m) => m.kind.type === "Shape" && m.kind.shape === "Line")).toHaveLength(0);
  p = cutMarks(p, p.elements.map((e) => e.id), null);
  expect(marksOf(p)).toHaveLength(0);
  expect(p.elements).toHaveLength(1);
  expect(p.elements[0].group ?? null).toBeNull();
});
