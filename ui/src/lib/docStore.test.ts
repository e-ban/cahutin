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

test("flushPending commits edits still held by an editor", () => {
  const stop = editor.onFlush(() => editor.commit({ ...editor.doc, title: "tapé mais pas validé" }));
  editor.flushPending();
  expect(editor.doc.title).toBe("tapé mais pas validé");
  expect(editor.dirty).toBe(true);
  stop();
  editor.reset(newDocument(), null);
  editor.flushPending();
  expect(editor.dirty).toBe(false);
});

test("markSaved records the written snapshot", () => {
  const written = { ...editor.doc, title: "écrit" };
  editor.commit(written);
  editor.commit({ ...editor.doc, title: "après" });
  editor.markSaved("/tmp/x.carnet", written);
  expect(editor.dirty).toBe(true);
});

test("updatePageById changes the page it names, whatever the current page", () => {
  const second = { ...editor.page, id: "p2" };
  editor.commit({ ...editor.doc, pages: [editor.page, second] });
  editor.pageIndex = 0;
  editor.updatePageById("p2", (p) => ({ ...p, size: { width_mm: 297, height_mm: 210 } }));
  expect(editor.doc.pages[1].size.width_mm).toBe(297);
  expect(editor.doc.pages[0].size.width_mm).toBe(210);
});

test("a slider preview ends in a single undo step", () => {
  const before = editor.doc;
  editor.preview({ ...editor.doc, title: "1" });
  editor.preview({ ...editor.doc, title: "12" });
  expect(editor.doc.title).toBe("12");
  editor.endPreview();
  expect(editor.doc.title).toBe("12");
  editor.undo();
  expect(editor.doc).toBe(before);
  expect(editor.canUndo).toBe(false);
});
