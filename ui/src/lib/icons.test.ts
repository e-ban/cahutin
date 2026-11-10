import { describe, expect, it } from "vitest";
import { ICONS } from "./icons";

// Les emojis et les signes que Windows, macOS et Linux dessinent chacun à leur façon : l'interface passe par Icon.
const EMOJI = /[\u{1F000}-\u{1FAFF}\u{2600}-\u{27BF}\u{2B00}-\u{2BFF}]\u{FE0F}?/u;

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
        .filter(({ line }) => EMOJI.test(line) && !/^\s*content:/.test(line))
        .map(({ line, i }) => `${file}:${i + 1}: ${line.trim()}`),
    );
    expect(found).toEqual([]);
  });
});
