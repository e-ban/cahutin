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

test("a question can name what Escape answers, so Escape never destroys anything", async () => {
  const choices = [
    { value: "discard", label: "Non, l'oublier" },
    { value: "open", label: "Reprendre", primary: true },
  ];
  const p = modal.choose("Reprendre ce travail ?", choices, { escape: "later" });
  modal.cancel();
  expect(await p).toBe("later");
  // Remplacée par une autre question : même réponse que Échap.
  const q = modal.choose("Reprendre ce travail ?", choices, { escape: "later" });
  const r = modal.choose("Autre ?", choices);
  expect(await q).toBe("later");
  modal.cancel();
  expect(await r).toBe("discard");
});
