import { expect, test, vi } from "vitest";

const calls: string[] = [];
vi.mock("./api", () => ({
  backupInfo: async () => ({ id: "fenetre-arretee", info: { source: null, title: "Sortie", saved_at: "2026-11-09T17:05:00Z" } }),
  backupDiscard: async (id: string) => void calls.push(`discard:${id}`),
  backupClear: async () => void calls.push("clear"),
}));

/** Échap (« plus tard ») ne touche à rien ; seul « Non, l'oublier » efface la copie de la fenêtre arrêtée. */
test("only « Non, l'oublier » deletes the copy left by a crash", async () => {
  const { offerBackup } = await import("./backup");
  const { modal } = await import("./modal.svelte");
  const later = offerBackup();
  await vi.waitFor(() => expect(modal.message).not.toBeNull());
  modal.cancel();
  expect(await later).toBe(false);
  expect(calls).toEqual([]);
  const forget = offerBackup();
  await vi.waitFor(() => expect(modal.message).not.toBeNull());
  modal.answer("discard");
  expect(await forget).toBe(false);
  expect(calls).toEqual(["discard:fenetre-arretee"]);
});
