// Actions sur les fichiers : nouveau, ouvrir, enregistrer, exporter, imprimer, rapport de problème.
import { open, save as saveDialog } from "@tauri-apps/plugin-dialog";
import { tick } from "svelte";
import * as api from "./api";
import { assistant } from "./assistant.svelte";
import { ask } from "./ask";
import { classHoldAfterSave, localTime } from "./autosave";
import { autosave, autosaver } from "./autosave.svelte";
import { clearBackup, copyState } from "./backupCopy";
import { imagesDecoded } from "./printImages";
import { confirmDiscardIfDirty, editor, type DiscardChoice } from "./docStore.svelte";
import { newDocument } from "./factory";
import { fonts } from "./fonts.svelte";
import { fr } from "./i18n/fr";
import { importReportText, placeImported } from "./importPages";
import { summarizeImport } from "./importSummary";
import { pendingPaths, type LibraryPhoto } from "./photoLibrary";
import { addRecent, withoutRecent, writeRecents } from "./recents";
import { missingFamilies } from "./fontNames";
import { settings } from "./settings.svelte";
import type { Document, ImportedPages, ImportOutcome, OpenedDocument, SaveMode, SaveOutcome } from "./types";
import { modal } from "./modal.svelte";
import { IS_LINUX } from "./cursors";
import { expandSelection } from "./groups";
import { atRisk, mixedSheets, pagesAtRisk, printWarning } from "./printSafety";
import { formatOf, printSheet } from "./pageFormat";

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

let saving: Promise<unknown> | null = null;
/** Un enregistrement est en cours : la copie de secours et l'enregistrement automatique attendent. */
export const isSaving = () => saving !== null;

/** Une écriture à la fois : chacune attend la fin de la précédente (Ctrl+S pendant un enregistrement automatique). */
async function oneAtATime<T>(write: () => Promise<T>): Promise<T> {
  while (saving) await saving.catch(() => {});
  const p = write();
  saving = p;
  try {
    return await p;
  } finally {
    if (saving === p) saving = null;
  }
}

/**
 * Écrit `doc` (le document validé au moment de l'appel) dans `path` ; ce qui change pendant l'écriture reste
 * « modifié ». « ChangedElsewhere » : rien n'est écrit, le fichier a changé par ailleurs. Lève en cas d'échec.
 * `generation` : le document ouvert au départ ; s'il a été remplacé entre-temps, rien n'est marqué.
 */
export function writeDoc(path: string, doc: Document, mode: SaveMode, generation = editor.generation): Promise<SaveOutcome> {
  return oneAtATime(async () => {
    // Les photos importées mais non placées sont retrouvées à la réouverture (chemins seulement).
    const written = { ...doc, pending_photos: pendingPaths(editor.photos, doc, editor.carriedPending) };
    const outcome = await api.saveDocument(path, written, mode, settings.autosave);
    if (outcome === "ChangedElsewhere" || generation !== editor.generation) return outcome ?? "Saved";
    editor.markSaved(path, doc);
    // L'enregistrement automatique ne range pas le document en tête des récents à chaque fois : seulement si
    // son titre a changé.
    if (mode !== "Auto" || editor.recents.find((r) => r.path === path)?.title !== (doc.title.trim() || fileName(path))) remember(path, doc.title);
    // Enregistré : la copie de secours n'a plus lieu d'être (l'enregistrement automatique ne l'efface que s'il y
    // en a une, pour ne pas remplir le journal).
    if (mode !== "Auto" || copyState.lastBacked) void clearBackup();
    void autosave.refreshStart();
    return "Saved";
  });
}

/** Écrit et dit ce qui ne va pas ; un fichier changé par ailleurs : l'enseignant choisit. */
async function write(path: string, mode: SaveMode): Promise<boolean> {
  const hold = editor.classHold;
  const generation = editor.generation;
  let outcome: SaveOutcome;
  try {
    outcome = await writeDoc(path, editor.doc, mode);
  } catch (e) {
    editor.notice = String(e);
    return false;
  }
  if (outcome === "ChangedElsewhere") {
    const choice = await modal.choose(fr.changedElsewhereAsk, [
      { value: "cancel", label: fr.backToDocument },
      { value: "replace", label: fr.changedElsewhereReplace },
      { value: "copy", label: fr.saveCopy, primary: true },
    ]);
    if (choice === "copy") return saveAs();
    return choice === "replace" ? write(path, "Replace") : false;
  }
  autosaver.cleared();
  // Enregistré par l'enseignant : ce que la classe a mis dans le document est voulu, l'écriture automatique reprend.
  if (generation === editor.generation) editor.classHold = classHoldAfterSave(editor.classHold, hold);
  return true;
}

export async function saveAs(): Promise<boolean> {
  if (!(await assistant.settle())) return false;
  editor.flushPending();
  const picked = await saveDialog({ defaultPath: `${baseName()}.carnet`, filters: [{ name: "Carnet (Cahutin)", extensions: ["carnet"] }] });
  if (!picked) return false;
  // La fenêtre d'enregistrement a déjà demandé s'il fallait remplacer un fichier existant.
  return write(withExt(picked, "carnet"), "Replace");
}

export async function save(): Promise<boolean> {
  if (!(await assistant.settle())) return false;
  editor.flushPending();
  if (!editor.path) return saveAs();
  // Enregistrement automatique décoché : comme avant, le fichier est écrit sans autre question.
  return write(editor.path, settings.autosave ? "Manual" : "Replace");
}

function askToSave(): Promise<DiscardChoice> {
  return modal.choose<DiscardChoice>(fr.saveChangesTo(editor.doc.title.trim() || fr.untitled), [
    { value: "cancel", label: fr.backToDocument },
    { value: "discard", label: fr.dontSave },
    { value: "save", label: fr.save, primary: true },
  ]);
}

/** Quitter le document en connaissance de cause : une proposition de l'assistant est réglée d'abord, et la copie
 *  de secours est effacée. */
export async function confirmLeave(): Promise<boolean> {
  const leave = (await assistant.settle()) && (await savedOrAsked());
  if (leave) await clearBackup();
  return leave;
}

/** Le texte en cours d'écriture est validé ; un document qui s'enregistre tout seul l'est tout de suite, sans
 *  question ; sinon (ou si l'écriture échoue) : « Enregistrer les modifications ? », comme avant. */
async function savedOrAsked(): Promise<boolean> {
  editor.flushPending();
  if (editor.dirty && (await autosaver.flush())) return true;
  return confirmDiscardIfDirty(askToSave, save);
}

/** Fichier > « Revenir à la version du début de séance » : la version gardée avant la première écriture de la
 *  séance prend la place du document, en un pas d'annulation (Ctrl+Z revient aux changements), puis s'enregistre
 *  comme tout changement. */
export async function revertToStart() {
  const path = editor.ownFile ? editor.path : null;
  const at = autosave.startVersion;
  if (!path || !at || !(await assistant.settle())) return;
  editor.flushPending();
  const choice = await modal.choose(fr.revertToStartAsk(localTime(at)), [
    { value: "cancel", label: fr.backToDocument },
    { value: "revert", label: fr.revertToStartDo, primary: true },
  ]);
  if (choice !== "revert") return;
  const opened = await guard(() => api.startVersionOpen(path));
  if (!opened || editor.path !== path) return;
  editor.addPhotos(docPhotos(opened.doc));
  editor.commit(opened.doc);
  editor.selection = [];
  // « Annuler » ne vaut que pour ce document : resté affiché après en avoir ouvert un autre, il n'y défait rien.
  const generation = editor.generation;
  editor.notify(fr.revertedToStart(localTime(at)), { label: fr.undo, run: () => editor.generation === generation && editor.undo() });
}

/** Les photos d'un document, pour le panneau Photos : un document qui s'ouvre ou se crée y montre les siennes,
 *  jamais celles du document quitté. */
function docPhotos(doc: Document): LibraryPhoto[] {
  return Object.values(doc.assets).flatMap((meta) => (meta ? [{ meta, path: "", folder: "" }] : []));
}

export async function newDoc() {
  if (!(await confirmLeave())) return;
  editor.reset(newDocument(), null);
  assistant.clear();
  editor.carriedPending = [];
  editor.replaceLibrary([]);
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
  assistant.clear();
  editor.carriedPending = [];
  editor.replaceLibrary([]);
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
  assistant.clear();
  editor.carriedPending = [];
  editor.replaceLibrary(docPhotos(doc));
  editor.templateMode = false;
  if (editor.drawer === "grid") editor.drawer = null;
  editor.showWelcome = false;
}

export async function openDoc() {
  if (!(await confirmLeave())) return;
  const picked = await open({ filters: [{ name: "Carnet (Cahutin)", extensions: ["carnet"] }] });
  if (typeof picked === "string") await openPath(picked, false);
}

/** Retire un document de la liste des récents (le fichier reste). */
export function forgetRecent(path: string) {
  editor.recents = withoutRecent(editor.recents, path);
  writeRecents(editor.recents);
}

/** Vide la liste des récents, après confirmation (les fichiers restent). */
export async function clearRecents() {
  if (!(await ask(fr.recentsClearConfirm, fr.recentsClear))) return;
  editor.recents = [];
  writeRecents([]);
}

/** Met un document récent à la corbeille de l'ordinateur, après confirmation, et le retire de la liste. */
export async function trashRecent(path: string, title: string) {
  if (!(await ask(fr.recentTrashConfirm(title), fr.recentTrash))) return;
  try {
    await api.documentTrash(path);
    forgetRecent(path);
    editor.notice = fr.recentTrashed(title);
  } catch (e) {
    editor.notice = String(e);
  }
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
  editor.reset(opened.doc, path, { own: true });
  assistant.clear();
  remember(path, opened.doc.title);
  await showOpened(opened);
}

/**
 * Le document ouvert prend la place du précédent (déjà remis à zéro dans `editor`) : ses photos dans le
 * panneau, ses polices proposées, ses photos en attente réimportées. Sert aussi à la reprise d'une copie de secours.
 */
export async function showOpened(opened: OpenedDocument) {
  editor.carriedPending = opened.doc.pending_photos;
  void checkDocumentFonts(opened.doc, opened.offered_fonts);
  editor.replaceLibrary(docPhotos(opened.doc));
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
      assistant.clear();
      editor.carriedPending = [];
      editor.replaceLibrary(docPhotos(placed.doc));
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

/** `target` : chemin déjà choisi, sans fenêtre (essais en développement). */
export async function exportOdg(target?: string) {
  if (!(await assistant.settle())) return;
  editor.flushPending();
  const picked = target ?? (await saveDialog({ defaultPath: `${baseName()}.odg`, filters: [{ name: "LibreOffice Draw", extensions: ["odg"] }] }));
  if (!picked) return;
  const path = withExt(picked, "odg");
  editor.busy = fr.exportingOdg;
  try {
    if ((await guard(() => api.exportOdg(path, editor.doc))) !== undefined) editor.notice = `Exporté pour LibreOffice : ${fileName(path)}`;
  } finally {
    editor.busy = null;
  }
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
  await imagesDecoded(document.querySelectorAll<HTMLImageElement>(".print-root img"));
  await new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)));
  try {
    return await action();
  } finally {
    editor.printing = false;
  }
}

/** Les affiches A3 en deux feuilles A4 qui se recouvrent. */
export async function printTiled() {
  if (!(await assistant.settle())) return;
  editor.printTiles = true;
  try {
    await print();
  } finally {
    editor.printTiles = false;
  }
}

/** Cartes en recto-verso : chaque page paire est le verso de la précédente. */
export async function printDuplex() {
  if (!(await assistant.settle())) return;
  editor.printDuplex = true;
  try {
    await print();
  } finally {
    editor.printDuplex = false;
  }
}

/** Prévient, sans retenir l'impression, de ce que l'imprimante risque de couper (lib/printSafety.ts), après
 *  `before` s'il y a déjà un avis. L'avis reste affiché pendant la fenêtre d'impression et après. */
function warnPrint(pdf: boolean, before?: string) {
  const w = printWarning(editor.doc, { linux: IS_LINUX, pdf, tiles: editor.printTiles });
  if (!w) {
    if (before) editor.notice = before;
    return;
  }
  editor.notify(before ? `${before}\n\n${w.message}` : w.message, w.ids.length ? { label: fr.edgeRiskShow, run: showAtRisk } : undefined);
}

/** Bouton de l'avis : la première page en cause, ses éléments à risque choisis (avec leur groupe). */
function showAtRisk() {
  const i = pagesAtRisk(editor.doc)[0];
  if (i === undefined) return;
  editor.goToPage(i + 1);
  editor.selection = expandSelection(editor.page, atRisk(editor.page).map((e) => e.id));
}

export async function print() {
  if (!(await assistant.settle())) return;
  warnPrint(false);
  const sheet = printSheet(editor.doc.pages[0].size, editor.printTiles);
  // Selon le moteur, window.print() bloque ou rend la main tout de suite : on garde les pages
  // jusqu'à la fin de l'impression.
  await withPrintPages(async () => {
    // Sous Linux, la fenêtre d'impression s'ouvre sur le papier et l'orientation de la première feuille.
    await api.preparePrint(sheet.width_mm, sheet.height_mm).catch(() => undefined);
    await new Promise<void>((done) => {
      window.addEventListener("afterprint", () => done(), { once: true });
      window.print();
    });
  });
}

/** `target` : chemin déjà choisi, sans fenêtre (essais en développement). */
export async function exportPdf(target?: string) {
  if (!(await assistant.settle())) return;
  const picked = target ?? (await saveDialog({ defaultPath: `${baseName()}.pdf`, filters: [{ name: "PDF", extensions: ["pdf"] }] }));
  if (!picked) return;
  const path = withExt(picked, "pdf");
  const first = editor.doc.pages[0].size;
  // Chaque page à son format, sauf avec une page de format libre : seule la voie de secours la met à sa taille.
  const pageSizes = editor.doc.pages.every((p) => formatOf(p.size) !== null);
  const route = await withPrintPages(() => guard(() => api.exportPdf(path, first.width_mm, first.height_mm, pageSizes)));
  if (route === undefined) return;
  const saved = `PDF enregistré : ${fileName(path)}`;
  warnPrint(true, route === "first_page_size" && mixedSheets(editor.doc, false) ? `${saved}\n\n${fr.pdfFirstPageSize}` : saved);
}

export async function problemReport() {
  const picked = await saveDialog({ defaultPath: "rapport-carnet.txt", filters: [{ name: "Texte", extensions: ["txt"] }] });
  if (!picked) return;
  const path = withExt(picked, "txt");
  if ((await guard(() => api.exportProblemReport(path))) === undefined) return;
  // La messagerie s'ouvre sur un mail déjà rempli ; le rapport est à joindre (un lien de mail ne le peut pas).
  const mail = await guard(() => api.problemReportMail(path));
  if (!mail) return;
  const folder = fileName(path.slice(0, Math.max(path.lastIndexOf("/"), path.lastIndexOf("\\"))));
  editor.notice = mail.opened ? fr.reportMailOpened(fileName(path), folder, mail.address) : fr.reportMailFailed(fileName(path), mail.address);
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


