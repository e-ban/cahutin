// Un texte trop long pour son cadre : « Agrandir le cadre » (la hauteur qu'il faut, sans sortir de la page) et
// « Réduire le texte » (toutes les tailles dans la même proportion, jusqu'à ce qu'il tienne). Ici le calcul ;
// la mesure (en pixels à taille réelle, sans le zoom) est faite par le bloc affiché.
import { isLockedSelection } from "./groups";
import { isPlacedCell } from "./table";
import { mmToPx, PX_PER_MM } from "./units";
import type { Element, Frame, Page, TextProps } from "./types";

/** Les plus petites lettres d'un texte réduit ne descendent pas sous cette taille : encore lisibles. */
export const MIN_TEXT_PT = 6;

/** Un gain de hauteur plus petit n'est pas une modification (la précision des cadres). */
const MIN_GAIN_MM = 0.1;

/** Hauteur de cadre qu'il faut pour `contentPx` de texte (pixels à taille réelle) et sa marge, au 0,1 mm supérieur. */
export function neededHeightMm(contentPx: number, paddingMm: number): number {
  return Math.ceil((contentPx / PX_PER_MM + 2 * paddingMm) * 10 - 1e-9) / 10;
}

/** Hauteur du cadre agrandi, bornée au bas de la page ; `fits` : le texte y tient. Null : rien à agrandir. */
export function grownHeight(neededMm: number, frame: Frame, pageHeightMm: number): { h_mm: number; fits: boolean } | null {
  const max = Math.round((pageHeightMm - frame.y_mm) * 10) / 10;
  const h_mm = Math.min(neededMm, max);
  if (h_mm < frame.h_mm + MIN_GAIN_MM) return null;
  return { h_mm, fits: h_mm >= neededMm };
}

const halfPt = (pt: number) => Math.round(pt * 2) / 2;

/** Les tailles du bloc et des passages qui ont la leur, multipliées par `factor` (au demi-point). */
export function scaledText(props: TextProps, factor: number): TextProps {
  const size = (pt: number) => Math.max(MIN_TEXT_PT, halfPt(pt * factor));
  return {
    ...props,
    style: { ...props.style, size_pt: size(props.style.size_pt) },
    content: {
      paragraphs: props.content.paragraphs.map((p) => ({ ...p, runs: p.runs.map((r) => (r.size_pt ? { ...r, size_pt: size(r.size_pt) } : r)) })),
    },
  };
}

/** La plus petite taille du texte : celle du bloc ou d'un passage. */
function smallestPt(props: TextProps): number {
  return Math.min(props.style.size_pt, ...props.content.paragraphs.flatMap((p) => p.runs.map((r) => r.size_pt ?? Infinity)));
}

/** Tailles possibles du bloc, de la plus grande à la plus petite, au demi-point, sans que rien passe sous le plancher. */
function blockSizes(props: TextProps): number[] {
  const block = props.style.size_pt;
  const lowest = Math.ceil(((block * MIN_TEXT_PT) / smallestPt(props)) * 2 - 1e-9) / 2;
  const out: number[] = [];
  for (let b = halfPt(block) - 0.5; b >= lowest; b -= 0.5) if (b < block) out.push(b);
  return out;
}

/**
 * Le texte réduit à la plus grande taille où `fits` dit qu'il tient ; s'il ne tient nulle part, au plancher.
 * Null : il tient déjà, ou ses plus petites lettres sont au plancher.
 */
export function shrunkText(props: TextProps, fits: (p: TextProps) => boolean): TextProps | null {
  const sizes = blockSizes(props);
  if (sizes.length === 0 || fits(props)) return null;
  const at = (i: number) => scaledText(props, sizes[i] / props.style.size_pt);
  // Les tailles vont en décroissant : une fois que le texte tient, il tient aussi plus petit.
  let lo = 0;
  let hi = sizes.length - 1;
  if (!fits(at(hi))) return at(hi);
  while (lo < hi) {
    const mid = (lo + hi) >> 1;
    if (fits(at(mid))) hi = mid;
    else lo = mid + 1;
  }
  return at(lo);
}

/**
 * Ce qu'on propose pour un bloc qui dépasse. Verrouillé (lui ou son groupe) : rien. Case de grille ou de
 * tableau : son cadre est placé par la grille, seul le texte se réduit. Tourné : il tourne autour de son centre,
 * l'agrandir le déplacerait ; on peut réduire. Style nommé : réduire le détacherait de son style (ou changerait
 * tous ses textes) ; on peut agrandir.
 */
export function fitActions(page: Page, e: Element, inside: string[] = []): { grow: boolean; shrink: boolean } {
  if (e.kind.type !== "Text" || isLockedSelection(page, [e.id], inside)) return { grow: false, shrink: false };
  const room = e.frame.y_mm + e.frame.h_mm < page.size.height_mm - MIN_GAIN_MM;
  return {
    grow: room && !isPlacedCell(page, e.id) && e.frame.rotation_deg % 360 === 0,
    shrink: !e.kind.style.named && blockSizes(e.kind).length > 0,
  };
}

/**
 * Où mettre les boutons (`heightPx` de haut, `widthPx` de large, écart compris) : sous le bloc, ou dans le bloc au-dessus
 * de son bord bas quand la page n'a pas la place sous lui ; calés sur le bord droit du bloc, ou sur son bord gauche
 * quand ils sortiraient de la page par la gauche (un bloc étroit près du bord).
 */
export function actionsPlace(frame: Frame, page: { width_mm: number; height_mm: number }, zoom: number, heightPx: number, widthPx: number): { inside: boolean; left: boolean } {
  const right = mmToPx(frame.x_mm + frame.w_mm, zoom);
  const fromLeft = mmToPx(frame.x_mm, zoom) + widthPx;
  return {
    inside: mmToPx(page.height_mm - frame.y_mm - frame.h_mm, zoom) < heightPx,
    left: right - widthPx < 0 && fromLeft - mmToPx(page.width_mm, zoom) < widthPx - right,
  };
}

/** Hauteur du texte d'un bloc affiché (`.text`), du haut de son premier paragraphe au bas du dernier, en pixels à
 *  taille réelle (sans le zoom ni la rotation). */
export function contentHeightPx(box: HTMLElement): number {
  const flow = box.firstElementChild;
  const first = flow?.firstElementChild as HTMLElement | null | undefined;
  const last = flow?.lastElementChild as HTMLElement | null | undefined;
  if (!first || !last) return 0;
  return last.offsetTop + last.offsetHeight - first.offsetTop;
}
