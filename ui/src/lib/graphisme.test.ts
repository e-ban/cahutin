import { expect, test } from "vitest";
import { newPage } from "./factory";
import { MOTIFS, MOTIF_HEIGHT_MM, PATHS, buildRow, cellsFor, cmdsFor, dotSize, graphismeRow, graphismeRows, motifCount, pathInFrame, presetOf, progressive, type Motif, type RowPreset } from "./graphisme";
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
    expect(arrow.kind).toMatchObject({ type: "Shape", shape: "Path", fill: null, stroke: { width_mm: 0.7, color: "#2e9d4a", style: "Solid" } });
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
/** Les tracés des motifs (pas la flèche, verte). */
const paths = (row: Element[]) => row.filter((e) => shapeOf(e)?.shape === "Path" && shapeOf(e)?.stroke?.color !== "#2e9d4a");

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
  expect(row.map((e) => shapeOf(e)?.shape)).toEqual(["Line", "Ellipse", "Path", "Path", "Path"]);
  expect(new Set(row.map((e) => e.group)).size).toBe(1);
  const base = shapeOf(row[0])!;
  expect(base.graphisme).toMatchObject({ motif: "ponts", level: "MS", guides: "Base", arrow: true });
  expect(base.graphisme!.cells).toHaveLength(motifCount("ponts", "MS", 180));
  expect(row[0].frame).toEqual({ x_mm: 15, y_mm: 40 + 16 - 1, w_mm: 180, h_mm: 2, rotation_deg: 0 });
  expect(shapeOf(row[3])!.stroke).toMatchObject({ style: "Solid", color: "#333333" });
  expect(shapeOf(row[4])!.stroke).toMatchObject({ style: "Dotted", color: "#9e9e9e" });
  // La flèche : un trait fin vert, plus épais pour les petits.
  expect(shapeOf(row[2])!.stroke).toEqual({ width_mm: 0.7, color: "#2e9d4a", style: "Solid" });
  expect(shapeOf(row[2])!.path).toMatch(PATH_RE);
  expect(shapeOf(graphismeRow("ponts", "PS", 15, 40, 180)[2])!.stroke!.width_mm).toBe(0.8);
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
  expect(noArrow.some((e) => shapeOf(e)?.stroke?.color === "#2e9d4a")).toBe(false);
  expect(noArrow).toHaveLength(4);
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

type P = [number, number];
/** Les points d'un tracé en mm, chaque courbe prise en 16 points, chaque segment tous les 0,5 mm, descendus de `top`. */
function samples(cmds: ReturnType<typeof cmdsFor>, top: number): P[] {
  const out: P[] = [];
  let cur: P = [0, 0];
  for (const c of cmds) {
    if (c[0] === "M") cur = [c[1], c[2]];
    else if (c[0] === "L") {
      const n = Math.max(1, Math.ceil(Math.hypot(c[1] - cur[0], c[2] - cur[1]) / 0.5));
      for (let i = 1; i < n; i++) out.push([cur[0] + ((c[1] - cur[0]) * i) / n, cur[1] + ((c[2] - cur[1]) * i) / n]);
      cur = [c[1], c[2]];
    } else if (c[0] === "C") {
      const [, a, b, cc, d, e, f] = c;
      for (let i = 1; i <= 16; i++) {
        const t = i / 16, u = 1 - t;
        out.push([u * u * u * cur[0] + 3 * u * u * t * a + 3 * u * t * t * cc + t * t * t * e, u * u * u * cur[1] + 3 * u * u * t * b + 3 * u * t * t * d + t * t * t * f]);
      }
      cur = [e, f];
      continue;
    }
    out.push(cur);
  }
  return out.map(([x, y]) => [x, y + top]);
}
/** Les morceaux (après chaque M) d'un tracé posé dans `frame`, en mm. */
function pieces(e: Element): P[][] {
  const s = shapeOf(e)!;
  const f = e.frame;
  const t = s.path!.split(" ");
  const out: P[][] = [];
  for (let i = 0; i < t.length; i += 3) {
    const p: P = [f.x_mm + (Number(t[i + 1]) / 1000) * f.w_mm, f.y_mm + (Number(t[i + 2]) / 1000) * f.h_mm];
    if (t[i] === "M") out.push([p]);
    else out[out.length - 1].push(p);
  }
  return out;
}
const LEVELS = ["PS", "MS", "GS"] as const;
/** Le point, la flèche et le début du tracé d'une ligne posée en (15, 40). */
function startOf(m: Motif, level: (typeof LEVELS)[number]) {
  const row = graphismeRow(m, level, 15, 40, 180);
  const h = MOTIF_HEIGHT_MM[level];
  const start = 15 + dotSize(h) + 2;
  return { dot: row[1].frame, arrow: row[2], first: samples(cmdsFor(m, start, h, 0, 1), 40), two: samples(cmdsFor(m, start, h, 0, 2), 40) };
}
const len = (c: P[]) => c.slice(1).reduce((s, q, i) => s + Math.hypot(q[0] - c[i][0], q[1] - c[i][1]), 0);
/** Le sens de la fin de la flèche : du point deux avant la pointe à la pointe. */
function endDir(arrow: Element): P {
  const [shaft] = pieces(arrow);
  const [e, d] = [shaft[shaft.length - 1], shaft[Math.max(0, shaft.length - 3)]];
  return [e[0] - d[0], e[1] - d[1]];
}

test("the start dot sits on the first point of the model", () => {
  for (const level of LEVELS)
    for (const m of MOTIFS) {
      const { dot, first } = startOf(m, level);
      expect(dot.w_mm).toBe(dotSize(MOTIF_HEIGHT_MM[level]));
      expect(Math.abs(dot.x_mm + dot.w_mm / 2 - first[0][0]), `${m} ${level}`).toBeLessThan(0.01);
      expect(Math.abs(dot.y_mm + dot.h_mm / 2 - first[0][1]), `${m} ${level}`).toBeLessThan(0.01);
    }
});

test("the arrow is a thin green line beside the first two motifs, long enough, with an open head at its end", () => {
  for (const level of LEVELS)
    for (const m of MOTIFS) {
      const { arrow, two } = startOf(m, level);
      const at = `${m} ${level}`;
      expect(arrow.kind, at).toMatchObject({ type: "Shape", shape: "Path", fill: null, stroke: { color: "#2e9d4a", style: "Solid" } });
      expect(shapeOf(arrow)!.path, at).toMatch(PATH_RE);
      const [shaft, head] = pieces(arrow);
      expect(head, at).toHaveLength(3);
      const h = MOTIF_HEIGHT_MM[level];
      const arm = Math.min(2.4, Math.max(1.6, 0.16 * h));
      // Le trait est assez long pour se voir derrière la pointe, même en grande section.
      expect(len(shaft), at).toBeGreaterThanOrEqual(2.5 * arm - 0.05);
      // La pointe est au bout du trait, ses deux branches de la longueur prévue.
      const tip = shaft[shaft.length - 1];
      expect(Math.hypot(head[1][0] - tip[0], head[1][1] - tip[1]), at).toBeLessThan(0.05);
      for (const b of [head[0], head[2]]) expect(Math.hypot(b[0] - tip[0], b[1] - tip[1]), at).toBeCloseTo(arm, 1);
      // Loin des deux premiers motifs.
      let near = Infinity;
      for (const [x, y] of shaft) for (const [u, v] of two) near = Math.min(near, Math.hypot(x - u, y - v));
      expect(near, at).toBeGreaterThanOrEqual(1.7);
    }
});

test("the arrow follows the first gesture", () => {
  for (const level of LEVELS) {
    const dir = (m: Motif) => endDir(startOf(m, level).arrow);
    for (const m of ["ponts", "creneaux"] as Motif[]) {
      const [dx, dy] = dir(m);
      expect(dy < 0 && Math.abs(dy) > Math.abs(dx), `${m} ${level} ${dx} ${dy}`).toBe(true);
    }
    for (const m of ["coupes", "traits"] as Motif[]) {
      const [dx, dy] = dir(m);
      expect(dy > 0 && Math.abs(dy) > Math.abs(dx), `${m} ${level} ${dx} ${dy}`).toBe(true);
    }
    expect(dir("ronds")[0], `ronds ${level}`).toBeLessThan(0);
    expect(dir("boucles")[0], `boucles ${level}`).toBeGreaterThan(0);
  }
});

test("buildRow keeps its colours in a copy of the recipe", () => {
  const r = { motif: "ponts", level: "MS", cells: cellsFor("seul", 4), guides: "Base" as const, arrow: true };
  const [base] = buildRow(r, 15, 40, 180, { model: "#ff0000", trace: "#0000ff" });
  expect(base.kind).toMatchObject({ graphisme: { ...r, model_color: "#ff0000", trace_color: "#0000ff" } });
  expect(r).not.toHaveProperty("model_color");
});
