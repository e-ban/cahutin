import { expect, test } from "vitest";
import { textElement } from "./factory";
import { templateRole, withTemplateRole } from "./templateRole";
import type { Element } from "./types";

const titled = (text: string): Element => {
  const e = textElement({ x_mm: 0, y_mm: 0, w_mm: 80, h_mm: 20, rotation_deg: 0 });
  if (e.kind.type === "Text") e.kind = { ...e.kind, content: { paragraphs: [{ align: null, runs: [{ text, bold: false, italic: false, underline: false, color: null, font: null, size_pt: null }] }] } };
  return e;
};

test("a template text is to fill, fixed, or not kept", () => {
  const t = titled("Mon prénom");
  expect(templateRole(t)).toBe("none");
  const fill = withTemplateRole(t, "fill");
  expect(templateRole(fill)).toBe("fill");
  expect(fill.kind.type === "Text" && fill.kind.prompt).toBe("Mon prénom");
  const fixed = withTemplateRole(fill, "fixed");
  expect(templateRole(fixed)).toBe("fixed");
  expect(fixed.locked && !fixed.zone).toBe(true);
  // Une invite déjà choisie reste ; un texte vide prend l'invite « Texte ».
  expect(withTemplateRole(fixed, "fill").kind).toMatchObject({ prompt: "Mon prénom" });
  expect(withTemplateRole(titled(""), "fill").kind).toMatchObject({ prompt: "Texte" });
});
