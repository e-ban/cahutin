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

test("une flèche maintenue ne compte que pour une étape d'annulation", async () => {
  const { editor } = await import("./docStore.svelte");
  const { handleShortcut, handleKeyUp } = await import("./shortcuts");
  const { newDocument, textElement } = await import("./factory");
  const doc = newDocument();
  const a = textElement({ x_mm: 0, y_mm: 0, w_mm: 10, h_mm: 10, rotation_deg: 0 });
  const b = textElement({ x_mm: 50, y_mm: 0, w_mm: 10, h_mm: 10, rotation_deg: 0 });
  doc.pages[0].elements.push(a, b);
  editor.reset(doc, null);
  const ev = (k: string, o: { ctrl?: boolean; repeat?: boolean } = {}) =>
    ({ key: k, ctrlKey: o.ctrl ?? false, metaKey: false, shiftKey: false, repeat: o.repeat ?? false, target: null, preventDefault() {} }) as unknown as KeyboardEvent;
  const x = (id: string) => editor.page.elements.find((e) => e.id === id)!.frame.x_mm;

  // Une rafale plus longue que l'historique (100 étapes), terminée par le relâchement de la touche.
  editor.selection = [a.id];
  for (let i = 0; i < 150; i++) handleShortcut(ev("ArrowRight", { repeat: i > 0 }));
  expect(x(a.id)).toBe(150);
  handleKeyUp(ev("ArrowRight"));
  expect(editor.dirty).toBe(true);
  editor.undo();
  expect(x(a.id)).toBe(0);
  expect(editor.canUndo).toBe(false);
  editor.redo();
  expect(x(a.id)).toBe(150);

  // Changer de sélection termine la rafale : chaque élément a sa propre étape.
  editor.selection = [a.id];
  handleShortcut(ev("ArrowLeft"));
  editor.selection = [b.id];
  handleShortcut(ev("ArrowRight"));
  handleShortcut(ev("ArrowRight", { repeat: true }));
  handleKeyUp(ev("ArrowRight"));
  editor.undo();
  expect([x(a.id), x(b.id)]).toEqual([149, 50]);
  editor.undo();
  expect([x(a.id), x(b.id)]).toEqual([150, 50]);

  // Une autre action (Ctrl+Z, touche encore enfoncée) termine la rafale avant d'agir.
  editor.selection = [b.id];
  handleShortcut(ev("ArrowRight"));
  handleShortcut(ev("ArrowRight", { repeat: true }));
  handleShortcut(ev("z", { ctrl: true }));
  expect(x(b.id)).toBe(50);
  editor.redo();
  expect(x(b.id)).toBe(52);
});

test("shortcutAction maps Ctrl+A to select all, even held down", () => {
  expect(shortcutAction(key("a"))).toBe("selectAll");
  expect(shortcutAction(key("A", { repeat: true }))).toBe("selectAll");
});

test("Ctrl+A choisit tous les éléments libres de la page, ou ceux du groupe où l'on est entré", async () => {
  const { editor } = await import("./docStore.svelte");
  const { modal } = await import("./modal.svelte");
  const { handleShortcut } = await import("./shortcuts");
  const { newDocument, textElement } = await import("./factory");
  const doc = newDocument();
  const frame = { x_mm: 0, y_mm: 0, w_mm: 10, h_mm: 10, rotation_deg: 0 };
  const fond = { ...textElement(frame), locked: true };
  const a = { ...textElement(frame), group: "g" };
  const b = { ...textElement(frame), group: "g" };
  const c = textElement(frame);
  doc.pages[0].elements.push(fond, a, b, c);
  editor.reset(doc, null);
  let prevented = false;
  const ctrlA = (target: unknown = null) =>
    handleShortcut({ key: "a", ctrlKey: true, metaKey: false, shiftKey: false, repeat: false, target, preventDefault: () => (prevented = true) } as unknown as KeyboardEvent);

  // L'élément verrouillé reste à part : avec lui, rien ne pourrait plus bouger.
  editor.selection = [];
  ctrlA();
  expect(prevented).toBe(true);
  expect(editor.selection).toEqual([a.id, b.id, c.id]);

  // Entré dans le groupe : ses éléments seulement.
  editor.insideGroup = ["g"];
  editor.focusedMember = a.id;
  editor.selection = [a.id];
  ctrlA();
  expect(editor.selection).toEqual([a.id, b.id]);
  expect(editor.focusedMember).toBeNull();
  expect(editor.inside).toEqual(["g"]);

  // Mode modèle : un élément fixé se règle, il est pris aussi.
  editor.insideGroup = [];
  editor.templateMode = true;
  ctrlA();
  expect(editor.selection).toEqual([fond.id, a.id, b.id, c.id]);
  editor.templateMode = false;

  // Dans un champ de saisie, ou sous une boîte de dialogue : rien.
  editor.selection = [];
  ctrlA({ tagName: "INPUT", isContentEditable: false });
  expect(editor.selection).toEqual([]);
  void modal.choose("Question ?", [{ value: "ok", label: "OK" }]);
  ctrlA();
  expect(editor.selection).toEqual([]);
  modal.answer("ok");
});

test("Échap sort d'un groupe, puis désélectionne", async () => {
  const { editor } = await import("./docStore.svelte");
  const { handleShortcut } = await import("./shortcuts");
  const { newDocument, textElement } = await import("./factory");
  const doc = newDocument();
  const frame = { x_mm: 0, y_mm: 0, w_mm: 10, h_mm: 10, rotation_deg: 0 };
  const a = { ...textElement(frame), group: "g" };
  const b = { ...textElement(frame), group: "g" };
  doc.pages[0].elements.push(a, b);
  editor.reset(doc, null);
  const esc = () => handleShortcut({ key: "Escape", ctrlKey: false, metaKey: false, shiftKey: false, repeat: false, target: null, preventDefault() {} } as unknown as KeyboardEvent);

  editor.insideGroup = ["g"];
  editor.focusedMember = a.id;
  editor.selection = [a.id];
  esc();
  expect(editor.selection).toEqual([a.id, b.id]);
  expect(editor.inside).toEqual([]);
  esc();
  expect(editor.selection).toEqual([]);
  expect(editor.insideGroup).toEqual([]);

  // Pendant un recadrage, Échap termine le recadrage seulement.
  editor.selection = [a.id];
  editor.cropping = a.id;
  esc();
  expect(editor.selection).toEqual([a.id]);
  editor.cropping = null;
});

test("une flèche ne sort pas un élément de la page", async () => {
  const { editor } = await import("./docStore.svelte");
  const { handleShortcut, handleKeyUp } = await import("./shortcuts");
  const { newDocument, textElement } = await import("./factory");
  const doc = newDocument();
  const a = textElement({ x_mm: 0, y_mm: 0, w_mm: 20, h_mm: 10, rotation_deg: 0 });
  doc.pages[0].elements.push(a);
  editor.reset(doc, null);
  const ev = (k: string) => ({ key: k, ctrlKey: false, metaKey: false, shiftKey: true, repeat: false, target: null, preventDefault() {} }) as unknown as KeyboardEvent;
  const x = () => editor.page.elements.find((e) => e.id === a.id)!.frame.x_mm;

  editor.selection = [a.id];
  for (let i = 0; i < 10; i++) handleShortcut(ev("ArrowLeft"));
  handleKeyUp(ev("ArrowLeft"));
  // Il reste 5 mm de l'élément sur la page.
  expect(x()).toBe(-15);
  const width = editor.page.size.width_mm;
  for (let i = 0; i < 100; i++) handleShortcut(ev("ArrowRight"));
  handleKeyUp(ev("ArrowRight"));
  expect(x()).toBe(width - 5);
});

test("Échap qui vient de fermer l'écriture d'un texte (zone à remplir) garde l'élément sélectionné", async () => {
  const { editor } = await import("./docStore.svelte");
  const { handleShortcut } = await import("./shortcuts");
  const { newDocument, textElement } = await import("./factory");
  const doc = newDocument();
  const frame = { x_mm: 0, y_mm: 0, w_mm: 10, h_mm: 10, rotation_deg: 0 };
  const zone = textElement(frame, true);
  const a = { ...textElement(frame), group: "g" };
  const b = { ...textElement(frame), group: "g" };
  doc.pages[0].elements.push(zone, a, b);
  editor.reset(doc, null);
  // L'éditeur a déjà traité la touche (et a pu quitter la page) : ce n'est plus une zone de saisie qui la reçoit.
  const esc = (handled: boolean) => handleShortcut({ key: "Escape", ctrlKey: false, metaKey: false, shiftKey: false, repeat: false, target: null, defaultPrevented: handled, preventDefault() {} } as unknown as KeyboardEvent);

  editor.selection = [zone.id];
  esc(true);
  expect(editor.selection).toEqual([zone.id]);
  esc(false);
  expect(editor.selection).toEqual([]);

  // Dans un groupe, de même : on sort de l'écriture, puis du groupe.
  editor.insideGroup = ["g"];
  editor.focusedMember = a.id;
  editor.selection = [a.id];
  esc(true);
  expect(editor.selection).toEqual([a.id]);
  expect(editor.focusedMember).toBe(a.id);
  esc(false);
  expect(editor.selection).toEqual([a.id, b.id]);
});
