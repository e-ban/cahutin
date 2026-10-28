// Placement de la barre flottante : au-dessus de l'élément, sinon en dessous, sinon en haut de la zone visible.
export interface Box {
  left: number;
  top: number;
  width: number;
  height: number;
}

export function placeContextBar(target: Box, bar: { width: number; height: number }, viewport: Box, gap = 8): { left: number; top: number } {
  const minTop = viewport.top + gap;
  const maxTop = viewport.top + viewport.height - gap - bar.height;
  const above = target.top - gap - bar.height;
  const below = target.top + target.height + gap;
  const top = above >= minTop ? above : below <= maxTop ? below : minTop;
  const centered = target.left + target.width / 2 - bar.width / 2;
  const minLeft = viewport.left + gap;
  const maxLeft = Math.max(minLeft, viewport.left + viewport.width - gap - bar.width);
  return { left: Math.min(maxLeft, Math.max(minLeft, centered)), top };
}
