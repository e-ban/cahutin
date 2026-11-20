import { expect, test } from "vitest";
import { newDocument, newPage } from "./factory";
import { startChapter } from "./chapters";
import { deletePages, duplicatePages, followPage, keepsPages, movePagesTo } from "./manyPages";
import type { Document } from "./types";

/** Six pages : la première sans chapitre, « Septembre » sur 1 et 2, « Octobre » sur 3 à 5. */
function year(): Document {
  const doc = newDocument();
  doc.pages.push(newPage(), newPage(), newPage(), newPage(), newPage());
  return startChapter(startChapter(doc, 1, "Septembre"), 3, "Octobre");
}
const titles = (d: Document) => d.pages.map((p) => p.chapter?.title ?? "");

test("des pages choisies vont à la fin d'un chapitre, dans leur ordre", () => {
  const doc = year();
  const p = doc.pages;
  const moved = movePagesTo(doc, [p[4].id, p[0].id], 1);
  expect(moved.pages.map((x) => x.id)).toEqual([p[1].id, p[2].id, p[0].id, p[4].id, p[3].id, p[5].id]);
  expect(titles(moved)).toEqual(["Septembre", "", "", "", "Octobre", ""]);
});

test("la première page d'un chapitre part : le chapitre passe à sa page suivante", () => {
  const doc = year();
  const p = doc.pages;
  const moved = movePagesTo(doc, [p[3].id], 1);
  expect(moved.pages.map((x) => x.id)).toEqual([p[0].id, p[1].id, p[2].id, p[3].id, p[4].id, p[5].id]);
  expect(titles(moved)).toEqual(["", "Septembre", "", "", "Octobre", ""]);
});

test("un chapitre dont toutes les pages partent disparaît ; on ne range pas des pages dans un chapitre qui n'en garde aucune", () => {
  const doc = year();
  const p = doc.pages;
  const moved = movePagesTo(doc, [p[1].id, p[2].id], 3);
  expect(titles(moved)).toEqual(["", "Octobre", "", "", "", ""]);
  expect(moved.pages.slice(4).map((x) => x.id)).toEqual([p[1].id, p[2].id]);
  expect(keepsPages(p, 1, [p[1].id, p[2].id])).toBe(false);
  expect(movePagesTo(doc, [p[1].id, p[2].id], 1)).toBe(doc);
});

test("avant le premier chapitre : les pages vont au début du carnet, hors chapitre", () => {
  const doc = year();
  const p = doc.pages;
  const moved = movePagesTo(doc, [p[5].id], -1);
  expect(moved.pages.map((x) => x.id)).toEqual([p[0].id, p[5].id, p[1].id, p[2].id, p[3].id, p[4].id]);
});

test("dupliquer met chaque copie juste après sa page, sans rouvrir de chapitre", () => {
  const doc = year();
  const p = doc.pages;
  const twice = duplicatePages(doc, [p[1].id, p[4].id]);
  expect(twice.pages.length).toBe(8);
  expect(twice.pages[1].id).toBe(p[1].id);
  expect(twice.pages[2].chapter).toBeNull();
  expect(titles(twice)).toEqual(["", "Septembre", "", "", "Octobre", "", "", ""]);
});

test("supprimer garde les chapitres des autres pages, et jamais toutes les pages", () => {
  const doc = year();
  const p = doc.pages;
  expect(titles(deletePages(doc, [p[3].id, p[0].id]))).toEqual(["Septembre", "", "Octobre", ""]);
  expect(deletePages(doc, p.map((x) => x.id))).toBe(doc);
});

test("la page affichée est suivie ; supprimée, c'est la suivante qui reste qui s'affiche", () => {
  const doc = year();
  const p = doc.pages;
  const moved = movePagesTo(doc, [p[0].id], 3);
  expect(followPage(p, moved.pages, 0)).toBe(moved.pages.findIndex((x) => x.id === p[0].id));
  const fewer = deletePages(doc, [p[2].id, p[3].id]);
  expect(fewer.pages[followPage(p, fewer.pages, 2)].id).toBe(p[4].id);
  expect(followPage(p, deletePages(doc, [p[5].id]).pages, 5)).toBe(4);
});
