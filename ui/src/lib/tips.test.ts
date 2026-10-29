import { expect, test } from "vitest";
import { TIP_THEMES, TIPS } from "./tips";

test("every theme has at least three tips", () => {
  expect(TIP_THEMES).toEqual(["Photos", "Texte", "Mise en page", "Pages et modèles", "Fichiers"]);
  for (const theme of TIP_THEMES) expect(TIPS.filter((t) => t.theme === theme).length).toBeGreaterThanOrEqual(3);
  expect(TIPS.length).toBeGreaterThanOrEqual(18);
});

test("tip ids are unique", () => {
  expect(new Set(TIPS.map((t) => t.id)).size).toBe(TIPS.length);
});

test("tips use no long dashes", () => {
  for (const t of TIPS) expect(t.title + t.text).not.toMatch(/[–—]/);
});
