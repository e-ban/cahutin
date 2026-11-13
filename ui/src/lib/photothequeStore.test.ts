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
const lib = (ids: string[], migration_settled = false): Phototheque => ({ format_version: 2, folders: [{ path: "/Images/Sortie", missing: false }], photos: ids.map(photo), migration_settled });
const scans: { send: (e: ScanEvent) => void; finish: () => void }[] = [];

vi.mock("./api", () => ({
  photothequeOpen: async () => lib(["a"]),
  photothequeSettleMigration: async () => lib(["a"], true),
  photothequeUnfollow: async () => ({ format_version: 2, folders: [], photos: [], migration_settled: false }),
  photothequeRefollow: vi.fn(async () => lib(["a"])),
  // Étiqueter et retirer ne renvoient que les photos changées.
  photothequeTag: vi.fn(async (ids: string[]) => ids.map((id) => ({ ...photo(id), tags: ["Ferme"] }))),
  photothequeHide: vi.fn(async (ids: string[], hidden: boolean) => ids.map((id) => ({ ...photo(id), hidden }))),
  photothequeScan: (onEvent: (e: ScanEvent) => void) =>
    new Promise((resolve) => scans.push({ send: onEvent, finish: () => resolve({ added: 0, moved: 0, missing: 0, missing_folders: [] }) })),
}));

vi.mock("./ask", () => ({ ask: async () => true }));

const { classLibrary } = await import("./phototheque.svelte");
const { editor } = await import("./docStore.svelte");
const { fr } = await import("./i18n/fr");
const api = await import("./api");
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

test("« Non merci » : la question de la migration est tranchée et le reste", async () => {
  await classLibrary.open("/classes/ms.cahutin");
  expect(classLibrary.migrationSettled).toBe(false);
  await classLibrary.settleMigration();
  expect(classLibrary.migrationSettled).toBe(true);
});

test("« Ne plus suivre », puis « Annuler » dans l'avis : le dossier et ses photos reviennent", async () => {
  await classLibrary.open("/classes/ms.cahutin");
  expect(await classLibrary.unfollow("/Images/Sortie")).toBe(true);
  expect(classLibrary.photos).toEqual([]);
  expect(editor.notice).toBe(fr.libraryUnfollowed("Sortie"));
  expect(editor.noticeAction?.label).toBe(fr.undo);
  editor.runNoticeAction();
  await tick();
  expect(api.photothequeRefollow).toHaveBeenCalledWith("/Images/Sortie");
  expect(classLibrary.photos.map((p) => p.id)).toEqual(["a"]);
  expect(classLibrary.folders.map((f) => f.path)).toEqual(["/Images/Sortie"]);
});

test("étiqueter et retirer : seules les photos changées reviennent, et rejoignent la liste à leur place", async () => {
  await classLibrary.open("/classes/ms.cahutin");
  scans.at(-1)!.send({ type: "Done", report: { added: 0, moved: 0, missing: 0, missing_folders: [] }, phototheque: lib(["a", "b", "c"]) });
  await classLibrary.tag(["b"], { add_tags: ["Ferme"] });
  expect(classLibrary.photos.map((p) => [p.id, p.tags])).toEqual([
    ["a", []],
    ["b", ["Ferme"]],
    ["c", []],
  ]);
  await classLibrary.hide(["c"]);
  expect(classLibrary.photos.map((p) => p.hidden)).toEqual([false, false, true]);
  expect(classLibrary.folders.map((f) => f.path)).toEqual(["/Images/Sortie"]);
  editor.runNoticeAction();
  await tick();
  expect(classLibrary.photos.map((p) => p.hidden)).toEqual([false, false, false]);
});

test("une réponse d'étiquette arrivée après le changement de classe ne touche pas la nouvelle liste", async () => {
  await classLibrary.open("/classes/ms.cahutin");
  let answer!: (photos: ClassPhoto[]) => void;
  vi.mocked(api.photothequeTag).mockImplementationOnce(() => new Promise((r) => (answer = r)));
  const tagging = classLibrary.tag(["a"], { add_tags: ["Ferme"] });
  await classLibrary.open("/classes/gs.cahutin");
  answer([{ ...photo("z"), tags: ["Ferme"] }]);
  await tagging;
  expect(classLibrary.photos.map((p) => p.id)).toEqual(["a"]);
});
