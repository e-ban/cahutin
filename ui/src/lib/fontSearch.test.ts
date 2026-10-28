import { expect, test } from "vitest";
import { groupFamilies, searchFamilies } from "./fontSearch";
import type { FontFamilyInfo } from "./types";

const fam = (name: string, kind: "builtin" | "user" | "system"): FontFamilyInfo => ({
  name,
  has_bold: false,
  has_italic: false,
  user: kind === "user",
  system: kind === "system",
  embeddable: true,
  kind: kind === "builtin" ? "scripte" : null,
});

const all = [fam("Andika", "builtin"), fam("Belle Allure", "user"), fam("Arial", "system"), fam("Écolier", "system"), fam("Comic Sans MS", "system")];

test("searchFamilies ignores case and accents and matches inside names", () => {
  expect(searchFamilies(all, "ecol").map((f) => f.name)).toEqual(["Écolier"]);
  expect(searchFamilies(all, "  SANS ").map((f) => f.name)).toEqual(["Comic Sans MS"]);
  expect(searchFamilies(all, "a").length).toBe(4);
  expect(searchFamilies(all, "")).toBe(all);
});

test("groupFamilies splits provided, added and computer fonts", () => {
  const g = groupFamilies(all);
  expect(g.builtin.map((f) => f.name)).toEqual(["Andika"]);
  expect(g.user.map((f) => f.name)).toEqual(["Belle Allure"]);
  expect(g.system.map((f) => f.name)).toEqual(["Arial", "Écolier", "Comic Sans MS"]);
});
