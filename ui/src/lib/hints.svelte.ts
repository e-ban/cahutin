// Astuces : une bulle discrète liée au contexte, montrée au plus 3 fois chacune (compteurs sur le poste).
import { readPref, writePref } from "./prefs";

export const HINT_MAX_SHOWS = 3;
export type HintId = "dblclick-crop" | "drop-photo" | "dblclick-text" | "rightclick" | "blur-faces" | "table-cell";

export function nextHint(counts: Record<string, number>, wanted: HintId): HintId | null {
  return (counts[wanted] ?? 0) < HINT_MAX_SHOWS ? wanted : null;
}

/** La première astuce de la liste pas encore montrée 3 fois : après le recadrage, le flou des visages. */
export function firstHint(enabled: boolean, counts: Record<string, number>, wanted: HintId[]): HintId | null {
  return enabled ? (wanted.find((id) => nextHint(counts, id)) ?? null) : null;
}

/** Astuce à montrer : aucune quand elles sont coupées (menu Aide). */
export function hintFor(enabled: boolean, counts: Record<string, number>, wanted: HintId): HintId | null {
  return enabled ? nextHint(counts, wanted) : null;
}

class Hints {
  counts = $state<Record<string, number>>(readPref("hints", {}));
  /** Astuces pendant le travail (menu Aide > « Afficher les astuces pendant le travail »). */
  enabled = $state(readPref("hintsEnabled", true));

  toggle() {
    this.enabled = !this.enabled;
    writePref("hintsEnabled", this.enabled);
  }

  /** Compte une apparition de l'astuce. */
  shown(id: HintId) {
    this.counts = { ...this.counts, [id]: (this.counts[id] ?? 0) + 1 };
    writePref("hints", this.counts);
  }
}

export const hints = new Hints();
