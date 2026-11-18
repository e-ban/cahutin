import { expect, test } from "vitest";
import { lineSvg, pathSvg } from "./lineSvg";
import { pathPointsMm } from "./linePath";

test("default arrow matches today", () => {
  const svg = lineSvg(200, 30, { start: "None", end: "Arrow", size: "Medium" }, 16);
  expect(svg.x1).toBe(0);
  expect(svg.x2).toBe(200 - 12);
  expect(svg.parts).toEqual([{ kind: "polygon", attrs: { points: "200,15 176,7 176,23" } }]);
});

test("a plain line runs edge to edge with nothing at its ends", () => {
  const svg = lineSvg(200, 30, { start: "None", end: "None", size: "Medium" }, 16);
  expect([svg.x1, svg.x2, svg.parts]).toEqual([0, 200, []]);
});

test("a filled arrow at the start points left and shortens the segment there", () => {
  const svg = lineSvg(200, 30, { start: "Arrow", end: "None", size: "Medium" }, 16);
  expect(svg.x1).toBe(12);
  expect(svg.parts).toEqual([{ kind: "polygon", attrs: { points: "0,15 24,7 24,23" } }]);
});

test("open arrow keeps the segment to the tip", () => {
  const svg = lineSvg(200, 30, { start: "None", end: "OpenArrow", size: "Medium" }, 16);
  expect(svg.x2).toBe(200);
  expect(svg.parts).toHaveLength(1);
  expect(svg.parts[0].kind).toBe("polyline");
  expect(svg.parts[0].attrs.points).toBe("176,7 200,15 176,23");
});

test("circle and square centred on the end", () => {
  const svg = lineSvg(200, 30, { start: "Circle", end: "Square", size: "Medium" }, 16);
  expect([svg.x1, svg.x2]).toEqual([0, 200]);
  const d = 16 * 0.6;
  expect(svg.parts[0]).toEqual({ kind: "circle", attrs: { cx: 0, cy: 15, r: d / 2 } });
  expect(svg.parts[1]).toEqual({ kind: "rect", attrs: { x: 200 - d / 2, y: 15 - d / 2, width: d, height: d } });
});

test("path with elbows, no ends", () => {
  const svg = pathSvg([{ x: 0, y: 0 }, { x: 10, y: 0 }, { x: 10, y: 10 }], false, { start: "None", end: "None", size: "Medium" }, 4);
  expect(svg.d).toBe("M 0 0 L 10 0 L 10 10");
  expect(svg.parts).toEqual([]);
});

test("filled arrow at the end follows the last segment", () => {
  const svg = pathSvg([{ x: 0, y: 0 }, { x: 10, y: 0 }, { x: 10, y: 10 }], false, { start: "None", end: "Arrow", size: "Medium" }, 4);
  expect(svg.d).toBe("M 0 0 L 10 0 L 10 7");
  expect(svg.parts).toHaveLength(1);
  expect(svg.parts[0].attrs.transform).toBe("translate(10 10) rotate(90)");
});

test("a start end points backwards along the first segment", () => {
  const svg = pathSvg([{ x: 0, y: 0 }, { x: 10, y: 0 }, { x: 10, y: 10 }], false, { start: "Circle", end: "None", size: "Medium" }, 4);
  expect(svg.d).toBe("M 0 0 L 10 0 L 10 10");
  expect(svg.parts[0].attrs.transform).toBe("translate(0 0) rotate(0)");
});

test("path parts in mm do not stretch", () => {
  const s = { shape: "Arrow" as const, fill: null, stroke: { width_mm: 1, color: "#000", style: null }, corner_radius_mm: 0, path: null, points: [[0, 0], [0.5, 1], [1, 0]] as [number, number][] };
  const narrow = pathPointsMm({ x_mm: 0, y_mm: 0, w_mm: 20, h_mm: 10, rotation_deg: 0 }, s);
  const wide = pathPointsMm({ x_mm: 0, y_mm: 0, w_mm: 40, h_mm: 10, rotation_deg: 0 }, { ...s, points: [[0.25, 0], [0.5, 1], [0.75, 0]] });
  const shift = wide.map((p) => ({ x: p.x - 10, y: p.y }));
  const ends = { start: "Arrow", end: "Square", size: "Large" } as const;
  expect(pathSvg(shift, false, ends, 6).parts).toEqual(pathSvg(narrow, false, ends, 6).parts);
});

test("curve d uses C", () => {
  const svg = pathSvg([{ x: 0, y: 0 }, { x: 10, y: 10 }, { x: 20, y: 0 }], true, { start: "None", end: "None", size: "Medium" }, 4);
  expect(svg.d).toMatch(/^M [-\d.]+ [-\d.]+( C( [-\d.]+){6})+$/);
});
