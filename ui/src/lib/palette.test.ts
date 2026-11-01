import { expect, test } from "vitest";
import { CVD_PALETTE, PALETTE, colorName } from "./palette";

test("every proposed colour has a name, found whatever the case of the code", () => {
  for (const s of [...PALETTE, ...CVD_PALETTE]) expect(colorName(s.color.toUpperCase())).toBe(s.name);
  expect(colorName("#123456")).toBeNull();
  expect(colorName(null)).toBeNull();
});

test("the two palettes have no two colours of the same name", () => {
  for (const p of [PALETTE, CVD_PALETTE]) expect(new Set(p.map((s) => s.name)).size).toBe(p.length);
});
