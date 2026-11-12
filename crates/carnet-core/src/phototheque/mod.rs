//! Photothèque de la classe : les photos des dossiers que l'enseignant a choisis de suivre, jamais
//! copiées : l'index `phototheque.json` (voir `index`) garde, pour chaque photo, son chemin, la date
//! de prise de vue, les élèves présents et les activités.
//! Aucun visage n'est reconnu : les élèves présents sont choisis à la main.

pub mod index;
pub mod scan;
pub mod thumbs;

use crate::class;
use crate::model::Id;
pub use index::*;
pub use scan::{ScanReport, follow, scan, unfollow};
use serde::{Deserialize, Serialize};
use std::io::{BufRead, Seek};
use std::path::Path;
use ts_rs::TS;

pub const INDEX_FILE: &str = "phototheque.json";

fn iso((y, m, d): (i32, u32, u32)) -> String {
    format!("{y:04}-{m:02}-{d:02}")
}

/// Date de prise de vue écrite par l'appareil (EXIF), lue dans un flux : seul l'en-tête est lu.
fn capture_date_from(source: &mut (impl BufRead + Seek)) -> Option<String> {
    let exif = exif::Reader::new().read_from_container(source).ok()?;
    let field = exif.get_field(exif::Tag::DateTimeOriginal, exif::In::PRIMARY).or_else(|| exif.get_field(exif::Tag::DateTime, exif::In::PRIMARY))?;
    let text = field.display_value().to_string();
    let date = text.get(..10)?.replace(':', "-");
    class::ymd(&date).map(|_| date)
}

/// Date de prise de vue écrite par l'appareil (EXIF), lue avant que l'import retire les métadonnées.
pub fn capture_date(data: &[u8]) -> Option<String> {
    capture_date_from(&mut std::io::Cursor::new(data))
}

fn file_date(path: &Path) -> Option<String> {
    std::fs::metadata(path).and_then(|m| m.modified()).ok().map(|t| iso(class::date_of(t)))
}

/// Jour de la prise de vue d'un fichier : celui de l'appareil (EXIF), sinon sa date de modification.
pub fn taken_of(path: &Path, data: &[u8]) -> Option<String> {
    capture_date(data).or_else(|| file_date(path))
}

/// Comme `taken_of`, sans lire toute la photo : seul l'en-tête du fichier est lu.
pub fn taken_of_file(path: &Path) -> Option<String> {
    let exif_date = std::fs::File::open(path).ok().and_then(|f| capture_date_from(&mut std::io::BufReader::new(f)));
    exif_date.or_else(|| file_date(path))
}

/// Masque ou réaffiche des photos : elles restent dans l'index (étiquettes gardées) et leurs fichiers
/// ne sont pas touchés.
pub fn hide(lib: &mut Phototheque, ids: &[Id], hidden: bool) {
    for p in lib.photos.iter_mut().filter(|p| ids.contains(&p.id)) {
        p.hidden = hidden;
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Mime;

    fn photo(id: &str) -> ClassPhoto {
        ClassPhoto {
            id: id.into(),
            width_px: 4,
            height_px: 3,
            mime: Mime::Jpeg,
            taken: None,
            added: "2026-10-01".into(),
            students: vec![],
            tags: vec![],
            path: format!("/photos/{id}.jpg"),
            folder: "photos".into(),
            size: 1,
            modified: 1,
            fingerprint: "1-0".into(),
            hidden: false,
            missing: false,
        }
    }

    #[test]
    fn photos_are_tagged_and_hidden() {
        let mut lib = Phototheque { format_version: FORMAT_VERSION, folders: vec![], photos: vec![photo("a"), photo("b")] };
        let ids: Vec<Id> = vec!["a".into(), "b".into()];
        tag(&mut lib, &ids, &TagChange { add_students: vec!["lea".into(), "noe".into()], add_tags: vec![" Sortie à la ferme ".into(), "".into()], ..Default::default() });
        tag(&mut lib, &ids[1..], &TagChange { remove_students: vec!["noe".into()], ..Default::default() });
        assert_eq!(lib.photos[0].students, ["lea", "noe"]);
        assert_eq!(lib.photos[1].students, ["lea"]);
        assert_eq!(lib.photos[1].tags, ["Sortie à la ferme"]);
        hide(&mut lib, &ids[..1], true);
        assert_eq!(lib.photos.len(), 2);
        assert!(lib.photos[0].hidden && !lib.photos[1].hidden);
        hide(&mut lib, &ids[..1], false);
        assert!(!lib.photos[0].hidden);
    }

    #[test]
    fn the_camera_date_is_read_from_the_file() {
        // date.jpg : prise le 14 octobre 2026 (DateTimeOriginal), modifiée le 20 (DateTime).
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/date.jpg");
        let data = std::fs::read(&path).unwrap();
        assert_eq!(capture_date(&data).as_deref(), Some("2026-10-14"));
        assert_eq!(taken_of(&path, &data).as_deref(), Some("2026-10-14"));
        assert_eq!(taken_of_file(&path).as_deref(), Some("2026-10-14"));
    }
}
