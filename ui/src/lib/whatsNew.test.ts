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

test("each novelty has a title, a text, its own screenshot on disk, and no long dash", () => {
  expect(WHATS_NEW.items.length).toBeGreaterThanOrEqual(5);
  for (const r of RELEASES) {
    expect(r.items.length).toBeGreaterThan(0);
    for (const n of r.items) {
      expect(n.title && n.text).toBeTruthy();
      expect(n.image.startsWith(`/nouveautes/${r.version}/`)).toBe(true);
      expect(onDisk.has(n.image)).toBe(true);
      expect(n.title + n.text).not.toMatch(/[\u2013\u2014]/);
    }
  }
  const images = RELEASES.flatMap((r) => r.items.map((n) => n.image));
  expect(new Set(images).size).toBe(images.length);
});
