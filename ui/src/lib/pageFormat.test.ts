import { expect, test } from "vitest";
import { a3Tiles, changeFormat, formatOf, printPageName, TILE_OVERLAP_MM } from "./pageFormat";
import type { Page } from "./types";

test("formats are recognised in both orientations", () => {
  expect(formatOf({ width_mm: 210, height_mm: 297 })).toBe("A4");
  expect(formatOf({ width_mm: 420, height_mm: 297 })).toBe("A3");
  expect(formatOf({ width_mm: 100, height_mm: 100 })).toBeNull();
  expect(printPageName({ width_mm: 210, height_mm: 148 })).toBe("a5l");
});

test("a page grows to A3 with its content", () => {
  const page: Page = {
    id: "p",
    size: { width_mm: 210, height_mm: 297 },
    grid: null,
    background: null,
    elements: [{ id: "t", frame: { x_mm: 10, y_mm: 20, w_mm: 100, h_mm: 30, rotation_deg: 0 }, locked: false, zone: false, group: null, kind: { type: "Text", content: { paragraphs: [] }, style: { font: "Andika", size_pt: 20, color: "#000000", align: "Left" }, prompt: null } }],
  };
  const a3 = changeFormat(page, "A3");
  expect(a3.size).toEqual({ width_mm: 297, height_mm: 420 });
  expect(a3.elements[0].frame.w_mm).toBeCloseTo(141.4, 0);
  expect(a3.elements[0].kind.type === "Text" && a3.elements[0].kind.style.size_pt).toBe(28.5);
});

test("an A3 poster prints on two A4 sheets that overlap", () => {
  const [top, bottom] = a3Tiles({ width_mm: 297, height_mm: 420 });
  expect(top.sheet).toEqual({ width_mm: 297, height_mm: 210 });
  // L'affiche réduite fait deux demi-feuilles moins le recouvrement.
  expect(420 * top.scale).toBeCloseTo(2 * 210 - TILE_OVERLAP_MM);
  expect(top.dy_mm).toBe(0);
  expect(bottom.dy_mm).toBeCloseTo(-(420 * top.scale - 210));
  // Centrée en largeur.
  expect(top.dx_mm).toBeCloseTo((297 - 297 * top.scale) / 2);
});
