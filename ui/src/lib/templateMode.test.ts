import { expect, test } from "vitest";
import { cellsCoveredBy, mergePair, spannedCell } from "./templateMode";
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

const grid3: Page = {
  ...page,
  grid: {
    cols: 3,
    rows: 3,
    margin_mm: 10,
    gap_mm: 4,
    cells: [
      { element_id: "big", col: 0, row: 0, col_span: 2, row_span: 1 },
      { element_id: "r0c2", col: 2, row: 0, col_span: 1, row_span: 1 },
      ...[1, 2].flatMap((row) => [0, 1, 2].map((col) => ({ element_id: `r${row}c${col}`, col, row, col_span: 1, row_span: 1 }))),
    ],
  },
};

test("cellsCoveredBy two neighbours", () => {
  expect(cellsCoveredBy(grid3, "r1c0", "r1c1")?.sort()).toEqual(["r1c0", "r1c1"]);
});

test("cellsCoveredBy a 2x2 diagonal drag", () => {
  expect(cellsCoveredBy(grid3, "r2c2", "r1c1")?.sort()).toEqual(["r1c1", "r1c2", "r2c1", "r2c2"]);
});

test("cellsCoveredBy refuses a rectangle cutting a merged cell", () => {
  expect(cellsCoveredBy(grid3, "r0c2", "r1c1")).toBeNull();
  expect(cellsCoveredBy(grid3, "big", "r1c2")?.length).toBe(5);
});
