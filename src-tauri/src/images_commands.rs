//! La collection : les images de l'enseignant (dessins téléchargés, cliparts, images faites à la main),
//! gardées d'un document à l'autre dans `Documents/Cahutin/Images`, à côté des classes. Le dossier est la
//! liste : une image qu'on y dépose avec l'explorateur de fichiers apparaît aussi. Posée sur une page, une
//! image est copiée dans le document, qui reste complet si on l'envoie à une collègue. Les thèmes
//! (« Automne », « Animaux ») sont notés dans `themes.json`, à côté des images : les fichiers ne bougent pas.

use crate::state::AppState;
use carnet_core::{AssetMeta, Mime};
use carnet_core::photo::import_path;
use serde::Serialize;
use std::collections::BTreeMap;
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
    /// Thèmes où l'image est rangée, par ordre alphabétique.
    pub themes: Vec<String>,
}

const THEMES_FILE: &str = "themes.json";
/// Longueur maximale d'un nom de thème ou d'image.
const MAX_LABEL: usize = 80;

/// Thèmes de chaque image (nom de fichier → thèmes) ; vide si le fichier manque ou est illisible.
fn read_themes(dir: &Path) -> BTreeMap<String, Vec<String>> {
    std::fs::read(dir.join(THEMES_FILE)).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default()
}

fn write_themes(dir: &Path, themes: &BTreeMap<String, Vec<String>>) -> Result<(), String> {
    let kept: BTreeMap<_, _> = themes.iter().filter(|(_, t)| !t.is_empty()).collect();
    let json = serde_json::to_vec_pretty(&kept).map_err(|e| e.to_string())?;
    let tmp = dir.join(".themes.json.tmp");
    std::fs::write(&tmp, json).and_then(|_| std::fs::rename(&tmp, dir.join(THEMES_FILE))).map_err(|_| "Les thèmes n'ont pas pu être enregistrés.".to_string())
}

/// Un libellé propre (thème ou nom d'image) : sans espaces en trop, sans séparateur de dossier ni deux-points
/// (lecteur ou flux caché sous Windows), pas trop long.
fn clean_label(s: &str) -> Option<String> {
    let t: String = s.split_whitespace().collect::<Vec<_>>().join(" ");
    let ok = !t.is_empty() && !t.starts_with('.') && !t.contains(['/', '\\', ':']) && !t.chars().any(char::is_control);
    ok.then(|| t.chars().take(MAX_LABEL).collect())
}

pub fn images_dir(state: &AppState) -> PathBuf {
    state.classes_dir.parent().unwrap_or(&state.classes_dir).join("Images")
}

fn extension(name: &str) -> Option<String> {
    Path::new(name).extension().and_then(|e| e.to_str()).map(str::to_ascii_lowercase)
}

/// Un nom de fichier du dossier, sans chemin : pas de séparateur, de deux-points ni de « .. », une extension acceptée.
pub fn is_image_name(name: &str) -> bool {
    !name.is_empty() && !name.contains(['/', '\\', ':']) && !name.starts_with('.') && extension(name).is_some_and(|e| EXTENSIONS.contains(&e.as_str()))
}

/// Les images du dossier, par ordre alphabétique (sans tenir compte des majuscules).
pub fn list_in(dir: &Path) -> Vec<MyImage> {
    let Ok(entries) = std::fs::read_dir(dir) else { return Vec::new() };
    let themes = read_themes(dir);
    let mut out: Vec<MyImage> = entries
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().to_str()?.to_string();
            let meta = e.metadata().ok().filter(|m| m.is_file())?;
            let modified = meta.modified().ok().and_then(|t| t.duration_since(UNIX_EPOCH).ok()).map_or(0, |d| d.as_secs());
            is_image_name(&name).then(|| MyImage { themes: themes.get(&name).cloned().unwrap_or_default(), name, size: meta.len(), modified })
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
    std::fs::create_dir_all(dir).map_err(|_| "Le dossier de la collection ne peut pas être créé.".to_string())?;
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

/// Garde une image du document dans le dossier, sous un nom tiré de `name` (sans chemin) : rend `false` si
/// elle y est déjà, à l'octet près.
pub fn keep_in(dir: &Path, bytes: &[u8], mime: Mime, name: &str) -> Result<(Vec<MyImage>, bool), String> {
    std::fs::create_dir_all(dir).map_err(|_| "Le dossier de la collection ne peut pas être créé.".to_string())?;
    let same = list_in(dir).iter().any(|i| i.size == bytes.len() as u64 && std::fs::read(dir.join(&i.name)).is_ok_and(|b| b == bytes));
    if same {
        return Ok((list_in(dir), false));
    }
    let stem = Path::new(name.rsplit(['/', '\\']).next().unwrap_or("")).file_stem().and_then(|s| s.to_str()).map(str::trim).filter(|s| !s.is_empty() && !s.starts_with('.')).unwrap_or("image");
    let file = format!("{stem}.{}", if mime == Mime::Jpeg { "jpg" } else { "png" });
    std::fs::write(dir.join(free_name(dir, &file)), bytes).map_err(|_| "L'image n'a pas pu être gardée.".to_string())?;
    Ok((list_in(dir), true))
}

#[derive(Serialize, Clone, Debug, TS)]
#[ts(export)]
pub struct MyImageKept {
    pub images: Vec<MyImage>,
    /// Faux si l'image y était déjà.
    pub added: bool,
}

/// « Garder dans Mes images » : l'image `asset` du document rejoint le dossier.
#[tauri::command(async)]
pub fn my_images_keep(state: State<'_, AppState>, asset: String, name: String) -> Result<MyImageKept, String> {
    // Posée depuis « Mes images » (copie réencodée, autres octets que le fichier) : elle y est déjà.
    if state.my_images.lock().unwrap().values().any(|m| m.id == asset) {
        return Ok(MyImageKept { images: list_in(&images_dir(&state)), added: false });
    }
    let a = state.store.lock().unwrap().get(&asset).ok_or("Image introuvable.")?;
    keep_in(&images_dir(&state), &a.bytes, a.meta.mime, &name).map(|(images, added)| MyImageKept { images, added })
}

#[tauri::command]
pub fn my_images_list(state: State<'_, AppState>) -> Vec<MyImage> {
    list_in(&images_dir(&state))
}

#[tauri::command(async)]
pub fn my_images_add(state: State<'_, AppState>, paths: Vec<String>) -> Result<MyImagesAdded, String> {
    add_in(&images_dir(&state), &paths).map(|(images, refused)| MyImagesAdded { images, refused })
}

/// Sous-dossier des images retirées : rien n'est effacé, l'image peut y être reprise.
pub const REMOVED: &str = "Retirées";

/// Retire une image de la liste en la rangeant dans « Retirées » (les documents qui l'utilisent gardent leur
/// copie ; le fichier était peut-être le seul exemplaire).
pub fn remove_in(dir: &Path, name: &str) -> Result<Vec<MyImage>, String> {
    if !is_image_name(name) {
        return Err("Image inconnue.".into());
    }
    let removed = dir.join(REMOVED);
    std::fs::create_dir_all(&removed)
        .and_then(|_| std::fs::rename(dir.join(name), removed.join(free_name(&removed, name))))
        .map_err(|_| "L'image n'a pas pu être retirée.".to_string())?;
    let mut themes = read_themes(dir);
    if themes.remove(name).is_some() {
        write_themes(dir, &themes)?;
    }
    Ok(list_in(dir))
}

/// Les images retirées, qu'on peut remettre.
pub fn removed_in(dir: &Path) -> Vec<MyImage> {
    list_in(&dir.join(REMOVED))
}

/// Remet une image retirée dans la collection (sous un nom libre).
pub fn restore_in(dir: &Path, name: &str) -> Result<Vec<MyImage>, String> {
    if !is_image_name(name) {
        return Err("Image inconnue.".into());
    }
    std::fs::rename(dir.join(REMOVED).join(name), dir.join(free_name(dir, name))).map_err(|_| "L'image n'a pas pu être remise.".to_string())?;
    Ok(list_in(dir))
}

/// Renomme une image : le nouveau nom garde l'extension, et prend « (2) » s'il est déjà pris. Ses thèmes la
/// suivent.
pub fn rename_in(dir: &Path, name: &str, new_name: &str) -> Result<Vec<MyImage>, String> {
    let label = clean_label(new_name).ok_or("Ce nom ne convient pas.")?;
    if !is_image_name(name) || !dir.join(name).is_file() {
        return Err("Image inconnue.".into());
    }
    let ext = extension(name).unwrap_or_else(|| "png".into());
    let wanted = format!("{label}.{ext}");
    if wanted == name {
        return Ok(list_in(dir));
    }
    let target = if wanted.to_lowercase() == name.to_lowercase() { wanted } else { free_name(dir, &wanted) };
    std::fs::rename(dir.join(name), dir.join(&target)).map_err(|_| "L'image n'a pas pu être renommée.".to_string())?;
    let mut themes = read_themes(dir);
    if let Some(t) = themes.remove(name) {
        themes.insert(target, t);
        write_themes(dir, &themes)?;
    }
    Ok(list_in(dir))
}

/// Range des images dans un thème (`add`), ou les en sort (`remove`).
pub fn theme_in(dir: &Path, names: &[String], add: Option<&str>, remove: Option<&str>) -> Result<Vec<MyImage>, String> {
    let add = match add {
        Some(a) => Some(clean_label(a).ok_or("Ce nom de thème ne convient pas.")?),
        None => None,
    };
    let mut themes = read_themes(dir);
    for n in names.iter().filter(|n| is_image_name(n) && dir.join(n).is_file()) {
        let t = themes.entry(n.clone()).or_default();
        if let Some(r) = remove {
            t.retain(|x| x != r);
        }
        if let Some(a) = &add
            && !t.contains(a)
        {
            t.push(a.clone());
            t.sort_by_key(|x| x.to_lowercase());
        }
    }
    write_themes(dir, &themes)?;
    Ok(list_in(dir))
}

#[tauri::command]
pub fn my_images_removed(state: State<'_, AppState>) -> Vec<MyImage> {
    removed_in(&images_dir(&state))
}

#[tauri::command(async)]
pub fn my_images_restore(state: State<'_, AppState>, name: String) -> Result<Vec<MyImage>, String> {
    restore_in(&images_dir(&state), &name)
}

#[tauri::command(async)]
pub fn my_images_rename(state: State<'_, AppState>, name: String, new_name: String) -> Result<Vec<MyImage>, String> {
    rename_in(&images_dir(&state), &name, &new_name)
}

#[tauri::command(async)]
pub fn my_images_theme(state: State<'_, AppState>, names: Vec<String>, add: Option<String>, remove: Option<String>) -> Result<Vec<MyImage>, String> {
    theme_in(&images_dir(&state), &names, add.as_deref(), remove.as_deref())
}

#[tauri::command(async)]
pub fn my_images_remove(state: State<'_, AppState>, name: String) -> Result<Vec<MyImage>, String> {
    remove_in(&images_dir(&state), &name)
}

/// L'image prête à poser : importée dans les images de l'application (une seule fois tant qu'elle n'a pas
/// changé : dix clics ne la copient pas dix fois dans le document).
pub fn use_in(state: &AppState, name: &str) -> Result<AssetMeta, String> {
    let dir = images_dir(state);
    let image = list_in(&dir).into_iter().find(|i| i.name == name).ok_or("Cette image n'est plus dans la collection.")?;
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
        AppState::new(Default::default(), Library::open(dir.join("modeles")).unwrap(), dir.join("polices"), classes, dir.join("secours"))
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
        // Retirée deux fois sous le même nom : les deux fichiers sont gardés à part, rien n'est effacé.
        assert_eq!(remove_in(&dir, "vache.svg").unwrap().len(), 1);
        std::fs::rename(dir.join("vache (2).svg"), dir.join("vache.svg")).unwrap();
        assert!(remove_in(&dir, "vache.svg").unwrap().is_empty());
        assert!(dir.join(REMOVED).join("vache.svg").exists() && dir.join(REMOVED).join("vache (2).svg").exists());
        assert!(remove_in(&dir, "../x.svg").is_err());
    }

    #[test]
    fn a_kept_picture_is_named_after_its_file_and_kept_once() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("Images");
        let (list, added) = keep_in(&dir, b"png-bytes", Mime::Png, "/home/x/Téléchargements/vache.svg").unwrap();
        assert!(added);
        assert_eq!(list.iter().map(|i| i.name.as_str()).collect::<Vec<_>>(), ["vache.png"]);
        assert!(!keep_in(&dir, b"png-bytes", Mime::Png, "autre.png").unwrap().1, "déjà là");
        let (list, _) = keep_in(&dir, b"other", Mime::Jpeg, "").unwrap();
        assert!(list.iter().any(|i| i.name == "image.jpg"));
        let (list, _) = keep_in(&dir, b"third", Mime::Png, "vache.png").unwrap();
        assert!(list.iter().any(|i| i.name == "vache (2).png"));
    }

    #[test]
    fn themes_follow_renames_and_removals_and_removed_pictures_come_back() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("Images");
        add_in(&dir, &[svg(tmp.path(), "vache.svg"), svg(tmp.path(), "citrouille.svg")]).unwrap();
        let names = |l: &[MyImage]| l.iter().map(|i| (i.name.clone(), i.themes.clone())).collect::<Vec<_>>();
        let all = ["vache.svg".to_string(), "citrouille.svg".to_string()];
        theme_in(&dir, &all, Some("  Automne "), None).unwrap();
        let l = theme_in(&dir, &all[..1], Some("Animaux"), None).unwrap();
        assert_eq!(names(&l), [("citrouille.svg".into(), vec!["Automne".to_string()]), ("vache.svg".into(), vec!["Animaux".to_string(), "Automne".to_string()])]);
        // Renommée : l'extension reste, les thèmes suivent.
        let l = rename_in(&dir, "vache.svg", "La vache Marguerite").unwrap();
        assert!(l.iter().any(|i| i.name == "La vache Marguerite.svg" && i.themes.len() == 2));
        assert!(rename_in(&dir, "citrouille.svg", "../x").is_err());
        let l = rename_in(&dir, "citrouille.svg", "La vache Marguerite").unwrap();
        assert!(l.iter().any(|i| i.name == "La vache Marguerite (2).svg"));
        let l = theme_in(&dir, &["La vache Marguerite (2).svg".into()], None, Some("Automne")).unwrap();
        assert!(l.iter().find(|i| i.name == "La vache Marguerite (2).svg").unwrap().themes.is_empty());
        // Retirée puis remise.
        remove_in(&dir, "La vache Marguerite.svg").unwrap();
        assert_eq!(removed_in(&dir).len(), 1);
        let l = restore_in(&dir, "La vache Marguerite.svg").unwrap();
        assert_eq!(l.len(), 2);
        assert!(removed_in(&dir).is_empty());
        assert!(theme_in(&dir, &all, Some(" "), None).is_err());
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

    /// Un deux-points désigne un lecteur sous Windows (« C:x ») ou un flux caché de NTFS : refusé.
    #[test]
    fn a_colon_is_refused_in_image_names_and_labels() {
        assert!(!is_image_name("C:x.png"));
        assert!(is_image_name("chat.png"));
        assert_eq!(clean_label("C:piège"), None);
        assert_eq!(clean_label("  Animaux  "), Some("Animaux".into()));
    }
}
