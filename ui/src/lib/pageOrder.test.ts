import { expect, test } from "vitest";
import { newDocument, newPage } from "./factory";
import { dropSlot, movedPage, movePage, slotTarget } from "./pageOrder";
import type { Document } from "./types";

function fourPages(): Document {
  const doc = newDocument();
  doc.pages.push(newPage(), newPage(), newPage());
  return doc;
}
const ids = (d: Document) => d.pages.map((p) => p.id);

test("déplacer une page la met à sa place finale, les autres gardent leur ordre et restent les mêmes", () => {
  const doc = fourPages();
  const [a, b, c, d] = doc.pages;
  const after = movePage(doc, 0, 2);
  expect(ids(after)).toEqual([b.id, c.id, a.id, d.id]);
  expect(after.pages[2]).toBe(a);
  expect(ids(movePage(doc, 3, 0))).toEqual([d.id, a.id, b.id, c.id]);
  expect(ids(movePage(doc, 1, 2))).toEqual([a.id, c.id, b.id, d.id]);
  // Le document de départ n'est pas touché.
  expect(ids(doc)).toEqual([a.id, b.id, c.id, d.id]);
});

test("rien à faire : le même document (aucun pas d'annulation vide)", () => {
  const doc = fourPages();
  expect(movePage(doc, 1, 1)).toBe(doc);
  expect(movePage(doc, 0, -1)).toBe(doc);
  expect(movePage(doc, 3, 4)).toBe(doc);
  const one = newDocument();
  expect(movePage(one, 0, 1)).toBe(one);
});

test("la fente sous le pointeur, d'après le milieu des miniatures (de tailles différentes)", () => {
  // Milieux de miniatures A4, A3 paysage, A5.
  const mids = [20, 90, 150];
  expect(dropSlot(mids, 0)).toBe(0);
  expect(dropSlot(mids, 21)).toBe(1);
  expect(dropSlot(mids, 89)).toBe(1);
  expect(dropSlot(mids, 149)).toBe(2);
  expect(dropSlot(mids, 400)).toBe(3);
});

test("une page lâchée juste avant ou juste après elle-même ne bouge pas", () => {
  expect(slotTarget(0, 0)).toBeNull();
  expect(slotTarget(0, 1)).toBeNull();
  // Page 1 après la page 3 (fente 3) : elle devient la troisième.
  expect(slotTarget(0, 3)).toBe(2);
  expect(slotTarget(0, 4)).toBe(3);
  // Page 3 tout au début.
  expect(slotTarget(2, 0)).toBe(0);
  expect(slotTarget(2, 2)).toBeNull();
  expect(slotTarget(2, 3)).toBeNull();
});

test("la page déplacée d'un ordre à l'autre", () => {
  const doc = fourPages();
  const [a, b, , d] = doc.pages;
  expect(movedPage(doc.pages, movePage(doc, 0, 2).pages)).toBe(a.id);
  expect(movedPage(movePage(doc, 0, 2).pages, doc.pages)).toBe(a.id);
  expect(movedPage(doc.pages, movePage(doc, 3, 0).pages)).toBe(d.id);
  // Deux voisines échangées : l'une ou l'autre, celle qu'on préfère si elle en est.
  const swapped = movePage(doc, 0, 1).pages;
  expect(movedPage(doc.pages, swapped, b.id)).toBe(b.id);
  expect(movedPage(doc.pages, swapped, d.id)).toBe(a.id);
  // Même ordre, page ajoutée ou retirée : aucune.
  expect(movedPage(doc.pages, doc.pages)).toBeNull();
  expect(movedPage(doc.pages, [...doc.pages, newPage()])).toBeNull();
  expect(movedPage(doc.pages, doc.pages.slice(1))).toBeNull();
});
