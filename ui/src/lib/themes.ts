// Les thèmes de l'interface : les couleurs de Cahutin autour de la page. La page elle-même reste blanche, comme
// sur papier, quel que soit le thème (PageView remet ses couleurs). Le contraste élevé passe avant le thème.

/** Les couleurs que l'interface emploie, par rôle ; chaque composant passe par elles (var(--surface)…). */
export interface ThemeColors {
  /** Fond de la fenêtre, autour de la page. */
  bg: string;
  /** Barres, tiroirs, panneaux. */
  panel: string;
  /** Boutons, champs, menus, cartes : ce qui se pose sur un panneau. */
  surface: string;
  line: string;
  text: string;
  muted: string;
  accent: string;
  /** Texte sur un fond de la couleur d'accent (bouton principal). */
  onAccent: string;
  /** Fond teinté d'accent : survol, bouton actif, choix conseillé. */
  accentSoft: string;
  /** Le rouge du logo et des titres manuscrits. */
  accentStrong: string;
  danger: string;
  dangerSoft: string;
  warnBg: string;
  warnLine: string;
  /** Texte d'avertissement (orange foncé), lisible sur warnBg comme sur un panneau. */
  warnText: string;
  ok: string;
  shadow: string;
  /** Voile derrière une fenêtre de dialogue. */
  backdrop: string;
}

export interface Theme {
  id: ThemeId;
  dark: boolean;
  colors: ThemeColors;
}

export const THEME_IDS = ["papier", "craie", "crayon", "nuit"] as const;
export type ThemeId = (typeof THEME_IDS)[number];

const common = { danger: "#c0392b", dangerSoft: "#fdecea", warnBg: "#fff4e5", warnLine: "#f0b86e", warnText: "#8a4f00", ok: "#2e7d32", onAccent: "#ffffff" };

export const THEMES: Record<ThemeId, Theme> = {
  // Le thème d'origine : papier beige, bleu.
  papier: {
    id: "papier",
    dark: false,
    colors: {
      ...common,
      bg: "#efe9df",
      panel: "#fbf8f3",
      surface: "#ffffff",
      line: "#e2d9cb",
      text: "#2d2a26",
      muted: "#7d756b",
      accent: "#2e86de",
      accentSoft: "#e8f1fb",
      accentStrong: "#c0392b",
      shadow: "rgba(0, 0, 0, 0.15)",
      backdrop: "rgba(40, 30, 20, 0.35)",
    },
  },
  // Tableau d'école : gris-vert, vert.
  craie: {
    id: "craie",
    dark: false,
    colors: {
      ...common,
      bg: "#e3e8e3",
      panel: "#f4f6f3",
      surface: "#ffffff",
      line: "#cfd8cf",
      text: "#22302a",
      muted: "#66746c",
      accent: "#2f7d5b",
      accentSoft: "#e2f0e8",
      accentStrong: "#b5452e",
      shadow: "rgba(0, 0, 0, 0.14)",
      backdrop: "rgba(20, 35, 28, 0.38)",
    },
  },
  // Crayons de couleur : lilas pâle, violet.
  crayon: {
    id: "crayon",
    dark: false,
    colors: {
      ...common,
      bg: "#efe9f6",
      panel: "#fbf9fe",
      surface: "#ffffff",
      line: "#e0d7ee",
      text: "#2e293a",
      muted: "#776e88",
      accent: "#7652c2",
      accentSoft: "#efe8fb",
      accentStrong: "#c2416e",
      shadow: "rgba(30, 10, 60, 0.15)",
      backdrop: "rgba(35, 20, 60, 0.35)",
    },
  },
  // Sombre : pour travailler le soir, la page reste blanche.
  nuit: {
    id: "nuit",
    dark: true,
    colors: {
      bg: "#1c1b19",
      panel: "#262421",
      surface: "#302d29",
      line: "#46423c",
      text: "#ece6dc",
      muted: "#a8a094",
      accent: "#6aaef0",
      onAccent: "#0f1d2b",
      accentSoft: "#253546",
      accentStrong: "#ef8a77",
      danger: "#f08a7b",
      dangerSoft: "#3d2523",
      warnBg: "#3a2f1d",
      warnLine: "#8c6a2c",
      warnText: "#f0b86a",
      ok: "#7cc48a",
      shadow: "rgba(0, 0, 0, 0.5)",
      backdrop: "rgba(0, 0, 0, 0.55)",
    },
  },
};

/** Contraste élevé : textes noirs, bordures foncées, fonds blancs, bleu plus sombre ; passe avant le thème. */
export const HIGH_CONTRAST: ThemeColors = {
  bg: "#ffffff",
  panel: "#ffffff",
  surface: "#ffffff",
  line: "#5c554c",
  text: "#000000",
  muted: "#3a352f",
  accent: "#0b4f99",
  onAccent: "#ffffff",
  accentSoft: "#dbe8f7",
  accentStrong: "#8e1d12",
  danger: "#a3160c",
  dangerSoft: "#fde3df",
  warnBg: "#fff1d6",
  warnLine: "#8a5a00",
  warnText: "#6b3f00",
  ok: "#1b5e20",
  shadow: "rgba(0, 0, 0, 0.3)",
  backdrop: "rgba(0, 0, 0, 0.5)",
};

/** Le nom CSS d'une couleur : accentSoft → --accent-soft. */
export const cssVar = (key: string) => `--${key.replace(/[A-Z]/g, (c) => `-${c.toLowerCase()}`)}`;

/** Les variables CSS à poser sur la racine, et si l'interface est sombre. */
export function themeVars(id: ThemeId, highContrast: boolean): { vars: [string, string][]; dark: boolean } {
  const theme = THEMES[id] ?? THEMES.papier;
  const colors = highContrast ? HIGH_CONTRAST : theme.colors;
  return { vars: Object.entries(colors).map(([k, v]) => [cssVar(k), v]), dark: !highContrast && theme.dark };
}
