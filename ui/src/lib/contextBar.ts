// Placement de la barre flottante : au-dessus de l'élément, sinon en dessous, sinon en haut de la zone visible.
export interface Box {
  left: number;
  top: number;
  width: number;
  height: number;
}

/** Surface commune de deux boîtes. */
function overlap(a: Box, b: Box): number {
  const w = Math.min(a.left + a.width, b.left + b.width) - Math.max(a.left, b.left);
  const h = Math.min(a.top + a.height, b.top + b.height) - Math.max(a.top, b.top);
  return w > 0 && h > 0 ? w * h : 0;
}

/** `gap` : écart avec l'élément ; `margin` : écart minimal avec les bords de la zone visible. `others` : les
 *  autres éléments de la page, que la barre évite de cacher (une légende au-dessus d'une bulle) : elle va en
 *  dessous quand elle y cache moins. Un élément qui contient la sélection (un fond, un cadre) ne compte pas. */
export function placeContextBar(target: Box, bar: { width: number; height: number }, viewport: Box, gap = 8, margin = 8, others: Box[] = []): { left: number; top: number } {
  const minTop = viewport.top + margin;
  const maxTop = viewport.top + viewport.height - margin - bar.height;
  const above = target.top - gap - bar.height;
  // En dessous, pas de poignée de rotation : l'écart ordinaire suffit.
  const below = target.top + target.height + Math.min(gap, 8);
  const centered = target.left + target.width / 2 - bar.width / 2;
  const minLeft = viewport.left + margin;
  const maxLeft = Math.max(minLeft, viewport.left + viewport.width - margin - bar.width);
  const left = Math.min(maxLeft, Math.max(minLeft, centered));
  const contains = (o: Box) => o.left <= target.left && o.top <= target.top && o.left + o.width >= target.left + target.width && o.top + o.height >= target.top + target.height;
  const obstacles = others.filter((o) => !contains(o));
  const hidden = (top: number) => obstacles.reduce((sum, o) => sum + overlap({ left, top, width: bar.width, height: bar.height }, o), 0);
  let top: number;
  if (above >= minTop && below <= maxTop) top = hidden(below) < hidden(above) ? below : above;
  else top = above >= minTop ? above : below <= maxTop ? below : minTop;
  return { left, top };
}

/** Boîtes (à l'écran) des éléments de la page hors de `ids`. */
export function otherBoxes(pageEl: HTMLElement, ids: string[]): Box[] {
  return [...pageEl.querySelectorAll<HTMLElement>(".element[data-id]")]
    .filter((el) => !ids.includes(el.dataset.id!))
    .map((el) => el.getBoundingClientRect())
    .map((r) => ({ left: r.left, top: r.top, width: r.width, height: r.height }));
}

/** Boîte englobante (à l'écran) des éléments désignés de la page ; null si aucun n'est affiché. */
export function selectionBox(pageEl: HTMLElement, ids: string[]): Box | null {
  const rects = ids
    .map((id) => pageEl.querySelector<HTMLElement>(`[data-id="${CSS.escape(id)}"]`)?.getBoundingClientRect())
    .filter((r): r is DOMRect => !!r);
  if (rects.length === 0) return null;
  const left = Math.min(...rects.map((r) => r.left));
  const top = Math.min(...rects.map((r) => r.top));
  return { left, top, width: Math.max(...rects.map((r) => r.right)) - left, height: Math.max(...rects.map((r) => r.bottom)) - top };
}
