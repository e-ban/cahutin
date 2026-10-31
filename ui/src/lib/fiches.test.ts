import { expect, test } from "vitest";
import { newPage } from "./factory";
import { MAZE_CELLS, SYMMETRY_FIGURES, maze, mazePath, symmetry } from "./fiches";

const PATH_RE = /^M( -?\d+){2}(( [ML]( -?\d+){2})|( C( -?\d+){6})|( Z))*$/;

/** Murs du tracé (segments horizontaux et verticaux, en unités 0..1000). */
function walls(d: string): [number, number, number, number][] {
  const t = d.split(" ");
  const out: [number, number, number, number][] = [];
  for (let i = 0; i < t.length; i += 6) out.push([+t[i + 1], +t[i + 2], +t[i + 4], +t[i + 5]]);
  return out;
}

/** Cases atteintes depuis l'entrée (à gauche de la case 0, 0), en traversant les côtés sans mur. */
function reachable(n: number, d: string): { cells: number; exit: boolean } {
  const ws = walls(d);
  const u = 1000 / n;
  const blocked = (x1: number, y1: number, x2: number, y2: number) =>
    ws.some(([a, b, c, e]) => (x1 === x2 ? a === c && Math.abs(a - x1) < 1 && Math.min(b, e) <= y1 + 1 && Math.max(b, e) >= y2 - 1 : b === e && Math.abs(b - y1) < 1 && Math.min(a, c) <= x1 + 1 && Math.max(a, c) >= x2 - 1));
  const seen = new Set(["0,0"]);
  const todo = [[0, 0]];
  while (todo.length) {
    const [x, y] = todo.pop()!;
    const moves: [number, number, boolean][] = [
      [x + 1, y, !blocked((x + 1) * u, y * u, (x + 1) * u, (y + 1) * u)],
      [x - 1, y, !blocked(x * u, y * u, x * u, (y + 1) * u)],
      [x, y + 1, !blocked(x * u, (y + 1) * u, (x + 1) * u, (y + 1) * u)],
      [x, y - 1, !blocked(x * u, y * u, (x + 1) * u, y * u)],
    ];
    for (const [a, b, open] of moves) {
      if (open && a >= 0 && b >= 0 && a < n && b < n && !seen.has(`${a},${b}`)) {
        seen.add(`${a},${b}`);
        todo.push([a, b]);
      }
    }
  }
  const exitOpen = !blocked(1000, (n - 1) * u, 1000, n * u);
  return { cells: seen.size, exit: seen.has(`${n - 1},${n - 1}`) && exitOpen };
}

test("a maze reaches every cell from the entrance, and the way out", () => {
  for (const n of Object.values(MAZE_CELLS)) {
    for (const seed of [1, 7, 42]) {
      let s = seed;
      const d = mazePath(n, () => ((s = (s * 9301 + 49297) % 233280) / 233280));
      expect(d).toMatch(PATH_RE);
      expect(reachable(n, d)).toEqual({ cells: n * n, exit: true });
    }
  }
});

test("a maze sits under the page content, with a chick at the entrance and a hen at the exit", () => {
  const page = newPage();
  const els = maze(page, "MS", 3);
  expect(els.map((e) => (e.kind.type === "Image" ? e.kind.code : e.kind.type))).toEqual(["Shape", "1F424", "1F414"]);
  expect(new Set(els.map((e) => e.group)).size).toBe(1);
  for (const e of els) expect(e.frame.x_mm).toBeGreaterThanOrEqual(0);
  expect(els[0].frame.x_mm + els[0].frame.w_mm).toBeLessThanOrEqual(210);
});

test("a symmetry sheet: grid, left half of the drawing, red axis in the middle", () => {
  for (const f of Object.keys(SYMMETRY_FIGURES)) {
    const els = symmetry(newPage(), f, "MS");
    expect(els).toHaveLength(3);
    for (const e of els) if (e.kind.type === "Shape") expect(e.kind.path, f).toMatch(PATH_RE);
    const [grid, drawing, axis] = els;
    expect(axis.frame.x_mm + axis.frame.w_mm / 2).toBeCloseTo(grid.frame.x_mm + grid.frame.w_mm / 2);
    // Le dessin reste dans la moitié gauche (x ≤ 500 dans le carré du quadrillage).
    if (drawing.kind.type !== "Shape") throw new Error(f);
    const xs = drawing.kind.path!.split(" ").filter((_, i) => i % 3 === 1).map(Number);
    expect(Math.max(...xs), f).toBeLessThanOrEqual(500);
  }
  expect(symmetry(newPage(), "inconnu", "MS")).toEqual([]);
});
