// Groupes d'éléments : grouper, dissocier, développer une sélection, transformations d'ensemble,
// sélection au cadre. Pas de groupe imbriqué ; les cases de grille ne sont jamais groupées.
import { newId } from "./factory";
import type { Element, Frame, Page } from "./types";

type Point = { x_mm: number; y_mm: number };
type Rect = { x_mm: number; y_mm: number; w_mm: number; h_mm: number };

const isCell = (page: Page, id: string) => !!page.grid?.cells.some((c) => c.element_id === id);

/** Ids du groupe de l'élément, ou l'élément seul. */
export function groupOf(page: Page, id: string): string[] {
  const g = page.elements.find((e) => e.id === id)?.group;
  return g ? page.elements.filter((e) => e.group === g).map((e) => e.id) : [id];
}

/** La sélection, complétée par les autres membres des groupes touchés, dans l'ordre de la page. */
export function expandSelection(page: Page, ids: string[]): string[] {
  const groups = new Set(page.elements.filter((e) => ids.includes(e.id) && e.group).map((e) => e.group));
  return page.elements.filter((e) => ids.includes(e.id) || (e.group && groups.has(e.group))).map((e) => e.id);
}

/** Un seul groupe avec tous les éléments (et les groupes qu'ils touchent), rassemblés à la place du plus haut. */
export function groupElements(page: Page, ids: string[]): { page: Page; group: string | null } {
  // Les cases de grille sont écartées ; le reste est groupé s'il reste au moins deux éléments.
  const expanded = expandSelection(page, ids).filter((id) => !isCell(page, id));
  if (expanded.length < 2) return { page, group: null };
  const group = newId();
  const members = new Set(expanded);
  const top = Math.max(...page.elements.map((e, i) => (members.has(e.id) ? i : -1)));
  const before = page.elements.slice(0, top).filter((e) => !members.has(e.id)).length;
  const rest = page.elements.filter((e) => !members.has(e.id));
  const grouped = page.elements.filter((e) => members.has(e.id)).map((e) => ({ ...e, group }));
  return { page: { ...page, elements: [...rest.slice(0, before), ...grouped, ...rest.slice(before)] }, group };
}

/** Dissout les groupes des éléments désignés. */
export function ungroup(page: Page, ids: string[]): Page {
  const groups = new Set(page.elements.filter((e) => ids.includes(e.id) && e.group).map((e) => e.group));
  return { ...page, elements: page.elements.map((e) => (e.group && groups.has(e.group) ? { ...e, group: null } : e)) };
}

/** Un groupe réduit à un élément n'en est plus un. */
export function dissolveSmallGroups(page: Page): Page {
  const counts = new Map<string, number>();
  for (const e of page.elements) if (e.group) counts.set(e.group, (counts.get(e.group) ?? 0) + 1);
  if (![...counts.values()].some((n) => n < 2)) return page;
  return { ...page, elements: page.elements.map((e) => (e.group && counts.get(e.group)! < 2 ? { ...e, group: null } : e)) };
}

/** Vrai si un élément de la sélection (groupes compris) est verrouillé : rien ne bouge alors. */
export function isLockedSelection(page: Page, ids: string[]): boolean {
  const all = new Set(expandSelection(page, ids));
  return page.elements.some((e) => all.has(e.id) && e.locked);
}

/** Éléments qu'Aligner et Répartir peuvent déplacer : aucun si un membre (groupes compris) est verrouillé ; jamais les cases de grille. */
export function alignableIds(page: Page, ids: string[]): string[] {
  if (isLockedSelection(page, ids)) return [];
  const all = new Set(expandSelection(page, ids));
  return page.elements.filter((e) => all.has(e.id) && !isCell(page, e.id)).map((e) => e.id);
}

function corners(f: Frame): Point[] {
  const cx = f.x_mm + f.w_mm / 2;
  const cy = f.y_mm + f.h_mm / 2;
  const t = (f.rotation_deg * Math.PI) / 180;
  const [c, s] = [Math.cos(t), Math.sin(t)];
  return [
    [-1, -1],
    [1, -1],
    [1, 1],
    [-1, 1],
  ].map(([dx, dy]) => {
    const x = (dx * f.w_mm) / 2;
    const y = (dy * f.h_mm) / 2;
    return { x_mm: cx + x * c - y * s, y_mm: cy + x * s + y * c };
  });
}

/** Boîte englobante des cadres, coins tournés compris. */
export function boundsOf(frames: Frame[]): Rect {
  const pts = frames.flatMap(corners);
  const x = Math.min(...pts.map((p) => p.x_mm));
  const y = Math.min(...pts.map((p) => p.y_mm));
  return { x_mm: x, y_mm: y, w_mm: Math.max(...pts.map((p) => p.x_mm)) - x, h_mm: Math.max(...pts.map((p) => p.y_mm)) - y };
}

const round = (v: number) => Math.round(v * 10) / 10;
const halfPt = (v: number) => Math.round(v * 2) / 2;

/** Agrandit depuis `origin` : cadres, tailles de texte (au demi-point), traits, bordures et coins. */
export function scaleElements(els: Element[], factor: number, origin: Point): Element[] {
  return els.map((e) => {
    const f = e.frame;
    const frame = {
      ...f,
      x_mm: round(origin.x_mm + (f.x_mm - origin.x_mm) * factor),
      y_mm: round(origin.y_mm + (f.y_mm - origin.y_mm) * factor),
      w_mm: round(f.w_mm * factor),
      h_mm: round(f.h_mm * factor),
    };
    const k = e.kind;
    switch (k.type) {
      case "Text":
        return {
          ...e,
          frame,
          kind: {
            ...k,
            // Une marge par défaut reste celle par défaut : le document ne passe pas pour autant au format 3.
            style: { ...k.style, size_pt: halfPt(k.style.size_pt * factor), padding_mm: k.style.padding_mm == null ? k.style.padding_mm : round(k.style.padding_mm * factor) },
            frame: k.frame && { ...k.frame, corner_radius_mm: round(k.frame.corner_radius_mm * factor), stroke: k.frame.stroke && { ...k.frame.stroke, width_mm: round(k.frame.stroke.width_mm * factor) } },
            content: { paragraphs: k.content.paragraphs.map((p) => ({ ...p, runs: p.runs.map((r) => ({ ...r, size_pt: r.size_pt === null ? null : halfPt(r.size_pt * factor) })) })) },
          },
        };
      case "Shape":
        return {
          ...e,
          frame,
          kind: { ...k, corner_radius_mm: round(k.corner_radius_mm * factor), stroke: k.stroke && { ...k.stroke, width_mm: round(k.stroke.width_mm * factor) } },
        };
      case "Photo":
        return {
          ...e,
          frame,
          kind: { ...k, corner_radius_mm: round(k.corner_radius_mm * factor), border: k.border && { ...k.border, width_mm: round(k.border.width_mm * factor) } },
        };
      default:
        return { ...e, frame };
    }
  });
}

/** Tourne autour de `center` (sens horaire, comme à l'écran) : centres déplacés, rotation propre augmentée. */
export function rotateElements(els: Element[], deg: number, center: Point): Element[] {
  const t = (deg * Math.PI) / 180;
  const [c, s] = [Math.cos(t), Math.sin(t)];
  return els.map((e) => {
    const f = e.frame;
    const dx = f.x_mm + f.w_mm / 2 - center.x_mm;
    const dy = f.y_mm + f.h_mm / 2 - center.y_mm;
    const nx = center.x_mm + dx * c - dy * s;
    const ny = center.y_mm + dx * s + dy * c;
    const rotation = (((f.rotation_deg + deg) % 360) + 540) % 360 - 180;
    return { ...e, frame: { ...f, x_mm: round(nx - f.w_mm / 2), y_mm: round(ny - f.h_mm / 2), rotation_deg: Math.round(rotation * 10) / 10 } };
  });
}

/** Éléments entièrement couverts par le rectangle (coins tournés compris). */
export function elementsInRect(page: Page, rect: Rect): string[] {
  return page.elements
    .filter((e) => {
      const b = boundsOf([e.frame]);
      return b.x_mm >= rect.x_mm && b.y_mm >= rect.y_mm && b.x_mm + b.w_mm <= rect.x_mm + rect.w_mm && b.y_mm + b.h_mm <= rect.y_mm + rect.h_mm;
    })
    .map((e) => e.id);
}
