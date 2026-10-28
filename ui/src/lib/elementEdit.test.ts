import { expect, test } from "vitest";
import { textFrame } from "./elementEdit";
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
