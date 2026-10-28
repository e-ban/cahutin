import { expect, test } from "vitest";
import { shortcutAction } from "./shortcuts";

const key = (k: string, o: { shift?: boolean; repeat?: boolean; ctrl?: boolean } = {}) => ({
  key: k,
  ctrlKey: o.ctrl ?? true,
  metaKey: false,
  shiftKey: o.shift ?? false,
  repeat: o.repeat ?? false,
});

test("shortcutAction maps the file and edit shortcuts", () => {
  expect(shortcutAction(key("n"))).toBe("new");
  expect(shortcutAction(key("o"))).toBe("open");
  expect(shortcutAction(key("s"))).toBe("save");
  expect(shortcutAction(key("p"))).toBe("print");
  expect(shortcutAction(key("d"))).toBe("duplicate");
  expect(shortcutAction(key("z"))).toBe("undo");
  expect(shortcutAction(key("y"))).toBe("redo");
  expect(shortcutAction(key("Z", { shift: true }))).toBe("redo");
  expect(shortcutAction(key("c"))).toBe("copy");
  expect(shortcutAction(key("x"))).toBe("cut");
  expect(shortcutAction(key("v"))).toBe("paste");
});

test("shortcutAction ignores repeats and keys without Ctrl", () => {
  expect(shortcutAction(key("s", { repeat: true }))).toBeNull();
  expect(shortcutAction(key("s", { ctrl: false }))).toBeNull();
  expect(shortcutAction(key("q"))).toBeNull();
});
