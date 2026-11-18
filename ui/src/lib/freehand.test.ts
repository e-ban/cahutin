import { expect, test } from "vitest";
import { simplifyStroke, strokeElement } from "./freehand";
import { newDocument } from "./factory";
import { isPath, pagePoints, MAX_PATH_POINTS, type Pt } from "./linePath";

test("simplifyStroke keeps the ends and drops points along a straight run", () => {
  const pts: Pt[] = Array.from({ length: 50 }, (_, i) => ({ x: i, y: 0 }));
  expect(simplifyStroke(pts, 0.3)).toEqual([{ x: 0, y: 0 }, { x: 49, y: 0 }]);
});

test("simplifyStroke keeps the corners", () => {
  const pts: Pt[] = [...Array.from({ length: 20 }, (_, i) => ({ x: i, y: 0 })), ...Array.from({ length: 20 }, (_, i) => ({ x: 19, y: i + 1 }))];
  const out = simplifyStroke(pts, 0.3);
  expect(out).toEqual([{ x: 0, y: 0 }, { x: 19, y: 0 }, { x: 19, y: 20 }]);
});

test("simplifyStroke never returns more than 200 points", () => {
  const wiggle: Pt[] = Array.from({ length: 2000 }, (_, i) => ({ x: i * 0.1, y: Math.sin(i) * 5 }));
  const out = simplifyStroke(wiggle, 0.3);
  expect(out.length).toBeLessThanOrEqual(MAX_PATH_POINTS);
  expect(out[0]).toEqual(wiggle[0]);
  expect(out[out.length - 1]).toEqual(wiggle[wiggle.length - 1]);
});

test("strokeElement makes a curved path through the simplified points", () => {
  const page = newDocument().pages[0];
  const pts: Pt[] = Array.from({ length: 60 }, (_, i) => ({ x: 20 + i, y: 50 + 10 * Math.sin(i / 6) }));
  const e = strokeElement(page, pts)!;
  if (e.kind.type !== "Shape") throw new Error();
  expect(e.kind.shape).toBe("Line");
  expect(isPath(e.kind)).toBe(true);
  expect(e.kind.curve).toBe(true);
  const back = pagePoints(e.frame, e.kind, false, false);
  expect(back[0].x).toBeCloseTo(20, 6);
  expect(back[back.length - 1].x).toBeCloseTo(79, 6);
});

test("strokeElement: a click or a tiny stroke draws nothing; a straight stroke is a straight line", () => {
  const page = newDocument().pages[0];
  expect(strokeElement(page, [{ x: 10, y: 10 }])).toBeNull();
  expect(strokeElement(page, [{ x: 10, y: 10 }, { x: 10.5, y: 10.2 }])).toBeNull();
  const line = strokeElement(page, Array.from({ length: 30 }, (_, i) => ({ x: 10 + i, y: 10 })))!;
  if (line.kind.type !== "Shape") throw new Error();
  expect(isPath(line.kind)).toBe(false);
  expect(line.frame.w_mm).toBeCloseTo(29, 6);
});
