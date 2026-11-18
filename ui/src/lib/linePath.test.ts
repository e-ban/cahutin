import { expect, test } from "vitest";
import { closedCurveBeziers, curveBeziers, dragPoint, endNear, fromShapePath, toShapePath, endDirections, extend, fromPagePoints, insertPoint, isPath, pagePoints, removePoint, segmentMids, type Pt } from "./linePath";
import { lineEndpoints } from "./lineEnds";
import type { Frame, ShapeProps } from "./types";

const line = (extra: Partial<ShapeProps> = {}): ShapeProps => ({
  shape: "Line",
  fill: null,
  stroke: { width_mm: 1, color: "#333333", style: null },
  corner_radius_mm: 0,
  path: null,
  ...extra,
});
const close = (a: Pt[], b: Pt[], digits = 9) => {
  expect(a.length).toBe(b.length);
  a.forEach((p, i) => {
    expect(p.x).toBeCloseTo(b[i].x, digits);
    expect(p.y).toBeCloseTo(b[i].y, digits);
  });
};
const noLines = { xs: [], ys: [] };

test("bake keeps every point in place", () => {
  const frame: Frame = { x_mm: 30, y_mm: 40, w_mm: 60, h_mm: 30, rotation_deg: 30 };
  const s = line({ points: [[0, 0.5], [0.3, 0], [0.7, 1], [1, 0.5]] });
  const before = pagePoints(frame, s, true, false);
  const r = fromPagePoints(before, s);
  expect(r.frame.rotation_deg).toBe(0);
  expect([r.flipH, r.flipV]).toEqual([false, false]);
  close(pagePoints(r.frame, { ...s, ...r.patch }, false, false), before);
});

test("flip_v mirrors a path vertically", () => {
  const frame: Frame = { x_mm: 0, y_mm: 0, w_mm: 10, h_mm: 10, rotation_deg: 0 };
  const s = line({ points: [[0, 0], [0.5, 0.2], [1, 0]] });
  close(pagePoints(frame, s, false, true), [{ x: 0, y: 10 }, { x: 5, y: 8 }, { x: 10, y: 10 }]);
});

test("straight line round-trips", () => {
  const frame: Frame = { x_mm: 40, y_mm: 60, w_mm: 80, h_mm: 4, rotation_deg: 25 };
  const s = line();
  const pts = pagePoints(frame, s, false, false);
  close(pts, lineEndpoints(frame, false));
  const r = fromPagePoints(pts, s);
  expect(r.patch.points).toBeUndefined();
  expect(r.patch.curve).toBeUndefined();
  for (const k of ["x_mm", "y_mm", "w_mm", "h_mm", "rotation_deg"] as const) expect(r.frame[k]).toBeCloseTo(frame[k], 9);
});

test("insert then remove is identity", () => {
  const pts = [{ x: 0, y: 0 }, { x: 10, y: 0 }, { x: 10, y: 10 }];
  const more = insertPoint(pts, 0, { x: 5, y: 3 });
  expect(more).toEqual([{ x: 0, y: 0 }, { x: 5, y: 3 }, { x: 10, y: 0 }, { x: 10, y: 10 }]);
  expect(removePoint(more, 1)).toEqual(pts);
  expect(removePoint([{ x: 0, y: 0 }, { x: 1, y: 1 }], 0)).toHaveLength(2);
});

test("removing an end of a 3-point path gives a straight line", () => {
  const ends = { start: "Circle", end: "Arrow", size: "Large" } as const;
  const s = line({ shape: "Arrow", ends });
  const pts = removePoint([{ x: 0, y: 0 }, { x: 10, y: 0 }, { x: 10, y: 10 }], 0);
  expect(pts).toEqual([{ x: 10, y: 0 }, { x: 10, y: 10 }]);
  const r = fromPagePoints(pts, s);
  expect(r.patch.points).toBeUndefined();
  expect("ends" in r.patch).toBe(false);
  close(lineEndpoints(r.frame, false), pts);
});

test("extend at start and end", () => {
  const pts = [{ x: 0, y: 0 }, { x: 10, y: 0 }];
  expect(extend(pts, "end", { x: 10, y: 10 })).toEqual([...pts, { x: 10, y: 10 }]);
  expect(extend(pts, "start", { x: -5, y: 0 })).toEqual([{ x: -5, y: 0 }, ...pts]);
});

test("extend refuses the 201st point", () => {
  const full = Array.from({ length: 200 }, (_, i) => ({ x: i, y: 0 }));
  expect(extend(full, "end", { x: 300, y: 0 })).toBe(full);
});

test("frame keeps the hit margin", () => {
  const s = line();
  const r = fromPagePoints([{ x: 10, y: 50 }, { x: 30, y: 50 }, { x: 50, y: 50 }], s);
  expect(r.frame.h_mm).toBeCloseTo(4, 9);
  expect(r.frame.w_mm).toBeCloseTo(44, 9);
  expect(r.frame.x_mm).toBeCloseTo(8, 9);
  expect(r.patch.points).toEqual([[2 / 44, 0.5], [22 / 44, 0.5], [42 / 44, 0.5]]);
  expect(isPath({ ...s, ...r.patch })).toBe(true);
  expect(isPath(s)).toBe(false);
});

test("curve passes through points; parity with Rust", () => {
  const c = curveBeziers([{ x: 0, y: 0 }, { x: 10, y: 10 }, { x: 20, y: 0 }]);
  const r = (v: number) => Math.round(v * 1e6) / 1e6;
  // Copié de shapes.rs (CURVE_PARITY).
  expect(c.flatMap((b) => b.flatMap((p) => [r(p.x), r(p.y)]))).toEqual([3.333333, 3.333333, 6.666667, 10.0, 10.0, 10.0, 13.333333, 10.0, 16.666667, 3.333333, 20.0, 0.0]);
});

test("repeated points stay finite", () => {
  const pts = [{ x: 0, y: 0 }, { x: 0, y: 0 }, { x: 5, y: 5 }];
  const all = [...curveBeziers(pts).flat(), ...segmentMids(pts, true), ...segmentMids(pts, false)].flatMap((p) => [p.x, p.y]);
  const d = endDirections(pts, true);
  for (const v of [...all, d.start, d.end, endDirections(pts, false).start]) expect(Number.isFinite(v)).toBe(true);
});

test("endDirections point outward along the last segment", () => {
  const pts = [{ x: 0, y: 0 }, { x: 10, y: 0 }, { x: 10, y: 10 }];
  const d = endDirections(pts, false);
  expect(d.start).toBeCloseTo(180, 9);
  expect(d.end).toBeCloseTo(90, 9);
});

test("segment mids, straight and curved", () => {
  const pts = [{ x: 0, y: 0 }, { x: 10, y: 0 }, { x: 10, y: 10 }];
  expect(segmentMids(pts, false)).toEqual([{ x: 5, y: 0 }, { x: 10, y: 5 }]);
  const curved = segmentMids(pts, true);
  expect(curved).toHaveLength(2);
  expect(curved[0].y).not.toBeCloseTo(0, 3);
});

test("dragPoint snaps to another point of the same path, and Shift gives 45°", () => {
  const pts = [{ x: 0, y: 0 }, { x: 20, y: 10 }, { x: 40, y: 0 }];
  const snapped = dragPoint({ pts, i: 1, to: { x: 39.3, y: 25 }, shift: false, lines: noLines, tolMm: 1 });
  expect(snapped[1].x).toBeCloseTo(40, 9);
  expect(snapped[0]).toEqual(pts[0]);
  expect(snapped[2]).toEqual(pts[2]);
  const shifted = dragPoint({ pts, i: 1, to: { x: 20, y: 17 }, shift: true, lines: noLines, tolMm: 1 });
  const deg = (Math.atan2(shifted[1].y, shifted[1].x) * 180) / Math.PI;
  expect(deg).toBeCloseTo(45, 9);
  // Le premier point se règle depuis le suivant.
  const first = dragPoint({ pts, i: 0, to: { x: 0.4, y: 9.7 }, shift: false, lines: noLines, tolMm: 0 });
  expect(first[0].y).toBeCloseTo(10, 9);
});

test("insertPoint refuses the 201st point", () => {
  const full = Array.from({ length: 200 }, (_, i) => ({ x: i, y: 0 }));
  expect(insertPoint(full, 3, { x: 3.5, y: 5 })).toBe(full);
});

test("closed curve wraps around; parity with Rust", () => {
  const sq = [{ x: 0, y: 0 }, { x: 10, y: 0 }, { x: 10, y: 10 }, { x: 0, y: 10 }];
  const r = (v: number) => Math.round(v * 1e6) / 1e6 + 0;
  // Copié de shapes.rs (CLOSED_PARITY).
  expect(closedCurveBeziers(sq).flatMap((b) => b.flatMap((p) => [r(p.x), r(p.y)]))).toEqual([1.666667, -1.666667, 8.333333, -1.666667, 10.0, 0.0, 11.666667, 1.666667, 11.666667, 8.333333, 10.0, 10.0, 8.333333, 11.666667, 1.666667, 11.666667, 0.0, 10.0, -1.666667, 8.333333, -1.666667, 1.666667, 0.0, 0.0]);
});

test("a closed path keeps closed through fromPagePoints, and has a closing segment mid", () => {
  const s = line({ closed: true, fill: "#ff0000" });
  const sq = [{ x: 10, y: 10 }, { x: 30, y: 10 }, { x: 30, y: 30 }];
  expect(fromPagePoints(sq, s).patch.closed).toBe(true);
  expect(fromPagePoints(sq.slice(0, 2), s).patch.closed).toBeUndefined();
  expect(segmentMids(sq, false, true)).toEqual([{ x: 20, y: 10 }, { x: 30, y: 20 }, { x: 20, y: 20 }]);
});

test("toShapePath: a closed square becomes a closed path in the 0..1000 box", () => {
  const frame: Frame = { x_mm: 0, y_mm: 0, w_mm: 1, h_mm: 1, rotation_deg: 0 };
  const s = line({ closed: true });
  const r = fromPagePoints([{ x: 10, y: 20 }, { x: 50, y: 20 }, { x: 50, y: 60 }, { x: 10, y: 60 }], s);
  const out = toShapePath(r.frame, { ...s, ...r.patch }, false, false);
  expect(out.frame).toEqual({ x_mm: 10, y_mm: 20, w_mm: 40, h_mm: 40, rotation_deg: 0 });
  expect(out.path).toBe("M 0 0 L 1000 0 L 1000 1000 L 0 1000 Z");
  void frame;
});

test("toShapePath: an open curve uses C and a flat path keeps a height", () => {
  const s = line({ curve: true });
  const r = fromPagePoints([{ x: 0, y: 0 }, { x: 20, y: 20 }, { x: 40, y: 0 }], s);
  const out = toShapePath(r.frame, { ...s, ...r.patch }, false, false);
  expect(out.path).toMatch(/^M [\d.]+ [\d.]+( C( [-\d.]+){6}){2}$/);
  const flat = fromPagePoints([{ x: 0, y: 5 }, { x: 20, y: 5 }, { x: 40, y: 5 }], line());
  expect(toShapePath(flat.frame, { ...line(), ...flat.patch }, false, false).frame.h_mm).toBeGreaterThan(0);
});

test("fromShapePath reads back what toShapePath wrote: straight, closed, curved", () => {
  const pts = [{ x: 10, y: 20 }, { x: 50, y: 25 }, { x: 40, y: 60 }, { x: 15, y: 50 }];
  for (const extra of [{}, { closed: true }, { curve: true }, { curve: true, closed: true }]) {
    const s = line(extra);
    const r = fromPagePoints(pts, s);
    const shape = toShapePath(r.frame, { ...s, ...r.patch }, false, false);
    const back = fromShapePath(shape.frame, shape.path, false, false)!;
    expect(back.closed).toBe(!!(extra as { closed?: boolean }).closed);
    expect(back.curve).toBe(!!(extra as { curve?: boolean }).curve);
    back.pts.forEach((p, i) => {
      expect(p.x).toBeCloseTo(pts[i].x, 1);
      expect(p.y).toBeCloseTo(pts[i].y, 1);
    });
  }
});

test("fromShapePath follows rotation and mirror of the shape", () => {
  const frame: Frame = { x_mm: 0, y_mm: 0, w_mm: 100, h_mm: 50, rotation_deg: 90 };
  const back = fromShapePath(frame, "M 0 0 L 1000 0 L 1000 1000", true, false)!;
  // Miroir : x -> 1000 - x ; puis un quart de tour autour du centre (50, 25).
  expect(back.pts.map((p) => [Math.round(p.x), Math.round(p.y)])).toEqual([[75, 75], [75, -25], [25, -25]]);
});

test("fromShapePath refuses what it cannot rebuild", () => {
  const f: Frame = { x_mm: 0, y_mm: 0, w_mm: 10, h_mm: 10, rotation_deg: 0 };
  expect(fromShapePath(f, "M 0 0 L 10 10 M 20 20 L 30 30", false, false)).toBeNull();
  expect(fromShapePath(f, "M 0 100 C 450 100 550 900 1000 900", false, false)).toBeNull();
  expect(fromShapePath(f, "M 0 0 C 900 0 0 900 500 500 C 0 0 0 0 1000 1000", false, false)).toBeNull();
  expect(fromShapePath(f, "M 0 0 L 500 0 C 600 0 600 100 1000 1000", false, false)).toBeNull();
  expect(fromShapePath(f, "M 0 0 L 1000 1000", false, false)?.pts).toHaveLength(2);
});

test("endNear finds the end under the press, if any", () => {
  const s = line();
  const frame: Frame = { x_mm: 40, y_mm: 60, w_mm: 80, h_mm: 4, rotation_deg: 0 };
  expect(endNear(frame, s, false, false, { x: 41, y: 62.5 }, 3)).toBe("start");
  expect(endNear(frame, s, false, false, { x: 119, y: 62 }, 3)).toBe("end");
  expect(endNear(frame, s, false, false, { x: 80, y: 62 }, 3)).toBeNull();
  // Miroir : le début est à droite.
  expect(endNear(frame, s, true, false, { x: 119, y: 62 }, 3)).toBe("start");
  // Un chemin fermé n'a pas de bouts.
  const closed = line({ closed: true, points: [[0, 0.5], [0.5, 0], [1, 0.5]] });
  expect(endNear(frame, closed, false, false, { x: 40, y: 62 }, 3)).toBeNull();
});
