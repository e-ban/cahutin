import { expect, test } from "vitest";
import { newPage } from "./factory";
import { MOTIF_HEIGHT_MM, buildRow, graphismeRows, motifCount, motifStep, presetOf } from "./graphisme";
import { cellAt, changeRows, cycleCell, emptyCells, resizeRow, rowsIn } from "./graphismeEdit";
import { pathOf, scaleElements } from "./groups";
import type { Cell, Element, GraphismeRow, Page } from "./types";

/** Une page A4 avec `n` lignes de ponts en moyenne section. */
function withRows(n: number, preset?: "tout" | "progressif"): Page {
  const p = newPage();
  return { ...p, elements: graphismeRows(p, "ponts", "MS", n, { preset }) };
}
const ids = (p: Page) => p.elements.map((e) => e.id);
const rowOf = (p: Page, sel: string[] = ids(p)) => rowsIn(p, sel)![0];
const isBase = (e: Element) => e.kind.type === "Shape" && !!e.kind.graphisme;
const mapBase = (p: Page, f: (e: Element) => Element): Page => ({ ...p, elements: p.elements.map((e) => (isBase(e) ? f(e) : e)) });
const recipeWith = (p: Page, r: Partial<GraphismeRow>) =>
  mapBase(p, (e) => (e.kind.type === "Shape" ? { ...e, kind: { ...e.kind, graphisme: { ...e.kind.graphisme!, ...r } } } : e));
const strokes = (p: Page, style: string) =>
  p.elements.flatMap((e) => (e.kind.type === "Shape" && e.kind.shape === "Path" && e.kind.stroke?.style === style ? [e.kind.stroke.color] : []));

test("rowsIn finds the rows of a selection, top to bottom", () => {
  const page = withRows(2);
  const reversed = { ...page, elements: [...page.elements].reverse() };
  const rows = rowsIn(reversed, ids(reversed))!;
  expect(rows).toHaveLength(2);
  expect(rows[0].y).toBeLessThan(rows[1].y);
  expect(rows.flatMap((r) => r.ids).sort()).toEqual(ids(page).sort());
  expect(rows[0]).toMatchObject({ x: 15, width: 180, recipe: { motif: "ponts", level: "MS" } });
  expect(rows[0].y).toBeCloseTo(page.elements[0].frame.y_mm - MOTIF_HEIGHT_MM.MS + 1);
  // Un seul élément de la ligne suffit à la retrouver.
  expect(rowsIn(page, [page.elements[3].id])).toHaveLength(1);
});

test("rowsIn returns null for a broken, ungrouped, rotated, flipped or scaled row", () => {
  const page = withRows(1);
  const broken = { ...page, elements: page.elements.slice(0, -1) };
  expect(rowsIn(broken, ids(broken))).toBeNull();
  const ungrouped = { ...page, elements: page.elements.map((e) => ({ ...e, group: null })) };
  expect(rowsIn(ungrouped, ids(ungrouped))).toBeNull();
  expect(rowsIn(mapBase(page, (e) => ({ ...e, frame: { ...e.frame, rotation_deg: 10 } })), ids(page))).toBeNull();
  expect(rowsIn(mapBase(page, (e) => ({ ...e, flip_h: true })), ids(page))).toBeNull();
  const scaled = { ...page, elements: scaleElements(page.elements, 1.5, { x_mm: 0, y_mm: 0 }) };
  expect(rowsIn(scaled, ids(scaled))).toBeNull();
  expect(rowsIn(recipeWith(page, { motif: "spirales" }), ids(page))).toBeNull();
  expect(rowsIn(recipeWith(page, { level: "CP" }), ids(page))).toBeNull();
  // Un élément qui n'est pas dans une ligne réglable : rien à régler.
  const other: Element = { ...page.elements[1], id: "autre", group: null };
  const mixed = { ...page, elements: [...page.elements, other] };
  expect(rowsIn(mixed, ids(mixed))).toBeNull();
});

test("a row without recipe is not editable", () => {
  const page = mapBase(withRows(1), (e) => {
    if (e.kind.type !== "Shape") return e;
    const { graphisme: _, ...kind } = e.kind;
    return { ...e, kind };
  });
  expect(rowsIn(page, ids(page))).toBeNull();
});

test("changeRows applies a preset to every row, progressive top to bottom", () => {
  const page = withRows(4);
  const bases = rowsIn(page, ids(page))!.map((r) => r.baseId).reverse();
  const out = changeRows(page, bases, { preset: "progressif" });
  const rows = rowsIn(out.page, out.ids)!;
  expect(rows.map((r) => presetOf(r.recipe.cells))).toEqual(["tout", "moitie", "quelques", "seul"]);
  expect(out.ids.sort()).toEqual(ids(out.page).sort());
  // Un préréglage simple vaut pour toutes les lignes.
  const all = changeRows(out.page, rows.map((r) => r.baseId), { preset: "moitie" });
  expect(rowsIn(all.page, all.ids)!.map((r) => presetOf(r.recipe.cells))).toEqual(["moitie", "moitie", "moitie", "moitie"]);
});

test("changeRows keeps the model and dotted colours", () => {
  const page = withRows(1);
  const recolored = {
    ...page,
    elements: page.elements.map((e) =>
      e.kind.type === "Shape" && e.kind.shape === "Path" && e.kind.stroke
        ? { ...e, kind: { ...e.kind, stroke: { ...e.kind.stroke, color: e.kind.stroke.style === "Dotted" ? "#0000ff" : "#ff0000" } } }
        : e,
    ),
  };
  const preset = (p: Page, preset: "seul" | "tout" | "moitie") => changeRows(p, [rowOf(p).baseId], { preset }).page;
  // Les deux couleurs restent, même après un passage sans pointillés.
  const back = preset(preset(recolored, "seul"), "tout");
  expect(strokes(back, "Solid")).toEqual(["#ff0000"]);
  expect(strokes(back, "Dotted")).toEqual(["#0000ff"]);
  const kept = preset(preset(recolored, "moitie"), "tout");
  expect(strokes(kept, "Solid")).toEqual(["#ff0000"]);
  expect(strokes(kept, "Dotted")).toEqual(["#0000ff"]);
});

test("changeRows keeps the row where it was moved", () => {
  const page = withRows(1);
  const before = rowOf(page);
  const group = page.elements[0].group!;
  const other: Element = { ...page.elements[1], id: "autre", group: null };
  const moved = {
    ...page,
    elements: [
      other,
      ...page.elements.map((e) => ({ ...e, group: "G", subgroups: [group], frame: { ...e.frame, x_mm: e.frame.x_mm + 20, y_mm: e.frame.y_mm + 20 } })),
      { ...other, id: "autre2" },
    ],
  };
  const out = changeRows(moved, [rowOf(moved, ids(page)).baseId], { preset: "seul", guides: "Corridor", arrow: false });
  const after = rowOf(out.page, out.ids);
  expect(after.x).toBeCloseTo(before.x + 20);
  expect(after.y).toBeCloseTo(before.y + 20);
  expect(after.width).toBe(before.width);
  expect(after.recipe).toMatchObject({ guides: "Corridor", arrow: false });
  // Même groupe et sous-groupe, à la même place dans la page.
  expect(out.page.elements[0].id).toBe("autre");
  expect(out.page.elements.at(-1)!.id).toBe("autre2");
  expect(out.page.elements.slice(1, -1).map((e) => e.id)).toEqual(out.ids);
  for (const e of out.page.elements.slice(1, -1)) expect(pathOf(e)).toEqual(["G", group]);
});

test("changing the motif keeps each position's state", () => {
  const p = newPage();
  const n = motifCount("ponts", "MS", 180);
  const cells: Cell[] = Array.from({ length: n }, (_, i) => (i === 0 ? "Model" : i % 3 === 0 ? "Empty" : "Dotted"));
  const recipe: GraphismeRow = { motif: "ponts", level: "MS", cells, guides: "Base", arrow: true };
  const page = { ...p, elements: buildRow(recipe, 15, 40, 180) };
  // Moins de motifs : la fin est coupée.
  const vagues = changeRows(page, [rowOf(page).baseId], { motif: "vagues" });
  const v = rowOf(vagues.page, vagues.ids).recipe;
  expect(v.motif).toBe("vagues");
  expect(v.cells).toEqual(cells.slice(0, motifCount("vagues", "MS", 180)));
  // Plus de motifs : les nouveaux prennent l'état du dernier.
  const traits = changeRows(page, [rowOf(page).baseId], { motif: "traits" });
  const t = rowOf(traits.page, traits.ids).recipe.cells;
  expect(t).toHaveLength(motifCount("traits", "MS", 180));
  expect(t.slice(0, n)).toEqual(cells);
  expect(new Set(t.slice(n))).toEqual(new Set([cells[n - 1]]));
  // La recette posée n'est jamais changée sur place.
  expect(recipe.cells).toBe(cells);
  expect(recipe.motif).toBe("ponts");
});

test("resizeRow adds or removes motifs at the same size", () => {
  const page = withRows(1);
  const row = rowOf(page);
  const height = (p: Page) => p.elements.find((e) => e.kind.type === "Shape" && e.kind.shape === "Path")!.frame.h_mm;
  const out = resizeRow(page, row.baseId, 15, 90);
  const small = rowOf(out.page, out.ids);
  expect(small.width).toBe(90);
  expect(small.recipe.cells.length).toBeLessThan(row.recipe.cells.length);
  expect(small.recipe.cells).toHaveLength(motifCount("ponts", "MS", 90));
  expect(height(out.page)).toBeCloseTo(height(page));
  // Rallongée par la gauche : à la place donnée, les motifs en plus prennent l'état du dernier.
  const wide = resizeRow(out.page, small.baseId, 5, 100);
  const big = rowOf(wide.page, wide.ids);
  expect(big).toMatchObject({ x: 5, width: 100 });
  expect(big.recipe.cells).toEqual([...small.recipe.cells, ...Array(big.recipe.cells.length - small.recipe.cells.length).fill("Dotted")]);
  expect(big.recipe.cells.length).toBeGreaterThan(small.recipe.cells.length);
});

test("resizeRow keeps at least two motifs", () => {
  const page = withRows(1);
  const out = resizeRow(page, rowOf(page).baseId, 15, 5);
  expect(rowOf(out.page, out.ids).recipe.cells).toHaveLength(2);
});

test("cellAt and cycleCell turn one motif", () => {
  const page = withRows(1);
  const row = rowOf(page);
  const h = MOTIF_HEIGHT_MM.MS;
  const start = row.x + 3.5 + 2;
  const step = motifStep("ponts", "MS");
  expect(cellAt(page, row.baseId, start + 2.5 * step, row.y + h / 2)).toBe(2);
  expect(cellAt(page, row.baseId, start + 2.5 * step, row.y + h + 1)).toBe(2);
  expect(cellAt(page, row.baseId, start + 2.5 * step, row.y - 10)).toBeNull();
  expect(cellAt(page, row.baseId, row.x + 1, row.y + h / 2)).toBeNull();
  expect(cellAt(page, row.baseId, start + (row.recipe.cells.length + 0.5) * step, row.y + h / 2)).toBeNull();
  expect(cellAt(page, "inconnu", start, row.y)).toBeNull();
  let cur = page;
  let base = row.baseId;
  const seen: Cell[] = [];
  for (let i = 0; i < 3; i++) {
    const out = cycleCell(cur, base, 2);
    cur = out.page;
    const r = rowOf(cur, out.ids);
    base = r.baseId;
    seen.push(r.recipe.cells[2]);
    expect(r.recipe.cells.filter((_, j) => j !== 2)).toEqual(row.recipe.cells.filter((_, j) => j !== 2));
  }
  expect(seen).toEqual(["Empty", "Model", "Dotted"]);
  expect(rowOf(cur).recipe.cells).toEqual(row.recipe.cells);
});

test("emptyCells gives one shape per empty motif", () => {
  const page = withRows(1);
  const out = changeRows(page, [rowOf(page).baseId], { preset: "seul" });
  const row = rowOf(out.page, out.ids);
  const n = row.recipe.cells.length;
  const shapes = emptyCells(out.page, row.baseId);
  expect(shapes).toHaveLength(n - 1);
  const step = motifStep("ponts", "MS");
  const start = row.x + 3.5 + 2;
  shapes.forEach(({ frame, path }, i) => {
    expect(path).toMatch(/^M /);
    expect(frame.x_mm).toBeCloseTo(start + (i + 1) * step, 1);
    // Les ponts posent leurs deux pieds sur la ligne de base.
    expect(frame.y_mm + frame.h_mm).toBeCloseTo(row.y + MOTIF_HEIGHT_MM.MS, 1);
  });
  expect(emptyCells(page, rowOf(page).baseId)).toEqual([]);
});
