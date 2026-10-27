import { beforeEach, expect, test, vi } from "vitest";
import { confirmDiscardIfDirty, editor } from "./docStore.svelte";
import { newDocument } from "./factory";

beforeEach(() => editor.reset(newDocument(), null));

test("isDirty", () => {
  expect(editor.dirty).toBe(false);
  editor.commit({ ...editor.doc, title: "Sortie" });
  expect(editor.dirty).toBe(true);
  editor.markSaved("/tmp/sortie.carnet");
  expect(editor.dirty).toBe(false);
  expect(editor.path).toBe("/tmp/sortie.carnet");
});

test("confirmDiscardIfDirty", async () => {
  const ask = vi.fn();
  expect(await confirmDiscardIfDirty(ask)).toBe(true);
  expect(ask).not.toHaveBeenCalled();

  editor.commit({ ...editor.doc, title: "Sortie" });
  expect(await confirmDiscardIfDirty(async () => "cancel")).toBe(false);
  expect(await confirmDiscardIfDirty(async () => "discard")).toBe(true);
  expect(await confirmDiscardIfDirty(async () => "save", async () => false)).toBe(false);
  expect(await confirmDiscardIfDirty(async () => "save", async () => true)).toBe(true);
});
