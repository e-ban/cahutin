// Retrouver une ligne de graphisme posée et la refaire selon sa recette : changer ses cases, ses repères, sa
// flèche, son motif ou sa largeur. Une ligne déplacée ou recolorée reste réglable ; une ligne dissociée,
// entamée, tournée, retournée ou agrandie ne l'est plus. Refaire une ligne garde son groupe, sa place dans la
// page et les couleurs de son modèle et de ses pointillés.
import { DEFAULT_COLORS, MOTIFS, MOTIF_HEIGHT_MM, buildRow, cellsFor, cmdsFor, dotSize, motifCount, motifStep, pathInFrame, progressive, rowTop, type Level, type Motif, type Preset, type RowColors } from "./graphisme";
import { expandSelection, pathOf, withPath } from "./groups";
import type { Cell, Element, Frame, GraphismeRow, Guides, Page } from "./types";

/** Une ligne réglable : sa ligne de base, sa recette, sa place (haut de la ligne) et ses éléments. */
export type RowRef = { baseId: string; recipe: GraphismeRow; x: number; y: number; width: number; ids: string[] };

/** Ce qu'on change sur des lignes ; ce qui n'est pas donné reste comme avant. */
export type RowChange = { preset?: Preset; guides?: Guides; arrow?: boolean; motif?: Motif };

const LEVELS: Level[] = ["PS", "MS", "GS"];
const NEXT: Record<Cell, Cell> = { Model: "Dotted", Dotted: "Empty", Empty: "Model" };
const samePath = (a: string[], b: string[]) => a.length === b.length && a.every((g, i) => g === b[i]);
const recipeOf = (e: Element) => (e.kind.type === "Shape" ? e.kind.graphisme ?? null : null);

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
  for (const id of expandSelection(page, ids, inside)) {
    const e = page.elements.find((x) => x.id === id)!;
    const path = pathOf(e);
    const base = path.length ? page.elements.find((x) => recipeOf(x) && samePath(pathOf(x), path)) : undefined;
    const row = base && rowFrom(page, base);
    if (!row) return null;
    rows.set(row.baseId, row);
  }
  return [...rows.values()].sort((a, b) => a.y - b.y);
}

/** Les couleurs de la ligne : celles de son premier tracé plein et de son premier pointillé. */
function colorsOf(page: Page, row: RowRef): RowColors {
  const color = (style: "Solid" | "Dotted") => {
    for (const e of page.elements) {
      if (row.ids.includes(e.id) && e.kind.type === "Shape" && e.kind.shape === "Path" && e.kind.stroke?.style === style) return e.kind.stroke.color;
    }
    return null;
  };
  return { model: color("Solid") ?? DEFAULT_COLORS.model, trace: color("Dotted") ?? DEFAULT_COLORS.trace };
}

/** La ligne refaite avec `recipe`, à `x` sur `width` mm, à la place de l'ancienne dans la page. */
function rebuild(page: Page, row: RowRef, recipe: GraphismeRow, x = row.x, width = row.width): { page: Page; ids: string[] } {
  const base = page.elements.find((e) => e.id === row.baseId)!;
  const path = pathOf(base);
  const made = buildRow(recipe, x, row.y, width, colorsOf(page, row), path[0]).map((e) => withPath(e, path));
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

/** La ligne posée à `x` sur `width` mm : des motifs en plus ou en moins, toujours à la même taille. */
export function resizeRow(page: Page, baseId: string, x: number, width: number): { page: Page; ids: string[] } {
  const row = rowById(page, baseId);
  if (!row) return { page, ids: [] };
  const cells = fitCells(row.recipe.cells, motifCount(row.recipe.motif as Motif, row.recipe.level as Level, width));
  return rebuild(page, row, { ...row.recipe, cells }, x, width);
}

/** Le numéro du motif de la ligne sous le point (`x_mm`, `y_mm`), ou `null` s'il n'y en a pas. */
export function cellAt(page: Page, baseId: string, x_mm: number, y_mm: number): number | null {
  const row = rowById(page, baseId);
  if (!row) return null;
  const level = row.recipe.level as Level;
  const h = MOTIF_HEIGHT_MM[level];
  // Du haut de la ligne à la ligne de base, avec 2 mm de marge.
  if (y_mm < row.y - 2 || y_mm > row.y + h + 2) return null;
  const start = row.x + dotSize(h) + 2;
  const i = Math.floor((x_mm - start) / motifStep(row.recipe.motif as Motif, level));
  return x_mm >= start && i < row.recipe.cells.length ? i : null;
}

/** Le motif `index` passe à l'état suivant : modèle, pointillés, vide, modèle. */
export function cycleCell(page: Page, baseId: string, index: number): { page: Page; ids: string[] } {
  const row = rowById(page, baseId);
  if (!row || index < 0 || index >= row.recipe.cells.length) return { page, ids: row?.ids ?? [] };
  const cells = row.recipe.cells.map((c, i) => (i === index ? NEXT[c] : c));
  return rebuild(page, row, { ...row.recipe, cells });
}

/** Le tracé de chaque motif vide de la ligne, là où il serait dessiné (pour le montrer à l'écran). */
export function emptyCells(page: Page, baseId: string): { frame: Frame; path: string }[] {
  const row = rowById(page, baseId);
  if (!row) return [];
  const h = MOTIF_HEIGHT_MM[row.recipe.level as Level];
  const start = row.x + dotSize(h) + 2;
  return row.recipe.cells.flatMap((c, i) => (c === "Empty" ? [pathInFrame(cmdsFor(row.recipe.motif as Motif, start, h, i, i + 1), row.y)] : []));
}
