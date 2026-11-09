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

test("cancel answers the first choice, the way back", async () => {
  const p = modal.choose("Enregistrer ?", [
    { value: "cancel", label: "Revenir au document" },
    { value: "save", label: "Enregistrer", primary: true },
  ]);
  modal.cancel();
  expect(await p).toBe("cancel");
  expect(modal.message).toBeNull();
});
