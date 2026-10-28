import { expect, test } from "vitest";
import { frieze, pageFrame } from "./decor";
import type { Page } from "./types";

const a4 = (): Page => ({ id: "p", size: { width_mm: 210, height_mm: 297 }, elements: [], grid: null });

test("pageFrame double makes a grouped pair inside the margin", () => {
  const els = pageFrame(a4(), "double");
  expect(els).toHaveLength(2);
  expect(els[0].group).toBeTruthy();
  expect(els[0].group).toBe(els[1].group);
  expect(els.map((e) => e.frame.x_mm)).toEqual([8, 11]);
  expect(els.map((e) => e.kind.type === "Shape" && e.kind.stroke?.width_mm)).toEqual([0.5, 1.5]);
  expect(els.every((e) => e.kind.type === "Shape" && e.kind.fill === null)).toBe(true);
  const thin = pageFrame(a4(), "thin");
  expect(thin).toHaveLength(1);
  expect(thin[0].group ?? null).toBeNull();
  expect(thin[0].frame).toMatchObject({ x_mm: 8, y_mm: 8, w_mm: 194, h_mm: 281 });
  expect(pageFrame(a4(), "rounded")[0].kind.type === "Shape" && pageFrame(a4(), "rounded")[0].kind.corner_radius_mm).toBe(8);
});

test("frieze top spaces copies evenly within the margins", () => {
  const els = frieze(a4(), "2600", "top");
  const xs = els.map((e) => e.frame.x_mm);
  expect(xs[0]).toBe(8);
  expect(xs.at(-1)! + 15).toBeCloseTo(202, 1);
  const gaps = xs.slice(1).map((x, i) => x - xs[i]);
  expect(Math.max(...gaps) - Math.min(...gaps)).toBeLessThan(0.11);
  expect(els.every((e) => e.frame.y_mm === 8 && e.frame.w_mm === 15)).toBe(true);
  expect(els.length).toBe(Math.floor((194 + 3) / (15 + 3)));
});

test("frieze around has no duplicate corner", () => {
  const els = frieze(a4(), "2600", "around");
  const keys = els.map((e) => `${e.frame.x_mm},${e.frame.y_mm}`);
  expect(new Set(keys).size).toBe(keys.length);
  expect(keys).toContain("8,8");
  expect(keys).toContain("187,274");
});

test("frieze copies share one group", () => {
  const els = frieze(a4(), "1F98A", "bottom");
  expect(new Set(els.map((e) => e.group)).size).toBe(1);
  expect(els[0].group).toBeTruthy();
  expect(els.every((e) => e.kind.type === "Image" && e.kind.code === "1F98A")).toBe(true);
});
