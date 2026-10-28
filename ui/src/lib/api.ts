// Une fonction typée par commande Tauri (src-tauri/src/commands.rs).
import { Channel, invoke } from "@tauri-apps/api/core";
import type { Document, FontFamilyInfo, FontFileInfo, ImportOutcome, InstantiatedPage, OpenedDocument, Page, TemplateInfo, ZoneKind } from "./types";

/** Importe en arrière-plan ; `onOutcome` est appelé pour chaque photo dès qu'elle est prête. */
export function importPhotos(paths: string[], onOutcome: (o: ImportOutcome) => void): Promise<void> {
  const onProgress = new Channel<ImportOutcome>();
  onProgress.onmessage = onOutcome;
  return invoke<void>("import_photos", { paths, onProgress });
}
export const existingPaths = (paths: string[]) => invoke<string[]>("existing_paths", { paths });
export const listImportableIn = (dir: string) => invoke<string[]>("list_importable_in", { dir });
export const saveDocument = (path: string, doc: Document) => invoke<void>("save_document", { path, doc });
export const openDocument = (path: string) => invoke<OpenedDocument>("open_document", { path });
export const exportOdg = (path: string, doc: Document) => invoke<void>("export_odg", { path, doc });
export const gridSet = (page: Page, cols: number, rows: number) => invoke<Page>("grid_set", { page, cols, rows });
export const gridMerge = (page: Page, a: string, b: string) => invoke<Page>("grid_merge", { page, a, b });
export const gridSplit = (page: Page, cell: string) => invoke<Page>("grid_split", { page, cell });
export const gridSetKind = (page: Page, cell: string, kind: ZoneKind) => invoke<Page>("grid_set_kind", { page, cell, kind });
export const pageSetSize = (page: Page, widthMm: number, heightMm: number) => invoke<Page>("page_set_size", { page, widthMm, heightMm });
export const gridLiberate = (page: Page) => invoke<Page>("grid_liberate", { page });
export const listTemplates = () => invoke<TemplateInfo[]>("list_templates");
export const instantiateTemplate = (id: string) => invoke<InstantiatedPage>("instantiate_template", { id });
export const savePageAsTemplate = (name: string, page: Page) => invoke<TemplateInfo>("save_page_as_template", { name, page });
export const deleteTemplate = (id: string) => invoke<void>("delete_template", { id });
export const listFonts = () => invoke<FontFamilyInfo[]>("list_fonts");
export const addFont = (path: string) => invoke<FontFamilyInfo>("add_font", { path });
export const removeFont = (family: string) => invoke<void>("remove_font", { family });
export const fontFiles = () => invoke<FontFileInfo[]>("font_files");
export const acceptDocumentFonts = () => invoke<FontFamilyInfo[]>("accept_document_fonts");
export const exportProblemReport = (path: string) => invoke<void>("export_problem_report", { path });
export const canExportPdf = () => invoke<boolean>("can_export_pdf");
export const exportPdf = (path: string, landscape: boolean) => invoke<void>("export_pdf", { path, landscape });

/** URL d'une image importée, servie par le protocole `carnet-asset` (forme `http://<schéma>.localhost` sous Windows). */
export function assetUrl(id: string, thumb = false): string {
  const base = navigator.userAgent.includes("Windows") ? "http://carnet-asset.localhost/" : "carnet-asset://localhost/";
  return base + id + (thumb ? "/thumb" : "");
}
