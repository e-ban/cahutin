// Sections repliables du panneau Réglages : ouvertes ou fermées comme on les a laissées, sur ce poste.
import { readPref, writePref } from "./prefs";

const KEY = "inspectorSections";

export class InspectorSections {
  #open = $state<Record<string, boolean>>(readPref(KEY, {}));

  /** Ouverte ou fermée ; jamais touchée, `byDefault`. */
  isOpen(id: string, byDefault: boolean): boolean {
    return this.#open[id] ?? byDefault;
  }

  set(id: string, open: boolean) {
    this.#open = { ...this.#open, [id]: open };
    writePref(KEY, this.#open);
  }
}

export const inspectorSections = new InspectorSections();
