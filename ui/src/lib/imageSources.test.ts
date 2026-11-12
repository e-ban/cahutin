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

test("listes vides : des messages qui ne citent pas de bouton absent de la vue agrandie", () => {
  const d = doc([[]]);
  expect(photosView([], d, d.pages[0], "all").empty).toBe(fr.imagesNoPhotos);
  expect(libraryView([], [], "year|all", today, true).empty).toBe(fr.imagesLibraryEmpty);
});

test("photos du document : un dossier disparu revient à Toutes", () => {
  const d = doc([[]]);
  expect(photosView([lp("a", null)], d, d.pages[0], "folder:Piscine").filters.find((f) => f.on)?.id).toBe("all");
});

const student = (id: string, first_name: string, photo_allowed = true): Student =>
  ({ id, first_name, last_name: "", level: "MS", group: "", photo: null, photo_allowed, symbol: null, birth_date: null }) as Student;
const cp = (id: string, taken: string, students: string[] = [], tags: string[] = [], more: Partial<ClassPhoto> = {}): ClassPhoto => ({
  id,
  width_px: 4,
  height_px: 3,
  mime: "Jpeg",
  taken,
  added: taken,
  students,
  tags,
  path: `/Images/${id}-photo.jpg`,
  folder: "Images",
  size: 1200,
  modified: 1700000000,
  fingerprint: "0",
  hidden: false,
  missing: false,
  ...more,
});
const today = new Date("2026-11-12T12:00:00Z");

test("photothèque sans classe ouverte : un message, pas de grille", () => {
  const v = libraryView([], [], "year|all", today, false);
  expect(v.groups).toEqual([]);
  expect(v.empty).toBe(fr.libraryNoClass);
});

test("photothèque : Toutes et À trier en tête, hors des rubriques (ce ne sont pas des périodes)", () => {
  const v = libraryView([cp("x", "2026-10-05")], [student("e1", "Léo")], "year|all", today, true);
  expect(v.filters.slice(0, 2).map((f) => [f.id, f.heading ?? null])).toEqual([
    ["year|all", null],
    ["all|untagged", null],
  ]);
  expect(v.filters.slice(2, 6).every((f) => f.heading === fr.libraryPeriod)).toBe(true);
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

test("photothèque : les photos retirées ou absentes ne sont ni montrées ni comptées", () => {
  const photos = [cp("x", "2026-10-05"), cp("h", "2026-10-05", [], [], { hidden: true }), cp("m", "2026-10-05", [], [], { missing: true })];
  const v = libraryView(photos, [], "year|all", today, true);
  expect(v.groups.flatMap((g) => g.items.map((i) => i.key))).toEqual(["x"]);
  expect(v.filters[0].label).toBe(`${fr.libraryAll} (1)`);
  expect(libraryView([photos[1], photos[2]], [], "year|all", today, true).empty).toBe(fr.imagesLibraryEmpty);
});

test("photothèque : la rubrique Dossiers filtre par dossier, même avec une barre dans son nom", () => {
  const photos = [cp("a", "2026-10-05", [], [], { folder: "Sortie | ferme" }), cp("b", "2026-10-06", [], [], { folder: "Atelier" }), cp("c", "2026-10-07", [], [], { folder: "Sortie | ferme" })];
  const v = libraryView(photos, [], "year|folder:Sortie | ferme", today, true);
  const folders = v.filters.filter((f) => f.heading === fr.imagesFolders);
  expect(folders.map((f) => [f.id, f.label, f.on])).toEqual([
    ["year|folder:Atelier", "Atelier (1)", false],
    ["year|folder:Sortie | ferme", "Sortie | ferme (2)", true],
  ]);
  expect(v.groups.flatMap((g) => g.items.map((i) => i.key))).toEqual(["c", "a"]);
});

test("photothèque : l'adresse d'une vignette change avec le fichier, pour être rechargée", () => {
  const v = libraryView([cp("x", "2026-10-05")], [], "year|all", today, true);
  expect(v.groups[0].items[0].url).toMatch(/\/lib\/x\/thumb\?v=1200-1700000000$/);
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
