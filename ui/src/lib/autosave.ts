// Enregistrement automatique : un document qui a un fichier choisi par l'enseignant (ouvert ou enregistré par
// lui) y est écrit tout seul, quelques secondes après le dernier changement, et au plus tard une demi-minute
// après le premier d'une suite de changements rapprochés. Jamais pendant une écriture de texte, un geste, un
// aperçu, un recadrage, une proposition de l'assistant ou un autre enregistrement : on attend qu'ils finissent.
// Le minuteur lui-même ne connaît ni l'éditeur ni le disque (lib/autosave.svelte.ts les branche) : il se teste
// avec de faux minuteurs.

/** Après le dernier changement : un enregistrement de document chargé de photos prend bien moins (mesuré :
 *  115 Mo en 50 ms sur un disque rapide), une clé USB lente plusieurs secondes ; une écriture à la fois. */
export const AUTOSAVE_DELAY_MS = 4_000;
/** Une suite de changements rapprochés finit par être écrite. */
export const AUTOSAVE_MAX_WAIT_MS = 30_000;
/** Le minuteur regarde l'état chaque seconde (une comparaison, rien d'écrit). */
export const AUTOSAVE_TICK_MS = 1_000;

export interface AutosaveView<D> {
  /** Le réglage « Enregistrer automatiquement ». */
  enabled: boolean;
  /** Le fichier de l'enseignant, ou rien (document sans fichier, exemple, copie de secours reprise…). */
  path: string | null;
  dirty: boolean;
  /** Le document validé (jamais un aperçu ni une proposition). */
  doc: D;
  /** Change quand un autre document prend la place (nouveau, ouvert, repris). */
  generation: number;
  /** Une écriture de texte, un geste, un aperçu, un recadrage, une proposition ou un enregistrement en cours. */
  blocked: boolean;
  /** Retenu après un remplissage par la classe (`ClassHold`) : rien n'est écrit avant un enregistrement à la main. */
  held: boolean;
}

/** Ce qui empêche d'enregistrer, dit une fois : un échec (lecture seule, disque plein, clé retirée) ou un fichier
 *  changé par ailleurs. */
export type AutosaveProblem = { kind: "failed"; message: string } | { kind: "changed" } | null;

export interface AutosaveDeps<D> {
  view(): AutosaveView<D>;
  /** Écrit `doc` dans `path` ; « changed » : rien n'est écrit, le fichier a changé par ailleurs. Lève en cas d'échec. */
  write(path: string, doc: D, generation: number): Promise<"saved" | "changed">;
  /** Un problème commence (une seule fois tant qu'il dure), ou finit (`null`). */
  problem(p: AutosaveProblem): void;
  now(): number;
}

export class Autosaver<D> {
  #deps: AutosaveDeps<D>;
  #generation = -1;
  #enabled = false;
  #seen: D | null = null;
  #last = 0;
  #first: number | null = null;
  #inFlight: Promise<boolean> | null = null;
  /** Le document dont l'écriture a échoué : on réessaie au changement suivant, pas avant. */
  #failedDoc: D | null = null;
  #problem: AutosaveProblem = null;

  constructor(deps: AutosaveDeps<D>) {
    this.#deps = deps;
  }

  get problem(): AutosaveProblem {
    return this.#problem;
  }

  #setProblem(p: AutosaveProblem) {
    if (p === null && this.#problem === null) return;
    this.#problem = p;
    this.#deps.problem(p);
  }

  /** Un tour du minuteur : écrit si le moment est venu. */
  tick() {
    const now = this.#deps.now();
    const v = this.#deps.view();
    if (v.generation !== this.#generation) {
      // Un autre document : ses problèmes ne sont pas ceux du précédent.
      this.#generation = v.generation;
      this.#seen = v.doc;
      this.#first = null;
      this.#failedDoc = null;
      this.#setProblem(null);
    }
    if (!v.enabled) {
      this.#enabled = false;
      return;
    }
    if (!this.#enabled) {
      // Réglage tout juste coché : un document déjà modifié attend le délai normal, à partir de maintenant.
      this.#enabled = true;
      this.#first = null;
    }
    if (!v.path || !v.dirty || v.held) {
      this.#first = null;
      this.#seen = v.doc;
      return;
    }
    // Fichier changé ailleurs, ou échec sans nouveau changement : rien ne court, le délai repartira du prochain.
    if (this.#problem?.kind === "changed" || this.#failedDoc === v.doc) {
      this.#first = null;
      return;
    }
    if (v.doc !== this.#seen) {
      this.#seen = v.doc;
      this.#last = now;
    }
    if (this.#first === null) {
      this.#first = now;
      this.#last = Math.max(this.#last, now);
    }
    if (this.#inFlight || v.blocked) return;
    if (now - this.#last >= AUTOSAVE_DELAY_MS || now - this.#first >= AUTOSAVE_MAX_WAIT_MS) void this.#write(v);
  }

  async #write(v: AutosaveView<D>): Promise<boolean> {
    const run = async () => {
      try {
        const r = await this.#deps.write(v.path!, v.doc, v.generation);
        if (r === "changed") {
          if (this.#problem?.kind !== "changed") this.#setProblem({ kind: "changed" });
          return false;
        }
        this.#failedDoc = null;
        this.#setProblem(null);
        return true;
      } catch (e) {
        this.#failedDoc = v.doc;
        if (this.#problem?.kind !== "failed") this.#setProblem({ kind: "failed", message: String(e) });
        return false;
      } finally {
        // Ce qui a changé pendant l'écriture attend le délai à partir de sa fin (une clé USB lente n'écrit pas
        // sans arrêt).
        this.#first = null;
      }
    };
    const p = run();
    this.#inFlight = p;
    try {
      return await p;
    } finally {
      this.#inFlight = null;
    }
  }

  /** Avant de quitter le document : l'écriture en cours est attendue, celle qui attend part tout de suite. Vrai
   *  si le document est enregistré ; faux s'il faut poser la question d'avant (réglage décoché, document sans
   *  fichier, retenu après un remplissage par la classe, fichier changé par ailleurs, échec). */
  async flush(): Promise<boolean> {
    while (this.#inFlight) await this.#inFlight;
    const v = this.#deps.view();
    if (!v.enabled || !v.path || v.held || this.#problem?.kind === "changed") return false;
    if (!v.dirty) return true;
    return (await this.#write(v)) && !this.#deps.view().dirty;
  }

  /** Enregistré à la main (Ctrl+S, « Enregistrer sous », « Remplacer ») : les problèmes sont réglés. */
  cleared() {
    this.#failedDoc = null;
    this.#first = null;
    this.#setProblem(null);
  }

  /** Lance les tours réguliers ; rend de quoi les arrêter. */
  start(): () => void {
    const timer = setInterval(() => this.tick(), AUTOSAVE_TICK_MS);
    return () => clearInterval(timer);
  }
}

/**
 * Retenue après un remplissage par la classe : choisir une classe (les champs école, enseignant... qui attendaient
 * s'écrivent), « Remplir avec ma classe », la classe changée ou complétée, la classe rouverte au lancement mettent
 * des prénoms et des photos d'élèves dans le document sans geste de l'enseignant. Écrits tout seuls, ils
 * remplaceraient son modèle dans le fichier : l'écriture automatique de ce document attend qu'il enregistre
 * lui-même (Ctrl+S, Enregistrer, « Enregistrer sous », ou « Enregistrer » en partant). Propre au document ouvert :
 * un autre document repart sans retenue.
 *
 * `before` : le document juste avant le premier remplissage retenu, `filled` juste après (l'historique rend les
 * mêmes objets à Ctrl+Z et Ctrl+Y) ; `null` : inconnu (remplissage pendant une proposition de l'assistant),
 * retenu jusqu'à l'enregistrement. `on` : en vigueur ; levée tant que Ctrl+Z est revenu à `before`.
 */
export interface ClassHold<D> {
  before: D | null;
  filled: D | null;
  on: boolean;
}

/** Un remplissage par la classe a changé `before` en `filled`. Déjà retenu : le point de départ reste celui du
 *  premier remplissage (Ctrl+Z doit tous les défaire). Toujours un nouvel objet : un enregistrement à la main en
 *  cours sait qu'un remplissage est venu pendant l'écriture. */
export function classHoldAfterFill<D>(h: ClassHold<D> | null, before: D | null, filled: D | null): ClassHold<D> {
  return h?.on ? { ...h } : { before, filled, on: true };
}

/** L'historique a changé (modification, Ctrl+Z, Ctrl+Y) : `doc` est le document validé. Revenu avant le
 *  remplissage, rien de la classe n'y est : la retenue est levée ; Ctrl+Y qui ramène le remplissage la remet. */
export function classHoldAfterHistory<D>(h: ClassHold<D> | null, doc: D): ClassHold<D> | null {
  if (!h || h.before === null) return h;
  if (doc === h.before) return h.on ? { ...h, on: false } : h;
  if (doc === h.filled && !h.on) return { ...h, on: true };
  return h;
}

/** Enregistré à la main (`atStart` : la retenue au début de l'écriture) : levée, sauf si un remplissage est venu
 *  pendant l'écriture, ou si le document écrit est celui d'avant le remplissage (Ctrl+Y le ramènerait). */
export function classHoldAfterSave<D>(h: ClassHold<D> | null, atStart: ClassHold<D> | null): ClassHold<D> | null {
  return h && (h !== atStart || !h.on) ? h : null;
}

export type SaveStatus = "saved" | "saving" | "unsaved";

/** Ce que dit l'indicateur près du titre : « Enregistré », « Enregistrement… » (des changements partent dans
 *  quelques secondes, ou un texte en cours d'écriture, un recadrage, qui partiront à la sortie du bloc) ou « Non
 *  enregistré » (document sans fichier, réglage décoché et document modifié, retenu après un remplissage par la
 *  classe, échec, fichier changé par ailleurs). */
export function saveStatus(s: { enabled: boolean; path: string | null; ownFile: boolean; dirty: boolean; editing: boolean; problem: AutosaveProblem; held: boolean }): SaveStatus {
  if (!s.path) return "unsaved";
  const auto = s.enabled && s.ownFile && !s.held;
  // Un texte en cours d'écriture n'est encore ni dans le fichier ni dans la copie de secours : pas « Enregistré ».
  if (!s.dirty && !(s.editing && auto)) return "saved";
  return auto && !s.problem ? "saving" : "unsaved";
}

/** Heure locale d'une date enregistrée en UTC (« 9 novembre 2026 à 17:05 »). */
export function localTime(utc: string): string {
  const d = new Date(utc);
  return Number.isNaN(d.getTime()) ? utc : d.toLocaleString("fr-FR", { dateStyle: "long", timeStyle: "short" });
}
