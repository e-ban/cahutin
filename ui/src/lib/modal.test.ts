import { expect, test } from "vitest";
import { modal } from "./modal.svelte";

test("a second question settles the first one with its first choice", async () => {
  const choices = [
    { value: "cancel", label: "Annuler" },
    { value: "ok", label: "OK" },
  ];
  const first = modal.choose("Première ?", choices);
  const second = modal.choose("Seconde ?", choices);
  expect(await first).toBe("cancel");
  modal.answer("ok");
  expect(await second).toBe("ok");
  expect(modal.message).toBeNull();
});
