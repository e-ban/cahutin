import { expect, test } from "vitest";
import { createHistory } from "./history";

test("undo and redo", () => {
  const h = createHistory("init");
  h.commit("a");
  h.commit("b");
  h.undo();
  expect(h.current).toBe("a");
  h.redo();
  expect(h.current).toBe("b");
});

test("commit after undo clears redo", () => {
  const h = createHistory("init");
  h.commit("a");
  h.undo();
  h.commit("c");
  expect(h.canRedo).toBe(false);
  expect(h.current).toBe("c");
});

test("history keeps at most limit entries", () => {
  const h = createHistory(0, 3);
  for (let i = 1; i <= 5; i++) h.commit(i);
  let undos = 0;
  while (h.canUndo) {
    h.undo();
    undos++;
  }
  expect(undos).toBe(3);
  expect(h.current).toBe(2);
});

test("dirty tracking", () => {
  const h = createHistory("init");
  expect(h.isDirty).toBe(false);
  h.commit("a");
  h.markSaved();
  expect(h.isDirty).toBe(false);
  h.commit("b");
  expect(h.isDirty).toBe(true);
  h.undo();
  expect(h.isDirty).toBe(false);
});

test("markSaved with the snapshot actually written", () => {
  const h = createHistory("init");
  h.commit("a");
  const written = h.current;
  h.commit("b"); // modifié pendant l'enregistrement
  h.markSaved(written);
  expect(h.isDirty).toBe(true);
  h.undo();
  expect(h.isDirty).toBe(false);
});
