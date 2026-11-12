//! Photothèque de la classe : les photos des dossiers que l'enseignant a choisis de suivre, jamais
//! copiées : l'index `phototheque.json` (voir `index`) garde, pour chaque photo, son chemin, la date
//! de prise de vue, les élèves présents et les activités.
//! Aucun visage n'est reconnu : les élèves présents sont choisis à la main.

pub mod index;

use crate::class;
use crate::model::Id;
pub use index::*;
use serde::{Deserialize, Serialize};
use std::path::Path;
use ts_rs::TS;

pub const INDEX_FILE: &str = "phototheque.json";

fn iso((y, m, d): (i32, u32, u32)) -> String {
    format!("{y:04}-{m:02}-{d:02}")
}

/// Date de prise de vue écrite par l'appareil (EXIF), lue avant que l'import retire les métadonnées.
pub fn capture_date(data: &[u8]) -> Option<String> {
    let exif = exif::Reader::new().read_from_container(&mut std::io::Cursor::new(data)).ok()?;
    let field = exif.get_field(exif::Tag::DateTimeOriginal, exif::In::PRIMARY).or_else(|| exif.get_field(exif::Tag::DateTime, exif::In::PRIMARY))?;
    let text = field.display_value().to_string();
    let date = text.get(..10)?.replace(':', "-");
    class::ymd(&date).map(|_| date)
}

/// Jour de la prise de vue d'un fichier : celui de l'appareil (EXIF), sinon sa date de modification.
pub fn taken_of(path: &Path, data: &[u8]) -> Option<String> {
    capture_date(data).or_else(|| std::fs::metadata(path).and_then(|m| m.modified()).ok().map(|t| iso(class::date_of(t))))
}

/// Les photos retirées de l'index. Leurs fichiers ne sont pas touchés.
pub fn remove(lib: &mut Phototheque, ids: &[Id]) {
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
    fn photos_are_tagged_and_removed() {
        let mut lib = Phototheque { format_version: FORMAT_VERSION, folders: vec![], photos: vec![photo("a"), photo("b")] };
        let ids: Vec<Id> = vec!["a".into(), "b".into()];
        tag(&mut lib, &ids, &TagChange { add_students: vec!["lea".into(), "noe".into()], add_tags: vec![" Sortie à la ferme ".into(), "".into()], ..Default::default() });
        tag(&mut lib, &ids[1..], &TagChange { remove_students: vec!["noe".into()], ..Default::default() });
        assert_eq!(lib.photos[0].students, ["lea", "noe"]);
        assert_eq!(lib.photos[1].students, ["lea"]);
        assert_eq!(lib.photos[1].tags, ["Sortie à la ferme"]);
        remove(&mut lib, &ids[..1]);
        assert_eq!(lib.photos.len(), 1);
    }

    #[test]
    fn the_camera_date_is_read_from_the_file() {
        // date.jpg : prise le 14 octobre 2026 (DateTimeOriginal), modifiée le 20 (DateTime).
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/date.jpg");
        let data = std::fs::read(&path).unwrap();
        assert_eq!(capture_date(&data).as_deref(), Some("2026-10-14"));
        assert_eq!(taken_of(&path, &data).as_deref(), Some("2026-10-14"));
    }
}
