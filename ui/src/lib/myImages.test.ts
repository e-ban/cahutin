import { describe, expect, it } from "vitest";
import { displayName, filterByTheme, searchMyImages, showsChildren, themeCounts } from "./myImages";

const img = (name: string, themes: string[] = []) => ({ name, size: 1, modified: 0, themes });

describe("Mes images", () => {
  it("shows a clean name for downloaded files", () => {
    expect(displayName("960px-Lemmling-Cartoon-cow.svg.png")).toBe("Lemmling Cartoon cow");
    expect(displayName("citrouille_automne (2).PNG")).toBe("citrouille automne");
    expect(displayName("Ma maison.jpg")).toBe("Ma maison");
    expect(displayName(".png")).toBe(".png");
  });

  it("finds images by words, ignoring accents and case", () => {
    const all = [img("École-maternelle.png"), img("vache.svg"), img("cour d'école.jpg")];
    expect(searchMyImages(all, "ecole").map((i) => i.name)).toEqual(["École-maternelle.png", "cour d'école.jpg"]);
    expect(searchMyImages(all, "VACHE").map((i) => i.name)).toEqual(["vache.svg"]);
    expect(searchMyImages(all, "")).toHaveLength(3);
  });

  it("finds by theme, counts themes and filters by them", () => {
    const all = [img("vache.png", ["Animaux", "Ferme"]), img("citrouille.png", ["Automne"]), img("logo.png")];
    expect(searchMyImages(all, "ferme").map((i) => i.name)).toEqual(["vache.png"]);
    expect(themeCounts(all)).toEqual([{ theme: "Animaux", count: 1 }, { theme: "Automne", count: 1 }, { theme: "Ferme", count: 1 }]);
    expect(filterByTheme(all, { kind: "theme", theme: "Automne" }).map((i) => i.name)).toEqual(["citrouille.png"]);
    expect(filterByTheme(all, { kind: "none" }).map((i) => i.name)).toEqual(["logo.png"]);
  });

  it("never keeps a photo that shows children of the class", () => {
    const photo = (id: string, students: string[]) => ({ id, students, tags: [] }) as unknown as import("./types").ClassPhoto;
    const library = [photo("ferme", ["léa"]), photo("paysage", [])];
    expect(showsChildren("ferme", library, {})).toBe(true);
    expect(showsChildren("paysage", library, {})).toBe(false);
    expect(showsChildren("portrait", library, { portrait: {} })).toBe(true);
    expect(showsChildren("vache", library, {})).toBe(false);
  });
});
