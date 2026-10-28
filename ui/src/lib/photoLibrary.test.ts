import { expect, test } from "vitest";
import { filterPhotos, folderChips, pendingPaths, usedAssetIds, type LibraryPhoto } from "./photoLibrary";
import type { Document, Element } from "./types";

const meta = (id: string) => ({ id, width_px: 40, height_px: 30, mime: "Jpeg" as const });
const lp = (id: string, folder: string, path = `/photos/${folder}/${id}.jpg`): LibraryPhoto => ({ meta: meta(id), path, folder });

const photoEl = (asset: string | null): Element => ({
  id: crypto.randomUUID(),
  frame: { x_mm: 0, y_mm: 0, w_mm: 10, h_mm: 10, rotation_deg: 0 },
  locked: false,
  zone: true,
  kind: { type: "Photo", asset, crop: { scale: 1, center_x: 0.5, center_y: 0.5 }, corner_radius_mm: 0, border: null, shadow: false },
});

const doc = (pages: (string | null)[][]): Document => ({
  format_version: 1,
  title: "",
  assets: {},
  pending_photos: [],
  pages: pages.map((assets, i) => ({ id: `p${i}`, size: { width_mm: 210, height_mm: 297 }, grid: null, elements: assets.map(photoEl) })),
});

const photos = [lp("a", "Ferme"), lp("b", "Ferme"), lp("c", "Piscine"), lp("d", "Ferme")];

test("usedAssetIds collects photos placed on every page", () => {
  expect([...usedAssetIds(doc([["a", null], ["c"]]))].sort()).toEqual(["a", "c"]);
});

test("filterPhotos all, unused and folder", () => {
  const used = new Set(["a", "c"]);
  expect(filterPhotos(photos, { kind: "all" }, used).map((p) => p.meta.id)).toEqual(["a", "b", "c", "d"]);
  expect(filterPhotos(photos, { kind: "unused" }, used).map((p) => p.meta.id)).toEqual(["b", "d"]);
  expect(filterPhotos(photos, { kind: "folder", folder: "Piscine" }, used).map((p) => p.meta.id)).toEqual(["c"]);
});

test("folderChips counts photos per folder in order of first appearance", () => {
  expect(folderChips(photos)).toEqual([
    { folder: "Ferme", count: 3 },
    { folder: "Piscine", count: 1 },
  ]);
});

test("pendingPaths lists unused photos once", () => {
  const withDuplicate = [...photos, lp("e", "Ferme", "/photos/Ferme/b.jpg")];
  expect(pendingPaths(withDuplicate, doc([["a", "c"]]))).toEqual(["/photos/Ferme/b.jpg", "/photos/Ferme/d.jpg"]);
});
