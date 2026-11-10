import { describe, expect, it } from "vitest";
import { HIGH_CONTRAST, THEMES, THEME_IDS, cssVar, themeVars, type ThemeColors } from "./themes";

/** Contraste WCAG entre deux couleurs #rrggbb. */
function contrast(a: string, b: string): number {
  const lum = (hex: string) => {
    const [r, g, bl] = [1, 3, 5].map((i) => parseInt(hex.slice(i, i + 2), 16) / 255).map((c) => (c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4));
    return 0.2126 * r + 0.7152 * g + 0.0722 * bl;
  };
  const [hi, lo] = [lum(a), lum(b)].sort((x, y) => y - x);
  return (hi + 0.05) / (lo + 0.05);
}

// Le texte de chaque composant de l'interface, par chemin.
const COMPONENTS = import.meta.glob<string>("../components/**/*.svelte", { query: "?raw", import: "default", eager: true });
// Les anciennes teintes de l'interface, écrites en dur : elles ne suivent pas le thème (PageView les remet exprès
// sur la page blanche).
const OLD_TINTS = /#e8f1fb|#eef5fc|#eaf2fb|#f2f8fe|#fdecea|#fff4e5|#f0b86e|rgba\(40, 30, 20/i;

const all: [string, ThemeColors][] = [...THEME_IDS.map((id): [string, ThemeColors] => [id, THEMES[id].colors]), ["contraste", HIGH_CONTRAST]];

describe("themes", () => {
  it("defines every colour in every theme", () => {
    const keys = Object.keys(THEMES.papier.colors).sort();
    for (const [name, colors] of all) expect(Object.keys(colors).sort(), name).toEqual(keys);
  });

  it("keeps texts readable on bars, panels and controls", () => {
    for (const [name, c] of all) {
      for (const ground of [c.bg, c.panel, c.surface]) {
        expect(contrast(c.text, ground), `${name} : texte`).toBeGreaterThanOrEqual(7);
        expect(contrast(c.muted, ground), `${name} : texte discret`).toBeGreaterThanOrEqual(3.5);
      }
      expect(contrast(c.onAccent, c.accent), `${name} : bouton principal`).toBeGreaterThanOrEqual(3.5);
      expect(contrast(c.text, c.accentSoft), `${name} : survol`).toBeGreaterThanOrEqual(7);
      expect(contrast(c.danger, c.surface), `${name} : danger`).toBeGreaterThanOrEqual(4.5);
      expect(contrast(c.warnText, c.warnBg), `${name} : avertissement`).toBeGreaterThanOrEqual(4.5);
      expect(contrast(c.warnText, c.panel), `${name} : avertissement sur un panneau`).toBeGreaterThanOrEqual(4.5);
    }
  });

  it("lets high contrast win over a dark theme", () => {
    expect(themeVars("nuit", true).dark).toBe(false);
    expect(themeVars("nuit", false).dark).toBe(true);
    expect(new Map(themeVars("nuit", true).vars).get("--text")).toBe("#000000");
  });

  it("names the CSS variables", () => {
    expect(cssVar("accentSoft")).toBe("--accent-soft");
    expect(cssVar("bg")).toBe("--bg");
  });

  it("leaves no old fixed tint in the interface", () => {
    const found = Object.entries(COMPONENTS)
      .filter(([file]) => !file.endsWith("/PageView.svelte"))
      .flatMap(([file, text]) => text.split("\n").flatMap((line, i) => (OLD_TINTS.test(line) ? [`${file}:${i + 1}: ${line.trim()}`] : [])));
    expect(found).toEqual([]);
  });
});
