//! Copie de secours du document ouvert, écrite régulièrement par l'interface dans le dossier de
//! l'application (`secours/`), avec ses photos : après une panne, le travail non enregistré se reprend au
//! lancement suivant. Effacée à chaque enregistrement et quand l'utilisateur quitte le document.
//!
//! Chaque fenêtre de Cahutin a son dossier, `secours/<id>/`, verrouillé par `secours/<id>.lock` tant
//! qu'elle vit (comme le cache des images) : une fenêtre n'efface que sa copie, et seule la copie d'une
//! fenêtre arrêtée (verrou libre) est proposée au lancement.

use crate::commands::{OpenedDocument, open_document_in};
use crate::state::AppState;
use carnet_core::fonts::FontSet;
use carnet_core::photo::AssetStore;
use carnet_core::{Document, native};
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::{Path, PathBuf};
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

/// Copie proposée au lancement : le dossier d'une fenêtre arrêtée (`id`) et sa fiche.
#[derive(Serialize, Clone, PartialEq, Debug, TS)]
#[ts(export)]
pub struct BackupOffer {
    pub id: String,
    pub info: BackupInfo,
}

/// Document repris : `source` est retiré si l'original a été enregistré après la copie.
#[derive(Serialize, TS)]
#[ts(export)]
pub struct RestoredDocument {
    pub opened: OpenedDocument,
    pub source: Option<String>,
}

/// Dossier de copie de cette fenêtre, verrouillé tant qu'elle vit.
pub struct BackupSlot {
    path: PathBuf,
    _lock: std::fs::File,
}

impl BackupSlot {
    /// Range d'abord les dossiers vides des fenêtres fermées ; les copies laissées par une panne restent.
    pub fn acquire(root: &Path) -> std::io::Result<BackupSlot> {
        std::fs::create_dir_all(root)?;
        tidy(root);
        let id = carnet_core::new_id();
        let lock = std::fs::File::create(root.join(format!("{id}.lock")))?;
        lock.try_lock().map_err(std::io::Error::other)?;
        let path = root.join(&id);
        std::fs::create_dir_all(&path)?;
        Ok(BackupSlot { path, _lock: lock })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

/// Dossiers des fenêtres arrêtées : ceux dont le verrou est libre.
fn stopped(root: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(root) else { return Vec::new() };
    entries
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "lock"))
        .filter(|p| std::fs::OpenOptions::new().write(true).open(p).is_ok_and(|f| f.try_lock().is_ok()))
        .map(|p| p.with_extension(""))
        .collect()
}

fn remove_slot(dir: &Path) {
    clear_in(dir);
    let _ = std::fs::remove_dir(dir);
    let _ = std::fs::remove_file(dir.with_extension("lock"));
}

fn tidy(root: &Path) {
    for dir in stopped(root).into_iter().filter(|d| info_in(d).is_none()) {
        remove_slot(&dir);
    }
}

/// La copie la plus récente laissée par une fenêtre arrêtée, hors de la fenêtre `own`.
pub fn orphan(root: &Path, own: &Path) -> Option<(String, BackupInfo)> {
    stopped(root)
        .into_iter()
        .filter(|d| d != own)
        .filter_map(|d| Some((d.file_name()?.to_str()?.to_string(), info_in(&d)?)))
        .max_by(|a, b| a.1.saved_at.cmp(&b.1.saved_at))
}

/// Un dossier de copie d'une fenêtre arrêtée, désigné par l'interface.
fn stopped_slot(root: &Path, id: &str) -> Result<PathBuf, String> {
    let dir = root.join(id);
    if carnet_core::is_safe_id(id) && stopped(root).contains(&dir) {
        Ok(dir)
    } else {
        Err("Cette copie de secours est introuvable.".to_string())
    }
}

/// Reprise : la copie passe dans le dossier de la fenêtre `own`, l'ancien dossier disparaît.
pub fn adopt(root: &Path, id: &str, own: &Path) -> Result<(), String> {
    let from = stopped_slot(root, id)?;
    std::fs::create_dir_all(own).map_err(|_| SAVE_FAILED.to_string())?;
    for f in [DOCUMENT_FILE, INFO_FILE] {
        std::fs::rename(from.join(f), own.join(f)).map_err(|_| "La copie de secours n'a pas pu être reprise.".to_string())?;
    }
    remove_slot(&from);
    Ok(())
}

/// « Non, l'oublier » : la copie d'une fenêtre arrêtée est effacée.
pub fn discard(root: &Path, id: &str) -> Result<(), String> {
    remove_slot(&stopped_slot(root, id)?);
    Ok(())
}

/// Le fichier d'origine, s'il existe et n'a pas été enregistré après la copie `copy` ; sinon le document
/// repris sera « Sans titre », pour ne pas écraser un travail plus récent.
pub fn restore_source(source: Option<String>, copy: &Path) -> Option<String> {
    let modified = |p: &Path| std::fs::metadata(p).and_then(|m| m.modified()).ok();
    let original = modified(Path::new(source.as_deref()?))?;
    (original <= modified(copy)?).then_some(source?)
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

fn backup_root(state: &AppState) -> &Path {
    state.backup_dir.parent().unwrap_or(&state.backup_dir)
}

#[tauri::command]
pub fn backup_info(state: State<'_, AppState>) -> Option<BackupOffer> {
    orphan(backup_root(&state), &state.backup_dir).map(|(id, info)| BackupOffer { id, info })
}

#[tauri::command(async)]
pub fn backup_open(state: State<'_, AppState>, id: String) -> Result<RestoredDocument, String> {
    adopt(backup_root(&state), &id, &state.backup_dir)?;
    let copy = state.backup_dir.join(DOCUMENT_FILE);
    let source = restore_source(info_in(&state.backup_dir).and_then(|i| i.source), &copy);
    let opened = open_document_in(&state, &copy)?;
    log::info!("backup_restored source_kept={}", source.is_some());
    Ok(RestoredDocument { opened, source })
}

#[tauri::command]
pub fn backup_discard(state: State<'_, AppState>, id: String) -> Result<(), String> {
    discard(backup_root(&state), &id)?;
    log::info!("backup_discarded");
    Ok(())
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

    /// Chaque fenêtre a son dossier, verrouillé tant qu'elle vit : seule la copie d'une fenêtre arrêtée est proposée.
    #[test]
    fn only_the_copy_of_a_stopped_instance_is_offered() {
        let root = tempfile::tempdir().unwrap();
        let first = BackupSlot::acquire(root.path()).unwrap();
        save_in(first.path(), &Document::new("Première fenêtre"), &AssetStore::default(), &FontSet::builtin(), None).unwrap();
        let second = BackupSlot::acquire(root.path()).unwrap();
        assert_eq!(orphan(root.path(), second.path()), None, "la première fenêtre vit encore");
        drop(first);
        let (id, info) = orphan(root.path(), second.path()).unwrap();
        assert_eq!(info.title, "Première fenêtre");
        // Reprise : la copie devient celle de la fenêtre qui la reprend, l'ancien dossier disparaît.
        adopt(root.path(), &id, second.path()).unwrap();
        assert_eq!(info_in(second.path()).unwrap().title, "Première fenêtre");
        assert_eq!(orphan(root.path(), second.path()), None);
        assert!(!root.path().join(&id).exists());
    }

    #[test]
    fn a_discarded_copy_is_gone_and_empty_slots_are_tidied() {
        let root = tempfile::tempdir().unwrap();
        let crashed = BackupSlot::acquire(root.path()).unwrap();
        save_in(crashed.path(), &Document::new("Oubliée"), &AssetStore::default(), &FontSet::builtin(), None).unwrap();
        drop(crashed);
        let quit = BackupSlot::acquire(root.path()).unwrap();
        drop(quit);
        let me = BackupSlot::acquire(root.path()).unwrap();
        // Le dossier vide d'une fenêtre fermée normalement est rangé ; la copie de l'autre reste.
        assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 2 + 2);
        let (id, _) = orphan(root.path(), me.path()).unwrap();
        assert!(discard(root.path(), "../dehors").is_err());
        discard(root.path(), &id).unwrap();
        assert_eq!(orphan(root.path(), me.path()), None);
    }

    /// L'original a été enregistré après la copie : la reprise ne doit pas l'écraser au premier Ctrl+S.
    #[test]
    fn a_source_saved_after_the_copy_is_not_reused() {
        let dir = tempfile::tempdir().unwrap();
        let copy = dir.path().join("copie.carnet");
        let source = dir.path().join("ferme.carnet");
        std::fs::write(&copy, b"x").unwrap();
        std::fs::write(&source, b"x").unwrap();
        let t = std::time::SystemTime::now();
        std::fs::File::options().write(true).open(&copy).unwrap().set_modified(t).unwrap();
        let s = Some(source.to_string_lossy().into_owned());
        std::fs::File::options().write(true).open(&source).unwrap().set_modified(t - std::time::Duration::from_secs(60)).unwrap();
        assert_eq!(restore_source(s.clone(), &copy), s);
        std::fs::File::options().write(true).open(&source).unwrap().set_modified(t + std::time::Duration::from_secs(60)).unwrap();
        assert_eq!(restore_source(s, &copy), None);
        assert_eq!(restore_source(Some(dir.path().join("absent.carnet").to_string_lossy().into_owned()), &copy), None);
        assert_eq!(restore_source(None, &copy), None);
    }
}
