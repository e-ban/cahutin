//! Le cache de vignettes de la photothèque : `.vignettes/<id>-<taille>-<date>.jpg` dans le dossier de la
//! classe, 256 px au plus. Ce sont des fichiers de Cahutin, jamais ceux de l'enseignant ; le nom change
//! avec la taille ou la date du fichier, donc une photo modifiée a une nouvelle vignette.

use super::{ClassPhoto, Phototheque};
use crate::model::Mime;
use crate::photo::{Asset, ImportError};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

const THUMBS_DIR: &str = ".vignettes";

fn thumb_name(p: &ClassPhoto) -> String {
    format!("{}-{}-{}.jpg", p.id, p.size, p.modified)
}

/// La vignette de la photo (256 px, orientée), faite si elle manque ou si le fichier a changé.
pub fn thumb(library_dir: &Path, p: &ClassPhoto) -> Result<PathBuf, ImportError> {
    let dir = library_dir.join(THUMBS_DIR);
    let path = dir.join(thumb_name(p));
    if path.is_file() {
        return Ok(path);
    }
    let (thumb, mime) = crate::photo::thumbnail_of(Path::new(&p.path))?;
    let bytes = if mime == Mime::Jpeg {
        thumb
    } else {
        // Une photo transparente a une vignette PNG ; le cache ne garde que des JPEG.
        let img = image::load_from_memory(&thumb).map_err(|_| ImportError::Decode)?;
        crate::photo::encode(&img, Mime::Jpeg)?
    };
    std::fs::create_dir_all(&dir).map_err(|_| ImportError::Storage)?;
    // Fichier temporaire propre à cet appel : deux appels en même temps pour la même photo n'écrivent
    // jamais le même fichier, et le dernier renommage pose un fichier entier.
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let tmp = dir.join(format!("{}.{}-{}.tmp", thumb_name(p), std::process::id(), COUNTER.fetch_add(1, Ordering::Relaxed)));
    let written = std::fs::write(&tmp, &bytes).and_then(|_| std::fs::rename(&tmp, &path));
    if written.is_err() {
        let _ = std::fs::remove_file(&tmp);
        return Err(ImportError::Storage);
    }
    Ok(path)
}

/// Les vignettes qui n'existent plus pour aucune photo : à effacer (fichiers du cache, pas de l'enseignant).
pub fn stale_thumbs(library_dir: &Path, lib: &Phototheque) -> Vec<PathBuf> {
    let current: HashSet<String> = lib.photos.iter().map(thumb_name).collect();
    let Ok(entries) = std::fs::read_dir(library_dir.join(THUMBS_DIR)) else { return vec![] };
    let mut stale: Vec<PathBuf> = entries
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.is_file() && !p.file_name().and_then(|n| n.to_str()).is_some_and(|n| current.contains(n)))
        .collect();
    stale.sort();
    stale
}

/// La photo pour un document : l'original importé comme une photo du tiroir, avec l'identifiant de la photothèque.
pub fn load_asset(p: &ClassPhoto) -> Result<Asset, ImportError> {
    let mut asset = crate::photo::import_path(Path::new(&p.path))?;
    // L'identifiant reste celui de la photothèque : l'application s'en sert pour le consentement des enfants.
    asset.meta.id = p.id.clone();
    asset.meta.taken = p.taken.clone();
    Ok(asset)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Mime;
    use crate::phototheque::{FORMAT_VERSION, follow, scan};
    use image::{ImageFormat, Rgb, RgbImage};
    use std::io::Cursor;

    fn jpeg(path: &Path, w: u32, h: u32, shade: u8) {
        let mut out = Cursor::new(Vec::new());
        RgbImage::from_pixel(w, h, Rgb([shade, 120, 200])).write_to(&mut out, ImageFormat::Jpeg).unwrap();
        std::fs::write(path, out.into_inner()).unwrap();
    }

    /// Une classe, un dossier suivi avec une photo 800 x 600, et l'index parcouru.
    fn setup() -> (tempfile::TempDir, PathBuf, PathBuf, Phototheque) {
        let tmp = tempfile::tempdir().unwrap();
        let library = tmp.path().join("Classe");
        let folder = tmp.path().join("Sortie");
        std::fs::create_dir_all(&library).unwrap();
        std::fs::create_dir_all(&folder).unwrap();
        jpeg(&folder.join("a.jpg"), 800, 600, 10);
        let mut lib = Phototheque { format_version: FORMAT_VERSION, folders: vec![], photos: vec![] };
        follow(&mut lib, &folder).unwrap();
        scan(&mut lib, &library, &mut |_| {});
        (tmp, library, folder, lib)
    }

    fn dims(path: &Path) -> (u32, u32) {
        let img = image::open(path).unwrap();
        (img.width(), img.height())
    }

    #[test]
    fn a_thumbnail_is_made_once_and_named_after_the_file() {
        let (_tmp, library, _folder, lib) = setup();
        let p = &lib.photos[0];
        let t = thumb(&library, p).unwrap();
        assert_eq!(t, library.join(".vignettes").join(format!("{}-{}-{}.jpg", p.id, p.size, p.modified)));
        assert_eq!(dims(&t), (256, 192));
        let before = std::fs::metadata(&t).unwrap().modified().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(30));
        assert_eq!(thumb(&library, p).unwrap(), t);
        assert_eq!(std::fs::metadata(&t).unwrap().modified().unwrap(), before, "pas réécrite");
    }

    #[test]
    fn a_changed_file_gets_a_new_thumbnail_and_the_old_one_is_stale() {
        let (_tmp, library, folder, mut lib) = setup();
        let old = thumb(&library, &lib.photos[0]).unwrap();
        assert!(stale_thumbs(&library, &lib).is_empty());
        jpeg(&folder.join("a.jpg"), 600, 800, 90);
        let when = std::time::SystemTime::now() + std::time::Duration::from_secs(60);
        std::fs::File::options().write(true).open(folder.join("a.jpg")).unwrap().set_modified(when).unwrap();
        scan(&mut lib, &library, &mut |_| {});
        let new = thumb(&library, &lib.photos[0]).unwrap();
        assert_ne!(new, old);
        assert_eq!(dims(&new), (192, 256));
        assert_eq!(stale_thumbs(&library, &lib), vec![old.clone()]);
        assert!(old.exists(), "rien n'est effacé");
    }

    #[test]
    fn thumbnails_of_removed_photos_are_stale_and_other_files_are_left_alone() {
        let (_tmp, library, _folder, mut lib) = setup();
        let t = thumb(&library, &lib.photos[0]).unwrap();
        lib.photos.clear();
        assert_eq!(stale_thumbs(&library, &lib), vec![t]);
        assert!(stale_thumbs(&library.join("absent"), &lib).is_empty());
    }

    #[test]
    fn two_threads_making_the_same_thumbnail_both_get_a_whole_file() {
        let (_tmp, library, _folder, lib) = setup();
        let p = &lib.photos[0];
        let paths: Vec<PathBuf> = std::thread::scope(|s| {
            let hs: Vec<_> = (0..4).map(|_| s.spawn(|| thumb(&library, p).unwrap())).collect();
            hs.into_iter().map(|h| h.join().unwrap()).collect()
        });
        assert!(paths.iter().all(|t| *t == paths[0]));
        assert_eq!(dims(&paths[0]), (256, 192));
        assert!(stale_thumbs(&library, &lib).is_empty(), "aucun fichier temporaire ne reste");
    }

    #[test]
    fn leftover_temporary_files_are_listed_as_stale() {
        let (_tmp, library, _folder, lib) = setup();
        thumb(&library, &lib.photos[0]).unwrap();
        let leftover = library.join(".vignettes").join("x-1-2.jpg.99-0.tmp");
        std::fs::write(&leftover, b"x").unwrap();
        assert_eq!(stale_thumbs(&library, &lib), vec![leftover]);
    }

    #[test]
    fn a_missing_original_is_an_error() {
        let (_tmp, library, folder, lib) = setup();
        std::fs::remove_file(folder.join("a.jpg")).unwrap();
        assert_eq!(thumb(&library, &lib.photos[0]), Err(ImportError::Io));
    }

    #[test]
    fn the_asset_keeps_the_library_id_and_has_no_metadata() {
        let (_tmp, _library, _folder, mut lib) = setup();
        lib.photos[0].taken = Some("2026-10-14".into());
        let p = &lib.photos[0];
        let a = load_asset(p).unwrap();
        assert_eq!(a.meta.id, p.id);
        assert_eq!(a.meta.taken.as_deref(), Some("2026-10-14"));
        assert_eq!(a.meta.mime, Mime::Jpeg);
        let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/date.jpg");
        let mut q = p.clone();
        q.path = fixture.to_string_lossy().into_owned();
        let b = load_asset(&q).unwrap();
        assert!(exif::Reader::new().read_from_container(&mut Cursor::new(&*b.bytes)).is_err(), "sans EXIF");
    }
}
