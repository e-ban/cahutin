import { expect, test } from "vitest";
import { newPage } from "./factory";
import { MOTIFS, MOTIF_HEIGHT_MM, PATHS, buildRow, cellsFor, graphismeRow, graphismeRows, motifCount, pathInFrame, presetOf, progressive, type RowPreset } from "./graphisme";
import type { Cell, Element } from "./types";

const PATH_RE = /^M( -?\d+){2}(( [ML]( -?\d+){2})|( C( -?\d+){6})|( Z))*$/;

test("every motif makes a grouped row: start point, model, dotted rest, baseline, inside the width", () => {
  for (const m of MOTIFS) {
    const row = graphismeRow(m, "MS", 15, 40, 180);
    expect(row).toHaveLength(5);
    expect(new Set(row.map((e) => e.group)).size).toBe(1);
    const [base, point, arrow, model, rest] = row;
    expect(base.kind).toMatchObject({ type: "Shape", shape: "Line" });
    expect(point.kind).toMatchObject({ type: "Shape", shape: "Ellipse", fill: "#2e9d4a" });
    expect(arrow.kind).toMatchObject({ type: "Shape", shape: "Arrow", stroke: { width_mm: 0.8, color: "#2e9d4a" } });
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
  const first = rows.slice(0, 5);
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

const M: Cell = "Model";
const D: Cell = "Dotted";
const E: Cell = "Empty";
const times = (c: Cell, n: number): Cell[] => Array(n).fill(c);
const shapeOf = (e: Element) => (e.kind.type === "Shape" ? e.kind : null);
const paths = (row: Element[]) => row.filter((e) => shapeOf(e)?.shape === "Path");

test("cellsFor gives the presets", () => {
  expect(cellsFor("tout", 10)).toEqual([M, ...times(D, 9)]);
  expect(cellsFor("moitie", 10)).toEqual([M, ...times(D, 4), ...times(E, 5)]);
  expect(cellsFor("quelques", 10)).toEqual([M, ...times(D, 3), ...times(E, 6)]);
  expect(cellsFor("seul", 10)).toEqual([M, ...times(E, 9)]);
  expect(cellsFor("quelques", 5)).toEqual([M, D, D, E, E]);
  expect(cellsFor("quelques", 2)).toEqual([M, D]);
});

test("progressive spreads over the rows", () => {
  expect([0, 1, 2, 3].map((r) => progressive(r, 4))).toEqual(["tout", "moitie", "quelques", "seul"]);
  expect([0, 1].map((r) => progressive(r, 2))).toEqual(["tout", "seul"]);
  expect(progressive(0, 1)).toBe("tout");
});

test("presetOf recognises presets and hand edits", () => {
  for (const p of ["tout", "moitie", "quelques", "seul"] as RowPreset[]) expect(presetOf(cellsFor(p, 10))).toBe(p);
  expect(presetOf([D, M, ...times(D, 8)])).toBeNull();
});

test("a default row keeps today's layout, plus the arrow", () => {
  const row = graphismeRow("ponts", "MS", 15, 40, 180);
  expect(row.map((e) => shapeOf(e)?.shape)).toEqual(["Line", "Ellipse", "Arrow", "Path", "Path"]);
  expect(new Set(row.map((e) => e.group)).size).toBe(1);
  const base = shapeOf(row[0])!;
  expect(base.graphisme).toMatchObject({ motif: "ponts", level: "MS", guides: "Base", arrow: true });
  expect(base.graphisme!.cells).toHaveLength(motifCount("ponts", "MS", 180));
  expect(row[0].frame).toEqual({ x_mm: 15, y_mm: 40 + 16 - 1, w_mm: 180, h_mm: 2, rotation_deg: 0 });
  expect(shapeOf(row[3])!.stroke).toMatchObject({ style: "Solid", color: "#333333" });
  expect(shapeOf(row[4])!.stroke).toMatchObject({ style: "Dotted", color: "#9e9e9e" });
  // La flèche : 6 x 3 mm, centrée sur le point, 4 mm au-dessus.
  const [point, arrow] = [row[1].frame, row[2].frame];
  expect(arrow.w_mm).toBe(6);
  expect(arrow.h_mm).toBe(3);
  expect(arrow.x_mm + 3).toBeCloseTo(point.x_mm + point.w_mm / 2);
  expect(arrow.y_mm + 1.5).toBeCloseTo(point.y_mm + point.h_mm / 2 - 4);
  expect(arrow.rotation_deg).toBe(-70);
  // Seule la ligne de base porte la recette.
  for (const e of row.slice(1)) expect(shapeOf(e)?.graphisme).toBeUndefined();
});

test("empty motifs draw nothing and the base still spans the width", () => {
  const row = graphismeRow("vagues", "GS", 15, 40, 180, { preset: "seul" });
  expect(paths(row).filter((e) => shapeOf(e)?.stroke?.style === "Dotted")).toHaveLength(0);
  expect(paths(row)).toHaveLength(1);
  expect(row[0].frame.w_mm).toBe(180);
  const bare = graphismeRow("vagues", "GS", 15, 40, 180, { guides: "None" });
  expect(shapeOf(bare[0])!.stroke).toBeNull();
  expect(shapeOf(bare[0])!.graphisme).toBeDefined();
});

test("the corridor adds a top line", () => {
  const row = graphismeRow("dents", "MS", 15, 40, 180, { guides: "Corridor" });
  const lines = row.filter((e) => shapeOf(e)?.shape === "Line");
  expect(lines).toHaveLength(2);
  expect(lines.map((e) => e.frame.y_mm)).toContain(39);
  expect(lines.filter((e) => shapeOf(e)?.graphisme)).toHaveLength(1);
  const noArrow = graphismeRow("dents", "MS", 15, 40, 180, { arrow: false });
  expect(noArrow.some((e) => shapeOf(e)?.shape === "Arrow")).toBe(false);
});

test("hand-made cells make one path per run", () => {
  const r = { motif: "creneaux", level: "MS", cells: [M, D, E, D], guides: "Base" as const, arrow: true };
  const ps = paths(buildRow(r, 15, 40, 180, { model: "#ff0000", trace: "#00ff00" }, "g1"));
  expect(ps.map((e) => shapeOf(e)!.stroke!.style)).toEqual(["Solid", "Dotted", "Dotted"]);
  expect(ps.map((e) => shapeOf(e)!.stroke!.color)).toEqual(["#ff0000", "#00ff00", "#00ff00"]);
  expect(ps.every((e) => e.group === "g1")).toBe(true);
  for (const p of ps) expect(shapeOf(p)!.path).toMatch(PATH_RE);
  // Les suites vont de gauche à droite.
  expect(ps[1].frame.x_mm).toBeLessThan(ps[2].frame.x_mm);
});

test("circles turn the writing way", () => {
  const model = paths(graphismeRow("ronds", "MS", 15, 40, 180))[0];
  const nums = shapeOf(model)!.path!.split(" ");
  const startX = Number(nums[1]);
  const c = nums.indexOf("C");
  expect(Number(nums[c + 5])).toBeLessThan(startX);
});
