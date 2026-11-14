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
    if (!v.path || !v.dirty) {
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
   *  fichier, fichier changé par ailleurs, échec). */
  async flush(): Promise<boolean> {
    while (this.#inFlight) await this.#inFlight;
    const v = this.#deps.view();
    if (!v.enabled || !v.path || this.#problem?.kind === "changed") return false;
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

export type SaveStatus = "saved" | "saving" | "unsaved";

/** Ce que dit l'indicateur près du titre : « Enregistré », « Enregistrement… » (des changements partent dans
 *  quelques secondes, ou un texte en cours d'écriture, un recadrage, qui partiront à la sortie du bloc) ou « Non
 *  enregistré » (document sans fichier, réglage décoché et document modifié, échec, fichier changé par ailleurs). */
export function saveStatus(s: { enabled: boolean; path: string | null; ownFile: boolean; dirty: boolean; editing: boolean; problem: AutosaveProblem }): SaveStatus {
  if (!s.path) return "unsaved";
  // Un texte en cours d'écriture n'est encore ni dans le fichier ni dans la copie de secours : pas « Enregistré ».
  if (!s.dirty && !(s.editing && s.enabled && s.ownFile)) return "saved";
  return s.enabled && s.ownFile && !s.problem ? "saving" : "unsaved";
}

/** Heure locale d'une date enregistrée en UTC (« 9 novembre 2026 à 17:05 »). */
export function localTime(utc: string): string {
  const d = new Date(utc);
  return Number.isNaN(d.getTime()) ? utc : d.toLocaleString("fr-FR", { dateStyle: "long", timeStyle: "short" });
}
