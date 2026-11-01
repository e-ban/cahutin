/** Une couleur proposée dans les panneaux, avec son nom (dit au survol, écrit pour les daltoniens). */
export type Swatch = { color: string; name: string };

/** Couleurs proposées dans les panneaux (en plus du sélecteur libre). */
export const PALETTE: Swatch[] = [
  { color: "#222222", name: "Noir" },
  { color: "#c0392b", name: "Rouge" },
  { color: "#e67e22", name: "Orange" },
  { color: "#f1c40f", name: "Jaune" },
  { color: "#27ae60", name: "Vert" },
  { color: "#2e86de", name: "Bleu" },
  { color: "#8e44ad", name: "Violet" },
  { color: "#ffffff", name: "Blanc" },
];

/** Couleurs que les daltoniens distinguent entre elles (palette d'Okabe et Ito) : pour des fiches lisibles par
 *  un élève daltonien. */
export const CVD_PALETTE: Swatch[] = [
  { color: "#000000", name: "Noir" },
  { color: "#d55e00", name: "Vermillon" },
  { color: "#e69f00", name: "Orange" },
  { color: "#f0e442", name: "Jaune" },
  { color: "#009e73", name: "Vert bleuté" },
  { color: "#56b4e9", name: "Bleu ciel" },
  { color: "#0072b2", name: "Bleu" },
  { color: "#cc79a7", name: "Rose" },
  { color: "#ffffff", name: "Blanc" },
];

/** Le nom d'une couleur des deux palettes, ou null pour une couleur choisie librement. */
export function colorName(color: string | null): string | null {
  if (!color) return null;
  const c = color.toLowerCase();
  return [...PALETTE, ...CVD_PALETTE].find((s) => s.color === c)?.name ?? null;
}
