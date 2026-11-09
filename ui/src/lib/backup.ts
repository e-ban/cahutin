// Copie de secours : toutes les deux minutes, le document modifié et pas encore enregistré est écrit dans le
// dossier de l'application ; après une panne, Cahutin propose de le reprendre au lancement suivant.
import * as api from "./api";
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

let lastBacked: Document | null = null;
let running = false;

/** Écrit la copie si besoin ; une erreur est ignorée (la copie précédente reste). */
export async function backupNow(): Promise<void> {
  if (running || !shouldBackup(editor.dirty, isSaving(), editor.doc, lastBacked)) return;
  running = true;
  const doc = editor.doc;
  try {
    await api.backupSave(doc, editor.path);
    lastBacked = doc;
    // Enregistré ou quitté pendant l'écriture : la copie, effacée entre-temps, ne doit pas revenir.
    if (!editor.dirty) await api.backupClear();
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

/** Au lancement : propose de reprendre la copie laissée par une panne. Vrai si le document a été repris. */
export async function offerBackup(): Promise<boolean> {
  const info = await api.backupInfo();
  if (!info) return false;
  // Échap ne choisit rien : la copie reste, proposée au prochain lancement. Seul « Non, l'oublier » l'efface.
  const choice = await modal.choose(
    fr.backupFound(info.title.trim() || fr.untitled, when(info.saved_at)),
    [
      { value: "discard", label: fr.backupDiscard },
      { value: "open", label: fr.backupOpen, primary: true },
    ],
    { escape: "later" },
  );
  if (choice === "discard") await api.backupClear();
  if (choice !== "open") return false;
  let opened;
  try {
    opened = await api.backupOpen();
  } catch (e) {
    // Copie illisible : on le dit ; elle reste, « Non, l'oublier » l'effacera au prochain lancement.
    editor.notice = String(e);
    return false;
  }
  editor.reset(opened.doc, info.source, { unsaved: true });
  await showOpened(opened);
  return true;
}
