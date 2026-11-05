import { describe, expect, test } from "vitest";
import vectors from "../../../fixtures/crop_vectors.json";
import { MIN_TRIMMED_MM, cropPosition, cropToCss, dragCrop, photoTrims, sourceRect, wheelCrop, wholePhoto, withPosition, withTrims } from "./crop";
import type { AssetMeta, Crop, Frame } from "./types";

describe("crop", () => {
  test("sourceRect matches shared vectors", () => {
    expect(vectors.length).toBe(5);
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

  test("in a photo turned or mirrored, the visible area follows the mouse", () => {
    const c = { scale: 2, center_x: 0.5, center_y: 0.5 };
    const straight = dragCrop(c, 0, 20, meta, frame, 1);
    // Tournée de 90° (sens horaire) : le bas du cadre est à gauche de l'écran, glisser vers la gauche fait
    // comme glisser vers le bas dans une photo droite.
    const turned = dragCrop(c, -20, 0, meta, { ...frame, rotation_deg: 90 }, 1);
    expect(turned.center_x).toBeCloseTo(0.5, 6);
    expect(turned.center_y).toBeCloseTo(straight.center_y, 6);
    // En miroir : la gauche et la droite s'inversent.
    const right = dragCrop(c, 20, 0, meta, frame, 1);
    const mirrored = dragCrop(c, 20, 0, meta, frame, 1, true);
    expect(mirrored.center_x - 0.5).toBeCloseTo(0.5 - right.center_x, 6);
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

describe("trim", () => {
  const meta: AssetMeta = { id: "a", width_px: 4000, height_px: 3000, mime: "Jpeg" };
  const flat: Frame = { x_mm: 20, y_mm: 30, w_mm: 100, h_mm: 100, rotation_deg: 0 };
  const cover = { scale: 1, center_x: 0.5, center_y: 0.5 };

  /** Centre de la photo entière et sa taille, sur la page, tels qu'on les voit (rotation et miroirs compris). */
  function seen(frame: Frame, crop: Crop, flipH = false, flipV = false) {
    const css = cropToCss(meta, frame, crop);
    let lx = css.leftMm + css.widthMm / 2 - frame.w_mm / 2;
    let ly = css.topMm + css.heightMm / 2 - frame.h_mm / 2;
    if (flipH) lx = -lx;
    if (flipV) ly = -ly;
    const t = (frame.rotation_deg * Math.PI) / 180;
    return {
      x: frame.x_mm + frame.w_mm / 2 + lx * Math.cos(t) - ly * Math.sin(t),
      y: frame.y_mm + frame.h_mm / 2 + lx * Math.sin(t) + ly * Math.cos(t),
      w: css.widthMm,
      h: css.heightMm,
    };
  }

  function expectSamePhoto(a: ReturnType<typeof seen>, b: ReturnType<typeof seen>) {
    expect(a.x).toBeCloseTo(b.x, 6);
    expect(a.y).toBeCloseTo(b.y, 6);
    expect(a.w).toBeCloseTo(b.w, 6);
    expect(a.h).toBeCloseTo(b.h, 6);
  }

  test("photoTrims gives what each edge hides", () => {
    // Photo 4:3 dans un cadre carré : 16,67 mm cachés à gauche et à droite.
    const t = photoTrims(meta, flat, cover);
    expect(t.left).toBeCloseTo(16.667, 3);
    expect(t.right).toBeCloseTo(16.667, 3);
    expect(t.top).toBeCloseTo(0, 6);
    expect(t.bottom).toBeCloseTo(0, 6);
  });

  test("trimming an edge shrinks the frame and leaves the photo where it was", () => {
    const before = seen(flat, cover);
    const r = withTrims(meta, flat, cover, { left: 16.667, right: 16.667, top: 20, bottom: 0 });
    expect(r.frame.w_mm).toBeCloseTo(100, 2);
    expect(r.frame.h_mm).toBeCloseTo(80, 6);
    expect(r.frame.y_mm).toBeCloseTo(50, 6);
    expectSamePhoto(seen(r.frame, r.crop), before);
    const t = photoTrims(meta, r.frame, r.crop);
    expect(t.top).toBeCloseTo(20, 6);
    expect(t.bottom).toBeCloseTo(0, 6);
  });

  test("an edge cannot go past the photo, nor the frame shrink to nothing", () => {
    const r = withTrims(meta, flat, cover, { left: -50, right: 16.667, top: 0, bottom: 0 });
    expect(photoTrims(meta, r.frame, r.crop).left).toBeCloseTo(0, 6);
    expect(r.frame.w_mm).toBeCloseTo(116.667, 2);
    const tiny = withTrims(meta, flat, cover, { left: 16.667, right: 16.667, top: 60, bottom: 60 });
    expect(tiny.frame.h_mm).toBeCloseTo(MIN_TRIMMED_MM, 6);
  });

  test("a turned, mirrored photo stays in place too", () => {
    for (const [flipH, flipV] of [[false, false], [true, false], [false, true], [true, true]]) {
      const frame = { ...flat, rotation_deg: 30 };
      const crop = { scale: 1.6, center_x: 0.4, center_y: 0.6 };
      const before = seen(frame, crop, flipH, flipV);
      const trims = photoTrims(meta, frame, crop, flipH, flipV);
      const r = withTrims(meta, frame, crop, { ...trims, left: trims.left + 10, bottom: trims.bottom + 5 }, flipH, flipV);
      expectSamePhoto(seen(r.frame, r.crop, flipH, flipV), before);
      const after = photoTrims(meta, r.frame, r.crop, flipH, flipV);
      expect(after.left).toBeCloseTo(trims.left + 10, 6);
      expect(after.bottom).toBeCloseTo(trims.bottom + 5, 6);
      expect(after.right).toBeCloseTo(trims.right, 6);
    }
  });

  test("wholePhoto undoes a trim: the photo keeps its size and place", () => {
    const trimmed = withTrims(meta, flat, cover, { left: 30, right: 16.667, top: 10, bottom: 25 });
    const r = wholePhoto(meta, trimmed.frame, trimmed.crop, false, false, 297, 210);
    expect(r.crop.scale).toBeCloseTo(1, 6);
    expectSamePhoto(seen(r.frame, r.crop), seen(flat, cover));
    expect(r.frame.x_mm).toBeCloseTo(3.333, 2);
    expect(r.frame.w_mm).toBeCloseTo(133.333, 2);
    expect(r.frame.h_mm).toBeCloseTo(100, 6);
  });

  test("wholePhoto shrinks a photo larger than the page to fit it, around its centre", () => {
    // Agrandie deux fois : la photo entière fait 266,7 × 200 mm, plus large qu'une page A4 en portrait.
    const crop = { scale: 2, center_x: 0.3, center_y: 0.5 };
    const r = wholePhoto(meta, flat, crop, false, false, 210, 297);
    expect(r.crop.scale).toBeCloseTo(1, 6);
    expect(r.frame.w_mm).toBeCloseTo(210, 6);
    expect(r.frame.h_mm).toBeCloseTo(157.5, 6);
    // Le centre de la photo entière ne bouge pas : (-10 + 133,3 ; -20 + 100).
    expect(r.frame.x_mm + r.frame.w_mm / 2).toBeCloseTo(123.333, 2);
    expect(r.frame.y_mm + r.frame.h_mm / 2).toBeCloseTo(80, 6);
  });

  test("position runs across the hidden part, and is null without one", () => {
    const crop = { scale: 1, center_x: 0.5, center_y: 0.5 };
    expect(cropPosition(meta, flat, crop)).toEqual({ x: 0.5, y: null });
    expect(withPosition(meta, flat, crop, { x: 0 }).center_x).toBeCloseTo(0.375, 6);
    expect(withPosition(meta, flat, crop, { x: 1 }).center_x).toBeCloseTo(0.625, 6);
    // En miroir, « à gauche » à l'écran montre la droite de la photo.
    expect(withPosition(meta, flat, crop, { x: 0 }, true).center_x).toBeCloseTo(0.625, 6);
  });
});
