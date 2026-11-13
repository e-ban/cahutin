import { expect, test } from "vitest";
import { alignFrames, distributeFrames, pageArea } from "./align";
import { arrangeUnits, boundsOf } from "./groups";
import type { Element, Frame, Page } from "./types";

const f = (x_mm: number, y_mm: number, w_mm: number, h_mm: number): Frame => ({ x_mm, y_mm, w_mm, h_mm, rotation_deg: 0 });

test("alignFrames left / hcenter / bottom", () => {
  const frames = [f(10, 10, 20, 10), f(40, 30, 40, 20)];
  expect(alignFrames(frames, "left").map((x) => x.x_mm)).toEqual([10, 10]);
  expect(alignFrames(frames, "hcenter").map((x) => x.x_mm)).toEqual([35, 25]);
  expect(alignFrames(frames, "bottom").map((x) => x.y_mm + x.h_mm)).toEqual([50, 50]);
  expect(alignFrames(frames, "right").map((x) => x.x_mm + x.w_mm)).toEqual([80, 80]);
});

test("distributeFrames equalizes gaps and keeps the outer frames", () => {
  const frames = [f(0, 0, 10, 10), f(15, 0, 10, 10), f(90, 0, 10, 10)];
  const out = distributeFrames(frames, "h");
  expect(out[0].x_mm).toBe(0);
  expect(out[2].x_mm).toBe(90);
  expect(out[1].x_mm).toBe(45);
  expect(distributeFrames(frames.slice(0, 2), "h")).toEqual(frames.slice(0, 2));
});

test("distributeFrames sorts by position but returns frames in input order", () => {
  const frames = [f(0, 90, 10, 10), f(0, 0, 10, 10), f(0, 20, 10, 10)];
  expect(distributeFrames(frames, "v").map((x) => x.y_mm)).toEqual([90, 0, 45]);
});

test("alignFrames on the page: a lone block goes to the 10 mm margin, or to the page centre", () => {
  const page = { left: 10, right: 200, top: 10, bottom: 287 };
  const one = [f(50, 60, 40, 20)];
  expect(alignFrames(one, "left", page)[0].x_mm).toBe(10);
  expect(alignFrames(one, "hcenter", page)[0].x_mm).toBe(85);
  expect(alignFrames(one, "right", page)[0].x_mm).toBe(160);
  expect(alignFrames(one, "top", page)[0].y_mm).toBe(10);
  expect(alignFrames(one, "vmiddle", page)[0].y_mm).toBe(138.5);
  expect(alignFrames(one, "bottom", page)[0].y_mm).toBe(267);
});

test("pageArea keeps the 10 mm margin of the snapping lines", () => {
  expect(pageArea({ width_mm: 210, height_mm: 297 })).toEqual({ left: 10, right: 200, top: 10, bottom: 287 });
});

test("a turned element is aligned on the page by the box it really covers", () => {
  // 40 x 20 tourné d'un quart de tour : il couvre 20 de large et 40 de haut.
  const turned: Element = { id: "a", frame: { x_mm: 50, y_mm: 50, w_mm: 40, h_mm: 20, rotation_deg: 90 }, locked: false, zone: false, kind: { type: "Shape", shape: "Rect", fill: "#ffffff", stroke: null, corner_radius_mm: 0 } };
  const page: Page = { id: "p", size: { width_mm: 210, height_mm: 297 }, elements: [turned], grid: null };
  const area = pageArea(page.size);
  const left = boundsOf([arrangeUnits(page, ["a"], (fs) => alignFrames(fs, "left", area)).elements[0].frame]);
  expect(left.x_mm).toBeCloseTo(10);
  const bottom = boundsOf([arrangeUnits(page, ["a"], (fs) => alignFrames(fs, "bottom", area)).elements[0].frame]);
  expect(bottom.y_mm + bottom.h_mm).toBeCloseTo(287);
});

test("a lone group is aligned on the page as one block", () => {
  const shape = (id: string, x: number): Element => ({ id, group: "g", frame: { x_mm: x, y_mm: 50, w_mm: 20, h_mm: 20, rotation_deg: 0 }, locked: false, zone: false, kind: { type: "Shape", shape: "Rect", fill: "#ffffff", stroke: null, corner_radius_mm: 0 } });
  const page: Page = { id: "p", size: { width_mm: 210, height_mm: 297 }, elements: [shape("a", 50), shape("b", 80)], grid: null };
  const out = arrangeUnits(page, ["a", "b"], (fs) => alignFrames(fs, "right", pageArea(page.size)));
  expect(out.elements.map((e) => e.frame.x_mm)).toEqual([150, 180]);
});
