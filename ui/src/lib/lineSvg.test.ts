import { expect, test } from "vitest";
import { lineSvg } from "./lineSvg";

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
