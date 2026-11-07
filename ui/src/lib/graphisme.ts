// Lignes de graphisme à repasser : un point vert de départ, le premier motif en trait plein (le modèle), la
// suite en pointillés gris, sur une ligne de base légère. Chaque ligne est un groupe de formes ; les motifs
// sont des tracés libres (forme « Path »), calculés en mm puis ramenés au carré 0..1000 de leur cadre.
// Copié pour l'assistant dans crates/cahutin-mcp/src/exercises.rs : changer les deux ensemble.
import { newId } from "./factory";
import type { Element, Frame, Page, ShapeKind, Stroke } from "./types";

/** Une forme posée dans `frame`. */
function shapeElement(shape: ShapeKind, frame: Frame, fill: string | null, stroke: Stroke | null, path?: string): Element {
  return { id: newId(), frame, locked: false, zone: false, kind: { type: "Shape", shape, fill, stroke, corner_radius_mm: 0, ...(path ? { path } : {}) } };
}

export type Motif = "ponts" | "coupes" | "vagues" | "boucles" | "creneaux" | "dents" | "traits" | "ronds";
export type Level = "PS" | "MS" | "GS";

export const MOTIFS: Motif[] = ["ponts", "coupes", "vagues", "boucles", "creneaux", "dents", "traits", "ronds"];

/** Hauteur d'un motif : grande pour les petits, plus fine en grande section. */
export const MOTIF_HEIGHT_MM: Record<Level, number> = { PS: 24, MS: 16, GS: 10 };

/** Largeur d'un motif (pas de répétition), en hauteurs. */
const STEP: Record<Motif, number> = { ponts: 1.1, coupes: 1.1, vagues: 1.6, boucles: 0.9, creneaux: 1.2, dents: 1, traits: 0.6, ronds: 1.15 };

const MARGIN_MM = 15;
/** Boucles : segments par boucle, et ampleur du tour (au-delà de 1, le tracé repasse sur lui-même). */
const LOOP_STEPS = 24;
const LOOP_K = 2.2;
const round2 = (n: number) => Math.round(n * 100) / 100;
const MODEL: Stroke = { width_mm: 1, color: "#333333", style: "Solid" };
const TRACE: Stroke = { width_mm: 1, color: "#9e9e9e", style: "Dotted" };
const START_COLOR = "#2e9d4a";

type Cmd = ["M" | "L", number, number] | ["C", number, number, number, number, number, number];

/** Le motif numéro `i` (à partir de 0), en mm : `x` son bord gauche, `h` sa hauteur, la ligne de base en `h`.
 *  Un motif continu (vagues, boucles, créneaux, dents) reprend où le précédent s'arrête. */
function motifCmds(m: Motif, x: number, w: number, h: number, first: boolean): Cmd[] {
  const k = (4 / 3) * h;
  switch (m) {
    case "ponts":
      return [["M", x, h], ["C", x, h - k, x + w * 0.85, h - k, x + w * 0.85, h]];
    case "coupes":
      return [["M", x, 0], ["C", x, k, x + w * 0.85, k, x + w * 0.85, 0]];
    case "vagues": {
      const mid = h / 2;
      const a = (4 / 3) * (h / 2);
      const start: Cmd[] = first ? [["M", x, mid]] : [];
      return [...start, ["C", x, mid - a, x + w / 2, mid - a, x + w / 2, mid], ["C", x + w / 2, mid + a, x + w, mid + a, x + w, mid]];
    }
    case "boucles": {
      // Trochoïde : le crayon avance en tournant, chaque tour fait une boucle (l l l) qui se croise vers le bas.
      const pts: Cmd[] = [];
      // Départ en bas (t = π), boucle en haut (t = 2π), retour en bas (t = 3π).
      for (let i = first ? 0 : 1; i <= LOOP_STEPS; i++) {
        const t = Math.PI + (i / LOOP_STEPS) * 2 * Math.PI;
        const u = (t - Math.PI - LOOP_K * Math.sin(t)) / (2 * Math.PI);
        const v = (1 - Math.cos(t)) / 2;
        pts.push([i === 0 ? "M" : "L", round2(x + u * w), round2(h * v)]);
      }
      return pts;
    }
    case "creneaux":
      return [...(first ? ([["M", x, h]] as Cmd[]) : []), ["L", x, 0], ["L", x + w / 2, 0], ["L", x + w / 2, h], ["L", x + w, h]];
    case "dents":
      return [...(first ? ([["M", x, h]] as Cmd[]) : []), ["L", x + w / 2, 0], ["L", x + w, h]];
    case "traits":
      return [["M", x + w / 2, 0], ["L", x + w / 2, h]];
    case "ronds": {
      const r = h * 0.45;
      const [cx, cy, c] = [x + w / 2, h / 2, r * 0.5523];
      return [
        ["M", cx, cy - r],
        ["C", cx + c, cy - r, cx + r, cy - c, cx + r, cy],
        ["C", cx + r, cy + c, cx + c, cy + r, cx, cy + r],
        ["C", cx - c, cy + r, cx - r, cy + c, cx - r, cy],
        ["C", cx - r, cy - c, cx - c, cy - r, cx, cy - r],
      ];
    }
  }
}

/** Les motifs `from` à `to` (exclu) d'une ligne, en mm, à partir de `x0`. */
function cmdsFor(m: Motif, x0: number, h: number, from: number, to: number): Cmd[] {
  const w = STEP[m] * h;
  return Array.from({ length: to - from }, (_, j) => motifCmds(m, x0 + (from + j) * w, w, h, j === 0)).flat();
}

/** Aperçu d'un motif pour le tiroir : trois motifs, en trait plein. */
export const motifPreview = (m: Motif) => pathInFrame(cmdsFor(m, 0, 10, 0, 3), 0);

/** Un tracé en mm, posé en `top` : son cadre (la boîte de ses points) et son tracé dans le carré 0..1000. */
export function pathInFrame(cmds: Cmd[], top: number): { frame: Frame; path: string } {
  const xs = cmds.flatMap((c) => c.slice(1).filter((_, i) => i % 2 === 0) as number[]);
  const ys = cmds.flatMap((c) => c.slice(1).filter((_, i) => i % 2 === 1) as number[]);
  const [x0, x1, y0, y1] = [Math.min(...xs), Math.max(...xs), Math.min(...ys), Math.max(...ys)];
  const [w, h] = [Math.max(x1 - x0, 0.5), Math.max(y1 - y0, 0.5)];
  const px = (x: number) => Math.round(((x - x0) / w) * 1000);
  const py = (y: number) => Math.round(((y - y0) / h) * 1000);
  const path = cmds.map(([op, ...v]) => [op, ...v.map((n, i) => (i % 2 === 0 ? px(n) : py(n)))].join(" ")).join(" ");
  const r = (n: number) => Math.round(n * 100) / 100;
  return { frame: { x_mm: r(x0), y_mm: r(top + y0), w_mm: r(w), h_mm: r(h), rotation_deg: 0 }, path };
}

function pathElement(cmds: Cmd[], top: number, stroke: Stroke, group: string): Element {
  const { frame, path } = pathInFrame(cmds, top);
  return { ...shapeElement("Path", frame, null, stroke, path), group };
}

/** Une ligne de graphisme de `width` mm à partir de (`x`, `y`) : point de départ, modèle, suite à repasser,
 *  ligne de base ; un seul groupe. */
export function graphismeRow(m: Motif, level: Level, x: number, y: number, width: number): Element[] {
  const h = MOTIF_HEIGHT_MM[level];
  const dot = h < 14 ? 2.5 : 3.5;
  const start = x + dot + 2;
  const count = Math.max(2, Math.floor((x + width - start) / (STEP[m] * h)));
  const group = newId();
  const base = shapeElement("Line", { x_mm: x, y_mm: y + h - 1, w_mm: width, h_mm: 2, rotation_deg: 0 }, null, { width_mm: 0.3, color: "#d0d0d0", style: "Solid" });
  // Le point de départ, à gauche du premier motif, à l'endroit où le crayon se pose.
  const firstY = m === "coupes" || m === "traits" || m === "ronds" ? 0 : m === "vagues" ? h / 2 : h;
  const point = shapeElement("Ellipse", { x_mm: x, y_mm: y + firstY - dot / 2, w_mm: dot, h_mm: dot, rotation_deg: 0 }, START_COLOR, null);
  return [
    { ...base, group },
    { ...point, group },
    pathElement(cmdsFor(m, start, h, 0, 1), y, MODEL, group),
    pathElement(cmdsFor(m, start, h, 1, count), y, TRACE, group),
  ];
}

/** Des lignes de graphisme sur la largeur de la page, sous ce qui y est déjà, autant que demandé et que la page
 *  en contient. */
export function graphismeRows(page: Page, m: Motif, level: Level, rows: number): Element[] {
  const h = MOTIF_HEIGHT_MM[level];
  const gap = Math.max(12, h);
  const below = Math.max(MARGIN_MM + 10, ...page.elements.map((e) => e.frame.y_mm + e.frame.h_mm + 8));
  const width = page.size.width_mm - 2 * MARGIN_MM;
  const out: Element[] = [];
  for (let i = 0, y = below; i < rows && y + h <= page.size.height_mm - MARGIN_MM; i++, y += h + gap) {
    out.push(...graphismeRow(m, level, MARGIN_MM, y, width));
  }
  return out;
}

/** Chemins en pointillés à étirer entre deux images (« relie le poussin à sa maman »). */
export const PATHS: { id: "descend" | "monte" | "vague" | "arc"; path: string; w_mm: number; h_mm: number }[] = [
  { id: "descend", path: "M 0 100 C 450 100 550 900 1000 900", w_mm: 120, h_mm: 50 },
  { id: "monte", path: "M 0 900 C 450 900 550 100 1000 100", w_mm: 120, h_mm: 50 },
  { id: "vague", path: "M 0 500 C 83 0 167 0 250 500 C 333 1000 417 1000 500 500 C 583 0 667 0 750 500 C 833 1000 917 1000 1000 500", w_mm: 140, h_mm: 30 },
  { id: "arc", path: "M 0 1000 C 0 -333 1000 -333 1000 1000", w_mm: 120, h_mm: 45 },
];

/** Un chemin en pointillés, centré sur `frame`. */
export function dottedPath(path: string, frame: Frame): Element {
  return shapeElement("Path", frame, null, { width_mm: 1.2, color: "#555555", style: "Dotted" }, path);
}
