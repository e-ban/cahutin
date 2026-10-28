// Astuces : une bulle discrète liée au contexte, montrée au plus 3 fois chacune (compteurs sur le poste).
import { readPref, writePref } from "./prefs";

export const HINT_MAX_SHOWS = 3;
export type HintId = "dblclick-crop" | "drop-photo" | "dblclick-text" | "rightclick";

export function nextHint(counts: Record<string, number>, wanted: HintId): HintId | null {
  return (counts[wanted] ?? 0) < HINT_MAX_SHOWS ? wanted : null;
}

class Hints {
  counts = $state<Record<string, number>>(readPref("hints", {}));

  /** Compte une apparition de l'astuce. */
  shown(id: HintId) {
    this.counts = { ...this.counts, [id]: (this.counts[id] ?? 0) + 1 };
    writePref("hints", this.counts);
  }
}

export const hints = new Hints();
