import { expect, test } from "vitest";
import { BUBBLE_TAIL, LEGACY_BODY, LEGACY_TIP, bubblePath, hasTail, legacyBubbleToTailed, shapePath, tailOf, tailPagePoint, thoughtPath, tipFraction } from "./tail";
import { SHAPE_PATHS } from "./shapePaths";
import type { Element, Frame, ShapeProps } from "./types";

const nums = (d: string) => d.split(" ").filter((t) => !/[MLCZ]/.test(t)).map(Number);
const subpaths = (d: string) => d.split(" ").filter((t) => t === "M").length;

test("a bubble with its tip inside the body draws no tail", () => {
  const d = bubblePath(70, 35, [0.5, 0.5]);
  expect(d.startsWith("M ") && d.endsWith(" Z")).toBe(true);
  expect(Math.max(...nums(d))).toBeLessThanOrEqual(1000);
  expect(Math.min(...nums(d))).toBeGreaterThanOrEqual(0);
});

test("the tail reaches the tip, on the side that faces it", () => {
  // Pointe par défaut, sous la bulle à gauche.
  const below = bubblePath(70, 35, BUBBLE_TAIL);
  expect(below).toContain(" 129 1286 C ");
  // À droite : la pointe sort par le côté droit.
  const right = bubblePath(70, 35, [1.6, 0.4]);
  expect(right).toContain(" 1600 400 C ");
  expect(Math.max(...nums(right).filter((_, i) => i % 2 === 1))).toBeLessThanOrEqual(1000);
});

test("bubble corners stay round whatever the frame: same radius in mm on both axes", () => {
  // Le premier point est (r, 0) : r × 1000 / w ; le coin haut droit finit en (w, r) : r × 1000 / h.
  const d = bubblePath(100, 30, [0.5, 0.5]);
  const v = nums(d);
  const rx = (v[0] * 100) / 1000;
  const i = d.split(" ").indexOf("C");
  const tokens = d.split(" ");
  const ry = (Number(tokens[i + 6]) * 30) / 1000;
  expect(rx).toBeCloseTo(7.5, 0);
  expect(ry).toBeCloseTo(7.5, 0);
});

test("a thought adds three separate rounds toward the tip, none when the tip is inside", () => {
  const cloud = SHAPE_PATHS.Cloud!;
  expect(thoughtPath(70, 40, [0.5, 0.5])).toBe(cloud);
  const d = thoughtPath(70, 40, [0.15, 1.45]);
  expect(d.startsWith(cloud + " M ")).toBe(true);
  expect(subpaths(d) - subpaths(cloud)).toBe(3);
});

test("tailOf only reads a finite, bounded tail of a bubble or a cloud", () => {
  const s = (shape: ShapeProps["shape"], tail: unknown) => ({ shape, fill: null, stroke: null, corner_radius_mm: 0, tail }) as unknown as ShapeProps;
  expect(tailOf(s("Bubble", [0.1, 1.3]))).toEqual([0.1, 1.3]);
  expect(tailOf(s("Cloud", [0.1, 1.3]))).toEqual([0.1, 1.3]);
  expect(tailOf(s("Rect", [0.1, 1.3]))).toBeNull();
  expect(tailOf(s("Bubble", [Number.NaN, 1]))).toBeNull();
  expect(tailOf(s("Bubble", [0, 1e6]))).toBeNull();
  expect(tailOf(s("Bubble", undefined))).toBeNull();
});

test("shapePath: an untailed bubble keeps its fixed drawing", () => {
  const f: Frame = { x_mm: 0, y_mm: 0, w_mm: 70, h_mm: 45, rotation_deg: 0 };
  expect(shapePath(f, { shape: "Bubble", fill: null, stroke: null, corner_radius_mm: 0 })).toBe(SHAPE_PATHS.Bubble);
  expect(shapePath(f, { shape: "Bubble", fill: null, stroke: null, corner_radius_mm: 0, tail: [0.2, 1.4] })).toBe(bubblePath(70, 45, [0.2, 1.4]));
});

const near = (a: { x: number; y: number }, b: { x: number; y: number }) => {
  expect(a.x).toBeCloseTo(b.x, 6);
  expect(a.y).toBeCloseTo(b.y, 6);
};

test("the tip fraction and its page point are inverse, turned and mirrored", () => {
  const frame: Frame = { x_mm: 30, y_mm: 40, w_mm: 60, h_mm: 30, rotation_deg: 33 };
  for (const [fh, fv] of [[false, false], [true, false], [false, true], [true, true]]) {
    const p = tailPagePoint(frame, [0.2, 1.4], fh, fv);
    const back = tipFraction(frame, p, fh, fv);
    expect(back[0]).toBeCloseTo(0.2, 9);
    expect(back[1]).toBeCloseTo(1.4, 9);
  }
});

test("turning an old bubble into a tailed one keeps its body and its tip in place on the page", () => {
  for (const [fh, fv] of [[false, false], [true, true], [false, true]]) {
    const e: Element = { id: "b", locked: false, zone: false, flip_h: fh, flip_v: fv, frame: { x_mm: 30, y_mm: 40, w_mm: 80, h_mm: 50, rotation_deg: 25 }, kind: { type: "Shape", shape: "Bubble", fill: "#ffffff", stroke: null, corner_radius_mm: 0 } };
    const t = legacyBubbleToTailed(e);
    if (t.kind.type !== "Shape") throw new Error();
    // Les quatre coins du corps (78 % du haut de l'ancien cadre) ne bougent pas.
    for (const [x, y] of [[0, 0], [1, 0], [0, 1], [1, 1]]) near(tailPagePoint(e.frame, [x, y * LEGACY_BODY], fh, fv), tailPagePoint(t.frame, [x, y], fh, fv));
    // Et la pointe non plus.
    near(tailPagePoint(e.frame, LEGACY_TIP, fh, fv), tailPagePoint(t.frame, t.kind.tail!, fh, fv));
    expect(hasTail(t)).toBe(true);
  }
});

// Mêmes valeurs que `screen_and_export_draw_the_same_tails` (shapes.rs) : l'écran et l'export dessinent la même bulle.
test("same tails as the export, to the character", () => {
  expect(bubblePath(70, 35, [9 / 70, 45 / 35])).toBe("M 125 0 L 875 0 C 944 0 1000 112 1000 250 L 1000 750 C 1000 888 944 1000 875 1000 L 401 1000 C 401 1143 265 1214 129 1286 C 175 1211 244 1100 244 1000 L 125 1000 C 56 1000 0 888 0 750 L 0 250 C 0 112 56 0 125 0 Z");
  expect(bubblePath(52.3, 41.7, [1.37, 0.21])).toBe("M 172 0 L 828 0 C 923 0 1000 97 1000 216 L 1000 243 C 1185 243 1278 227 1370 210 C 1274 329 1130 507 1000 507 L 1000 784 C 1000 903 923 1000 828 1000 L 172 1000 C 77 1000 0 903 0 784 L 0 216 C 0 97 77 0 172 0 Z");
  expect(bubblePath(30, 60, [0.4, -0.8])).toBe("M 250 0 L 296 0 C 296 -400 348 -600 400 -800 C 498 -592 646 -280 646 0 L 750 0 C 888 0 1000 56 1000 125 L 1000 875 C 1000 944 888 1000 750 1000 L 250 1000 C 112 1000 0 944 0 875 L 0 125 C 0 56 112 0 250 0 Z");
  expect(thoughtPath(70, 40, [0.15, 1.45])).toBe(`${SHAPE_PATHS.Cloud} M 265 1035 C 297 1035 324 1081 324 1137 C 324 1193 297 1239 265 1239 C 233 1239 207 1193 207 1137 C 207 1081 233 1035 265 1035 Z M 199 1246 C 221 1246 240 1278 240 1317 C 240 1357 221 1389 199 1389 C 176 1389 158 1357 158 1317 C 158 1278 176 1246 199 1246 Z M 150 1404 C 164 1404 176 1425 176 1450 C 176 1475 164 1496 150 1496 C 136 1496 124 1475 124 1450 C 124 1425 136 1404 150 1404 Z`);
  expect(thoughtPath(55.5, 33.3, [-0.7, 0.3])).toBe(`${SHAPE_PATHS.Cloud} M -115 278 C -75 278 -43 331 -43 398 C -43 464 -75 518 -115 518 C -154 518 -187 464 -187 398 C -187 331 -154 278 -115 278 Z M -427 261 C -399 261 -377 299 -377 345 C -377 392 -399 429 -427 429 C -455 429 -477 392 -477 345 C -477 299 -455 261 -427 261 Z M -700 246 C -682 246 -668 270 -668 300 C -668 330 -682 354 -700 354 C -718 354 -732 330 -732 300 C -732 270 -718 246 -700 246 Z`);
});

test("a thought tile dragged onto the page keeps its tail; a broken tail is dropped", async () => {
  const { parseShapeTile, shapeTile } = await import("./dnd");
  const { shapeElement } = await import("./factory");
  const { newDocument } = await import("./factory");
  const t = parseShapeTile(shapeTile({ shape: "Cloud", radius: 0, tail: [0.15, 1.45] }))!;
  expect(t.tail).toEqual([0.15, 1.45]);
  expect(parseShapeTile(JSON.stringify({ shape: "Cloud", radius: 0, tail: ["x", 1] }))!.tail).toBeUndefined();
  const e = shapeElement(newDocument().pages[0], t.shape, t.radius, undefined, t.ends, t.tail);
  if (e.kind.type !== "Shape") throw new Error();
  expect(e.kind.tail).toEqual([0.15, 1.45]);
  expect(e.frame.h_mm).toBe(40);
});

test("an old bubble's tip and body match its fixed drawing", () => {
  const v = nums(SHAPE_PATHS.Bubble!);
  const pts = v.filter((_, i) => i % 2 === 0).map((x, i) => ({ x, y: v[2 * i + 1] }));
  // La pointe : le point le plus bas ; le corps : le bas des coins arrondis, juste au-dessus.
  const tip = pts.reduce((a, b) => (b.y > a.y ? b : a));
  expect(LEGACY_TIP[0]).toBeCloseTo(tip.x / 1000, 3);
  expect(LEGACY_TIP[1]).toBeCloseTo(tip.y / 1000, 3);
  // Le bas du corps : les segments droits (L) du bord du bas, de part et d'autre de la pointe.
  const t = SHAPE_PATHS.Bubble!.split(" ");
  const ls = t.flatMap((tok, i) => (tok === "L" ? [Number(t[i + 2])] : []));
  const body = Math.max(...ls.filter((y) => y < tip.y - 50));
  expect(LEGACY_BODY).toBeCloseTo(body / 1000, 3);
});
