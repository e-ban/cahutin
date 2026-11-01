import { expect, test } from "vitest";
import { blockFrieze, frieze, friezeIn, friezeSettings, isPageFrame, pageFrame, resizeFrieze, toggleMotif } from "./decor";
import { ungroup } from "./groups";
import type { Element, Page } from "./types";

/** Motifs pictogrammes. */
const m = (...codes: string[]) => codes.map((code) => ({ code }));

const a4 = (): Page => ({ id: "p", size: { width_mm: 210, height_mm: 297 }, elements: [], grid: null });

test("pageFrame double makes a grouped pair inside the margin", () => {
  const els = pageFrame(a4(), "double");
  expect(els).toHaveLength(2);
  expect(els[0].group).toBeTruthy();
  expect(els[0].group).toBe(els[1].group);
  expect(els.map((e) => e.frame.x_mm)).toEqual([8, 11]);
  expect(els.map((e) => e.kind.type === "Shape" && e.kind.stroke?.width_mm)).toEqual([0.5, 1.5]);
  expect(els.every((e) => e.kind.type === "Shape" && e.kind.fill === null)).toBe(true);
  const thin = pageFrame(a4(), "thin");
  expect(thin).toHaveLength(1);
  expect(thin[0].group ?? null).toBeNull();
  expect(thin[0].frame).toMatchObject({ x_mm: 8, y_mm: 8, w_mm: 194, h_mm: 281 });
  expect(pageFrame(a4(), "rounded")[0].kind.type === "Shape" && pageFrame(a4(), "rounded")[0].kind.corner_radius_mm).toBe(8);
});

test("a page frame is told apart from a smaller hollow rectangle", () => {
  const page = a4();
  for (const style of ["thin", "thick", "rounded", "double"] as const) expect(pageFrame(page, style).every((e) => isPageFrame(e, page.size))).toBe(true);
  const [frame] = pageFrame(page, "thin");
  expect(isPageFrame({ ...frame, frame: { ...frame.frame, w_mm: 80, h_mm: 60 } }, page.size)).toBe(false);
  expect(isPageFrame({ ...frame, kind: { ...frame.kind, fill: "#ffffff" } as Element["kind"] }, page.size)).toBe(false);
});

test("frieze top spaces copies evenly within the margins", () => {
  const els = frieze(a4(), m("2600"), "top");
  const xs = els.map((e) => e.frame.x_mm);
  expect(xs[0]).toBe(8);
  expect(xs.at(-1)! + 15).toBeCloseTo(202, 1);
  const gaps = xs.slice(1).map((x, i) => x - xs[i]);
  expect(Math.max(...gaps) - Math.min(...gaps)).toBeLessThan(0.11);
  expect(els.every((e) => e.frame.y_mm === 8 && e.frame.w_mm === 15)).toBe(true);
  expect(els.length).toBe(Math.floor((194 + 3) / (15 + 3)));
});

test("frieze around has no duplicate corner", () => {
  const els = frieze(a4(), m("2600"), "around");
  const keys = els.map((e) => `${e.frame.x_mm},${e.frame.y_mm}`);
  expect(new Set(keys).size).toBe(keys.length);
  expect(keys).toContain("8,8");
  expect(keys).toContain("187,274");
});

test("frieze copies share one group", () => {
  const els = frieze(a4(), m("1F98A"), "bottom");
  expect(new Set(els.map((e) => e.group)).size).toBe(1);
  expect(els[0].group).toBeTruthy();
  expect(els.every((e) => e.kind.type === "Image" && e.kind.code === "1F98A")).toBe(true);
});

const codes = (els: ReturnType<typeof frieze>) => els.map((e) => (e.kind.type === "Image" ? e.kind.code : ""));

test("frieze repeats the motif sequence along a row", () => {
  const els = frieze(a4(), m("A", "B", "C"), "top");
  expect(codes(els).slice(0, 7)).toEqual(["A", "B", "C", "A", "B", "C", "A"]);
});

test("frieze around is one continuous clockwise loop", () => {
  const els = frieze(a4(), m("A", "B"), "around");
  // Ordre horaire : haut de gauche à droite, droite de haut en bas, bas de droite à gauche, gauche de bas en haut.
  const pts = els.map((e) => [e.frame.x_mm, e.frame.y_mm]);
  const top = pts.filter(([, y]) => y === 8).length;
  expect(pts.slice(0, top).every(([, y], i) => y === 8 && (i === 0 || pts[i][0] > pts[i - 1][0]))).toBe(true);
  const right = pts.findIndex(([x, y]) => x === 187 && y > 8);
  expect(right).toBe(top);
  const last = pts.at(-1)!;
  expect(last[0]).toBe(8);
  expect(last[1]).toBeGreaterThan(8);
  expect(codes(els).every((c, i) => c === (i % 2 === 0 ? "A" : "B"))).toBe(true);
});

test("toggleMotif adds at the end and removes when present", () => {
  expect(toggleMotif(["A"], "B")).toEqual(["A", "B"]);
  expect(toggleMotif(["A", "B", "C"], "B")).toEqual(["A", "C"]);
  expect(toggleMotif(["A"], "A")).toEqual([]);
});

const block = (extra: Partial<Element> = {}): Element => ({
  id: "b",
  frame: { x_mm: 50, y_mm: 60, w_mm: 60, h_mm: 40, rotation_deg: 0 },
  locked: false,
  zone: false,
  kind: { type: "Shape", shape: "Rect", fill: "#ffffff", stroke: null, corner_radius_mm: 0 },
  ...extra,
});
const withBlock = (b: Element, more: Element[] = []): Page => ({ ...a4(), elements: [b, ...more] });

test("block frieze straddles the block edges and is grouped with it", () => {
  const page = blockFrieze(withBlock(block()), "b", m("A", "B"))!;
  const [b, ...pics] = page.elements;
  expect(b.group).toBeTruthy();
  expect(pics.length).toBeGreaterThan(8);
  expect(pics.every((p) => p.group === b.group && p.kind.type === "Image")).toBe(true);
  // Taille : le quart du plus petit côté (40 mm), centrée sur le bord.
  expect(pics[0].frame).toMatchObject({ x_mm: 45, y_mm: 55, w_mm: 10, h_mm: 10 });
  expect(codes(pics).slice(0, 4)).toEqual(["A", "B", "A", "B"]);
});

test("block frieze is grouped with the block's group, and Dissocier gives both back whole", () => {
  const other = { ...block(), id: "o", group: "g1" };
  const page = blockFrieze(withBlock(block({ group: "g1" }), [other]), "b", m("A"))!;
  expect(new Set(page.elements.map((e) => e.group)).size).toBe(1);
  expect(page.elements.filter((e) => e.subgroups?.[0] === "g1").map((e) => e.id)).toEqual(["b", "o"]);
  const back = ungroup(page, ["b"]);
  expect(back.elements.find((e) => e.id === "b")?.group).toBe("g1");
  const friezeGroups = new Set(back.elements.filter((e) => e.kind.type === "Image").map((e) => e.group));
  expect(friezeGroups.size).toBe(1);
  expect([...friezeGroups][0]).not.toBe("g1");
});

test("block frieze follows a rotated block", () => {
  const page = blockFrieze(withBlock(block({ frame: { x_mm: 50, y_mm: 60, w_mm: 60, h_mm: 40, rotation_deg: 90 } })), "b", m("A"))!;
  const pics = page.elements.slice(1);
  expect(pics.every((p) => p.frame.rotation_deg === 90)).toBe(true);
  // Le premier motif (coin haut gauche avant rotation, centre (50, 60)) tourne de 90° autour du centre (80, 80) : (100, 50).
  const c = { x: pics[0].frame.x_mm + 5, y: pics[0].frame.y_mm + 5 };
  expect(c.x).toBeCloseTo(100, 0);
  expect(c.y).toBeCloseTo(50, 0);
});

test("block frieze size stays between 6 and 15 mm, and grid cells get none", () => {
  const small = blockFrieze(withBlock(block({ frame: { x_mm: 0, y_mm: 0, w_mm: 16, h_mm: 16, rotation_deg: 0 } })), "b", m("A"))!;
  expect(small.elements[1].frame.w_mm).toBe(6);
  const big = blockFrieze(withBlock(block({ frame: { x_mm: 0, y_mm: 0, w_mm: 150, h_mm: 120, rotation_deg: 0 } })), "b", m("A"))!;
  expect(big.elements[1].frame.w_mm).toBe(15);
  const grid: Page = { ...withBlock(block()), grid: { cols: 1, rows: 1, margin_mm: 0, gap_mm: 0, cells: [{ element_id: "b", col: 0, row: 0, col_span: 1, row_span: 1, kind: "Photo" }] } as unknown as Page["grid"] };
  expect(blockFrieze(grid, "b", m("A"))).toBeNull();
  expect(blockFrieze(withBlock(block()), "b", [])).toBeNull();
});

const cow = { id: "vache", width_px: 2000, height_px: 1000, mime: "Png" as const };

test("a collection picture in a frieze keeps its proportions, its longest side the size of a pictogram", () => {
  const els = frieze(a4(), [{ code: "2600" }, { asset: cow }], "top", 20);
  const [sun, pic] = els;
  expect(sun.frame).toMatchObject({ w_mm: 20, h_mm: 20 });
  expect(pic.kind).toMatchObject({ type: "Photo", asset: "vache" });
  expect(pic.frame).toMatchObject({ w_mm: 20, h_mm: 10 });
  // Centrée dans sa case, sur la même ligne que le soleil.
  expect(pic.frame.y_mm + pic.frame.h_mm / 2).toBeCloseTo(sun.frame.y_mm + 10, 5);
});

test("a wider spacing puts fewer pictures along the same side", () => {
  expect(frieze(a4(), m("A"), "top", 15, 20).length).toBeLessThan(frieze(a4(), m("A"), "top", 15, 3).length);
});

const onPage = (els: Element[]): Page => ({ ...a4(), elements: els });

test("a placed frieze is found in the selection and redrawn in place, same motifs in the same order", () => {
  for (const side of ["top", "bottom", "around"] as const) {
    const page = onPage(frieze(a4(), [{ code: "A" }, { asset: cow }, { code: "B" }], side));
    const ids = friezeIn(page, [page.elements[0].id])!;
    expect(ids).toHaveLength(page.elements.length);
    expect(friezeSettings(page, ids).size).toBe(15);
    const out = resizeFrieze(page, ids, 25, 3).page;
    const keys = out.elements.map((e) => (e.kind.type === "Image" ? e.kind.code : e.kind.type === "Photo" ? e.kind.asset : ""));
    expect(keys.slice(0, 3)).toEqual(["A", "vache", "B"]);
    expect(new Set(out.elements.map((e) => e.group)).size).toBe(1);
    const tops = out.elements.map((e) => e.frame.y_mm);
    const bottoms = out.elements.map((e) => e.frame.y_mm + e.frame.h_mm);
    const before = page.elements.map((e) => e.frame.y_mm);
    // Le haut reste en haut, le bas reste en bas.
    if (side !== "bottom") expect(Math.min(...tops)).toBeCloseTo(Math.min(...before), 0);
    if (side === "bottom") expect(Math.max(...bottoms)).toBeCloseTo(Math.max(...page.elements.map((e) => e.frame.y_mm + e.frame.h_mm)), 0);
    expect(out.elements.filter((e) => e.kind.type === "Image")[0].frame.w_mm).toBe(25);
  }
});

test("the frieze around a block is found inside its group and keeps its levels when resized", () => {
  const page = blockFrieze(withBlock(block()), "b", m("A", "B"))!;
  const ids = friezeIn(page, page.elements.map((e) => e.id))!;
  expect(ids).not.toContain("b");
  const out = resizeFrieze(page, ids, 8, 2).page;
  const outer = out.elements.find((e) => e.id === "b")!.group;
  expect(out.elements.every((e) => e.group === outer)).toBe(true);
  expect(new Set(out.elements.filter((e) => e.id !== "b").map((e) => e.subgroups?.[0])).size).toBe(1);
});

test("a few loose pictures are not a frieze", () => {
  const page = onPage(frieze(a4(), m("A"), "top").slice(0, 2));
  expect(friezeIn(page, page.elements.map((e) => e.id))).toBeNull();
});
