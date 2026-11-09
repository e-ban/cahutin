// Boîte de choix dans l'interface (plus de deux réponses possibles, comme « Enregistrer les modifications ? »).
export interface Choice<T extends string> {
  value: T;
  label: string;
  primary?: boolean;
}

class Modal {
  message = $state<string | null>(null);
  choices = $state.raw<Choice<string>[]>([]);
  #resolve: ((v: string) => void) | null = null;
  #escape: string | null = null;

  /**
   * Une nouvelle question remplace la précédente, qui reçoit sa réponse d'Échap. Celle-ci est la première réponse
   * (en principe le retour, « Revenir au document »), ou `escape` quand le premier bouton efface quelque chose.
   */
  choose<T extends string, E extends string = T>(message: string, choices: Choice<T>[], opts: { escape?: E } = {}): Promise<T | E> {
    if (this.#resolve && this.choices.length) this.#resolve(this.#escapeValue());
    this.message = message;
    this.choices = choices;
    this.#escape = opts.escape ?? null;
    return new Promise<T | E>((resolve) => (this.#resolve = resolve as (v: string) => void));
  }

  #escapeValue(): string {
    return this.#escape ?? this.choices[0].value;
  }

  /** Échap : la réponse d'Échap de la question. */
  cancel() {
    if (this.choices.length) this.answer(this.#escapeValue());
  }

  answer(value: string) {
    this.message = null;
    this.#resolve?.(value);
    this.#resolve = null;
  }
}

export const modal = new Modal();
