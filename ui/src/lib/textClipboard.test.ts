import { expect, test } from "vitest";
import { copiedSlice, rememberCopy, withZoneStyle } from "./textClipboard";
import type { TextStyle } from "./types";

const zone = { font: "Andika", size_pt: 18, color: "#333333", align: "Left" } as TextStyle;

test("withZoneStyle writes the zone's font, size and colour on every piece of text, keeping its own", () => {
  const slice = {
    content: [
      {
        type: "paragraph",
        content: [
          { type: "text", text: "Bonjour " },
          { type: "text", text: "Léa", marks: [{ type: "bold" }, { type: "textStyle", attrs: { color: "#ff0000", fontFamily: null, fontSize: "24pt" } }] },
        ],
      },
    ],
    openStart: 1,
    openEnd: 1,
  };
  const out = withZoneStyle(slice, zone);
  const [plain, named] = out.content[0].content!;
  expect(plain.marks).toEqual([{ type: "textStyle", attrs: { color: "#333333", fontFamily: "Andika", fontSize: "18pt" } }]);
  expect(named.marks).toEqual([{ type: "bold" }, { type: "textStyle", attrs: { color: "#ff0000", fontFamily: "Andika", fontSize: "24pt" } }]);
  expect(out.openStart).toBe(1);
});

test("only the text copied last in Cahutin comes back with its formatting", () => {
  rememberCopy("Bonjour Léa", { content: [], openStart: 0, openEnd: 0 });
  expect(copiedSlice("Bonjour Léa")).not.toBeNull();
  // Copié ailleurs depuis : texte brut.
  expect(copiedSlice("autre chose")).toBeNull();
  // Les fins de ligne de Windows ne comptent pas.
  rememberCopy("a\nb", { content: [], openStart: 0, openEnd: 0 });
  expect(copiedSlice("a\r\nb")).not.toBeNull();
});
