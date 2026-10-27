import { newDocument, newId, textElement } from "./factory";
import type { Document } from "./types";

/** Document de démonstration affiché au démarrage, en attendant le choix d'un modèle. */
export function sampleDocument(): Document {
  const doc = newDocument("Exemple");
  const title = textElement({ x_mm: 10, y_mm: 8, w_mm: 190, h_mm: 18, rotation_deg: 0 });
  if (title.kind.type === "Text") {
    title.kind.content = { paragraphs: [{ align: "Center", runs: [{ text: "La sortie à la ferme", bold: false, italic: false, underline: false, color: "#c0392b", font: "Playwrite FR Moderne", size_pt: 24 }] }] };
  }
  doc.pages[0].elements = [
    { id: newId(), frame: { x_mm: 0, y_mm: 0, w_mm: 210, h_mm: 30, rotation_deg: 0 }, locked: true, zone: false, kind: { type: "Shape", shape: "Rect", fill: "#f6e3c5", stroke: null, corner_radius_mm: 0 } },
    title,
    { id: newId(), frame: { x_mm: 10, y_mm: 40, w_mm: 120, h_mm: 90, rotation_deg: 0 }, locked: false, zone: true, kind: { type: "Photo", asset: null, crop: { scale: 1, center_x: 0.5, center_y: 0.5 }, corner_radius_mm: 0, border: null, shadow: false } },
    textElement({ x_mm: 10, y_mm: 140, w_mm: 190, h_mm: 40, rotation_deg: 0 }, true),
  ];
  return doc;
}
