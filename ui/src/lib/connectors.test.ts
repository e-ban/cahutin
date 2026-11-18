import { expect, test } from "vitest";
import { anchorPoint, attachTarget, followConnectors, liveFollow, snapAllowed } from "./connectors";
import { frameFromEndpoints } from "./lineEnds";
import { fromPagePoints, pagePoints, type Pt } from "./linePath";
import type { Element, Frame, Page, ShapeKind, ShapeProps } from "./types";

const frame = (x: number, y: number, w: number, h: number, r = 0): Frame => ({ x_mm: x, y_mm: y, w_mm: w, h_mm: h, rotation_deg: r });
const props = (shape: ShapeKind): ShapeProps => ({ shape, fill: null, stroke: { width_mm: 1, color: "#333333", style: null }, corner_radius_mm: 0, path: null });
const el = (id: string, shape: ShapeKind, f: Frame): Element => ({ id, frame: f, locked: false, zone: false, group: null, subgroups: [], flip_h: false, flip_v: false, kind: { type: "Shape", ...props(shape) } });
/** Un rectangle « r » et une flèche « a » de (10, 50) à (60, 50), la fin accrochée au milieu gauche de « r » (comme connectors.rs). */
function page(): Page {
  const r = el("r", "Rect", frame(60, 40, 40, 20));
  const a = el("a", "Arrow", frameFromEndpoints({ x: 10, y: 50 }, { x: 60, y: 50 }, 4, false));
  if (a.kind.type === "Shape") a.kind.attach = { end: { element: "r", at: "Left" } };
  return { id: "p", size: { width_mm: 210, height_mm: 297 }, elements: [r, a], grid: null, background: null };
}
const get = (p: Page, id: string) => p.elements.find((e) => e.id === id)!;
const ptsOf = (p: Page, id: string): Pt[] => {
  const e = get(p, id);
  if (e.kind.type !== "Shape") throw new Error();
  return pagePoints(e.frame, e.kind, !!e.flip_h, !!e.flip_v);
};
const moved = (p: Page, id: string, f: (e: Element) => Element): Page => ({ ...p, elements: p.elements.map((e) => (e.id === id ? f(e) : e)) });
const attachOf = (p: Page, id: string) => {
  const e = get(p, id);
  return e.kind.type === "Shape" ? e.kind.attach : undefined;
};
const close = (a: Pt, b: Pt) => {
  expect(a.x).toBeCloseTo(b.x, 9);
  expect(a.y).toBeCloseTo(b.y, 9);
};

test("anchor points of a rotated frame", () => {
  const f = frame(0, 0, 40, 20, 90);
  close(anchorPoint(f, "Center"), { x: 20, y: 10 });
  close(anchorPoint(f, "Right"), { x: 20, y: 30 });
  close(anchorPoint(f, "Top"), { x: 30, y: 10 });
});

test("end follows its element; parity with Rust", () => {
  const before = page();
  const after = followConnectors(before, moved(before, "r", (e) => ({ ...e, frame: { ...e.frame, x_mm: e.frame.x_mm + 30 } })));
  const [s, e] = ptsOf(after, "a");
  close(s, { x: 10, y: 50 });
  close(e, { x: 90, y: 50 });
  const f = get(after, "a").frame;
  const r = (v: number) => Math.round(v * 1e6) / 1e6 + 0;
  // Copié de connectors.rs (PARITY).
  expect([f.x_mm, f.y_mm, f.w_mm, f.h_mm, f.rotation_deg].map(r)).toEqual([10.0, 48.0, 80.0, 4.0, 0.0]);
});

test("path elbows stay", () => {
  const base = page();
  const elbows = [{ x: 10, y: 50 }, { x: 20, y: 80 }, { x: 40, y: 30 }, { x: 60, y: 50 }];
  const before = moved(base, "a", (e) => {
    if (e.kind.type !== "Shape") return e;
    const fp = fromPagePoints(elbows, e.kind);
    return { ...e, frame: fp.frame, kind: { ...e.kind, ...fp.patch } };
  });
  const after = followConnectors(before, moved(before, "r", (e) => ({ ...e, frame: { ...e.frame, rotation_deg: 90 } })));
  const p = ptsOf(after, "a");
  for (let i = 0; i < 3; i++) close(p[i], elbows[i]);
  close(p[3], anchorPoint(get(after, "r").frame, "Left"));
});

test("both ends on one element", () => {
  const before = moved(page(), "a", (e) => (e.kind.type === "Shape" ? { ...e, kind: { ...e.kind, attach: { start: { element: "r", at: "Top" }, end: { element: "r", at: "Bottom" } } } } : e));
  const settled = followConnectors(null, before);
  const after = followConnectors(settled, moved(settled, "r", (e) => ({ ...e, frame: { ...e.frame, h_mm: 50 } })));
  const [s, e] = ptsOf(after, "a");
  close(s, { x: 80, y: 40 });
  close(e, { x: 80, y: 90 });
});

test("moving the line alone detaches", () => {
  const before = page();
  const after = followConnectors(before, moved(before, "a", (e) => ({ ...e, frame: { ...e.frame, y_mm: e.frame.y_mm + 10 } })));
  expect(attachOf(after, "a")).toBeUndefined();
  close(ptsOf(after, "a")[1], { x: 60, y: 60 });
});

test("moving both keeps the attachment", () => {
  const before = page();
  const shift = (e: Element) => ({ ...e, frame: { ...e.frame, x_mm: e.frame.x_mm + 5 } });
  const after = followConnectors(before, moved(moved(before, "a", shift), "r", shift));
  expect(attachOf(after, "a")).toBeDefined();
  close(ptsOf(after, "a")[1], { x: 65, y: 50 });
});

test("missing element drops the anchor", () => {
  const before = page();
  const after = followConnectors(before, { ...before, elements: before.elements.filter((e) => e.id !== "r") });
  expect(attachOf(after, "a")).toBeUndefined();
  close(ptsOf(after, "a")[1], { x: 60, y: 50 });
});

test("a page without attachment is returned as is", () => {
  const p = { ...page(), elements: [el("r", "Rect", frame(0, 0, 10, 10))] };
  expect(followConnectors(null, p)).toBe(p);
  const settled = followConnectors(null, page());
  expect(followConnectors(settled, settled)).toBe(settled);
});

test("live frames move the arrow without detaching", () => {
  const p = page();
  const shown = followConnectors(null, p, new Map([["r", frame(80, 40, 40, 20)]]));
  close(ptsOf(shown, "a")[1], { x: 80, y: 50 });
  expect(attachOf(shown, "a")).toBeDefined();
  expect(get(shown, "r").frame.x_mm).toBe(80);
});

test("attachTarget finds the element under the tip and its nearest point within the tolerance", () => {
  const p = page();
  const near = attachTarget(p, { x: 60.8, y: 50.5 }, "a", 2)!;
  expect(near.element.id).toBe("r");
  expect(near.snap?.at).toBe("Left");
  const inside = attachTarget(p, { x: 70, y: 45 }, "a", 2)!;
  expect(inside.element.id).toBe("r");
  expect(inside.snap).toBeNull();
  expect(attachTarget(p, { x: 150, y: 150 }, "a", 2)).toBeNull();
});

test("lines are never targets, a table cell gives its table", () => {
  const p = page();
  expect(attachTarget(p, { x: 30, y: 50 }, "x", 2)).toBeNull();
  const cell = el("c", "Rect", frame(0, 100, 20, 10));
  const table: Element = { ...el("t", "Rect", frame(0, 100, 40, 20)), kind: { type: "Table", cols: [20, 20], rows: [10, 10], cells: [{ element_id: "c", col: 0, row: 0, col_span: 1, row_span: 1 }] } as unknown as Element["kind"] };
  const q = { ...p, elements: [...p.elements, table, cell] };
  expect(attachTarget(q, { x: 5, y: 105 }, "a", 2)!.element.id).toBe("t");
});

test("moving an elbow keeps the attachment; a closed path has none", () => {
  const base = page();
  const withPts = (p: Page, pts: Pt[], closed = false) =>
    moved(p, "a", (e) => {
      if (e.kind.type !== "Shape") return e;
      const k = { ...e.kind, closed: closed || undefined };
      const fp = fromPagePoints(pts, k);
      return { ...e, frame: fp.frame, kind: { ...k, ...fp.patch } };
    });
  const before = withPts(base, [{ x: 10, y: 50 }, { x: 30, y: 80 }, { x: 60, y: 50 }]);
  const after = followConnectors(before, withPts(before, [{ x: 10, y: 50 }, { x: 35, y: 95 }, { x: 60, y: 50 }]));
  expect(attachOf(after, "a")).toBeDefined();
  const closed = followConnectors(before, withPts(before, [{ x: 10, y: 50 }, { x: 35, y: 95 }, { x: 60, y: 50 }], true));
  expect(attachOf(closed, "a")).toBeUndefined();
});

test("a fresh attachment is kept", () => {
  const before = page();
  // Le bout a été tiré jusqu'au milieu du haut de « r », et s'y accroche.
  const after = followConnectors(before, moved(before, "a", (e) => (e.kind.type === "Shape" ? { ...e, frame: frameFromEndpoints({ x: 10, y: 50 }, { x: 80.2, y: 40.1 }, 4, false), kind: { ...e.kind, attach: { end: { element: "r", at: "Top" } } } } : e)));
  expect(attachOf(after, "a")).toBeDefined();
  close(ptsOf(after, "a")[1], { x: 80, y: 40 });
});

test("liveFollow moves attached arrows but leaves the elements Moveable is moving", () => {
  const p = page();
  const live = new Map([["r", frame(80, 40, 40, 20)]]);
  const shown = liveFollow(p, live);
  close(ptsOf(shown, "a")[1], { x: 80, y: 50 });
  expect(get(shown, "r")).toBe(get(p, "r"));
  const both = liveFollow(p, new Map([["r", frame(80, 40, 40, 20)], ["a", { ...get(p, "a").frame, x_mm: 30 }]]));
  expect(get(both, "a")).toBe(get(p, "a"));
});

test("a tiny rounding does not detach", () => {
  const before = page();
  const after = followConnectors(before, moved(before, "a", (e) => ({ ...e, frame: { ...e.frame, rotation_deg: e.frame.rotation_deg + 0.04 } })));
  expect(attachOf(after, "a")).toBeDefined();
  close(ptsOf(after, "a")[1], { x: 60, y: 50 });
});

test("an end cannot snap onto the point the other end already holds", () => {
  const p = page();
  const a = get(p, "a");
  if (a.kind.type !== "Shape") throw new Error();
  // La fin tient le milieu gauche de « r » : le début ne peut pas s'y accrocher aussi (trait de longueur nulle).
  expect(snapAllowed(a.kind, "start", "r", "Left")).toBe(false);
  expect(snapAllowed(a.kind, "start", "r", "Top")).toBe(true);
  expect(snapAllowed(a.kind, "end", "r", "Left")).toBe(true);
});
