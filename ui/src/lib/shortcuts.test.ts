import { expect, test } from "vitest";
import { shortcutAction, wheelZoomStep } from "./shortcuts";

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
  expect(shortcutAction(key("g"))).toBe("group");
  expect(shortcutAction(key("G", { shift: true }))).toBe("ungroup");
});

test("shortcutAction ignores repeats and keys without Ctrl", () => {
  expect(shortcutAction(key("s", { repeat: true }))).toBeNull();
  expect(shortcutAction(key("s", { ctrl: false }))).toBeNull();
  expect(shortcutAction(key("q"))).toBeNull();
});

test("shortcutAction maps Ctrl with plus, minus and zero to the zoom", () => {
  expect(shortcutAction(key("+", { shift: true }))).toBe("zoomIn");
  expect(shortcutAction(key("="))).toBe("zoomIn");
  expect(shortcutAction(key("-"))).toBe("zoomOut");
  expect(shortcutAction(key("-", { repeat: true }))).toBe("zoomOut");
  expect(shortcutAction(key("0"))).toBe("zoomFit");
  expect(shortcutAction(key("à"))).toBe("zoomFit");
  expect(shortcutAction(key("-", { ctrl: false }))).toBeNull();
});

test("wheelZoomStep zooms in on wheel up, out on wheel down, once per notch", () => {
  expect(wheelZoomStep(0, -100, 0)).toEqual({ acc: 0, dir: 1 });
  expect(wheelZoomStep(0, 100, 0)).toEqual({ acc: 0, dir: -1 });
  expect(wheelZoomStep(0, 3, 1)).toEqual({ acc: 0, dir: -1 });
});

test("wheelZoomStep gathers the small steps of a touchpad pinch", () => {
  let acc = 0;
  const dirs: number[] = [];
  for (let i = 0; i < 10; i++) {
    const s = wheelZoomStep(acc, -8, 0);
    acc = s.acc;
    dirs.push(s.dir);
  }
  expect(dirs.filter((d) => d === 1)).toHaveLength(2);
  expect(dirs.filter((d) => d === -1)).toHaveLength(0);
  // Un changement de sens repart de zéro.
  expect(wheelZoomStep(-30, 8, 0)).toEqual({ acc: 8, dir: 0 });
});

test("editing shortcuts do nothing under the welcome screen or a dialog", async () => {
  const { editor } = await import("./docStore.svelte");
  const { modal } = await import("./modal.svelte");
  const { handleShortcut } = await import("./shortcuts");
  const { newDocument, textElement } = await import("./factory");
  const doc = newDocument();
  const el = textElement({ x_mm: 0, y_mm: 0, w_mm: 10, h_mm: 10, rotation_deg: 0 });
  doc.pages[0].elements.push(el);
  editor.reset(doc, null);
  const press = (k: string) => handleShortcut({ key: k, ctrlKey: false, metaKey: false, shiftKey: false, repeat: false, target: null, preventDefault() {} } as unknown as KeyboardEvent);

  editor.selection = [el.id];
  editor.showWelcome = true;
  press("Delete");
  expect(editor.page.elements).toHaveLength(1);
  editor.showWelcome = false;

  void modal.choose("Question ?", [{ value: "ok", label: "OK" }]);
  editor.selection = [el.id];
  press("Delete");
  expect(editor.page.elements).toHaveLength(1);
  modal.answer("ok");

  // La vue agrandie du tiroir Images couvre la page : la page ne bouge pas.
  editor.openImagesWide();
  editor.selection = [el.id];
  press("Delete");
  expect(editor.page.elements).toHaveLength(1);
  editor.closeImagesWide();

  editor.selection = [el.id];
  press("Delete");
  expect(editor.page.elements).toHaveLength(0);
});
