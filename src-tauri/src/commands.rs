//! Commandes appelées par l'interface. Les erreurs sont des messages en français, prêts à afficher.
//! Journal : uniquement des noms d'événements et des compteurs, jamais de chemin ni de contenu.

use crate::state::AppState;
use carnet_core::fonts::{FontFamilyInfo, FontSet};
use carnet_core::photo;
use carnet_core::templates::{self, TemplateInfo};
use carnet_core::{AssetMeta, Document, Page, PageSize, ZoneKind, grid, native, odg, report};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use tauri::ipc::Channel;
use tauri::{AppHandle, Manager, State};
use ts_rs::TS;

pub const LOG_FILE_NAME: &str = "carnet";
const IMPORT_THREADS: usize = 4;
const REPORT_MAX_LINES: usize = 500;

#[derive(Serialize, Clone, TS)]
#[serde(tag = "status")]
#[ts(export)]
pub enum ImportOutcome {
    /// `folder` : nom du dossier de la photo, sans le reste du chemin (pastilles de la photothèque).
    Ok { path: String, folder: String, asset: AssetMeta },
    Err { path: String, message: String },
}

#[derive(Serialize, TS)]
#[ts(export)]
pub struct InstantiatedPage {
    pub page: Page,
    pub assets: Vec<AssetMeta>,
}

/// Nom du dossier qui contient le fichier, seul.
fn folder_name(path: &str) -> String {
    Path::new(path).parent().and_then(|p| p.file_name()).map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
}

/// Importe en parallèle (4 fils au plus) et envoie un résultat par photo dès qu'elle est prête.
/// Exécutée hors du fil principal : la fenêtre reste réactive.
#[tauri::command(async)]
pub fn import_photos(state: State<'_, AppState>, paths: Vec<String>, on_progress: Channel<ImportOutcome>) -> Result<(), String> {
    let files: Vec<PathBuf> = paths.iter().map(PathBuf::from).collect();
    let failures = AtomicUsize::new(0);
    photo::import_many(&files, IMPORT_THREADS, |i, r| {
        let path = paths[i].clone();
        let r = r.and_then(|a| {
            let meta = a.meta.clone();
            state.store.lock().unwrap().insert(a).map(|()| meta)
        });
        let outcome = match r {
            Ok(asset) => ImportOutcome::Ok { folder: folder_name(&path), path, asset },
            Err(e) => {
                failures.fetch_add(1, Ordering::Relaxed);
                log::warn!("import_photo_failed kind={e:?}");
                ImportOutcome::Err { path, message: e.message_fr() }
            }
        };
        let _ = on_progress.send(outcome);
    });
    log::info!("import_photos total={} failures={}", paths.len(), failures.into_inner());
    Ok(())
}

/// Garde les chemins qui désignent encore un fichier.
#[tauri::command]
pub fn existing_paths(paths: Vec<String>) -> Vec<String> {
    paths.into_iter().filter(|p| Path::new(p).is_file()).collect()
}

#[tauri::command]
pub fn list_importable_in(dir: String) -> Vec<String> {
    photo::list_importable(Path::new(&dir)).into_iter().map(|p| p.to_string_lossy().into_owned()).collect()
}

/// Exécutée hors du fil principal : la fenêtre reste réactive.
#[tauri::command(async)]
pub fn save_document(state: State<'_, AppState>, path: String, doc: Document) -> Result<(), String> {
    let store = state.store.lock().unwrap();
    native::save_to_path(&doc, &store, &FontSet::builtin(), Path::new(&path)).map_err(|e| {
        log::error!("save_document_failed kind={e:?}");
        e.message_fr()
    })?;
    log::info!("save_document pages={}", doc.pages.len());
    Ok(())
}

/// Exécutée hors du fil principal : la fenêtre reste réactive.
#[tauri::command(async)]
pub fn open_document(state: State<'_, AppState>, path: String) -> Result<Document, String> {
    let native::LoadedDocument { doc, assets, .. } = native::load_from_path(Path::new(&path)).map_err(|e| {
        log::error!("open_document_failed kind={e:?}");
        e.message_fr()
    })?;
    let mut store = state.store.lock().unwrap();
    for a in assets {
        store.insert(a).map_err(|e| e.message_fr())?;
    }
    log::info!("open_document pages={}", doc.pages.len());
    Ok(doc)
}

/// Exécutée hors du fil principal : la fenêtre reste réactive.
#[tauri::command(async)]
pub fn export_odg(state: State<'_, AppState>, path: String, doc: Document) -> Result<(), String> {
    let store = state.store.lock().unwrap();
    odg::export_odg_to_path(&doc, &store, &FontSet::builtin(), Path::new(&path)).map_err(|e| {
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

/// Portrait ou paysage : la grille éventuelle est recalculée pour la nouvelle taille.
#[tauri::command]
pub fn page_set_size(mut page: Page, width_mm: f64, height_mm: f64) -> Page {
    grid::set_page_size(&mut page, PageSize { width_mm, height_mm });
    page
}

#[tauri::command]
pub fn grid_liberate(mut page: Page) -> Page {
    grid::liberate(&mut page);
    page
}

/// Exécutée hors du fil principal : la fenêtre reste réactive.
#[tauri::command(async)]
pub fn list_templates(state: State<'_, AppState>) -> Result<Vec<TemplateInfo>, String> {
    Ok(state.library.list())
}

/// Exécutée hors du fil principal : la fenêtre reste réactive.
#[tauri::command(async)]
pub fn instantiate_template(state: State<'_, AppState>, id: String) -> Result<InstantiatedPage, String> {
    let t = state.library.load(&id).ok_or("Ce modèle est introuvable ou abîmé.")?;
    let (page, assets) = templates::instantiate(&t);
    let mut store = state.store.lock().unwrap();
    let metas = assets.iter().map(|a| a.meta.clone()).collect();
    for a in assets {
        store.insert(a).map_err(|e| e.message_fr())?;
    }
    Ok(InstantiatedPage { page, assets: metas })
}

/// Exécutée hors du fil principal : la fenêtre reste réactive.
#[tauri::command(async)]
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
    FontSet::builtin().families()
}

fn log_path(app: &AppHandle) -> Option<PathBuf> {
    Some(app.path().app_log_dir().ok()?.join(format!("{LOG_FILE_NAME}.log")))
}

/// Exécutée hors du fil principal : la fenêtre reste réactive.
#[tauri::command(async)]
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

/// PDF au rendu de l'écran, via WebView2 ; ailleurs, l'interface passe par « Imprimer ».
#[tauri::command]
pub async fn export_pdf(webview: tauri::Webview, path: String, landscape: bool) -> Result<(), String> {
    #[cfg(windows)]
    {
        let result = tauri::async_runtime::spawn_blocking(move || crate::pdf_windows::print_to_pdf(&webview, &path, landscape))
            .await
            .map_err(|e| e.to_string())?;
        log::info!("export_pdf ok={}", result.is_ok());
        result
    }
    #[cfg(not(windows))]
    {
        let _ = (webview, path, landscape);
        Err("L'export PDF direct n'existe que sous Windows : utilisez Imprimer, puis Enregistrer en PDF.".into())
    }
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
    fn page_set_size_keeps_grid_cells_on_the_page() {
        let page = page_set_size(grid_set(Page::new(A4_PORTRAIT), 2, 4), 297.0, 210.0);
        assert!(page.elements.iter().all(|e| e.frame.y_mm + e.frame.h_mm <= 210.0));
    }

    #[test]
    fn grid_commands_return_the_updated_page() {
        let page = grid_set(Page::new(A4_PORTRAIT), 2, 2);
        assert_eq!(page.elements.len(), 4);
        let page = grid_liberate(page);
        assert!(page.grid.is_none());
    }

    #[test]
    fn folder_is_the_parent_name_only() {
        assert_eq!(folder_name("/a/Sortie ferme/x.jpg"), "Sortie ferme");
    }

    #[test]
    fn existing_paths_drops_missing_files() {
        let dir = tempfile::tempdir().unwrap();
        let kept = dir.path().join("là.jpg");
        std::fs::write(&kept, b"x").unwrap();
        let gone = dir.path().join("parti.jpg").to_string_lossy().into_owned();
        let kept = kept.to_string_lossy().into_owned();
        assert_eq!(existing_paths(vec![kept.clone(), gone]), vec![kept]);
    }

    #[test]
    fn pending_paths_keep_only_existing_files() {
        let dir = tempfile::tempdir().unwrap();
        let kept = dir.path().join("gardée.jpg");
        std::fs::write(&kept, b"x").unwrap();
        let removed = dir.path().join("supprimée.jpg");
        std::fs::write(&removed, b"x").unwrap();
        std::fs::remove_file(&removed).unwrap();
        let folder = dir.path().join("devenu un dossier.jpg");
        std::fs::create_dir(&folder).unwrap();
        let s = |p: &std::path::Path| p.to_string_lossy().into_owned();
        assert_eq!(existing_paths(vec![s(&kept), s(&removed), s(&folder)]), vec![s(&kept)]);
    }
}
