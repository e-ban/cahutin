import { expect, test } from "vitest";
import { coversWholeText, lookOnWholeText } from "./textLook";
import type { TextProps } from "./types";

const run = (text: string, own: { font?: string; size_pt?: number; color?: string } = {}) => ({
  text,
  bold: true,
  italic: false,
  underline: false,
  color: own.color ?? null,
  font: own.font ?? null,
  size_pt: own.size_pt ?? null,
});
const props = (): TextProps => ({
  content: { paragraphs: [{ align: "Right", runs: [run("Bonjour "), run("Léa", { font: "Arial", size_pt: 30, color: "#ff0000" })] }] },
  style: { font: "Andika", size_pt: 14, color: "#000000", align: "Left", named: "texte" },
  prompt: null,
});

test("une taille donnée au bloc entier s'applique aussi aux mots qui avaient la leur", () => {
  const t = lookOnWholeText(props(), { size_pt: 96 });
  expect(t.style.size_pt).toBe(96);
  expect(t.content.paragraphs[0].runs.map((r) => r.size_pt)).toEqual([null, null]);
  // Le reste de la mise en forme des mots ne bouge pas.
  expect(t.content.paragraphs[0].runs[1]).toMatchObject({ font: "Arial", color: "#ff0000", bold: true, text: "Léa" });
  expect(t.content.paragraphs[0].align).toBe("Right");
  expect(t.style.named).toBe("texte");
});

test("une police ou une couleur donnée au bloc entier s'applique à tous les mots", () => {
  const f = lookOnWholeText(props(), { font: "OpenDyslexic" });
  expect(f.style.font).toBe("OpenDyslexic");
  expect(f.content.paragraphs[0].runs[1]).toMatchObject({ font: null, size_pt: 30, color: "#ff0000" });
  const c = lookOnWholeText(props(), { color: "#1f2c4a" });
  expect(c.style.color).toBe("#1f2c4a");
  expect(c.content.paragraphs[0].runs[1]).toMatchObject({ font: "Arial", size_pt: 30, color: null });
});

test("tout le texte est sélectionné quand la sélection va du début à la fin", () => {
  // Un paragraphe « abc » : le texte va de 1 à 4, le document fait 5.
  expect(coversWholeText(1, 4, 5)).toBe(true);
  expect(coversWholeText(0, 5, 5)).toBe(true);
  expect(coversWholeText(2, 4, 5)).toBe(false);
  expect(coversWholeText(1, 3, 5)).toBe(false);
  // Un simple curseur ne sélectionne rien.
  expect(coversWholeText(1, 1, 2)).toBe(false);
});
