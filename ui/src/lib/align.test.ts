import { expect, test } from "vitest";
import { alignFrames, distributeFrames } from "./align";
import type { Frame } from "./types";

const f = (x_mm: number, y_mm: number, w_mm: number, h_mm: number): Frame => ({ x_mm, y_mm, w_mm, h_mm, rotation_deg: 0 });

test("alignFrames left / hcenter / bottom", () => {
  const frames = [f(10, 10, 20, 10), f(40, 30, 40, 20)];
  expect(alignFrames(frames, "left").map((x) => x.x_mm)).toEqual([10, 10]);
  expect(alignFrames(frames, "hcenter").map((x) => x.x_mm)).toEqual([35, 25]);
  expect(alignFrames(frames, "bottom").map((x) => x.y_mm + x.h_mm)).toEqual([50, 50]);
  expect(alignFrames(frames, "right").map((x) => x.x_mm + x.w_mm)).toEqual([80, 80]);
});

test("distributeFrames equalizes gaps and keeps the outer frames", () => {
  const frames = [f(0, 0, 10, 10), f(15, 0, 10, 10), f(90, 0, 10, 10)];
  const out = distributeFrames(frames, "h");
  expect(out[0].x_mm).toBe(0);
  expect(out[2].x_mm).toBe(90);
  expect(out[1].x_mm).toBe(45);
  expect(distributeFrames(frames.slice(0, 2), "h")).toEqual(frames.slice(0, 2));
});

test("distributeFrames sorts by position but returns frames in input order", () => {
  const frames = [f(0, 90, 10, 10), f(0, 0, 10, 10), f(0, 20, 10, 10)];
  expect(distributeFrames(frames, "v").map((x) => x.y_mm)).toEqual([90, 0, 45]);
});
