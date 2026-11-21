import { expect, test } from "vitest";
import { newDocument, newPage } from "./factory";
import { carryChapter, chapterFileName, chapterOf, chapterOutline, hiddenPages, chapterSpans, moveChapter, removeChapter, startChapter, updateChapter, withoutPage } from "./chapters";
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

test("dans la bande, seul le chapitre de la page affichée montre ses pages ; celles d'avant le premier chapitre restent", () => {
  const doc = year();
  expect(hiddenPages(doc.pages, 0)).toEqual([false, true, true, true, true, true]);
  expect(hiddenPages(doc.pages, 2)).toEqual([false, false, false, true, true, true]);
  expect(hiddenPages(doc.pages, 4)).toEqual([false, true, true, false, false, false]);
});

test("un nouveau chapitre ne prend pas la couleur d'un chapitre voisin", () => {
  const doc = year();
  const [sept, oct] = [doc.pages[1].chapter!.color, doc.pages[3].chapter!.color];
  const between = startChapter(doc, 2, "Entre les deux").pages[2].chapter!.color;
  expect([sept, oct]).not.toContain(between);
  // Sept chapitres et plus : les couleurs reviennent, jamais côte à côte.
  let many = newDocument();
  for (let k = 0; k < 9; k++) many.pages.push(newPage());
  for (let k = 0; k < 9; k++) many = startChapter(many, k, `C${k}`);
  const colors = chapterSpans(many.pages).map((c) => c.color);
  colors.slice(1).forEach((c, k) => expect(c).not.toBe(colors[k]));
});

test("signets d'un PDF : un par chapitre, à sa première page dans le PDF", () => {
  const doc = year();
  expect(chapterOutline(doc.pages)).toEqual([
    { title: "Septembre", page: 1 },
    { title: "Octobre", page: 3 },
  ]);
  // Le PDF du seul chapitre Octobre : un signet, à sa première page.
  expect(chapterOutline(doc.pages, 3, 6)).toEqual([{ title: "Octobre", page: 0 }]);
  expect(chapterFileName("Cahier de vie", "Sortie : la ferme / mai")).toBe("Cahier de vie - Sortie la ferme mai");
  expect(chapterFileName("Cahier", "  ")).toBe("Cahier");
});
