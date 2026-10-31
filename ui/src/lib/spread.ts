// Pages en vis-à-vis, comme dans un cahier ouvert : la première page est seule (à droite), puis 2-3, 4-5…

/** La page d'en face, et son côté ; aucune pour la première page ou la dernière restée seule. */
export function facingOf(index: number, count: number): { index: number; side: "left" | "right" } | null {
  if (index === 0) return null;
  const other = index % 2 === 1 ? index + 1 : index - 1;
  if (other >= count) return null;
  return { index: other, side: index % 2 === 1 ? "right" : "left" };
}

/** Recto-verso de cartes : au verso, chaque bloc va à la place symétrique (gauche ↔ droite), pour tomber
 *  derrière son recto quand on retourne la feuille par le grand côté. Le contenu reste à l'endroit. */
export function mirrorLayout<P extends { size: { width_mm: number }; elements: { frame: { x_mm: number; w_mm: number; rotation_deg: number } }[] }>(page: P): P {
  const w = page.size.width_mm;
  return {
    ...page,
    elements: page.elements.map((e) => ({ ...e, frame: { ...e.frame, x_mm: Math.round((w - e.frame.x_mm - e.frame.w_mm) * 10) / 10, rotation_deg: -e.frame.rotation_deg } })),
  };
}
