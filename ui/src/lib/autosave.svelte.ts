// Branche le minuteur d'enregistrement automatique (lib/autosave.ts) sur l'éditeur et le disque, et tient ce
// que l'interface en montre : le problème en cours (indicateur près du titre) et la version du début de séance
// (menu Fichier).
import * as api from "./api";
import { AUTOSAVE_TICK_MS, Autosaver, type AutosaveProblem } from "./autosave";
import { editor } from "./docStore.svelte";
import { isSaving, saveAs, writeDoc } from "./fileActions";
import { fr } from "./i18n/fr";
import { settings } from "./settings.svelte";
import type { Document } from "./types";

/** Le fichier où le document s'enregistre tout seul : celui que l'enseignant a ouvert ou enregistré. */
const ownPath = () => (editor.ownFile ? editor.path : null);

class AutosaveState {
  problem = $state<AutosaveProblem>(null);
  /** Date de la version du début de séance du document ouvert (UTC), s'il y en a une. */
  startVersion = $state<string | null>(null);

  async refreshStart() {
    const [path, generation] = [ownPath(), editor.generation];
    let at: string | null = null;
    try {
      at = path ? await api.startVersionInfo(path) : null;
    } catch {
      // Rien à proposer : le menu n'affiche pas la commande.
    }
    if (path === ownPath() && generation === editor.generation) this.startVersion = at;
  }

  /** Lance le minuteur ; rend de quoi l'arrêter. */
  start(): () => void {
    const stop = autosaver.start();
    let seen = "";
    const timer = setInterval(() => {
      const key = `${editor.generation}|${ownPath() ?? ""}`;
      if (key === seen) return;
      seen = key;
      void this.refreshStart();
    }, AUTOSAVE_TICK_MS);
    return () => {
      stop();
      clearInterval(timer);
    };
  }
}

export const autosave = new AutosaveState();

/** L'avis du problème en cours, pour l'effacer quand il est réglé. */
let shown: string | null = null;

export const autosaver = new Autosaver<Document>({
  view: () => ({
    enabled: settings.autosave,
    // En mode modèle, l'enseignant construit un modèle : rien ne part dans son fichier sans un geste (Ctrl+S, ou
    // la question d'avant en partant).
    path: editor.templateMode ? null : ownPath(),
    dirty: editor.dirty,
    // Pendant une proposition de l'assistant ou un aperçu : le document de départ (de toute façon, on attend).
    doc: editor.committed,
    generation: editor.generation,
    blocked: !!editor.editingText || editor.gesture || editor.previewing || !!editor.cropping || editor.proposing || isSaving(),
  }),
  write: async (path, doc, generation) => ((await writeDoc(path, doc, "Auto", generation)) === "ChangedElsewhere" ? "changed" : "saved"),
  problem: (p) => {
    // Le problème réglé (enregistré depuis) : son avis, s'il est encore affiché, n'a plus lieu d'être.
    if (!p && editor.notice === shown) editor.notice = null;
    autosave.problem = p;
    shown = p?.kind === "failed" ? fr.autosaveFailed(p.message) : p?.kind === "changed" ? fr.autosaveChangedElsewhere : null;
    if (p?.kind === "failed") editor.notice = shown;
    if (p?.kind === "changed") editor.notify(fr.autosaveChangedElsewhere, { label: fr.saveCopy, run: () => void saveAs() });
  },
  now: () => Date.now(),
});
