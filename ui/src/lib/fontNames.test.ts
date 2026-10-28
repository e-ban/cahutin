import { afterEach, expect, test } from "vitest";
import { EMBEDDED_FAMILIES, missingFamilies, setKnownFamilies } from "./fontNames";
import { newDocument, textElement } from "./factory";

afterEach(() => setKnownFamilies(EMBEDDED_FAMILIES));

test("missingFamilies lists the fonts a document uses that this computer lacks", () => {
  const doc = newDocument();
  const a = textElement({ x_mm: 0, y_mm: 0, w_mm: 10, h_mm: 10, rotation_deg: 0 });
  if (a.kind.type === "Text") {
    a.kind.style.font = "Police Collègue";
    a.kind.content = { paragraphs: [{ align: null, runs: [{ text: "x", bold: false, italic: false, underline: false, color: null, font: "Autre Police", size_pt: null }] }] };
  }
  doc.pages[0].elements.push(a);
  setKnownFamilies([...EMBEDDED_FAMILIES, "Autre Police"]);
  expect(missingFamilies(doc)).toEqual(["Police Collègue"]);
});
