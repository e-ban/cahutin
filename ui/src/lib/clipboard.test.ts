import { expect, test } from "vitest";
import { copyElements, duplicateElements, pasteElements, removeElements, reorder, reorderStep } from "./clipboard";
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

test("pasting a table remaps its cells", () => {
  const cell = (id: string) => el(id, 10, { group: "g" });
  const table: Element = {
    ...el("t", 10, { group: "g" }),
    kind: { type: "Table", cols: [1, 1], rows: [1], cells: ["a", "b"].map((element_id, col) => ({ element_id, col, row: 0, col_span: 1, row_span: 1 })), border: null },
  };
  const p = page([cell("a"), cell("b"), table]);
  const r = pasteElements(p, copyElements(p, ["a", "b", "t"]));
  const pasted = r.page.elements.filter((e) => r.ids.includes(e.id));
  const t = pasted.find((e) => e.kind.type === "Table")!;
  expect(t.kind.type === "Table" && t.kind.cells.map((c) => c.element_id)).toEqual(pasted.slice(0, 2).map((e) => e.id));
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

test("a pasted group of groups gets its own ids at every level", () => {
  const p = page([el("a", 10, { group: "g", subgroups: ["s"] }), el("b", 20, { group: "g", subgroups: ["s"] }), el("c", 30, { group: "g" })]);
  const { page: out, ids } = pasteElements(p, copyElements(p, ["a", "b", "c"]));
  const [a, b, c] = ids.map((id) => out.elements.find((e) => e.id === id)!);
  expect(a.group).not.toBe("g");
  expect(a.subgroups).toHaveLength(1);
  expect(a.subgroups![0]).not.toBe("s");
  expect([b.group, b.subgroups]).toEqual([a.group, a.subgroups]);
  expect(c.group).toBe(a.group);
  expect(c.subgroups).toBeUndefined();
});

const ids = (p: Page) => p.elements.map((e) => e.id);

test("reorderStep moves one step forward or backward among the page's elements", () => {
  const p = page([el("a"), el("b"), el("c"), el("d")]);
  expect(ids(reorderStep(p, ["b"], "forward"))).toEqual(["a", "c", "b", "d"]);
  expect(ids(reorderStep(p, ["c"], "backward"))).toEqual(["a", "c", "b", "d"]);
});

test("reorderStep keeps the order between several chosen elements", () => {
  const p = page([el("a"), el("b"), el("c"), el("d")]);
  expect(ids(reorderStep(p, ["a", "b"], "forward"))).toEqual(["c", "a", "b", "d"]);
  expect(ids(reorderStep(p, ["a", "c"], "forward"))).toEqual(["b", "a", "d", "c"]);
  expect(ids(reorderStep(p, ["b", "d"], "backward"))).toEqual(["b", "a", "d", "c"]);
});

test("reorderStep changes nothing, and returns the same page, at the very front or back", () => {
  const p = page([el("a"), el("b"), el("c")]);
  expect(reorderStep(p, ["c"], "forward")).toBe(p);
  expect(reorderStep(p, ["b", "c"], "forward")).toBe(p);
  expect(reorderStep(p, ["a"], "backward")).toBe(p);
});

test("reorderStep moves a group as one block and steps over a whole group", () => {
  const p = page([el("a"), el("g1", 10, { group: "g" }), el("g2", 10, { group: "g" }), el("b")]);
  expect(ids(reorderStep(p, ["a"], "forward"))).toEqual(["g1", "g2", "a", "b"]);
  expect(ids(reorderStep(p, ["g1"], "forward"))).toEqual(["a", "b", "g1", "g2"]);
});

test("inside a group, reorderStep moves among the group's members only", () => {
  const p = page([el("x"), el("g1", 10, { group: "g" }), el("y"), el("g2", 10, { group: "g" }), el("g3", 10, { group: "g" })]);
  expect(ids(reorderStep(p, ["g1"], "forward", ["g"]))).toEqual(["x", "g2", "y", "g1", "g3"]);
  expect(reorderStep(p, ["g3"], "forward", ["g"])).toBe(p);
  expect(reorderStep(p, ["g1"], "backward", ["g"])).toBe(p);
});
