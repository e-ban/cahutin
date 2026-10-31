import { expect, test } from "vitest";
import { replaceAsset, replaceInLibrary, toggleZone, zoneFromDrag } from "./faceBlur";
import type { LibraryPhoto } from "./photoLibrary";
import type { Document, Element } from "./types";

const meta = (id: string) => ({ id, width_px: 40, height_px: 30, mime: "Jpeg" as const });
const photoEl = (asset: string | null): Element => ({
  id: crypto.randomUUID(),
  frame: { x_mm: 0, y_mm: 0, w_mm: 10, h_mm: 10, rotation_deg: 0 },
  locked: false,
  zone: false,
  kind: { type: "Photo", asset, crop: { scale: 2, center_x: 0.3, center_y: 0.4 }, corner_radius_mm: 0, border: null, shadow: false },
});
const doc = (pages: (string | null)[][]): Document => ({
  format_version: 1,
  title: "",
  assets: { a: meta("a"), b: meta("b") },
  pending_photos: [],
  pages: pages.map((assets, i) => ({ id: `p${i}`, size: { width_mm: 210, height_mm: 297 }, grid: null, elements: assets.map(photoEl) })),
});

test("replaceAsset swaps the photo everywhere, keeps the crop, drops the original", () => {
  const out = replaceAsset(doc([["a", "b"], ["a"]]), "a", meta("flou"));
  const assets = out.pages.flatMap((p) => p.elements.map((e) => (e.kind.type === "Photo" ? e.kind.asset : null)));
  expect(assets).toEqual(["flou", "b", "flou"]);
  const k = out.pages[0].elements[0].kind;
  expect(k.type === "Photo" && k.crop).toEqual({ scale: 2, center_x: 0.3, center_y: 0.4 });
  expect(Object.keys(out.assets).sort()).toEqual(["b", "flou"]);
});

test("replaceInLibrary swaps the photo and forgets its file, so the original cannot come back", () => {
  const lib: LibraryPhoto[] = [{ meta: meta("a"), path: "/photos/sortie/a.jpg", folder: "sortie" }, { meta: meta("b"), path: "/p/b.jpg", folder: "p" }];
  const out = replaceInLibrary(lib, "a", meta("flou"));
  expect(out[0]).toEqual({ meta: meta("flou"), path: "", folder: "sortie" });
  expect(out[1]).toBe(lib[1]);
});

test("zoneFromDrag gives a box whatever the drag direction, inside the image", () => {
  expect(zoneFromDrag(0.6, 0.5, 0.4, 0.2)).toEqual({ x: 0.4, y: 0.2, w: 0.2, h: 0.3 });
  expect(zoneFromDrag(0.9, 0.9, 1.4, 1.2)).toEqual({ x: 0.9, y: 0.9, w: 0.1, h: 0.1 });
  expect(zoneFromDrag(0.5, 0.5, 0.501, 0.5)).toBeNull();
});

test("toggleZone adds or removes a box by index", () => {
  expect(toggleZone(new Set([1]), 2)).toEqual(new Set([1, 2]));
  expect(toggleZone(new Set([1, 2]), 1)).toEqual(new Set([2]));
});
