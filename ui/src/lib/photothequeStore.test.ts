import { expect, test, vi } from "vitest";
import type { ClassPhoto, OriginalsProgress, OriginalsReport, Phototheque, ScanEvent } from "./types";

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
  photothequeSettleMigration: vi.fn(async () => lib(["a"], true)),
  photothequeFindOriginals: vi.fn(async (_onEvent: (p: OriginalsProgress) => void): Promise<OriginalsReport | null> => ({ matches: [{ copy: "a", original: "b" }], unmatched: [] })),
  photothequeStopOriginals: vi.fn(async () => {}),
  photothequeApplyOriginals: vi.fn(async () => ({ phototheque: lib(["a"], true), not_trashed: 0 })),
  photothequeUnfollow: async () => ({ format_version: 2, folders: [], photos: [], migration_settled: false }),
  photothequeRefollow: vi.fn(async () => lib(["a"])),
  // Étiqueter et retirer ne renvoient que les photos changées.
  photothequeTag: vi.fn(async (ids: string[]) => ids.map((id) => ({ ...photo(id), tags: ["Ferme"] }))),
  photothequeHide: vi.fn(async (ids: string[], hidden: boolean) => ids.map((id) => ({ ...photo(id), hidden }))),
  photothequeScan: (onEvent: (e: ScanEvent) => void) =>
    new Promise((resolve) => scans.push({ send: onEvent, finish: () => resolve({ added: 0, moved: 0, missing: 0, missing_folders: [] }) })),
}));

vi.mock("./ask", () => ({ ask: vi.fn(async () => true) }));

const { classLibrary } = await import("./phototheque.svelte");
const { editor } = await import("./docStore.svelte");
const { fr } = await import("./i18n/fr");
const api = await import("./api");
const { ask } = await import("./ask");
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

test("« Non » à la question de la corbeille : la migration est tranchée comme « Non merci »", async () => {
  await classLibrary.open("/classes/ms.cahutin");
  vi.mocked(api.photothequeSettleMigration).mockClear();
  vi.mocked(ask).mockResolvedValueOnce(false);
  await classLibrary.migrate();
  expect(ask).toHaveBeenLastCalledWith(fr.libraryOriginalsFound(1, 0), fr.yes, fr.no);
  expect(api.photothequeApplyOriginals).not.toHaveBeenCalled();
  expect(api.photothequeSettleMigration).toHaveBeenCalledTimes(1);
  expect(classLibrary.migrationSettled).toBe(true);
  expect(editor.notice).toBe(fr.libraryMigrationDeclined);
});

test("aucun original retrouvé : on le dit, et la question reste posée (le bandeau reste)", async () => {
  await classLibrary.open("/classes/ms.cahutin");
  vi.mocked(api.photothequeSettleMigration).mockClear();
  vi.mocked(api.photothequeFindOriginals).mockResolvedValueOnce({ matches: [], unmatched: ["a"] });
  await classLibrary.migrate();
  expect(api.photothequeSettleMigration).not.toHaveBeenCalled();
  expect(classLibrary.migrationSettled).toBe(false);
  expect(editor.notice).toBe(fr.libraryOriginalsNone);
});

test("la recherche des originaux dit où elle en est ; une seule à la fois", async () => {
  await classLibrary.open("/classes/ms.cahutin");
  let send!: (p: OriginalsProgress) => void;
  let answer!: (r: OriginalsReport | null) => void;
  vi.mocked(api.photothequeFindOriginals).mockClear();
  vi.mocked(api.photothequeFindOriginals).mockImplementationOnce((onEvent) => {
    send = onEvent;
    return new Promise((r) => (answer = r));
  });
  const search = classLibrary.migrate();
  expect(classLibrary.searching).toEqual({ seen: 0, total: 0 });
  send({ seen: 120, total: 500 });
  expect(classLibrary.searching).toEqual({ seen: 120, total: 500 });
  // Page fermée puis rouverte, ou lien de la colonne : pas de seconde recherche.
  await classLibrary.migrate();
  expect(api.photothequeFindOriginals).toHaveBeenCalledTimes(1);
  answer({ matches: [], unmatched: [] });
  await search;
  expect(classLibrary.searching).toBeNull();
});

test("changer de classe pendant la recherche des originaux l'arrête", async () => {
  await classLibrary.open("/classes/ms.cahutin");
  let answer!: (r: OriginalsReport | null) => void;
  vi.mocked(api.photothequeFindOriginals).mockImplementationOnce(() => new Promise((r) => (answer = r)));
  vi.mocked(api.photothequeStopOriginals).mockClear();
  vi.mocked(ask).mockClear();
  editor.notice = null;
  const search = classLibrary.migrate();
  await classLibrary.open("/classes/gs.cahutin");
  expect(api.photothequeStopOriginals).toHaveBeenCalledTimes(1);
  answer(null);
  await search;
  // Rien n'est dit ni demandé à la nouvelle classe.
  expect(classLibrary.searching).toBeNull();
  expect(ask).not.toHaveBeenCalled();
  expect(editor.notice).toBeNull();
});

test("« Arrêter » la recherche : rien n'est tranché ni demandé, et on le dit", async () => {
  await classLibrary.open("/classes/ms.cahutin");
  vi.mocked(api.photothequeSettleMigration).mockClear();
  vi.mocked(ask).mockClear();
  let answer!: (r: OriginalsReport | null) => void;
  vi.mocked(api.photothequeFindOriginals).mockImplementationOnce(() => new Promise((r) => (answer = r)));
  const search = classLibrary.migrate();
  await classLibrary.stopSearch();
  expect(api.photothequeStopOriginals).toHaveBeenCalledTimes(1);
  // Le moteur répond « arrêtée ».
  answer(null);
  await search;
  expect(classLibrary.searching).toBeNull();
  expect(ask).not.toHaveBeenCalled();
  expect(api.photothequeSettleMigration).not.toHaveBeenCalled();
  expect(editor.notice).toBe(fr.libraryOriginalsStopped);
});

test("le lien « Retrouver les originaux », question déjà tranchée : « Non » ne la retranche pas", async () => {
  await classLibrary.open("/classes/ms.cahutin");
  await classLibrary.settleMigration();
  vi.mocked(api.photothequeSettleMigration).mockClear();
  vi.mocked(ask).mockResolvedValueOnce(false);
  editor.notice = null;
  await classLibrary.migrate();
  expect(api.photothequeSettleMigration).not.toHaveBeenCalled();
  expect(classLibrary.migrationSettled).toBe(true);
  expect(editor.notice).toBeNull();
});

test("« Oui » : les copies vont à la corbeille et on dit combien", async () => {
  await classLibrary.open("/classes/ms.cahutin");
  await classLibrary.migrate();
  expect(api.photothequeApplyOriginals).toHaveBeenCalledWith([{ copy: "a", original: "b" }]);
  expect(editor.notice).toBe(fr.libraryOriginalsDone(1));
});
