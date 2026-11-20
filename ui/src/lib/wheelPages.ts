// La molette tourne les pages : quand la vue ne peut plus défiler dans ce sens (page entière à l'écran, ou bord de la
// page atteint), un coup de molette passe à la page suivante ou précédente. Juste après un défilement ou une page
// tournée, la molette attend un instant : l'élan d'un pavé tactile ne fait pas défiler plusieurs pages d'un coup.

/** Pixels de molette pour tourner une page (un cran de souris suffit). */
export const WHEEL_PAGE_STEP = 40;
/** Pause demandée après un défilement ou une page tournée, en millisecondes. */
export const WHEEL_PAGE_PAUSE_MS = 300;

export interface WheelPages {
  acc: number;
  /** Dernier mouvement : un défilement de la vue ou une page tournée. */
  last: number;
}

export const wheelPagesStart = (): WheelPages => ({ acc: 0, last: -Infinity });

/** Un coup de molette (`deltaY`, `deltaMode` comme dans WheelEvent) sur la vue, qui peut encore défiler vers le haut
 *  (`canUp`) ou vers le bas (`canDown`), à l'instant `now`. `scroll` : la vue défile ; `dir` : la page à tourner. */
export function wheelPages(state: WheelPages, deltaY: number, deltaMode: number, canUp: boolean, canDown: boolean, now: number): { state: WheelPages; scroll: boolean; dir: 1 | -1 | 0 } {
  const px = deltaMode === 1 ? deltaY * 40 : deltaMode === 2 ? deltaY * 800 : deltaY;
  if (px === 0) return { state, scroll: false, dir: 0 };
  if ((px > 0 && canDown) || (px < 0 && canUp)) return { state: { acc: 0, last: now }, scroll: true, dir: 0 };
  if (now - state.last < WHEEL_PAGE_PAUSE_MS) return { state: { acc: 0, last: now }, scroll: false, dir: 0 };
  const acc = Math.sign(px) === Math.sign(state.acc) ? state.acc + px : px;
  if (Math.abs(acc) < WHEEL_PAGE_STEP) return { state: { ...state, acc }, scroll: false, dir: 0 };
  return { state: { acc: 0, last: now }, scroll: false, dir: acc > 0 ? 1 : -1 };
}
