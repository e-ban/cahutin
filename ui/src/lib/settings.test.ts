import { expect, test } from "vitest";
import { settings } from "./settings.svelte";

/** Barre courte par défaut : les outils moins utilisés sont rangés dans « Plus ». */
test("the More menu is on by default", () => {
  expect(settings.moreMenu).toBe(true);
});
