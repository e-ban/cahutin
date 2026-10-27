import { describe, expect, test } from "vitest";
import vectors from "../../../fixtures/crop_vectors.json";
import { cropToCss, sourceRect } from "./crop";
import type { AssetMeta, Frame } from "./types";

describe("crop", () => {
  test("sourceRect matches shared vectors", () => {
    expect(vectors.length).toBe(4);
    for (const v of vectors) {
      const r = sourceRect(v.img_w, v.img_h, v.frame_w, v.frame_h, v.crop);
      expect(r.x).toBeCloseTo(v.expected.x, 6);
      expect(r.y).toBeCloseTo(v.expected.y, 6);
      expect(r.w).toBeCloseTo(v.expected.w, 6);
      expect(r.h).toBeCloseTo(v.expected.h, 6);
    }
  });

  test("cropToCss of first vector", () => {
    const meta: AssetMeta = { id: "a", width_px: 4000, height_px: 3000, mime: "Jpeg" };
    const frame: Frame = { x_mm: 0, y_mm: 0, w_mm: 100, h_mm: 100, rotation_deg: 0 };
    const css = cropToCss(meta, frame, { scale: 1, center_x: 0.5, center_y: 0.5 });
    expect(css.widthMm).toBeCloseTo(133.333, 3);
    expect(css.heightMm).toBeCloseTo(100, 3);
    expect(css.leftMm).toBeCloseTo(-16.667, 3);
    expect(css.topMm).toBeCloseTo(0, 6);
  });
});
