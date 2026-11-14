import { getSchema } from "@tiptap/core";
import Bold from "@tiptap/extension-bold";
import { history, undo } from "@tiptap/pm/history";
import { AllSelection, EditorState, TextSelection, type Transaction } from "@tiptap/pm/state";
import { expect, test } from "vitest";
import { NBSP, insertChar, insertGuillemets, typeWithSpacing } from "./frenchTypo";
import { plainText } from "./fieldNode";
import { richToTiptap, tiptapToRich } from "./richtext";
import { textNodes } from "./textNodes";
import type { Run, TextStyle } from "./types";

const schema = getSchema([...textNodes, Bold]);
const style: TextStyle = { font: "Andika", size_pt: 14, color: "#222222", align: "Left" };
const run = (text: string, bold = false): Run => ({ text, bold, italic: false, underline: false, color: null, font: null, size_pt: null });

/** Un texte d'un paragraphe (les champs en accolades), le curseur à la fin, avec l'historique d'annulation. */
function start(text: string, bold = false): EditorState {
  const doc = schema.nodeFromJSON(richToTiptap({ paragraphs: [{ align: null, runs: [run(text, bold)] }] }));
  return EditorState.create({ doc, plugins: [history()], selection: TextSelection.atEnd(doc) });
}

/** La frappe de chaque caractère au curseur, comme au clavier : la règle d'abord, sinon la frappe normale. */
function type(state: EditorState, keys: string): EditorState {
  for (const k of keys) {
    const { from, to } = state.selection;
    const apply = (tr: Transaction) => (state = state.apply(tr));
    if (!typeWithSpacing(state, from, to, k, apply)) apply(state.tr.insertText(k, from, to));
  }
  return state;
}

const text = (state: EditorState) => tiptapToRich(state.doc.toJSON(), style).paragraphs[0]?.runs.map((r) => r.text).join("") ?? "";

function undone(state: EditorState): EditorState {
  undo(state, (tr) => (state = state.apply(tr)));
  return state;
}

test("a space typed before : ; ! ? becomes a non-breaking space", () => {
  for (const p of [":", ";", "!", "?"]) expect(text(type(start("Bravo"), ` ${p}`))).toBe(`Bravo${NBSP}${p}`);
  // Le texte copié garde l'espace insécable.
  expect(plainText(type(start("Bravo"), " !").doc.content)).toBe(`Bravo${NBSP}!`);
});

test("nothing is added when no space was typed", () => {
  expect(text(type(start("Hello"), "!"))).toBe("Hello!");
  expect(text(type(start(""), "http://site"))).toBe("http://site");
  expect(text(type(start("Bravo"), `${NBSP}!`))).toBe(`Bravo${NBSP}!`);
});

test("a space typed after « or before » becomes a non-breaking space", () => {
  expect(text(type(start("«"), " oui »"))).toBe(`«${NBSP}oui${NBSP}»`);
  // Le curseur juste avant un » déjà écrit.
  let s = start("«oui»");
  s = s.apply(s.tr.setSelection(TextSelection.create(s.doc, 5)));
  expect(text(type(s, " "))).toBe(`«oui${NBSP}»`);
});

test("straight quotes are never changed", () => {
  expect(text(type(start(""), '"Hello" !'))).toBe(`"Hello"${NBSP}!`);
  expect(text(type(start(""), '" oui "'))).toBe('" oui "');
});

test("Ctrl+Z right after the change gives back exactly what was typed", () => {
  const s = type(start("Bravo"), " !");
  expect(text(undone(s))).toBe("Bravo !");
  expect(text(undone(type(start("«"), " ")))).toBe("« ");
  // Plus tard, la frappe suivante s'annule seule : l'espace insécable reste.
  expect(text(undone(type(s, " super")))).toBe(`Bravo${NBSP}!`);
});

test("the non-breaking space keeps the formatting of the typed space", () => {
  const s = type(start("Bravo", true), " !");
  expect(tiptapToRich(s.doc.toJSON(), style).paragraphs[0].runs).toEqual([run(`Bravo${NBSP}!`, true)]);
});

test("class fields are never changed", () => {
  // Un champ suivi d'une espace : seule l'espace change, le champ s'enregistre toujours en accolades.
  expect(text(type(start("Bravo {prenom}"), " !"))).toBe(`Bravo {prenom}${NBSP}!`);
  // Un champ juste avant le signe ne compte pas comme une espace.
  expect(text(type(start("Bravo {prenom}"), "!"))).toBe("Bravo {prenom}!");
  expect(text(type(start("{prenom}"), "?"))).toBe("{prenom}?");
});

test("a palette character goes at the cursor, in place of the chosen passage", () => {
  let s = start("cole");
  s = s.apply(s.tr.setSelection(TextSelection.create(s.doc, 1)));
  s = s.apply(insertChar(s, "É"));
  expect(text(s)).toBe("École");
  expect(s.selection.from).toBe(2);
  s = s.apply(s.tr.setSelection(TextSelection.create(s.doc, 1, 2)));
  expect(text(s.apply(insertChar(s, "À")))).toBe("Àcole");
});

test("guillemets go by pair, cursor in the middle, or around the chosen passage", () => {
  let s = start("Il dit ");
  s = s.apply(insertGuillemets(s));
  expect(text(s)).toBe(`Il dit «${NBSP}${NBSP}»`);
  s = type(s, "oui");
  expect(text(s)).toBe(`Il dit «${NBSP}oui${NBSP}»`);

  let w = start("Il dit oui.");
  w = w.apply(w.tr.setSelection(TextSelection.create(w.doc, 8, 11)));
  w = w.apply(insertGuillemets(w));
  expect(text(w)).toBe(`Il dit «${NBSP}oui${NBSP}».`);
  expect(w.doc.textBetween(w.selection.from, w.selection.to)).toBe("oui");
  // Un seul pas d'annulation.
  expect(text(undone(w))).toBe("Il dit oui.");
});

test("guillemets around a class field leave the field whole", () => {
  let s = start("{prenom}");
  s = s.apply(s.tr.setSelection(TextSelection.create(s.doc, 1, 2)));
  s = s.apply(insertGuillemets(s));
  expect(text(s)).toBe(`«${NBSP}{prenom}${NBSP}»`);
});

test("a palette character replaces a whole-text selection", () => {
  const s = start("Titre");
  const all = s.apply(s.tr.setSelection(new AllSelection(s.doc)));
  expect(text(all.apply(insertChar(all, "Ç")))).toBe("Ç");
});
