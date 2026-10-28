import { expect, test } from "vitest";
import { placeContextBar } from "./contextBar";

const viewport = { left: 0, top: 0, width: 1000, height: 700 };
const bar = { width: 300, height: 40 };

test("placeContextBar above when there is room", () => {
  const p = placeContextBar({ left: 400, top: 300, width: 200, height: 100 }, bar, viewport);
  expect(p).toEqual({ left: 350, top: 252 });
});

test("placeContextBar below when the target touches the top", () => {
  const p = placeContextBar({ left: 400, top: 20, width: 200, height: 100 }, bar, viewport);
  expect(p.top).toBe(128);
});

test("placeContextBar stays inside the viewport for a huge target", () => {
  const p = placeContextBar({ left: 100, top: -50, width: 800, height: 900 }, bar, viewport);
  expect(p.top).toBe(8);
  expect(p.left).toBe(350);
});

test("placeContextBar clamps horizontally", () => {
  expect(placeContextBar({ left: 0, top: 300, width: 40, height: 40 }, bar, viewport).left).toBe(8);
  expect(placeContextBar({ left: 980, top: 300, width: 40, height: 40 }, bar, viewport).left).toBe(692);
});

test("a larger gap above the element does not widen the viewport margin", () => {
  // Écart de 44 px (poignée de rotation) : la marge avec le bord de la zone visible reste de 8 px.
  expect(placeContextBar({ left: 100, top: -50, width: 800, height: 900 }, bar, viewport, 44).top).toBe(8);
  expect(placeContextBar({ left: 400, top: 100, width: 200, height: 100 }, bar, viewport, 44).top).toBe(16);
  expect(placeContextBar({ left: 0, top: 300, width: 40, height: 40 }, bar, viewport, 44).left).toBe(8);
});
