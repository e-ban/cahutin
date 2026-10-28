import { expect, test } from "vitest";
import { copyElements, duplicateElements, pasteElements, removeElements, reorder } from "./clipboard";
import type { Element, Page } from "./types";

const el = (id: string, x = 10, extra: Partial<Element> = {}): Element => ({
  id,
  frame: { x_mm: x, y_mm: 20, w_mm: 30, h_mm: 20, rotation_deg: 0 },
  locked: false,
  zone: false,
  kind: { type: "Shape", shape: "Rect", fill: "#ffffff", stroke: null, corner_radius_mm: 0 },
  ...extra,
});

const page = (elements: Element[], gridIds: string[] = []): Page => ({
  id: "page",
  size: { width_mm: 210, height_mm: 297 },
  elements,
  grid: gridIds.length
    ? { cols: gridIds.length, rows: 1, margin_mm: 10, gap_mm: 4, cells: gridIds.map((element_id, col) => ({ element_id, col, row: 0, col_span: 1, row_span: 1 })) }
    : null,
});

test("copyElements returns deep copies in page order", () => {
  const p = page([el("a"), el("b"), el("c")]);
  const copied = copyElements(p, ["c", "a"]);
  expect(copied.map((e) => e.id)).toEqual(["a", "c"]);
  copied[0].frame.x_mm = 999;
  expect(p.elements[0].frame.x_mm).toBe(10);
});

test("pasteElements with an empty clipboard leaves the page unchanged", () => {
  const p = page([el("a")]);
  const r = pasteElements(p, []);
  expect(r.page).toBe(p);
  expect(r.ids).toEqual([]);
});

test("pasteElements onto a grid page adds free, movable copies offset by 5 mm", () => {
  const zone = el("z", 10, { zone: true });
  const p = page([zone], ["z"]);
  const r = pasteElements(p, copyElements(p, ["z"]));
  expect(r.ids).toHaveLength(1);
  expect(r.ids[0]).not.toBe("z");
  const pasted = r.page.elements.find((e) => e.id === r.ids[0])!;
  expect(pasted.zone).toBe(false);
  expect(pasted.locked).toBe(false);
  expect(pasted.frame.x_mm).toBe(15);
  expect(pasted.frame.y_mm).toBe(25);
  expect(r.page.grid!.cells.map((c) => c.element_id)).toEqual(["z"]);
});

test("duplicateElements copies the selection next to it", () => {
  const p = page([el("a"), el("b", 50)]);
  const r = duplicateElements(p, ["b"]);
  expect(r.page.elements).toHaveLength(3);
  expect(r.page.elements[2].frame.x_mm).toBe(55);
  expect(r.ids).toEqual([r.page.elements[2].id]);
});

test("reorder brings to front or sends to back, keeping relative order", () => {
  const p = page([el("a"), el("b"), el("c"), el("d")]);
  expect(reorder(p, ["b", "a"], "front").elements.map((e) => e.id)).toEqual(["c", "d", "a", "b"]);
  expect(reorder(p, ["d", "c"], "back").elements.map((e) => e.id)).toEqual(["c", "d", "a", "b"]);
});

test("removeElements keeps locked elements and cleans the grid", () => {
  const p = page([el("a", 10, { zone: true }), el("b", 50, { zone: true }), el("l", 90, { locked: true })], ["a", "b"]);
  const out = removeElements(p, ["a", "l"]);
  expect(out.elements.map((e) => e.id)).toEqual(["b", "l"]);
  expect(out.grid!.cells.map((c) => c.element_id)).toEqual(["b"]);
});
