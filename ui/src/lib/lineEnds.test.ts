import { expect, test } from "vitest";
import { dragEnd, endsOf, isLineTarget, frameFromEndpoints, headWidthMm, lineEndpoints, normalizeEnds, withHitHeight } from "./lineEnds";
import type { Frame, LineEnds, ShapeProps } from "./types";

const shape = (kind: "Line" | "Arrow", width_mm: number, ends: LineEnds | null = null): ShapeProps => ({
  shape: kind,
  fill: null,
  stroke: { width_mm, color: "#333333", style: null },
  corner_radius_mm: 0,
  path: null,
  ends: ends ?? undefined,
});
const frame = (rotation_deg: number): Frame => ({ x_mm: 40, y_mm: 60, w_mm: 80, h_mm: 8, rotation_deg });
const noLines = { xs: [], ys: [] };
const angleOf = (f: Frame) => f.rotation_deg;

test("endpoints round-trip", () => {
  for (const r of [0, 30, -135]) {
    for (const flip of [false, true]) {
      const f = frame(r);
      const [a, b] = lineEndpoints(f, flip);
      const g = frameFromEndpoints(a, b, f.h_mm, flip);
      for (const k of ["x_mm", "y_mm", "w_mm", "h_mm", "rotation_deg"] as const) expect(g[k]).toBeCloseTo(f[k], 9);
    }
  }
});

test("endpoints of a flat line: start at the left, end at the right; mirrored, the other way", () => {
  expect(lineEndpoints(frame(0), false)).toEqual([{ x: 40, y: 64 }, { x: 120, y: 64 }]);
  expect(lineEndpoints(frame(0), true)).toEqual([{ x: 120, y: 64 }, { x: 40, y: 64 }]);
});

test("dragEnd moves only that end", () => {
  for (const flip of [false, true]) {
    const f = frame(30);
    const [s] = lineEndpoints(f, flip);
    const g = dragEnd({ frame: f, flipH: flip, which: "end", to: { x: 100, y: 130 }, shift: false, lines: noLines, tolMm: 0 });
    const [s2, e2] = lineEndpoints(g, flip);
    expect(s2.x).toBeCloseTo(s.x, 9);
    expect(s2.y).toBeCloseTo(s.y, 9);
    expect(e2.x).toBeCloseTo(100, 9);
    expect(e2.y).toBeCloseTo(130, 9);
  }
});

test("dragEnd magnet: near horizontal it lies flat, further it keeps its angle", () => {
  const f = frame(0);
  const at = (deg: number) => ({ x: 40 + 80 * Math.cos((deg * Math.PI) / 180), y: 64 + 80 * Math.sin((deg * Math.PI) / 180) });
  expect(angleOf(dragEnd({ frame: f, flipH: false, which: "end", to: at(2), shift: false, lines: noLines, tolMm: 0 }))).toBeCloseTo(0, 9);
  expect(angleOf(dragEnd({ frame: f, flipH: false, which: "end", to: at(4), shift: false, lines: noLines, tolMm: 0 }))).toBeCloseTo(4, 6);
  expect(angleOf(dragEnd({ frame: f, flipH: false, which: "end", to: at(88), shift: false, lines: noLines, tolMm: 0 }))).toBeCloseTo(90, 9);
});

test("dragEnd shift snaps to 45°", () => {
  const to = { x: 40 + 80 * Math.cos((50 * Math.PI) / 180), y: 64 + 80 * Math.sin((50 * Math.PI) / 180) };
  const g = dragEnd({ frame: frame(0), flipH: false, which: "end", to, shift: true, lines: { xs: [87], ys: [] }, tolMm: 5 });
  expect(angleOf(g)).toBeCloseTo(45, 9);
  expect(g.w_mm).toBeCloseTo(80, 9);
});

test("dragEnd keeps 3 mm when ends meet", () => {
  const g = dragEnd({ frame: frame(0), flipH: false, which: "end", to: { x: 40, y: 64 }, shift: false, lines: noLines, tolMm: 0 });
  expect(g.w_mm).toBeCloseTo(3, 9);
  for (const v of Object.values(g)) expect(Number.isNaN(v)).toBe(false);
  const [s, e] = lineEndpoints(g, false);
  expect(s).toEqual({ x: 40, y: 64 });
  expect(e.x).toBeCloseTo(43, 9);
});

test("dragEnd snaps to a line, and the angle magnet then stays out", () => {
  const g = dragEnd({ frame: frame(0), flipH: false, which: "end", to: { x: 100.8, y: 66.3 }, shift: false, lines: { xs: [100], ys: [] }, tolMm: 1 });
  const [, e] = lineEndpoints(g, false);
  expect(e.x).toBeCloseTo(100, 9);
  expect(e.y).toBeCloseTo(66.3, 9);
});

test("dragEnd: the angle magnet beats the 5 mm grid", () => {
  const g = dragEnd({ frame: frame(0), flipH: false, which: "end", to: { x: 101.3, y: 65.9 }, shift: false, lines: noLines, tolMm: 1 });
  expect(angleOf(g)).toBeCloseTo(0, 9);
});

test("dragEnd snaps to the 5 mm grid when nothing else applies", () => {
  const g = dragEnd({ frame: frame(0), flipH: false, which: "end", to: { x: 100.6, y: 90.4 }, shift: false, lines: noLines, tolMm: 1 });
  const [, e] = lineEndpoints(g, false);
  expect(e.x).toBeCloseTo(100, 9);
  expect(e.y).toBeCloseTo(90, 9);
});

test("withHitHeight grows h to the head width", () => {
  const big = shape("Arrow", 3, { start: "None", end: "Arrow", size: "Large" });
  const g = withHitHeight(frame(30), big);
  expect(g.h_mm).toBeCloseTo(18, 9);
  expect(g.y_mm + g.h_mm / 2).toBeCloseTo(64, 9);
  expect(g.x_mm).toBe(40);
  const tall = { ...frame(0), h_mm: 30 };
  expect(withHitHeight(tall, big)).toEqual(tall);
  expect(withHitHeight(frame(0), shape("Line", 3))).toEqual(frame(0));
});

test("endsOf and normalizeEnds", () => {
  expect(endsOf(shape("Line", 1))).toEqual({ start: "None", end: "None", size: "Medium" });
  expect(endsOf(shape("Arrow", 1))).toEqual({ start: "None", end: "Arrow", size: "Medium" });
  expect(normalizeEnds("Arrow", { start: "None", end: "Arrow", size: "Medium" })).toBeNull();
  expect(normalizeEnds("Line", { start: "None", end: "Arrow", size: "Medium" })).toEqual({ start: "None", end: "Arrow", size: "Medium" });
});

test("headWidthMm matches the Rust sizes", () => {
  expect(headWidthMm(shape("Arrow", 1))).toBe(4);
  expect(headWidthMm(shape("Arrow", 2))).toBe(8);
  expect(headWidthMm(shape("Arrow", 1, { start: "None", end: "Arrow", size: "Small" }))).toBeCloseTo(2.8, 9);
  expect(headWidthMm(shape("Arrow", 2, { start: "None", end: "Arrow", size: "Large" }))).toBeCloseTo(12, 9);
});

test("isLineTarget: lines and arrows, never a graphisme base line", () => {
  const el = (kind: ShapeProps) => ({ id: "a", frame: frame(0), locked: false, zone: false, kind: { type: "Shape" as const, ...kind }, subgroups: [], flip_h: false, flip_v: false });
  expect(isLineTarget(el(shape("Arrow", 1)))).toBe(true);
  expect(isLineTarget(el(shape("Line", 1)))).toBe(true);
  expect(isLineTarget(el({ ...shape("Line", 1), shape: "Rect" }))).toBe(false);
  const recipe = { motif: "ponts", level: "MS", cells: [], guides: "Base", arrow: true } as unknown as NonNullable<ShapeProps["graphisme"]>;
  expect(isLineTarget(el({ ...shape("Line", 1), graphisme: recipe }))).toBe(false);
  expect(isLineTarget(undefined)).toBe(false);
});
