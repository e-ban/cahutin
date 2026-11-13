// Grilles de milliers de vignettes (page Photothèque, vue agrandie du tiroir Images) : les dessiner toutes d'un
// coup gèlerait l'application. On en dessine `DRAWN_STEP`, puis davantage quand le bas de la grille approche.

/** Vignettes dessinées d'abord, puis à chaque fois que le bas de la grille approche. */
export const DRAWN_STEP = 300;

/** Le plus proche parent qui défile (la page Photothèque, le corps du tiroir), sinon rien. */
export function scrollParent(el: Element): Element | null {
  for (let p = el.parentElement; p; p = p.parentElement) {
    const o = getComputedStyle(p).overflowY;
    if (o === "auto" || o === "scroll") return p;
  }
  return null;
}

/** Le repère `mark` (sous la dernière vignette dessinée) à moins de 1500 px du bas de l'écran, ou déjà passé
 *  au-dessus (un saut tout en bas ; une colonne de filtres plus longue que la grille fait descendre plus bas que
 *  lui) : `more` est appelé. Vu à chaque défilement, et une image après le départ, au cas où il serait déjà
 *  proche. Rend de quoi arrêter. */
export function drawMoreNearEnd(mark: HTMLElement, more: () => void): () => void {
  const scroller = scrollParent(mark);
  if (!scroller) return () => {};
  const check = () => {
    if (mark.getBoundingClientRect().top < scroller.getBoundingClientRect().bottom + 1500) more();
  };
  scroller.addEventListener("scroll", check, { passive: true });
  const frame = requestAnimationFrame(check);
  return () => {
    scroller.removeEventListener("scroll", check);
    cancelAnimationFrame(frame);
  };
}
