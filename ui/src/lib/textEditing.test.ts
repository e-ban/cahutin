import { expect, test } from "vitest";
import { keepsEditing, takeClick, textEditing } from "./textEditing.svelte";

/** Un élément qui se trouve dans le conteneur `inside` (une classe CSS). */
const inside = (cls: string) => ({ closest: (sel: string) => (sel.split(",").some((s) => s.trim() === `.${cls}`) ? {} : null) });

test("a click in the right panel, the floating bar or the field list keeps the text open", () => {
  expect(keepsEditing(inside("right"))).toBe(true);
  expect(keepsEditing(inside("ctx"))).toBe(true);
  // La liste des champs du tiroir Champs : le champ s'insère au curseur.
  expect(keepsEditing(inside("fields"))).toBe(true);
  expect(keepsEditing(inside("page"))).toBe(false);
  expect(keepsEditing(null)).toBe(false);
});

// Le double-clic qui ouvre un texte y place le curseur : son point ne sert qu'une fois, et qu'à ce texte.
test("the double-click point is taken once, by the text it opened", () => {
  textEditing.click = { id: "a", x: 10, y: 20 };
  expect(takeClick("b")).toBeNull();
  textEditing.click = { id: "a", x: 10, y: 20 };
  expect(takeClick("a")).toEqual({ x: 10, y: 20 });
  expect(takeClick("a")).toBeNull();
});
