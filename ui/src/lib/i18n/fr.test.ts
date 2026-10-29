import { expect, test } from "vitest";
import { fr } from "./fr";

/** Toutes les chaînes de `fr`, fonctions comprises (appelées avec des valeurs d'exemple). */
function texts(value: unknown): string[] {
  if (typeof value === "string") return [value];
  if (typeof value === "function") return texts(value("x", "x", "x"));
  if (value && typeof value === "object") return Object.values(value).flatMap(texts);
  return [];
}

test("the app is called Cahutin", () => {
  expect(fr.appName).toBe("Cahutin");
  expect(texts(fr).filter((t) => t.includes("Carnet"))).toEqual([]);
});
