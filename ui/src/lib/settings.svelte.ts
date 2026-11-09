// Préférences de l'application, gardées sur le poste (fenêtre Préférences).
import type { Level } from "./graphisme";
import type { Format } from "./pageFormat";
import { readPref, writePref } from "./prefs";

/** Taille de l'interface : menus, boutons et tiroirs (la page suit, en « Ajuster »). */
export const UI_SCALES = [1, 1.25, 1.5] as const;
export type UiScale = (typeof UI_SCALES)[number];

/** Police des menus et des boutons : l'habituelle, très lisible, ou pour les lecteurs dyslexiques. */
export const UI_FONTS = ["Atkinson Hyperlegible", "Andika", "OpenDyslexic"] as const;
export type UiFont = (typeof UI_FONTS)[number];

export type ExportKind = "pdf" | "odg";

/** Attente avant la languette du nom d'un outil (barre en icônes seules), en secondes. */
export const LABEL_DELAYS = [0, 0.5, 1] as const;
export type LabelDelay = (typeof LABEL_DELAYS)[number];

/** Animations réduites par défaut si le système le demande. */
function systemReducesMotion(): boolean {
  try {
    return matchMedia("(prefers-reduced-motion: reduce)").matches;
  } catch {
    return false;
  }
}

class Settings {
  /** Barre au-dessus de l'élément choisi : icônes seules, le nom au-dessus de l'icône au survol. Par défaut,
   *  l'icône et le nom sont écrits (plus facile à lire, et sans survol sur un écran tactile). */
  compactToolbar = $state(readPref("compactToolbar", false));

  /** Les outils moins utilisés (miroir, quarts de tour, frise, répéter, découpe…) dans un menu « Plus » plutôt
   *  que dans la barre : activé par défaut, pour une barre courte ; tout afficher d'un coup d'œil reste un choix
   *  des Préférences. */
  moreMenu = $state(readPref("moreMenu", true));

  uiScale = $state<UiScale>(readPref("uiScale", 1));
  uiFont = $state<UiFont>(readPref("uiFont", "Atkinson Hyperlegible"));
  /** Textes plus foncés, bordures et sélection plus marquées, fonds blancs. */
  highContrast = $state(readPref("highContrast", false));
  /** Poignées deux fois plus grosses : écran tactile, tableau blanc, souris difficile. */
  bigHandles = $state(readPref("bigHandles", false));
  /** Rien ne glisse ni ne s'anime. */
  reduceMotion = $state(readPref("reduceMotion", systemReducesMotion()));
  labelDelay = $state<LabelDelay>(readPref("labelDelay", 0));

  /** Daltonisme : chaque couleur proposée dit son nom ; ce qui ne se distingue que par la couleur reçoit aussi un
   *  signe ; couleurs proposées que les daltoniens distinguent ; bouton qui montre la page comme ils la voient. */
  colorNames = $state(readPref("colorNames", true));
  doubleCues = $state(readPref("doubleCues", true));
  cvdPalette = $state(readPref("cvdPalette", false));
  cvdPreview = $state(readPref("cvdPreview", false));

  /** Une nouvelle page (document neuf, « Ajouter une page ») : son format et la police de ses textes. */
  pageFormat = $state<Format>(readPref("pageFormat", "A4"));
  pageLandscape = $state(readPref("pageLandscape", false));
  textFont = $state(readPref("textFont", "Andika"));

  /** Section choisie d'avance pour les fiches de graphisme (la taille des motifs). */
  graphismeLevel = $state<Level>(readPref("graphismeLevel", "MS"));
  /** L'export proposé en premier dans « Imprimer / Exporter ». */
  preferredExport = $state<ExportKind>(readPref("preferredExport", "pdf"));

  setMoreMenu(on: boolean) {
    this.moreMenu = on;
    writePref("moreMenu", on);
  }

  setCompactToolbar(on: boolean) {
    this.compactToolbar = on;
    writePref("compactToolbar", on);
  }

  /** Change un réglage et le garde sur le poste. */
  set<K extends Stored>(key: K, value: Settings[K]) {
    (this as Settings)[key] = value;
    writePref(key, value);
  }
}

type Stored =
  | "uiScale"
  | "uiFont"
  | "highContrast"
  | "bigHandles"
  | "reduceMotion"
  | "labelDelay"
  | "colorNames"
  | "doubleCues"
  | "cvdPalette"
  | "cvdPreview"
  | "pageFormat"
  | "pageLandscape"
  | "textFont"
  | "graphismeLevel"
  | "preferredExport";

export const settings = new Settings();
