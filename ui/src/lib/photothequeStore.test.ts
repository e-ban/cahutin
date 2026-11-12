import { expect, test, vi } from "vitest";
import type { ClassPhoto, Phototheque, ScanEvent } from "./types";

// Le moteur simulé : l'index enregistré, et des parcours qu'on termine à la main.
const photo = (id: string): ClassPhoto => ({
  id,
  width_px: 4,
  height_px: 3,
  mime: "Jpeg",
  taken: "2026-10-05",
  added: "2026-10-05",
  students: [],
  tags: [],
  path: `/Images/Sortie/IMG_${id}.jpg`,
  folder: "Sortie",
  size: 1,
  modified: 1,
  fingerprint: "0",
  hidden: false,
  missing: false,
});
const lib = (ids: string[]): Phototheque => ({ format_version: 2, folders: [{ path: "/Images/Sortie", missing: false }], photos: ids.map(photo) });
const scans: { send: (e: ScanEvent) => void; finish: () => void }[] = [];

vi.mock("./api", () => ({
  photothequeOpen: async () => lib(["a"]),
  photothequeScan: (onEvent: (e: ScanEvent) => void) =>
    new Promise((resolve) => scans.push({ send: onEvent, finish: () => resolve({ added: 0, moved: 0, missing: 0, missing_folders: [] }) })),
}));

const { classLibrary } = await import("./phototheque.svelte");
const tick = () => new Promise((r) => setTimeout(r, 0));

test("rouvrir la photothèque d'une classe déjà chargée relance un parcours, un seul à la fois", async () => {
  await classLibrary.open("/classes/ms.cahutin");
  expect(scans.length).toBe(1);
  // Page et tiroir ouverts ensemble pendant le parcours : pas de second parcours.
  classLibrary.refresh();
  classLibrary.refresh();
  expect(scans.length).toBe(1);
  scans[0].send({ type: "Done", report: { added: 0, moved: 0, missing: 0, missing_folders: [] }, phototheque: lib(["a"]) });
  scans[0].finish();
  await tick();
  expect(classLibrary.scanning).toBeNull();

  // La semaine suivante : de nouvelles photos dans le dossier, la page rouverte.
  classLibrary.refresh();
  classLibrary.refresh();
  expect(scans.length).toBe(2);
  expect(classLibrary.scanning).toEqual({ found: 0 });
  scans[1].send({ type: "Photos", photos: [photo("b")] });
  scans[1].send({ type: "Done", report: { added: 1, moved: 0, missing: 0, missing_folders: [] }, phototheque: lib(["a", "b"]) });
  scans[1].finish();
  await tick();
  expect(classLibrary.photos.map((p) => p.id)).toEqual(["a", "b"]);
  expect(classLibrary.scanning).toBeNull();
});
