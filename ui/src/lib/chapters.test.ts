import { expect, test } from "vitest";
import { newDocument, newPage } from "./factory";
import { carryChapter, chapterOf, hiddenPages, chapterSpans, moveChapter, removeChapter, startChapter, updateChapter, withoutPage } from "./chapters";
import { duplicatePage, movePage } from "./pageOrder";
import type { Document } from "./types";

/** Six pages : la première sans chapitre, « Septembre » sur 1 et 2, « Octobre » sur 3 à 5. */
function year(): Document {
  const doc = newDocument();
  doc.pages.push(newPage(), newPage(), newPage(), newPage(), newPage());
  return startChapter(startChapter(doc, 1, "Septembre"), 3, "Octobre");
}
const titles = (d: Document) => d.pages.map((p) => p.chapter?.title ?? "");

test("un chapitre va de sa page jusqu'au suivant ; les pages d'avant le premier n'en ont pas", () => {
  const doc = year();
  expect(chapterSpans(doc.pages).map(({ title, start, end }) => [title, start, end])).toEqual([
    ["Septembre", 1, 3],
    ["Octobre", 3, 6],
  ]);
  expect(chapterOf(doc.pages, 0)).toBeNull();
  expect(chapterOf(doc.pages, 2)?.title).toBe("Septembre");
  expect(chapterOf(doc.pages, 5)?.title).toBe("Octobre");
});

test("chaque nouveau chapitre prend une autre couleur ; on le renomme et on le retire", () => {
  const doc = year();
  expect(doc.pages[1].chapter!.color).not.toBe(doc.pages[3].chapter!.color);
  expect(updateChapter(doc, 3, { title: "Automne" }).pages[3].chapter).toEqual({ title: "Automne", color: doc.pages[3].chapter!.color });
  expect(updateChapter(doc, 0, { title: "x" })).toBe(doc);
  const removed = removeChapter(doc, 3);
  expect(chapterOf(removed.pages, 5)?.title).toBe("Septembre");
  expect(removed.pages.length).toBe(6);
});

test("déplacer la première page d'un chapitre laisse le chapitre à sa place", () => {
  const doc = year();
  const moved = movePage(doc, 3, 0);
  expect(moved.pages[0].id).toBe(doc.pages[3].id);
  expect(titles(moved)).toEqual(["", "", "Septembre", "", "Octobre", ""]);
  // Posée ailleurs, une page rejoint le chapitre de la page d'avant.
  expect(chapterOf(movePage(doc, 0, 4).pages, 4)?.title).toBe("Octobre");
});

test("supprimer la première page d'un chapitre le passe à la suivante", () => {
  const pages = withoutPage(year().pages, 3);
  expect(titles({ ...year(), pages })).toEqual(["", "Septembre", "", "Octobre", ""]);
});

test("une copie de page ne rouvre pas le chapitre ; des pages qui en remplacent une gardent son chapitre sur la première", () => {
  const doc = year();
  expect(duplicatePage(doc.pages[1]).chapter).toBeNull();
  const made = carryChapter(doc.pages[3], [duplicatePage(doc.pages[3]), duplicatePage(doc.pages[3])].map((p) => ({ ...p, chapter: doc.pages[3].chapter })));
  expect(made.map((p) => p.chapter?.title ?? "")).toEqual(["Octobre", ""]);
});

test("déplacer un chapitre emporte toutes ses pages ; les pages sans chapitre restent en tête", () => {
  const doc = year();
  const ids = doc.pages.map((p) => p.id);
  const before = moveChapter(doc, 3, -1)!;
  expect(before.start).toBe(1);
  expect(before.doc.pages.map((p) => p.id)).toEqual([ids[0], ids[3], ids[4], ids[5], ids[1], ids[2]]);
  const after = moveChapter(doc, 1, 1)!;
  expect(after.start).toBe(4);
  expect(after.doc.pages.map((p) => p.id)).toEqual(before.doc.pages.map((p) => p.id));
  expect(moveChapter(doc, 1, -1)).toBeNull();
  expect(moveChapter(doc, 3, 1)).toBeNull();
});

test("un chapitre replié cache ses pages, même celui de la page affichée, sauf cette page", () => {
  const doc = year();
  const folded = [doc.pages[3].id];
  expect(hiddenPages(doc.pages, folded, 0)).toEqual([false, false, false, true, true, true]);
  // La page affichée est dans le chapitre replié : seule sa miniature reste.
  expect(hiddenPages(doc.pages, folded, 4)).toEqual([false, false, false, true, false, true]);
  expect(hiddenPages(doc.pages, [], 4)).toEqual([false, false, false, false, false, false]);
});
