//! Le parcours des dossiers suivis : met l'index à jour d'après le disque, sans rien copier ni toucher.

use super::{ClassPhoto, INDEX_FILE, Phototheque, iso, taken_of_file};
use crate::class;
use crate::model::{Mime, new_id};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

/// Nombre de photos envoyées à la fois à `on_batch`.
const BATCH_SIZE: usize = 50;

#[derive(Debug, Default, PartialEq)]
pub struct ScanReport {
    /// Photos nouvelles.
    pub added: usize,
    /// Photos retrouvées ailleurs (renommées ou déplacées).
    pub moved: usize,
    /// Photos dont le fichier manque après le parcours.
    pub missing: usize,
    /// Dossiers suivis introuvables.
    pub missing_folders: Vec<String>,
}

/// Un fichier image trouvé sur le disque.
struct Found {
    path: PathBuf,
    key: String,
    folder: String,
    mime: Mime,
    size: u64,
    modified: u64,
}

fn mime_of(path: &Path) -> Option<Mime> {
    match path.extension()?.to_str()?.to_ascii_lowercase().as_str() {
        // Un WebP compte comme un JPEG : c'est le format qu'il prend une fois posé dans un document (une photo
        // n'a pas de transparence). La route qui sert l'original donne le type du contenu d'après l'extension.
        "jpg" | "jpeg" | "webp" => Some(Mime::Jpeg),
        "png" => Some(Mime::Png),
        _ => None,
    }
}

/// Les images d'un dossier suivi et de ses sous-dossiers, triées par chemin. Les dossiers cachés
/// (nom commençant par « . »), `phototheque.json` et les `*.vignette.*` sont sautés.
fn walk(root: &Path, out: &mut Vec<Found>) {
    let root_name = root.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let mut stack = vec![root.to_path_buf()];
    let mut found = Vec::new();
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else { continue };
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().into_owned();
            let Ok(kind) = entry.file_type() else { continue };
            if kind.is_dir() {
                if !name.starts_with('.') {
                    stack.push(path);
                }
                continue;
            }
            if name == INDEX_FILE || name.contains(".vignette.") {
                continue;
            }
            let Some(mime) = mime_of(&path) else { continue };
            let Ok(meta) = std::fs::metadata(&path) else { continue };
            if !meta.is_file() {
                continue;
            }
            let modified = meta.modified().ok().and_then(|t| t.duration_since(UNIX_EPOCH).ok()).map_or(0, |d| d.as_secs());
            let rel: Vec<String> = path.strip_prefix(root).ok().and_then(Path::parent).map(|p| p.components().map(|c| c.as_os_str().to_string_lossy().into_owned()).collect()).unwrap_or_default();
            let folder = if rel.is_empty() { root_name.clone() } else { rel.join("/") };
            found.push(Found { key: path.to_string_lossy().into_owned(), path, folder, mime, size: meta.len(), modified });
        }
    }
    found.sort_by(|a, b| a.path.cmp(&b.path));
    out.extend(found);
}

/// Envoie les photos par paquets de `BATCH_SIZE`.
struct Batcher<'a> {
    buffer: Vec<ClassPhoto>,
    on_batch: &'a mut dyn FnMut(&[ClassPhoto]),
}

impl Batcher<'_> {
    fn push(&mut self, photo: &ClassPhoto) {
        self.buffer.push(photo.clone());
        if self.buffer.len() == BATCH_SIZE {
            self.flush();
        }
    }

    fn flush(&mut self) {
        if !self.buffer.is_empty() {
            (self.on_batch)(&self.buffer);
            self.buffer.clear();
        }
    }
}

/// Met l'index à jour d'après le disque. `on_batch` reçoit les photos nouvelles, retrouvées ailleurs,
/// changées ou de retour, par paquets de 50. `_library_dir` (le dossier de la classe) est déjà écarté
/// par la règle des dossiers cachés (`.vignettes`) et celle de `phototheque.json`.
pub fn scan(lib: &mut Phototheque, _library_dir: &Path, on_batch: &mut dyn FnMut(&[ClassPhoto])) -> ScanReport {
    let mut report = ScanReport::default();
    let mut files = Vec::new();
    for folder in &mut lib.folders {
        let root = Path::new(&folder.path);
        folder.missing = !root.is_dir();
        if folder.missing {
            report.missing_folders.push(folder.path.clone());
        } else {
            walk(root, &mut files);
        }
    }
    let mut seen_paths = HashSet::new();
    files.retain(|f| seen_paths.insert(f.key.clone()));

    let mut batcher = Batcher { buffer: Vec::new(), on_batch };
    let index_of: HashMap<String, usize> = lib.photos.iter().enumerate().map(|(i, p)| (p.path.clone(), i)).collect();

    // Chemins connus : relus seulement si la taille ou la date ont changé.
    let mut unknown = Vec::new();
    for f in files {
        let Some(&i) = index_of.get(&f.key) else {
            unknown.push(f);
            continue;
        };
        let p = &mut lib.photos[i];
        let changed = p.size != f.size || p.modified != f.modified;
        if !changed && !p.missing {
            continue;
        }
        if changed {
            let (Some(fp), Ok((w, h))) = (super::fingerprint(&f.path), image::image_dimensions(&f.path)) else { continue };
            (p.width_px, p.height_px, p.size, p.modified, p.fingerprint) = (w, h, f.size, f.modified, fp);
        }
        p.missing = false;
        batcher.push(p);
    }

    // Chemins inconnus : une photo devenue absente de même empreinte prend sa place, sinon c'est une nouvelle photo.
    let mut vanished: Vec<usize> = (0..lib.photos.len()).filter(|&i| !seen_paths.contains(&lib.photos[i].path)).collect();
    for f in unknown {
        let Some(fp) = super::fingerprint(&f.path) else { continue };
        if let Some(pos) = vanished.iter().position(|&i| lib.photos[i].fingerprint == fp) {
            let p = &mut lib.photos[vanished.remove(pos)];
            (p.path, p.folder, p.size, p.modified, p.missing) = (f.key, f.folder, f.size, f.modified, false);
            batcher.push(p);
            report.moved += 1;
            continue;
        }
        let Ok((width_px, height_px)) = image::image_dimensions(&f.path) else { continue };
        let photo = ClassPhoto {
            id: new_id(),
            width_px,
            height_px,
            mime: f.mime,
            taken: taken_of_file(&f.path),
            added: iso(class::today()),
            students: vec![],
            tags: vec![],
            path: f.key,
            folder: f.folder,
            size: f.size,
            modified: f.modified,
            fingerprint: fp,
            hidden: false,
            missing: false,
        };
        batcher.push(&photo);
        lib.photos.push(photo);
        report.added += 1;
    }
    batcher.flush();

    for i in vanished {
        lib.photos[i].missing = true;
    }
    report.missing = lib.photos.iter().filter(|p| p.missing).count();
    report
}

/// Suit un dossier. Refusé s'il est déjà suivi, contenu dans un dossier suivi, ou s'il en contient un
/// (les photos seraient comptées deux fois).
pub fn follow(lib: &mut Phototheque, folder: &Path) -> Result<(), String> {
    if !folder.is_dir() {
        return Err("Ce dossier est introuvable.".into());
    }
    for f in &lib.folders {
        let followed = Path::new(&f.path);
        if followed == folder {
            return Err("Ce dossier est déjà suivi.".into());
        }
        if folder.starts_with(followed) {
            return Err("Ce dossier est déjà dans un dossier suivi.".into());
        }
        if followed.starts_with(folder) {
            return Err("Ce dossier contient un dossier déjà suivi.".into());
        }
    }
    lib.folders.push(super::FollowedFolder { path: folder.to_string_lossy().into_owned(), missing: false });
    Ok(())
}

/// Oublie le dossier et ses photos : aucun fichier n'est touché.
pub fn unfollow(lib: &mut Phototheque, folder: &str) {
    let root = Path::new(folder);
    lib.folders.retain(|f| f.path != folder);
    lib.photos.retain(|p| !Path::new(&p.path).starts_with(root));
}

#[cfg(test)]
mod tests {
    use crate::model::Mime;
    use super::super::{FORMAT_VERSION, INDEX_FILE, TagChange, hide, tag};
    use super::*;
    use image::{ImageFormat, Rgb, RgbImage};
    use std::io::Cursor;

    fn jpeg(path: &Path, shade: u8) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let mut out = Cursor::new(Vec::new());
        RgbImage::from_pixel(40, 30, Rgb([shade, 120, 200])).write_to(&mut out, ImageFormat::Jpeg).unwrap();
        std::fs::write(path, out.into_inner()).unwrap();
    }

    fn empty() -> Phototheque {
        Phototheque { format_version: FORMAT_VERSION, folders: vec![], photos: vec![] }
    }

    fn run(lib: &mut Phototheque, library: &Path) -> (ScanReport, Vec<usize>) {
        let mut batches = vec![];
        let report = scan(lib, library, &mut |b| batches.push(b.len()));
        (report, batches)
    }

    fn by_name<'a>(lib: &'a Phototheque, name: &str) -> &'a ClassPhoto {
        lib.photos.iter().find(|p| Path::new(&p.path).file_name().unwrap() == name).unwrap()
    }

    fn tag_all(lib: &mut Phototheque, id: &str, t: &str) {
        tag(lib, &[id.to_string()], &TagChange { add_tags: vec![t.into()], ..Default::default() });
    }

    /// Une bibliothèque de classe et un dossier suivi `Sortie` à côté.
    fn setup() -> (tempfile::TempDir, std::path::PathBuf, std::path::PathBuf, Phototheque) {
        let tmp = tempfile::tempdir().unwrap();
        let library = tmp.path().join("Photos").join("Classe");
        std::fs::create_dir_all(&library).unwrap();
        let folder = tmp.path().join("Sortie");
        std::fs::create_dir_all(&folder).unwrap();
        let mut lib = empty();
        follow(&mut lib, &folder).unwrap();
        (tmp, library, folder, lib)
    }

    #[test]
    fn first_scan_finds_photos_and_skips_the_rest() {
        let (_tmp, library, folder, mut lib) = setup();
        jpeg(&folder.join("a.jpg"), 10);
        jpeg(&folder.join("ferme/b.JPG"), 20);
        jpeg(&folder.join("ferme/b.vignette.jpg"), 30);
        jpeg(&folder.join(".vignettes/c.jpg"), 40);
        jpeg(&folder.join(".cache/d.jpg"), 50);
        std::fs::write(folder.join(INDEX_FILE), b"{}").unwrap();
        std::fs::write(folder.join("notes.txt"), b"x").unwrap();
        std::fs::write(folder.join("faux.jpg"), b"pas une image").unwrap();
        let (report, batches) = run(&mut lib, &library);
        assert_eq!(report, ScanReport { added: 2, moved: 0, missing: 0, missing_folders: vec![] });
        assert_eq!(batches, [2]);
        assert_eq!(lib.photos.len(), 2);
        assert_eq!(by_name(&lib, "a.jpg").folder, "Sortie");
        assert_eq!(by_name(&lib, "b.JPG").folder, "ferme");
        let a = by_name(&lib, "a.jpg");
        assert_eq!((a.width_px, a.height_px), (40, 30));
        assert!(a.taken.is_some() && !a.added.is_empty() && a.size > 0 && a.modified > 0 && !a.fingerprint.is_empty());
    }

    #[test]
    fn batches_hold_fifty_photos() {
        let (_tmp, library, folder, mut lib) = setup();
        for i in 0..120 {
            jpeg(&folder.join(format!("p{i}.jpg")), i as u8);
        }
        let (report, batches) = run(&mut lib, &library);
        assert_eq!(report.added, 120);
        assert_eq!(batches, [50, 50, 20]);
    }

    #[test]
    fn a_second_scan_without_change_sends_nothing() {
        let (_tmp, library, folder, mut lib) = setup();
        jpeg(&folder.join("a.jpg"), 10);
        run(&mut lib, &library);
        let before = lib.clone();
        let (report, batches) = run(&mut lib, &library);
        assert_eq!(report, ScanReport::default());
        assert!(batches.is_empty());
        assert_eq!(lib, before);
    }

    #[test]
    fn a_renamed_then_moved_file_keeps_its_tags() {
        let (_tmp, library, folder, mut lib) = setup();
        jpeg(&folder.join("a.jpg"), 10);
        jpeg(&folder.join("b.jpg"), 90);
        run(&mut lib, &library);
        let id = by_name(&lib, "a.jpg").id.clone();
        tag_all(&mut lib, &id, "Sortie à la ferme");
        std::fs::rename(folder.join("a.jpg"), folder.join("renommee.jpg")).unwrap();
        let (report, batches) = run(&mut lib, &library);
        assert_eq!((report.added, report.moved, report.missing), (0, 1, 0));
        assert_eq!(batches, [1]);
        assert_eq!(lib.photos.len(), 2);
        let p = by_name(&lib, "renommee.jpg");
        assert_eq!((p.id.as_str(), p.tags.as_slice()), (id.as_str(), ["Sortie à la ferme".to_string()].as_slice()));
        std::fs::create_dir_all(folder.join("sous")).unwrap();
        std::fs::rename(folder.join("renommee.jpg"), folder.join("sous/renommee.jpg")).unwrap();
        let (report, _) = run(&mut lib, &library);
        assert_eq!((report.added, report.moved), (0, 1));
        let p = by_name(&lib, "renommee.jpg");
        assert_eq!((p.id.as_str(), p.folder.as_str(), p.missing), (id.as_str(), "sous", false));
        assert_eq!(p.path, folder.join("sous/renommee.jpg").to_string_lossy());
        assert_eq!(p.tags, ["Sortie à la ferme"]);
    }

    #[test]
    fn identical_files_stay_two_photos() {
        let (_tmp, library, folder, mut lib) = setup();
        jpeg(&folder.join("a.jpg"), 10);
        std::fs::copy(folder.join("a.jpg"), folder.join("copie.jpg")).unwrap();
        run(&mut lib, &library);
        assert_eq!(lib.photos.len(), 2);
        let (ida, idc) = (by_name(&lib, "a.jpg").id.clone(), by_name(&lib, "copie.jpg").id.clone());
        tag_all(&mut lib, &ida, "A");
        tag_all(&mut lib, &idc, "Copie");
        std::fs::create_dir_all(folder.join("sous")).unwrap();
        std::fs::rename(folder.join("copie.jpg"), folder.join("sous/copie.jpg")).unwrap();
        let (report, _) = run(&mut lib, &library);
        assert_eq!((report.added, report.moved), (0, 1));
        assert_eq!(lib.photos.len(), 2);
        assert_eq!(by_name(&lib, "a.jpg").id, ida);
        assert_eq!(by_name(&lib, "a.jpg").tags, ["A"]);
        let c = by_name(&lib, "copie.jpg");
        assert_eq!((c.id.as_str(), c.tags.as_slice()), (idc.as_str(), ["Copie".to_string()].as_slice()));
    }

    #[test]
    fn a_deleted_file_is_missing_and_comes_back() {
        let (_tmp, library, folder, mut lib) = setup();
        jpeg(&folder.join("a.jpg"), 10);
        run(&mut lib, &library);
        let id = by_name(&lib, "a.jpg").id.clone();
        tag_all(&mut lib, &id, "Motricité");
        let bytes = std::fs::read(folder.join("a.jpg")).unwrap();
        std::fs::remove_file(folder.join("a.jpg")).unwrap();
        let (report, _) = run(&mut lib, &library);
        assert_eq!((report.added, report.missing), (0, 1));
        assert!(lib.photos[0].missing);
        assert_eq!(lib.photos[0].tags, ["Motricité"]);
        std::fs::write(folder.join("a.jpg"), bytes).unwrap();
        let (report, batches) = run(&mut lib, &library);
        assert_eq!((report.added, report.moved, report.missing), (0, 0, 0));
        assert_eq!(batches, [1]);
        assert_eq!((lib.photos.len(), lib.photos[0].id.as_str(), lib.photos[0].missing), (1, id.as_str(), false));
        assert_eq!(lib.photos[0].tags, ["Motricité"]);
    }

    #[test]
    fn a_missing_folder_keeps_everything() {
        let (tmp, library, folder, mut lib) = setup();
        jpeg(&folder.join("a.jpg"), 10);
        run(&mut lib, &library);
        let id = by_name(&lib, "a.jpg").id.clone();
        tag_all(&mut lib, &id, "Motricité");
        let away = tmp.path().join("Sortie-absente");
        std::fs::rename(&folder, &away).unwrap();
        let (report, _) = run(&mut lib, &library);
        assert_eq!(report.missing_folders, [folder.to_string_lossy().into_owned()]);
        assert_eq!((report.added, report.missing), (0, 1));
        assert!(lib.folders[0].missing && lib.photos[0].missing);
        assert_eq!(lib.photos[0].tags, ["Motricité"]);
        std::fs::rename(&away, &folder).unwrap();
        let (report, _) = run(&mut lib, &library);
        assert_eq!((report.missing, report.missing_folders.len()), (0, 0));
        assert!(!lib.folders[0].missing && !lib.photos[0].missing);
    }

    #[test]
    fn follow_refuses_a_folder_already_followed_or_inside_one() {
        let (_tmp, _library, folder, mut lib) = setup();
        std::fs::create_dir_all(folder.join("sous")).unwrap();
        assert!(follow(&mut lib, &folder).is_err());
        assert!(follow(&mut lib, &folder.join("sous")).is_err());
        assert_eq!(lib.folders.len(), 1);
    }

    #[test]
    fn unfollow_forgets_the_folder_and_its_photos() {
        let (tmp, library, folder, mut lib) = setup();
        let autre = tmp.path().join("Autre");
        jpeg(&folder.join("a.jpg"), 10);
        jpeg(&autre.join("b.jpg"), 60);
        follow(&mut lib, &autre).unwrap();
        run(&mut lib, &library);
        assert_eq!(lib.photos.len(), 2);
        unfollow(&mut lib, &folder.to_string_lossy());
        assert_eq!(lib.folders.len(), 1);
        assert_eq!(lib.photos.len(), 1);
        assert_eq!(lib.photos[0].folder, "Autre");
    }

    #[test]
    fn a_hidden_photo_stays_hidden_after_a_scan() {
        let (_tmp, library, folder, mut lib) = setup();
        jpeg(&folder.join("a.jpg"), 10);
        run(&mut lib, &library);
        let id = lib.photos[0].id.clone();
        hide(&mut lib, &[id], true);
        let (report, batches) = run(&mut lib, &library);
        assert_eq!(report, ScanReport::default());
        assert!(batches.is_empty());
        assert_eq!(lib.photos.len(), 1);
        assert!(lib.photos[0].hidden);
    }

    #[test]
    fn a_webp_file_is_found_as_a_jpeg() {
        let (_tmp, library, folder, mut lib) = setup();
        RgbImage::from_pixel(40, 30, Rgb([10, 120, 200])).save(folder.join("a.WEBP")).unwrap();
        let (report, _) = run(&mut lib, &library);
        assert_eq!(report.added, 1);
        let p = &lib.photos[0];
        assert_eq!((p.mime, p.width_px, p.height_px), (Mime::Jpeg, 40, 30));
    }

    #[test]
    fn a_changed_file_is_read_again() {
        let (_tmp, library, folder, mut lib) = setup();
        jpeg(&folder.join("a.jpg"), 10);
        run(&mut lib, &library);
        let id = lib.photos[0].id.clone();
        let mut out = Cursor::new(Vec::new());
        RgbImage::from_pixel(80, 60, Rgb([200, 20, 20])).write_to(&mut out, ImageFormat::Jpeg).unwrap();
        std::fs::write(folder.join("a.jpg"), out.into_inner()).unwrap();
        let (report, batches) = run(&mut lib, &library);
        assert_eq!((report.added, report.moved), (0, 0));
        assert_eq!(batches, [1]);
        assert_eq!((lib.photos[0].id.as_str(), lib.photos[0].width_px, lib.photos[0].height_px), (id.as_str(), 80, 60));
    }
}
