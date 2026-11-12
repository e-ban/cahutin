import { expect, test } from "vitest";
import { MANY_PHOTOS, assetsOnPage, byDay, elementAssets, filterPhotos, folderChips, newlyPlaced, pageBadge, pendingPaths, shownFilter, startFilter, usedAssetIds, type LibraryPhoto } from "./photoLibrary";
import { fr } from "./i18n/fr";
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
/** Une photo datée (ou sans date : document d'avant la version 10). */
const lpd = (id: string, taken: string | null): LibraryPhoto => ({ meta: { ...meta(id), taken }, path: `/p/${id}.jpg`, folder: "Ferme" });

test("usedAssetIds collects photos placed on every page", () => {
  expect([...usedAssetIds(doc([["a", null], ["c"]]))].sort()).toEqual(["a", "c"]);
});

test("a text frame's background image counts as a used photo", () => {
  const d = doc([["a"]]);
  d.pages[0].elements.push({
    id: "t",
    frame: { x_mm: 0, y_mm: 0, w_mm: 10, h_mm: 10, rotation_deg: 0 },
    locked: false,
    zone: false,
    kind: { type: "Text", content: { paragraphs: [] }, style: { font: "Andika", size_pt: 14, color: "#222222", align: "Left" }, prompt: null, frame: { fill: null, stroke: null, corner_radius_mm: 0, image: "b" } },
  });
  expect([...usedAssetIds(d)].sort()).toEqual(["a", "b"]);
  expect(elementAssets(d.pages[0].elements[1])).toEqual(["b"]);
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

test("pendingPaths keeps the opened document's paths that are not re-imported yet", () => {
  // Réimport en cours, ou clé USB absente : ces chemins ne doivent pas être oubliés au prochain enregistrement.
  const carried = ["/usb/x.jpg", "/photos/Ferme/b.jpg", "/photos/Ferme/a.jpg"];
  expect(pendingPaths(photos, doc([["a", "c"]]), carried)).toEqual(["/photos/Ferme/b.jpg", "/photos/Ferme/d.jpg", "/usb/x.jpg"]);
});

test("a photo removed from the document leaves its photo boxes empty, in place", async () => {
  const { pagesUsing, withoutAsset } = await import("./photoLibrary");
  const d = doc([["a", "b"], [], ["a"]]);
  expect(pagesUsing(d, "a")).toEqual([1, 3]);
  const out = withoutAsset(d, "a");
  expect(out.pages.map((p) => p.elements.map((e) => (e.kind.type === "Photo" ? e.kind.asset : "?")))).toEqual([[null, "b"], [], [null]]);
  expect(pagesUsing(out, "a")).toEqual([]);
});

test("byDay: du plus récent au plus ancien, sans date à la fin, ordre d'import gardé dans un jour", () => {
  const g = byDay([lpd("a", "2026-10-05"), lpd("b", null), lpd("c", "2026-10-07"), lpd("d", "2026-10-05")]);
  expect(g.map((x) => [x.day, x.photos.map((p) => p.meta.id)])).toEqual([
    ["2026-10-07", ["c"]],
    ["2026-10-05", ["a", "d"]],
    [null, ["b"]],
  ]);
});

test("un document d'avant la version 10 : toutes ses photos sont sans date", () => {
  expect(byDay([lp("a", "Ferme"), lp("b", "Ferme")]).map((x) => [x.day, x.photos.length])).toEqual([[null, 2]]);
});

test("filtre Sur cette page", () => {
  expect(filterPhotos(photos, { kind: "page" }, new Set(["a", "c"]), new Set(["c"])).map((p) => p.meta.id)).toEqual(["c"]);
  expect(filterPhotos(photos, { kind: "page" }, new Set(["a", "c"]), new Set())).toEqual([]);
});

test("startFilter: Non placées à partir de 20 photos, sauf choix mémorisé", () => {
  expect(MANY_PHOTOS).toBe(20);
  expect(startFilter(null, 19)).toEqual({ kind: "all" });
  expect(startFilter(null, 20)).toEqual({ kind: "unused" });
  expect(startFilter({ kind: "all" }, 300)).toEqual({ kind: "all" });
  expect(startFilter({ kind: "page" }, 3)).toEqual({ kind: "page" });
  // Un dossier n'a de sens que dans son document : il n'est pas repris.
  expect(startFilter({ kind: "folder", folder: "Ferme" }, 300)).toEqual({ kind: "unused" });
});

test("pageBadge: la page où la photo est posée, la première s'il y en a plusieurs", () => {
  expect(pageBadge([])).toBeNull();
  expect(pageBadge([3])).toBe("p. 3");
  expect(pageBadge([2, 5])).toBe("p. 2…");
});

test("newlyPlaced: une photo du tiroir vient d'être posée, pas un autre document ouvert", () => {
  const tiroir = new Set(["a", "b"]);
  expect(newlyPlaced(new Set(), tiroir, new Set(["a"]), tiroir)).toBe(true);
  expect(newlyPlaced(new Set(["a"]), tiroir, new Set(["a"]), tiroir)).toBe(false);
  // Un autre document : ses photos posées n'étaient pas dans le tiroir d'avant.
  expect(newlyPlaced(new Set(), tiroir, new Set(["x", "y"]), new Set(["x", "y", "z"]))).toBe(false);
});

test("shownFilter: un dossier disparu revient à Toutes", () => {
  const chips = [{ folder: "Ferme", count: 2 }];
  expect(shownFilter({ kind: "folder", folder: "Ferme" }, chips)).toEqual({ kind: "folder", folder: "Ferme" });
  expect(shownFilter({ kind: "folder", folder: "Piscine" }, chips)).toEqual({ kind: "all" });
  expect(shownFilter({ kind: "unused" }, chips)).toEqual({ kind: "unused" });
});

test("textes au singulier pour une seule photo", () => {
  expect(fr.photosRemoveUnused(1)).toBe("Retirer la photo non placée");
  expect(fr.photosRemoveUnused(3)).toBe("Retirer les 3 photos non placées");
  expect(fr.photosRemoveUnusedConfirm(1)).toBe("Retirer la photo non placée du document ? Votre fichier n'est pas touché.");
  expect(fr.photosRemovedUnused(1)).toBe("1 photo retirée du document.");
  expect(fr.photosRemovedUnused(2)).toBe("2 photos retirées du document.");
  expect(fr.photosAllPlaced).toBe("Toutes les photos sont placées.");
});

test("assetsOnPage: les photos et les fonds de cadre de la page", () => {
  const d = doc([["a", null]]);
  d.pages[0].elements.push({
    id: "t",
    frame: { x_mm: 0, y_mm: 0, w_mm: 10, h_mm: 10, rotation_deg: 0 },
    locked: false,
    zone: false,
    kind: { type: "Text", content: { paragraphs: [] }, style: { font: "Andika", size_pt: 14, color: "#222222", align: "Left" }, prompt: null, frame: { fill: null, stroke: null, corner_radius_mm: 0, image: "b" } },
  });
  expect([...assetsOnPage(d.pages[0])].sort()).toEqual(["a", "b"]);
});
