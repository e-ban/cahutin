// Connecteurs : un bout de trait, de flèche ou de chemin accroché à un point d'un élément (milieu d'un côté ou
// centre de son cadre) le suit. Même règle que `connectors.rs` : d'abord décrocher un trait déplacé seul, puis amener
// chaque bout encore accroché sur son point. L'éditeur l'applique à chaque modification (docStore) ; la page s'en sert
// aussi pour dessiner la flèche en direct pendant qu'on fait glisser un élément (`live`).
import { fromPagePoints, pagePoints, type Pt } from "./linePath";
import { tableOf } from "./table";
import type { Anchor, AnchorPoint, Attach, Element, Frame, Page } from "./types";

export const ANCHORS: AnchorPoint[] = ["Top", "Right", "Bottom", "Left", "Center"];
const AT: Record<AnchorPoint, [number, number]> = { Top: [0.5, 0], Right: [1, 0.5], Bottom: [0.5, 1], Left: [0, 0.5], Center: [0.5, 0.5] };

/** Le point d'accroche d'un cadre, en mm de page : milieu d'un côté ou centre, rotation comprise (miroir ignoré). */
export function anchorPoint(f: Frame, at: AnchorPoint): Pt {
  const [fx, fy] = AT[at];
  const cx = f.x_mm + f.w_mm / 2;
  const cy = f.y_mm + f.h_mm / 2;
  const [dx, dy] = [(fx - 0.5) * f.w_mm, (fy - 0.5) * f.h_mm];
  const a = (f.rotation_deg * Math.PI) / 180;
  return { x: cx + dx * Math.cos(a) - dy * Math.sin(a), y: cy + dx * Math.sin(a) + dy * Math.cos(a) };
}

const isLine = (e: Element) => e.kind.type === "Shape" && (e.kind.shape === "Line" || e.kind.shape === "Arrow");
const attachOf = (e: Element): Attach | undefined => (e.kind.type === "Shape" && isLine(e) ? (e.kind.attach ?? undefined) : undefined);
const sameFrame = (a: Frame, b: Frame) => a.x_mm === b.x_mm && a.y_mm === b.y_mm && a.w_mm === b.w_mm && a.h_mm === b.h_mm && a.rotation_deg === b.rotation_deg;
const samePoints = (a: [number, number][] | null | undefined, b: [number, number][] | null | undefined) =>
  a === b || (!!a && !!b && a.length === b.length && a.every((p, i) => p[0] === b[i][0] && p[1] === b[i][1]));

/** Le trait sans son accroche (d'un bout, ou des deux). */
export function withoutAttach(e: Element, which?: "start" | "end"): Element {
  const a = attachOf(e);
  if (!a || e.kind.type !== "Shape") return e;
  const rest: Attach = which ? { ...a, [which]: undefined } : {};
  const { attach: _drop, ...kind } = e.kind;
  void _drop;
  return { ...e, kind: rest.start || rest.end ? { ...kind, attach: clean(rest) } : kind };
}

function clean<T extends object>(o: T): T {
  return Object.fromEntries(Object.entries(o).filter(([, v]) => v !== undefined)) as T;
}

/**
 * La règle des connecteurs, après une modification de la page (`before` : la page d'avant, ou `null`).
 * 1. Décrocher : un bout accroché dont le trait a bougé (cadre, points, miroir) alors que son élément n'a pas bougé
 *    perd son accroche (on a déplacé la flèche seule).
 * 2. Suivre : chaque bout encore accroché va sur le point de son élément ; un élément absent le décroche. Les coudes
 *    d'un chemin restent ; le cadre est recalculé (`fromPagePoints`).
 * `live` : cadres en cours d'un geste (affichage seul) ; ils remplacent ceux de la page, sans rien décrocher.
 * Rend la même page (le même objet) si rien ne change.
 */
export function followConnectors(before: Page | null, after: Page, live?: Map<string, Frame> | null): Page {
  if (!after.elements.some((e) => attachOf(e))) return after;
  const elements = live?.size ? after.elements.map((e) => (live.has(e.id) ? { ...e, frame: live.get(e.id)! } : e)) : after.elements;
  const frames = new Map(elements.map((e) => [e.id, e.frame]));
  const old = new Map((before?.elements ?? []).map((e) => [e.id, e]));
  let changedAny = elements !== after.elements;
  const out = elements.map((e) => {
    const attach = attachOf(e);
    if (!attach || e.kind.type !== "Shape") return e;
    const o = live ? undefined : old.get(e.id);
    const lineMoved = !!o && o.kind.type === "Shape" && (!sameFrame(o.frame, e.frame) || !samePoints(o.kind.points, e.kind.points) || !!o.flip_h !== !!e.flip_h || !!o.flip_v !== !!e.flip_v);
    const keep = (a: Anchor | null | undefined): Anchor | undefined => {
      if (!a) return undefined;
      const now = frames.get(a.element);
      if (!now) return undefined;
      const was = old.get(a.element);
      return lineMoved && was && sameFrame(was.frame, now) ? undefined : a;
    };
    const start = keep(attach.start);
    const end = keep(attach.end);
    const pts = pagePoints(e.frame, e.kind, !!e.flip_h, !!e.flip_v);
    let moved = false;
    for (const [a, k] of [[start, 0], [end, pts.length - 1]] as const) {
      if (!a) continue;
      const p = anchorPoint(frames.get(a.element)!, a.at);
      if (Math.abs(p.x - pts[k].x) > 1e-9 || Math.abs(p.y - pts[k].y) > 1e-9) {
        pts[k] = p;
        moved = true;
      }
    }
    const sameAttach = start === attach.start && end === attach.end;
    if (!moved && sameAttach) return e;
    changedAny = true;
    const { attach: _drop, ...rest } = e.kind;
    void _drop;
    const kind = start || end ? { ...rest, attach: clean({ start, end }) } : rest;
    if (!moved) return { ...e, kind };
    const r = fromPagePoints(pts, kind);
    return { ...e, frame: r.frame, flip_h: false, flip_v: false, kind: clean({ ...kind, ...r.patch }) };
  });
  return changedAny ? { ...after, elements: out } : after;
}

/** Un point du plan dans le repère d'un cadre (tourné) : vrai s'il est dedans, élargi de `margin` mm. */
function inside(f: Frame, p: Pt, margin: number): boolean {
  const cx = f.x_mm + f.w_mm / 2;
  const cy = f.y_mm + f.h_mm / 2;
  const a = (-f.rotation_deg * Math.PI) / 180;
  const [dx, dy] = [p.x - cx, p.y - cy];
  const [lx, ly] = [dx * Math.cos(a) - dy * Math.sin(a), dx * Math.sin(a) + dy * Math.cos(a)];
  return Math.abs(lx) <= f.w_mm / 2 + margin && Math.abs(ly) <= f.h_mm / 2 + margin;
}

/** Marge autour d'un élément où le bout tiré montre ses points d'accroche. */
export const ATTACH_MARGIN_MM = 3;

/**
 * L'élément sous le bout tiré (le plus haut, cadre élargi de 3 mm ; jamais un trait ; une case de tableau désigne son
 * tableau), dont on montre les points ; `snap` : son point d'accroche à moins de `tolMm`, sinon `null`.
 */
export function attachTarget(page: Page, tip: Pt, ignoreId: string, tolMm: number): { element: Element; snap: { at: AnchorPoint; point: Pt } | null } | null {
  for (let i = page.elements.length - 1; i >= 0; i--) {
    const e = page.elements[i];
    if (e.id === ignoreId || isLine(e) || !inside(e.frame, tip, ATTACH_MARGIN_MM)) continue;
    const target = (e.kind.type === "Table" ? e : tableOf(page, e.id)) ?? e;
    let snap: { at: AnchorPoint; point: Pt } | null = null;
    let best = tolMm;
    for (const at of ANCHORS) {
      const p = anchorPoint(target.frame, at);
      const d = Math.hypot(p.x - tip.x, p.y - tip.y);
      if (d <= best) [best, snap] = [d, { at, point: p }];
    }
    return { element: target, snap };
  }
  return null;
}

/** `followConnectors` sur chaque page qui a changé entre `before` et `after` ; rend `after` lui-même si rien ne
 *  change. */
export function followDoc<D extends { pages: Page[] }>(before: D | null, after: D): D {
  const old = new Map((before?.pages ?? []).map((p) => [p.id, p]));
  let changed = false;
  const pages = after.pages.map((p) => {
    const was = old.get(p.id) ?? null;
    if (was === p) return p;
    const q = followConnectors(was, p);
    if (q !== p) changed = true;
    return q;
  });
  return changed ? { ...after, pages } : after;
}
