import { expect, test } from "vitest";
import { dashArray } from "./strokes";

test("dash pattern matches the ODG dashes, in multiples of the stroke width", () => {
  expect(dashArray({ width_mm: 1, color: "#000000" }, 4)).toBeUndefined();
  expect(dashArray({ width_mm: 1, color: "#000000", style: "Solid" }, 4)).toBeUndefined();
  expect(dashArray({ width_mm: 1, color: "#000000", style: "Dashed" }, 4)).toBe("12 8");
  expect(dashArray({ width_mm: 1, color: "#000000", style: "Dotted" }, 4)).toBe("4 4");
  expect(dashArray(null, 4)).toBeUndefined();
});
