// Actions sur les fichiers : nouveau, ouvrir, enregistrer, exporter, imprimer, rapport de problème.
import { open, save as saveDialog } from "@tauri-apps/plugin-dialog";
import * as api from "./api";
import { ask } from "./ask";
import { confirmDiscardIfDirty, editor, type DiscardChoice } from "./docStore.svelte";
import { newDocument } from "./factory";
import { modal } from "./modal.svelte";

const baseName = () => (editor.doc.title.trim() || "Sans titre").replace(/[\\/:*?"<>|]/g, "-");

function withExt(path: string, ext: string): string {
  return path.toLowerCase().endsWith(`.${ext}`) ? path : `${path}.${ext}`;
}

const fileName = (path: string) => path.split(/[\\/]/).pop() ?? path;

async function guard<T>(action: () => Promise<T>): Promise<T | undefined> {
  try {
    return await action();
  } catch (e) {
    editor.notice = String(e);
    return undefined;
  }
}

let saving = false;

/** Écrit l'état du document au moment de l'appel ; ce qui change pendant l'écriture reste « modifié ». */
async function write(path: string): Promise<boolean> {
  if (saving) return false;
  saving = true;
  try {
    const snapshot = editor.doc;
    const ok = await guard(() => api.saveDocument(path, snapshot));
    if (ok === undefined) return false;
    editor.markSaved(path, snapshot);
    return true;
  } finally {
    saving = false;
  }
}

export async function saveAs(): Promise<boolean> {
  editor.flushPending();
  const picked = await saveDialog({ defaultPath: `${baseName()}.carnet`, filters: [{ name: "Document Carnet", extensions: ["carnet"] }] });
  if (!picked) return false;
  return write(withExt(picked, "carnet"));
}

export async function save(): Promise<boolean> {
  editor.flushPending();
  if (!editor.path) return saveAs();
  return write(editor.path);
}

function askToSave(): Promise<DiscardChoice> {
  return modal.choose<DiscardChoice>("Enregistrer les modifications ?", [
    { value: "cancel", label: "Annuler" },
    { value: "discard", label: "Ne pas enregistrer" },
    { value: "save", label: "Enregistrer", primary: true },
  ]);
}

export const confirmLeave = () => confirmDiscardIfDirty(askToSave, save);

export async function newDoc() {
  if (!(await confirmLeave())) return;
  editor.reset(newDocument(), null);
  editor.templateMode = false;
  editor.showPicker = true;
}

export async function openDoc() {
  if (!(await confirmLeave())) return;
  const picked = await open({ filters: [{ name: "Document Carnet", extensions: ["carnet"] }] });
  if (typeof picked !== "string") return;
  const doc = await guard(() => api.openDocument(picked));
  if (!doc) return;
  editor.reset(doc, picked);
  editor.showPicker = false;
  editor.templateMode = false;
}

export async function exportOdg() {
  editor.flushPending();
  const picked = await saveDialog({ defaultPath: `${baseName()}.odg`, filters: [{ name: "LibreOffice Draw", extensions: ["odg"] }] });
  if (!picked) return;
  const path = withExt(picked, "odg");
  if ((await guard(() => api.exportOdg(path, editor.doc))) !== undefined) editor.notice = `Exporté pour LibreOffice : ${fileName(path)}`;
}

export function print() {
  editor.flushPending();
  editor.selection = [];
  editor.cropping = null;
  editor.editingText = null;
  // Laisser l'interface se mettre à jour avant d'ouvrir la boîte d'impression.
  requestAnimationFrame(() => window.print());
}

export async function exportPdf() {
  editor.flushPending();
  const picked = await saveDialog({ defaultPath: `${baseName()}.pdf`, filters: [{ name: "PDF", extensions: ["pdf"] }] });
  if (!picked) return;
  const path = withExt(picked, "pdf");
  editor.selection = [];
  const first = editor.doc.pages[0].size;
  if ((await guard(() => api.exportPdf(path, first.width_mm > first.height_mm))) !== undefined) editor.notice = `PDF enregistré : ${fileName(path)}`;
}

export async function problemReport() {
  const picked = await saveDialog({ defaultPath: "rapport-carnet.txt", filters: [{ name: "Texte", extensions: ["txt"] }] });
  if (!picked) return;
  const path = withExt(picked, "txt");
  if ((await guard(() => api.exportProblemReport(path))) !== undefined) {
    editor.notice = `Rapport enregistré : ${fileName(path)}. Il ne contient ni photo, ni texte, ni nom de fichier : vous pouvez l'envoyer.`;
  }
}

export async function deletePage(index: number) {
  if (editor.doc.pages.length === 1) return;
  if (!(await ask(`Supprimer la page ${index + 1} ?`, "Supprimer"))) return;
  editor.commit({ ...editor.doc, pages: editor.doc.pages.filter((_, i) => i !== index) });
}
