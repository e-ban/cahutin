import { expect, test } from "vitest";
import { hasNameFields, matchPhotos, childOfPage, findChildPage } from "./classFill";
import type { Element, Page, Student } from "./types";

const frame = { x_mm: 0, y_mm: 0, w_mm: 10, h_mm: 10, rotation_deg: 0 };
const text = (prompt: string | null, content: string, zone = prompt !== null): Element => ({
  id: Math.random().toString(),
  frame,
  locked: false,
  zone,
  kind: {
    type: "Text",
    content: { paragraphs: content ? [{ align: null, runs: [{ text: content, bold: false, italic: false, underline: false, color: null, font: null, size_pt: null }] }] : [] },
    style: { font: "Andika", size_pt: 14, color: "#222222", align: "Left" },
    prompt,
  },
});
const page = (elements: Element[]): Page => ({ id: "p", size: { width_mm: 210, height_mm: 297 }, grid: null, elements });

test("hasNameFields: a field in braces, not an old Prénom zone, a photo zone or a title", () => {
  expect(hasNameFields(page([text(null, "{prenom}")]))).toBe(true);
  expect(hasNameFields(page([text(null, "Bravo {prénom} !")]))).toBe(true);
  expect(hasNameFields(page([text(null, "{PRENOM_2}")]))).toBe(true);
  expect(hasNameFields(page([text(null, "{anniversaires_mars}")]))).toBe(true);
  expect(hasNameFields(page([text("Prénom", "")]))).toBe(false);
  expect(hasNameFields(page([text("Titre", "")]))).toBe(false);
  expect(hasNameFields(page([text(null, "Bonjour {toto}")]))).toBe(false);
  // La date et la classe s'écrivent à la création de la page : pas de bandeau pour elles.
  expect(hasNameFields(page([text(null, "{date} {ecole}")]))).toBe(false);
});

const s = (id: string, first_name: string, last_name = ""): Student => ({ id, first_name, last_name, level: null, group: "", photo: null, photo_allowed: false, symbol: null });

test("matchPhotos: by first name, accents and case aside, full name when first names repeat", () => {
  const students = [s("1", "Léa", "Martin"), s("2", "Léa", "Durand"), s("3", "Noé"), s("4", "Anna-Lou")];
  const r = matchPhotos(students, ["/photos/NOE.jpg", "C:\\photos\\lea_durand.JPG", "/p/Anna-Lou 2.png", "/p/léa.jpg", "/p/inconnu.jpg"]);
  expect(r.matched).toEqual([
    { student: "3", path: "/photos/NOE.jpg" },
    { student: "2", path: "C:\\photos\\lea_durand.JPG" },
    { student: "4", path: "/p/Anna-Lou 2.png" },
  ]);
  expect(r.unmatched).toEqual(["/p/léa.jpg", "/p/inconnu.jpg"]);
});

test("a page made for one child is known by that child's name, and found by it", () => {
  const kids = [s("1", "Léa"), s("2", "Noé"), s("3", "Anna", "Durand"), s("4", "Anna", "Martin")];
  const page = (...texts: string[]): Page => ({ id: "p", size: { width_mm: 210, height_mm: 297 }, elements: texts.map((t) => text(null, t)), grid: null });
  expect(childOfPage(page("J'écris mon prénom", "LÉA"), kids)).toBe("Léa");
  expect(childOfPage(page("Léa et Noé"), kids)).toBeUndefined();
  expect(childOfPage(page("Léandre"), kids)).toBeUndefined();
  // Deux Anna : l'initiale quand le nom est sur la page, le prénom seul sinon.
  expect(childOfPage(page("Anna", "Martin"), kids)).toBe("Anna M.");
  expect(childOfPage(page("Anna"), kids)).toBe("Anna");
  const pages = [page("Léa"), page("Anna", "Durand"), page("Noé"), page("Anna", "Martin")];
  expect(findChildPage(pages, kids, "noe", 0)).toBe(2);
  expect(findChildPage(pages, kids, "anna", 1)).toBe(3);
  expect(findChildPage(pages, kids, "anna", 3)).toBe(1);
  expect(findChildPage(pages, kids, "Anna M.", 0)).toBe(3);
  expect(findChildPage(pages, kids, "Zoé", 0)).toBeUndefined();
});

test("une page de la classe changée de place se retrouve à sa nouvelle place", async () => {
  const { movePage } = await import("./pageOrder");
  const kids = [s("1", "Léa"), s("2", "Noé")];
  const named = (id: string, who: string): Page => ({ id, size: { width_mm: 210, height_mm: 297 }, elements: [text(null, who)], grid: null });
  const doc = { title: "", pages: [named("a", "Léa"), named("b", "Noé"), named("c", "Consigne")], assets: {} } as unknown as import("./types").Document;
  const moved = movePage(doc, 1, 2).pages;
  expect(moved.map((p) => childOfPage(p, kids))).toEqual(["Léa", undefined, "Noé"]);
  expect(findChildPage(moved, kids, "noe", 0)).toBe(2);
});
