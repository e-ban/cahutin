//! Commandes de la liste de la classe. Les classes vivent dans `Documents/Cahutin/Classes` ; leurs photos
//! portraits rejoignent les images de l'application à l'ouverture. Le journal ne reçoit que des nombres :
//! ni prénom, ni nom de classe, ni chemin.

use crate::commands::ImportedPages;
use crate::state::AppState;
use carnet_core::class::{self, CLASS_EXTENSION, Class, Student};
use carnet_core::fill::{self, FillMode};
use carnet_core::import::ImportReport;
use carnet_core::photo::{self, AssetStore};
use carnet_core::{AssetMeta, ElementKind, Page};
use serde::Serialize;
use std::path::{Path, PathBuf};
use tauri::State;
use ts_rs::TS;

#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[ts(export)]
pub struct ClassFileInfo {
    pub path: String,
    pub name: String,
    pub year: String,
    pub students: usize,
}

#[derive(Serialize, Clone, Debug, TS)]
#[ts(export)]
pub struct OpenedClass {
    pub path: String,
    pub class: Class,
}

fn ensure_dir(dir: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|_| "Le dossier des classes ne peut pas être créé.".to_string())
}

/// Classes du dossier, de l'année la plus récente à la plus ancienne, puis par nom. Un fichier illisible
/// est ignoré.
pub fn list_in(dir: &Path) -> Vec<ClassFileInfo> {
    let Ok(entries) = std::fs::read_dir(dir) else { return Vec::new() };
    let mut out: Vec<ClassFileInfo> = entries
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some(CLASS_EXTENSION))
        .filter_map(|p| {
            let c = class::load_from_path(&p).ok()?.class;
            Some(ClassFileInfo { path: p.to_string_lossy().into_owned(), name: c.name, year: c.year, students: c.students.len() })
        })
        .collect();
    out.sort_by(|a, b| b.year.cmp(&a.year).then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase())));
    out
}

pub fn create_in(dir: &Path, store: &AssetStore, name: &str, year: &str) -> Result<OpenedClass, String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("Donnez un nom à la classe.".into());
    }
    ensure_dir(dir)?;
    let c = Class::new(name, year.trim());
    let path = dir.join(class::default_file_name(&c));
    if path.exists() {
        return Err("Une classe porte déjà ce nom pour cette année.".into());
    }
    class::save_to_path(&c, store, &path).map_err(|e| e.message_fr())?;
    Ok(OpenedClass { path: path.to_string_lossy().into_owned(), class: c })
}

pub fn open_into(path: &Path, store: &mut AssetStore) -> Result<OpenedClass, String> {
    let loaded = class::load_from_path(path).map_err(|e| e.message_fr())?;
    for a in loaded.assets {
        store.insert(a).map_err(|e| e.message_fr())?;
    }
    Ok(OpenedClass { path: path.to_string_lossy().into_owned(), class: loaded.class })
}

/// Seulement dans le dossier des classes : l'interface ne peut pas faire écrire ailleurs.
pub fn save_in(dir: &Path, path: &Path, c: &Class, store: &AssetStore) -> Result<(), String> {
    let inside = path.parent().zip(dir.canonicalize().ok()).is_some_and(|(p, d)| p.canonicalize().ok() == Some(d));
    if !inside || path.extension().and_then(|e| e.to_str()) != Some(CLASS_EXTENSION) {
        return Err("Cette classe ne peut pas être enregistrée à cet endroit.".into());
    }
    class::save_to_path(c, store, path).map_err(|e| e.message_fr())
}

/// Pages pour les élèves, avec les photos qu'elles utilisent.
pub fn generate_with(page: &Page, students: &[Student], mode: FillMode, store: &AssetStore) -> Result<ImportedPages, String> {
    if !fill::has_fields(page) {
        return Err("Cette page n'a ni zone Prénom, ni {prénom}, ni zone photo à remplir.".into());
    }
    if students.is_empty() {
        return Err("Aucun élève ne correspond.".into());
    }
    let pages = fill::generate(page, students, mode);
    let mut assets: Vec<AssetMeta> = Vec::new();
    for e in pages.iter().flat_map(|p| &p.elements) {
        if let ElementKind::Photo(ph) = &e.kind
            && let Some(id) = &ph.asset
            && !assets.iter().any(|m| &m.id == id)
            && let Some(a) = store.get(id)
        {
            assets.push(a.meta);
        }
    }
    let report = ImportReport::count(&pages);
    Ok(ImportedPages { pages, assets, offered_fonts: Vec::new(), report })
}

#[tauri::command]
pub fn class_dir(state: State<'_, AppState>) -> Result<String, String> {
    ensure_dir(&state.classes_dir)?;
    Ok(state.classes_dir.to_string_lossy().into_owned())
}

/// Exécutée hors du fil principal : chaque classe est ouverte pour son nom et son nombre d'élèves.
#[tauri::command(async)]
pub fn class_list(state: State<'_, AppState>) -> Vec<ClassFileInfo> {
    let list = list_in(&state.classes_dir);
    log::info!("class_list count={}", list.len());
    list
}

#[tauri::command]
pub fn class_create(state: State<'_, AppState>, name: String, year: String) -> Result<OpenedClass, String> {
    let store = state.store.lock().unwrap();
    create_in(&state.classes_dir, &store, &name, &year)
}

#[tauri::command(async)]
pub fn class_open(state: State<'_, AppState>, path: String) -> Result<OpenedClass, String> {
    let opened = open_into(Path::new(&path), &mut state.store.lock().unwrap())?;
    log::info!("class_open students={} photos={}", opened.class.students.len(), opened.class.photos.len());
    Ok(opened)
}

#[tauri::command(async)]
pub fn class_save(state: State<'_, AppState>, path: String, class: Class) -> Result<(), String> {
    let store = state.store.lock().unwrap();
    save_in(&state.classes_dir, Path::new(&path), &class, &store)
}

/// Photo portrait : réencodée sans métadonnées, comme à l'import de photos.
#[tauri::command(async)]
pub fn class_import_photo(state: State<'_, AppState>, path: String) -> Result<AssetMeta, String> {
    let asset = photo::import_path(&PathBuf::from(path)).map_err(|e| e.message_fr())?;
    let meta = asset.meta.clone();
    state.store.lock().unwrap().insert(asset).map_err(|e| e.message_fr())?;
    Ok(meta)
}

#[tauri::command]
pub fn class_parse_list(text: String) -> Vec<Student> {
    class::parse_list(&text)
}

#[tauri::command]
pub fn class_generate(state: State<'_, AppState>, page: Page, students: Vec<Student>, mode: FillMode) -> Result<ImportedPages, String> {
    let out = generate_with(&page, &students, mode, &state.store.lock().unwrap())?;
    log::info!("class_generate students={} pages={} photos={}", students.len(), out.pages.len(), out.assets.len());
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use carnet_core::{A4_PORTRAIT, Crop, Element, Frame, PhotoProps, RichText, TextProps, TextStyle, new_id};

    fn page_with_name_zone() -> Page {
        let mut p = Page::new(A4_PORTRAIT);
        let f = Frame { x_mm: 10.0, y_mm: 10.0, w_mm: 80.0, h_mm: 40.0, rotation_deg: 0.0 };
        p.elements.push(Element {
            id: new_id(),
            frame: f,
            locked: false,
            zone: true,
            group: None,
            kind: ElementKind::Text(TextProps { content: RichText::default(), style: TextStyle::default_zone(), prompt: Some("Prénom".into()), frame: None }),
        });
        p.elements.push(Element {
            id: new_id(),
            frame: Frame { y_mm: 60.0, ..f },
            locked: false,
            zone: true,
            group: None,
            kind: ElementKind::Photo(PhotoProps { asset: None, crop: Crop::COVER, corner_radius_mm: 0.0, border: None, shadow: false }),
        });
        p
    }

    #[test]
    fn create_then_list() {
        let dir = tempfile::tempdir().unwrap();
        let store = AssetStore::default();
        let opened = create_in(dir.path(), &store, "Moyenne section", "2026-2027").unwrap();
        assert!(opened.path.ends_with("Moyenne section 2026-2027.cahutin-classe"));
        assert_eq!(list_in(dir.path()), vec![ClassFileInfo { path: opened.path.clone(), name: "Moyenne section".into(), year: "2026-2027".into(), students: 0 }]);
        assert_eq!(create_in(dir.path(), &store, "Moyenne section", "2026-2027").unwrap_err(), "Une classe porte déjà ce nom pour cette année.");
    }

    #[test]
    fn save_only_inside_the_class_folder() {
        let dir = tempfile::tempdir().unwrap();
        let classes = dir.path().join("Classes");
        let store = AssetStore::default();
        let opened = create_in(&classes, &store, "MS", "2026-2027").unwrap();
        let mut c = opened.class.clone();
        c.students = class::parse_list("Léa\nNoé");
        save_in(&classes, Path::new(&opened.path), &c, &store).unwrap();
        assert_eq!(list_in(&classes)[0].students, 2);
        let outside = dir.path().join("ailleurs.cahutin-classe");
        assert!(save_in(&classes, &outside, &c, &store).is_err());
    }

    #[test]
    fn generate_without_consent_carries_no_photo() {
        let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("../crates/carnet-core/tests/fixtures/gps.jpg");
        let asset = photo::import_path(&fixture).unwrap();
        let mut store = AssetStore::default();
        store.insert(asset.clone()).unwrap();
        let mut s = Student::new("Léa", "");
        s.photo = Some(asset.meta.id.clone());
        let page = page_with_name_zone();
        let out = generate_with(&page, std::slice::from_ref(&s), FillMode::PagePerStudent, &store).unwrap();
        assert!(out.assets.is_empty());
        s.photo_allowed = true;
        let out = generate_with(&page, &[s], FillMode::PagePerStudent, &store).unwrap();
        assert_eq!(out.assets, vec![asset.meta]);
    }

    #[test]
    fn generate_explains_what_is_missing() {
        let store = AssetStore::default();
        assert_eq!(generate_with(&page_with_name_zone(), &[], FillMode::Sheet, &store).err().unwrap(), "Aucun élève ne correspond.");
        let blank = Page::new(A4_PORTRAIT);
        assert!(generate_with(&blank, &[Student::new("Léa", "")], FillMode::Sheet, &store).err().unwrap().contains("ni zone Prénom"));
    }
}
