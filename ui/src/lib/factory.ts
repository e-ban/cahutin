import { sizeIn } from "./pageFormat";
import { settings } from "./settings.svelte";
import type { Document, Element, Frame, Page, ShapeKind, Stroke, TextStyle } from "./types";

export const newId = () => crypto.randomUUID();

/** Page vierge, au format choisi dans les Préférences (A4 en portrait par défaut). */
export function newPage(): Page {
  return { id: newId(), size: sizeIn(settings.pageFormat, settings.pageLandscape), elements: [], grid: null };
}

export function newDocument(title = ""): Document {
  return { format_version: 1, title, pages: [newPage()], assets: {}, pending_photos: [] };
}

export function defaultTextStyle(): TextStyle {
  return { font: settings.textFont, size_pt: 14, color: "#222222", align: "Left" };
}

/** Cadre centré sur la page ; si un élément y est déjà centré, décalé de 8 mm en diagonale (autant de fois
 *  qu'il faut, sans sortir de la page) : deux ajouts de suite ne se recouvrent pas exactement. */
export function centeredFrame(page: Page, w: number, h: number): Frame {
  const STEP = 8;
  const centre = (f: Frame) => [f.x_mm + f.w_mm / 2, f.y_mm + f.h_mm / 2];
  let [cx, cy] = [page.size.width_mm / 2, page.size.height_mm / 2];
  const taken = (x: number, y: number) => page.elements.some((e) => Math.hypot(centre(e.frame)[0] - x, centre(e.frame)[1] - y) < 1);
  while (taken(cx, cy) && cx + w / 2 + STEP <= page.size.width_mm && cy + h / 2 + STEP <= page.size.height_mm) {
    cx += STEP;
    cy += STEP;
  }
  return { x_mm: cx - w / 2, y_mm: cy - h / 2, w_mm: w, h_mm: h, rotation_deg: 0 };
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

/** Zone photo à remplir (modèle), libre, au centre de la page. */
export function photoZoneElement(page: Page): Element {
  return {
    id: newId(),
    frame: centeredFrame(page, 80, 60),
    locked: false,
    zone: true,
    kind: { type: "Photo", asset: null, crop: { scale: 1, center_x: 0.5, center_y: 0.5 }, corner_radius_mm: 3, border: null, shadow: false },
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
  // Tracé libre : posé par les lignes de graphisme et les chemins, qui donnent son tracé.
  Path: { w: 120, h: 40, fill: null, stroke: { width_mm: 1.2, color: "#555555", style: "Dotted" } },
};

/** Nouvelle forme centrée sur la page (ou sur le point donné). */
export function shapeElement(page: Page, shape: ShapeKind, corner_radius_mm = 0, at?: { x_mm: number; y_mm: number }): Element {
  const d = SHAPE_DEFAULTS[shape];
  const frame = at ? { x_mm: Math.round(at.x_mm - d.w / 2), y_mm: Math.round(at.y_mm - d.h / 2), w_mm: d.w, h_mm: d.h, rotation_deg: 0 } : centeredFrame(page, d.w, d.h);
  return { id: newId(), frame, locked: false, zone: false, kind: { type: "Shape", shape, fill: d.fill, stroke: d.stroke && { ...d.stroke }, corner_radius_mm } };
}
