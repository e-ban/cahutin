import { expect, test, vi } from "vitest";
import type { MyImage } from "./types";

// Le dossier simulé : les images de la collection et celles retirées.
const img = (name: string, themes: string[] = []): MyImage => ({ name, size: 1, modified: 0, themes });
let kept = [img("chat.png", ["Animaux"]), img("pomme.png"), img("soleil.png")];
let removed: MyImage[] = [];

vi.mock("./api", () => ({
  myImagesList: async () => kept,
  myImagesRemoved: async () => removed,
  myImagesRemove: async (name: string) => {
    removed = [...removed, ...kept.filter((i) => i.name === name)];
    kept = kept.filter((i) => i.name !== name);
    return kept;
  },
}));

const { myImages } = await import("./myImages.svelte");
const { filterByTheme } = await import("./myImages");

test("a removed picture leaves « Sans thème » and joins the removed ones at once", async () => {
  await myImages.load();
  await myImages.loadRemoved();
  expect(filterByTheme(myImages.images, { kind: "none" }).length).toBe(2);
  expect(myImages.removed.length).toBe(0);
  await myImages.remove("pomme.png");
  expect(filterByTheme(myImages.images, { kind: "none" }).map((i) => i.name)).toEqual(["soleil.png"]);
  expect(myImages.removed.map((i) => i.name)).toEqual(["pomme.png"]);
});
