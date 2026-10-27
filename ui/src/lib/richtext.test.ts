import type { JSONContent } from "@tiptap/core";
import { expect, test } from "vitest";
import { richToTiptap, tiptapToRich } from "./richtext";
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
