// Copie de secours : toutes les deux minutes, le document modifié et pas encore enregistré est écrit dans le
// dossier de l'application ; après une panne, Cahutin propose de le reprendre au lancement suivant.
import * as api from "./api";
import { clearBackup, copyState } from "./backupCopy";
import { editor } from "./docStore.svelte";
import { isSaving, showOpened } from "./fileActions";
import { fr } from "./i18n/fr";
import { modal } from "./modal.svelte";
import type { Document } from "./types";

export const BACKUP_INTERVAL_MS = 120_000;

/** Une copie seulement pour un document modifié depuis la dernière copie, et jamais pendant un enregistrement. */
export function shouldBackup(dirty: boolean, saving: boolean, doc: Document, lastBacked: Document | null): boolean {
  return dirty && !saving && doc !== lastBacked;
}

let running = false;

/** Écrit la copie si besoin ; une erreur est ignorée (la copie précédente reste). */
export async function backupNow(): Promise<void> {
  // Pendant une proposition de l'assistant, la copie prend le document de départ : rien de non accepté n'y entre.
  const doc = editor.committed;
  if (running || !shouldBackup(editor.dirty, isSaving(), doc, copyState.lastBacked)) return;
  running = true;
  try {
    await api.backupSave(doc, editor.path);
    copyState.lastBacked = doc;
    // Enregistré ou quitté pendant l'écriture : la copie, effacée entre-temps, ne doit pas revenir.
    if (!editor.dirty) await clearBackup();
  } catch {
    // Rien à dire à l'utilisateur : la prochaine copie réessaiera.
  } finally {
    running = false;
  }
}

/** Lance les copies régulières ; rend de quoi les arrêter. */
export function startBackups(): () => void {
  const timer = setInterval(() => void backupNow(), BACKUP_INTERVAL_MS);
  return () => clearInterval(timer);
}

/** Heure locale d'une copie (« 9 novembre 2026 à 17:05 »). */
function when(savedAt: string): string {
  const d = new Date(savedAt);
  return Number.isNaN(d.getTime()) ? savedAt : d.toLocaleString("fr-FR", { dateStyle: "long", timeStyle: "short" });
}

/**
 * Au lancement : propose de reprendre la copie laissée par une fenêtre arrêtée sans enregistrer (panne). Vrai
 * si le document a été repris. La copie d'une autre fenêtre encore ouverte n'est jamais proposée.
 */
export async function offerBackup(): Promise<boolean> {
  const offer = await api.backupInfo();
  if (!offer) return false;
  const info = offer.info;
  // Échap ne choisit rien : la copie reste, proposée au prochain lancement. Seul « Non, l'oublier » l'efface.
  const choice = await modal.choose(
    fr.backupFound(info.title.trim() || fr.untitled, when(info.saved_at)),
    [
      { value: "discard", label: fr.backupDiscard },
      { value: "open", label: fr.backupOpen, primary: true },
    ],
    { escape: "later" },
  );
  if (choice === "discard") await api.backupDiscard(offer.id);
  if (choice !== "open") return false;
  let restored;
  try {
    restored = await api.backupOpen(offer.id);
  } catch (e) {
    // Copie illisible : on le dit ; elle reste, « Non, l'oublier » l'effacera au prochain lancement.
    editor.notice = String(e);
    return false;
  }
  // Sans `source` (original enregistré depuis la copie) : « Sans titre », pour ne pas l'écraser.
  editor.reset(restored.opened.doc, restored.source, { unsaved: true });
  await showOpened(restored.opened);
  return true;
}
