//! Copie de secours du document ouvert, écrite régulièrement par l'interface dans le dossier de
//! l'application (`secours/`), avec ses photos : après une panne, le travail non enregistré se reprend au
//! lancement suivant. Effacée à chaque enregistrement et quand l'utilisateur quitte le document.

use crate::commands::{OpenedDocument, open_document_in};
use crate::state::AppState;
use carnet_core::fonts::FontSet;
use carnet_core::photo::AssetStore;
use carnet_core::{Document, native};
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::Path;
use tauri::State;
use ts_rs::TS;

pub const DOCUMENT_FILE: &str = "document.carnet";
pub const INFO_FILE: &str = "info.json";

/// Ce que la question de reprise dit de la copie : son titre, le fichier d'où vient le document (pour
/// l'enregistrer au même endroit), et quand elle a été écrite (UTC, RFC 3339 : l'interface l'affiche à
/// l'heure locale).
#[derive(Serialize, Deserialize, Clone, PartialEq, Debug, TS)]
#[ts(export)]
pub struct BackupInfo {
    pub source: Option<String>,
    pub title: String,
    pub saved_at: String,
}

const SAVE_FAILED: &str = "La copie de secours n'a pas pu être écrite.";

/// Écrit la copie puis sa fiche, chacune d'un bloc : un échec laisse la copie précédente entière.
pub fn save_in(dir: &Path, doc: &Document, store: &AssetStore, fonts: &FontSet, source: Option<String>) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|_| SAVE_FAILED.to_string())?;
    native::save_to_path(doc, store, fonts, &dir.join(DOCUMENT_FILE)).map_err(|e| e.message_fr())?;
    let saved_at = time::OffsetDateTime::now_utc().format(&time::format_description::well_known::Rfc3339).unwrap_or_default();
    let info = BackupInfo { source, title: doc.title.clone(), saved_at };
    let json = serde_json::to_vec_pretty(&info).map_err(|_| SAVE_FAILED.to_string())?;
    carnet_core::fsutil::write_atomically(&dir.join(INFO_FILE), |f| f.write_all(&json)).map_err(|_: std::io::Error| SAVE_FAILED.to_string())
}

/// La fiche de la copie, s'il y a une copie complète et lisible.
pub fn info_in(dir: &Path) -> Option<BackupInfo> {
    if !dir.join(DOCUMENT_FILE).is_file() {
        return None;
    }
    serde_json::from_slice(&std::fs::read(dir.join(INFO_FILE)).ok()?).ok()
}

/// Efface la copie ; les erreurs sont ignorées.
pub fn clear_in(dir: &Path) {
    let _ = std::fs::remove_file(dir.join(INFO_FILE));
    let _ = std::fs::remove_file(dir.join(DOCUMENT_FILE));
}

/// Exécutée hors du fil principal : l'interface reste réactive pendant l'écriture.
#[tauri::command(async)]
pub fn backup_save(state: State<'_, AppState>, doc: Document, source: Option<String>) -> Result<(), String> {
    // Comme l'enregistrement : le magasin n'est tenu que le temps de la copie de ses entrées.
    let snapshot = state.store.lock().unwrap().subset(&native::used_assets(&doc));
    save_in(&state.backup_dir, &doc, &snapshot, &state.fonts.read().unwrap(), source)?;
    log::info!("backup_saved");
    Ok(())
}

#[tauri::command]
pub fn backup_info(state: State<'_, AppState>) -> Option<BackupInfo> {
    info_in(&state.backup_dir)
}

#[tauri::command(async)]
pub fn backup_open(state: State<'_, AppState>) -> Result<OpenedDocument, String> {
    let opened = open_document_in(&state, &state.backup_dir.join(DOCUMENT_FILE))?;
    log::info!("backup_restored");
    Ok(opened)
}

#[tauri::command]
pub fn backup_clear(state: State<'_, AppState>) {
    clear_in(&state.backup_dir);
    log::info!("backup_cleared");
}

#[cfg(test)]
mod tests {
    use super::*;
    use carnet_core::fonts::FontSet;
    use carnet_core::native;
    use carnet_core::photo::AssetStore;
    use carnet_core::*;

    fn photo_element(asset: &str) -> Element {
        Element {
            id: new_id(),
            frame: Frame { x_mm: 10.0, y_mm: 10.0, w_mm: 50.0, h_mm: 40.0, rotation_deg: 0.0 },
            locked: false,
            zone: true,
            group: None,
            kind: ElementKind::Photo(PhotoProps { asset: Some(asset.into()), crop: Crop::COVER, corner_radius_mm: 0.0, border: None, shadow: false, mirrored: false }),
            subgroups: Vec::new(),
            flip_h: false,
            flip_v: false,
        }
    }

    #[test]
    fn a_backup_is_announced_then_cleared() {
        let dir = tempfile::tempdir().unwrap();
        let dir = dir.path().join("secours");
        assert_eq!(info_in(&dir), None);
        let doc = Document::new("Sortie à la ferme");
        save_in(&dir, &doc, &AssetStore::default(), &FontSet::builtin(), Some("/x/ferme.carnet".into())).unwrap();
        let info = info_in(&dir).unwrap();
        assert_eq!((info.title.as_str(), info.source.as_deref()), ("Sortie à la ferme", Some("/x/ferme.carnet")));
        assert!(native::load_from_path(&dir.join(DOCUMENT_FILE)).is_ok());
        clear_in(&dir);
        assert_eq!(info_in(&dir), None);
    }

    /// Une photo absente du magasin : la copie échoue et la copie précédente reste.
    #[test]
    fn a_failed_backup_keeps_the_previous_one() {
        let dir = tempfile::tempdir().unwrap();
        let dir = dir.path().join("secours");
        let first = Document::new("Première");
        save_in(&dir, &first, &AssetStore::default(), &FontSet::builtin(), None).unwrap();
        let mut broken = Document::new("Cassée");
        let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../crates/carnet-core/tests/fixtures/faces-apollo11.jpg");
        let a = carnet_core::photo::import_path(&fixture).unwrap();
        broken.assets.insert(a.meta.id.clone(), a.meta.clone());
        broken.pages[0].elements.push(photo_element(&a.meta.id));
        assert!(save_in(&dir, &broken, &AssetStore::default(), &FontSet::builtin(), None).is_err());
        assert_eq!(info_in(&dir).unwrap().title, "Première");
        assert!(native::load_from_path(&dir.join(DOCUMENT_FILE)).is_ok());
    }
}
