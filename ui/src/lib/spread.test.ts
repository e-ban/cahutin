import { expect, test } from "vitest";
import { facingOf, mirrorLayout } from "./spread";

test("pages face each other like an open notebook", () => {
  expect(facingOf(0, 5)).toBeNull();
  expect(facingOf(1, 5)).toEqual({ index: 2, side: "right" });
  expect(facingOf(2, 5)).toEqual({ index: 1, side: "left" });
  expect(facingOf(3, 4)).toBeNull();
});

test("a verso page puts each card behind its recto", () => {
  const page = { size: { width_mm: 210 }, elements: [{ frame: { x_mm: 10, w_mm: 90, rotation_deg: 5 } }] };
  expect(mirrorLayout(page).elements[0].frame).toEqual({ x_mm: 110, w_mm: 90, rotation_deg: -5 });
});
