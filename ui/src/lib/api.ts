// Une fonction typée par commande Tauri (src-tauri/src/commands.rs).
import { invoke } from "@tauri-apps/api/core";
import type { Document, FontFamilyInfo, ImportOutcome, InstantiatedPage, Page, TemplateInfo, ZoneKind } from "./types";

export const importPhotos = (paths: string[]) => invoke<ImportOutcome[]>("import_photos", { paths });
export const listImportableIn = (dir: string) => invoke<string[]>("list_importable_in", { dir });
export const saveDocument = (path: string, doc: Document) => invoke<void>("save_document", { path, doc });
export const openDocument = (path: string) => invoke<Document>("open_document", { path });
export const exportOdg = (path: string, doc: Document) => invoke<void>("export_odg", { path, doc });
export const gridSet = (page: Page, cols: number, rows: number) => invoke<Page>("grid_set", { page, cols, rows });
export const gridMerge = (page: Page, a: string, b: string) => invoke<Page>("grid_merge", { page, a, b });
export const gridSplit = (page: Page, cell: string) => invoke<Page>("grid_split", { page, cell });
export const gridSetKind = (page: Page, cell: string, kind: ZoneKind) => invoke<Page>("grid_set_kind", { page, cell, kind });
export const gridLiberate = (page: Page) => invoke<Page>("grid_liberate", { page });
export const listTemplates = () => invoke<TemplateInfo[]>("list_templates");
export const instantiateTemplate = (id: string) => invoke<InstantiatedPage>("instantiate_template", { id });
export const savePageAsTemplate = (name: string, page: Page) => invoke<TemplateInfo>("save_page_as_template", { name, page });
export const deleteTemplate = (id: string) => invoke<void>("delete_template", { id });
export const listFonts = () => invoke<FontFamilyInfo[]>("list_fonts");
export const exportProblemReport = (path: string) => invoke<void>("export_problem_report", { path });
export const canExportPdf = () => invoke<boolean>("can_export_pdf");
export const exportPdf = (path: string) => invoke<void>("export_pdf", { path });

/** URL d'une image importée, servie par le protocole `carnet-asset` (forme `http://<schéma>.localhost` sous Windows). */
export function assetUrl(id: string, thumb = false): string {
  const base = navigator.userAgent.includes("Windows") ? "http://carnet-asset.localhost/" : "carnet-asset://localhost/";
  return base + id + (thumb ? "/thumb" : "");
}
