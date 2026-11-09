import { expect, test, vi } from "vitest";

const calls: string[] = [];
let finishWrite: () => void = () => {};
vi.mock("./api", () => ({
  backupSave: () => {
    calls.push("save");
    return new Promise<void>((done) => (finishWrite = done));
  },
  backupClear: async () => void calls.push("clear"),
}));

/** Une copie encore en cours quand le document est enregistré (ou quitté) ne doit pas survivre à l'effacement. */
test("a backup that lands after a save is cleared again, and only one runs at a time", async () => {
  const { backupNow } = await import("./backup");
  const { editor } = await import("./docStore.svelte");
  const { newDocument } = await import("./factory");
  editor.reset(newDocument(), null);
  editor.commit({ ...editor.doc, title: "Sortie" });
  const first = backupNow();
  void backupNow();
  expect(calls).toEqual(["save"]);
  editor.markSaved("/x/sortie.carnet");
  finishWrite();
  await first;
  expect(calls).toEqual(["save", "clear"]);
});
