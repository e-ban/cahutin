// Groupes d'éléments : grouper, dissocier, développer une sélection, transformations d'ensemble,
// sélection au cadre. Un groupe peut en contenir d'autres : le chemin d'un élément est son groupe (le plus
// large, celui qu'un clic sélectionne) puis ses sous-groupes. `inside` est le chemin du groupe où l'on est
// entré par double-clic : les blocs sont alors ses membres directs. Les cases de grille ne sont jamais
// groupées, un tableau garde son propre groupe.
import { newId } from "./factory";
import type { Element, Frame, Page } from "./types";

type Point = { x_mm: number; y_mm: number };
type Rect = { x_mm: number; y_mm: number; w_mm: number; h_mm: number };

const isCell = (page: Page, id: string) => !!page.grid?.cells.some((c) => c.element_id === id);

/** Les ids encore présents sur la page, dans le même ordre. */
export function existingIds(page: Page, ids: string[]): string[] {
  return ids.filter((id) => page.elements.some((e) => e.id === id));
}

/** Groupe puis sous-groupes de l'élément, du plus large au plus petit ; vide s'il n'est pas groupé. */
export const pathOf = (e: Element): string[] => (e.group ? [e.group, ...(e.subgroups ?? [])] : []);

/** L'élément avec ce chemin (sans sous-groupes notés s'il n'en a pas : le document ne change pas de version). */
export function withPath(e: Element, path: string[]): Element {
  const { subgroups: _, ...rest } = e;
  return path.length > 1 ? { ...rest, group: path[0], subgroups: path.slice(1) } : { ...rest, group: path[0] ?? null };
}

const startsWith = (path: string[], prefix: string[]) => prefix.every((g, i) => path[i] === g);

/** Le bloc de l'élément dans le groupe `inside` : le groupe (ou sous-groupe) suivant de son chemin, ou rien
 *  s'il y est seul. */
function blockKey(e: Element, inside: string[]): string | null {
  const p = pathOf(e);
  return p.length > inside.length && startsWith(p, inside) ? p[inside.length] : null;
}

/** Ids du bloc de l'élément (son groupe, ou son sous-groupe dans le groupe où l'on est entré), ou l'élément seul. */
export function groupOf(page: Page, id: string, inside: string[] = []): string[] {
  const e = page.elements.find((x) => x.id === id);
  const key = e && blockKey(e, inside);
  return key ? page.elements.filter((x) => blockKey(x, inside) === key).map((x) => x.id) : [id];
}

/** La sélection, complétée par les autres membres des blocs touchés, dans l'ordre de la page. */
export function expandSelection(page: Page, ids: string[], inside: string[] = []): string[] {
  const keys = new Set<string | null>(page.elements.filter((e) => ids.includes(e.id)).map((e) => blockKey(e, inside)).filter((k) => k !== null));
  return page.elements.filter((e) => ids.includes(e.id) || keys.has(blockKey(e, inside))).map((e) => e.id);
}

/** Un groupe de plus autour des blocs choisis : les groupes qu'ils forment déjà y restent, entiers (un groupe
 *  dans un groupe) ; tous rassemblés à la place du plus haut. */
export function groupElements(page: Page, ids: string[], inside: string[] = []): { page: Page; group: string | null } {
  // Les cases de grille et les tableaux sont écartés ; le reste est groupé s'il reste au moins deux éléments.
  const tableGroups = new Set(page.elements.filter((e) => e.kind.type === "Table" && e.group).map((e) => e.group));
  const inTable = (id: string) => tableGroups.has(page.elements.find((e) => e.id === id)?.group);
  const expanded = expandSelection(page, ids, inside).filter((id) => !isCell(page, id) && !inTable(id));
  if (expanded.length < 2) return { page, group: null };
  const group = newId();
  const members = new Set(expanded);
  // Le nouveau groupe se place juste sous la part de chemin commune à tous : dans le groupe qui les contient
  // tous, s'il y en a un.
  const paths = page.elements.filter((e) => members.has(e.id)).map(pathOf);
  let common = 0;
  while (paths.every((p) => p.length > common && p[common] === paths[0][common])) common++;
  const top = Math.max(...page.elements.map((e, i) => (members.has(e.id) ? i : -1)));
  const before = page.elements.slice(0, top).filter((e) => !members.has(e.id)).length;
  const rest = page.elements.filter((e) => !members.has(e.id));
  const grouped = page.elements.filter((e) => members.has(e.id)).map((e) => {
    const p = pathOf(e);
    return withPath(e, [...p.slice(0, common), group, ...p.slice(common)]);
  });
  return { page: normalizeGroups({ ...page, elements: [...rest.slice(0, before), ...grouped, ...rest.slice(before)] }), group };
}

/** Dissocier : les blocs désignés perdent un seul niveau (leurs sous-groupes restent) ; le groupe d'un tableau
 *  reste (il se dissocie par le moteur). */
export function ungroup(page: Page, ids: string[], inside: string[] = []): Page {
  const tables = new Set(page.elements.filter((e) => e.kind.type === "Table" && e.group).map((e) => e.group));
  const keys = new Set(page.elements.filter((e) => ids.includes(e.id) && !tables.has(e.group)).map((e) => blockKey(e, inside)).filter((k) => k !== null));
  return {
    ...page,
    elements: page.elements.map((e) => {
      const key = blockKey(e, inside);
      return key !== null && keys.has(key) ? withPath(e, pathOf(e).filter((_, i) => i !== inside.length)) : e;
    }),
  };
}

/** Remet les groupes d'aplomb, comme le moteur : un groupe garde la même place dans chaque chemin (sinon le
 *  chemin s'arrête avant), et un groupe d'un seul membre (élément ou sous-groupe) disparaît, à chaque niveau.
 *  Le groupe d'un tableau reste toujours. */
export function normalizeGroups(page: Page): Page {
  const paths = page.elements.map(pathOf);
  const parent = new Map<string, string[]>();
  paths.forEach((p, i) => {
    const bad = p.findIndex((g, d) => p.slice(0, d).includes(g) || (parent.has(g) && parent.get(g)!.join("/") !== p.slice(0, d).join("/")));
    if (bad >= 0) paths[i] = p.slice(0, bad);
    paths[i].forEach((g, d) => parent.has(g) || parent.set(g, paths[i].slice(0, d)));
  });
  const tables = new Set(page.elements.filter((e) => e.kind.type === "Table" && e.group).map((e) => e.group!));
  for (;;) {
    const members = new Map<string, Set<string>>();
    paths.forEach((p, i) => p.forEach((g, d) => members.set(g, (members.get(g) ?? new Set()).add(p[d + 1] ?? page.elements[i].id))));
    const lone = new Set([...members].filter(([g, m]) => m.size < 2 && !tables.has(g)).map(([g]) => g));
    if (!lone.size) break;
    paths.forEach((p, i) => (paths[i] = p.filter((g) => !lone.has(g))));
  }
  if (page.elements.every((e, i) => pathOf(e).join("/") === paths[i].join("/"))) return page;
  return { ...page, elements: page.elements.map((e, i) => (pathOf(e).join("/") === paths[i].join("/") ? e : withPath(e, paths[i]))) };
}

/** Un groupe réduit à un élément n'en est plus un (à chaque niveau). */
export const dissolveSmallGroups = normalizeGroups;

/** Vrai si un élément de la sélection (groupes compris) est verrouillé : rien ne bouge alors. */
export function isLockedSelection(page: Page, ids: string[], inside: string[] = []): boolean {
  const all = new Set(expandSelection(page, ids, inside));
  return page.elements.some((e) => all.has(e.id) && e.locked);
}

/** Éléments qu'Aligner et Répartir peuvent déplacer : aucun si un membre (groupes compris) est verrouillé ; jamais les cases de grille. */
export function alignableIds(page: Page, ids: string[], inside: string[] = []): string[] {
  if (isLockedSelection(page, ids, inside)) return [];
  const all = new Set(expandSelection(page, ids, inside));
  return page.elements.filter((e) => all.has(e.id) && !isCell(page, e.id)).map((e) => e.id);
}

/** Blocs qu'Aligner et Répartir déplacent : chaque groupe (ou sous-groupe du groupe où l'on est entré) d'un
 *  tenant, chaque élément seul sinon. */
export function alignUnits(page: Page, ids: string[], inside: string[] = []): string[][] {
  const units = new Map<string, string[]>();
  for (const id of alignableIds(page, ids, inside)) {
    const key = blockKey(page.elements.find((e) => e.id === id)!, inside) ?? `seul:${id}`;
    units.set(key, [...(units.get(key) ?? []), id]);
  }
  return [...units.values()];
}

/** Aligne ou répartit les boîtes des blocs avec `arrange`, puis déplace chaque bloc d'une pièce. */
export function arrangeUnits(page: Page, ids: string[], arrange: (frames: Frame[]) => Frame[], inside: string[] = []): Page {
  const units = alignUnits(page, ids, inside);
  const frameOf = (id: string) => page.elements.find((e) => e.id === id)!.frame;
  const boxes: Frame[] = units.map((u) => ({ ...boundsOf(u.map(frameOf)), rotation_deg: 0 }));
  const moved = arrange(boxes);
  const delta = new Map<string, [number, number]>();
  units.forEach((u, i) => {
    const [dx, dy] = [moved[i].x_mm - boxes[i].x_mm, moved[i].y_mm - boxes[i].y_mm];
    if (dx !== 0 || dy !== 0) for (const id of u) delta.set(id, [dx, dy]);
  });
  if (delta.size === 0) return page;
  return {
    ...page,
    elements: page.elements.map((e) => {
      const d = delta.get(e.id);
      return d ? { ...e, frame: { ...e.frame, x_mm: round(e.frame.x_mm + d[0]), y_mm: round(e.frame.y_mm + d[1]) } } : e;
    }),
  };
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
      case "Table":
        return { ...e, frame, kind: { ...k, border: k.border && { ...k.border, width_mm: round(k.border.width_mm * factor) } } };
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
