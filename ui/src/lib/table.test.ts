import { expect, test } from "vitest";
import { borderSegments, isTableCell, tableOf } from "./table";
import type { Element, GridCell, Page, TableProps } from "./types";

const frame = { x_mm: 0, y_mm: 0, w_mm: 20, h_mm: 10, rotation_deg: 0 };
const cell = (element_id: string, col: number, row: number, col_span = 1, row_span = 1): GridCell => ({ element_id, col, row, col_span, row_span });
const sorted = (s: number[][]) => [...s].sort((a, b) => a.join(",").localeCompare(b.join(",")));

// Mêmes cas que carnet_core::table::tests::border_segments_skip_merged_cells.
test("borderSegments draws the outline and the separations", () => {
  const t: TableProps = { cols: [1, 1], rows: [1, 1], cells: [cell("a", 0, 0), cell("b", 1, 0), cell("c", 0, 1), cell("d", 1, 1)], border: null };
  expect(sorted(borderSegments(t, frame))).toEqual(
    sorted([
      [0, 0, 0, 10],
      [0, 0, 20, 0],
      [0, 5, 20, 5],
      [0, 10, 20, 10],
      [10, 0, 10, 10],
      [20, 0, 20, 10],
    ]),
  );
});

test("borderSegments skips a merged cell", () => {
  const t: TableProps = { cols: [1, 1], rows: [1, 1], cells: [cell("a", 0, 0, 2, 1), cell("c", 0, 1), cell("d", 1, 1)], border: null };
  const segs = borderSegments(t, frame);
  expect(segs).toContainEqual([10, 5, 10, 10]);
  expect(segs.some((s) => s[0] === 10 && s[2] === 10 && s[1] === 0)).toBe(false);
});

test("tableOf and isTableCell find the table of a cell", () => {
  const text = (id: string, group: string | null): Element => ({
    id,
    frame,
    locked: false,
    zone: false,
    group,
    kind: { type: "Text", content: { paragraphs: [] }, style: { font: "Andika", size_pt: 14, color: "#222222", align: "Left" }, prompt: null },
  });
  const table: Element = { id: "t", frame, locked: false, zone: false, group: "g", kind: { type: "Table", cols: [1], rows: [1], cells: [cell("a", 0, 0)], border: null } };
  const page: Page = { id: "p", size: { width_mm: 210, height_mm: 297 }, grid: null, elements: [text("a", "g"), table, text("x", null)] };
  expect(tableOf(page, "a")?.id).toBe("t");
  expect(tableOf(page, "t")?.id).toBe("t");
  expect(tableOf(page, "x")).toBeUndefined();
  expect(isTableCell(page, "a")).toBe(true);
  expect(isTableCell(page, "t")).toBe(false);
  expect(isTableCell(page, "x")).toBe(false);
});
