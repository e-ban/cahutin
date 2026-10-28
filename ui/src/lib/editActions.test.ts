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
