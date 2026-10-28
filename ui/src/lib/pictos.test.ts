import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { expect, test } from "vitest";
import { pageHasPictos, pictoElement, replacePictos, searchPictos, type PictoEntry } from "./pictos.svelte";
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
