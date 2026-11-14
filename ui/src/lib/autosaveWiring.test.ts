// L'enregistrement automatique branché sur l'éditeur : ce qui l'arrête, le départ sans question, Ctrl+S face à
// un fichier changé ailleurs, la copie de secours reprise.
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import type { Document, SaveMode } from "./types";

const writes: { path: string; title: string; mode: SaveMode; keepStart: boolean }[] = [];
let outcome: "Saved" | "ChangedElsewhere" | Error = "Saved";
let startVersion: string | null = null;
vi.mock("./api", () => ({
  saveDocument: async (path: string, doc: Document, mode: SaveMode, keepStart: boolean) => {
    if (outcome instanceof Error) throw outcome.message;
    if (outcome === "Saved") writes.push({ path, title: doc.title, mode, keepStart });
    return outcome;
  },
  startVersionInfo: async () => startVersion,
  startVersionOpen: async () => ({ doc: { ...editor.doc, title: "Début de séance" }, offered_fonts: [] }),
  backupClear: async () => {},
}));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn(), save: vi.fn(), ask: vi.fn() }));

const { editor } = await import("./docStore.svelte");
const { autosaver, autosave } = await import("./autosave.svelte");
const { confirmLeave, revertToStart, save } = await import("./fileActions");
const { settings } = await import("./settings.svelte");
const { modal } = await import("./modal.svelte");
const { newDocument } = await import("./factory");
const { AUTOSAVE_DELAY_MS } = await import("./autosave");
const { fr } = await import("./i18n/fr");

const PATH = "/docs/fiche.carnet";
const retitle = (title: string) => editor.commit({ ...editor.doc, title });
/** Un tour du minuteur assez tard après le dernier changement, puis les écritures finies. */
async function later() {
  vi.setSystemTime(Date.now() + AUTOSAVE_DELAY_MS + 1);
  autosaver.tick();
  for (let i = 0; i < 3; i++) await new Promise((r) => setTimeout(r, 0));
}

beforeEach(() => {
  vi.useFakeTimers({ toFake: ["Date"] });
  writes.length = 0;
  outcome = "Saved";
  settings.autosave = true;
  editor.reset(newDocument(), PATH, { own: true });
  autosaver.tick();
});
afterEach(() => {
  editor.editingText = null;
  editor.gesture = false;
  editor.cropping = null;
  editor.endPreview();
  editor.showProposal(null);
  editor.templateMode = false;
  startVersion = null;
  vi.useRealTimers();
});

test("a teacher's file is written a few seconds after a change, keeping the start version", async () => {
  retitle("Lundi");
  autosaver.tick();
  await later();
  expect(writes).toEqual([{ path: PATH, title: "Lundi", mode: "Auto", keepStart: true }]);
  expect(editor.dirty).toBe(false);
});

for (const [what, start] of [
  ["a text being written", () => (editor.editingText = "t1")],
  ["a gesture", () => (editor.gesture = true)],
  ["a crop", () => (editor.cropping = "p1")],
  ["a live preview (arrows, slider, paste)", () => editor.preview({ ...editor.doc, title: "Aperçu" })],
  ["an assistant proposal", () => editor.showProposal({ ...editor.doc, title: "Proposé" })],
] as const) {
  test(`nothing is written during ${what}`, async () => {
    retitle("Lundi");
    autosaver.tick();
    start();
    for (let i = 0; i < 10; i++) await later();
    expect(writes).toEqual([]);
  });
}

test("nothing is written for a copy recovered after a crash, an example or a new document", async () => {
  for (const reset of [() => editor.reset(newDocument(), PATH, { unsaved: true }), () => editor.reset(newDocument(), null)]) {
    reset();
    retitle("Repris");
    autosaver.tick();
    await later();
  }
  expect(writes).toEqual([]);
});

test("setting off: nothing written; leaving asks the old question; Ctrl+S writes as before", async () => {
  settings.autosave = false;
  retitle("Lundi");
  autosaver.tick();
  await later();
  expect(writes).toEqual([]);
  const leaving = confirmLeave();
  await vi.waitFor(() => expect(modal.message).not.toBeNull());
  modal.answer("cancel");
  expect(await leaving).toBe(false);
  expect(await save()).toBe(true);
  expect(writes).toEqual([{ path: PATH, title: "Lundi", mode: "Replace", keepStart: false }]);
});

test("leaving writes at once, without a question, the text being written included", async () => {
  retitle("Lundi");
  const stop = editor.onFlush(() => retitle("Lundi, texte fini"));
  expect(await confirmLeave()).toBe(true);
  stop();
  expect(modal.message).toBeNull();
  expect(writes.map((w) => w.title)).toEqual(["Lundi, texte fini"]);
});

test("leaving when the write fails: the old question comes back", async () => {
  outcome = new Error("Disque plein.");
  retitle("Lundi");
  const leaving = confirmLeave();
  await vi.waitFor(() => expect(modal.message).not.toBeNull());
  modal.answer("cancel");
  expect(await leaving).toBe(false);
  expect(autosave.problem).toEqual({ kind: "failed", message: "Disque plein." });
  expect(editor.dirty).toBe(true);
});

test("Ctrl+S on a file changed elsewhere asks first; « Remplacer par ma version » writes it", async () => {
  outcome = "ChangedElsewhere";
  retitle("Lundi");
  const saving = save();
  await vi.waitFor(() => expect(modal.message).not.toBeNull());
  outcome = "Saved";
  modal.answer("replace");
  expect(await saving).toBe(true);
  expect(writes).toEqual([{ path: PATH, title: "Lundi", mode: "Replace", keepStart: true }]);
});

test("the notice of a file changed elsewhere goes away once saved by hand", async () => {
  outcome = "ChangedElsewhere";
  retitle("Lundi");
  autosaver.tick();
  await later();
  expect(autosave.problem).toEqual({ kind: "changed" });
  expect(editor.notice).toBe(fr.autosaveChangedElsewhere);
  const saving = save();
  await vi.waitFor(() => expect(modal.message).not.toBeNull());
  outcome = "Saved";
  modal.answer("replace");
  expect(await saving).toBe(true);
  expect(autosave.problem).toBeNull();
  expect(editor.notice).toBeNull();
});

// Le mode modèle se joue sur la page du document, mais ce que l'enseignant y construit est un modèle : rien n'est
// écrit dans son fichier sans un geste de sa part.
test("template mode: nothing is written; leaving asks the old question", async () => {
  editor.templateMode = true;
  retitle("Grille du modèle");
  autosaver.tick();
  for (let i = 0; i < 10; i++) await later();
  expect(writes).toEqual([]);
  const leaving = confirmLeave();
  await vi.waitFor(() => expect(modal.message).not.toBeNull());
  modal.answer("cancel");
  expect(await leaving).toBe(false);
  expect(writes).toEqual([]);
});

test("the « Annuler » of the start version notice does nothing in another document", async () => {
  startVersion = "2026-11-14T08:00:00Z";
  await autosave.refreshStart();
  const reverting = revertToStart();
  await vi.waitFor(() => expect(modal.message).not.toBeNull());
  modal.answer("revert");
  await reverting;
  expect(editor.doc.title).toBe("Début de séance");
  const undo = editor.noticeAction!;
  editor.reset({ ...newDocument(), title: "Autre" }, "/docs/autre.carnet", { own: true });
  retitle("Autre, modifié");
  undo.run();
  expect(editor.doc.title).toBe("Autre, modifié");
});
