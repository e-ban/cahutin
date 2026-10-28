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

  /** Une nouvelle question remplace la précédente, qui reçoit sa première réponse (en principe « Annuler »). */
  choose<T extends string>(message: string, choices: Choice<T>[]): Promise<T> {
    if (this.#resolve && this.choices.length) this.#resolve(this.choices[0].value);
    this.message = message;
    this.choices = choices;
    return new Promise<T>((resolve) => (this.#resolve = resolve as (v: string) => void));
  }

  answer(value: string) {
    this.message = null;
    this.#resolve?.(value);
    this.#resolve = null;
  }
}

export const modal = new Modal();
