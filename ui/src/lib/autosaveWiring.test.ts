// L'enregistrement automatique branché sur l'éditeur : ce qui l'arrête, le départ sans question, Ctrl+S face à
// un fichier changé ailleurs, la copie de secours reprise, la retenue après un remplissage par la classe.
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import type { Class, Document, Page, SaveMode } from "./types";

const writes: { path: string; title: string; mode: SaveMode; keepStart: boolean }[] = [];
let outcome: "Saved" | "ChangedElsewhere" | Error = "Saved";
let startVersion: string | null = null;
const backups: string[] = [];
/** Le remplissage par la classe, comme le moteur pour {classe} ; `fillGate` le fait attendre. */
let fillGate: Promise<void> | null = null;
vi.mock("./api", () => ({
  fillAutoFields: async (page: Page, _date: string | null, cls: Class | null) => {
    if (fillGate) await fillGate;
    const named = (t: string) => t.replace("{classe}", cls?.name ?? "{classe}");
    return { ...page, elements: page.elements.map((e) => (e.kind.type === "Text" ? { ...e, kind: { ...e.kind, content: { paragraphs: e.kind.content.paragraphs.map((q) => ({ ...q, runs: q.runs.map((r) => ({ ...r, text: named(r.text) })) })) } } } : e)) };
  },
  backupSave: async (doc: Document) => void backups.push(doc.title),
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
const { settings: initialSettings, AUTOSAVE_DEFAULT } = await import("./settings.svelte");
/** Installation neuve : rien n'est encore écrit sur le poste. */
const autosaveAtStart = initialSettings.autosave;
const { autosaver, autosave } = await import("./autosave.svelte");
const { confirmLeave, revertToStart, save } = await import("./fileActions");
const { settings } = await import("./settings.svelte");
const { modal } = await import("./modal.svelte");
const { newDocument } = await import("./factory");
const { AUTOSAVE_DELAY_MS } = await import("./autosave");
const { fr } = await import("./i18n/fr");
const { fillWaitingClassFields } = await import("./fieldPlacing.svelte");
const { textWith } = await import("./fieldEdit");
const { backupNow } = await import("./backup");

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
  fillGate = null;
  backups.length = 0;
  vi.useRealTimers();
});

const CLASS = { name: "Les Lutins" } as Class;
/** Un document enregistré dont une page attend le nom de la classe (un modèle). */
function openTemplateWithClassField() {
  const doc = newDocument();
  const page = doc.pages[0];
  editor.reset({ ...doc, title: "Modèle", pages: [{ ...page, elements: [textWith(page, "Classe : {classe}")] }] }, PATH, { own: true });
  autosaver.tick();
}
const pageText = () => JSON.stringify(editor.doc.pages[0].elements);

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

test("a fresh install: the setting is unchecked", () => {
  expect(AUTOSAVE_DEFAULT).toBe(false);
  expect(autosaveAtStart).toBe(false);
});

test("setting off: Ctrl+S keeps no start version of the session", async () => {
  settings.autosave = false;
  retitle("Lundi");
  expect(await save()).toBe(true);
  expect(writes.map((w) => w.keepStart)).toEqual([false]);
});

test("a class chosen fills the waiting fields: held until saved by hand, then written again on its own", async () => {
  openTemplateWithClassField();
  await fillWaitingClassFields(CLASS);
  expect(pageText()).toContain("Classe : Les Lutins");
  expect(editor.classHold?.on).toBe(true);
  for (let i = 0; i < 10; i++) await later();
  // Un changement ordinaire ensuite : retenu aussi (le document contient les données de la classe).
  retitle("Modèle rempli");
  autosaver.tick();
  for (let i = 0; i < 10; i++) await later();
  expect(writes).toEqual([]);
  // La copie de secours continue.
  await backupNow();
  expect(backups).toEqual(["Modèle rempli"]);
  // En partant : la question d'avant.
  const leaving = confirmLeave();
  await vi.waitFor(() => expect(modal.message).not.toBeNull());
  modal.answer("cancel");
  expect(await leaving).toBe(false);
  expect(writes).toEqual([]);
  // Ctrl+S : écrit, la retenue est levée, le changement suivant part tout seul.
  expect(await save()).toBe(true);
  expect(writes).toEqual([{ path: PATH, title: "Modèle rempli", mode: "Manual", keepStart: true }]);
  expect(editor.classHold).toBeNull();
  retitle("Mardi");
  autosaver.tick();
  await later();
  expect(writes.map((w) => [w.title, w.mode])).toEqual([["Modèle rempli", "Manual"], ["Mardi", "Auto"]]);
});

test("« Remplir avec ma classe » (pages generated for the class) is held the same way", async () => {
  editor.commitFromClass({ ...editor.doc, title: "Étiquettes de la classe" });
  for (let i = 0; i < 10; i++) await later();
  expect(writes).toEqual([]);
  expect(await autosaver.flush()).toBe(false);
});

test("the hold is kept when the setting is checked later", async () => {
  settings.autosave = false;
  editor.commitFromClass({ ...editor.doc, title: "Étiquettes de la classe" });
  settings.autosave = true;
  for (let i = 0; i < 10; i++) await later();
  expect(writes).toEqual([]);
});

test("Ctrl+Z back to before the fill lifts the hold; Ctrl+Y brings it back", async () => {
  retitle("Lundi");
  editor.commitFromClass({ ...editor.doc, title: "Étiquettes de la classe" });
  editor.undo();
  expect(editor.classHold?.on).toBe(false);
  autosaver.tick();
  await later();
  expect(writes.map((w) => w.title)).toEqual(["Lundi"]);
  editor.redo();
  expect(editor.classHold?.on).toBe(true);
  autosaver.tick();
  for (let i = 0; i < 10; i++) await later();
  expect(writes.map((w) => w.title)).toEqual(["Lundi"]);
});

test("another document starts without the hold", async () => {
  editor.commitFromClass({ ...editor.doc, title: "Étiquettes de la classe" });
  editor.reset({ ...newDocument(), title: "Autre" }, "/docs/autre.carnet", { own: true });
  expect(editor.classHold).toBeNull();
  retitle("Autre, modifié");
  autosaver.tick();
  await later();
  expect(writes.map((w) => w.title)).toEqual(["Autre, modifié"]);
});

// Au lancement, la dernière classe se rouvre (classes.restore()) pendant qu'un fichier passé à Cahutin s'ouvre :
// les champs du document ouvert s'écrivent avec la classe, sans geste de l'enseignant.
test("launch: the class restored after the file opened fills it, held, nothing written", async () => {
  openTemplateWithClassField();
  await fillWaitingClassFields(CLASS);
  expect(pageText()).toContain("Les Lutins");
  for (let i = 0; i < 10; i++) await later();
  expect(writes).toEqual([]);
});

test("launch: a fill still running when another document opens changes nothing there", async () => {
  openTemplateWithClassField();
  let release!: () => void;
  fillGate = new Promise((r) => (release = r));
  const filling = fillWaitingClassFields(CLASS);
  openTemplateWithClassField();
  const opened = editor.doc;
  release();
  await filling;
  expect(editor.doc).toBe(opened);
  expect(editor.classHold).toBeNull();
});
