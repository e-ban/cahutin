import { expect, test } from "vitest";
import { nextZoom } from "./units";

test("nextZoom steps through the zoom levels from any value", () => {
  expect(nextZoom(1, 1)).toBe(1.25);
  expect(nextZoom(1, -1)).toBe(0.75);
  expect(nextZoom(0.83, 1)).toBe(1);
  expect(nextZoom(0.83, -1)).toBe(0.75);
  expect(nextZoom(3, 1)).toBe(3);
  expect(nextZoom(0.25, -1)).toBe(0.25);
});
