// Documents récents : gardés sur le poste seulement, jamais envoyés au moteur ni journalisés.
import { readPref, writePref } from "./prefs";

export interface Recent {
  path: string;
  title: string;
  openedAt: number;
}

export const MAX_RECENTS = 8;

/** Le plus récent d'abord ; un document déjà listé remonte en tête. */
export function addRecent(list: Recent[], r: Recent, max = MAX_RECENTS): Recent[] {
  return [r, ...list.filter((x) => x.path !== r.path)].slice(0, max);
}

export const readRecents = (): Recent[] => readPref<Recent[]>("recents", []);
export const writeRecents = (list: Recent[]) => writePref("recents", list);
