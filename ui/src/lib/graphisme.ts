// Lignes de graphisme à repasser : un point vert de départ et sa flèche, puis des motifs en trait plein (le
// modèle), en pointillés gris (à repasser) ou laissés vides (à faire seul), sur une ligne de base légère.
// La ligne de base garde la recette de la ligne (motif, niveau, cases, repères, flèche) pour la refaire. Chaque ligne est un groupe de formes ; les motifs
// sont des tracés libres (forme « Path »), calculés en mm puis ramenés au carré 0..1000 de leur cadre.
// Copié pour l'assistant dans crates/cahutin-mcp/src/exercises.rs : changer les deux ensemble.
import { newId } from "./factory";
import type { Cell, Element, Frame, GraphismeRow, Guides, Page, ShapeKind, Stroke } from "./types";

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
const START_COLOR = "#2e9d4a";

/** Couleurs des motifs d'une ligne : le modèle en trait plein, la suite en pointillés. */
export type RowColors = { model: string; trace: string };
export const DEFAULT_COLORS: RowColors = { model: "#333333", trace: "#9e9e9e" };

/** Préréglages d'une ligne : combien de motifs en pointillés après le modèle ; « progressif » les fait
 *  passer de « tout » à « seul » d'une ligne à l'autre. */
export type Preset = "tout" | "moitie" | "quelques" | "seul" | "progressif";
export type RowPreset = Exclude<Preset, "progressif">;
export const PRESETS: Preset[] = ["tout", "moitie", "quelques", "seul", "progressif"];
const ROW_PRESETS: RowPreset[] = ["tout", "moitie", "quelques", "seul"];

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
      // Dans le sens de l'écriture : haut, gauche, bas, droite, haut.
      return [
        ["M", cx, cy - r],
        ["C", cx - c, cy - r, cx - r, cy - c, cx - r, cy],
        ["C", cx - r, cy + c, cx - c, cy + r, cx, cy + r],
        ["C", cx + c, cy + r, cx + r, cy + c, cx + r, cy],
        ["C", cx + r, cy - c, cx + c, cy - r, cx, cy - r],
      ];
    }
  }
}

/** Largeur d'un motif (son pas), en mm. */
export const motifStep = (m: Motif, level: Level): number => STEP[m] * MOTIF_HEIGHT_MM[level];

/** Les motifs `from` à `to` (exclu) d'une ligne, en mm, à partir de `x0`. */
export function cmdsFor(m: Motif, x0: number, h: number, from: number, to: number): Cmd[] {
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

type Pt = [number, number];
const dist = (a: Pt, b: Pt) => Math.hypot(a[0] - b[0], a[1] - b[1]);
const lengthOf = (c: Pt[]) => c.slice(1).reduce((s, q, i) => s + dist(q, c[i]), 0);

/** Les points d'un tracé en mm, chaque courbe prise en 16 points, chaque segment tous les 0,5 mm. */
function samples(cmds: Cmd[]): Pt[] {
  const out: Pt[] = [];
  let cur: Pt = [0, 0];
  for (const c of cmds) {
    if (c[0] === "M") {
      cur = [c[1], c[2]];
      out.push(cur);
    } else if (c[0] === "L") {
      const n = Math.max(1, Math.ceil(Math.hypot(c[1] - cur[0], c[2] - cur[1]) / 0.5));
      for (let i = 1; i <= n; i++) out.push([cur[0] + ((c[1] - cur[0]) * i) / n, cur[1] + ((c[2] - cur[1]) * i) / n]);
      cur = [c[1], c[2]];
    } else if (c[0] === "C") {
      const [, a, b, cc, d, e, f] = c;
      for (let i = 1; i <= 16; i++) {
        const t = i / 16, u = 1 - t;
        out.push([u * u * u * cur[0] + 3 * u * u * t * a + 3 * u * t * t * cc + t * t * t * e, u * u * u * cur[1] + 3 * u * u * t * b + 3 * u * t * t * d + t * t * t * f]);
      }
      cur = [e, f];
    }
  }
  return out;
}

/** La flèche de départ, en mm : un trait fin qui suit le début du geste à côté du tracé, et sa pointe ouverte.
 *  `first` : les points du premier motif, `near` : ceux des deux premiers (à ne pas toucher), `top` : le haut de la ligne. */
function startArrow(first: Pt[], near: Pt[], h: number, dot: number, top: number): Cmd[] {
  const p0 = first[0];
  const arm = Math.min(2.4, Math.max(1.6, 0.16 * h));
  const ang = Math.PI * 0.2;
  // L'écart entre un trait et les deux premiers motifs.
  const clear = (c: Pt[]) => Math.min(...c.map((a) => Math.min(...near.map((b) => dist(a, b)))));
  // Du côté gauche du geste ; pour un geste à plat, du côté loin du milieu du motif.
  const flip = (n: number[]) => (Math.abs(n[0]) > 0.3 ? n[0] > 0 : n[1] > 0 !== p0[1] > top + h / 2);
  // Le début du geste, de 0,8 mm après le départ jusqu'à `len` le long du tracé, décalé à côté de lui.
  const build = (len: number): { curve: Pt[]; cl: number } => {
    const path: Pt[] = [];
    let acc = 0;
    for (let i = 1; i < first.length; i++) {
      acc += dist(first[i], first[i - 1]);
      if (acc >= 0.8) path.push(first[i]);
      if (acc >= len) break;
    }
    const seg = [p0, ...path];
    const last = path[path.length - 1];
    const tl = Math.hypot(last[0] - p0[0], last[1] - p0[1]);
    const t = [(last[0] - p0[0]) / tl, (last[1] - p0[1]) / tl];
    const side = flip([t[1], -t[0]]) ? -1 : 1;
    const normalAt = (i: number) => {
      const [a, b] = [seg[Math.max(0, i - 1)], seg[Math.min(seg.length - 1, i + 1)]];
      const [dx, dy] = [b[0] - a[0], b[1] - a[1]];
      const l = Math.hypot(dx, dy) || 1;
      return [dy / l, -dx / l];
    };
    const offset = (s: number, off: number) =>
      path.map((p, j): Pt => {
        const n = normalAt(j + 1);
        return [p[0] + s * n[0] * off, p[1] + s * n[1] * off];
      });
    // Trop près du tracé : il s'écarte de 0,5 mm à la fois, 3 mm au plus, puis essaie l'autre côté.
    let best: Pt[] = [];
    let bestCl = -1;
    for (const s of [side, -side]) {
      for (let k = 0; k <= 6; k++) {
        const c = offset(s, dot / 2 + 1.5 + 0.5 * k);
        const cl = clear(c);
        if (cl > bestCl) [best, bestCl] = [c, cl];
        if (cl >= 1.8) break;
      }
      if (bestCl >= 1.8) break;
    }
    return { curve: best, cl: bestCl };
  };
  const base = Math.min(10, Math.max(6, 0.6 * h));
  let pick = build(base);
  // Trop court pour se voir derrière la pointe : un peu plus loin le long du tracé.
  for (const extra of [2, 4, 6]) {
    if (lengthOf(pick.curve) >= 2.5 * arm) break;
    const b = build(base + extra);
    if (b.cl >= 1.8 && lengthOf(b.curve) > lengthOf(pick.curve)) pick = b;
  }
  // Combien le trait tourne en tout.
  const turn = (c: Pt[]) => {
    let tot = 0;
    for (let i = 2; i < c.length; i++) {
      const a = Math.atan2(c[i - 1][1] - c[i - 2][1], c[i - 1][0] - c[i - 2][0]);
      const b = Math.atan2(c[i][1] - c[i - 1][1], c[i][0] - c[i - 1][0]);
      let d = b - a;
      while (d > Math.PI) d -= 2 * Math.PI;
      while (d < -Math.PI) d += 2 * Math.PI;
      tot += Math.abs(d);
    }
    return tot;
  };
  let curve = pick.curve;
  // Toujours trop court, ou le début tourne trop serré (boucles des petits motifs) : une flèche droite le long
  // du premier geste, vers le point du tracé à 0,35 hauteur du départ.
  if (lengthOf(curve) < 2.5 * arm || turn(curve) > Math.PI * 0.6) {
    let q = first[first.length - 1];
    let acc = 0;
    for (let i = 1; i < first.length; i++) {
      acc += dist(first[i], first[i - 1]);
      if (acc >= 0.35 * h) {
        q = first[i];
        break;
      }
    }
    const v = [q[0] - p0[0], q[1] - p0[1]];
    const vl = Math.hypot(v[0], v[1]);
    const chord = [v[0] / vl, v[1] / vl];
    let n = [chord[1], -chord[0]];
    if (flip(n)) n = [-n[0], -n[1]];
    const len = Math.min(8, Math.max(5, 0.45 * h));
    const line = (nn: number[], off: number): Pt[] => {
      const c = [p0[0] + nn[0] * off + chord[0] * (len / 2 + 0.5), p0[1] + nn[1] * off + chord[1] * (len / 2 + 0.5)];
      return Array.from({ length: 9 }, (_, i): Pt => [c[0] + chord[0] * len * (i / 8 - 0.5), c[1] + chord[1] * len * (i / 8 - 0.5)]);
    };
    let bestCl = -1;
    curve = line(n, dot / 2 + 2);
    for (const nn of [n, [-n[0], -n[1]]]) {
      for (let k = 0; k <= 6; k++) {
        const c = line(nn, dot / 2 + 2 + 0.5 * k);
        const cl = clear(c);
        if (cl > bestCl) [curve, bestCl] = [c, cl];
        if (cl >= 1.8) break;
      }
      if (bestCl >= 1.8) break;
    }
  }
  // La pointe ouverte, au bout du trait, dans le sens de sa fin.
  const e = curve[curve.length - 1];
  const d = curve[Math.max(0, curve.length - 3)];
  const dl = Math.hypot(e[0] - d[0], e[1] - d[1]);
  const back = [-(e[0] - d[0]) / dl, -(e[1] - d[1]) / dl];
  const rot = (a: number) => [back[0] * Math.cos(a) - back[1] * Math.sin(a), back[0] * Math.sin(a) + back[1] * Math.cos(a)];
  const [l1, l2] = [rot(ang), rot(-ang)];
  return [
    ["M", curve[0][0], curve[0][1]],
    ...curve.slice(1).map((p): Cmd => ["L", p[0], p[1]]),
    ["M", e[0] + l1[0] * arm, e[1] + l1[1] * arm],
    ["L", e[0], e[1]],
    ["L", e[0] + l2[0] * arm, e[1] + l2[1] * arm],
  ];
}

/** Taille du point de départ, selon la hauteur des motifs. */
export const dotSize = (h: number) => (h < 14 ? 2.5 : 3.5);

/** Nombre de motifs qui tiennent sur `width` mm, après le point de départ ; au moins 2. */
export function motifCount(m: Motif, level: Level, width: number): number {
  const h = MOTIF_HEIGHT_MM[level];
  return Math.max(2, Math.floor((width - dotSize(h) - 2) / (STEP[m] * h)));
}

/** Les cases d'une ligne de `n` motifs : le modèle, puis des pointillés, puis des cases vides. */
export function cellsFor(p: RowPreset, n: number): Cell[] {
  const dotted = { tout: n - 1, moitie: Math.floor((n - 1) / 2), quelques: n < 8 ? 2 : 3, seul: 0 }[p];
  const d = Math.min(dotted, n - 1);
  return Array.from({ length: n }, (_, i): Cell => (i === 0 ? "Model" : i <= d ? "Dotted" : "Empty"));
}

/** Préréglage de la ligne `row` sur `rows` : de « tout » à « seul ». */
export function progressive(row: number, rows: number): RowPreset {
  return rows <= 1 ? "tout" : ROW_PRESETS[Math.round((row * 3) / (rows - 1))];
}

/** Le préréglage qui donne ces cases, ou `null` si elles ont été réglées à la main. */
export function presetOf(cells: Cell[]): RowPreset | null {
  const same = (a: Cell[]) => a.length === cells.length && a.every((c, i) => c === cells[i]);
  return ROW_PRESETS.find((p) => same(cellsFor(p, cells.length))) ?? null;
}

/** Une ligne de graphisme de `width` mm à partir de (`x`, `y`), faite selon sa recette : ligne de base (qui
 *  porte la recette), ligne du haut si couloir, point de départ, flèche, puis un tracé par suite de motifs de
 *  même sorte ; un seul groupe. */
export function buildRow(r: GraphismeRow, x: number, y: number, width: number, colors: RowColors = DEFAULT_COLORS, group: string = newId()): Element[] {
  const m = r.motif as Motif;
  const h = MOTIF_HEIGHT_MM[r.level as Level];
  const dot = dotSize(h);
  const start = x + dot + 2;
  const line: Stroke | null = r.guides === "None" ? null : { width_mm: 0.3, color: "#d0d0d0", style: "Solid" };
  const lineAt = (top: number) => shapeElement("Line", { x_mm: x, y_mm: top, w_mm: width, h_mm: 2, rotation_deg: 0 }, null, line);
  const base = lineAt(y + h - 1);
  // La recette garde aussi les couleurs : une ligne sans pointillés se souvient de la leur.
  if (base.kind.type === "Shape") base.kind.graphisme = { ...r, model_color: colors.model, trace_color: colors.trace };
  const out: Element[] = [base];
  if (r.guides === "Corridor") out.push(lineAt(y - 1));
  // Le point de départ, juste où le crayon se pose : au début du modèle.
  const p = samples(cmdsFor(m, start, h, 0, 1)).map(([a, b]): Pt => [a, b + y]);
  const p0 = p[0];
  out.push(shapeElement("Ellipse", { x_mm: round2(p0[0] - dot / 2), y_mm: round2(p0[1] - dot / 2), w_mm: dot, h_mm: dot, rotation_deg: 0 }, START_COLOR, null));
  // La flèche, un trait fin à côté du point, suit le début du geste.
  if (r.arrow) {
    const near = samples(cmdsFor(m, start, h, 0, 2)).map(([a, b]): Pt => [a, b + y]);
    const { frame, path } = pathInFrame(startArrow(p, near, h, dot, y), 0);
    out.push(shapeElement("Path", frame, null, { width_mm: r.level === "PS" ? 0.8 : 0.7, color: START_COLOR, style: "Solid" }, path));
  }
  // Un tracé par suite de cases de même sorte ; les cases vides ne dessinent rien.
  for (let from = 0; from < r.cells.length; ) {
    const cell = r.cells[from];
    let to = from + 1;
    while (to < r.cells.length && r.cells[to] === cell) to++;
    if (cell === "Model") out.push(pathElement(cmdsFor(m, start, h, from, to), y, { width_mm: 1, color: colors.model, style: "Solid" }, group));
    if (cell === "Dotted") out.push(pathElement(cmdsFor(m, start, h, from, to), y, { width_mm: 1, color: colors.trace, style: "Dotted" }, group));
    from = to;
  }
  return out.map((e) => ({ ...e, group }));
}

/** Une ligne de graphisme de `width` mm à partir de (`x`, `y`), avec un préréglage : par défaut le modèle puis
 *  des pointillés jusqu'au bout, la ligne de base et la flèche. */
export function graphismeRow(m: Motif, level: Level, x: number, y: number, width: number, opts: { preset?: RowPreset; guides?: Guides; arrow?: boolean } = {}): Element[] {
  const cells = cellsFor(opts.preset ?? "tout", motifCount(m, level, width));
  return buildRow({ motif: m, level, cells, guides: opts.guides ?? "Base", arrow: opts.arrow ?? true }, x, y, width);
}

/** Des lignes de graphisme sur la largeur de la page, sous ce qui y est déjà, autant que demandé et que la page
 *  en contient. « Progressif » se répartit sur les lignes réellement posées. */
export function graphismeRows(page: Page, m: Motif, level: Level, rows: number, opts: { preset?: Preset; guides?: Guides; arrow?: boolean } = {}): Element[] {
  const h = MOTIF_HEIGHT_MM[level];
  const gap = Math.max(12, h);
  const below = Math.max(MARGIN_MM + 10, ...page.elements.map((e) => e.frame.y_mm + e.frame.h_mm + 8));
  const width = page.size.width_mm - 2 * MARGIN_MM;
  const ys: number[] = [];
  for (let i = 0, y = below; i < rows && y + h <= page.size.height_mm - MARGIN_MM; i++, y += h + gap) ys.push(y);
  return ys.flatMap((y, i) => {
    const preset = opts.preset === "progressif" ? progressive(i, ys.length) : opts.preset;
    return graphismeRow(m, level, MARGIN_MM, y, width, { ...opts, preset });
  });
}

/** Le haut d'une ligne (son `y`), retrouvé à partir de sa ligne de base. */
export function rowTop(base: Element, r: GraphismeRow): number {
  return base.frame.y_mm - MOTIF_HEIGHT_MM[r.level as Level] + 1;
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
