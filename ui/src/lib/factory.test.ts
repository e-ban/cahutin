import { expect, test } from "vitest";
import { newPage, shapeElement } from "./factory";

test("shapeElement gives each shape sensible defaults, centred on the page", () => {
  const page = newPage();
  const arrow = shapeElement(page, "Arrow");
  expect(arrow.kind.type === "Shape" && arrow.kind.stroke).toEqual({ width_mm: 1, color: "#333333" });
  expect(arrow.kind.type === "Shape" && arrow.kind.fill).toBeNull();
  expect(arrow.frame.x_mm + arrow.frame.w_mm / 2).toBeCloseTo(105);
  const bubble = shapeElement(page, "Bubble");
  expect(bubble.kind.type === "Shape" && bubble.kind.fill).toBe("#ffffff");
  expect(bubble.kind.type === "Shape" && bubble.kind.stroke?.width_mm).toBe(0.5);
  const rounded = shapeElement(page, "Rect", 6);
  expect(rounded.kind.type === "Shape" && rounded.kind.corner_radius_mm).toBe(6);
  const star = shapeElement(page, "Star");
  expect(star.frame.w_mm).toBe(star.frame.h_mm);
});
