import { expect, test } from "vitest";
import { newPage } from "./factory";
import { MOTIFS, MOTIF_HEIGHT_MM, PATHS, graphismeRow, graphismeRows, pathInFrame } from "./graphisme";

const PATH_RE = /^M( -?\d+){2}(( [ML]( -?\d+){2})|( C( -?\d+){6})|( Z))*$/;

test("every motif makes a grouped row: start point, model, dotted rest, baseline, inside the width", () => {
  for (const m of MOTIFS) {
    const row = graphismeRow(m, "MS", 15, 40, 180);
    expect(row).toHaveLength(4);
    expect(new Set(row.map((e) => e.group)).size).toBe(1);
    const [base, point, model, rest] = row;
    expect(base.kind).toMatchObject({ type: "Shape", shape: "Line" });
    expect(point.kind).toMatchObject({ type: "Shape", shape: "Ellipse", fill: "#2e9d4a" });
    for (const p of [model, rest]) {
      if (p.kind.type !== "Shape") throw new Error(m);
      expect(p.kind.shape).toBe("Path");
      expect(p.kind.path, m).toMatch(PATH_RE);
      const nums = p.kind.path!.split(" ").filter((t) => !/[MLCZ]/.test(t)).map(Number);
      expect(Math.min(...nums), m).toBeGreaterThanOrEqual(0);
      expect(Math.max(...nums), m).toBeLessThanOrEqual(1000);
    }
    expect(model.kind).toMatchObject({ stroke: { style: "Solid" } });
    expect(rest.kind).toMatchObject({ stroke: { style: "Dotted" } });
    // Les motifs restent dans la ligne (les boucles débordent un peu à gauche de leur pas).
    expect(rest.frame.x_mm + rest.frame.w_mm, m).toBeLessThanOrEqual(15 + 180 + MOTIF_HEIGHT_MM.MS);
    expect(model.frame.x_mm, m).toBeGreaterThan(15);
  }
});

test("rows go under what the page already holds and stop at the bottom margin", () => {
  const page = newPage();
  const rows = graphismeRows(page, "ponts", "PS", 20);
  const groups = new Set(rows.map((e) => e.group));
  expect(groups.size).toBeGreaterThan(3);
  expect(groups.size).toBeLessThan(20);
  expect(Math.max(...rows.map((e) => e.frame.y_mm + e.frame.h_mm))).toBeLessThanOrEqual(297 - 15 + 2);
  const first = rows.slice(0, 4);
  page.elements.push(...first);
  const more = graphismeRows(page, "vagues", "GS", 1);
  expect(Math.min(...more.map((e) => e.frame.y_mm))).toBeGreaterThan(Math.max(...first.map((e) => e.frame.y_mm + e.frame.h_mm)));
});

test("a path in mm is mapped onto its frame's 0..1000 square", () => {
  const { frame, path } = pathInFrame([["M", 10, 5], ["L", 30, 15]], 100);
  expect(frame).toEqual({ x_mm: 10, y_mm: 105, w_mm: 20, h_mm: 10, rotation_deg: 0 });
  expect(path).toBe("M 0 0 L 1000 1000");
  for (const p of PATHS) expect(p.path).toMatch(PATH_RE);
});
