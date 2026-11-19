import { expect, test } from "vitest";
import { startsWriting, textToWrite } from "./typeToWrite";
import type { Element, Page } from "./types";

const text = (id: string, extra: Partial<Element> = {}): Element => ({
  id,
  frame: { x_mm: 10, y_mm: 10, w_mm: 40, h_mm: 20, rotation_deg: 0 },
  locked: false,
  zone: false,
  kind: {
    type: "Text",
    content: { paragraphs: [] },
    style: { font: "Andika", size_pt: 16, color: "#222222", align: "Left" },
    prompt: null,
  },
  ...extra,
});

const shape = (id: string, extra: Partial<Element> = {}): Element => ({
  id,
  frame: { x_mm: 0, y_mm: 10, w_mm: 10, h_mm: 10, rotation_deg: 0 },
  locked: false,
  zone: false,
  kind: { type: "Shape", shape: "Rect", fill: "#ffffff", stroke: { width_mm: 1, color: "#000000" }, corner_radius_mm: 2 },
  ...extra,
});

const page = (elements: Element[]): Page => ({ id: "p", size: { width_mm: 210, height_mm: 297 }, elements, grid: null });

test("a single selected text is the one to write in", () => {
  expect(textToWrite(page([text("t")]), ["t"])).toBe("t");
});

test("a locked text, a shape or several free elements give nothing", () => {
  expect(textToWrite(page([text("t", { locked: true })]), ["t"])).toBeNull();
  expect(textToWrite(page([shape("s")]), ["s"])).toBeNull();
  expect(textToWrite(page([text("a"), text("b")]), ["a", "b"])).toBeNull();
  expect(textToWrite(page([text("t")]), [])).toBeNull();
});

test("a group holding one text (a speech bubble) writes in that text", () => {
  const p = page([shape("bubble", { group: "g" }), text("t", { group: "g" })]);
  expect(textToWrite(p, ["bubble", "t"])).toBe("t");
});

test("a group holding two texts gives nothing: which one would be ambiguous", () => {
  const p = page([text("a", { group: "g" }), text("b", { group: "g" })]);
  expect(textToWrite(p, ["a", "b"])).toBeNull();
});

const key = (k: string, mods: Partial<KeyboardEvent> = {}) => ({ key: k, ctrlKey: false, metaKey: false, altKey: false, ...mods });

test("a letter, a digit, a space or Enter starts writing; shortcuts and other keys do not", () => {
  expect(startsWriting(key("a"))).toEqual({ insert: "a" });
  expect(startsWriting(key("É"))).toEqual({ insert: "É" });
  expect(startsWriting(key("7"))).toEqual({ insert: "7" });
  expect(startsWriting(key(" "))).toEqual({ insert: " " });
  expect(startsWriting(key("Enter"))).toEqual({ insert: "" });
  expect(startsWriting(key("F2"))).toEqual({ insert: "" });
  expect(startsWriting(key("a", { ctrlKey: true }))).toBeNull();
  expect(startsWriting(key("a", { metaKey: true }))).toBeNull();
  expect(startsWriting(key("Escape"))).toBeNull();
  expect(startsWriting(key("ArrowLeft"))).toBeNull();
  expect(startsWriting(key("Delete"))).toBeNull();
  expect(startsWriting(key("Enter", { ctrlKey: true }))).toBeNull();
});

test("AltGr characters (Ctrl+Alt on Windows) still start writing", () => {
  expect(startsWriting(key("€", { ctrlKey: true, altKey: true }))).toEqual({ insert: "€" });
});
