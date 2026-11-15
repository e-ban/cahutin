import { describe, expect, it } from "vitest";
import { firstFitting, topBarHas, TOP_BAR_MAX, TOP_BAR_STEPS } from "./topBarFit";

describe("paliers de la barre du haut", () => {
  it("au palier 0, rien ne change", () => {
    for (const step of TOP_BAR_STEPS) expect(topBarHas(0, step)).toBe(false);
  });

  it("chaque palier garde ce que font les précédents", () => {
    for (let level = 1; level <= TOP_BAR_MAX; level++)
      for (const step of TOP_BAR_STEPS) if (topBarHas(level - 1, step)) expect(topBarHas(level, step)).toBe(true);
    for (const step of TOP_BAR_STEPS) expect(topBarHas(TOP_BAR_MAX, step)).toBe(true);
  });

  it("l'ordre voulu : écarts, nom, outils, « Imprimer », au plus juste, icône, la case « Automatique », puis « Enregistrer »", () => {
    expect(TOP_BAR_STEPS).toEqual(["gaps", "appName", "toolIcons", "shortOutput", "tight", "outputIcon", "autoBox", "saveIcon"]);
    // La case garde son mot tant qu'un autre pas reste possible avant elle.
    const auto = TOP_BAR_STEPS.indexOf("autoBox");
    for (const step of ["gaps", "appName", "toolIcons", "shortOutput", "tight", "outputIcon"] as const) expect(TOP_BAR_STEPS.indexOf(step)).toBeLessThan(auto);
    // « Enregistrer » garde son mot plus longtemps que tout le reste.
    expect(TOP_BAR_STEPS.at(-1)).toBe("saveIcon");
  });

  it("prend le premier palier où tout tient", () => {
    const tried: number[] = [];
    expect(firstFitting((l) => (tried.push(l), l >= 3))).toBe(3);
    expect(tried).toEqual([0, 1, 2, 3]);
    expect(firstFitting(() => true)).toBe(0);
  });

  it("s'arrête au dernier palier si rien ne tient", () => {
    const tried: number[] = [];
    expect(firstFitting((l) => (tried.push(l), false))).toBe(TOP_BAR_MAX);
    expect(tried).toHaveLength(TOP_BAR_MAX);
  });
});
