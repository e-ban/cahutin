// Actions sur les fichiers : nouveau, ouvrir, enregistrer, exporter, imprimer, rapport de problème.
import { open, save as saveDialog } from "@tauri-apps/plugin-dialog";
import { tick } from "svelte";
import * as api from "./api";
import { ask } from "./ask";
import { confirmDiscardIfDirty, editor, type DiscardChoice } from "./docStore.svelte";
import { newDocument, newId } from "./factory";
import { fonts } from "./fonts.svelte";
import { fr } from "./i18n/fr";
import { importReportText, placeImported } from "./importPages";
import { summarizeImport } from "./importSummary";
import { pendingPaths } from "./photoLibrary";
import { addRecent, writeRecents } from "./recents";
import { missingFamilies } from "./fontNames";
import type { Document, ImportedPages, ImportOutcome } from "./types";
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
    // Les photos importées mais non placées sont retrouvées à la réouverture (chemins seulement).
    const written = { ...snapshot, pending_photos: pendingPaths(editor.photos, snapshot, editor.carriedPending) };
    const ok = await guard(() => api.saveDocument(path, written));
    if (ok === undefined) return false;
    editor.markSaved(path, snapshot);
    remember(path, snapshot.title);
    return true;
  } finally {
    saving = false;
  }
}

export async function saveAs(): Promise<boolean> {
  editor.flushPending();
  const picked = await saveDialog({ defaultPath: `${baseName()}.carnet`, filters: [{ name: "Carnet (Cahutin)", extensions: ["carnet"] }] });
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
  editor.carriedPending = [];
  editor.templateMode = false;
  if (editor.drawer === "grid") editor.drawer = null;
  editor.welcomeOverDoc = false;
  editor.showWelcome = true;
}

/** L'accueil, par-dessus le document ouvert : on y revient sans rien perdre. */
export function goHome() {
  editor.flushPending();
  editor.selection = [];
  editor.editingText = null;
  editor.welcomeOverDoc = true;
  editor.showWelcome = true;
}

/** Avant un choix de l'accueil qui crée quelque chose : un document neuf, si l'accueil était ouvert par-dessus
 *  un document (enregistrement proposé). Faux : l'enseignant a annulé. */
export async function freshFromHome(): Promise<boolean> {
  if (!editor.welcomeOverDoc) return true;
  if (!(await confirmLeave())) return false;
  editor.reset(newDocument(), null);
  editor.carriedPending = [];
  editor.templateMode = false;
  editor.welcomeOverDoc = false;
  return true;
}

/** Ouvre une copie sans nom d'un exemple (rien n'écrase l'exemple ; on enregistre sous un nouveau nom). */
export async function openExampleDoc(id: string) {
  if (!(await confirmLeave())) return;
  const doc = await guard(() => api.openExample(id));
  if (!doc) return;
  editor.reset(doc, null);
  editor.carriedPending = [];
  editor.templateMode = false;
  if (editor.drawer === "grid") editor.drawer = null;
  editor.showWelcome = false;
}

export async function openDoc() {
  if (!(await confirmLeave())) return;
  const picked = await open({ filters: [{ name: "Carnet (Cahutin)", extensions: ["carnet"] }] });
  if (typeof picked === "string") await openPath(picked, false);
}

/** Ouvre un document (choisi, ou dans les documents récents). Un récent introuvable est retiré de la liste. */
export async function openPath(path: string, confirm = true) {
  if (confirm && !(await confirmLeave())) return;
  if ((await api.existingPaths([path]).catch(() => [path])).length === 0) {
    editor.recents = editor.recents.filter((r) => r.path !== path);
    writeRecents(editor.recents);
    editor.notice = fr.recentMissing;
    return;
  }
  const opened = await guard(() => api.openDocument(path));
  if (!opened) return;
  editor.reset(opened.doc, path);
  editor.carriedPending = opened.doc.pending_photos;
  remember(path, opened.doc.title);
  void checkDocumentFonts(opened.doc, opened.offered_fonts);
  editor.replaceLibrary(Object.values(opened.doc.assets).flatMap((meta) => (meta ? [{ meta, path: "", folder: "" }] : [])));
  editor.showWelcome = false;
  editor.templateMode = false;
  if (editor.drawer === "grid") editor.drawer = null;
  // Photos en attente : celles qui existent encore sont réimportées, sans message pour les autres.
  const pending = await api.existingPaths(opened.doc.pending_photos).catch(() => []);
  if (pending.length) void importPaths(pending, { quiet: true });
}

const IMPORTABLE = ["carnet", "docx", "doc", "odt", "odg", "pdf", "pub"];
let importCancelled = false;

/**
 * Fichier > Importer : chaque fichier est converti tour à tour, puis toutes les pages sont ajoutées
 * après la page courante en un seul pas d'annulation. `fresh` (accueil) : un document neuf avec ces pages.
 */
export async function importFiles(paths?: string[], opts: { fresh?: boolean } = {}) {
  if (!paths) {
    const picked = await open({ multiple: true, filters: [{ name: fr.importFilter, extensions: IMPORTABLE }] });
    if (!picked) return;
    paths = Array.isArray(picked) ? picked : [picked];
  }
  if (!paths.length || editor.fileImport) return;
  if (opts.fresh && !(await confirmLeave())) return;
  editor.flushPending();
  importCancelled = false;
  const startedFrom = opts.fresh ? editor.doc : null;
  const parts: ImportedPages[] = [];
  const notes: string[] = [];
  const offered = new Set<string>();
  try {
    for (const path of paths) {
      const name = fileName(path);
      editor.fileImport = { name };
      try {
        const part = await api.importFile(path);
        parts.push(part);
        notes.push(importReportText(name, part.report));
        part.offered_fonts.forEach((f) => offered.add(f));
      } catch (e) {
        notes.push(`${name} : ${String(e)}`);
      }
      if (importCancelled) break;
    }
  } finally {
    editor.fileImport = null;
  }
  if (parts.length) {
    const placed = placeImported(editor.doc, editor.pageIndex, parts, startedFrom);
    if (placed.replaced) {
      editor.reset(placed.doc, null);
      editor.carriedPending = [];
      editor.showWelcome = false;
      editor.templateMode = false;
    } else {
      editor.commit(placed.doc);
      editor.pageIndex = placed.pageIndex;
      editor.selection = [];
    }
    editor.addPhotos(parts.flatMap((p) => p.assets.map((meta) => ({ meta, path: "", folder: "" }))));
    if (offered.size) await checkDocumentFonts(editor.doc, [...offered]);
  }
  editor.notice = [...notes, editor.notice].filter(Boolean).join("\n");
}

/** Arrête la conversion en cours ; les fichiers suivants ne sont pas importés. */
export async function cancelFileImport() {
  importCancelled = true;
  await guard(() => api.cancelImport());
}

function remember(path: string, title: string) {
  editor.recents = addRecent(editor.recents, { path, title: title.trim() || fileName(path), openedAt: Date.now() });
  writeRecents(editor.recents);
}

export async function exportOdg() {
  editor.flushPending();
  const picked = await saveDialog({ defaultPath: `${baseName()}.odg`, filters: [{ name: "LibreOffice Draw", extensions: ["odg"] }] });
  if (!picked) return;
  const path = withExt(picked, "odg");
  if ((await guard(() => api.exportOdg(path, editor.doc))) !== undefined) editor.notice = `Exporté pour LibreOffice : ${fileName(path)}`;
}

/** Rend les pages d'impression, attend qu'elles soient affichées (polices, images), puis lance `action`. */
async function withPrintPages<T>(action: () => T | Promise<T>): Promise<T> {
  editor.flushPending();
  editor.selection = [];
  editor.cropping = null;
  editor.editingText = null;
  editor.printing = true;
  await tick();
  await document.fonts.ready;
  await new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)));
  try {
    return await action();
  } finally {
    editor.printing = false;
  }
}

/** Les affiches A3 en deux feuilles A4 qui se recouvrent. */
export async function printTiled() {
  editor.printTiles = true;
  try {
    await print();
  } finally {
    editor.printTiles = false;
  }
}

/** Cartes en recto-verso : chaque page paire est le verso de la précédente. */
export async function printDuplex() {
  editor.printDuplex = true;
  try {
    await print();
  } finally {
    editor.printDuplex = false;
  }
}

export async function print() {
  // Selon le moteur, window.print() bloque ou rend la main tout de suite : on garde les pages
  // jusqu'à la fin de l'impression.
  await withPrintPages(
    () =>
      new Promise<void>((done) => {
        window.addEventListener("afterprint", () => done(), { once: true });
        window.print();
      }),
  );
}

export async function exportPdf() {
  const picked = await saveDialog({ defaultPath: `${baseName()}.pdf`, filters: [{ name: "PDF", extensions: ["pdf"] }] });
  if (!picked) return;
  const path = withExt(picked, "pdf");
  const first = editor.doc.pages[0].size;
  const done = await withPrintPages(() => guard(() => api.exportPdf(path, first.width_mm, first.height_mm)));
  if (done !== undefined) editor.notice = `PDF enregistré : ${fileName(path)}`;
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

/**
 * Importe en arrière-plan : les vignettes apparaissent au fil de l'eau, la progression avance,
 * les échecs sont résumés à la fin (sauf `quiet`, pour les photos en attente d'un document).
 */
export async function importPaths(paths: string[], opts: { fromFolder?: boolean; quiet?: boolean } = {}) {
  if (paths.length === 0) {
    if (opts.fromFolder) editor.notice = summarizeImport([], { fromFolder: true }).message;
    return;
  }
  const progress = editor.importProgress ?? { done: 0, total: 0 };
  editor.importProgress = { done: progress.done, total: progress.total + paths.length };
  const outcomes: ImportOutcome[] = [];
  try {
    await api.importPhotos(paths, (o) => {
      outcomes.push(o);
      if (o.status === "Ok") editor.addPhotos([{ meta: o.asset, path: o.path, folder: o.folder }]);
      if (editor.importProgress) editor.importProgress = { ...editor.importProgress, done: editor.importProgress.done + 1 };
    });
  } catch (e) {
    if (!opts.quiet) editor.notice = String(e);
  } finally {
    // Import interrompu : les photos jamais traitées comptent comme terminées, pour ne pas bloquer la barre.
    const p = editor.importProgress;
    if (p) {
      const done = p.done + (paths.length - outcomes.length);
      editor.importProgress = done >= p.total ? null : { ...p, done };
    }
  }
  const { message } = summarizeImport(outcomes);
  if (message && !opts.quiet) editor.notice = message;
}

/**
 * « Ce document utilise la police X : l'ajouter ? » pour les polices qu'il transporte ; puis, pour celles
 * qui manquent encore (refusées, ou non intégrables), un avertissement : le texte s'affiche en Andika.
 */
async function checkDocumentFonts(doc: Document, offered: string[]) {
  if (offered.length) {
    const choice = await modal.choose(fr.offerFonts(offered), [
      { value: "later", label: fr.later },
      { value: "add", label: fr.addFontsNow, primary: true },
    ]);
    if (choice === "add" && (await guard(() => api.acceptDocumentFonts())) !== undefined) await fonts.load();
  }
  const missing = missingFamilies(doc);
  if (missing.length) editor.notice = fr.missingFonts(missing);
}

export const LOGO_WIDTH_MM = 40;
const LOGO_MARGIN_MM = 10;

/** Logo d'un modèle : une image fixée (verrouillée), 40 mm de large, en haut à droite de la page. */
export async function pickLogo() {
  const picked = await open({ filters: [{ name: "Images", extensions: ["jpg", "jpeg", "png", "webp", "JPG", "JPEG", "PNG", "WEBP"] }] });
  if (typeof picked !== "string") return;
  let outcome: ImportOutcome | null = null;
  if ((await guard(() => api.importPhotos([picked], (o) => (outcome = o)))) === undefined) return;
  const o = outcome as ImportOutcome | null;
  if (!o) return;
  if (o.status === "Err") {
    editor.notice = o.message;
    return;
  }
  const meta = o.asset;
  const w = LOGO_WIDTH_MM;
  const h = Math.round(((w * meta.height_px) / meta.width_px) * 10) / 10;
  const page = editor.page;
  const id = newId();
  editor.commit({
    ...editor.doc,
    assets: { ...editor.doc.assets, [meta.id]: meta },
    pages: editor.doc.pages.map((p) =>
      p.id !== page.id
        ? p
        : {
            ...p,
            elements: [
              ...p.elements,
              {
                id,
                frame: { x_mm: p.size.width_mm - LOGO_MARGIN_MM - w, y_mm: LOGO_MARGIN_MM, w_mm: w, h_mm: h, rotation_deg: 0 },
                locked: true,
                zone: false,
                kind: { type: "Photo", asset: meta.id, crop: { scale: 1, center_x: 0.5, center_y: 0.5 }, corner_radius_mm: 0, border: null, shadow: false },
              },
            ],
          },
    ),
  });
  editor.selection = [id];
}
