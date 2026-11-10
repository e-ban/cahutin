import { describe, expect, it } from "vitest";
import { ICONS } from "./icons";

// Les emojis et les signes (flèches, formes) que Windows, macOS et Linux dessinent chacun à leur façon : l'interface
// passe par Icon. Seuls les points de la jauge de coût de l'assistant restent écrits.
const SIGNS = /[\u{1F000}-\u{1FAFF}\u{2190}-\u{21FF}\u{2300}-\u{23FF}\u{25A0}-\u{25FF}\u{2600}-\u{27BF}\u{2B00}-\u{2BFF}]/gu;
const ALLOWED = new Set(["●", "○"]);
const hasSign = (line: string) => (line.match(SIGNS) ?? []).some((c) => !ALLOWED.has(c));

// Le texte de chaque composant de l'interface, par chemin.
const COMPONENTS = import.meta.glob<string>("../components/**/*.svelte", { query: "?raw", import: "default", eager: true });

describe("icons", () => {
  it("gives every name a Lucide drawing", () => {
    for (const [name, glyph] of Object.entries(ICONS)) expect(glyph, name).toBeTruthy();
  });

  it("leaves no emoji in the interface", () => {
    const found = Object.entries(COMPONENTS).flatMap(([file, text]) =>
      text
        .split("\n")
        .map((line, i) => ({ line, i }))
        // Ce qui est imprimé sur la page (coches de fusion en CSS) n'est pas l'interface.
        .filter(({ line }) => hasSign(line) && !/^\s*content:/.test(line))
        .map(({ line, i }) => `${file}:${i + 1}: ${line.trim()}`),
    );
    expect(found).toEqual([]);
  });
});
