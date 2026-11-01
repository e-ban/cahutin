// Préférences de l'application, gardées sur le poste (fenêtre Préférences).
import { readPref, writePref } from "./prefs";

class Settings {
  /** Barre au-dessus de l'élément choisi : icônes seules, le nom au-dessus de l'icône au survol. Par défaut,
   *  l'icône et le nom sont écrits (plus facile à lire, et sans survol sur un écran tactile). */
  compactToolbar = $state(readPref("compactToolbar", false));

  setCompactToolbar(on: boolean) {
    this.compactToolbar = on;
    writePref("compactToolbar", on);
  }
}

export const settings = new Settings();
