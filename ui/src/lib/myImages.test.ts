import { describe, expect, it } from "vitest";
import { displayName, searchMyImages } from "./myImages";

const img = (name: string) => ({ name, size: 1, modified: 0 });

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
});
