import { expect, test } from "vitest";
import { FALLBACK_METRICS, LINE_SPACINGS, lineBox } from "./lineSpacing";
import type { FontMetrics } from "./types";

const METRICS: Record<string, FontMetrics> = {
  Andika: { units_per_em: 2048, ascent: 2500, descent: 800 },
  Playwrite: { units_per_em: 1000, ascent: 1550, descent: 607 },
};
const MM_PER_PT = 25.4 / 72;

// Campagne de mesure (LibreOffice 26.8 sans écran, WebKitGTK), trois lignes, en mm depuis le haut de la zone de
// texte (marge ôtée) : lignes de base de LibreOffice à N %, et première ligne de base de WebKit avec le facteur
// N × hauteur naturelle exacte / taille, sans décalage.
const MEASURED: { font: string; pt: number; n: number; lo: number[]; web: number }[] = [
  { font: "Andika", pt: 14, n: 0.8, lo: [5.12, 11.52, 17.92], web: 5.292 },
  { font: "Andika", pt: 28, n: 0.8, lo: [10.16, 22.86, 35.56], web: 10.46 },
  { font: "Playwrite", pt: 14, n: 0.8, lo: [6.83, 15.37, 23.91], web: 6.636 },
  { font: "Playwrite", pt: 28, n: 0.8, lo: [13.6, 30.6, 47.6], web: 13.147 },
  { font: "Andika", pt: 14, n: 1.5, lo: [10.05, 22.05, 34.05], web: 8.075 },
  { font: "Andika", pt: 28, n: 1.5, lo: [19.96, 43.78, 67.6], web: 16.033 },
  { font: "Playwrite", pt: 14, n: 1.5, lo: [12.99, 28.99, 44.99], web: 10.365 },
  { font: "Playwrite", pt: 28, n: 1.5, lo: [25.9, 57.77, 89.64], web: 20.605 },
  { font: "Andika", pt: 14, n: 2, lo: [14.05, 30.05, 46.05], web: 10.067 },
  { font: "Andika", pt: 28, n: 2, lo: [27.9, 59.66, 91.42], web: 20.01 },
  { font: "Playwrite", pt: 14, n: 2, lo: [18.33, 39.67, 61.01], web: 13.023 },
  { font: "Playwrite", pt: 28, n: 2, lo: [36.53, 79.03, 121.53], web: 25.93 },
];

test.each(MEASURED)("$font $pt pt à $n : le pas et les lignes de base de LibreOffice", ({ font, pt, n, lo, web }) => {
  const m = METRICS[font];
  const { lineHeight, shiftMm } = lineBox(n, m, pt);
  const pitch = lineHeight! * pt * MM_PER_PT;
  expect(Math.abs(pitch - (lo[2] - lo[0]) / 2)).toBeLessThan(0.011);
  // La ligne de base de WebKit suit la moitié de l'écart de hauteur de ligne avec le cas mesuré.
  const exact = (n * (m.ascent + m.descent) * pt * MM_PER_PT) / m.units_per_em;
  const first = web + (pitch - exact) / 2 + shiftMm;
  expect(Math.abs(first - lo[0])).toBeLessThan(0.02);
});

test("sans réglage, rien ne change : interligne de la police, aucun décalage", () => {
  expect(lineBox(null, METRICS.Andika, 14)).toEqual({ lineHeight: null, shiftMm: 0 });
  expect(lineBox(undefined, METRICS.Playwrite, 28)).toEqual({ lineHeight: null, shiftMm: 0 });
});

test("plus d'interligne : l'espace ajouté descend le texte de la moitié d'une ligne par demi-ligne", () => {
  const n = 8.001; // hauteur naturelle d'Andika 14 pt à 600 ppp, en mm
  expect(Math.abs(lineBox(1.5, METRICS.Andika, 14).shiftMm - (0.5 * n) / 2)).toBeLessThan(0.15);
  expect(lineBox(2, METRICS.Andika, 14).shiftMm).toBeGreaterThan(lineBox(1.5, METRICS.Andika, 14).shiftMm);
  expect(Math.abs(lineBox(0.85, METRICS.Andika, 14).shiftMm)).toBeLessThan(0.5);
});

test("police aux hauteurs inconnues : celles d'Andika, la police de secours, jamais d'erreur", () => {
  expect(lineBox(1.5, null, 14)).toEqual(lineBox(1.5, FALLBACK_METRICS, 14));
  expect(lineBox(1.5, { units_per_em: 0, ascent: 0, descent: 0 }, 14)).toEqual(lineBox(1.5, FALLBACK_METRICS, 14));
  expect(lineBox(1.5, METRICS.Andika, 0)).toEqual({ lineHeight: null, shiftMm: 0 });
});

test("quatre choix, dont « normal » qui n'écrit rien", () => {
  expect(LINE_SPACINGS.map((s) => s.value)).toEqual([0.85, null, 1.5, 2]);
});
