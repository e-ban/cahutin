import { expect, test } from "vitest";
import { centeredFrame, newPage, shapeElement, textElement } from "./factory";

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

test("a new element does not land exactly on the previous one", () => {
  const page = newPage();
  const first = centeredFrame(page, 40, 20);
  expect(first).toMatchObject({ x_mm: 85, y_mm: 138.5 });
  page.elements.push(textElement(first));
  const second = centeredFrame(page, 30, 30);
  expect([second.x_mm + 15, second.y_mm + 15]).toEqual([113, 156.5]);
});
