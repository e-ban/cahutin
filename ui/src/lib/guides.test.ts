import { existsSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { expect, test } from "vitest";
import { editor } from "./docStore.svelte";
import { GUIDES, guides } from "./guides.svelte";
import { templates } from "./templates.svelte";
import type { TemplateInfo } from "./types";

test("every guide panel has its animated capture in assets", () => {
  expect(new Set(GUIDES.map((g) => g.id)).size).toBe(GUIDES.length);
  for (const g of GUIDES)
    for (const p of g.panels) expect(existsSync(fileURLToPath(new URL(`../../../assets${p.image}`, import.meta.url))), p.image).toBe(true);
});

test("the walk moves on when a step is done, even the one after, and stops after the last", () => {
  editor.templateMode = false;
  guides.walk("modele");
  expect(guides.walking?.step).toBe(0);
  guides.advance();
  expect(guides.walking?.step).toBe(0);
  editor.templateMode = true;
  guides.advance();
  expect(guides.walking?.step).toBe(1);
  guides.next();
  expect(guides.walking?.step).toBe(2);
  // Enregistré sans passer par « Étape suivante » : l'étape 3 est sautée avec l'enregistrement.
  templates.infos = [...templates.infos, { id: "t", name: "Essai", builtin: false, for_class: false } as TemplateInfo];
  guides.advance();
  expect(guides.walking?.step).toBe(4);
  editor.templateMode = false;
  guides.advance();
  expect(guides.walking?.step).toBe(5);
  guides.next();
  expect(guides.walking).toBeNull();
});

/** L'assistant IA a son guide : l'activer, demander, accepter ou refuser, signaler une réponse. */
test("the assistant guide explains activation, what is sent, accepting and reporting", () => {
  const g = GUIDES.find((x) => x.id === "assistant");
  expect(g?.panels.map((p) => p.image)).toEqual(["activer", "demander", "accepter", "signaler"].map((n) => `/guides/assistant/${n}.webp`));
  const text = g!.panels.map((p) => p.text).join(" ");
  for (const word of ["Préférences", "Accepter", "Signaler", "prénom"]) expect(text).toContain(word);
});
