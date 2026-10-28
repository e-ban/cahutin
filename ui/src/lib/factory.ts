import type { Document, Element, Frame, Page, ShapeKind, Stroke, TextStyle } from "./types";

export const A4_PORTRAIT = { width_mm: 210, height_mm: 297 };

export const newId = () => crypto.randomUUID();

export function newPage(): Page {
  return { id: newId(), size: { ...A4_PORTRAIT }, elements: [], grid: null };
}

export function newDocument(title = ""): Document {
  return { format_version: 1, title, pages: [newPage()], assets: {}, pending_photos: [] };
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

/** Taille (mm), remplissage et trait par défaut de chaque forme. */
const SHAPE_DEFAULTS: Record<ShapeKind, { w: number; h: number; fill: string | null; stroke: Stroke | null }> = {
  Rect: { w: 60, h: 40, fill: "#f6e3c5", stroke: null },
  Ellipse: { w: 60, h: 40, fill: "#f6e3c5", stroke: null },
  Line: { w: 60, h: 8, fill: null, stroke: { width_mm: 1, color: "#333333" } },
  Arrow: { w: 60, h: 8, fill: null, stroke: { width_mm: 1, color: "#333333" } },
  Star: { w: 40, h: 40, fill: "#f1c40f", stroke: null },
  Heart: { w: 40, h: 40, fill: "#e74c3c", stroke: null },
  Bubble: { w: 70, h: 45, fill: "#ffffff", stroke: { width_mm: 0.5, color: "#333333" } },
  Cloud: { w: 70, h: 45, fill: "#ffffff", stroke: { width_mm: 0.5, color: "#333333" } },
};

/** Nouvelle forme centrée sur la page (ou sur le point donné). */
export function shapeElement(page: Page, shape: ShapeKind, corner_radius_mm = 0, at?: { x_mm: number; y_mm: number }): Element {
  const d = SHAPE_DEFAULTS[shape];
  const frame = at ? { x_mm: Math.round(at.x_mm - d.w / 2), y_mm: Math.round(at.y_mm - d.h / 2), w_mm: d.w, h_mm: d.h, rotation_deg: 0 } : centeredFrame(page, d.w, d.h);
  return { id: newId(), frame, locked: false, zone: false, kind: { type: "Shape", shape, fill: d.fill, stroke: d.stroke && { ...d.stroke }, corner_radius_mm } };
}
