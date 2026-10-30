// Tableaux : un élément `Table` (grille et traits) et ses cases, éléments ordinaires du même groupe.
// Les opérations sont dans le moteur (carnet_core::table) ; ici, ce que l'affichage doit savoir tout de suite.
import { TEXT_PADDING_MM } from "./richtext";
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

/** Vide une case texte ; sa police, sa taille et son cadre restent. */
export function clearTextCell(page: Page, id: string): Page {
  return { ...page, elements: page.elements.map((e) => (e.id === id && e.kind.type === "Text" ? { ...e, kind: { ...e.kind, content: { paragraphs: [] } } } : e)) };
}

/** Tableaux recopiés avec de nouveaux ids : leurs cases désignent les copies ; une case restée en arrière
 *  (copie partielle) n'est plus désignée. */
export function remapTables(elements: Element[], ids: Map<string, string>): Element[] {
  return elements.map((e) =>
    e.kind.type === "Table" ? { ...e, kind: { ...e.kind, cells: e.kind.cells.filter((c) => ids.has(c.element_id)).map((c) => ({ ...c, element_id: ids.get(c.element_id)! })) } } : e,
  );
}

/** Case placée par une grille (mode modèle) ou par un tableau : elle ne bouge ni ne change de taille seule. */
export const isPlacedCell = (page: Page, id: string) => !!page.grid?.cells.some((c) => c.element_id === id) || isTableCell(page, id);

/** Vrai si l'élément est un tableau ou l'une de ses cases (même groupe qu'un tableau). */
export function inTableGroup(page: Page, id: string): boolean {
  const g = page.elements.find((e) => e.id === id)?.group;
  return !!g && page.elements.some((e) => e.group === g && e.kind.type === "Table");
}

/** Donne au tableau le cadre `frame` et y replace ses cases, comme carnet_core::table::layout : une case
 *  picto ou photo garde une marge pour laisser voir les traits, un picto reste carré et centré. La taille
 *  des textes ne change pas. */
export function resizeTable(page: Page, tableId: string, frame: Frame): Page {
  const t = page.elements.find((e) => e.id === tableId);
  if (t?.kind.type !== "Table") return page;
  const xs = edges(t.kind.cols, frame.x_mm, frame.w_mm);
  const ys = edges(t.kind.rows, frame.y_mm, frame.h_mm);
  const cells = new Map(t.kind.cells.map((c) => [c.element_id, c]));
  return {
    ...page,
    elements: page.elements.map((e) => {
      if (e.id === tableId) return { ...e, frame };
      const c = cells.get(e.id);
      if (!c) return e;
      const [x0, x1, y0, y1] = [xs[c.col], xs[c.col + c.col_span], ys[c.row], ys[c.row + c.row_span]];
      const inset = e.kind.type === "Text" ? 0 : Math.min(TEXT_PADDING_MM, (x1 - x0) / 4, (y1 - y0) / 4);
      let [w, h] = [x1 - x0 - 2 * inset, y1 - y0 - 2 * inset];
      if (e.kind.type === "Image") w = h = Math.min(w, h);
      return { ...e, frame: { x_mm: (x0 + x1 - w) / 2, y_mm: (y0 + y1 - h) / 2, w_mm: w, h_mm: h, rotation_deg: 0 } };
    }),
  };
}
