import { expect, test } from "vitest";
import { fitCount, repeatBlock } from "./repeat";
import type { Element, Page } from "./types";

const el = (id: string, x: number, y: number, zone = false): Element => ({
  id,
  frame: { x_mm: x, y_mm: y, w_mm: 80, h_mm: 30, rotation_deg: 0 },
  locked: false,
  zone,
  group: null,
  kind: { type: "Text", content: { paragraphs: [] }, style: { font: "Andika", size_pt: 14, color: "#222222", align: "Center" }, prompt: zone ? "Prénom" : null },
});
const page = (elements: Element[]): Page => ({ id: "p", size: { width_mm: 210, height_mm: 297 }, elements, grid: null, background: null });

test("a block repeated in a grid keeps its fields and gets a group per copy", () => {
  const p = page([el("cadre", 10, 10), el("nom", 15, 15, true)]);
  const { page: out, ids } = repeatBlock(p, ["cadre", "nom"], 2, 3);
  expect(out.elements).toHaveLength(12);
  expect(ids).toHaveLength(12);
  const groups = new Set(out.elements.map((e) => e.group));
  expect(groups.size).toBe(6);
  expect(groups.has(null)).toBe(false);
  // Bloc de 85 × 35 mm : la zone de la 2e colonne, 3e ligne est décalée de (85 + 5, 2 × (35 + 5)).
  const zones = out.elements.filter((e) => e.zone);
  expect(zones).toHaveLength(6);
  expect(zones.some((e) => e.frame.x_mm === 105 && e.frame.y_mm === 95)).toBe(true);
});

test("the page tells how many copies fit", () => {
  const p = page([el("cadre", 10, 10)]);
  // (210 - 10 - 10 + 5) / 85 = 2,29 ; (297 - 10 - 10 + 5) / 35 = 8,05.
  expect(fitCount(p, ["cadre"])).toEqual({ cols: 2, rows: 8 });
});
