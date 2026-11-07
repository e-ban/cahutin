import { expect, test } from "vitest";
import { addField, appendText, fieldsOf, fieldText, removeField, textWith } from "./fieldEdit";
import { hasTemplateRole } from "./templateRole";
import type { Page, RichText } from "./types";

const run = (text: string) => ({ text, bold: false, italic: false, underline: false, color: null, font: null, size_pt: null });
const rich = (...texts: string[]): RichText => ({ paragraphs: texts.map((t) => ({ align: null, runs: [run(t)] })) });
const words = (r: RichText) => r.paragraphs.map((p) => p.runs.map((x) => x.text).join("")).join("\n");
const page: Page = { id: "p", size: { width_mm: 210, height_mm: 297 }, grid: null, elements: [] };

test("fieldText is a plain text holding the field, never a zone", () => {
  const e = fieldText(page, "age");
  expect(e.kind.type === "Text" && words(e.kind.content)).toBe("{age}");
  expect([e.zone, e.locked]).toEqual([false, false]);
  expect(hasTemplateRole(e)).toBe(false);
});

test("addField always adds, at the end, after a space", () => {
  expect(words(addField(rich("Bonjour"), "prenom"))).toBe("Bonjour {prenom}");
  expect(words(addField(rich("{prenom}"), "nom"))).toBe("{prenom} {nom}");
  expect(words(addField(rich(), "prenom"))).toBe("{prenom}");
  expect(words(addField(rich("Ligne 1", "Ligne 2"), "age"))).toBe("Ligne 1\nLigne 2 {age}");
});

test("removeField takes every copy of the field out, with one space next to it", () => {
  expect(words(removeField(rich("{prenom} {nom}"), "prenom"))).toBe("{nom}");
  expect(words(removeField(rich("Bonjour {prenom} !", "{prenom}"), "prenom"))).toBe("Bonjour !\n");
  expect(words(removeField(rich("{prenom} {age} ans"), "age"))).toBe("{prenom} ans");
  expect(words(removeField(rich("{PRENOM}"), "prenom"))).toBe("{PRENOM}");
});

test("fieldsOf lists the known fields only", () => {
  expect(fieldsOf(rich("{prenom} et {toto}", "{PRENOM_2}"))).toEqual(["prenom", "PRENOM_2"]);
});

test("a date written into a text goes at its end, after a space; a new text can hold it", () => {
  expect(words(appendText(rich("Le"), "9 novembre 2026"))).toBe("Le 9 novembre 2026");
  expect(words(appendText(rich(), "2026-2027"))).toBe("2026-2027");
  const e = textWith(page, "automne");
  expect(e.kind.type === "Text" && words(e.kind.content)).toBe("automne");
});
