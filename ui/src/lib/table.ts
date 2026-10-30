// Tableaux : un élément `Table` (grille et traits) et ses cases, éléments ordinaires du même groupe.
// Les opérations sont dans le moteur (carnet_core::table) ; ici, ce que l'affichage doit savoir tout de suite.
import type { Element, Frame, GridCell, Page, TableProps } from "./types";

type Segment = [number, number, number, number];

/** Bords des colonnes (ou des lignes) en mm, de `start` à `start + len`. */
function edges(sizes: number[], start: number, len: number): number[] {
  const total = sizes.reduce((a, b) => a + b, 0);
  let acc = 0;
  return [start, ...sizes.map((s) => start + (len * (acc += s)) / total)];
}

/** Traits du tableau `[x1, y1, x2, y2]` (mm), comme carnet_core::table::border_segments. */
export function borderSegments(t: TableProps, f: Frame): Segment[] {
  const xs = edges(t.cols, f.x_mm, f.w_mm);
  const ys = edges(t.rows, f.y_mm, f.h_mm);
  // Vrai si une case traverse la séparation `k` à la position `at` de l'autre axe.
  const crosses = (cols: boolean, k: number, at: number) =>
    t.cells.some((c: GridCell) => {
      const [pos, span, o, ospan] = cols ? [c.col, c.col_span, c.row, c.row_span] : [c.row, c.row_span, c.col, c.col_span];
      return pos < k && k < pos + span && o <= at && at < o + ospan;
    });
  const out: Segment[] = [];
  for (const cols of [true, false]) {
    const [n, along] = cols ? [t.cols.length, t.rows.length] : [t.rows.length, t.cols.length];
    for (let k = 0; k <= n; k++) {
      let start: number | null = null;
      for (let at = 0; at <= along; at++) {
        const drawn = at < along && (k === 0 || k === n || !crosses(cols, k, at));
        if (drawn && start === null) start = at;
        else if (!drawn && start !== null) {
          out.push(cols ? [xs[k], ys[start], xs[k], ys[at]] : [xs[start], ys[k], xs[at], ys[k]]);
          start = null;
        }
      }
    }
  }
  return out;
}

/** L'élément `Table` dont `id` est une case, ou `id` lui-même s'il est un tableau. */
export function tableOf(page: Page, id: string): Element | undefined {
  return page.elements.find((e) => e.kind.type === "Table" && (e.id === id || e.kind.cells.some((c) => c.element_id === id)));
}

export const isTableCell = (page: Page, id: string) => tableOf(page, id)?.id !== id && !!tableOf(page, id);

/** Le tableau dont la sélection est exactement le groupe (tableau et toutes ses cases). */
export function selectedTable(page: Page, selection: string[]): Element | undefined {
  const t = selection.map((id) => page.elements.find((e) => e.id === id)).find((e) => e?.kind.type === "Table");
  if (!t?.group) return undefined;
  const members = page.elements.filter((e) => e.group === t.group);
  return members.length === selection.length && members.every((e) => selection.includes(e.id)) ? t : undefined;
}
