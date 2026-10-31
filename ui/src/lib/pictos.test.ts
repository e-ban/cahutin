import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { expect, test } from "vitest";
import { assignSymbols, CLASS_PICTO_GROUPS, STUDENT_SYMBOLS, toned, pageHasPictos, pictoElement, pictosOf, replacePictos, searchPictos, type PictoEntry } from "./pictos.svelte";
import type { Page } from "./types";

const index: PictoEntry[] = JSON.parse(readFileSync(fileURLToPath(new URL("../../../assets/openmoji/index.json", import.meta.url)), "utf8"));
const page = (): Page => ({ id: "p", size: { width_mm: 210, height_mm: 297 }, elements: [], grid: null });

test("searchPictos finds French words without accents", () => {
  expect(searchPictos(index, "renard")[0].code).toBe("1F98A");
  expect(searchPictos(index, "gateau").map((p) => p.code)).toContain("1F382");
  expect(searchPictos(index, "")).toBe(index);
});

test("searchPictos falls back to English tags", () => {
  expect(searchPictos(index, "fox").map((p) => p.code)).toContain("1F98A");
});

test("replacePictos changes every picture of a selected frieze", () => {
  const p = page();
  const a = { ...pictoElement(p, "2600"), group: "f" };
  const b = { ...pictoElement(p, "2600"), group: "f" };
  p.elements.push(a, b);
  const out = replacePictos(p, [a.id], "1F98A");
  expect(out.elements.map((e) => e.kind.type === "Image" && e.kind.code)).toEqual(["1F98A", "1F98A"]);
});

test("pageHasPictos and pictoElement defaults", () => {
  const p = page();
  expect(pageHasPictos(p)).toBe(false);
  const e = pictoElement(p, "1F98A");
  expect(e.frame).toMatchObject({ w_mm: 30, h_mm: 30, x_mm: 90 });
  p.elements.push(e);
  expect(pageHasPictos(p)).toBe(true);
});

test("class categories only name pictures of the bank, each once", () => {
  const index: PictoEntry[] = JSON.parse(readFileSync(new URL("../../../assets/openmoji/index.json", import.meta.url), "utf8"));
  for (const [g, codes] of Object.entries(CLASS_PICTO_GROUPS)) {
    expect(pictosOf(index, g).map((p) => p.code)).toEqual(codes);
    expect(new Set(codes).size).toBe(codes.length);
  }
});

test("each child without a symbol gets one of their own, taken from the bank", () => {
  const index: PictoEntry[] = JSON.parse(readFileSync(new URL("../../../assets/openmoji/index.json", import.meta.url), "utf8"));
  expect(STUDENT_SYMBOLS.filter((c) => !index.some((p) => p.code === c))).toEqual([]);
  expect(new Set(STUDENT_SYMBOLS).size).toBe(STUDENT_SYMBOLS.length);
  const kids = [{ symbol: null }, { symbol: "1F436" }, { symbol: null }];
  expect(assignSymbols(kids).map((k) => k.symbol)).toEqual(["1F431", "1F436", "1F42D"]);
});

test("a person takes the chosen skin tone, other pictures stay as they are", () => {
  const tones = JSON.parse(readFileSync(new URL("../../../assets/openmoji/tones.json", import.meta.url), "utf8"));
  expect(toned(tones, "1F9D2", 3)).toBe("1F9D2-1F3FD");
  expect(toned(tones, "1F9D2", 0)).toBe("1F9D2");
  expect(toned(tones, "1F98A", 4)).toBe("1F98A");
  expect(Object.values(tones).every((v) => (v as string[]).length === 5)).toBe(true);
});
