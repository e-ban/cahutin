// L'éditeur applique la règle des connecteurs à chaque modification : un seul pas d'annulation, flèche comprise.
import { expect, test } from "vitest";
import { followDoc } from "./connectors";
import { editor } from "./docStore.svelte";
import { newDocument, shapeElement } from "./factory";
import { frameFromEndpoints } from "./lineEnds";
import { pagePoints } from "./linePath";

function setup(locked = false) {
  editor.reset(newDocument(), null);
  const r = { ...shapeElement(editor.page, "Rect"), frame: { x_mm: 60, y_mm: 40, w_mm: 40, h_mm: 20, rotation_deg: 0 } };
  const a0 = shapeElement(editor.page, "Arrow");
  const a = { ...a0, locked, frame: frameFromEndpoints({ x: 10, y: 50 }, { x: 60, y: 50 }, 4, false), kind: { ...a0.kind, attach: { end: { element: r.id, at: "Left" as const } } } };
  editor.reset({ ...editor.doc, pages: [{ ...editor.page, elements: [r, a] }] }, null);
  return { r: r.id, a: a.id };
}
const get = (id: string) => editor.page.elements.find((e) => e.id === id)!;
const tip = (id: string) => {
  const e = get(id);
  if (e.kind.type !== "Shape") throw new Error();
  return pagePoints(e.frame, e.kind, !!e.flip_h, !!e.flip_v)[1];
};
const attach = (id: string) => {
  const e = get(id);
  return e.kind.type === "Shape" ? e.kind.attach : undefined;
};

test("one undo step for label and arrow", () => {
  const { r, a } = setup();
  editor.updateElement(r, (e) => ({ ...e, frame: { ...e.frame, x_mm: 90 } }));
  expect(tip(a).x).toBeCloseTo(90, 9);
  editor.undo();
  expect(tip(a).x).toBeCloseTo(60, 9);
  expect(get(r).frame.x_mm).toBe(60);
  editor.redo();
  expect(tip(a).x).toBeCloseTo(90, 9);
});

test("moving the arrow alone detaches it", () => {
  const { a } = setup();
  editor.updateElement(a, (e) => ({ ...e, frame: { ...e.frame, y_mm: e.frame.y_mm + 10 } }));
  expect(attach(a)).toBeUndefined();
  expect(tip(a).y).toBeCloseTo(60, 9);
});

test("undo a deletion gives the attachment back", () => {
  const { r, a } = setup();
  editor.updatePage((p) => ({ ...p, elements: p.elements.filter((e) => e.id !== r) }));
  expect(attach(a)).toBeUndefined();
  editor.undo();
  expect(attach(a)?.end?.element).toBe(r);
});

test("a locked arrow still follows", () => {
  const { r, a } = setup(true);
  editor.updateElement(r, (e) => ({ ...e, frame: { ...e.frame, y_mm: 70 } }));
  expect(tip(a).y).toBeCloseTo(80, 9);
});

test("a preview follows too", () => {
  const { r, a } = setup();
  const doc = editor.doc;
  editor.preview({ ...doc, pages: doc.pages.map((p) => ({ ...p, elements: p.elements.map((e) => (e.id === r ? { ...e, frame: { ...e.frame, x_mm: 100 } } : e)) })) });
  expect(tip(a).x).toBeCloseTo(100, 9);
  editor.endPreview();
  expect(tip(a).x).toBeCloseTo(100, 9);
});

test("a document without attachment is returned as is", () => {
  const doc = newDocument();
  expect(followDoc(doc, { ...doc })).toEqual({ ...doc });
  const next = { ...doc, pages: [...doc.pages] };
  expect(followDoc(doc, next)).toBe(next);
});

// Actions qui gardent ou retirent l'accroche.
import { convertToShape, removePathPoint, setClosed, setPathPoints, swapLineEnds } from "./elementEdit";
import { extending } from "./extending.svelte";
import { pasteElements } from "./clipboard";

function pathSetup() {
  const ids = setup();
  setPathPoints(ids.a, [{ x: 10, y: 50 }, { x: 30, y: 80 }, { x: 60, y: 50 }]);
  editor.selection = [ids.a];
  return ids;
}

test("swap keeps the anchors at their points", () => {
  const { a, r } = setup();
  editor.selection = [a];
  swapLineEnds();
  expect(attach(a)?.end?.element).toBe(r);
});

test("dragging an elbow keeps the attachment, and closing or converting drops it", () => {
  const { a, r } = pathSetup();
  expect(attach(a)?.end?.element).toBe(r);
  setClosed(true);
  expect(attach(a)).toBeUndefined();
  editor.undo();
  convertToShape();
  expect(attach(a)).toBeUndefined();
});

test("removing an attached end drops it", () => {
  const { a } = pathSetup();
  removePathPoint(a, 2);
  expect(attach(a)).toBeUndefined();
});

test("extending from an attached end detaches it", () => {
  const { a } = pathSetup();
  extending.start(a, "end");
  extending.add({ x: 120, y: 90 }, false, { xs: [], ys: [] }, 0);
  extending.finish();
  expect(attach(a)).toBeUndefined();
});

test("pasting arrow and label together reattaches to the copy; the arrow alone loses it", () => {
  const { a, r } = setup();
  const both = pasteElements(editor.page, [get(r), get(a)]);
  const copyA = both.page.elements.find((e) => e.id === both.ids[1])!;
  expect(copyA.kind.type === "Shape" && copyA.kind.attach?.end?.element).toBe(both.ids[0]);
  const alone = pasteElements(editor.page, [get(a)]);
  const copy = alone.page.elements.find((e) => e.id === alone.ids[0])!;
  expect(copy.kind.type === "Shape" && copy.kind.attach).toBeFalsy();
});
