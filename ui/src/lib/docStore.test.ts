import { beforeEach, expect, test, vi } from "vitest";
import { confirmDiscardIfDirty, editor } from "./docStore.svelte";
import { newDocument, photoZoneElement } from "./factory";
import { pendingPaths } from "./photoLibrary";
import { graphismeRows } from "./graphisme";
import { changeRows, rowsIn, rowsSettings } from "./graphismeEdit";

beforeEach(() => editor.reset(newDocument(), null));

/** Un tiroir de trois photos ; « a » est posée sur la page, « b » et « c » ne le sont pas. */
function drawerWithOnePlaced() {
  const meta = (id: string) => ({ id, width_px: 40, height_px: 30, mime: "Jpeg" as const });
  editor.replaceLibrary(["a", "b", "c"].map((id) => ({ meta: meta(id), path: `/photos/${id}.jpg`, folder: "Ferme" })));
  editor.carriedPending = ["/photos/b.jpg"];
  const box = photoZoneElement(editor.page);
  if (box.kind.type !== "Photo") throw new Error("case photo attendue");
  const placed = { ...box, kind: { ...box.kind, asset: "a" } };
  editor.updatePage((p) => ({ ...p, elements: [...p.elements, placed] }), [meta("a")]);
}

test("retirer les non placées garde les photos posées", () => {
  drawerWithOnePlaced();
  const gone = editor.removeUnusedPhotos();
  expect(gone.photos.map((p) => p.meta.id)).toEqual(["b", "c"]);
  expect(editor.photos.map((p) => p.meta.id)).toEqual(["a"]);
  expect(editor.carriedPending).toEqual([]);
  expect(pendingPaths(editor.photos, editor.doc, editor.carriedPending)).toEqual([]);
});

test("retirer les non placées oublie aussi celles encore en attente (clé USB absente)", () => {
  drawerWithOnePlaced();
  editor.carriedPending = ["/photos/b.jpg", "/usb/x.jpg"];
  const gone = editor.removeUnusedPhotos();
  expect(pendingPaths(editor.photos, editor.doc, editor.carriedPending)).toEqual([]);
  editor.restorePhotos(gone);
  expect(pendingPaths(editor.photos, editor.doc, editor.carriedPending)).toEqual(["/photos/b.jpg", "/photos/c.jpg", "/usb/x.jpg"]);
});

test("annuler le retrait remet les photos, et l'enregistrement garde leurs chemins", () => {
  drawerWithOnePlaced();
  editor.restorePhotos(editor.removeUnusedPhotos());
  expect(editor.photos.map((p) => p.meta.id).sort()).toEqual(["a", "b", "c"]);
  expect(pendingPaths(editor.photos, editor.doc, editor.carriedPending)).toEqual(["/photos/b.jpg", "/photos/c.jpg"]);
});

test("aller à une page oublie la sélection de la page quittée", () => {
  editor.commit({ ...editor.doc, pages: [...editor.doc.pages, { ...editor.doc.pages[0], id: "p2", elements: [] }] });
  editor.selection = ["quelque-chose"];
  editor.goToPage(2);
  expect(editor.pageIndex).toBe(1);
  expect(editor.selection).toEqual([]);
});

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

test("re-importing the same file does not duplicate it in the library", () => {
  const meta = (id: string) => ({ id, width_px: 10, height_px: 10, mime: "Jpeg" as const });
  editor.replaceLibrary([]);
  editor.addPhotos([{ meta: meta("a"), path: "/p/IMG_1.jpg", folder: "p" }]);
  editor.addPhotos([{ meta: meta("b"), path: "/p/IMG_1.jpg", folder: "p" }, { meta: meta("c"), path: "", folder: "" }]);
  expect(editor.photos.map((p) => p.meta.id)).toEqual(["a", "c"]);
  editor.replaceLibrary([{ meta: meta("d"), path: "", folder: "" }]);
  expect(editor.photos.map((p) => p.meta.id)).toEqual(["d"]);
});

test("a change that keeps the selected elements keeps the selection itself (open menus stay open)", async () => {
  const { textElement, newDocument } = await import("./factory");
  const doc = newDocument("s");
  const t = textElement({ x_mm: 0, y_mm: 0, w_mm: 10, h_mm: 10, rotation_deg: 0 });
  doc.pages[0].elements.push(t);
  editor.reset(doc, null);
  editor.selection = [t.id];
  const before = editor.selection;
  editor.updateElement(t.id, (e) => ({ ...e, frame: { ...e.frame, x_mm: 5 } }));
  expect(editor.selection).toBe(before);
  editor.updatePage((p) => ({ ...p, elements: [] }));
  expect(editor.selection).toEqual([]);
});

test("a photo removed from the drawer comes back with undo, on the page and in the drawer", () => {
  const meta = { id: "ph1", width_px: 40, height_px: 30, mime: "Jpeg" as const };
  const doc = newDocument("retrait");
  doc.assets = { ph1: meta };
  doc.pages[0].elements.push({
    id: "e1",
    frame: { x_mm: 0, y_mm: 0, w_mm: 40, h_mm: 30, rotation_deg: 0 },
    locked: false,
    zone: false,
    kind: { type: "Photo", asset: "ph1", crop: { scale: 1, center_x: 0.5, center_y: 0.5 }, corner_radius_mm: 0, border: null, shadow: false },
  });
  editor.reset(doc, null);
  editor.replaceLibrary([{ meta, path: "/p/ph1.jpg", folder: "p" }]);
  editor.removePhoto("ph1");
  expect(editor.photos).toHaveLength(0);
  expect(editor.page.elements[0].kind).toMatchObject({ asset: null });
  editor.undo();
  expect(editor.page.elements[0].kind).toMatchObject({ asset: "ph1" });
  expect(editor.photos.map((p) => p.meta.id)).toEqual(["ph1"]);
});

test("a notice can carry one action, dropped when another notice replaces it", () => {
  let ran = 0;
  editor.notify("Léa n'est plus dans la classe.", { label: "Remettre dans la classe", run: () => ran++ });
  expect(editor.notice).toBe("Léa n'est plus dans la classe.");
  expect(editor.noticeAction?.label).toBe("Remettre dans la classe");
  editor.noticeAction!.run();
  expect(ran).toBe(1);
  editor.notice = "Autre message";
  expect(editor.noticeAction).toBeNull();
  editor.notify("Sans action");
  expect(editor.noticeAction).toBeNull();
  editor.notice = null;
});

test("each change of placed handwriting rows is one undo step for all the rows", () => {
  editor.updatePage((p) => ({ ...p, elements: graphismeRows(p, "ponts", "MS", 4) }));
  const placed = editor.page;
  // Comme la barre contextuelle : un seul updatePage par choix, puis les lignes refaites restent choisies.
  const change = (c: Parameters<typeof changeRows>[2]) => {
    const bases = rowsIn(editor.page, editor.selection)!.map((r) => r.baseId);
    let fresh: string[] = [];
    editor.updatePage((p) => {
      const r = changeRows(p, bases, c);
      fresh = r.ids;
      return r.page;
    });
    editor.selection = fresh;
  };
  editor.selection = placed.elements.map((e) => e.id);
  change({ preset: "progressif" });
  change({ guides: "None" });
  expect(rowsSettings(rowsIn(editor.page, editor.selection)!)).toMatchObject({ preset: "progressif", guides: "None" });
  editor.undo();
  expect(editor.page).not.toEqual(placed);
  editor.undo();
  expect(editor.page).toEqual(placed);
});
