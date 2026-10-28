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

test("pasting a group twice makes two groups", () => {
  const p = page([el("a", 10, { group: "g" }), el("b", 50, { group: "g" })]);
  const copied = copyElements(p, ["a", "b"]);
  const first = pasteElements(p, copied);
  const second = pasteElements(first.page, copied);
  const groupsOf = (ids: string[], pg: Page) => new Set(pg.elements.filter((e) => ids.includes(e.id)).map((e) => e.group));
  const g1 = groupsOf(first.ids, second.page);
  const g2 = groupsOf(second.ids, second.page);
  expect(g1.size).toBe(1);
  expect(g2.size).toBe(1);
  expect([...g1][0]).not.toBe("g");
  expect([...g1][0]).not.toBe([...g2][0]);
});

test("removing a member dissolves a group of one", () => {
  const p = page([el("a", 10, { group: "g" }), el("b", 50, { group: "g" }), el("c", 90, { group: "h" }), el("d", 90, { group: "h" }), el("e", 90, { group: "h" })]);
  const out = removeElements(p, ["a", "c"]);
  expect(out.elements.map((e) => [e.id, e.group ?? null])).toEqual([["b", null], ["d", "h"], ["e", "h"]]);
});

test("reorder moves the whole group", () => {
  const p = page([el("a", 0, { group: "g" }), el("b"), el("c", 0, { group: "g" }), el("d")]);
  expect(reorder(p, ["a"], "front").elements.map((e) => e.id)).toEqual(["b", "d", "a", "c"]);
});
