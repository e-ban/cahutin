import { expect, test } from "vitest";
import { FIELDS, fieldLabel, letterKey, monthKey, splitFields } from "./classFields";

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
