import { expect, test, vi } from "vitest";

const calls: string[] = [];
vi.mock("./api", () => ({
  classOpen: async (path: string) => ({ path, class: { name: "MS", year: "2026-2027", students: [], photos: {} } }),
  classSave: async () => void calls.push("save"),
  classTrash: async () => void calls.push("trash"),
  classList: async () => [],
}));

test("trash drops the pending save, so it cannot write the class back", async () => {
  const { classes } = await import("./classStore.svelte");
  await classes.open("/Classes/MS 2026-2027.cahutin-classe");
  classes.update((c) => ({ ...c, name: "Moyenne section" }));
  expect(await classes.trash()).toBe(true);
  await new Promise((r) => setTimeout(r, 700));
  expect(calls).toEqual(["trash"]);
  expect(classes.current).toBeNull();
  expect(classes.unsaved).toBe(false);
});
