import { getSchema } from "@tiptap/core";
import Bold from "@tiptap/extension-bold";
import { EditorState, TextSelection } from "@tiptap/pm/state";
import { expect, test } from "vitest";
import { richToTiptap, tiptapToRich } from "./richtext";
import { insertLines, readable, textNodes } from "./textNodes";
import type { RichText, Run, TextStyle } from "./types";

const style: TextStyle = { font: "Andika", size_pt: 14, color: "#222222", align: "Left" };
const run = (text: string, bold = false): Run => ({ text, bold, italic: false, underline: false, color: null, font: null, size_pt: null });

// Un texte collé sur plusieurs lignes est enregistré avec des « \n » : il doit se rouvrir intact.
test("a text saved with line breaks reopens in the editor, line breaks kept", () => {
  const schema = getSchema([...textNodes, Bold]);
  for (const rt of [
    { paragraphs: [{ align: null, runs: [run("lundi\nmardi")] }] },
    { paragraphs: [{ align: null, runs: [run("lundi "), run("\nmardi", true)] }] },
    { paragraphs: [{ align: null, runs: [run("lundi", true), run("\nmardi\n")] }] },
  ] as RichText[]) {
    const doc = schema.nodeFromJSON(richToTiptap(rt));
    doc.check();
    expect(tiptapToRich(doc.toJSON(), style)).toEqual(rt);
  }
});

// Collé ou déposé : chaque ligne du texte est séparée par un vrai retour à la ligne, pas par un « \n » dans
// le texte, que le navigateur peut changer en espace pendant la frappe.
test("pasted lines are separated by line breaks, with the formatting at the cursor", () => {
  const schema = getSchema([...textNodes, Bold]);
  const doc = schema.nodeFromJSON(richToTiptap({ paragraphs: [{ align: null, runs: [run("[]", true)] }] }));
  let state = EditorState.create({ doc, selection: TextSelection.create(doc, 2) });
  state = state.apply(insertLines(state, "lundi\r\nmardi\n"));
  const json = state.doc.toJSON();
  expect(JSON.stringify(json)).not.toContain("\\n");
  expect(tiptapToRich(json, style)).toEqual({ paragraphs: [{ align: null, runs: [run("[lundi\nmardi\n]", true)] }] });
});

// Une zone que l'éditeur ne sait pas lire s'ouvrirait vide : elle garde alors son texte enregistré.
test("readable tells an empty zone from a content the editor would refuse", () => {
  const schema = getSchema(textNodes);
  expect(readable(schema, richToTiptap({ paragraphs: [] }))).toBe(true);
  expect(readable(schema, { type: "doc", content: [{ type: "paragraph", content: [{ type: "image" }] }] })).toBe(false);
});
