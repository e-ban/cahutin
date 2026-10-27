export interface History<T> {
  readonly current: T;
  readonly canUndo: boolean;
  readonly canRedo: boolean;
  readonly isDirty: boolean;
  commit(next: T): void;
  undo(): void;
  redo(): void;
  markSaved(): void;
}

/** Historique d'annulation par instantanés. `limit` borne le nombre d'annulations possibles. */
export function createHistory<T>(initial: T, limit = 100): History<T> {
  let past: T[] = [];
  let future: T[] = [];
  let current = initial;
  let saved: T | null = initial;
  return {
    get current() {
      return current;
    },
    get canUndo() {
      return past.length > 0;
    },
    get canRedo() {
      return future.length > 0;
    },
    get isDirty() {
      return current !== saved;
    },
    commit(next: T) {
      past.push(current);
      if (past.length > limit) past = past.slice(past.length - limit);
      future = [];
      current = next;
    },
    undo() {
      const prev = past.pop();
      if (prev === undefined) return;
      future.push(current);
      current = prev;
    },
    redo() {
      const next = future.pop();
      if (next === undefined) return;
      past.push(current);
      current = next;
    },
    markSaved() {
      saved = current;
    },
  };
}
