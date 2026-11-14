// Tailles de texte : la liste proposée, et une taille tapée à la main (« 13,5 », « 250 pt »).

/** Bornes d'une taille tapée, les mêmes que l'assistant (crates/cahutin-mcp/src/edit.rs, set_text_style). */
export const MIN_FONT_PT = 4;
export const MAX_FONT_PT = 400;

export const FONT_SIZES = [10, 12, 14, 16, 18, 20, 24, 28, 32, 40, 48, 60, 72, 96, 120, 144, 200];

/** La taille tapée, ramenée entre les bornes et arrondie au dixième ; `null` si ce n'est pas une taille. */
export function parseFontSize(input: string): number | null {
  const m = input.trim().match(/^(\d+(?:[.,]\d*)?)\s*(?:pt)?$/i);
  if (!m) return null;
  const pt = Math.round(Number(m[1].replace(",", ".")) * 10) / 10;
  return Math.min(MAX_FONT_PT, Math.max(MIN_FONT_PT, pt));
}

/** La taille écrite à la française : « 13,5 ». */
export const formatFontSize = (pt: number) => String(Math.round(pt * 10) / 10).replace(".", ",");
