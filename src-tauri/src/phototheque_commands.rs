//! Commandes de la photothèque de la classe : un dossier par classe, à côté du dossier des classes
//! (`Documents/Cahutin/Photos/<nom du fichier de la classe>/`). Une seule photothèque ouverte à la fois,
//! celle de la classe courante. Le journal ne contient que des nombres.

use crate::state::AppState;
use carnet_core::AssetMeta;
use carnet_core::class::CLASS_EXTENSION;
use carnet_core::phototheque::{self, Phototheque, TagChange};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use tauri::State;
use tauri::ipc::Channel;
use ts_rs::TS;

const ADD_THREADS: usize = 4;

/// Dossier des photos d'une classe : seulement pour un fichier de classe du dossier des classes.
pub fn library_dir(classes_dir: &Path, class_path: &Path) -> Result<PathBuf, String> {
    let inside = class_path.parent().zip(classes_dir.canonicalize().ok()).is_some_and(|(p, d)| p.canonicalize().ok() == Some(d));
    let stem = class_path.file_stem().and_then(|s| s.to_str()).filter(|_| class_path.extension().and_then(|e| e.to_str()) == Some(CLASS_EXTENSION));
    match (inside, stem) {
        (true, Some(stem)) => Ok(classes_dir.parent().unwrap_or(classes_dir).join("Photos").join(stem)),
        _ => Err("Cette classe n'a pas de photothèque.".into()),
    }
}

fn with_open<T>(state: &AppState, f: impl FnOnce(&Path, &mut Phototheque) -> Result<T, String>) -> Result<T, String> {
    let mut lib = state.phototheque.lock().unwrap();
    let Some((dir, index)) = lib.as_mut() else { return Err("Aucune photothèque n'est ouverte.".into()) };
    f(dir, index)
}

pub fn open_in(state: &AppState, class_path: &Path) -> Result<Phototheque, String> {
    let dir = library_dir(&state.classes_dir, class_path)?;
    let index = phototheque::open(&dir)?;
    log::info!("phototheque_open photos={}", index.photos.len());
    *state.phototheque.lock().unwrap() = Some((dir, index.clone()));
    Ok(index)
}

#[tauri::command]
pub fn phototheque_open(state: State<'_, AppState>, class_path: String) -> Result<Phototheque, String> {
    open_in(&state, Path::new(&class_path))
}

#[derive(Serialize, Clone, Debug, TS)]
#[ts(export)]
#[serde(tag = "type")]
pub enum ClassPhotoAdded {
    Ok { photo: phototheque::ClassPhoto },
    Err { message: String },
}

/// Ajout d'un lot de photos, sur plusieurs fils : chaque photo prête entre dans l'index, enregistré
/// aussitôt, puis elle est annoncée. Une fermeture en cours d'import garde les photos déjà ajoutées.
pub fn add_in(state: &AppState, paths: &[String], on_each: impl Fn(ClassPhotoAdded) + Sync) -> Result<Phototheque, String> {
    let dir = with_open(state, |dir, _| Ok(dir.to_path_buf()))?;
    let files: Vec<PathBuf> = paths.iter().map(PathBuf::from).collect();
    let failures = AtomicUsize::new(0);
    let next = AtomicUsize::new(0);
    std::thread::scope(|s| {
        for _ in 0..ADD_THREADS.min(files.len()) {
            s.spawn(|| {
                while let Some(f) = files.get(next.fetch_add(1, Ordering::Relaxed)) {
                    match phototheque::add_file(&dir, f).map_err(|e| e.message_fr()).and_then(|p| record(state, &dir, p)) {
                        Ok(p) => on_each(ClassPhotoAdded::Ok { photo: p }),
                        Err(message) => {
                            failures.fetch_add(1, Ordering::Relaxed);
                            on_each(ClassPhotoAdded::Err { message });
                        }
                    }
                }
            });
        }
    });
    log::info!("phototheque_add total={} failures={}", files.len(), failures.into_inner());
    with_open(state, |_, index| Ok(index.clone()))
}

/// Une photo ajoutée entre dans l'index de son dossier, enregistré : celui qui est ouvert, ou celui du
/// disque si l'on a changé de classe pendant l'import.
fn record(state: &AppState, dir: &Path, photo: phototheque::ClassPhoto) -> Result<phototheque::ClassPhoto, String> {
    let mut lib = state.phototheque.lock().unwrap();
    match lib.as_mut() {
        Some((open, index)) if open == dir => {
            index.photos.push(photo.clone());
            phototheque::save(dir, index)?;
        }
        _ => {
            let mut index = phototheque::open(dir)?;
            index.photos.push(photo.clone());
            phototheque::save(dir, &index)?;
        }
    }
    Ok(photo)
}

#[tauri::command(async)]
pub fn phototheque_add(state: State<'_, AppState>, paths: Vec<String>, on_progress: Channel<ClassPhotoAdded>) -> Result<Phototheque, String> {
    add_in(&state, &paths, |o| {
        let _ = on_progress.send(o);
    })
}

#[tauri::command(async)]
pub fn phototheque_remove(state: State<'_, AppState>, ids: Vec<String>) -> Result<Phototheque, String> {
    with_open(&state, |dir, index| {
        // À la corbeille de l'ordinateur, comme un document ou une classe : une erreur se rattrape.
        let files = phototheque::files_to_dispose(dir, index, &ids);
        // Fichiers déjà tous disparus : rien à mettre à la corbeille, l'index se met à jour quand même.
        if !files.is_empty() {
            trash::delete_all(&files).map_err(|e| {
                log::error!("phototheque_trash_failed kind={}", crate::commands::trash_error_kind(&e));
                "Les photos n'ont pas pu être mises à la corbeille.".to_string()
            })?;
        }
        phototheque::remove(index, &ids);
        phototheque::save(dir, index)?;
        log::info!("phototheque_remove photos={}", ids.len());
        Ok(index.clone())
    })
}

#[tauri::command(async)]
pub fn phototheque_tag(state: State<'_, AppState>, ids: Vec<String>, change: TagChange) -> Result<Phototheque, String> {
    with_open(&state, |dir, index| {
        phototheque::tag(index, &ids, &change);
        phototheque::save(dir, index)?;
        Ok(index.clone())
    })
}

/// Les photos choisies, prêtes pour le document : elles rejoignent les images de l'application.
pub fn use_in(state: &AppState, ids: &[String]) -> Result<Vec<AssetMeta>, String> {
    let assets = with_open(state, |dir, index| {
        Ok(index.photos.iter().filter(|p| ids.contains(&p.id)).filter_map(|p| phototheque::load_asset(dir, p)).collect::<Vec<_>>())
    })?;
    let mut store = state.store.lock().unwrap();
    let mut metas = Vec::new();
    for a in assets {
        metas.push(a.meta.clone());
        store.insert(a).map_err(|e| e.message_fr())?;
    }
    log::info!("phototheque_use photos={}", metas.len());
    Ok(metas)
}

#[tauri::command(async)]
pub fn phototheque_use(state: State<'_, AppState>, ids: Vec<String>) -> Result<Vec<AssetMeta>, String> {
    use_in(&state, &ids)
}

#[cfg(test)]
mod tests {
    use super::*;
    use carnet_core::templates::Library;

    fn state(dir: &Path) -> AppState {
        let classes = dir.join("Cahutin").join("Classes");
        std::fs::create_dir_all(&classes).unwrap();
        AppState::new(Default::default(), Library::open(dir.join("modeles")).unwrap(), dir.join("polices"), classes)
    }

    fn jpeg(dir: &Path, name: &str) -> String {
        let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("../crates/carnet-core/tests/fixtures/date.jpg");
        let p = dir.join(name);
        std::fs::copy(fixture, &p).unwrap();
        p.to_string_lossy().into_owned()
    }

    #[test]
    fn a_class_gets_its_own_folder_of_photos() {
        let dir = tempfile::tempdir().unwrap();
        let s = state(dir.path());
        let class = s.classes_dir.join("MS 2026-2027.cahutin-classe");
        std::fs::write(&class, b"").unwrap();
        assert_eq!(library_dir(&s.classes_dir, &class).unwrap(), dir.path().join("Cahutin").join("Photos").join("MS 2026-2027"));
        // Ailleurs que dans le dossier des classes, ou un autre fichier : refusé.
        assert!(library_dir(&s.classes_dir, &dir.path().join("MS.cahutin-classe")).is_err());
        assert!(library_dir(&s.classes_dir, &s.classes_dir.join("notes.txt")).is_err());
    }

    #[test]
    fn photos_are_added_then_used_in_a_document() {
        let dir = tempfile::tempdir().unwrap();
        let s = state(dir.path());
        let class = s.classes_dir.join("MS.cahutin-classe");
        std::fs::write(&class, b"").unwrap();
        assert!(open_in(&s, &class).unwrap().photos.is_empty());
        let files = [jpeg(dir.path(), "a.jpg"), jpeg(dir.path(), "b.jpg"), dir.path().join("absent.jpg").to_string_lossy().into_owned()];
        let seen = AtomicUsize::new(0);
        let saved_dir = library_dir(&s.classes_dir, &class).unwrap();
        let lib = add_in(&s, &files, |o| {
            seen.fetch_add(1, Ordering::Relaxed);
            // Une photo annoncée est déjà dans l'index enregistré.
            if let ClassPhotoAdded::Ok { photo } = o {
                assert!(phototheque::open(&saved_dir).unwrap().photos.iter().any(|p| p.id == photo.id));
            }
        })
        .unwrap();
        assert_eq!((lib.photos.len(), seen.into_inner()), (2, 3));
        // Rouverte, la photothèque a gardé ses photos.
        assert_eq!(open_in(&s, &class).unwrap().photos.len(), 2);
        let metas = use_in(&s, &[lib.photos[0].id.clone()]).unwrap();
        assert_eq!(metas.len(), 1);
        assert!(s.store.lock().unwrap().get(&metas[0].id).is_some());
    }
}
