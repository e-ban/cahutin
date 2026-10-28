import { expect, test } from "vitest";
import { nudge, rectPxToFrame, settle, settleMove, settleResize, settleScale, snapLines } from "./geometry";
import { PX_PER_MM } from "./units";

test("rectPxToFrame at zoom 1 rounds to 0.1 mm", () => {
  const f = rectPxToFrame({ left: 10 * PX_PER_MM + 0.2, top: 20 * PX_PER_MM, width: 50 * PX_PER_MM, height: 33.333 * PX_PER_MM, rotation: 12.5 }, 1);
  expect(f).toEqual({ x_mm: 10.1, y_mm: 20, w_mm: 50, h_mm: 33.3, rotation_deg: 12.5 });
});

test("rectPxToFrame at zoom 2", () => {
  const f = rectPxToFrame({ left: 2 * 10 * PX_PER_MM, top: 2 * 5 * PX_PER_MM, width: 2 * 40 * PX_PER_MM, height: 2 * 30 * PX_PER_MM, rotation: 0 }, 2);
  expect(f).toEqual({ x_mm: 10, y_mm: 5, w_mm: 40, h_mm: 30, rotation_deg: 0 });
});

test("nudge moves by 1 mm, 5 mm with shift", () => {
  const f = { x_mm: 10, y_mm: 10, w_mm: 20, h_mm: 20, rotation_deg: 0 };
  expect(nudge(f, "ArrowRight", false).x_mm).toBe(11);
  expect(nudge(f, "ArrowLeft", true).x_mm).toBe(5);
  expect(nudge(f, "ArrowUp", false).y_mm).toBe(9);
  expect(nudge(f, "ArrowDown", true).y_mm).toBe(15);
  expect(nudge(f, "ArrowDown", true).w_mm).toBe(20);
});

const A4 = { width_mm: 210, height_mm: 297 };

test("snap lines are the page edges, margins and middles, and the other blocks' edges and centres", () => {
  const lines = snapLines(A4, [{ x_mm: 30, y_mm: 200, w_mm: 100, h_mm: 30 }]);
  expect(lines.xs).toEqual([0, 10, 105, 200, 210, 30, 80, 130]);
  expect(lines.ys).toEqual([0, 10, 148.5, 287, 297, 200, 215, 230]);
});

test("settle gives the offset to the nearest line or grid step within the tolerance", () => {
  expect(settle([74.9, 104.9, 134.9], [105], 0.3)).toBeCloseTo(0.1);
  expect(settle([39.8], [], 0.3)).toBeCloseTo(0.2);
  expect(settle([41.2], [105], 0.3)).toBe(0);
});

test("a moved block lands exactly on the page middle", () => {
  const f = settleMove({ x_mm: 74.9, y_mm: 128.4, w_mm: 60, h_mm: 40, rotation_deg: 0 }, snapLines(A4, []), 0.3);
  expect(f.x_mm + f.w_mm / 2).toBeCloseTo(105, 6);
  expect(f.y_mm + f.h_mm / 2).toBeCloseTo(148.5, 6);
});

test("a rotated block is left as Moveable put it", () => {
  const f = { x_mm: 74.9, y_mm: 128.4, w_mm: 60, h_mm: 40, rotation_deg: 15 };
  expect(settleMove(f, snapLines(A4, []), 0.3)).toEqual(f);
});

test("resizing settles only the edges that moved", () => {
  const before = { x_mm: 10.2, y_mm: 20, w_mm: 50, h_mm: 30, rotation_deg: 0 };
  const f = settleResize(before, { ...before, w_mm: 94.6 }, snapLines(A4, []), 0.3);
  expect(f.x_mm).toBe(10.2);
  expect(f.x_mm + f.w_mm).toBeCloseTo(105, 6);
  expect(f.h_mm).toBe(30);
});

test("a group scaled from a corner settles the factor on the nearest moving edge", () => {
  const box = { x_mm: 20, y_mm: 20, w_mm: 40, h_mm: 20 };
  // Depuis le coin bas droit : le bord droit arrive à 104,9 au lieu de 105.
  expect(box.x_mm + box.w_mm * settleScale(box, 84.9 / 40, [1, 1], snapLines(A4, []), 0.3)).toBeCloseTo(105, 6);
  // Depuis le coin haut gauche : le bord gauche, l'origine restant le coin bas droit (60, 40).
  const f = settleScale(box, 50.1 / 40, [-1, -1], snapLines(A4, []), 0.3);
  expect(60 - box.w_mm * f).toBeCloseTo(10, 6);
  expect(settleScale(box, 1.31, [1, 1], snapLines(A4, []), 0.3)).toBe(1.31);
});
