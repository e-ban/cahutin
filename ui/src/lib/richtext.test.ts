import type { JSONContent } from "@tiptap/core";
import { expect, test } from "vitest";
import { setKnownFamilies } from "./fontNames";
import { richToTiptap, sanitizePlainText, textPadding, tiptapToRich } from "./richtext";
import type { RichText, TextStyle } from "./types";

const style: TextStyle = { font: "Andika", size_pt: 14, color: "#222222", align: "Left" };

const plainRun = { bold: false, italic: false, underline: false, color: null, font: null, size_pt: null };

test("rich text roundtrip", () => {
  const rt: RichText = {
    paragraphs: [
      {
        align: "Center",
        runs: [
          { ...plainRun, text: "Bonjour " },
          { ...plainRun, text: "la classe", bold: true, color: "#cc0000", font: "Playwrite FR Moderne", size_pt: 20 },
        ],
      },
      { align: null, runs: [] },
      { align: "Right", runs: [{ ...plainRun, text: "fin", italic: true, underline: true }] },
    ],
  };
  expect(tiptapToRich(richToTiptap(rt), style)).toEqual(rt);
});

test("defaults are elided", () => {
  const doc: JSONContent = {
    type: "doc",
    content: [
      {
        type: "paragraph",
        attrs: { textAlign: "left" },
        content: [{ type: "text", text: "Titre", marks: [{ type: "textStyle", attrs: { fontFamily: "Andika", fontSize: "14pt", color: "#222222" } }] }],
      },
    ],
  };
  expect(tiptapToRich(doc, style)).toEqual({ paragraphs: [{ align: null, runs: [{ ...plainRun, text: "Titre" }] }] });
});

test("adjacent runs with the same formatting are merged", () => {
  const doc: JSONContent = {
    type: "doc",
    content: [{ type: "paragraph", content: [{ type: "text", text: "a" }, { type: "text", text: "b", marks: [{ type: "link", attrs: { href: "x" } }] }] }],
  };
  expect(tiptapToRich(doc, style).paragraphs[0].runs).toEqual([{ ...plainRun, text: "ab" }]);
});

test("drops_unsupported_nodes_and_marks", () => {
  const doc: JSONContent = {
    type: "doc",
    content: [
      { type: "heading", attrs: { level: 1 }, content: [{ type: "text", text: "Grand titre" }] },
      {
        type: "bulletList",
        content: [
          { type: "listItem", content: [{ type: "paragraph", content: [{ type: "text", text: "un" }] }] },
          { type: "listItem", content: [{ type: "paragraph", content: [{ type: "text", text: "deux", marks: [{ type: "highlight" }, { type: "bold" }] }] }] },
        ],
      },
      { type: "paragraph", content: [{ type: "text", text: "ligne" }, { type: "hardBreak" }, { type: "text", text: "suivante" }] },
    ],
  };
  const rt = tiptapToRich(doc, style);
  expect(rt.paragraphs.map((p) => p.runs.map((r) => r.text).join(""))).toEqual(["Grand titre", "un", "deux", "ligne\nsuivante"]);
  expect(rt.paragraphs[2].runs[0]).toEqual({ ...plainRun, text: "deux", bold: true });
});

test("sanitizePlainText turns Word manual breaks into new lines and drops control characters", () => {
  expect(sanitizePlainText("a\u000bb\fc\u0001d\r\ne\rf\tg")).toBe("a\nb\ncd\ne\nf\tg");
});

test("only embedded fonts and #rrggbb colours survive", () => {
  const doc: JSONContent = {
    type: "doc",
    content: [
      {
        type: "paragraph",
        content: [
          { type: "text", text: "Word", marks: [{ type: "textStyle", attrs: { fontFamily: "Calibri, sans-serif", color: "rgb(255, 0, 0)", fontSize: "11pt" } }] },
          { type: "text", text: "ok", marks: [{ type: "textStyle", attrs: { fontFamily: "OpenDyslexic", color: "#C0392B" } }] },
        ],
      },
    ],
  };
  const runs = tiptapToRich(doc, style).paragraphs[0].runs;
  expect(runs[0]).toEqual({ ...plainRun, text: "Word", size_pt: 11 });
  expect(runs[1]).toEqual({ ...plainRun, text: "ok", font: "OpenDyslexic", color: "#C0392B" });
});

test("a user font added at runtime survives tiptapToRich", () => {
  setKnownFamilies(["Andika", "Carnet Test"]);
  const doc = (fontFamily: string): JSONContent => ({
    type: "doc",
    content: [{ type: "paragraph", content: [{ type: "text", text: "Bonjour", marks: [{ type: "textStyle", attrs: { fontFamily } }] }] }],
  });
  expect(tiptapToRich(doc("Carnet Test"), style).paragraphs[0].runs[0].font).toBe("Carnet Test");
  expect(tiptapToRich(doc("Comic Sans MS"), style).paragraphs[0].runs[0].font).toBeNull();
});

test("text padding defaults to the engine's 1.5 mm and follows the setting", () => {
  const style = { font: "Andika", size_pt: 14, color: "#222222", align: "Left" as const };
  expect(textPadding(style)).toBe(1.5);
  expect(textPadding({ ...style, padding_mm: null })).toBe(1.5);
  expect(textPadding({ ...style, padding_mm: 0 })).toBe(0);
  expect(textPadding({ ...style, padding_mm: 6 })).toBe(6);
});
