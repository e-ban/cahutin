// La date d'une page, demandée dans une petite fenêtre (DateDialog) : à la création d'une page depuis un modèle
// qui a des champs de date, ou à la pose d'un tel champ. Elle s'ouvre sur la date du jour.

/** La date du jour, « 2026-11-09 », celle de l'ordinateur. */
export function todayIso(): string {
  const d = new Date();
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`;
}

export class DateAsk {
  open = $state(false);
  value = $state("");
  /** À la pose d'un champ, « Annuler » ne pose rien ; à la création d'une page, la fenêtre n'a que « Valider ». */
  cancellable = $state(false);
  #resolve: ((v: string | null) => void) | null = null;

  /** La date choisie (« 2026-11-09 »), ou `null` si l'on a annulé. */
  ask(cancellable: boolean): Promise<string | null> {
    this.#resolve?.(this.cancellable ? null : todayIso());
    this.value = todayIso();
    this.cancellable = cancellable;
    this.open = true;
    return new Promise((resolve) => (this.#resolve = resolve));
  }

  #end(v: string | null) {
    this.open = false;
    this.#resolve?.(v);
    this.#resolve = null;
  }

  confirm() {
    this.#end(this.value || todayIso());
  }

  cancel() {
    this.#end(this.cancellable ? null : todayIso());
  }

  /** Fermée sans répondre (Échap) : annuler si on peut, sinon la date du jour. */
  close() {
    this.cancel();
  }
}

export const dateAsk = new DateAsk();
