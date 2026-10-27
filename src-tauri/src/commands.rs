//! Commandes appelées par l'interface. Les erreurs sont des messages en français, prêts à afficher.
//! Journal : uniquement des noms d'événements et des compteurs, jamais de chemin ni de contenu.

use crate::state::AppState;
use carnet_core::fonts::{self, FontFamilyInfo};
use carnet_core::photo::{self, Asset};
use carnet_core::templates::{self, TemplateInfo};
use carnet_core::{AssetMeta, Document, Page, ZoneKind, grid, native, odg, report};
use serde::Serialize;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager, State};
use ts_rs::TS;

pub const LOG_FILE_NAME: &str = "carnet";
const IMPORT_THREADS: usize = 4;
const REPORT_MAX_LINES: usize = 500;

#[derive(Serialize, TS)]
#[serde(tag = "status")]
#[ts(export)]
pub enum ImportOutcome {
    Ok { path: String, asset: AssetMeta },
    Err { path: String, message: String },
}

#[derive(Serialize, TS)]
#[ts(export)]
pub struct InstantiatedPage {
    pub page: Page,
    pub assets: Vec<AssetMeta>,
}

/// Importe en parallèle (4 fils au plus) en gardant l'ordre des fichiers.
#[tauri::command]
pub fn import_photos(state: State<'_, AppState>, paths: Vec<String>) -> Vec<ImportOutcome> {
    let chunk = paths.len().div_ceil(IMPORT_THREADS).max(1);
    let results: Vec<(String, Result<Asset, photo::ImportError>)> = std::thread::scope(|s| {
        let handles: Vec<_> = paths
            .chunks(chunk)
            .map(|part| s.spawn(move || part.iter().map(|p| (p.clone(), photo::import_path(Path::new(p)))).collect::<Vec<_>>()))
            .collect();
        handles.into_iter().flat_map(|h| h.join().unwrap()).collect()
    });
    let mut store = state.store.lock().unwrap();
    let mut failures = 0;
    let outcomes: Vec<ImportOutcome> = results
        .into_iter()
        .map(|(path, r)| match r {
            Ok(a) => {
                let meta = a.meta.clone();
                store.insert(a);
                ImportOutcome::Ok { path, asset: meta }
            }
            Err(e) => {
                failures += 1;
                log::warn!("import_photo_failed kind={e:?}");
                ImportOutcome::Err { path, message: e.message_fr() }
            }
        })
        .collect();
    log::info!("import_photos total={} failures={failures}", outcomes.len());
    outcomes
}

#[tauri::command]
pub fn list_importable_in(dir: String) -> Vec<String> {
    photo::list_importable(Path::new(&dir)).into_iter().map(|p| p.to_string_lossy().into_owned()).collect()
}

#[tauri::command]
pub fn save_document(state: State<'_, AppState>, path: String, doc: Document) -> Result<(), String> {
    let store = state.store.lock().unwrap();
    native::save_to_path(&doc, &store, Path::new(&path)).map_err(|e| {
        log::error!("save_document_failed kind={e:?}");
        e.message_fr()
    })?;
    log::info!("save_document pages={}", doc.pages.len());
    Ok(())
}

#[tauri::command]
pub fn open_document(state: State<'_, AppState>, path: String) -> Result<Document, String> {
    let (doc, assets) = native::load_from_path(Path::new(&path)).map_err(|e| {
        log::error!("open_document_failed kind={e:?}");
        e.message_fr()
    })?;
    let mut store = state.store.lock().unwrap();
    for a in assets {
        store.insert(a);
    }
    log::info!("open_document pages={}", doc.pages.len());
    Ok(doc)
}

#[tauri::command]
pub fn export_odg(state: State<'_, AppState>, path: String, doc: Document) -> Result<(), String> {
    let store = state.store.lock().unwrap();
    odg::export_odg_to_path(&doc, &store, Path::new(&path)).map_err(|e| {
        log::error!("export_odg_failed kind={e:?}");
        e.message_fr()
    })?;
    log::info!("export_odg pages={}", doc.pages.len());
    Ok(())
}

#[tauri::command]
pub fn grid_set(mut page: Page, cols: u8, rows: u8) -> Page {
    grid::set_grid(&mut page, cols, rows);
    page
}

#[tauri::command]
pub fn grid_merge(mut page: Page, a: String, b: String) -> Result<Page, String> {
    grid::merge_cells(&mut page, &a, &b).map_err(|e| e.message_fr().to_string())?;
    Ok(page)
}

#[tauri::command]
pub fn grid_split(mut page: Page, cell: String) -> Result<Page, String> {
    grid::split_cell(&mut page, &cell).map_err(|e| e.message_fr().to_string())?;
    Ok(page)
}

#[tauri::command]
pub fn grid_set_kind(mut page: Page, cell: String, kind: ZoneKind) -> Result<Page, String> {
    grid::set_cell_kind(&mut page, &cell, kind).map_err(|e| e.message_fr().to_string())?;
    Ok(page)
}

#[tauri::command]
pub fn grid_liberate(mut page: Page) -> Page {
    grid::liberate(&mut page);
    page
}

#[tauri::command]
pub fn list_templates(state: State<'_, AppState>) -> Vec<TemplateInfo> {
    state.library.list()
}

#[tauri::command]
pub fn instantiate_template(state: State<'_, AppState>, id: String) -> Result<InstantiatedPage, String> {
    let t = state.library.load(&id).ok_or("Ce modèle est introuvable ou abîmé.")?;
    let (page, assets) = templates::instantiate(&t);
    let mut store = state.store.lock().unwrap();
    let metas = assets.iter().map(|a| a.meta.clone()).collect();
    for a in assets {
        store.insert(a);
    }
    Ok(InstantiatedPage { page, assets: metas })
}

#[tauri::command]
pub fn save_page_as_template(state: State<'_, AppState>, name: String, page: Page) -> Result<TemplateInfo, String> {
    let t = {
        let store = state.store.lock().unwrap();
        templates::from_page(&name, &page, &store)
    };
    let info = state.library.save(&t).map_err(|e| e.message_fr())?;
    log::info!("save_template elements={}", page.elements.len());
    Ok(info)
}

#[tauri::command]
pub fn delete_template(state: State<'_, AppState>, id: String) -> Result<(), String> {
    state.library.delete(&id).map_err(|_| "Ce modèle ne peut pas être supprimé.".to_string())
}

#[tauri::command]
pub fn list_fonts() -> Vec<FontFamilyInfo> {
    fonts::families()
}

fn log_path(app: &AppHandle) -> Option<PathBuf> {
    Some(app.path().app_log_dir().ok()?.join(format!("{LOG_FILE_NAME}.log")))
}

#[tauri::command]
pub fn export_problem_report(app: AppHandle, path: String) -> Result<(), String> {
    let log = log_path(&app).and_then(|p| std::fs::read_to_string(p).ok()).unwrap_or_default();
    let now = time::OffsetDateTime::now_local().unwrap_or_else(|_| time::OffsetDateTime::now_utc());
    let date = now
        .format(&time::format_description::parse_borrowed::<2>("[year]-[month]-[day] [hour]:[minute]").unwrap())
        .unwrap_or_default();
    let os = format!("{} {}", std::env::consts::OS, std::env::consts::ARCH);
    let text = report::problem_report(env!("CARGO_PKG_VERSION"), &os, &date, &log, REPORT_MAX_LINES);
    std::fs::write(&path, text).map_err(|_| "Impossible d'écrire le rapport.".to_string())
}

#[tauri::command]
pub fn can_export_pdf() -> bool {
    cfg!(windows)
}

#[cfg(test)]
mod tests {
    use super::*;
    use carnet_core::{A4_PORTRAIT, Page, grid};

    #[test]
    fn grid_merge_error_is_french() {
        let page = grid_set(Page::new(A4_PORTRAIT), 3, 3);
        let at = |p: &Page, col, row| {
            p.grid.as_ref().unwrap().cells.iter().find(|c| c.col == col && c.row == row).unwrap().element_id.clone()
        };
        let (a, b) = (at(&page, 1, 0), at(&page, 2, 0));
        let page = grid_merge(page, a, b).unwrap();
        let (c, d) = (at(&page, 0, 0), at(&page, 1, 1));
        let err = grid_merge(page, c, d).unwrap_err();
        assert_eq!(err, grid::GridError::NotRectangular.message_fr());
    }

    #[test]
    fn grid_commands_return_the_updated_page() {
        let page = grid_set(Page::new(A4_PORTRAIT), 2, 2);
        assert_eq!(page.elements.len(), 4);
        let page = grid_liberate(page);
        assert!(page.grid.is_none());
    }
}
