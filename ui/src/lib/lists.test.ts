import { getSchema } from "@tiptap/core";
import Bold from "@tiptap/extension-bold";
import TextAlign from "@tiptap/extension-text-align";
import { TextStyle as TextStyleMark, FontSize } from "@tiptap/extension-text-style";
import { history, undo } from "@tiptap/pm/history";
import { EditorState, TextSelection, type Transaction } from "@tiptap/pm/state";
import { expect, test } from "vitest";
import { FALLBACK_METRICS } from "./lineSpacing";
import { blankWithoutBullets, bulletLiftFactor, bulletsOn, firstNodePt, firstWordPt, listActive, listBackspace, listEnter, toggledBullets, toggleList } from "./lists";
import { richToTiptap, tiptapToRich } from "./richtext";
import { insertLines, textNodes } from "./textNodes";
import { finishedText } from "./textLook";
import type { Paragraph, RichText, Run, TextProps, TextStyle } from "./types";

const schema = getSchema([...textNodes, Bold, TextStyleMark, FontSize, TextAlign.configure({ types: ["paragraph"] })]);
const style: TextStyle = { font: "Andika", size_pt: 14, color: "#222222", align: "Left" };
const run = (text: string, size_pt: number | null = null): Run => ({ text, bold: false, italic: false, underline: false, color: null, font: null, size_pt });
const para = (text: string, bullet = false): Paragraph => (bullet ? { align: null, runs: text ? [run(text)] : [], list: "Bullet" } : { align: null, runs: text ? [run(text)] : [] });
const props = (paragraphs: Paragraph[]): TextProps => ({ content: { paragraphs }, style, prompt: null });

/** Un éditeur sur `rt`, le curseur en `pos` (ou de `pos` à `to`), avec l'historique d'annulation. */
function start(rt: RichText, pos: number, to = pos): EditorState {
  const doc = schema.nodeFromJSON(richToTiptap(rt));
  return EditorState.create({ doc, plugins: [history()], selection: TextSelection.create(doc, pos, to) });
}
const rich = (s: EditorState) => tiptapToRich(s.doc.toJSON(), style);
const apply = (s: EditorState, tr: Transaction | null) => (tr ? s.apply(tr) : s);
const bullets = (s: EditorState) => rich(s).paragraphs.map((p) => !!p.list);
const texts = (s: EditorState) => rich(s).paragraphs.map((p) => p.runs.map((r) => r.text).join(""));

test("the bullet survives every conversion, and a paragraph without one has no key", () => {
  // Comme le moteur l'écrit : `list` après `runs`, absente sans puce.
  const rt: RichText = { paragraphs: [para("Avant"), para("Colle", true), { align: "Center", runs: [run("Ciseaux", 28)], list: "Bullet" }, para("", true)] };
  const back = tiptapToRich(richToTiptap(rt), style);
  expect(JSON.stringify(back)).toBe(JSON.stringify(rt));
  expect("list" in back.paragraphs[0]).toBe(false);
  // Ouvert puis refermé sans rien taper : rien à enregistrer, pas de pas d'annulation.
  expect(finishedText(props(rt.paragraphs), back)).toBeNull();
  const doc = schema.nodeFromJSON(richToTiptap(rt));
  doc.check();
  expect(tiptapToRich(doc.toJSON(), style)).toEqual(rt);
});

test("a bullet list pasted from elsewhere becomes bulleted paragraphs, one level", () => {
  const item = (text: string, more: object[] = []) => ({ type: "listItem", content: [{ type: "paragraph", content: [{ type: "text", text }] }, ...more] });
  const doc = {
    type: "doc",
    content: [
      { type: "paragraph", content: [{ type: "text", text: "Matériel" }] },
      { type: "bulletList", content: [item("colle"), item("ciseaux", [{ type: "bulletList", content: [item("à bouts ronds")] }])] },
      { type: "orderedList", content: [item("un")] },
    ],
  };
  const rt = tiptapToRich(doc, style);
  expect(rt.paragraphs.map((p) => [p.runs[0].text, p.list ?? null])).toEqual([
    ["Matériel", null],
    ["colle", "Bullet"],
    ["ciseaux", "Bullet"],
    ["à bouts ronds", "Bullet"],
    ["un", null],
  ]);
});

test("out of writing, the button bullets every non-empty paragraph, or removes all", () => {
  const t = props([para("Colle"), para(""), para("Ciseaux", true)]);
  const on = toggledBullets(t)!;
  expect(on.content.paragraphs.map((p) => p.list ?? null)).toEqual(["Bullet", null, "Bullet"]);
  expect(bulletsOn(on.content)).toBe(true);
  const off = toggledBullets(on)!;
  expect(off.content.paragraphs.every((p) => !("list" in p))).toBe(true);
  expect(bulletsOn(off.content)).toBe(false);
  // Sans texte : rien à changer.
  expect(toggledBullets(props([para(""), para(" ")]))).toBeNull();
  expect(bulletsOn({ paragraphs: [] })).toBe(false);
});

test("a text left empty loses its bullets, a text with words keeps its empty bulleted lines", () => {
  expect(blankWithoutBullets({ paragraphs: [para("", true)] })).toEqual({ paragraphs: [para("")] });
  const kept = { paragraphs: [para("Colle", true), para("", true)] };
  expect(blankWithoutBullets(kept)).toBe(kept);
  expect(finishedText(props([]), { paragraphs: [para("", true)] })).toEqual(props([para("")]));
});

test("Enter in an item continues the list (the split keeps the attribute)", () => {
  const s = start({ paragraphs: [para("Colle", true)] }, 6);
  const split = s.apply(s.tr.split(6));
  expect(bullets(split)).toEqual([true, true]);
  // Sur un élément plein, la règle laisse Entrée suivre son cours.
  expect(listEnter(s)).toBeNull();
});

test("Enter on an empty item leaves the list, Backspace at the start of an item removes its bullet without merging", () => {
  let s = start({ paragraphs: [para("Colle", true), para("", true)] }, 8);
  s = apply(s, listEnter(s));
  expect(bullets(s)).toEqual([true, false]);
  expect(listEnter(s)).toBeNull();

  s = start({ paragraphs: [para("Colle", true), para("Ciseaux", true)] }, 8);
  expect(listBackspace(s)).not.toBeNull();
  s = apply(s, listBackspace(s));
  expect(texts(s)).toEqual(["Colle", "Ciseaux"]);
  expect(bullets(s)).toEqual([true, false]);
  // Ailleurs qu'au début, ou sans puce : Retour arrière efface comme d'habitude.
  expect(listBackspace(start({ paragraphs: [para("Colle", true)] }, 3))).toBeNull();
  expect(listBackspace(s)).toBeNull();
  // Ctrl+Z rend la puce.
  undo(s, (tr) => (s = s.apply(tr)));
  expect(bullets(s)).toEqual([true, true]);
});

test("Ctrl+Z right after a keyboard rule gives back the bullet, not the typed text", () => {
  let s = start({ paragraphs: [para("Colle", true)] }, 6);
  s = s.apply(s.tr.insertText("s"));
  s = s.apply(s.tr.split(s.selection.from));
  s = apply(s, listEnter(s));
  expect(bullets(s)).toEqual([true, false]);
  undo(s, (tr) => (s = s.apply(tr)));
  expect([texts(s), bullets(s)]).toEqual([["Colles", ""], [true, true]]);

  let b = start({ paragraphs: [para("Colle", true)] }, 1);
  b = b.apply(b.tr.insertText("La "));
  b = b.apply(b.tr.setSelection(TextSelection.create(b.doc, 1)));
  b = apply(b, listBackspace(b));
  undo(b, (tr) => (b = b.apply(tr)));
  expect([texts(b), bullets(b)]).toEqual([["La Colle"], [true]]);
});

test("Shift+Enter in an item stays in it; pasted lines become items", () => {
  const s = start({ paragraphs: [para("Colle", true)] }, 6);
  const br = s.apply(s.tr.replaceSelectionWith(schema.nodes.hardBreak.create(), true));
  expect(rich(br).paragraphs).toEqual([{ align: null, runs: [run("Colle\n")], list: "Bullet" }]);

  const pasted = s.apply(insertLines(s, " et pinceaux\r\nfeutres\n"));
  expect(texts(pasted)).toEqual(["Colle et pinceaux", "feutres", ""]);
  expect(bullets(pasted)).toEqual([true, true, true]);
  // Hors liste, les lignes restent dans le paragraphe, comme avant.
  const plain = start({ paragraphs: [para("Colle")] }, 6);
  expect(texts(plain.apply(insertLines(plain, "\nfeutres")))).toEqual(["Colle\nfeutres"]);
});

test("pasted lines in an item keep the formatting at the cursor", () => {
  const rt: RichText = { paragraphs: [{ align: null, runs: [{ ...run("[]"), bold: true }], list: "Bullet" }] };
  const s = start(rt, 2);
  const out = rich(s.apply(insertLines(s, "a\nb")));
  expect(out.paragraphs.map((p) => p.runs.map((r) => [r.text, r.bold]))).toEqual([[["[a", true]], [["b]", true]]]);
});

test("while writing, the button toggles the cursor's paragraph, or the non-empty ones of the selection, in one step", () => {
  const rt = { paragraphs: [para("Colle"), para(""), para("Ciseaux")] };
  let s = start(rt, 3);
  expect(listActive(s)).toBe(false);
  s = s.apply(toggleList(s));
  expect(bullets(s)).toEqual([true, false, false]);
  expect(listActive(s)).toBe(true);
  // Une ligne vide seule prend une puce (pour commencer une liste).
  let e = start(rt, 8);
  e = e.apply(toggleList(e));
  expect(bullets(e)).toEqual([false, true, false]);
  // Une sélection sur plusieurs paragraphes : ceux qui ont du texte.
  s = s.apply(s.tr.setSelection(TextSelection.create(s.doc, 1, 16)));
  expect(listActive(s)).toBe(false);
  s = s.apply(toggleList(s));
  expect(bullets(s)).toEqual([true, false, true]);
  s = s.apply(toggleList(s));
  expect(bullets(s)).toEqual([false, false, false]);
  undo(s, (tr) => (s = s.apply(tr)));
  expect(bullets(s)).toEqual([true, false, true]);
  // Juste après une frappe, Ctrl+Z n'annule que le bouton.
  let t = start({ paragraphs: [para("Colle")] }, 6);
  t = t.apply(t.tr.insertText("s"));
  t = t.apply(toggleList(t));
  undo(t, (tr) => (t = t.apply(tr)));
  expect([texts(t), bullets(t)]).toEqual([["Colles"], [false]]);
});

test("in a brand-new text (no paragraph yet), the button starts the list with the cursor in it", () => {
  let s = start({ paragraphs: [] }, 0);
  expect(s.doc.childCount).toBe(0);
  s = s.apply(toggleList(s));
  expect(bullets(s)).toEqual([true]);
  s = s.apply(s.tr.insertText("colle"));
  expect(rich(s).paragraphs).toEqual([para("colle", true)]);
});

test("the first word's own size lifts the bullet as LibreOffice does", () => {
  expect(firstWordPt([run(""), run("Grand", 28), run(" mot")])).toBe(28);
  expect(firstWordPt([run("mot"), run("Grand", 28)])).toBeNull();
  expect(firstWordPt([])).toBeNull();
  // Andika : (2500 - 800) / (2 × 2048) em par point d'écart.
  expect(bulletLiftFactor(FALLBACK_METRICS)).toBeCloseTo(0.41504, 5);
  expect(bulletLiftFactor({ units_per_em: 1000, ascent: 900, descent: 300 })).toBeCloseTo(0.3, 9);
  expect(bulletLiftFactor(null)).toBe(0);
  // Dans l'éditeur, la taille du premier morceau.
  const doc = schema.nodeFromJSON(richToTiptap({ paragraphs: [{ align: null, runs: [run("Grand", 28), run(" mot")], list: "Bullet" }, para("mot", true)] }));
  expect(firstNodePt(doc.child(0))).toBe(28);
  expect(firstNodePt(doc.child(1))).toBeNull();
});
