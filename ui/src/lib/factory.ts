import type { Document, Element, Frame, Page, TextStyle } from "./types";

export const A4_PORTRAIT = { width_mm: 210, height_mm: 297 };

export const newId = () => crypto.randomUUID();

export function newPage(): Page {
  return { id: newId(), size: { ...A4_PORTRAIT }, elements: [], grid: null };
}

export function newDocument(title = ""): Document {
  return { format_version: 1, title, pages: [newPage()], assets: {} };
}

export function defaultTextStyle(): TextStyle {
  return { font: "Andika", size_pt: 14, color: "#222222", align: "Left" };
}

export function centeredFrame(page: Page, w: number, h: number): Frame {
  return { x_mm: (page.size.width_mm - w) / 2, y_mm: (page.size.height_mm - h) / 2, w_mm: w, h_mm: h, rotation_deg: 0 };
}

export function textElement(frame: Frame, zone = false): Element {
  return {
    id: newId(),
    frame,
    locked: false,
    zone,
    kind: { type: "Text", content: { paragraphs: [] }, style: defaultTextStyle(), prompt: zone ? "Texte" : null },
  };
}
