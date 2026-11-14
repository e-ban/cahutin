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
import type { FontMetrics } from "./types";

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

/** Facteur et décalage d'un bloc de `sizePt` dans une police de `metrics`, à l'interligne `spacing`. */
export function lineBox(spacing: number | null | undefined, metrics: FontMetrics | null | undefined, sizePt: number): LineBox {
  if (spacing == null || !(spacing > 0) || !(sizePt > 0)) return { lineHeight: null, shiftMm: 0 };
  const m = metrics && metrics.units_per_em > 0 && metrics.ascent + metrics.descent > 0 ? metrics : FALLBACK_METRICS;
  const em = (units: number) => units / m.units_per_em;
  // LibreOffice : au point à 600 ppp.
  const dots = Math.round((sizePt * 600) / 72);
  const loA = Math.round(dots * em(m.ascent)) * MM_PER_DOT;
  const loD = Math.round(dots * em(m.descent)) * MM_PER_DOT;
  const loN = loA + loD;
  const pitch = spacing * loN;
  const loBaseline = spacing >= 1 ? pitch - loD : Math.min(loA, 0.8 * pitch);
  // Moteur web : au pixel CSS, l'espace ajouté partagé en deux.
  const px = (sizePt * 96) / 72;
  const webA = Math.round(px * em(m.ascent)) * MM_PER_PX;
  const webD = Math.round(px * em(m.descent)) * MM_PER_PX;
  const webBaseline = webA + (pitch - (webA + webD)) / 2;
  return { lineHeight: pitch / (sizePt * MM_PER_PT), shiftMm: loBaseline - webBaseline };
}
