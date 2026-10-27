import { describe, expect, test } from "vitest";
import vectors from "../../../fixtures/crop_vectors.json";
import { cropToCss, dragCrop, sourceRect, wheelCrop } from "./crop";
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

describe("crop editing", () => {
  const meta: AssetMeta = { id: "a", width_px: 4000, height_px: 3000, mime: "Jpeg" };
  const frame: Frame = { x_mm: 0, y_mm: 0, w_mm: 100, h_mm: 100, rotation_deg: 0 };

  test("dragCrop moves the visible area opposite to the drag and clamps", () => {
    const c = { scale: 1, center_x: 0.5, center_y: 0.5 };
    // Glisser l'image vers la droite montre une partie plus à gauche de l'image.
    const moved = dragCrop(c, 20, 0, meta, frame, 1);
    expect(moved.center_x).toBeLessThan(0.5);
    expect(moved.center_y).toBe(0.5);
    // Très loin : bloqué au bord gauche (la partie visible fait 3000 px sur 4000).
    const far = dragCrop(c, 10000, 0, meta, frame, 1);
    expect(far.center_x).toBeCloseTo(1500 / 4000, 6);
    // En hauteur, l'image entière est déjà visible : le centre ne bouge pas.
    expect(dragCrop(c, 0, 500, meta, frame, 1).center_y).toBe(0.5);
  });

  test("wheelCrop clamps scale between 1 and 5", () => {
    let c = { scale: 1, center_x: 0.5, center_y: 0.5 };
    c = wheelCrop(c, 100);
    expect(c.scale).toBe(1);
    for (let i = 0; i < 50; i++) c = wheelCrop(c, -100);
    expect(c.scale).toBe(5);
    expect(wheelCrop({ scale: 2, center_x: 0.5, center_y: 0.5 }, -100).scale).toBeCloseTo(2.2, 6);
  });
});
