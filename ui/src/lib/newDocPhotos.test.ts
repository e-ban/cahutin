import { beforeEach, expect, test, vi } from "vitest";
import type { Document } from "./types";

// Un document neuf (menu, accueil, exemple) n'hérite pas des photos du document quitté.
let example: Document | null = null;
vi.mock("./api", () => ({
  backupClear: async () => {},
  openExample: async () => example,
}));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn(), save: vi.fn(), ask: vi.fn() }));

const { editor } = await import("./docStore.svelte");
const { newDocument } = await import("./factory");
const { freshFromHome, newDoc, openExampleDoc } = await import("./fileActions");

const meta = (id: string) => ({ id, width_px: 40, height_px: 30, mime: "Jpeg" as const });

beforeEach(() => {
  editor.reset(newDocument(), null);
  editor.replaceLibrary([{ meta: meta("vieille"), path: "/photos/vieille.jpg", folder: "photos" }]);
  editor.carriedPending = ["/photos/attente.jpg"];
});

test("« Nouveau » vide le panneau des photos", async () => {
  await newDoc();
  expect(editor.photos).toEqual([]);
  expect(editor.carriedPending).toEqual([]);
});

test("une page vierge depuis l'accueil vide le panneau des photos", async () => {
  editor.welcomeOverDoc = true;
  expect(await freshFromHome()).toBe(true);
  expect(editor.photos).toEqual([]);
});

test("un exemple montre ses propres photos, pas celles du document quitté", async () => {
  const doc = newDocument();
  example = { ...doc, assets: { ...doc.assets, ferme: meta("ferme") } };
  await openExampleDoc("ferme");
  expect(editor.photos.map((p) => p.meta.id)).toEqual(["ferme"]);
});
