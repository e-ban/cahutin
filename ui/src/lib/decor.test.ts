import { expect, test } from "vitest";
import { blockFrieze, frieze, pageFrame, toggleMotif } from "./decor";
import type { Element, Page } from "./types";

const a4 = (): Page => ({ id: "p", size: { width_mm: 210, height_mm: 297 }, elements: [], grid: null });

test("pageFrame double makes a grouped pair inside the margin", () => {
  const els = pageFrame(a4(), "double");
  expect(els).toHaveLength(2);
  expect(els[0].group).toBeTruthy();
  expect(els[0].group).toBe(els[1].group);
  expect(els.map((e) => e.frame.x_mm)).toEqual([8, 11]);
  expect(els.map((e) => e.kind.type === "Shape" && e.kind.stroke?.width_mm)).toEqual([0.5, 1.5]);
  expect(els.every((e) => e.kind.type === "Shape" && e.kind.fill === null)).toBe(true);
  const thin = pageFrame(a4(), "thin");
  expect(thin).toHaveLength(1);
  expect(thin[0].group ?? null).toBeNull();
  expect(thin[0].frame).toMatchObject({ x_mm: 8, y_mm: 8, w_mm: 194, h_mm: 281 });
  expect(pageFrame(a4(), "rounded")[0].kind.type === "Shape" && pageFrame(a4(), "rounded")[0].kind.corner_radius_mm).toBe(8);
});

test("frieze top spaces copies evenly within the margins", () => {
  const els = frieze(a4(), ["2600"], "top");
  const xs = els.map((e) => e.frame.x_mm);
  expect(xs[0]).toBe(8);
  expect(xs.at(-1)! + 15).toBeCloseTo(202, 1);
  const gaps = xs.slice(1).map((x, i) => x - xs[i]);
  expect(Math.max(...gaps) - Math.min(...gaps)).toBeLessThan(0.11);
  expect(els.every((e) => e.frame.y_mm === 8 && e.frame.w_mm === 15)).toBe(true);
  expect(els.length).toBe(Math.floor((194 + 3) / (15 + 3)));
});

test("frieze around has no duplicate corner", () => {
  const els = frieze(a4(), ["2600"], "around");
  const keys = els.map((e) => `${e.frame.x_mm},${e.frame.y_mm}`);
  expect(new Set(keys).size).toBe(keys.length);
  expect(keys).toContain("8,8");
  expect(keys).toContain("187,274");
});

test("frieze copies share one group", () => {
  const els = frieze(a4(), ["1F98A"], "bottom");
  expect(new Set(els.map((e) => e.group)).size).toBe(1);
  expect(els[0].group).toBeTruthy();
  expect(els.every((e) => e.kind.type === "Image" && e.kind.code === "1F98A")).toBe(true);
});

const codes = (els: ReturnType<typeof frieze>) => els.map((e) => (e.kind.type === "Image" ? e.kind.code : ""));

test("frieze repeats the motif sequence along a row", () => {
  const els = frieze(a4(), ["A", "B", "C"], "top");
  expect(codes(els).slice(0, 7)).toEqual(["A", "B", "C", "A", "B", "C", "A"]);
});

test("frieze around is one continuous clockwise loop", () => {
  const els = frieze(a4(), ["A", "B"], "around");
  // Ordre horaire : haut de gauche à droite, droite de haut en bas, bas de droite à gauche, gauche de bas en haut.
  const pts = els.map((e) => [e.frame.x_mm, e.frame.y_mm]);
  const top = pts.filter(([, y]) => y === 8).length;
  expect(pts.slice(0, top).every(([, y], i) => y === 8 && (i === 0 || pts[i][0] > pts[i - 1][0]))).toBe(true);
  const right = pts.findIndex(([x, y]) => x === 187 && y > 8);
  expect(right).toBe(top);
  const last = pts.at(-1)!;
  expect(last[0]).toBe(8);
  expect(last[1]).toBeGreaterThan(8);
  expect(codes(els).every((c, i) => c === (i % 2 === 0 ? "A" : "B"))).toBe(true);
});

test("toggleMotif adds at the end and removes when present", () => {
  expect(toggleMotif(["A"], "B")).toEqual(["A", "B"]);
  expect(toggleMotif(["A", "B", "C"], "B")).toEqual(["A", "C"]);
  expect(toggleMotif(["A"], "A")).toEqual([]);
});

const block = (extra: Partial<Element> = {}): Element => ({
  id: "b",
  frame: { x_mm: 50, y_mm: 60, w_mm: 60, h_mm: 40, rotation_deg: 0 },
  locked: false,
  zone: false,
  kind: { type: "Shape", shape: "Rect", fill: "#ffffff", stroke: null, corner_radius_mm: 0 },
  ...extra,
});
const withBlock = (b: Element, more: Element[] = []): Page => ({ ...a4(), elements: [b, ...more] });

test("block frieze straddles the block edges and is grouped with it", () => {
  const page = blockFrieze(withBlock(block()), "b", ["A", "B"])!;
  const [b, ...pics] = page.elements;
  expect(b.group).toBeTruthy();
  expect(pics.length).toBeGreaterThan(8);
  expect(pics.every((p) => p.group === b.group && p.kind.type === "Image")).toBe(true);
  // Taille : le quart du plus petit côté (40 mm), centrée sur le bord.
  expect(pics[0].frame).toMatchObject({ x_mm: 45, y_mm: 55, w_mm: 10, h_mm: 10 });
  expect(codes(pics).slice(0, 4)).toEqual(["A", "B", "A", "B"]);
});

test("block frieze joins the block's existing group", () => {
  const other = { ...block(), id: "o", group: "g1" };
  const page = blockFrieze(withBlock(block({ group: "g1" }), [other]), "b", ["A"])!;
  expect(new Set(page.elements.map((e) => e.group))).toEqual(new Set(["g1"]));
});

test("block frieze follows a rotated block", () => {
  const page = blockFrieze(withBlock(block({ frame: { x_mm: 50, y_mm: 60, w_mm: 60, h_mm: 40, rotation_deg: 90 } })), "b", ["A"])!;
  const pics = page.elements.slice(1);
  expect(pics.every((p) => p.frame.rotation_deg === 90)).toBe(true);
  // Le premier motif (coin haut gauche avant rotation, centre (50, 60)) tourne de 90° autour du centre (80, 80) : (100, 50).
  const c = { x: pics[0].frame.x_mm + 5, y: pics[0].frame.y_mm + 5 };
  expect(c.x).toBeCloseTo(100, 0);
  expect(c.y).toBeCloseTo(50, 0);
});

test("block frieze size stays between 6 and 15 mm, and grid cells get none", () => {
  const small = blockFrieze(withBlock(block({ frame: { x_mm: 0, y_mm: 0, w_mm: 16, h_mm: 16, rotation_deg: 0 } })), "b", ["A"])!;
  expect(small.elements[1].frame.w_mm).toBe(6);
  const big = blockFrieze(withBlock(block({ frame: { x_mm: 0, y_mm: 0, w_mm: 150, h_mm: 120, rotation_deg: 0 } })), "b", ["A"])!;
  expect(big.elements[1].frame.w_mm).toBe(15);
  const grid: Page = { ...withBlock(block()), grid: { cols: 1, rows: 1, margin_mm: 0, gap_mm: 0, cells: [{ element_id: "b", col: 0, row: 0, col_span: 1, row_span: 1, kind: "Photo" }] } as unknown as Page["grid"] };
  expect(blockFrieze(grid, "b", ["A"])).toBeNull();
  expect(blockFrieze(withBlock(block()), "b", [])).toBeNull();
});
