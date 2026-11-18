import { expect, test } from "vitest";
import { editor } from "./docStore.svelte";
import { setPathPoints } from "./elementEdit";
import { extending } from "./extending.svelte";
import { newDocument, shapeElement } from "./factory";
import { pagePoints, type Pt } from "./linePath";

const noLines = { xs: [], ys: [] };

function oneLine() {
  editor.reset(newDocument(), null);
  const a = shapeElement(editor.page, "Arrow");
  editor.updatePage((p) => ({ ...p, elements: [a] }));
  editor.selection = [a.id];
  return a.id;
}
const pointsOf = (id: string): Pt[] => {
  const e = editor.page.elements.find((o) => o.id === id)!;
  if (e.kind.type !== "Shape") throw new Error("forme attendue");
  return pagePoints(e.frame, e.kind, !!e.flip_h, !!e.flip_v);
};
const round = (pts: Pt[]) => pts.map((p) => [Math.round(p.x * 1e6) / 1e6, Math.round(p.y * 1e6) / 1e6]);

test("a whole session is one undo step", () => {
  const id = oneLine();
  const before = editor.doc;
  extending.start(id, "end");
  extending.add({ x: 150, y: 200 }, false, noLines, 0);
  extending.add({ x: 170, y: 230 }, false, noLines, 0);
  extending.finish();
  expect(pointsOf(id)).toHaveLength(4);
  editor.undo();
  expect(editor.doc).toEqual(before);
});

test("undoLast removes the last added point, not the session", () => {
  const id = oneLine();
  const start = pointsOf(id);
  extending.start(id, "end");
  extending.add({ x: 150, y: 200 }, false, noLines, 0);
  extending.add({ x: 170, y: 230 }, false, noLines, 0);
  extending.undoLast();
  extending.finish();
  expect(round(pointsOf(id))).toEqual(round([...start, { x: 150, y: 200 }]));
});

test("extending from start prepends", () => {
  const id = oneLine();
  const start = pointsOf(id);
  extending.start(id, "start");
  extending.add({ x: 20, y: 250 }, false, noLines, 0);
  extending.finish();
  expect(round(pointsOf(id))).toEqual(round([{ x: 20, y: 250 }, ...start]));
});

test("stops at 200 points", () => {
  const id = oneLine();
  setPathPoints(id, Array.from({ length: 199 }, (_, i) => ({ x: 5 + i, y: 100 + (i % 2) * 10 })));
  extending.start(id, "end");
  extending.add({ x: 10, y: 280 }, false, noLines, 0);
  expect(extending.active).toBe(true);
  extending.add({ x: 30, y: 280 }, false, noLines, 0);
  expect(extending.active).toBe(false);
  expect(extending.full).toBe(true);
  expect(pointsOf(id)).toHaveLength(200);
});

test("finish without any point changes nothing", () => {
  const id = oneLine();
  const before = editor.doc;
  extending.start(id, "end");
  extending.finish();
  expect(editor.doc).toBe(before);
  expect(extending.active).toBe(false);
});
