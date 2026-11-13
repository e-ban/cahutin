// Une fonction typée par commande Tauri (src-tauri/src/commands.rs).
import { Channel, invoke } from "@tauri-apps/api/core";
import type { AssistantEvent, AssistantRequest, AssistantSettings, AssetMeta, ClassPhoto, CellContent, FaceBox, Class, GeneratePlan, ClassFileInfo, Document, FillMode, OpenedClass, Student, ExampleInfo, Frame, FontFamilyInfo, GridPreset, FontFileInfo, ImportOutcome, ImportedPages, InstantiatedPage, OpenedDocument, Page, TemplateInfo, ZoneKind, Match, OriginalsApplied, OriginalsReport, Phototheque, OriginalsProgress, ScanEvent, ScanReport, TagChange, MyImage, MyImagesAdded, MyImageKept, ReportMail, BackupOffer, RestoredDocument } from "./types";

/** Importe en arrière-plan ; `onOutcome` est appelé pour chaque photo dès qu'elle est prête. */
export function importPhotos(paths: string[], onOutcome: (o: ImportOutcome) => void): Promise<void> {
  const onProgress = new Channel<ImportOutcome>();
  onProgress.onmessage = onOutcome;
  return invoke<void>("import_photos", { paths, onProgress });
}
/** L'index tel qu'il est enregistré, tout de suite, sans parcourir le disque. */
export const photothequeOpen = (classPath: string) => invoke<Phototheque>("phototheque_open", { classPath });
/** Parcourt les dossiers suivis ; `onEvent` reçoit les photos au fil de l'eau, puis l'index complet. */
export function photothequeScan(onEvent: (e: ScanEvent) => void): Promise<ScanReport> {
  const onProgress = new Channel<ScanEvent>();
  onProgress.onmessage = onEvent;
  return invoke<ScanReport>("phototheque_scan", { onProgress });
}
/** Suit un dossier, sans le parcourir : `photothequeScan` ensuite. */
export const photothequeFollow = (folder: string) => invoke<Phototheque>("phototheque_follow", { folder });
export const photothequeUnfollow = (folder: string) => invoke<Phototheque>("phototheque_unfollow", { folder });
/** « Annuler » après « Ne plus suivre » : le dossier revient avec ses photos et leurs étiquettes. */
export const photothequeRefollow = (folder: string) => invoke<Phototheque>("phototheque_refollow", { folder });
/** « Retirer » ou remettre : rend seulement les photos changées. */
export const photothequeHide = (ids: string[], hidden: boolean) => invoke<ClassPhoto[]>("phototheque_hide", { ids, hidden });
/** Cherche les originaux des anciennes copies, en disant où elle en est ; `null` si elle a été arrêtée. */
export function photothequeFindOriginals(onEvent: (p: OriginalsProgress) => void): Promise<OriginalsReport | null> {
  const onProgress = new Channel<OriginalsProgress>();
  onProgress.onmessage = onEvent;
  return invoke<OriginalsReport | null>("phototheque_find_originals", { onProgress });
}
/** « Arrêter » la recherche des originaux en cours. */
export const photothequeStopOriginals = () => invoke<void>("phototheque_stop_originals");
/** « Non merci » : la question de la migration ne revient plus. */
export const photothequeSettleMigration = () => invoke<Phototheque>("phototheque_settle_migration");
export const photothequeApplyOriginals = (matches: Match[]) => invoke<OriginalsApplied>("phototheque_apply_originals", { matches });
/** Élèves et activités : rend seulement les photos changées. */
export const photothequeTag = (ids: string[], change: TagChange) => invoke<ClassPhoto[]>("phototheque_tag", { ids, change });
export const photothequeUse = (ids: string[]) => invoke<AssetMeta[]>("phototheque_use", { ids });
export const myImagesList = () => invoke<MyImage[]>("my_images_list");
export const myImagesAdd = (paths: string[]) => invoke<MyImagesAdded>("my_images_add", { paths });
export const myImagesRemove = (name: string) => invoke<MyImage[]>("my_images_remove", { name });
export const myImagesUse = (name: string) => invoke<AssetMeta>("my_images_use", { name });
export const myImagesKeep = (asset: string, name: string) => invoke<MyImageKept>("my_images_keep", { asset, name });
export const myImagesRemoved = () => invoke<MyImage[]>("my_images_removed");
export const myImagesRestore = (name: string) => invoke<MyImage[]>("my_images_restore", { name });
export const myImagesRename = (name: string, newName: string) => invoke<MyImage[]>("my_images_rename", { name, newName });
export const myImagesTheme = (names: string[], add: string | null, remove: string | null) => invoke<MyImage[]>("my_images_theme", { names, add, remove });
/** Une image retirée de la collection, pour l'aperçu. */
export const removedImageUrl = (name: string) => `${assetBase()}/mine/${encodeURIComponent("Retirées/" + name)}`;
export const existingPaths = (paths: string[]) => invoke<string[]>("existing_paths", { paths });
/** Met un document .carnet à la corbeille de l'ordinateur. */
export const documentTrash = (path: string) => invoke<void>("document_trash", { path });
/** Le document reçu au lancement (fichier .carnet ouvert depuis l'explorateur), s'il y en a un. */
export const launchDocument = () => invoke<string | null>("launch_document");
export const listImportableIn = (dir: string) => invoke<string[]>("list_importable_in", { dir });
export const saveDocument = (path: string, doc: Document) => invoke<void>("save_document", { path, doc });
export const openDocument = (path: string) => invoke<OpenedDocument>("open_document", { path });
/** Copie de secours du document ouvert (dossier de l'application), reprise au lancement après une panne. */
export const backupSave = (doc: Document, source: string | null) => invoke<void>("backup_save", { doc, source });
/** Copie laissée par une fenêtre arrêtée sans enregistrer, s'il y en a une. */
export const backupInfo = () => invoke<BackupOffer | null>("backup_info");
export const backupOpen = (id: string) => invoke<RestoredDocument>("backup_open", { id });
export const backupDiscard = (id: string) => invoke<void>("backup_discard", { id });
/** Efface la copie de cette fenêtre. */
export const backupClear = () => invoke<void>("backup_clear");
export const previewDocument = (path: string) => invoke<Document>("preview_document", { path });
export const importFile = (path: string) => invoke<ImportedPages>("import_file", { path });
export const cancelImport = () => invoke<void>("cancel_import");
export const exportOdg = (path: string, doc: Document) => invoke<void>("export_odg", { path, doc });
export const gridSet = (page: Page, cols: number, rows: number) => invoke<Page>("grid_set", { page, cols, rows });
export const gridMerge = (page: Page, a: string, b: string) => invoke<Page>("grid_merge", { page, a, b });
export const gridSplit = (page: Page, cell: string) => invoke<Page>("grid_split", { page, cell });
export const gridSetKind = (page: Page, cell: string, kind: ZoneKind) => invoke<Page>("grid_set_kind", { page, cell, kind });
export const pageSetSize = (page: Page, widthMm: number, heightMm: number) => invoke<Page>("page_set_size", { page, widthMm, heightMm });
export const gridLiberate = (page: Page) => invoke<Page>("grid_liberate", { page });
export const gridPreset = (page: Page, preset: GridPreset) => invoke<Page>("grid_preset", { page, preset });
export const photoDetectFaces = (assetId: string) => invoke<FaceBox[]>("photo_detect_faces", { assetId });
export const photoBlur = (assetId: string, zones: FaceBox[]) => invoke<AssetMeta>("photo_blur", { assetId, zones });
export const tableAdd = (page: Page, frame: Frame, cols: number, rows: number) => invoke<[Page, string]>("table_add", { page, frame, cols, rows });
export const tableInsertRow = (page: Page, cell: string, below: boolean) => invoke<Page>("table_insert_row", { page, cell, below });
export const tableInsertCol = (page: Page, cell: string, right: boolean) => invoke<Page>("table_insert_col", { page, cell, right });
export const tableRemoveRow = (page: Page, cell: string) => invoke<Page>("table_remove_row", { page, cell });
export const tableRemoveCol = (page: Page, cell: string) => invoke<Page>("table_remove_col", { page, cell });
export const tableMerge = (page: Page, a: string, b: string) => invoke<Page>("table_merge", { page, a, b });
export const tableSplit = (page: Page, cell: string) => invoke<Page>("table_split", { page, cell });
export const tableSetCell = (page: Page, cell: string, content: CellContent) => invoke<Page>("table_set_cell", { page, cell, content });
export const tableDissolve = (page: Page, table: string) => invoke<Page>("table_dissolve", { page, table });
export const classDir = () => invoke<string>("class_dir");
export const classList = () => invoke<ClassFileInfo[]>("class_list");
export const classCreate = (name: string, year: string) => invoke<OpenedClass>("class_create", { name, year });
export const classOpen = (path: string) => invoke<OpenedClass>("class_open", { path });
export const classSave = (path: string, cls: Class) => invoke<void>("class_save", { path, class: cls });
export const classTrash = (path: string) => invoke<void>("class_trash", { path });
export const classImportPhoto = (path: string) => invoke<AssetMeta>("class_import_photo", { path });
export const classPlan = (page: Page) => invoke<GeneratePlan>("class_plan", { page });
export const classParseList = (text: string) => invoke<Student[]>("class_parse_list", { text });
export const classReadList = (path: string) => invoke<Student[]>("class_read_list", { path });
export const classGenerate = (page: Page, students: Student[], mode: FillMode, classPhotos: string[], cls: Class | null = null) =>
  invoke<ImportedPages>("class_generate", { page, students, mode, classPhotos, class: cls });
/** Écrit les champs de la date (`date` : « 2026-11-09 ») et de la classe d'une page ; les champs d'élève restent. */
export const fillAutoFields = (page: Page, date: string | null, cls: Class | null) => invoke<Page>("fill_auto_fields", { page, date, class: cls });
/** Chaque champ de la date puis de la classe, avec ce qu'il donne aujourd'hui (vide sans classe). */
export const autoFieldExamples = (cls: Class | null) => invoke<[string, string][]>("auto_field_examples", { class: cls });
export const listTemplates = () => invoke<TemplateInfo[]>("list_templates");
export const instantiateTemplate = (id: string) => invoke<InstantiatedPage>("instantiate_template", { id });
export const savePageAsTemplate = (name: string, page: Page, keep?: string[]) => invoke<TemplateInfo>("save_page_as_template", { name, page, keep: keep ?? null });
export const deleteTemplate = (id: string) => invoke<void>("delete_template", { id });
export const listExamples = () => invoke<ExampleInfo[]>("list_examples");
export const openExample = (id: string) => invoke<Document>("open_example", { id });
export const listFonts = () => invoke<FontFamilyInfo[]>("list_fonts");
export const addFont = (path: string) => invoke<FontFamilyInfo>("add_font", { path });
export const removeFont = (family: string) => invoke<void>("remove_font", { family });
export const fontFiles = () => invoke<FontFileInfo[]>("font_files");
export const acceptDocumentFonts = () => invoke<FontFamilyInfo[]>("accept_document_fonts");
export const exportProblemReport = (path: string) => invoke<void>("export_problem_report", { path });
/** Ouvre la messagerie sur le mail de signalement, qui nomme le rapport à joindre. */
export const problemReportMail = (path: string) => invoke<ReportMail>("problem_report_mail", { path });
export const canExportPdf = () => invoke<boolean>("can_export_pdf");
/** Une taille de feuille pour tout le PDF : celle de la première page. */
export const isStoreBuild = () => invoke<boolean>("is_store_build");
export const exportPdf = (path: string, widthMm: number, heightMm: number) => invoke<void>("export_pdf", { path, widthMm, heightMm });
export const checkUpdate = () => invoke<string | null>("check_update");
export const openDownloadPage = () => invoke<void>("open_download_page");

// Protocole `carnet-asset` (forme `http://<schéma>.localhost` sous Windows).
const assetBase = () => (navigator.userAgent.includes("Windows") ? "http://carnet-asset.localhost" : "carnet-asset://localhost");

/** URL d'une image importée. */
/** Photo de la photothèque ouverte (servie depuis son dossier). */
/** Une image de « Mes images », telle qu'elle est dans le dossier. */
export const myImageUrl = (name: string) => `${assetBase()}/mine/${encodeURIComponent(name)}`;

/** `version` (`photoVersion`) : une photo changée sur le disque, ou sa vignette refaite, est rechargée. */
export function classPhotoUrl(id: string, thumb = true, version?: string): string {
  return `${assetBase()}/lib/${id}${thumb ? "/thumb" : ""}${version ? `?v=${version}` : ""}`;
}

export function assetUrl(id: string, thumb = false): string {
  return `${assetBase()}/${id}${thumb ? "/thumb" : ""}`;
}

/** URL servie par le protocole `carnet-asset`, à partir d'un chemin (`/font/<fichier>`, `/picto/<code>.svg`). */
export const protocolUrl = (path: string) => assetBase() + path;
export const fontUrl = protocolUrl;

// Assistant de mise en page (src-tauri/src/assistant.rs).
export const assistantSettings = () => invoke<AssistantSettings>("assistant_settings");
/** `apiKey` : `undefined` garde la clé enregistrée, `""` l'efface. */
export const assistantConfigure = (enabled: boolean, url: string, model: string, apiKey?: string) =>
  invoke<AssistantSettings>("assistant_configure", { enabled, url, model, apiKey: apiKey ?? null });
export const assistantTest = (url: string, model: string, apiKey?: string) => invoke<void>("assistant_test", { url, model, apiKey: apiKey ?? null });
/** Une demande ; `onEvent` reçoit chaque étape dès qu'elle est faite, puis la fin. */
export function assistantSend(request: AssistantRequest, onEvent: (e: AssistantEvent) => void): Promise<void> {
  const channel = new Channel<AssistantEvent>();
  channel.onmessage = onEvent;
  return invoke<void>("assistant_send", { request, onEvent: channel });
}
export const assistantStop = () => invoke<void>("assistant_stop");
/** Les documents des étapes suivantes sans l'étape before → after ; rejette avec AssistantUndoBlocked. */
export const assistantUndoStep = (before: Document, after: Document, rest: Document[]) => invoke<Document[]>("assistant_undo_step", { before, after, rest });
export const assistantReport = (request: string, reply: string, steps: string[]) => invoke<string>("assistant_report", { request, reply, steps });
