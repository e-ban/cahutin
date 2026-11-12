import { expect, test } from "vitest";
import { pick, toggleAll } from "./selection";

const order = ["a", "b", "c", "d", "e"];
const none = { ctrl: false, shift: false };
const ctrl = { ctrl: true, shift: false };
const shift = { ctrl: false, shift: true };

test("un clic choisit cette image seule", () => {
  expect(pick(["a", "b"], order, "c", none, "a")).toEqual(["c"]);
});

test("un second clic sur la seule image choisie la retire", () => {
  expect(pick(["c"], order, "c", none, "c")).toEqual([]);
});

test("Ctrl ajoute puis retire", () => {
  expect(pick(["a"], order, "c", ctrl, "a")).toEqual(["a", "c"]);
  expect(pick(["a", "c"], order, "a", ctrl, "c")).toEqual(["c"]);
});

test("Maj choisit tout entre la dernière image cliquée et celle-ci, dans les deux sens", () => {
  expect(pick(["b"], order, "d", shift, "b")).toEqual(["b", "c", "d"]);
  expect(pick(["d"], order, "b", shift, "d")).toEqual(["d", "b", "c"]);
});

test("Maj sans image cliquée avant : comme un clic", () => {
  expect(pick([], order, "c", shift, null)).toEqual(["c"]);
});

test("toggleAll ajoute un groupe, puis le retire s'il y est déjà tout entier", () => {
  expect(toggleAll(["a"], ["b", "c"])).toEqual(["a", "b", "c"]);
  expect(toggleAll(["a", "b", "c"], ["b", "c"])).toEqual(["a"]);
  expect(toggleAll(["b"], ["b", "c"])).toEqual(["b", "c"]);
});
