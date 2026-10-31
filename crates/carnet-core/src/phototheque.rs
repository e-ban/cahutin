//! Photothèque de la classe : les photos de l'année, dans un dossier visible que l'enseignant peut
//! sauvegarder (`Documents/Cahutin/Photos/<classe>/`), avec un index `phototheque.json` : pour chaque
//! photo, la date de prise de vue, les élèves présents et les activités. Les photos y sont rangées
//! comme à l'import dans un document : réduites, orientées, sans métadonnées (ni lieu ni appareil).
//! Aucun visage n'est reconnu : les élèves présents sont choisis à la main.

use crate::class;
use crate::fsutil::write_atomically;
use crate::model::{AssetMeta, Id, Mime, is_safe_id};
use crate::photo::{self, Asset, ImportError};
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use ts_rs::TS;

pub const INDEX_FILE: &str = "phototheque.json";
pub const FORMAT_VERSION: u32 = 1;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, TS)]
#[ts(export)]
pub struct ClassPhoto {
    pub id: Id,
    pub width_px: u32,
    pub height_px: u32,
    pub mime: Mime,
    /// Jour de la prise de vue (« 2026-10-14 ») : celui de l'appareil, sinon celui du fichier.
    pub taken: Option<String>,
    /// Jour de l'ajout à la photothèque.
    pub added: String,
    /// Élèves présents sur la photo (identifiants de la classe).
    pub students: Vec<Id>,
    /// Activités, sorties, domaines : « Sortie à la ferme », « Motricité ».
    pub tags: Vec<String>,
}

impl ClassPhoto {
    pub fn meta(&self) -> AssetMeta {
        AssetMeta { id: self.id.clone(), width_px: self.width_px, height_px: self.height_px, mime: self.mime }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default, TS)]
#[ts(export)]
pub struct Phototheque {
    pub format_version: u32,
    pub photos: Vec<ClassPhoto>,
}

fn ext(mime: Mime) -> &'static str {
    match mime {
        Mime::Jpeg => "jpg",
        Mime::Png => "png",
    }
}

/// Fichier de la photo (ou de sa vignette) dans le dossier ; jamais en dehors.
pub fn file_of(dir: &Path, id: &str, mime: Mime, thumb: bool) -> Option<PathBuf> {
    is_safe_id(id).then(|| dir.join(format!("{id}{}.{}", if thumb { ".vignette" } else { "" }, ext(mime))))
}

fn iso((y, m, d): (i32, u32, u32)) -> String {
    format!("{y:04}-{m:02}-{d:02}")
}

/// La photothèque du dossier ; vide s'il n'y en a pas encore. Un index plus récent que l'application est
/// refusé plutôt qu'écrasé.
pub fn open(dir: &Path) -> Result<Phototheque, String> {
    let path = dir.join(INDEX_FILE);
    let Ok(bytes) = std::fs::read(&path) else { return Ok(Phototheque { format_version: FORMAT_VERSION, photos: vec![] }) };
    let lib: Phototheque = serde_json::from_slice(&bytes).map_err(|_| "La photothèque de cette classe est illisible.".to_string())?;
    if lib.format_version > FORMAT_VERSION {
        return Err("Cette photothèque a été faite avec une version plus récente de Cahutin.".into());
    }
    Ok(lib)
}

pub fn save(dir: &Path, lib: &Phototheque) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|_| "Le dossier des photos ne peut pas être créé.".to_string())?;
    let json = serde_json::to_vec_pretty(lib).map_err(|_| "La photothèque ne peut pas être enregistrée.".to_string())?;
    write_atomically::<std::io::Error>(&dir.join(INDEX_FILE), |f| f.write_all(&json)).map_err(|_| "La photothèque ne peut pas être enregistrée.".to_string())
}

/// Date de prise de vue écrite par l'appareil (EXIF), lue avant que l'import retire les métadonnées.
pub fn capture_date(data: &[u8]) -> Option<String> {
    let exif = exif::Reader::new().read_from_container(&mut std::io::Cursor::new(data)).ok()?;
    let field = exif.get_field(exif::Tag::DateTimeOriginal, exif::In::PRIMARY).or_else(|| exif.get_field(exif::Tag::DateTime, exif::In::PRIMARY))?;
    let text = field.display_value().to_string();
    let date = text.get(..10)?.replace(':', "-");
    class::ymd(&date).map(|_| date)
}

/// Une photo du disque, importée dans la photothèque : ses fichiers écrits, son entrée rendue (l'index
/// n'est pas encore enregistré).
pub fn add_file(dir: &Path, path: &Path) -> Result<ClassPhoto, ImportError> {
    let data = std::fs::read(path).map_err(|_| ImportError::Io)?;
    let taken = capture_date(&data).or_else(|| std::fs::metadata(path).and_then(|m| m.modified()).ok().map(|t| iso(class::date_of(t))));
    let asset = photo::import_bytes(&data)?;
    add_asset(dir, &asset, taken)
}

pub fn add_asset(dir: &Path, asset: &Asset, taken: Option<String>) -> Result<ClassPhoto, ImportError> {
    std::fs::create_dir_all(dir).map_err(|_| ImportError::Storage)?;
    let m = &asset.meta;
    for (thumb, bytes) in [(false, &asset.bytes), (true, &asset.thumb)] {
        let path = file_of(dir, &m.id, m.mime, thumb).ok_or(ImportError::Storage)?;
        std::fs::write(path, bytes.as_slice()).map_err(|_| ImportError::Storage)?;
    }
    Ok(ClassPhoto { id: m.id.clone(), width_px: m.width_px, height_px: m.height_px, mime: m.mime, taken, added: iso(class::today()), students: vec![], tags: vec![] })
}

/// Les photos retirées de l'index et du disque.
pub fn remove(dir: &Path, lib: &mut Phototheque, ids: &[Id]) {
    for p in lib.photos.iter().filter(|p| ids.contains(&p.id)) {
        for thumb in [false, true] {
            if let Some(f) = file_of(dir, &p.id, p.mime, thumb) {
                let _ = std::fs::remove_file(f);
            }
        }
    }
    lib.photos.retain(|p| !ids.contains(&p.id));
}

/// Changement d'étiquettes sur plusieurs photos à la fois.
#[derive(Serialize, Deserialize, Clone, Debug, Default, TS)]
#[ts(export)]
pub struct TagChange {
    pub add_students: Vec<Id>,
    pub remove_students: Vec<Id>,
    pub add_tags: Vec<String>,
    pub remove_tags: Vec<String>,
}

pub fn tag(lib: &mut Phototheque, ids: &[Id], change: &TagChange) {
    for p in lib.photos.iter_mut().filter(|p| ids.contains(&p.id)) {
        p.students.retain(|s| !change.remove_students.contains(s));
        for s in &change.add_students {
            if !p.students.contains(s) {
                p.students.push(s.clone());
            }
        }
        p.tags.retain(|t| !change.remove_tags.contains(t));
        for t in change.add_tags.iter().map(|t| t.trim()).filter(|t| !t.is_empty()) {
            if !p.tags.iter().any(|x| x == t) {
                p.tags.push(t.to_string());
            }
        }
    }
}

/// La photo, pour un document : ses octets et sa vignette lus dans le dossier.
pub fn load_asset(dir: &Path, p: &ClassPhoto) -> Option<Asset> {
    let read = |thumb| file_of(dir, &p.id, p.mime, thumb).and_then(|f| std::fs::read(f).ok());
    Some(Asset { meta: p.meta(), bytes: Arc::new(read(false)?), thumb: Arc::new(read(true)?) })
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageFormat, Rgb, RgbImage};
    use std::io::Cursor;

    fn jpeg_file(dir: &Path, name: &str) -> PathBuf {
        let mut out = Cursor::new(Vec::new());
        RgbImage::from_pixel(40, 30, Rgb([10, 120, 200])).write_to(&mut out, ImageFormat::Jpeg).unwrap();
        let p = dir.join(name);
        std::fs::write(&p, out.into_inner()).unwrap();
        p
    }

    #[test]
    fn photos_are_added_tagged_saved_and_removed() {
        let src = tempfile::tempdir().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let lib_dir = dir.path().join("Moyenne section 2026-2027");
        let mut lib = open(&lib_dir).unwrap();
        assert!(lib.photos.is_empty());
        for n in ["a.jpg", "b.jpg"] {
            lib.photos.push(add_file(&lib_dir, &jpeg_file(src.path(), n)).unwrap());
        }
        // Sans date de l'appareil : celle du fichier, aujourd'hui.
        assert_eq!(lib.photos[0].taken.as_deref(), Some(iso(class::today()).as_str()));
        let ids: Vec<Id> = lib.photos.iter().map(|p| p.id.clone()).collect();
        tag(&mut lib, &ids, &TagChange { add_students: vec!["lea".into(), "noe".into()], add_tags: vec![" Sortie à la ferme ".into(), "".into()], ..Default::default() });
        tag(&mut lib, &ids[1..], &TagChange { remove_students: vec!["noe".into()], ..Default::default() });
        assert_eq!(lib.photos[0].students, ["lea", "noe"]);
        assert_eq!(lib.photos[1].students, ["lea"]);
        assert_eq!(lib.photos[1].tags, ["Sortie à la ferme"]);
        save(&lib_dir, &lib).unwrap();
        let back = open(&lib_dir).unwrap();
        assert_eq!(back, lib);
        let asset = load_asset(&lib_dir, &back.photos[0]).unwrap();
        assert_eq!((asset.meta.width_px, asset.meta.height_px), (40, 30));
        let mut lib = back;
        remove(&lib_dir, &mut lib, &ids[..1]);
        assert_eq!(lib.photos.len(), 1);
        // Les fichiers de la photo retirée ont disparu, l'index et l'autre photo restent.
        assert_eq!(std::fs::read_dir(&lib_dir).unwrap().count(), 1 + 2);
    }

    #[test]
    fn the_camera_date_is_kept_before_metadata_are_removed() {
        // date.jpg : prise le 14 octobre 2026 (DateTimeOriginal), modifiée le 20 (DateTime).
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/date.jpg");
        assert_eq!(capture_date(&std::fs::read(&path).unwrap()).as_deref(), Some("2026-10-14"));
        let dir = tempfile::tempdir().unwrap();
        let p = add_file(dir.path(), &path).unwrap();
        assert_eq!(p.taken.as_deref(), Some("2026-10-14"));
        let stored = std::fs::read(file_of(dir.path(), &p.id, p.mime, false).unwrap()).unwrap();
        assert!(capture_date(&stored).is_none(), "plus aucune métadonnée dans la photothèque");
    }

    #[test]
    fn a_newer_index_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(INDEX_FILE), br#"{"format_version": 9, "photos": []}"#).unwrap();
        assert!(open(dir.path()).unwrap_err().contains("plus récente"));
    }

    #[test]
    fn ids_never_leave_the_folder() {
        assert!(file_of(Path::new("/x"), "../secret", Mime::Jpeg, false).is_none());
        assert_eq!(file_of(Path::new("/x"), "abc", Mime::Png, true), Some(PathBuf::from("/x/abc.vignette.png")));
    }
}
