import { expect, test } from "vitest";
import { editor } from "./docStore.svelte";
import { copySelection, paste } from "./editActions";
import { newDocument } from "./factory";
import type { Document } from "./types";

test("a photo pasted into another document brings its image with it", () => {
  const meta = { id: "img1", width_px: 40, height_px: 30, mime: "Jpeg" as const };
  const doc: Document = { ...newDocument("source"), assets: { img1: meta } };
  doc.pages[0].elements.push({
    id: "ph",
    frame: { x_mm: 10, y_mm: 10, w_mm: 40, h_mm: 30, rotation_deg: 0 },
    locked: false,
    zone: false,
    kind: { type: "Photo", asset: "img1", crop: { scale: 1, center_x: 0.5, center_y: 0.5 }, corner_radius_mm: 0, border: null, shadow: false },
  });
  editor.reset(doc, null);
  editor.selection = ["ph"];
  copySelection();
  editor.reset(newDocument("autre"), null);
  paste();
  expect(editor.page.elements).toHaveLength(1);
  expect(editor.doc.assets.img1).toEqual(meta);
});

test("groupSelection then ungroupSelection restores ungrouped elements in one undo step each", async () => {
  const { groupSelection, ungroupSelection } = await import("./editActions");
  const { textElement } = await import("./factory");
  const doc = newDocument("g");
  const a = textElement({ x_mm: 0, y_mm: 0, w_mm: 10, h_mm: 10, rotation_deg: 0 });
  const b = textElement({ x_mm: 20, y_mm: 0, w_mm: 10, h_mm: 10, rotation_deg: 0 });
  doc.pages[0].elements.push(a, b);
  editor.reset(doc, null);
  editor.selection = [a.id, b.id];
  groupSelection();
  const g = editor.page.elements[0].group;
  expect(g).toBeTruthy();
  expect(editor.page.elements[1].group).toBe(g);
  ungroupSelection();
  expect(editor.page.elements.every((e) => !e.group)).toBe(true);
  editor.undo();
  expect(editor.page.elements[0].group).toBe(g);
  editor.undo();
  expect(editor.page.elements.every((e) => !e.group)).toBe(true);
});

test("a resize gesture on a group commits one history step", async () => {
  const { applyGroupTransform } = await import("./editActions");
  const { textElement } = await import("./factory");
  const doc = newDocument("t");
  const a = textElement({ x_mm: 10, y_mm: 10, w_mm: 20, h_mm: 10, rotation_deg: 0 });
  const b = textElement({ x_mm: 40, y_mm: 10, w_mm: 20, h_mm: 10, rotation_deg: 0 });
  doc.pages[0].elements.push({ ...a, group: "g" }, { ...b, group: "g" });
  editor.reset(doc, null);
  applyGroupTransform([a.id, b.id], { factor: 2, origin: { x_mm: 10, y_mm: 10 }, deg: 0, dx_mm: 0, dy_mm: 0 });
  const [ea, eb] = editor.page.elements;
  expect(ea.frame).toMatchObject({ x_mm: 10, w_mm: 40 });
  expect(eb.frame).toMatchObject({ x_mm: 70, w_mm: 40 });
  expect(ea.kind.type === "Text" && ea.kind.style.size_pt).toBe(28);
  editor.undo();
  expect(editor.page.elements[1].frame.x_mm).toBe(40);
  expect(editor.canUndo).toBe(false);
});

test("after a double-click, delete removes only the focused member and dissolves the pair", async () => {
  const { deleteSelection } = await import("./editActions");
  const { textElement } = await import("./factory");
  const doc = newDocument("f");
  const a = { ...textElement({ x_mm: 0, y_mm: 0, w_mm: 10, h_mm: 10, rotation_deg: 0 }), group: "g" };
  const b = { ...textElement({ x_mm: 20, y_mm: 0, w_mm: 10, h_mm: 10, rotation_deg: 0 }), group: "g" };
  doc.pages[0].elements.push(a, b);
  editor.reset(doc, null);
  editor.focusedMember = a.id;
  editor.selection = [a.id];
  deleteSelection();
  expect(editor.page.elements.map((e) => [e.id, e.group ?? null])).toEqual([[b.id, null]]);
  editor.focusedMember = null;
});

test("a picture stays square when its size is typed in the inspector", async () => {
  const { setSelectedFrame } = await import("./editActions");
  const { pictoElement } = await import("./pictos.svelte");
  const doc = newDocument("s");
  const fox = pictoElement(doc.pages[0], "1F98A");
  doc.pages[0].elements.push(fox);
  editor.reset(doc, null);
  editor.selection = [fox.id];
  setSelectedFrame("w_mm", 40);
  expect(editor.page.elements[0].frame).toMatchObject({ w_mm: 40, h_mm: 40 });
  setSelectedFrame("h_mm", 22);
  expect(editor.page.elements[0].frame).toMatchObject({ w_mm: 22, h_mm: 22 });
});

test("setBackground changes the current page only", async () => {
  const { setBackground } = await import("./editActions");
  const { newPage } = await import("./factory");
  const doc = newDocument("b");
  doc.pages.push(newPage());
  editor.reset(doc, null);
  editor.pageIndex = 1;
  setBackground({ color: "#d8bc8a", pattern: "Seyes" }, false);
  expect(editor.doc.pages.map((p) => p.background ?? null)).toEqual([null, { color: "#d8bc8a", pattern: "Seyes" }]);
});

test("apply to all pages is one undo step", async () => {
  const { setBackground } = await import("./editActions");
  const { newPage } = await import("./factory");
  const doc = newDocument("b");
  doc.pages.push(newPage(), newPage());
  editor.reset(doc, null);
  setBackground({ color: "#ffffff", pattern: "SmallSquares" }, true);
  expect(editor.doc.pages.every((p) => p.background?.pattern === "SmallSquares")).toBe(true);
  editor.undo();
  expect(editor.doc.pages.every((p) => !p.background)).toBe(true);
  expect(editor.canUndo).toBe(false);
});

function photoDoc(extra: Record<string, unknown> = {}) {
  const doc = newDocument("c");
  const photo = {
    id: "ph",
    frame: { x_mm: 20, y_mm: 30, w_mm: 80, h_mm: 60, rotation_deg: 0 },
    locked: false,
    zone: false,
    kind: { type: "Photo" as const, asset: null, crop: { scale: 1, center_x: 0.5, center_y: 0.5 }, corner_radius_mm: 0, border: null, shadow: false },
    ...extra,
  };
  doc.pages[0].elements.push(photo);
  editor.reset(doc, null);
  return photo;
}

test("caption is grouped below the photo", async () => {
  const { addCaption } = await import("./editActions");
  photoDoc();
  addCaption("ph");
  const [p, c] = editor.page.elements;
  expect(c.frame).toEqual({ x_mm: 20, y_mm: 92, w_mm: 80, h_mm: 10, rotation_deg: 0 });
  expect(c.kind.type === "Text" && [c.kind.style.font, c.kind.style.size_pt, c.kind.style.align, c.kind.prompt]).toEqual(["Andika", 12, "Center", "Légende…"]);
  expect(p.group).toBeTruthy();
  expect(c.group).toBe(p.group);
  expect(editor.selection).toEqual([c.id]);
  expect(editor.editingText).toBe(c.id);
  editor.editingText = null;
  editor.focusedMember = null;
});

test("caption joins the photo's existing group", async () => {
  const { addCaption } = await import("./editActions");
  photoDoc({ group: "g" });
  addCaption("ph");
  expect(editor.page.elements.map((e) => e.group)).toEqual(["g", "g"]);
  editor.editingText = null;
  editor.focusedMember = null;
});

test("a second click selects the existing caption", async () => {
  const { addCaption } = await import("./editActions");
  photoDoc();
  addCaption("ph");
  const id = editor.page.elements[1].id;
  editor.editingText = null;
  editor.selection = ["ph"];
  addCaption("ph");
  expect(editor.page.elements).toHaveLength(2);
  expect(editor.selection).toEqual([id]);
  editor.editingText = null;
  editor.focusedMember = null;
});

test("grid cell caption is not grouped", async () => {
  const { addCaption } = await import("./editActions");
  photoDoc({ zone: true });
  editor.commit({ ...editor.doc, pages: [{ ...editor.page, grid: { cols: 1, rows: 1, margin_mm: 10, gap_mm: 4, cells: [{ element_id: "ph", col: 0, row: 0, col_span: 1, row_span: 1 }] } }] });
  addCaption("ph");
  expect(editor.page.elements.every((e) => !e.group)).toBe(true);
  expect(editor.page.elements).toHaveLength(2);
  editor.editingText = null;
});
