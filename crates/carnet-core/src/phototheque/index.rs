//! L'index de la photothèque, version 2 : des dossiers suivis et, pour chaque photo, son chemin (rien
//! n'est copié). Un index de la version 1 (photos copiées dans le dossier de la classe) est converti en
//! mémoire à l'ouverture.

use super::INDEX_FILE;
use crate::fsutil::write_atomically;
use crate::model::{AssetMeta, Id, Mime, is_safe_id};
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::path::Path;
use ts_rs::TS;

pub const FORMAT_VERSION: u32 = 2;

/// Un dossier que l'enseignant suit.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, TS)]
#[ts(export)]
pub struct FollowedFolder {
    pub path: String,
    /// Absent au dernier parcours.
    #[serde(default)]
    pub missing: bool,
}

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
    /// Chemin absolu du fichier.
    pub path: String,
    /// Dossier suivi d'où vient la photo, pour le filtre « Dossiers ».
    pub folder: String,
    /// Taille en octets.
    #[ts(type = "number")]
    pub size: u64,
    /// Date de modification, en secondes depuis 1970 : un fichier changé se voit.
    #[ts(type = "number")]
    pub modified: u64,
    /// Empreinte (taille + FNV-1a des 64 premiers Kio), en hexadécimal.
    pub fingerprint: String,
    /// « Retirer » : masquée mais gardée, pour ne pas revenir au prochain parcours.
    #[serde(default)]
    pub hidden: bool,
    /// Fichier absent au dernier parcours.
    #[serde(default)]
    pub missing: bool,
}

impl ClassPhoto {
    pub fn meta(&self) -> AssetMeta {
        AssetMeta { id: self.id.clone(), width_px: self.width_px, height_px: self.height_px, mime: self.mime, taken: self.taken.clone() }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default, TS)]
#[ts(export)]
pub struct Phototheque {
    pub format_version: u32,
    pub folders: Vec<FollowedFolder>,
    pub photos: Vec<ClassPhoto>,
}

/// Une photo de l'index version 1 : copiée dans le dossier de la classe sous `<id>.<ext>`.
#[derive(Deserialize)]
struct PhotoV1 {
    id: Id,
    width_px: u32,
    height_px: u32,
    mime: Mime,
    taken: Option<String>,
    added: String,
    students: Vec<Id>,
    tags: Vec<String>,
}

#[derive(Deserialize)]
struct PhototequeV1 {
    photos: Vec<PhotoV1>,
}

const FINGERPRINT_BYTES: u64 = 64 * 1024;

/// Empreinte d'un fichier : sa taille et le FNV-1a 64 bits de ses 64 premiers Kio, en hexadécimal
/// (« taille-hachage »). `None` si le fichier ne peut pas être lu.
pub fn fingerprint(path: &Path) -> Option<String> {
    let file = std::fs::File::open(path).ok()?;
    let size = file.metadata().ok()?.len();
    let mut head = Vec::new();
    file.take(FINGERPRINT_BYTES).read_to_end(&mut head).ok()?;
    let hash = head.iter().fold(0xcbf2_9ce4_8422_2325_u64, |h, b| (h ^ u64::from(*b)).wrapping_mul(0x0100_0000_01b3));
    Some(format!("{size:x}-{hash:016x}"))
}

fn ext(mime: Mime) -> &'static str {
    match mime {
        Mime::Jpeg => "jpg",
        Mime::Png => "png",
    }
}

fn convert_v1(dir: &Path, old: PhototequeV1) -> Phototheque {
    let folder = dir.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let photos = old
        .photos
        .into_iter()
        .filter(|p| is_safe_id(&p.id))
        .map(|p| {
            let path = dir.join(format!("{}.{}", p.id, ext(p.mime)));
            let fp = fingerprint(&path);
            let meta = std::fs::metadata(&path).ok();
            let modified = meta.as_ref().and_then(|m| m.modified().ok()).and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map_or(0, |d| d.as_secs());
            ClassPhoto {
                id: p.id,
                width_px: p.width_px,
                height_px: p.height_px,
                mime: p.mime,
                taken: p.taken,
                added: p.added,
                students: p.students,
                tags: p.tags,
                path: path.to_string_lossy().into_owned(),
                folder: folder.clone(),
                size: meta.map_or(0, |m| m.len()),
                modified,
                missing: fp.is_none(),
                fingerprint: fp.unwrap_or_default(),
                hidden: false,
            }
        })
        .collect();
    Phototheque { format_version: FORMAT_VERSION, folders: vec![FollowedFolder { path: dir.to_string_lossy().into_owned(), missing: false }], photos }
}

/// La photothèque du dossier ; vide s'il n'y en a pas encore. Un index plus récent que l'application est
/// refusé plutôt qu'écrasé ; un index de la version 1 est converti en mémoire (le fichier n'est réécrit
/// qu'à l'enregistrement).
pub fn open(dir: &Path) -> Result<Phototheque, String> {
    let path = dir.join(INDEX_FILE);
    let Ok(bytes) = std::fs::read(&path) else { return Ok(Phototheque { format_version: FORMAT_VERSION, folders: vec![], photos: vec![] }) };
    let unreadable = || "La photothèque de cette classe est illisible.".to_string();
    #[derive(Deserialize)]
    struct Head {
        format_version: u32,
    }
    let head: Head = serde_json::from_slice(&bytes).map_err(|_| unreadable())?;
    match head.format_version {
        v if v > FORMAT_VERSION => Err("Cette photothèque a été faite avec une version plus récente de Cahutin.".into()),
        1 => Ok(convert_v1(dir, serde_json::from_slice(&bytes).map_err(|_| unreadable())?)),
        _ => serde_json::from_slice(&bytes).map_err(|_| unreadable()),
    }
}

pub fn save(dir: &Path, lib: &Phototheque) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|_| "Le dossier des photos ne peut pas être créé.".to_string())?;
    let json = serde_json::to_vec_pretty(lib).map_err(|_| "La photothèque ne peut pas être enregistrée.".to_string())?;
    write_atomically::<std::io::Error>(&dir.join(INDEX_FILE), |f| f.write_all(&json)).map_err(|_| "La photothèque ne peut pas être enregistrée.".to_string())
}

#[cfg(test)]
mod tests {
    use super::super::INDEX_FILE;
    use super::*;
    use image::{ImageFormat, Rgb, RgbImage};
    use std::io::Cursor;
    use std::path::Path;

    fn jpeg_file(dir: &Path, name: &str) {
        let mut out = Cursor::new(Vec::new());
        RgbImage::from_pixel(40, 30, Rgb([10, 120, 200])).write_to(&mut out, ImageFormat::Jpeg).unwrap();
        std::fs::write(dir.join(name), out.into_inner()).unwrap();
    }

    const V1: &str = r#"{"format_version": 1, "photos": [
        {"id": "p1", "width_px": 40, "height_px": 30, "mime": "Jpeg", "taken": "2026-10-14", "added": "2026-10-15",
         "students": ["lea", "noe"], "tags": ["Sortie à la ferme"]},
        {"id": "p2", "width_px": 40, "height_px": 30, "mime": "Jpeg", "taken": null, "added": "2026-10-16",
         "students": [], "tags": ["Motricité"]}]}"#;

    #[test]
    fn a_v1_index_opens_as_v2() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("Moyenne section 2026-2027");
        std::fs::create_dir_all(&dir).unwrap();
        jpeg_file(&dir, "p1.jpg");
        jpeg_file(&dir, "p2.jpg");
        std::fs::write(dir.join(INDEX_FILE), V1).unwrap();
        let lib = open(&dir).unwrap();
        assert_eq!(lib.format_version, 2);
        assert_eq!(lib.folders, [FollowedFolder { path: dir.to_string_lossy().into_owned(), missing: false }]);
        assert_eq!(lib.photos.len(), 2);
        let p = &lib.photos[0];
        assert_eq!((p.id.as_str(), p.taken.as_deref(), p.added.as_str()), ("p1", Some("2026-10-14"), "2026-10-15"));
        assert_eq!(p.students, ["lea", "noe"]);
        assert_eq!(p.tags, ["Sortie à la ferme"]);
        assert_eq!(p.path, dir.join("p1.jpg").to_string_lossy());
        assert_eq!(p.folder, "Moyenne section 2026-2027");
        assert_eq!(p.size, std::fs::metadata(dir.join("p1.jpg")).unwrap().len());
        assert_eq!(p.fingerprint, fingerprint(&dir.join("p1.jpg")).unwrap());
        assert!(p.modified > 0 && !p.missing && !p.hidden);
        assert_eq!(lib.photos[1].tags, ["Motricité"]);
    }

    #[test]
    fn a_missing_v1_file_is_marked_missing() {
        let tmp = tempfile::tempdir().unwrap();
        jpeg_file(tmp.path(), "p1.jpg");
        std::fs::write(tmp.path().join(INDEX_FILE), V1).unwrap();
        let lib = open(tmp.path()).unwrap();
        assert!(!lib.photos[0].missing);
        let p2 = &lib.photos[1];
        assert!(p2.missing);
        assert_eq!((p2.size, p2.modified, p2.fingerprint.as_str()), (0, 0, ""));
        assert_eq!(p2.tags, ["Motricité"], "les étiquettes sont gardées");
    }

    #[test]
    fn a_v2_index_survives_save_and_open() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join(INDEX_FILE), V1).unwrap();
        jpeg_file(tmp.path(), "p1.jpg");
        let mut lib = open(tmp.path()).unwrap();
        lib.photos[0].hidden = true;
        lib.folders.push(FollowedFolder { path: "/ailleurs".into(), missing: true });
        save(tmp.path(), &lib).unwrap();
        assert_eq!(open(tmp.path()).unwrap(), lib);
    }

    #[test]
    fn no_index_gives_an_empty_library() {
        let tmp = tempfile::tempdir().unwrap();
        let lib = open(tmp.path()).unwrap();
        assert_eq!((lib.format_version, lib.folders.len(), lib.photos.len()), (2, 0, 0));
    }

    #[test]
    fn a_newer_index_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(INDEX_FILE), br#"{"format_version": 3, "folders": [], "photos": []}"#).unwrap();
        assert!(open(dir.path()).unwrap_err().contains("plus récente"));
    }

    #[test]
    fn the_fingerprint_value_is_stable() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("f.bin");
        let data: Vec<u8> = (0..100 * 1024).map(|i| (i % 251) as u8).collect();
        std::fs::write(&path, &data).unwrap();
        assert_eq!(fingerprint(&path).as_deref(), Some("19000-7df71ef445707abc"));
        assert!(fingerprint(&tmp.path().join("absent")).is_none());
    }
}
