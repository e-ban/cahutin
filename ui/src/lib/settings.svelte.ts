// Préférences de l'application, gardées sur le poste (fenêtre Préférences).
import { readPref, writePref } from "./prefs";

class Settings {
  /** Barre au-dessus de l'élément choisi : icônes seules, le nom au-dessus de l'icône au survol. Par défaut,
   *  l'icône et le nom sont écrits (plus facile à lire, et sans survol sur un écran tactile). */
  compactToolbar = $state(readPref("compactToolbar", false));

  /** Les outils moins utilisés (miroir, quarts de tour, frise, répéter, découpe…) dans un menu « Plus » plutôt
   *  que dans la barre : désactivé par défaut, tout est visible d'un coup d'œil. */
  moreMenu = $state(readPref("moreMenu", false));

  setMoreMenu(on: boolean) {
    this.moreMenu = on;
    writePref("moreMenu", on);
  }

  setCompactToolbar(on: boolean) {
    this.compactToolbar = on;
    writePref("compactToolbar", on);
  }
}

export const settings = new Settings();
