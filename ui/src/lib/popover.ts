// Un menu qui s'ouvre sous un bouton reste dans la fenêtre : au-dessus du bouton s'il y a plus de place, calé
// sur le bord gauche ou droit, et il défile s'il reste trop haut.

export interface Fit {
  /** Ouvert au-dessus du bouton plutôt qu'en dessous. */
  above: boolean;
  /** Hauteur au plus, le menu défilant au-delà ; null : il tient en entier. */
  maxHeight: number | null;
  /** Décalage horizontal pour rester dans la fenêtre. */
  shiftX: number;
}

/** Hauteur au plus d'un menu qui défile : assez pour plusieurs lignes, même quand la place manque. */
const MIN_HEIGHT = 120;

/**
 * `anchor` : le haut et le bas du bouton ; `pop` : la position gauche et la taille du menu, s'il s'ouvrait sous le
 * bouton sans décalage ; `view` : la taille de la fenêtre. `gap` : écart entre le bouton et le menu ;
 * `margin` : écart minimal avec les bords de la fenêtre.
 */
export function fitPopover(
  anchor: { top: number; bottom: number },
  pop: { left: number; width: number; height: number },
  view: { width: number; height: number },
  gap = 6,
  margin = 8,
): Fit {
  const below = view.height - margin - (anchor.bottom + gap);
  const aboveSpace = anchor.top - gap - margin;
  const above = pop.height > below && aboveSpace > below;
  const room = above ? aboveSpace : below;
  const maxHeight = pop.height > room ? Math.max(MIN_HEIGHT, Math.floor(room)) : null;
  let shiftX = 0;
  const right = pop.left + pop.width;
  if (right > view.width - margin) shiftX = view.width - margin - right;
  if (pop.left + shiftX < margin) shiftX = margin - pop.left;
  return { above, maxHeight, shiftX: Math.round(shiftX) };
}

/**
 * Action Svelte pour un menu en `position: absolute` sous son bouton (le parent) : à l'ouverture, quand son
 * contenu change de taille et quand la fenêtre change, il est replacé pour rester visible en entier.
 */
export function fitInWindow(node: HTMLElement, gap = 6) {
  let frame = 0;
  function place() {
    const anchorEl = node.parentElement;
    if (!anchorEl) return;
    // Mesuré sans les réglages d'avant.
    node.style.maxHeight = "";
    node.style.overflowY = "";
    node.style.translate = "";
    // Mesuré sous le bouton, quel que soit le sens choisi par la feuille de style.
    node.style.top = `calc(100% + ${gap}px)`;
    node.style.bottom = "auto";
    const a = anchorEl.getBoundingClientRect();
    const r = node.getBoundingClientRect();
    const fit = fitPopover({ top: a.top, bottom: a.bottom }, { left: r.left, width: r.width, height: r.height }, { width: window.innerWidth, height: window.innerHeight }, gap);
    // Les deux sens sont écrits : un menu ouvert au-dessus par sa feuille de style peut aussi passer dessous.
    node.style.top = fit.above ? "auto" : `calc(100% + ${gap}px)`;
    node.style.bottom = fit.above ? `calc(100% + ${gap}px)` : "auto";
    if (fit.maxHeight !== null) {
      node.style.maxHeight = `${fit.maxHeight}px`;
      node.style.overflowY = "auto";
    }
    if (fit.shiftX) node.style.translate = `${fit.shiftX}px 0`;
  }
  // Une seule mesure par image : replacer change la taille, que l'observateur signale encore.
  const schedule = () => {
    cancelAnimationFrame(frame);
    frame = requestAnimationFrame(place);
  };
  place();
  let last = node.scrollHeight;
  const observer = new ResizeObserver(() => {
    if (node.scrollHeight !== last) {
      last = node.scrollHeight;
      schedule();
    }
  });
  observer.observe(node);
  window.addEventListener("resize", schedule);
  return {
    destroy() {
      cancelAnimationFrame(frame);
      observer.disconnect();
      window.removeEventListener("resize", schedule);
    },
  };
}
