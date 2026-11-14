import type { Editor } from "@tiptap/core";
import { afterEach, expect, test } from "vitest";
import { editor } from "./docStore.svelte";
import { newDocument, textElement } from "./factory";
import { isListActive, setColor, setFont, setLineSpacing, setSize, toggleList } from "./textFormat.svelte";
import { textEditing } from "./textEditing.svelte";
import type { Element, TextProps } from "./types";

const run = (text: string, size_pt: number | null) => ({ text, bold: false, italic: false, underline: false, color: size_pt ? "#ff0000" : null, font: size_pt ? "Arial" : null, size_pt });

function open(): string {
  const doc = newDocument("t");
  const e: Element = textElement({ x_mm: 0, y_mm: 0, w_mm: 50, h_mm: 20, rotation_deg: 0 });
  if (e.kind.type === "Text") e.kind.content = { paragraphs: [{ align: null, runs: [run("Bonjour ", null), run("Léa", 30)] }] };
  doc.pages[0].elements.push(e);
  editor.reset(doc, null);
  editor.selection = [e.id];
  return e.id;
}

const props = (): TextProps => {
  const k = editor.page.elements[0].kind;
  if (k.type !== "Text") throw new Error();
  return k;
};

/** Un éditeur en cours d'écriture, sélection de `from` à `to` dans « Bonjour Léa » (document de taille 13). */
function writing(id: string, from: number, to: number): string[] {
  const calls: string[] = [];
  const chain: Record<string, unknown> = {};
  for (const c of ["focus", "setFontSize", "setFontFamily", "setColor"]) chain[c] = (v?: string) => (calls.push(v ? `${c} ${v}` : c), chain);
  chain.run = () => true;
  textEditing.current = { chain: () => chain, state: { selection: { from, to }, doc: { content: { size: 13 } } } } as unknown as Editor;
  editor.editingText = id;
  return calls;
}

afterEach(() => {
  textEditing.current = null;
  editor.editingText = null;
});

test("sans écriture, taille, police et couleur vont à tout le bloc, mots compris", () => {
  open();
  setSize(144);
  setFont("OpenDyslexic");
  setColor("#1f2c4a");
  expect(props().style).toMatchObject({ size_pt: 144, font: "OpenDyslexic", color: "#1f2c4a" });
  expect(props().content.paragraphs[0].runs[1]).toMatchObject({ text: "Léa", size_pt: null, font: null, color: null });
});

test("pendant l'écriture, une sélection partielle ne change que le passage", () => {
  const id = open();
  const calls = writing(id, 9, 12);
  setSize(96);
  expect(calls).toContain("setFontSize 96pt");
  expect(props().style.size_pt).toBe(14);
});

test("pendant l'écriture, tout le texte sélectionné ne change que l'éditeur : son Ctrl+Z l'annule", () => {
  const id = open();
  const calls = writing(id, 1, 12);
  setSize(96);
  setFont("OpenDyslexic");
  expect(calls).toEqual(expect.arrayContaining(["setFontSize 96pt", "setFontFamily OpenDyslexic"]));
  // Le bloc ne change qu'à la fin de l'écriture (`liftUniformLook`), dans le même pas que le contenu.
  expect(props().style).toMatchObject({ size_pt: 14, font: "Andika" });
  expect(editor.canUndo).toBe(false);
});

test("l'interligne va au bloc entier, même pendant l'écriture, en un pas d'annulation", () => {
  const id = open();
  const calls = writing(id, 9, 12);
  setLineSpacing(1.5);
  expect(props().style.line_spacing).toBe(1.5);
  expect(calls).toEqual([]);
  setLineSpacing(null);
  expect(props().style.line_spacing).toBeNull();
  editor.undo();
  expect(props().style.line_spacing).toBe(1.5);
  editor.undo();
  expect(props().style.line_spacing ?? null).toBeNull();
});

test("sans écriture, « Puces » met une puce à chaque paragraphe non vide, ou les retire, en un pas d'annulation", () => {
  open();
  expect(isListActive(props())).toBe(false);
  toggleList();
  expect(props().content.paragraphs[0].list).toBe("Bullet");
  expect(isListActive(props())).toBe(true);
  toggleList();
  expect("list" in props().content.paragraphs[0]).toBe(false);
  editor.undo();
  expect(props().content.paragraphs[0].list).toBe("Bullet");
  editor.undo();
  expect(props().content.paragraphs[0].list).toBeUndefined();
  expect(editor.canUndo).toBe(false);
});
