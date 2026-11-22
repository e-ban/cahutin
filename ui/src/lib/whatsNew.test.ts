import { expect, test } from "vitest";
import { RELEASES, WHATS_NEW, shouldShowWhatsNew } from "./whatsNew";

// Captures présentes dans assets/nouveautes, servies à la racine (« /nouveautes/0.6/flou.webp »).
const onDisk = new Set(Object.keys(import.meta.glob("../../../assets/nouveautes/**/*.webp")).map((k) => k.replace("../../../assets", "")));

test("shown once to someone who already used Cahutin, never to a newcomer", () => {
  expect(shouldShowWhatsNew(null, true)).toBe(true);
  expect(shouldShowWhatsNew("0.5", true)).toBe(true);
  expect(shouldShowWhatsNew(WHATS_NEW.version, true)).toBe(false);
  expect(shouldShowWhatsNew(null, false)).toBe(false);
});

test("versions go from the newest to the oldest, each one once", () => {
  const parts = (v: string) => v.split(".").map(Number);
  const newer = (a: string, b: string) => {
    const [x, y] = [parts(a), parts(b)];
    for (let k = 0; k < Math.max(x.length, y.length); k++) if ((x[k] ?? 0) !== (y[k] ?? 0)) return (x[k] ?? 0) > (y[k] ?? 0);
    return false;
  };
  expect(WHATS_NEW).toBe(RELEASES[0]);
  for (let k = 1; k < RELEASES.length; k++) expect(newer(RELEASES[k - 1].version, RELEASES[k].version)).toBe(true);
});

// La capture est facultative (rien de visible, pas de capture) ; celle qui est donnée existe, à sa version.
test("each novelty has a title, a text, its own screenshot on disk if any, and no long dash", () => {
  for (const r of RELEASES) {
    expect(r.items.length).toBeGreaterThan(0);
    for (const n of r.items) {
      expect(n.title && n.text).toBeTruthy();
      if (n.image !== undefined) {
        expect(n.image.startsWith(`/nouveautes/${r.version}/`)).toBe(true);
        expect(onDisk.has(n.image)).toBe(true);
      }
      expect(n.title + n.text).not.toMatch(/[\u2013\u2014]/);
    }
  }
  const images = RELEASES.flatMap((r) => r.items.flatMap((n) => (n.image === undefined ? [] : [n.image])));
  expect(new Set(images).size).toBe(images.length);
});

test("version 0.20 opens with chapters", () => {
  expect(WHATS_NEW.version).toBe("0.20");
  expect(WHATS_NEW.items[0].image).toBe("/nouveautes/0.20/chapitres.webp");
});

// L'état de l'enregistrement se voit au point du bouton « Enregistrer » : plus rien ne s'affiche près du titre. La
// case « Automatique » est à côté de ce bouton.
test("the autosave novelty describes the real top bar", () => {
  const n = RELEASES.find((r) => r.version === "0.18")!.items.find((x) => x.image === "/nouveautes/0.18/enregistrement.webp")!;
  expect(n.text).toContain("« Automatique », à côté du bouton « Enregistrer »");
  expect(n.text).not.toMatch(/à côté du titre|« Enregistré »/);
});
