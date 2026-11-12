import { expect, test } from "vitest";
import { fr } from "./i18n/fr";
import { libraryView, mineView, photosView } from "./imageSources";
import type { LibraryPhoto } from "./photoLibrary";
import type { ClassPhoto, Document, Element, MyImage, Student } from "./types";

const meta = (id: string, taken: string | null) => ({ id, width_px: 40, height_px: 30, mime: "Jpeg" as const, taken });
const lp = (id: string, taken: string | null, folder = "Ferme"): LibraryPhoto => ({ meta: meta(id, taken), path: `/p/${id}.jpg`, folder });
const photoEl = (asset: string): Element => ({
  id: crypto.randomUUID(),
  frame: { x_mm: 0, y_mm: 0, w_mm: 10, h_mm: 10, rotation_deg: 0 },
  locked: false,
  zone: true,
  kind: { type: "Photo", asset, crop: { scale: 1, center_x: 0.5, center_y: 0.5 }, corner_radius_mm: 0, border: null, shadow: false },
});
const doc = (pages: string[][]): Document => ({
  format_version: 1,
  title: "",
  assets: {},
  pending_photos: [],
  pages: pages.map((assets, i) => ({ id: `p${i}`, size: { width_mm: 210, height_mm: 297 }, grid: null, elements: assets.map(photoEl) })),
});

test("photos du document : Non placées, groupées par jour", () => {
  const photos = [lp("a", "2026-10-05"), lp("b", "2026-10-07"), lp("c", "2026-10-05")];
  const d = doc([[], ["a"]]);
  const v = photosView(photos, d, d.pages[0], "unused");
  expect(v.groups.map((g) => [g.title, g.items.map((i) => i.key)])).toEqual([
    [fr.libraryDayShort("2026-10-07"), ["b"]],
    [fr.libraryDayShort("2026-10-05"), ["c"]],
  ]);
  expect(v.filters.find((f) => f.on)?.id).toBe("unused");
  expect(v.empty).toBeNull();
});

test("photos du document : le numéro de page d'une photo posée", () => {
  const d = doc([[], ["a"]]);
  const v = photosView([lp("a", null)], d, d.pages[0], "all");
  expect(v.groups[0].title).toBe(fr.photosNoDate);
  expect(v.groups[0].items[0].badge).toBe("p. 2");
});

test("photos du document : tout est placé", () => {
  const d = doc([["a"]]);
  const v = photosView([lp("a", null)], d, d.pages[0], "unused");
  expect(v.groups).toEqual([]);
  expect(v.empty).toBe(fr.photosAllPlaced);
});

test("photos du document : un dossier disparu revient à Toutes", () => {
  const d = doc([[]]);
  expect(photosView([lp("a", null)], d, d.pages[0], "folder:Piscine").filters.find((f) => f.on)?.id).toBe("all");
});

const student = (id: string, first_name: string, photo_allowed = true): Student =>
  ({ id, first_name, last_name: "", level: "MS", group: "", photo: null, photo_allowed, symbol: null, birth_date: null }) as Student;
const cp = (id: string, taken: string, students: string[] = [], tags: string[] = []): ClassPhoto => ({ id, width_px: 4, height_px: 3, mime: "Jpeg", taken, added: taken, students, tags });
const today = new Date("2026-11-12T12:00:00Z");

test("photothèque sans classe ouverte : un message, pas de grille", () => {
  const v = libraryView([], [], "year|all", today, false);
  expect(v.groups).toEqual([]);
  expect(v.empty).toBe(fr.libraryNoClass);
});

test("photothèque : les photos d'un enfant, et l'avertissement sans autorisation", () => {
  const students = [student("e1", "Léo"), student("e2", "Inès", false)];
  const photos = [cp("x", "2026-10-05", ["e1"]), cp("y", "2026-10-06", ["e1", "e2"]), cp("z", "2026-10-06", [])];
  const v = libraryView(photos, students, "year|student:e1", today, true);
  expect(v.groups.flatMap((g) => g.items.map((i) => i.key))).toEqual(["y", "x"]);
  expect(v.groups[0].items[0].warn).toBe(fr.libraryNoConsent(["Inès"]));
  expect(v.groups[0].items[0].badge).toBe("2");
  expect(v.filters.find((f) => f.id === "year|student:e1")?.on).toBe(true);
});

const img = (name: string, themes: string[]): MyImage => ({ name, size: 1, modified: 1, themes });

test("collection : groupée par premier thème, Sans thème à la fin", () => {
  const v = mineView([img("pomme.png", ["Automne", "Fruits"]), img("chat.png", []), img("feuille.png", ["Automne"])], "all");
  expect(v.groups.map((g) => [g.title, g.items.map((i) => i.key)])).toEqual([
    ["Automne", ["pomme.png", "feuille.png"]],
    [fr.imagesNoTheme, ["chat.png"]],
  ]);
  expect(v.groups[0].items[0].title).toBe("pomme");
});

test("collection filtrée sur un thème : un seul groupe", () => {
  const v = mineView([img("pomme.png", ["Automne", "Fruits"]), img("chat.png", [])], "theme:Fruits");
  expect(v.groups.map((g) => [g.title, g.items.map((i) => i.key)])).toEqual([["Fruits", ["pomme.png"]]]);
});
