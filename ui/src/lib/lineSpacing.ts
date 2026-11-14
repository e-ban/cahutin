// Interligne d'un texte (`TextStyle.line_spacing`, multiple de la hauteur naturelle d'une ligne), rendu à l'écran
// comme dans LibreOffice, où l'ODG porte `fo:line-height` en pourcentage.
//
// Mesuré (LibreOffice 26.8 sans écran contre WebKitGTK, polices Andika et Playwrite FR Moderne) :
// - LibreOffice arrondit l'ascendante et la descendante au point à 600 ppp ; la hauteur naturelle n est leur
//   somme. À N %, le pas des lignes vaut N × n et tout l'espace ajouté va au-dessus de chaque ligne : la première
//   ligne de base est à N × n - descendante (N > 1), ou à min(ascendante ; 0,8 × N × n) (N < 1).
// - Le moteur web arrondit l'ascendante et la descendante au pixel CSS, et partage l'espace ajouté moitié
//   au-dessus, moitié au-dessous de la ligne.
// D'où un facteur `line-height` qui donne le pas de LibreOffice, plus un décalage vertical du texte qui pose la
// première ligne de base (et donc toutes) sur celle de LibreOffice. La hauteur occupée est la même des deux
// côtés : le décalage vaut pour les trois alignements verticaux, et ne compte pas dans « texte trop long ».
//
// Limites connues : des tailles mêlées dans une ligne (un grand mot) ne suivent pas la même règle des deux côtés
// (2 à 4 mm d'écart mesurés sur la ligne du grand mot) ; des polices mêlées, les polices autres que les deux
// mesurées, et les moteurs web autres que WebKitGTK n'ont pas été mesurés. L'interligne externe de la police
// (« line gap ») est ignoré : nul pour les quatre polices fournies. Sous 1, les jambages de la dernière ligne
// descendent sous la hauteur occupée (LibreOffice les dessine hors du cadre, l'écran les couperait) : ils comptent
// dans « texte trop long » (TextEl), et « Agrandir le cadre » leur fait la place.
import type { FontMetrics, TextFrame } from "./types";

/** Les quatre choix du panneau ; null : l'interligne de la police (rien d'écrit dans le fichier). Serré à 0,85
 *  plutôt que 0,8 : la cursive Playwrite y garde un peu de blanc entre les boucles du haut et du bas. */
export const LINE_SPACINGS = [
  { key: "tight", value: 0.85 },
  { key: "normal", value: null },
  { key: "airy", value: 1.5 },
  { key: "double", value: 2 },
] as const;

/** Métriques d'Andika, la police de secours de l'écran : pour une police inconnue ou absente. */
export const FALLBACK_METRICS: FontMetrics = { units_per_em: 2048, ascent: 2500, descent: 800 };

const MM_PER_PT = 25.4 / 72;
const MM_PER_DOT = 25.4 / 600;
const MM_PER_PX = 25.4 / 96;

export type LineBox = {
  /** Facteur CSS `line-height` (sans unité) ; null : `normal`, comme sans réglage. */
  lineHeight: number | null;
  /** Décalage vertical du texte vers le bas, en mm (négatif : vers le haut). */
  shiftMm: number;
};

/** Ascendante et descendante en mm, comme LibreOffice les arrondit (au point à 600 ppp) et comme le moteur web
 *  (au pixel CSS) ; police aux hauteurs inconnues : celles d'Andika. */
function heights(metrics: FontMetrics | null | undefined, sizePt: number) {
  const m = metrics && metrics.units_per_em > 0 && metrics.ascent + metrics.descent > 0 ? metrics : FALLBACK_METRICS;
  const em = (units: number) => units / m.units_per_em;
  const dots = Math.round((sizePt * 600) / 72);
  const px = (sizePt * 96) / 72;
  return {
    loADots: Math.round(dots * em(m.ascent)),
    loDDots: Math.round(dots * em(m.descent)),
    webA: Math.round(px * em(m.ascent)) * MM_PER_PX,
    webD: Math.round(px * em(m.descent)) * MM_PER_PX,
  };
}

/** Facteur et décalage d'un bloc de `sizePt` dans une police de `metrics`, à l'interligne `spacing`. */
export function lineBox(spacing: number | null | undefined, metrics: FontMetrics | null | undefined, sizePt: number): LineBox {
  if (spacing == null || !(spacing > 0) || !(sizePt > 0)) return { lineHeight: null, shiftMm: 0 };
  const { loADots, loDDots, webA, webD } = heights(metrics, sizePt);
  // LibreOffice : au point à 600 ppp.
  const loA = loADots * MM_PER_DOT;
  const loD = loDDots * MM_PER_DOT;
  const loN = loA + loD;
  const pitch = spacing * loN;
  const loBaseline = spacing >= 1 ? pitch - loD : Math.min(loA, 0.8 * pitch);
  // Moteur web : au pixel CSS, l'espace ajouté partagé en deux.
  const webBaseline = webA + (pitch - (webA + webD)) / 2;
  return { lineHeight: pitch / (sizePt * MM_PER_PT), shiftMm: loBaseline - webBaseline };
}

/** Écart des lignes où l'on écrit dans un cadre (lignes fortes du Seyès, lignes simples), comme
 *  `background::WRITING_LINE_MM` du moteur ; la première à un écart du haut du cadre (lib/background.ts). */
export const WRITING_LINE_MM = 8;

/** Le texte du cadre s'écrit sur ses lignes : le réglage est mis et le motif a des lignes où écrire (comme
 *  `background::writes_on_lines`). */
export function writesOnLines(frame: TextFrame | null | undefined): boolean {
  return !!frame?.write_on_lines && (frame.pattern === "Seyes" || frame.pattern === "Lines");
}

export type WritingLineBox = {
  /** Hauteur CSS d'une ligne : le pas, en mm. */
  lineHeight: string;
  /** Décalage vertical du texte vers le bas, en mm. */
  shiftMm: number;
  /** Marge du haut : la descendante de LibreOffice, comme dans l'ODG. */
  padTopMm: number;
  /** D'une ligne de base à la suivante, la première à un pas du haut du cadre. */
  pitchMm: number;
};

/**
 * Texte écrit sur les lignes du cadre (mesuré dans LibreOffice 26.8 et WebKitGTK, Andika et Playwrite FR Moderne,
 * 12 à 28 pt). L'ODG porte une hauteur de ligne d'au moins un pas et une marge du haut égale à la descendante
 * arrondie à 600 ppp (D) : LibreOffice pose la première ligne de base à D + pas - D = pas du haut du cadre. Le pas
 * est le plus petit multiple de 8 mm où tient la hauteur d'une ligne de LibreOffice (à un point près) : des
 * lettres trop hautes pour une ligne écrivent une ligne sur deux (ou sur trois), sans chevaucher celle d'au-dessus,
 * et la plus haute des ascendantes reste dans le cadre. À l'écran, même marge, une hauteur de ligne du pas : la
 * ligne de base tombe à la moitié du pas plus la moitié de (ascendante - descendante) au pixel, d'où le décalage.
 * Même calcul dans le moteur (`background::writing_lines`). Limite : un passage d'une autre taille ou d'une autre
 * police dans le bloc fait grandir sa ligne dans LibreOffice, pas à l'écran (les lignes suivantes ne tombent plus
 * juste).
 */
export function writingLineBox(metrics: FontMetrics | null | undefined, sizePt: number): WritingLineBox {
  const { loADots, loDDots, webA, webD } = heights(metrics, sizePt);
  const lines = Math.max(1, Math.ceil(((loADots + loDDots - 1) * MM_PER_DOT) / WRITING_LINE_MM));
  const pitchMm = lines * WRITING_LINE_MM;
  const padTopMm = loDDots * MM_PER_DOT;
  return { lineHeight: `${pitchMm}mm`, shiftMm: pitchMm - padTopMm - (pitchMm + webA - webD) / 2, padTopMm, pitchMm };
}
