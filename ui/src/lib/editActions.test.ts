import { expect, test } from "vitest";
import { editor } from "./docStore.svelte";
import { copySelection, paste } from "./editActions";
import { newDocument } from "./factory";
import type { Document } from "./types";

test("a photo pasted into another document brings its image with it", () => {
  const meta = { id: "img1", width_px: 40, height_px: 30, mime: "Jpeg" as const };
  const doc: Document = { ...newDocument("source"), assets: { img1: meta } };
  doc.pages[0].elements.push({
    id: "ph",
    frame: { x_mm: 10, y_mm: 10, w_mm: 40, h_mm: 30, rotation_deg: 0 },
    locked: false,
    zone: false,
    kind: { type: "Photo", asset: "img1", crop: { scale: 1, center_x: 0.5, center_y: 0.5 }, corner_radius_mm: 0, border: null, shadow: false },
  });
  editor.reset(doc, null);
  editor.selection = ["ph"];
  copySelection();
  editor.reset(newDocument("autre"), null);
  paste();
  expect(editor.page.elements).toHaveLength(1);
  expect(editor.doc.assets.img1).toEqual(meta);
});
