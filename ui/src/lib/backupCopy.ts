// Copie de secours de cette fenêtre : le dernier état copié, oublié quand la copie est effacée pour qu'un
// document resté modifié soit copié de nouveau au tour suivant.
import * as api from "./api";
import type { Document } from "./types";

export const copyState: { lastBacked: Document | null } = { lastBacked: null };

/** Efface la copie de cette fenêtre (enregistré, ou document quitté) ; une erreur est ignorée. */
export async function clearBackup(): Promise<void> {
  copyState.lastBacked = null;
  await api.backupClear().catch(() => {});
}
