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

class Settings {
  /** Barre au-dessus de l'élément choisi : icônes seules, le nom au-dessus de l'icône au survol. Par défaut,
   *  l'icône et le nom sont écrits (plus facile à lire, et sans survol sur un écran tactile). */
  compactToolbar = $state(readPref("compactToolbar", false));

  /** Les outils moins utilisés (miroir, quarts de tour, frise, répéter, découpe…) dans un menu « Plus » plutôt
   *  que dans la barre : désactivé par défaut, tout est visible d'un coup d'œil. */
  moreMenu = $state(readPref("moreMenu", false));

  uiScale = $state<UiScale>(readPref("uiScale", 1));
  uiFont = $state<UiFont>(readPref("uiFont", "Atkinson Hyperlegible"));
  /** Textes plus foncés, bordures et sélection plus marquées, fonds blancs. */
  highContrast = $state(readPref("highContrast", false));
  /** Poignées deux fois plus grosses : écran tactile, tableau blanc, souris difficile. */
  bigHandles = $state(readPref("bigHandles", false));

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

type Stored = "uiScale" | "uiFont" | "highContrast" | "bigHandles" | "pageFormat" | "pageLandscape" | "textFont" | "graphismeLevel" | "preferredExport";

export const settings = new Settings();
