//! « Mes images » : les images de l'enseignant (dessins téléchargés, cliparts, images faites à la main),
//! gardées d'un document à l'autre dans `Documents/Cahutin/Images`, à côté des classes. Le dossier est la
//! liste : une image qu'on y dépose avec l'explorateur de fichiers apparaît aussi. Posée sur une page, une
//! image est copiée dans le document, qui reste complet si on l'envoie à une collègue.

use crate::state::AppState;
use carnet_core::AssetMeta;
use carnet_core::photo::import_path;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;
use tauri::State;
use ts_rs::TS;

/// Fichiers acceptés dans « Mes images ».
const EXTENSIONS: [&str; 5] = ["png", "jpg", "jpeg", "webp", "svg"];

#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[ts(export)]
pub struct MyImage {
    /// Nom du fichier dans le dossier (sert d'identifiant).
    pub name: String,
    #[ts(type = "number")]
    pub size: u64,
    /// Date de modification, en secondes : une image remplacée sous le même nom n'est pas confondue.
    #[ts(type = "number")]
    pub modified: u64,
}

pub fn images_dir(state: &AppState) -> PathBuf {
    state.classes_dir.parent().unwrap_or(&state.classes_dir).join("Images")
}

fn extension(name: &str) -> Option<String> {
    Path::new(name).extension().and_then(|e| e.to_str()).map(str::to_ascii_lowercase)
}

/// Un nom de fichier du dossier, sans chemin : pas de séparateur ni de « .. », une extension acceptée.
pub fn is_image_name(name: &str) -> bool {
    !name.is_empty() && !name.contains(['/', '\\']) && !name.starts_with('.') && extension(name).is_some_and(|e| EXTENSIONS.contains(&e.as_str()))
}

/// Les images du dossier, par ordre alphabétique (sans tenir compte des majuscules).
pub fn list_in(dir: &Path) -> Vec<MyImage> {
    let Ok(entries) = std::fs::read_dir(dir) else { return Vec::new() };
    let mut out: Vec<MyImage> = entries
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().to_str()?.to_string();
            let meta = e.metadata().ok().filter(|m| m.is_file())?;
            is_image_name(&name).then(|| MyImage { name, size: meta.len(), modified: meta.modified().ok().and_then(|t| t.duration_since(UNIX_EPOCH).ok()).map_or(0, |d| d.as_secs()) })
        })
        .collect();
    out.sort_by_key(|i| i.name.to_lowercase());
    out
}

/// Un nom libre dans le dossier : le nom d'origine, sinon « nom (2).png », « nom (3).png »…
fn free_name(dir: &Path, name: &str) -> String {
    if !dir.join(name).exists() {
        return name.to_string();
    }
    let p = Path::new(name);
    let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or("image");
    let ext = p.extension().and_then(|s| s.to_str()).unwrap_or("png");
    (2..).map(|n| format!("{stem} ({n}).{ext}")).find(|n| !dir.join(n).exists()).unwrap()
}

/// Copie les fichiers choisis dans le dossier, sans jamais en écraser un ; ceux qui ne sont pas des images
/// sont laissés. Rend la liste à jour et le nombre de fichiers laissés.
pub fn add_in(dir: &Path, paths: &[String]) -> Result<(Vec<MyImage>, usize), String> {
    std::fs::create_dir_all(dir).map_err(|_| "Le dossier « Mes images » ne peut pas être créé.".to_string())?;
    let mut refused = 0;
    for p in paths.iter().map(Path::new) {
        let Some(name) = p.file_name().and_then(|n| n.to_str()).filter(|n| is_image_name(n)) else {
            refused += 1;
            continue;
        };
        if std::fs::copy(p, dir.join(free_name(dir, name))).is_err() {
            refused += 1;
        }
    }
    log::info!("my_images_add files={} refused={refused}", paths.len());
    Ok((list_in(dir), refused))
}

#[derive(Serialize, Clone, Debug, TS)]
#[ts(export)]
pub struct MyImagesAdded {
    pub images: Vec<MyImage>,
    /// Fichiers laissés (pas une image, ou illisibles).
    pub refused: usize,
}

#[tauri::command]
pub fn my_images_list(state: State<'_, AppState>) -> Vec<MyImage> {
    list_in(&images_dir(&state))
}

#[tauri::command(async)]
pub fn my_images_add(state: State<'_, AppState>, paths: Vec<String>) -> Result<MyImagesAdded, String> {
    add_in(&images_dir(&state), &paths).map(|(images, refused)| MyImagesAdded { images, refused })
}

/// Retire une image du dossier (les documents qui l'utilisent gardent leur copie).
#[tauri::command(async)]
pub fn my_images_remove(state: State<'_, AppState>, name: String) -> Result<Vec<MyImage>, String> {
    let dir = images_dir(&state);
    if !is_image_name(&name) {
        return Err("Image inconnue.".into());
    }
    std::fs::remove_file(dir.join(&name)).map_err(|_| "L'image n'a pas pu être retirée.".to_string())?;
    Ok(list_in(&dir))
}

/// L'image prête à poser : importée dans les images de l'application (une seule fois tant qu'elle n'a pas
/// changé : dix clics ne la copient pas dix fois dans le document).
pub fn use_in(state: &AppState, name: &str) -> Result<AssetMeta, String> {
    let dir = images_dir(state);
    let image = list_in(&dir).into_iter().find(|i| i.name == name).ok_or("Cette image n'est plus dans « Mes images ».")?;
    let key = (image.name.clone(), image.modified, image.size);
    if let Some(meta) = state.my_images.lock().unwrap().get(&key).cloned()
        && state.store.lock().unwrap().get(&meta.id).is_some()
    {
        return Ok(meta);
    }
    let asset = import_path(&dir.join(name)).map_err(|e| e.message_fr())?;
    let meta = asset.meta.clone();
    state.store.lock().unwrap().insert(asset).map_err(|e| e.message_fr())?;
    state.my_images.lock().unwrap().insert(key, meta.clone());
    Ok(meta)
}

#[tauri::command(async)]
pub fn my_images_use(state: State<'_, AppState>, name: String) -> Result<AssetMeta, String> {
    use_in(&state, &name)
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

    fn svg(dir: &Path, name: &str) -> String {
        let p = dir.join(name);
        std::fs::write(&p, r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 10"><rect width="20" height="10" fill="teal"/></svg>"#).unwrap();
        p.to_string_lossy().into_owned()
    }

    #[test]
    fn images_are_kept_beside_the_classes_never_overwritten_and_used_once() {
        let tmp = tempfile::tempdir().unwrap();
        let s = state(tmp.path());
        let dir = images_dir(&s);
        assert_eq!(dir, tmp.path().join("Cahutin").join("Images"));
        let notes = tmp.path().join("notes.txt");
        std::fs::write(&notes, b"x").unwrap();
        let files = [svg(tmp.path(), "vache.svg"), notes.to_string_lossy().into_owned()];
        let (list, refused) = add_in(&dir, &files).unwrap();
        assert_eq!((list.iter().map(|i| i.name.as_str()).collect::<Vec<_>>(), refused), (vec!["vache.svg"], 1));
        // Même nom une seconde fois : rien n'est écrasé.
        let (list, _) = add_in(&dir, &files[..1]).unwrap();
        assert_eq!(list.iter().map(|i| i.name.as_str()).collect::<Vec<_>>(), ["vache (2).svg", "vache.svg"]);
        let a = use_in(&s, "vache.svg").unwrap();
        assert_eq!((a.width_px, a.height_px), (2000, 1000));
        assert_eq!(use_in(&s, "vache.svg").unwrap(), a, "une seule copie");
        assert!(use_in(&s, "../vache.svg").is_err());
    }

    #[test]
    fn only_plain_image_names_are_accepted() {
        for ok in ["vache.png", "Citrouille d'automne.SVG", "a.jpeg"] {
            assert!(is_image_name(ok), "{ok}");
        }
        for bad in ["", "../x.png", "a/b.png", "a\\b.png", ".cache.png", "notes.txt", "photo"] {
            assert!(!is_image_name(bad), "{bad}");
        }
    }
}
