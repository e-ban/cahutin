import { expect, test } from "vitest";
import { CLASS_INFO_FIELDS, DATE_FIELDS, FIELDS, fieldLabel, fieldShown, isAutoField, isDateField, letterKey, monthKey, needsDate, splitFields } from "./classFields";
import type { Page } from "./types";

test("splitFields keeps unknown braces as text", () => {
  expect(splitFields("Bonjour {prenom}, {toto} !")).toEqual([{ text: "Bonjour " }, { field: "prenom" }, { text: ", {toto} !" }]);
  expect(splitFields("{prenom}{age}")).toEqual([{ field: "prenom" }, { field: "age" }]);
  expect(splitFields("{ {prenom}")).toEqual([{ text: "{ " }, { field: "prenom" }]);
  expect(splitFields("{prénom}")).toEqual([{ field: "prénom" }]);
  expect(splitFields("{prenom_0} {PRENOM_x}")).toEqual([{ text: "{prenom_0} {PRENOM_x}" }]);
});

test("labels, letters and months", () => {
  expect(FIELDS.map((f) => f.key)).toEqual(["prenom", "PRENOM", "nom", "groupe", "age", "anniversaire"]);
  expect(fieldLabel("prenom")).toBe("Prénom");
  expect(fieldLabel("prénom")).toBe("Prénom");
  expect(fieldLabel("PRENOM_3")).toBe("Lettre 3");
  expect(fieldLabel("prenom_3")).toBe("lettre 3");
  expect(fieldLabel("anniversaires_aout")).toBe("Anniversaires d'août");
  expect(fieldLabel("anniversaires_mars")).toBe("Anniversaires de mars");
  expect(fieldLabel("toto")).toBeNull();
  expect(letterKey(2, true)).toBe("PRENOM_2");
  expect(letterKey(2, false)).toBe("prenom_2");
  expect(monthKey(2)).toBe("anniversaires_fevrier");
  expect(monthKey(12)).toBe("anniversaires_decembre");
});

test("a letter shows its number only: its case is small", () => {
  expect(fieldShown("PRENOM_10")).toBe("10");
  expect(fieldShown("prenom_2")).toBe("2");
  expect(fieldShown("prenom")).toBe("Prénom");
});

const textPage = (text: string): Page => ({
  id: "p",
  size: { width_mm: 210, height_mm: 297 },
  grid: null,
  elements: [
    {
      id: "t",
      frame: { x_mm: 0, y_mm: 0, w_mm: 10, h_mm: 10, rotation_deg: 0 },
      locked: false,
      zone: false,
      kind: { type: "Text", content: { paragraphs: [{ align: null, runs: [{ text, bold: false, italic: false, underline: false, color: null, font: null, size_pt: null }] }] }, style: { font: "Andika", size_pt: 14, color: "#222222", align: "Left" }, prompt: null },
    },
  ],
});

test("date and class fields", () => {
  expect(DATE_FIELDS.map((f) => f.key)).toEqual(["date", "date_courte", "date_longue", "jour_mois", "jour_semaine", "mois", "annee", "annee_scolaire", "saison"]);
  expect(CLASS_INFO_FIELDS.map((f) => f.key)).toEqual(["classe", "niveau", "nombre_eleves", "ecole", "enseignant"]);
  expect(fieldLabel("annee_scolaire")).toBe("Année scolaire");
  expect(splitFields("{date} {toto}")).toEqual([{ field: "date" }, { text: " {toto}" }]);
  expect([isAutoField("saison"), isDateField("saison"), isAutoField("ecole"), isDateField("ecole"), isAutoField("prenom")]).toEqual([true, true, true, false, false]);
  expect(needsDate(textPage("Le {date}"))).toBe(true);
  expect(needsDate(textPage("{prenom} {ecole}"))).toBe(false);
});
