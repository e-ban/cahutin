// Repères de découpe autour des blocs (étiquettes, cartes) : traits aux coins, ou pointillés avec des
// ciseaux. Ce sont des formes ordinaires groupées avec le bloc : elles le suivent, s'exportent et
// s'impriment comme le reste. On les reconnaît à leur trait gris fin, pour les remplacer ou les retirer.
import { newId } from "./factory";
import { boundsOf, dissolveSmallGroups, expandSelection } from "./groups";
import type { Element, Page } from "./types";

export type CutStyle = "corners" | "dashed";

/** Écart entre le bloc et ses repères, longueur d'un trait de coin (mm). */
export const CUT_GAP_MM = 2;
export const CUT_LEN_MM = 5;
const CUT_COLOR = "#8a8a8a";
const CUT_WIDTH_MM = 0.3;
const SCISSORS = "2702";
const SCISSORS_MM = 5;
/** Hauteur du cadre d'un trait (zone de clic). */
const LINE_FRAME_MM = 2;

const stroke = (dashed: boolean) => ({ width_mm: CUT_WIDTH_MM, color: CUT_COLOR, style: dashed ? ("Dashed" as const) : null });

export function isCutMark(e: Element): boolean {
  const k = e.kind;
  if (k.type === "Image") return k.code === SCISSORS && e.frame.w_mm === SCISSORS_MM && e.frame.h_mm === SCISSORS_MM;
  return k.type === "Shape" && (k.shape === "Line" || k.shape === "Rect") && k.fill === null && k.stroke?.color === CUT_COLOR && k.stroke.width_mm === CUT_WIDTH_MM;
}

const shape = (group: string, frame: Element["frame"], kind: Element["kind"]): Element => ({ id: newId(), frame, locked: false, zone: false, group, kind });

/** Trait de (x1, y1) à (x2, y2), horizontal ou vertical. */
function line(group: string, x1: number, y1: number, x2: number, y2: number): Element {
  const len = Math.hypot(x2 - x1, y2 - y1);
  const [cx, cy] = [(x1 + x2) / 2, (y1 + y2) / 2];
  const frame = { x_mm: cx - len / 2, y_mm: cy - LINE_FRAME_MM / 2, w_mm: len, h_mm: LINE_FRAME_MM, rotation_deg: x1 === x2 ? 90 : 0 };
  return shape(group, frame, { type: "Shape", shape: "Line", fill: null, stroke: stroke(false), corner_radius_mm: 0 });
}

function marks(group: string, b: { x_mm: number; y_mm: number; w_mm: number; h_mm: number }, style: CutStyle): Element[] {
  const [g, l] = [CUT_GAP_MM, CUT_LEN_MM];
  const [x0, y0, x1, y1] = [b.x_mm, b.y_mm, b.x_mm + b.w_mm, b.y_mm + b.h_mm];
  if (style === "dashed") {
    const outline = { x_mm: x0 - g, y_mm: y0 - g, w_mm: b.w_mm + 2 * g, h_mm: b.h_mm + 2 * g, rotation_deg: 0 };
    const scissors = { x_mm: x0 + 2 * g, y_mm: y0 - g - SCISSORS_MM / 2, w_mm: SCISSORS_MM, h_mm: SCISSORS_MM, rotation_deg: 0 };
    return [
      shape(group, outline, { type: "Shape", shape: "Rect", fill: null, stroke: stroke(true), corner_radius_mm: 0 }),
      shape(group, scissors, { type: "Image", code: SCISSORS }),
    ];
  }
  // Pour chaque coin, deux traits dans le prolongement des bords, à l'écart du bloc.
  return [
    line(group, x0 - g - l, y0, x0 - g, y0),
    line(group, x0, y0 - g - l, x0, y0 - g),
    line(group, x1 + g, y0, x1 + g + l, y0),
    line(group, x1, y0 - g - l, x1, y0 - g),
    line(group, x0 - g - l, y1, x0 - g, y1),
    line(group, x0, y1 + g, x0, y1 + g + l),
    line(group, x1 + g, y1, x1 + g + l, y1),
    line(group, x1, y1 + g, x1, y1 + g + l),
  ];
}

/**
 * Pose (ou remplace) les repères de chaque bloc de la sélection : un groupe compte pour un bloc, un bloc
 * seul est groupé avec ses repères. `null` les retire. Les cases de grille sont ignorées.
 */
export function cutMarks(page: Page, ids: string[], style: CutStyle | null): Page {
  const inGrid = (id: string) => !!page.grid?.cells.some((c) => c.element_id === id);
  const selected = expandSelection(page, ids).filter((id) => !inGrid(id));
  const units = new Map<string, Element[]>();
  for (const e of page.elements) {
    if (!selected.includes(e.id) || isCutMark(e)) continue;
    const key = e.group ?? `seul:${e.id}`;
    units.set(key, [...(units.get(key) ?? []), e]);
  }
  let elements = page.elements;
  for (const members of units.values()) {
    const group = members[0].group ?? newId();
    const memberIds = new Set(members.map((m) => m.id));
    elements = elements.filter((e) => !(e.group && e.group === members[0].group && isCutMark(e)));
    elements = elements.map((e) => (memberIds.has(e.id) ? { ...e, group } : e));
    if (!style) continue;
    const last = Math.max(...elements.map((e, i) => (memberIds.has(e.id) ? i : -1)));
    const added = marks(group, boundsOf(members.map((m) => m.frame)), style);
    elements = [...elements.slice(0, last + 1), ...added, ...elements.slice(last + 1)];
  }
  return dissolveSmallGroups({ ...page, elements });
}
