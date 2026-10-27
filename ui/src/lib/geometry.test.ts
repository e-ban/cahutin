import { expect, test } from "vitest";
import { nudge, rectPxToFrame } from "./geometry";
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
