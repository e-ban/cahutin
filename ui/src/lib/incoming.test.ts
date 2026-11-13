import { beforeEach, expect, test, vi } from "vitest";

const imported: number[] = [];
vi.mock("./api", async (orig) => ({
  ...(await orig<typeof import("./api")>()),
  importPhotoBytes: async (bytes: Uint8Array) => {
    if (bytes[0] === 0) throw "fichier illisible ou abîmé";
    imported.push(bytes.length);
    return { id: `img${imported.length}`, width_px: 200, height_px: 100, mime: "Jpeg" as const };
  },
}));

const { armPaste, choosePaste, dropFiles, guardWindowDrop, handlePasteEvent, htmlText, isImageFile, pastedTextElement, pasteIncoming } = await import("./incoming");
const { plainTextOf } = await import("./systemClipboard");
const { editor } = await import("./docStore.svelte");
const { newDocument, textElement } = await import("./factory");
const { copySelection } = await import("./editActions");
const { fr } = await import("./i18n/fr");

const mark = { token: "t1", text: "Bonjour" };
const sys = (c: Partial<{ token: string; text: string; html: string; image: boolean }>) => ({ token: "", text: "", html: "", image: false, ...c });

beforeEach(() => {
  imported.length = 0;
  editor.reset(newDocument("d"), null);
  editor.notice = null;
});

test("Cahutin's own copy wins while the system clipboard still holds what Cahutin put there", () => {
  expect(choosePaste(sys({ token: "t1", text: "Bonjour" }), mark, true)).toBe("internal");
  // Le type propre à Cahutin perdu en route : le texte écrit suffit.
  expect(choosePaste(sys({ text: "Bonjour\r\n" }), mark, true)).toBe("internal");
  // Rien d'écrit (une photo copiée) : le presse-papiers vide est encore celui de Cahutin.
  expect(choosePaste(sys({}), { token: "t2", text: "" }, true)).toBe("internal");
});

test("anything copied elsewhere after Cahutin's copy wins", () => {
  expect(choosePaste(sys({ text: "Au revoir" }), mark, true)).toBe("text");
  expect(choosePaste(sys({ text: "Bonjour", image: true }), mark, true)).toBe("image");
  expect(choosePaste(sys({ image: true }), null, false)).toBe("image");
  expect(choosePaste(sys({ text: "Lundi" }), null, false)).toBe("text");
});

test("when Cahutin could not mark its copy, its own clipboard is kept as before", () => {
  expect(choosePaste(sys({ text: "Au revoir", image: true }), null, true)).toBe("internal");
});

test("text from an office program, also offered as a picture, is pasted as text", () => {
  expect(choosePaste(sys({ text: "Lundi", html: "<p>Lundi</p>", image: true }), null, false)).toBe("text");
  // Une image copiée d'un navigateur : son HTML contient l'image, ce sont ses pixels qui comptent.
  expect(choosePaste(sys({ text: "", html: '<img src="https://ex.fr/a.png">', image: true }), null, false)).toBe("image");
});

test("an image from the internet without its pixels is never fetched", () => {
  expect(choosePaste(sys({ html: '<meta charset="utf-8"><img src="https://ex.fr/chat.png">' }), null, false)).toBe("remoteImage");
  expect(choosePaste(sys({ html: '<img src="https://ex.fr/chat.png" alt=""> Le chat' }), null, false)).toBe("text");
  expect(choosePaste(sys({}), null, false)).toBe("nothing");
});

test("text is read from HTML without loading anything", () => {
  expect(htmlText("<style>p{color:red}</style><p>Le&nbsp;chat &amp; le chien</p><p>Fin</p>")).toBe("Le chat & le chien\nFin");
  expect(htmlText('<img src="https://ex.fr/a.png">')).toBe("");
});

test("only image files are accepted from the file explorer", () => {
  expect(isImageFile({ name: "Sortie.JPG", type: "" })).toBe(true);
  expect(isImageFile({ name: "dessin.svg", type: "image/svg+xml" })).toBe(true);
  expect(isImageFile({ name: "x", type: "image/webp" })).toBe(true);
  expect(isImageFile({ name: "lettre.docx", type: "application/vnd.openxmlformats" })).toBe(false);
  // Une photo d'iPhone va jusqu'à l'import, qui dit comment l'enregistrer en JPEG.
  expect(isImageFile({ name: "photo.HEIC", type: "" })).toBe(true);
});

test("a pasted text becomes a new text block, inside the page", () => {
  const page = editor.page;
  const el = pastedTextElement(page, "Ligne 1\r\nLigne 2\u0007");
  expect(el.kind.type === "Text" && el.kind.content.paragraphs[0].runs[0].text).toBe("Ligne 1\nLigne 2");
  expect(el.frame.x_mm).toBeGreaterThanOrEqual(0);
  expect(el.frame.x_mm + el.frame.w_mm).toBeLessThanOrEqual(page.size.width_mm);
  const long = pastedTextElement(page, "mot ".repeat(5000));
  expect(long.frame.y_mm + long.frame.h_mm).toBeLessThanOrEqual(page.size.height_mm);
});

test("each pasted text or image is one undo step", async () => {
  await pasteIncoming({ token: "", text: "Bonjour les enfants", html: "", image: null });
  expect(editor.page.elements).toHaveLength(1);
  await pasteIncoming({ token: "", text: "", html: "", image: new Blob([new Uint8Array([1, 2, 3])], { type: "image/png" }) });
  expect(editor.page.elements.map((e) => e.kind.type)).toEqual(["Text", "Photo"]);
  expect(editor.doc.assets.img1).toBeTruthy();
  editor.undo();
  expect(editor.page.elements.map((e) => e.kind.type)).toEqual(["Text"]);
  editor.undo();
  expect(editor.page.elements).toHaveLength(0);
});

test("nothing usable, or an internet image, gives a short message", async () => {
  await pasteIncoming({ token: "", text: "", html: '<img src="https://ex.fr/a.png">', image: null });
  expect(editor.notice).toBe(fr.pasteRemoteImage);
  editor.notice = null;
  await pasteIncoming({ token: "", text: "", html: "", image: null });
  expect(editor.notice).toBe(fr.pasteNothing);
  expect(editor.page.elements).toHaveLength(0);
});

test("image files dropped on the page are placed where they land, others are refused aloud", async () => {
  const file = (name: string, bytes: number[], type = "") => new File([new Uint8Array(bytes)], name, { type });
  await dropFiles([file("a.png", [1, 2], "image/png"), file("b.jpg", [1, 2, 3]), file("notes.txt", [1], "text/plain")], 50, 60);
  const photos = editor.page.elements.filter((e) => e.kind.type === "Photo");
  expect(photos).toHaveLength(2);
  expect(photos[0].frame.x_mm + photos[0].frame.w_mm / 2).toBeCloseTo(50);
  expect(photos[0].frame.y_mm + photos[0].frame.h_mm / 2).toBeCloseTo(60);
  expect(editor.notice).toBe(fr.dropNotImage(1));
  editor.undo();
  expect(editor.page.elements).toHaveLength(1);
  editor.notice = null;
  await dropFiles([file("abime.png", [0], "image/png")], 50, 60);
  expect(editor.notice).toBe(fr.imageNotPlaced("fichier illisible ou abîmé"));
});

test("a link dragged from a web page is not fetched: a message says why", async () => {
  await dropFiles([], 50, 60);
  expect(editor.notice).toBe(fr.dropFromWeb);
});

const clipboardEvent = (text: string) => {
  const ev = {
    prevented: false,
    preventDefault() {
      ev.prevented = true;
    },
    clipboardData: { items: [], getData: (t: string) => (t === "text/plain" ? text : "") },
  };
  return ev;
};

test("Ctrl+V pastes once from the paste event, which the page then keeps for itself", async () => {
  const ev = clipboardEvent("Bonjour");
  armPaste();
  handlePasteEvent(ev as unknown as ClipboardEvent);
  expect(ev.prevented).toBe(true);
  await vi.waitFor(() => expect(editor.page.elements).toHaveLength(1));
  await new Promise((r) => setTimeout(r, 5));
  expect(editor.page.elements).toHaveLength(1);
  // Sans Ctrl+V juste avant (une zone de saisie, une boîte ouverte), l'événement n'est pas pris.
  const other = clipboardEvent("Encore");
  handlePasteEvent(other as unknown as ClipboardEvent);
  expect(other.prevented).toBe(false);
  expect(editor.page.elements).toHaveLength(1);
});

test("Ctrl+V without a paste event from the web view reads the clipboard instead", async () => {
  vi.stubGlobal("navigator", { clipboard: { read: async () => [{ types: ["text/plain"], getType: async () => new Blob(["Du dehors"]) }] } });
  armPaste();
  await vi.waitFor(() => expect(editor.page.elements).toHaveLength(1));
  vi.unstubAllGlobals();
});

test("a file dropped beside the page is never opened by the web view", () => {
  const drag = (type: string, types: string[]) => {
    const ev = { type, dataTransfer: { types }, prevented: false, preventDefault: () => (ev.prevented = true) };
    return ev;
  };
  const over = drag("dragover", ["Files"]);
  guardWindowDrop(over as unknown as DragEvent);
  expect(over.prevented).toBe(true);
  expect(editor.notice).toBeNull();
  const drop = drag("drop", ["Files"]);
  guardWindowDrop(drop as unknown as DragEvent);
  expect(drop.prevented).toBe(true);
  expect(editor.notice).toBe(fr.dropOnPage);
  const inside = drag("drop", ["application/x-carnet-asset"]);
  guardWindowDrop(inside as unknown as DragEvent);
  expect(inside.prevented).toBe(false);
});

// En dernier : la copie interne reste d'un test à l'autre.
test("Cahutin's own copy is pasted when the system clipboard still holds its text", async () => {
  const a = textElement({ x_mm: 10, y_mm: 10, w_mm: 40, h_mm: 10, rotation_deg: 0 });
  if (a.kind.type === "Text") a.kind.content = { paragraphs: [{ align: null, runs: [{ text: "Coucou", bold: false, italic: false, underline: false, color: null, font: null, size_pt: null }] }] };
  const doc = newDocument("d");
  doc.pages[0].elements.push(a);
  editor.reset(doc, null);
  editor.selection = [a.id];
  copySelection();
  expect(plainTextOf(editor.page.elements)).toBe("Coucou");
  // Sans vue web, rien n'a pu être écrit dans le presse-papiers du système : la copie interne reste.
  await pasteIncoming({ token: "", text: "Autre chose", html: "", image: null });
  expect(editor.page.elements).toHaveLength(2);
  expect(editor.page.elements[1].frame.x_mm).toBe(a.frame.x_mm + 5);
});

test("« Coller » of the menu keeps Cahutin's copy without reading the system clipboard when it could not be marked", async () => {
  const read = vi.fn(async () => []);
  vi.stubGlobal("navigator", { clipboard: { read } });
  const { pasteFromMenu } = await import("./incoming");
  const before = editor.page.elements.length;
  await pasteFromMenu();
  expect(read).not.toHaveBeenCalled();
  expect(editor.page.elements).toHaveLength(before + 1);
  vi.unstubAllGlobals();
});
