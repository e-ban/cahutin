import { expect, test } from "vitest";
import { mergePair, spannedCell } from "./templateMode";
import type { Page } from "./types";

const page: Page = {
  id: "p",
  size: { width_mm: 210, height_mm: 297 },
  elements: [],
  grid: {
    cols: 2,
    rows: 2,
    margin_mm: 10,
    gap_mm: 4,
    cells: [
      { element_id: "a", col: 0, row: 0, col_span: 2, row_span: 1 },
      { element_id: "b", col: 0, row: 1, col_span: 1, row_span: 1 },
      { element_id: "c", col: 1, row: 1, col_span: 1, row_span: 1 },
    ],
  },
};

test("mergePair needs exactly two grid cells", () => {
  expect(mergePair(["b", "c"], page)).toEqual(["b", "c"]);
  expect(mergePair(["b", "hors-grille"], page)).toBeNull();
  expect(mergePair(["a", "b", "c"], page)).toBeNull();
  expect(mergePair(["b"], page)).toBeNull();
  expect(mergePair(["b", "c"], { ...page, grid: null })).toBeNull();
});

test("spannedCell finds a merged cell", () => {
  expect(spannedCell(["a"], page)).toBe("a");
  expect(spannedCell(["b"], page)).toBeNull();
  expect(spannedCell(["a", "b"], page)).toBeNull();
});
