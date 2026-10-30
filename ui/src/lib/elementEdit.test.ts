import { expect, test } from "vitest";
import { textFrame, textStyle } from "./elementEdit";
import type { Element } from "./types";

const text: Element = {
  id: "t",
  frame: { x_mm: 0, y_mm: 0, w_mm: 50, h_mm: 20, rotation_deg: 0 },
  locked: false,
  zone: false,
  kind: { type: "Text", content: { paragraphs: [] }, style: { font: "Andika", size_pt: 14, color: "#222222", align: "Left" }, prompt: null },
};

test("a text frame is created by its first setting and removed when nothing is left", () => {
  const filled = textFrame({ fill: "#ffeeaa" })(text);
  expect(filled.kind.type === "Text" && filled.kind.frame).toEqual({ fill: "#ffeeaa", stroke: null, corner_radius_mm: 0, image: null });
  const rounded = textFrame({ corner_radius_mm: 5 })(filled);
  const cleared = textFrame({ fill: null })(rounded);
  expect(cleared.kind.type === "Text" && cleared.kind.frame).toBeNull();
  // Des coins seuls ne font pas un cadre.
  expect(textFrame({ corner_radius_mm: 5 })(text).kind.type === "Text" && textFrame({ corner_radius_mm: 5 })(text).kind).toMatchObject({ frame: null });
});

test("the default margin and top alignment are stored as absent", () => {
  const back = textStyle({ padding_mm: 1.5, valign: "Top" })(textStyle({ padding_mm: 4, valign: "Middle" })(text));
  expect(back.kind.type === "Text" && [back.kind.style.padding_mm, back.kind.style.valign]).toEqual([null, null]);
});

test("hollow letters off are stored as absent, so the format does not change", () => {
  const on = textStyle({ outline: true })(text);
  expect(on.kind.type === "Text" && on.kind.style.outline).toBe(true);
  const off = textStyle({ outline: false })(on);
  expect(off.kind.type === "Text" && off.kind.style.outline).toBeNull();
});
