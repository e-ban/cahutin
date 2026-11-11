// Retrouver une ligne de graphisme posée et la refaire selon sa recette : changer ses cases, ses repères, sa
// flèche, son motif ou sa largeur. Une ligne déplacée ou recolorée reste réglable ; une ligne dissociée,
// entamée, tournée, retournée ou agrandie ne l'est plus. Refaire une ligne garde son groupe, sa place dans la
// page et les couleurs de son modèle et de ses pointillés.
import { DEFAULT_COLORS, MOTIFS, MOTIF_HEIGHT_MM, PRESETS, buildRow, cellsFor, cmdsFor, dotSize, motifCount, motifStep, pathInFrame, progressive, rowTop, type Level, type Motif, type Preset, type RowColors, type RowPreset } from "./graphisme";
import { expandSelection, pathOf, withPath } from "./groups";
import type { Cell, Element, Frame, GraphismeRow, Guides, Page } from "./types";

/** Une ligne réglable : sa ligne de base, sa recette, sa place (haut de la ligne) et ses éléments. */
export type RowRef = { baseId: string; recipe: GraphismeRow; x: number; y: number; width: number; ids: string[] };

/** Ce qu'on change sur des lignes ; ce qui n'est pas donné reste comme avant. */
export type RowChange = { preset?: Preset; guides?: Guides; arrow?: boolean; motif?: Motif };

const LEVELS: Level[] = ["PS", "MS", "GS"];
const samePath = (a: string[], b: string[]) => a.length === b.length && a.every((g, i) => g === b[i]);
const recipeOf = (e: Element) => (e.kind.type === "Shape" ? e.kind.graphisme ?? null : null);

/** Où commence le premier motif, en mm depuis le bord gauche de la ligne : après le point de départ. */
export const motifsStart = (level: Level): number => dotSize(MOTIF_HEIGHT_MM[level]) + 2;

/** La ligne réglable dont `base` est la ligne de base, ou `null` si elle ne l'est pas (ou plus). */
function rowFrom(page: Page, base: Element): RowRef | null {
  const r = recipeOf(base);
  const path = pathOf(base);
  if (!r || !path.length || !MOTIFS.includes(r.motif as Motif) || !LEVELS.includes(r.level as Level)) return null;
  if (base.frame.rotation_deg !== 0 || base.flip_h || base.flip_v || Math.abs(base.frame.h_mm - 2) > 0.01) return null;
  const members = page.elements.filter((e) => samePath(pathOf(e), path));
  const dot = members.find((e) => e.kind.type === "Shape" && e.kind.shape === "Ellipse");
  if (!dot || Math.abs(dot.frame.w_mm - dotSize(MOTIF_HEIGHT_MM[r.level as Level])) > 0.01) return null;
  // Un morceau effacé ou ajouté : ce n'est plus la ligne que la recette ferait.
  if (members.length !== buildRow(r, 0, 0, 100).length) return null;
  return { baseId: base.id, recipe: r, x: base.frame.x_mm, y: rowTop(base, r), width: base.frame.w_mm, ids: members.map((e) => e.id) };
}

function rowById(page: Page, baseId: string): RowRef | null {
  const base = page.elements.find((e) => e.id === baseId);
  return base ? rowFrom(page, base) : null;
}

/** Les lignes réglables touchées par la sélection, de haut en bas ; `null` si un élément sélectionné n'est
 *  dans aucune ligne réglable. */
export function rowsIn(page: Page, ids: string[], inside: string[] = []): RowRef[] | null {
  const rows = new Map<string, RowRef>();
  // Une ligne n'est relue qu'une fois, pour le premier de ses éléments sélectionnés.
  const seen = new Set<string>();
  for (const id of expandSelection(page, ids, inside)) {
    const e = page.elements.find((x) => x.id === id)!;
    const path = pathOf(e);
    if (path.length && seen.has(path.join("/"))) continue;
    const base = path.length ? page.elements.find((x) => recipeOf(x) && samePath(pathOf(x), path)) : undefined;
    const row = base && rowFrom(page, base);
    if (!row) return null;
    seen.add(path.join("/"));
    rows.set(row.baseId, row);
  }
  return [...rows.values()].sort((a, b) => a.y - b.y);
}

/** Les couleurs de la ligne : celles de son premier tracé plein et de son premier pointillé, sinon celles
 *  gardées dans la recette. La flèche de départ, posée juste après le point, est un tracé plein elle aussi :
 *  elle ne compte pas. */
function colorsOf(page: Page, row: RowRef): RowColors {
  const members = page.elements.filter((e) => row.ids.includes(e.id));
  const dot = members.findIndex((e) => e.kind.type === "Shape" && e.kind.shape === "Ellipse");
  const arrow = row.recipe.arrow ? members[dot + 1] : undefined;
  const color = (style: "Solid" | "Dotted") => {
    for (const e of members) {
      if (e !== arrow && e.kind.type === "Shape" && e.kind.shape === "Path" && e.kind.stroke?.style === style) return e.kind.stroke.color;
    }
    return null;
  };
  const r = row.recipe;
  return { model: color("Solid") ?? r.model_color ?? DEFAULT_COLORS.model, trace: color("Dotted") ?? r.trace_color ?? DEFAULT_COLORS.trace };
}

/** La ligne refaite avec `recipe`, à `x` sur `width` mm, à la place de l'ancienne dans la page. Une ligne
 *  verrouillée (ou zone du modèle) le reste. */
function rebuild(page: Page, row: RowRef, recipe: GraphismeRow, x = row.x, width = row.width): { page: Page; ids: string[] } {
  const base = page.elements.find((e) => e.id === row.baseId)!;
  const path = pathOf(base);
  const made = buildRow(recipe, x, row.y, width, colorsOf(page, row), path[0]).map((e) => ({ ...withPath(e, path), locked: base.locked, zone: base.zone }));
  const old = new Set(row.ids);
  const at = page.elements.findIndex((e) => old.has(e.id));
  const kept = page.elements.filter((e) => !old.has(e.id));
  const before = page.elements.slice(0, at).filter((e) => !old.has(e.id)).length;
  return { page: { ...page, elements: [...kept.slice(0, before), ...made, ...kept.slice(before)] }, ids: made.map((e) => e.id) };
}

/** Les cases ramenées à `n` : chaque position garde son état, les nouvelles prennent celui de la dernière. */
const fitCells = (cells: Cell[], n: number): Cell[] => Array.from({ length: n }, (_, i) => cells[Math.min(i, cells.length - 1)]);

/** Applique `change` aux lignes `baseIds` ; « progressif » les prend de haut en bas. */
export function changeRows(page: Page, baseIds: string[], change: RowChange): { page: Page; ids: string[] } {
  const rows = baseIds.map((id) => rowById(page, id)).filter((r): r is RowRef => r !== null).sort((a, b) => a.y - b.y);
  const ids: string[] = [];
  rows.forEach((row, i) => {
    const motif = change.motif ?? (row.recipe.motif as Motif);
    const n = motifCount(motif, row.recipe.level as Level, row.width);
    const preset = change.preset === "progressif" ? progressive(i, rows.length) : change.preset;
    const cells = preset ? cellsFor(preset, n) : fitCells(row.recipe.cells, n);
    const recipe: GraphismeRow = { ...row.recipe, motif, cells, guides: change.guides ?? row.recipe.guides, arrow: change.arrow ?? row.recipe.arrow };
    // La page a changé avec les lignes déjà refaites : relire la ligne avant de la refaire.
    const out = rebuild(page, rowById(page, row.baseId)!, recipe);
    page = out.page;
    ids.push(...out.ids);
  });
  return { page, ids };
}

/** Ce que les lignes ont en commun, `null` pour ce qui diffère d'une ligne à l'autre. Les pointillés se
 *  comparent motif par motif : sur une ligne courte, deux préréglages peuvent donner les mêmes cases. */
export function rowsSettings(rows: RowRef[]): { motif: Motif | null; preset: Preset | null; guides: Guides | null; arrow: boolean | null } {
  const common = <T>(f: (r: RowRef) => T): T | null => (rows.length && rows.every((r) => f(r) === f(rows[0])) ? f(rows[0]) : null);
  const gives = (r: RowRef, p: RowPreset) => cellsFor(p, r.recipe.cells.length).every((c, i) => c === r.recipe.cells[i]);
  const progressif = rows.length >= 2 && rows.every((r, i) => gives(r, progressive(i, rows.length)));
  const preset = progressif ? "progressif" : rows.length ? (PRESETS.find((p): p is RowPreset => p !== "progressif" && rows.every((r) => gives(r, p))) ?? null) : null;
  return { motif: common((r) => r.recipe.motif as Motif), preset, guides: common((r) => r.recipe.guides), arrow: common((r) => r.recipe.arrow) };
}

/** La ligne posée à `x` sur `width` mm : des motifs en plus ou en moins, toujours à la même taille. */
export function resizeRow(page: Page, baseId: string, x: number, width: number): { page: Page; ids: string[] } {
  const row = rowById(page, baseId);
  if (!row) return { page, ids: [] };
  const cells = fitCells(row.recipe.cells, motifCount(row.recipe.motif as Motif, row.recipe.level as Level, width));
  return rebuild(page, row, { ...row.recipe, cells }, x, width);
}

/** La place de la ligne étirée par un côté (`dir[0]` : -1 à gauche, 1 à droite) quand le cadre des poignées, `box`,
 *  passe à `w_mm` de large : le côté opposé ne bouge pas, et il reste la place de deux motifs. `null` si
 *  rien ne change. */
export function stretchRow(row: RowRef, box: { x_mm: number; w_mm: number }, w_mm: number, dir: number[]): { x: number; width: number } | null {
  const level = row.recipe.level as Level;
  const min = motifsStart(level) + 2 * motifStep(row.recipe.motif as Motif, level);
  // Le cadre des poignées peut déborder de la ligne (la flèche) : seul l'écart compte.
  const width = Math.max(min, Math.round((row.width + w_mm - box.w_mm) * 10) / 10);
  if (Math.abs(width - row.width) < 0.05) return null;
  return { x: dir[0] < 0 ? row.x + row.width - width : row.x, width };
}

/** Le tracé de chaque motif vide de la ligne, là où il serait dessiné (pour le montrer à l'écran). */
export function emptyCells(page: Page, baseId: string): { frame: Frame; path: string }[] {
  const row = rowById(page, baseId);
  if (!row) return [];
  const h = MOTIF_HEIGHT_MM[row.recipe.level as Level];
  const start = row.x + motifsStart(row.recipe.level as Level);
  return row.recipe.cells.flatMap((c, i) => (c === "Empty" ? [pathInFrame(cmdsFor(row.recipe.motif as Motif, start, h, i, i + 1), row.y)] : []));
}

/** Les motifs vides à montrer (à l'écran seulement) : ceux des lignes réglables sélectionnées ; rien si la
 *  sélection contient autre chose qu'une ligne réglable. `inside` : le groupe où l'on est entré. */
export function ghostsFor(page: Page, selection: string[], inside: string[] = []): { frame: Frame; path: string }[] {
  if (!selection.length) return [];
  return (rowsIn(page, selection, inside) ?? []).flatMap((r) => emptyCells(page, r.baseId));
}
