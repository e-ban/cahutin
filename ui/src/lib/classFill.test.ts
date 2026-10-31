import { expect, test } from "vitest";
import { hasNameFields, matchPhotos, nameFieldsFilled } from "./classFill";
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

test("hasNameFields: a Prénom zone or a {prénom} word, not a photo zone or a title", () => {
  expect(hasNameFields(page([text("Prénom", "")]))).toBe(true);
  expect(hasNameFields(page([text(null, "Bravo {prénom} !")]))).toBe(true);
  expect(hasNameFields(page([text("Titre", "")]))).toBe(false);
  expect(hasNameFields(page([text(null, "Bonjour")]))).toBe(false);
});

test("nameFieldsFilled: a Prénom zone already written", () => {
  expect(nameFieldsFilled(page([text("Prénom", "")]))).toBe(false);
  expect(nameFieldsFilled(page([text("Prénom", "Léa")]))).toBe(true);
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
