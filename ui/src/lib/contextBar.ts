// Placement de la barre flottante : au-dessus de l'élément, sinon en dessous, sinon en haut de la zone visible.
export interface Box {
  left: number;
  top: number;
  width: number;
  height: number;
}

/** `gap` : écart avec l'élément ; `margin` : écart minimal avec les bords de la zone visible. */
export function placeContextBar(target: Box, bar: { width: number; height: number }, viewport: Box, gap = 8, margin = 8): { left: number; top: number } {
  const minTop = viewport.top + margin;
  const maxTop = viewport.top + viewport.height - margin - bar.height;
  const above = target.top - gap - bar.height;
  const below = target.top + target.height + gap;
  const top = above >= minTop ? above : below <= maxTop ? below : minTop;
  const centered = target.left + target.width / 2 - bar.width / 2;
  const minLeft = viewport.left + margin;
  const maxLeft = Math.max(minLeft, viewport.left + viewport.width - margin - bar.width);
  return { left: Math.min(maxLeft, Math.max(minLeft, centered)), top };
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
