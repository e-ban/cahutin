import { expect, test } from "vitest";
import { fitActions, grownHeight, MIN_TEXT_PT, neededHeightMm, scaledText, shrunkText, actionsPlace } from "./textFit";
import { textElement } from "./factory";
import type { Element, Page, TextProps } from "./types";

const frame = (y: number, h: number, rotation_deg = 0) => ({ x_mm: 10, y_mm: y, w_mm: 80, h_mm: h, rotation_deg });

function textProps(block: number, runs: (number | null)[], named: string | null = null): TextProps {
  const el = textElement(frame(0, 10));
  if (el.kind.type !== "Text") throw new Error();
  return {
    ...el.kind,
    style: { ...el.kind.style, size_pt: block, named },
    content: { paragraphs: [{ align: null, runs: runs.map((size_pt) => ({ text: "mot", bold: false, italic: false, underline: false, color: null, font: null, size_pt })) }] },
  };
}

const sizes = (p: TextProps) => [p.style.size_pt, ...p.content.paragraphs.flatMap((q) => q.runs.map((r) => r.size_pt))];

test("the needed height is the content plus the padding, rounded up to 0.1 mm", () => {
  // 100 px de texte = 26,458 mm, plus 1,5 mm de chaque côté.
  expect(neededHeightMm(100, 1.5)).toBe(29.5);
  expect(neededHeightMm(0, 1.5)).toBe(3);
});

test("growing gives the needed height, without going past the bottom of the page", () => {
  expect(grownHeight(40, frame(100, 20), 297)).toEqual({ h_mm: 40, fits: true });
  // Plus assez de place : jusqu'au bas de la page, et le texte dépasse encore.
  expect(grownHeight(80, frame(250, 20), 297)).toEqual({ h_mm: 47, fits: false });
  // Le cadre ne rapetisse jamais, et un gain trop petit n'est pas une modification.
  expect(grownHeight(15, frame(100, 20), 297)).toBeNull();
  expect(grownHeight(20.05, frame(100, 20), 297)).toBeNull();
  // Déjà au bas de la page (ou en dessous) : rien à faire.
  expect(grownHeight(40, frame(277, 20), 297)).toBeNull();
  expect(grownHeight(40, frame(290, 20), 297)).toBeNull();
});

test("scaling keeps the proportion between the block and its words, in half points", () => {
  expect(sizes(scaledText(textProps(20, [null, 30]), 0.75))).toEqual([15, null, 22.5]);
  expect(sizes(scaledText(textProps(14, [null]), 0.8))).toEqual([11, null]);
});

test("shrinking takes the largest size that fits, never below the floor", () => {
  const p = textProps(20, [null, 40]);
  // Le texte tient dès que le bloc est à 15 pt ou moins.
  const fit = shrunkText(p, (q) => q.style.size_pt <= 15)!;
  expect(sizes(fit)).toEqual([15, null, 30]);
  // Rien ne tient : les plus petites lettres s'arrêtent au plancher, les autres dans la même proportion.
  const floor = shrunkText(p, () => false)!;
  expect(sizes(floor)).toEqual([MIN_TEXT_PT, null, 2 * MIN_TEXT_PT]);
  // Un passage plus petit que le bloc atteint le plancher le premier.
  expect(sizes(shrunkText(textProps(20, [10]), () => false)!)).toEqual([12, 6]);
});

test("nothing to shrink when the smallest letters are already at the floor", () => {
  expect(shrunkText(textProps(MIN_TEXT_PT, [null]), () => false)).toBeNull();
  expect(shrunkText(textProps(14, [5]), () => false)).toBeNull();
  // Le texte tient déjà : aucune modification.
  expect(shrunkText(textProps(14, [null]), () => true)).toBeNull();
});

function pageWith(e: Element, extra: Partial<Page> = {}): Page {
  return { id: "p", size: { width_mm: 210, height_mm: 297 }, elements: [e], grid: null, ...extra };
}

test("both actions for a plain text block with room below", () => {
  const e = textElement(frame(100, 20));
  expect(fitActions(pageWith(e), e)).toEqual({ grow: true, shrink: true });
});

test("exclusions: what each kind of block is offered", () => {
  const base = textElement(frame(100, 20));
  // Verrouillé : rien.
  const locked = { ...base, locked: true };
  expect(fitActions(pageWith(locked), locked)).toEqual({ grow: false, shrink: false });
  // Case de grille : son cadre est fixé par la grille, seul le texte se réduit.
  const grid = { cols: 1, rows: 1, margin_mm: 10, gap_mm: 4, cells: [{ element_id: base.id, col: 0, row: 0, col_span: 1, row_span: 1 }] } as unknown as Page["grid"];
  expect(fitActions(pageWith(base, { grid }), base)).toEqual({ grow: false, shrink: true });
  // Style nommé : réduire le détacherait de son style ; on peut agrandir.
  const named = { ...base, kind: { ...(base.kind as TextProps & { type: "Text" }), style: { ...(base.kind as TextProps).style, named: "titre" } } };
  expect(fitActions(pageWith(named), named)).toEqual({ grow: true, shrink: false });
  // Tourné : il tourne autour de son centre, agrandir le déplacerait ; on peut réduire.
  const turned = { ...base, frame: frame(100, 20, 30) };
  expect(fitActions(pageWith(turned), turned)).toEqual({ grow: false, shrink: true });
  // Au bas de la page : plus de place pour agrandir.
  const bottom = { ...base, frame: frame(277, 20) };
  expect(fitActions(pageWith(bottom), bottom)).toEqual({ grow: false, shrink: true });
  // Lettres déjà au plancher : plus rien à réduire.
  const tiny = { ...base, kind: { ...(base.kind as TextProps & { type: "Text" }), style: { ...(base.kind as TextProps).style, size_pt: MIN_TEXT_PT } } };
  expect(fitActions(pageWith(tiny), tiny)).toEqual({ grow: true, shrink: false });
});

test("a locked member of the block's group blocks the actions, as for moving it", () => {
  const a = { ...textElement(frame(100, 20)), group: "g" };
  const b = { ...textElement(frame(10, 20)), group: "g", locked: true };
  const page = { ...pageWith(a), elements: [a, b] };
  expect(fitActions(page, a)).toEqual({ grow: false, shrink: false });
  // Entré dans le groupe, le bloc se modifie seul.
  expect(fitActions(page, a, ["g"])).toEqual({ grow: true, shrink: true });
});

test("the actions go inside the block when there is no room under it on the page", () => {
  const A4 = { width_mm: 210, height_mm: 297 };
  expect(actionsPlace(frame(100, 20), A4, 1, 40, 200).inside).toBe(false);
  expect(actionsPlace(frame(280, 15), A4, 1, 40, 200).inside).toBe(true);
  // À fort zoom, les mêmes millimètres font plus de pixels.
  expect(actionsPlace(frame(280, 10), A4, 4, 40, 200).inside).toBe(false);
});

test("the actions keep to the page sideways: on the block's left edge when the right one is too near the page's left", () => {
  const A4 = { width_mm: 210, height_mm: 297 };
  const narrow = { ...frame(100, 20), x_mm: 5, w_mm: 20 };
  // 25 mm = 94 px : 200 px de boutons sortiraient de la page par la gauche.
  expect(actionsPlace(narrow, A4, 1, 40, 200).left).toBe(true);
  expect(actionsPlace(frame(100, 20), A4, 1, 40, 200).left).toBe(false);
  // À fort zoom, le bord droit est assez loin du bord de la page.
  expect(actionsPlace(narrow, A4, 3, 40, 200).left).toBe(false);
});
