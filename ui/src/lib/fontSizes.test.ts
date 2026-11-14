import { expect, test } from "vitest";
import { FONT_SIZES, formatFontSize, MAX_FONT_PT, MIN_FONT_PT, parseFontSize } from "./fontSizes";

test("la liste va au-delà de 72 pt, pour un prénom géant ou une affiche", () => {
  expect(FONT_SIZES).toEqual(expect.arrayContaining([72, 96, 120, 144, 200]));
  expect(FONT_SIZES.every((s) => s >= MIN_FONT_PT && s <= MAX_FONT_PT)).toBe(true);
});

test("une taille tapée se lit, avec la virgule française ou « pt »", () => {
  expect(parseFontSize("13,5")).toBe(13.5);
  expect(parseFontSize("13.5")).toBe(13.5);
  expect(parseFontSize(" 250 pt ")).toBe(250);
  expect(parseFontSize("90pt")).toBe(90);
  expect(parseFontSize("12,345")).toBe(12.3);
});

test("une taille hors bornes est ramenée entre le minimum et le maximum", () => {
  expect(parseFontSize("1000")).toBe(MAX_FONT_PT);
  expect(parseFontSize("1")).toBe(MIN_FONT_PT);
  expect(parseFontSize("0")).toBe(MIN_FONT_PT);
});

test("une saisie qui n'est pas une taille est ignorée", () => {
  for (const s of ["", "  ", "abc", "12a", "-5", "1,2,3", "pt", "Infinity"]) expect(parseFontSize(s)).toBeNull();
});

test("la taille s'affiche avec la virgule française", () => {
  expect(formatFontSize(13.5)).toBe("13,5");
  expect(formatFontSize(250)).toBe("250");
});
