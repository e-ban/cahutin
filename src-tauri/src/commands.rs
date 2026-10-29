//! Commandes appelées par l'interface. Les erreurs sont des messages en français, prêts à afficher.
//! Journal : uniquement des noms d'événements et des compteurs, jamais de chemin ni de contenu.

use crate::state::AppState;
use carnet_core::fonts::{self, FontFamilyInfo};
use carnet_core::photo;
use carnet_core::templates::{self, TemplateInfo};
use carnet_core::import::{self, ImportError, ImportReport};
use carnet_core::{AssetMeta, Document, Page, PageSize, ZoneKind, examples, grid, native, odg, report};
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
    let cache = state.store.lock().unwrap().cache_dir();
    photo::import_many(&files, IMPORT_THREADS, |i, r| {
        let path = paths[i].clone();
        // Écriture dans le cache hors du verrou : les vignettes restent servies pendant l'import.
        let r = r.and_then(|a| photo::stage(cache.as_deref(), a)).map(|staged| {
            let meta = staged.meta().clone();
            state.store.lock().unwrap().commit(staged);
            meta
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

fn save_document_in(state: &AppState, path: &Path, doc: &Document) -> Result<(), String> {
    let store = state.store.lock().unwrap();
    native::save_to_path(doc, &store, &state.fonts.read().unwrap(), path).map_err(|e| {
        log::error!("save_document_failed kind={e:?}");
        e.message_fr()
    })?;
    log::info!("save_document pages={}", doc.pages.len());
    Ok(())
}

/// Exécutée hors du fil principal : la fenêtre reste réactive.
#[tauri::command(async)]
pub fn save_document(state: State<'_, AppState>, path: String, doc: Document) -> Result<(), String> {
    save_document_in(&state, Path::new(&path), &doc)
}

#[derive(Serialize, TS)]
#[ts(export)]
pub struct OpenedDocument {
    pub doc: Document,
    /// Familles transportées par le document et absentes du poste, à proposer à l'ajout.
    pub offered_fonts: Vec<String>,
}

/// Polices transportées et absentes du poste : gardées en attente, familles à proposer à l'ajout.
/// Une ouverture remplace la liste en attente ; un import (`append`) la complète, pour que les polices
/// de plusieurs documents importés ensemble puissent toutes être ajoutées.
fn offer_fonts(state: &AppState, fonts: Vec<(String, Vec<u8>)>, append: bool) -> Vec<String> {
    let installed = state.fonts.read().unwrap().families();
    let mut offered_fonts: Vec<String> = Vec::new();
    let mut pending = if append { std::mem::take(&mut *state.pending_fonts.lock().unwrap()) } else { Vec::new() };
    let already: Vec<String> = pending.iter().filter_map(|(_, b): &(String, Vec<u8>)| fonts::family_of(b)).collect();
    for (file, bytes) in fonts {
        let Some(family) = fonts::family_of(&bytes) else { continue };
        if installed.iter().any(|f| f.name.eq_ignore_ascii_case(&family)) {
            continue;
        }
        if !offered_fonts.contains(&family) {
            offered_fonts.push(family.clone());
        }
        if !already.iter().any(|f| f.eq_ignore_ascii_case(&family)) {
            pending.push((file, bytes));
        }
    }
    *state.pending_fonts.lock().unwrap() = pending;
    offered_fonts
}

fn open_document_in(state: &AppState, path: &Path) -> Result<OpenedDocument, String> {
    let native::LoadedDocument { doc, assets, fonts } = native::load_from_path(path).map_err(|e| {
        log::error!("open_document_failed kind={e:?}");
        e.message_fr()
    })?;
    {
        let mut store = state.store.lock().unwrap();
        for a in assets {
            store.insert(a).map_err(|e| e.message_fr())?;
        }
    }
    let offered_fonts = offer_fonts(state, fonts, false);
    log::info!("open_document pages={} offered_fonts={}", doc.pages.len(), offered_fonts.len());
    Ok(OpenedDocument { doc, offered_fonts })
}

#[derive(Serialize, TS)]
#[ts(export)]
pub struct ImportedPages {
    pub pages: Vec<Page>,
    pub assets: Vec<AssetMeta>,
    /// Familles transportées par un document Carnet importé et absentes du poste.
    pub offered_fonts: Vec<String>,
    pub report: ImportReport,
}

/// Type de fichier pour le journal : un des formats connus, sinon « autre » (une extension inconnue peut
/// être un prénom : « Bilan.Martin »).
fn log_kind(path: &Path) -> &'static str {
    const KNOWN: [&str; 7] = ["carnet", "docx", "doc", "odt", "odg", "pdf", "pub"];
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
    KNOWN.into_iter().find(|k| *k == ext).unwrap_or("autre")
}

/// Ligne du journal d'un import : type de fichier et compteurs, jamais le nom ni le texte.
fn import_log_line(kind: &str, result: Result<&ImportReport, ImportError>, ms: u128) -> String {
    match result {
        Ok(r) => format!("import_file kind={kind} pages={} texts={} images={} shapes={} skipped={} ms={ms}", r.pages, r.texts, r.images, r.shapes, r.skipped),
        Err(e) => format!("import_file_failed kind={kind} error={e:?} ms={ms}"),
    }
}

fn import_file_in(state: &AppState, path: &Path, soffice: Option<&Path>) -> Result<ImportedPages, String> {
    state.import_cancel.store(false, Ordering::SeqCst);
    let kind = log_kind(path);
    let known: Vec<String> = state.fonts.read().unwrap().families().into_iter().map(|f| f.name).collect();
    let start = std::time::Instant::now();
    let result = import::office::import_file(path, soffice, &known, &state.import_cancel);
    let ms = start.elapsed().as_millis();
    let imported = match result {
        Ok(i) => {
            log::info!("{}", import_log_line(kind, Ok(&i.report), ms));
            i
        }
        Err(e) => {
            log::error!("{}", import_log_line(kind, Err(e), ms));
            return Err(e.message_fr());
        }
    };
    let assets: Vec<AssetMeta> = imported.assets.iter().map(|a| a.meta.clone()).collect();
    {
        let mut store = state.store.lock().unwrap();
        for a in imported.assets {
            store.insert(a).map_err(|e| e.message_fr())?;
        }
    }
    let offered_fonts = offer_fonts(state, imported.fonts, true);
    Ok(ImportedPages { pages: imported.pages, assets, offered_fonts, report: imported.report })
}

/// Exécutée hors du fil principal : la fenêtre reste réactive pendant la conversion.
#[tauri::command(async)]
pub fn import_file(state: State<'_, AppState>, path: String) -> Result<ImportedPages, String> {
    let soffice = import::office::find_soffice(&import::office::SofficeLookup::from_env());
    import_file_in(&state, Path::new(&path), soffice.as_deref())
}

#[tauri::command]
pub fn cancel_import(state: State<'_, AppState>) {
    state.import_cancel.store(true, Ordering::SeqCst);
}

/// Exécutée hors du fil principal : la fenêtre reste réactive.
#[tauri::command(async)]
pub fn open_document(state: State<'_, AppState>, path: String) -> Result<OpenedDocument, String> {
    open_document_in(&state, Path::new(&path))
}

/// Exécutée hors du fil principal : la fenêtre reste réactive.
#[tauri::command(async)]
pub fn export_odg(state: State<'_, AppState>, path: String, doc: Document) -> Result<(), String> {
    let store = state.store.lock().unwrap();
    odg::export_odg_to_path(&doc, &store, &state.fonts.read().unwrap(), Path::new(&path)).map_err(|e| {
        log::error!("export_odg_failed kind={}", e.kind());
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
pub fn list_examples() -> Vec<examples::ExampleInfo> {
    examples::list()
}

/// Copie neuve de l'exemple, sans titre ni chemin.
#[tauri::command]
pub fn open_example(id: String) -> Result<Document, String> {
    examples::document(&id).ok_or_else(|| "Cet exemple est introuvable.".to_string())
}

#[tauri::command]
pub fn grid_preset(mut page: Page, preset: grid::GridPreset) -> Page {
    grid::apply_preset(&mut page, preset);
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

#[tauri::command(async)]
pub fn list_fonts(state: State<'_, AppState>) -> Vec<FontFamilyInfo> {
    state.fonts.read().unwrap().families()
}

fn add_font_in(state: &AppState, path: &Path) -> Result<FontFamilyInfo, String> {
    let info = fonts::add_user_font(&state.fonts_dir, path).map_err(|e| {
        log::warn!("add_font_failed kind={e:?}");
        e.message_fr()
    })?;
    state.reload_fonts();
    log::info!("add_font embeddable={}", info.embeddable);
    Ok(info)
}

/// Exécutée hors du fil principal : la fenêtre reste réactive.
#[tauri::command(async)]
pub fn add_font(state: State<'_, AppState>, path: String) -> Result<FontFamilyInfo, String> {
    add_font_in(&state, Path::new(&path))
}

fn remove_font_in(state: &AppState, family: &str) -> Result<(), String> {
    let is_user = state.fonts.read().unwrap().families().iter().any(|f| f.user && f.name == family);
    if !is_user {
        return Err("Les polices fournies avec l'application ne peuvent pas être supprimées.".into());
    }
    fonts::remove_user_family(&state.fonts_dir, family).map_err(|_| "Impossible de supprimer cette police.".to_string())?;
    state.reload_fonts();
    log::info!("remove_font");
    Ok(())
}

#[tauri::command]
pub fn remove_font(state: State<'_, AppState>, family: String) -> Result<(), String> {
    remove_font_in(&state, &family)
}

#[derive(Serialize, TS)]
#[ts(export)]
pub struct FontFileInfo {
    pub family: String,
    pub bold: bool,
    pub italic: bool,
    /// Police du poste : l'interface ne la charge que lorsqu'un texte l'utilise.
    pub system: bool,
    /// Chemin servi par le protocole `carnet-asset` : `/font/<fichier>` ou `/sysfont/<indice>`.
    pub url_path: String,
}

fn font_files_in(state: &AppState) -> Vec<FontFileInfo> {
    let set = state.fonts.read().unwrap();
    set.faces()
        .iter()
        .filter_map(|f| {
            let url_path = if f.user {
                format!("/font/{}", f.file_name)
            } else {
                format!("/sysfont/{}", state.system_font_index(f.system_path()?)?)
            };
            Some(FontFileInfo { family: f.family.clone(), bold: f.bold, italic: f.italic, system: f.system, url_path })
        })
        .collect()
}

/// Fichiers des polices ajoutées et du poste, que l'interface charge pour l'affichage.
#[tauri::command(async)]
pub fn font_files(state: State<'_, AppState>) -> Vec<FontFileInfo> {
    font_files_in(&state)
}

fn accept_document_fonts_in(state: &AppState) -> Result<Vec<FontFamilyInfo>, String> {
    let pending = std::mem::take(&mut *state.pending_fonts.lock().unwrap());
    let mut added: Vec<FontFamilyInfo> = Vec::new();
    let mut failures = 0;
    // Un fichier abîmé n'empêche pas d'ajouter les autres polices du document.
    for (file, bytes) in pending {
        match fonts::add_user_font_bytes(&state.fonts_dir, &file, &bytes) {
            Ok(info) if !added.iter().any(|f| f.name == info.name) => added.push(info),
            Ok(_) => {}
            Err(e) => {
                failures += 1;
                log::warn!("accept_document_font_failed kind={e:?}");
            }
        }
    }
    state.reload_fonts();
    log::info!("accept_document_fonts count={} failures={failures}", added.len());
    Ok(added)
}

#[tauri::command]
pub fn accept_document_fonts(state: State<'_, AppState>) -> Result<Vec<FontFamilyInfo>, String> {
    accept_document_fonts_in(&state)
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
    use carnet_core::photo::AssetStore;
    use carnet_core::templates::Library;
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

    fn test_state(dir: &Path) -> AppState {
        AppState::new(AssetStore::default(), Library::open(dir.join("modeles")).unwrap(), dir.join("polices"))
    }

    fn fixture(name: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../crates/carnet-core/tests/fixtures").join(name)
    }

    #[test]
    fn remove_font_refuses_builtin() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_state(dir.path());
        assert!(remove_font_in(&state, "Andika").is_err());
        add_font_in(&state, &fixture("CarnetTest-Regular.ttf")).unwrap();
        assert!(remove_font_in(&state, "Carnet Test").is_ok());
        assert!(!state.fonts.read().unwrap().families().iter().any(|f| f.name == "Carnet Test"));
    }

    #[test]
    fn import_carnet_adds_assets_and_offers_fonts() {
        let dir = tempfile::tempdir().unwrap();
        let author = test_state(&dir.path().join("auteur"));
        add_font_in(&author, &fixture("CarnetTest-Regular.ttf")).unwrap();
        let asset = photo::import_path(&fixture("gps.jpg")).unwrap();
        let meta = asset.meta.clone();
        author.store.lock().unwrap().insert(asset).unwrap();
        let mut doc = Document::new("sortie");
        let mut style = carnet_core::TextStyle::default_zone();
        style.font = "Carnet Test".into();
        let frame = carnet_core::Frame { x_mm: 10.0, y_mm: 10.0, w_mm: 50.0, h_mm: 20.0, rotation_deg: 0.0 };
        doc.pages[0].elements.push(carnet_core::Element {
            id: carnet_core::new_id(),
            frame,
            locked: false,
            zone: false,
            group: None,
            kind: carnet_core::ElementKind::Text(carnet_core::TextProps { content: carnet_core::RichText::plain("bonjour"), style, prompt: None, frame: None }),
        });
        doc.pages[0].elements.push(carnet_core::Element {
            id: carnet_core::new_id(),
            frame,
            locked: false,
            zone: false,
            group: None,
            kind: carnet_core::ElementKind::Photo(carnet_core::PhotoProps { asset: Some(meta.id.clone()), crop: carnet_core::Crop::COVER, corner_radius_mm: 0.0, border: None, shadow: false }),
        });
        doc.assets.insert(meta.id.clone(), meta.clone());
        let path = dir.path().join("sortie.carnet");
        save_document_in(&author, &path, &doc).unwrap();

        let colleague = test_state(&dir.path().join("collègue"));
        let imported = import_file_in(&colleague, &path, None).unwrap();
        assert_eq!(imported.pages.len(), 1);
        assert!(imported.pages[0].elements.iter().all(|e| doc.pages[0].elements.iter().all(|o| o.id != e.id)));
        assert_eq!(imported.assets.iter().map(|a| a.id.clone()).collect::<Vec<_>>(), std::slice::from_ref(&meta.id));
        assert!(colleague.store.lock().unwrap().get(&meta.id).is_some());
        assert_eq!(imported.offered_fonts, vec!["Carnet Test".to_string()]);
        assert_eq!(imported.report.texts, 1);
    }

    /// Document Carnet dont un texte utilise la police du fichier `font` (ajoutée chez l'auteur).
    fn carnet_with_font(dir: &Path, font: &str, family: &str, name: &str) -> PathBuf {
        let author = test_state(&dir.join(format!("auteur-{name}")));
        add_font_in(&author, &fixture(font)).unwrap();
        let mut doc = Document::new(name);
        let mut style = carnet_core::TextStyle::default_zone();
        style.font = family.into();
        doc.pages[0].elements.push(carnet_core::Element {
            id: carnet_core::new_id(),
            frame: carnet_core::Frame { x_mm: 10.0, y_mm: 10.0, w_mm: 50.0, h_mm: 20.0, rotation_deg: 0.0 },
            locked: false,
            zone: false,
            group: None,
            kind: carnet_core::ElementKind::Text(carnet_core::TextProps { content: carnet_core::RichText::plain("bonjour"), style, prompt: None, frame: None }),
        });
        let path = dir.join(format!("{name}.carnet"));
        save_document_in(&author, &path, &doc).unwrap();
        path
    }

    #[test]
    fn fonts_of_several_imported_documents_can_all_be_added() {
        let dir = tempfile::tempdir().unwrap();
        let a = carnet_with_font(dir.path(), "CarnetTest-Regular.ttf", "Carnet Test", "a");
        let b = carnet_with_font(dir.path(), "CarnetAutre-Regular.ttf", "Carnet Autre", "b");
        let colleague = test_state(&dir.path().join("collègue"));
        import_file_in(&colleague, &a, None).unwrap();
        import_file_in(&colleague, &b, None).unwrap();
        let added: Vec<String> = accept_document_fonts_in(&colleague).unwrap().into_iter().map(|f| f.name).collect();
        assert!(added.contains(&"Carnet Test".to_string()) && added.contains(&"Carnet Autre".to_string()), "{added:?}");
    }

    #[test]
    fn office_import_without_libreoffice_is_a_french_message() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_state(dir.path());
        let docx = dir.path().join("fiche.docx");
        std::fs::write(&docx, b"PK").unwrap();
        let err = import_file_in(&state, &docx, None).err().unwrap();
        assert!(err.starts_with("Pour importer ce fichier, installez LibreOffice"), "{err}");
    }

    /// « Bilan.Martin » : l'extension d'un nom de fichier peut être un prénom, elle n'entre pas au journal.
    #[test]
    fn log_kind_is_one_of_the_known_formats() {
        assert_eq!(log_kind(Path::new("/école/Bilan.Martin")), "autre");
        assert_eq!(log_kind(Path::new("Fiche.DOCX")), "docx");
        assert_eq!(log_kind(Path::new("sortie.carnet")), "carnet");
        assert_eq!(log_kind(Path::new("sans-extension")), "autre");
    }

    #[test]
    fn import_log_never_names_the_file() {
        let report = carnet_core::import::ImportReport { pages: 2, texts: 14, images: 1, shapes: 6, skipped: 3 };
        assert_eq!(import_log_line("docx", Ok(&report), 1234), "import_file kind=docx pages=2 texts=14 images=1 shapes=6 skipped=3 ms=1234");
        assert_eq!(import_log_line("pdf", Err(carnet_core::import::ImportError::Timeout), 120000), "import_file_failed kind=pdf error=Timeout ms=120000");
    }

    #[test]
    fn open_document_offers_missing_fonts() {
        let dir = tempfile::tempdir().unwrap();
        let author = test_state(&dir.path().join("auteur"));
        add_font_in(&author, &fixture("CarnetTest-Regular.ttf")).unwrap();
        let mut doc = Document::new("partagé");
        let mut style = carnet_core::TextStyle::default_zone();
        style.font = "Carnet Test".into();
        doc.pages[0].elements.push(carnet_core::Element {
            id: carnet_core::new_id(),
            frame: carnet_core::Frame { x_mm: 10.0, y_mm: 10.0, w_mm: 50.0, h_mm: 20.0, rotation_deg: 0.0 },
            locked: false,
            zone: false,
            group: None,
            kind: carnet_core::ElementKind::Text(carnet_core::TextProps {
                content: carnet_core::RichText::plain("bonjour"),
                style,
                prompt: None,
                frame: None,
            }),
        });
        let path = dir.path().join("partagé.carnet");
        save_document_in(&author, &path, &doc).unwrap();

        let colleague = test_state(&dir.path().join("collègue"));
        let opened = open_document_in(&colleague, &path).unwrap();
        assert_eq!(opened.offered_fonts, vec!["Carnet Test".to_string()]);
        let added = accept_document_fonts_in(&colleague).unwrap();
        assert_eq!(added.iter().map(|f| f.name.as_str()).collect::<Vec<_>>(), ["Carnet Test"]);
        assert!(colleague.fonts.read().unwrap().face("Carnet Test", false, false).is_some());
        // Chez l'auteur, la police est déjà là : rien à proposer.
        assert!(open_document_in(&author, &path).unwrap().offered_fonts.is_empty());
    }

    #[test]
    fn font_files_serve_system_fonts_by_index() {
        let dir = tempfile::tempdir().unwrap();
        let sys = dir.path().join("systeme");
        std::fs::create_dir(&sys).unwrap();
        std::fs::copy(fixture("CarnetTest-Regular.ttf"), sys.join("carnettest.ttf")).unwrap();
        let state = test_state(dir.path());
        state.set_system_fonts(fonts::scan_font_dirs(&[sys.as_path()]));
        let files = font_files_in(&state);
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].family, "Carnet Test");
        assert!(files[0].system);
        assert_eq!(files[0].url_path, "/sysfont/0");
        assert_eq!(state.system_font_path(0).unwrap(), sys.join("carnettest.ttf"));
        assert!(remove_font_in(&state, "Carnet Test").is_err());
    }

    #[test]
    fn a_broken_document_font_does_not_stop_the_others() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_state(dir.path());
        let good = std::fs::read(fixture("CarnetTest-Regular.ttf")).unwrap();
        *state.pending_fonts.lock().unwrap() = vec![("abimee.ttf".into(), b"pas une police".to_vec()), ("CarnetTest-Regular.ttf".into(), good)];
        let added = accept_document_fonts_in(&state).unwrap();
        assert_eq!(added.iter().map(|f| f.name.as_str()).collect::<Vec<_>>(), ["Carnet Test"]);
        assert!(state.fonts.read().unwrap().face("Carnet Test", false, false).is_some());
    }
}
