import { expect, test, vi } from "vitest";
import type { Document } from "./types";

const saved: Document[] = [];
vi.mock("./api", () => ({
  backupSave: async (doc: Document) => void saved.push(doc),
  backupClear: async () => {},
}));

/** Rien ne change avant « Accepter » : la copie de secours prend le document de départ, jamais la proposition. */
test("a backup during an assistant proposal copies the document, not the proposal", async () => {
  const { backupNow } = await import("./backup");
  const { editor } = await import("./docStore.svelte");
  const { newDocument } = await import("./factory");
  editor.reset(newDocument(), null);
  editor.commit({ ...editor.doc, title: "Sortie" });
  const base = editor.doc;
  editor.showProposal({ ...base, title: "Proposé par l'assistant" });
  await backupNow();
  expect(saved.map((d) => d.title)).toEqual(["Sortie"]);
  expect(saved[0]).toBe(base);
  editor.showProposal(null);
});
