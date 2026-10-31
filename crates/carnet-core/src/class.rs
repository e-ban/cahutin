//! Liste de la classe : élèves (prénom, nom, niveau, groupe, photo portrait et autorisation, symbole), et
//! son fichier `.cahutin-classe` : un zip avec `classe.json` et les photos, comme un `.carnet`.
//! Prénoms et photos ne sortent pas du poste et ne sont jamais journalisés.

use crate::model::{AssetMeta, Id, is_safe_id, new_id};
use crate::native::{NativeError, extension, read_capped};
use crate::photo::{Asset, AssetStore};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::io::{Read, Seek, Write};
use std::path::Path;
use std::sync::Arc;
use ts_rs::TS;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

/// Version la plus récente du fichier de classe que cette application sait lire.
pub const CLASS_FORMAT_VERSION: u32 = 1;
pub const CLASS_EXTENSION: &str = "cahutin-classe";
const CLASS_ENTRY: &str = "classe.json";

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, TS)]
#[ts(export)]
pub enum Level {
    PS,
    MS,
    GS,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, TS)]
#[ts(export)]
pub struct Student {
    pub id: Id,
    pub first_name: String,
    pub last_name: String,
    pub level: Option<Level>,
    /// Groupe libre (« Les loups ») ; vide : aucun.
    pub group: String,
    /// Photo portrait, dans `Class::photos`.
    pub photo: Option<Id>,
    /// Les parents ont donné leur accord : sans lui, la photo n'est jamais utilisée.
    pub photo_allowed: bool,
    /// Symbole de l'enfant : code d'une image OpenMoji.
    pub symbol: Option<String>,
}

impl Student {
    pub fn new(first_name: &str, last_name: &str) -> Student {
        Student {
            id: new_id(),
            first_name: first_name.into(),
            last_name: last_name.into(),
            level: None,
            group: String::new(),
            photo: None,
            photo_allowed: false,
            symbol: None,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, TS)]
#[ts(export)]
pub struct Class {
    pub format_version: u32,
    pub name: String,
    /// Année scolaire, « 2026-2027 ».
    pub year: String,
    pub students: Vec<Student>,
    /// Photos portraits des élèves.
    pub photos: BTreeMap<Id, AssetMeta>,
}

impl Class {
    pub fn new(name: &str, year: &str) -> Class {
        Class { format_version: CLASS_FORMAT_VERSION, name: name.into(), year: year.into(), students: Vec::new(), photos: BTreeMap::new() }
    }
}

/// Une ligne par élève : le prénom, puis le nom après le premier blanc. Lignes vides ignorées (copie
/// d'une colonne de tableur, tabulations comprises).
pub fn parse_list(text: &str) -> Vec<Student> {
    text.lines()
        .map(|l| l.split_whitespace().collect::<Vec<_>>())
        .filter(|words| !words.is_empty())
        .map(|words| Student::new(words[0], &words[1..].join(" ")))
        .collect()
}

/// Clé de tri : minuscules, sans accents.
fn fold(s: &str) -> String {
    s.chars()
        .flat_map(char::to_lowercase)
        .map(|c| match c {
            'à' | 'â' | 'ä' | 'á' | 'ã' => 'a',
            'ç' => 'c',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'î' | 'ï' | 'í' | 'ì' => 'i',
            'ô' | 'ö' | 'ó' | 'ò' | 'õ' => 'o',
            'ù' | 'û' | 'ü' | 'ú' => 'u',
            'ÿ' => 'y',
            'ñ' => 'n',
            c => c,
        })
        .collect()
}

/// Ordre alphabétique : prénom puis nom, sans tenir compte des accents ni des capitales.
pub fn sort_students(students: &mut [Student]) {
    students.sort_by_cached_key(|s| (fold(&s.first_name), fold(&s.last_name)));
}

/// « <nom> <année>.cahutin-classe », sans caractère interdit dans un nom de fichier.
pub fn default_file_name(class: &Class) -> String {
    let base = format!("{} {}", class.name.trim(), class.year.trim());
    let safe: String = base.chars().map(|c| if matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|') || c.is_control() { '-' } else { c }).collect();
    format!("{}.{CLASS_EXTENSION}", safe.trim())
}

/// Seules les photos d'un élève sont enregistrées.
pub fn save(class: &Class, store: &AssetStore, out: impl Write + Seek) -> Result<(), NativeError> {
    let mut class = class.clone();
    let used: Vec<Id> = class.students.iter().filter_map(|s| s.photo.clone()).collect();
    class.photos.retain(|id, _| used.contains(id));
    class.format_version = CLASS_FORMAT_VERSION;

    let mut zip = ZipWriter::new(out);
    let deflated = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    let stored = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
    zip.start_file(CLASS_ENTRY, deflated)?;
    zip.write_all(&serde_json::to_vec_pretty(&class).map_err(|_| NativeError::Corrupt)?)?;
    for meta in class.photos.values() {
        let asset = store.get(&meta.id).ok_or(NativeError::MissingAsset)?;
        let ext = extension(meta.mime);
        zip.start_file(format!("photos/{}.{ext}", meta.id), stored)?;
        zip.write_all(&asset.bytes)?;
        zip.start_file(format!("photos/{}.thumb.{ext}", meta.id), stored)?;
        zip.write_all(&asset.thumb)?;
    }
    zip.finish()?;
    Ok(())
}

/// Écriture sûre : un échec ne détruit pas l'ancien fichier.
pub fn save_to_path(class: &Class, store: &AssetStore, path: &Path) -> Result<(), NativeError> {
    crate::fsutil::write_atomically(path, |f| save(class, store, f))
}

/// Plafonds de lecture d'un fichier de classe : la liste, chaque photo, toutes les photos.
const MAX_LIST_BYTES: u64 = 16 * 1024 * 1024;
const MAX_PHOTO_BYTES: u64 = 64 * 1024 * 1024;
const MAX_PHOTOS_TOTAL_BYTES: u64 = 512 * 1024 * 1024;

/// Ce qu'affiche la liste des classes, sans lire les photos.
pub struct ClassSummary {
    pub name: String,
    pub year: String,
    pub students: usize,
}

pub struct LoadedClass {
    pub class: Class,
    pub assets: Vec<Asset>,
}

fn read_entry<R: Read + Seek>(zip: &mut ZipArchive<R>, name: &str, max: u64) -> Result<Vec<u8>, NativeError> {
    let mut entry = zip.by_name(name)?;
    read_capped(&mut entry, max).ok_or(NativeError::Corrupt)
}

/// Nom, année et nombre d'élèves, sans lire les photos.
pub fn read_info(input: impl Read + Seek) -> Result<ClassSummary, NativeError> {
    let mut zip = ZipArchive::new(input)?;
    let json = read_entry(&mut zip, CLASS_ENTRY, MAX_LIST_BYTES)?;
    let c: Class = serde_json::from_slice(&json).map_err(|_| NativeError::Corrupt)?;
    Ok(ClassSummary { name: c.name, year: c.year, students: c.students.len() })
}

pub fn read_info_from_path(path: &Path) -> Result<ClassSummary, NativeError> {
    read_info(std::fs::File::open(path)?)
}

pub fn load(input: impl Read + Seek) -> Result<LoadedClass, NativeError> {
    let mut zip = ZipArchive::new(input)?;
    let json = read_entry(&mut zip, CLASS_ENTRY, MAX_LIST_BYTES)?;
    let version: serde_json::Value = serde_json::from_slice(&json).map_err(|_| NativeError::Corrupt)?;
    let found = version["format_version"].as_u64().ok_or(NativeError::Corrupt)? as u32;
    if found > CLASS_FORMAT_VERSION {
        return Err(NativeError::NewerVersion(found));
    }
    let mut class: Class = serde_json::from_slice(&json).map_err(|_| NativeError::Corrupt)?;
    let ids_ok = class.photos.iter().all(|(k, m)| is_safe_id(k) && *k == m.id) && class.students.iter().filter_map(|s| s.photo.as_ref()).all(|id| is_safe_id(id));
    if !ids_ok {
        return Err(NativeError::Corrupt);
    }
    // Une photo désignée mais absente du fichier n'est plus désignée.
    for s in &mut class.students {
        if s.photo.as_ref().is_some_and(|p| !class.photos.contains_key(p)) {
            s.photo = None;
        }
    }
    let mut assets = Vec::new();
    let mut total = 0u64;
    for meta in class.photos.values() {
        let ext = extension(meta.mime);
        let bytes = read_entry(&mut zip, &format!("photos/{}.{ext}", meta.id), MAX_PHOTO_BYTES)?;
        let thumb = read_entry(&mut zip, &format!("photos/{}.thumb.{ext}", meta.id), MAX_PHOTO_BYTES)?;
        total += (bytes.len() + thumb.len()) as u64;
        if total > MAX_PHOTOS_TOTAL_BYTES {
            return Err(NativeError::Corrupt);
        }
        assets.push(Asset { meta: meta.clone(), bytes: Arc::new(bytes), thumb: Arc::new(thumb) });
    }
    Ok(LoadedClass { class, assets })
}

pub fn load_from_path(path: &Path) -> Result<LoadedClass, NativeError> {
    load(std::fs::File::open(path)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn png() -> Vec<u8> {
        let img = image::RgbImage::from_pixel(8, 6, image::Rgb([200, 120, 40]));
        let mut out = Cursor::new(Vec::new());
        img.write_to(&mut out, image::ImageFormat::Png).unwrap();
        out.into_inner()
    }

    fn names(students: &[Student]) -> Vec<(&str, &str)> {
        students.iter().map(|s| (s.first_name.as_str(), s.last_name.as_str())).collect()
    }

    fn zip_with(json: &str) -> Cursor<Vec<u8>> {
        let mut buf = Cursor::new(Vec::new());
        {
            let mut zip = ZipWriter::new(&mut buf);
            zip.start_file(CLASS_ENTRY, SimpleFileOptions::default()).unwrap();
            zip.write_all(json.as_bytes()).unwrap();
            zip.finish().unwrap();
        }
        buf.set_position(0);
        buf
    }

    #[test]
    fn parse_list_splits_first_and_last_names() {
        let s = parse_list("Léa Martin\n\n  Noé \t\nAnna-Lou  De Sousa\r\n");
        assert_eq!(names(&s), [("Léa", "Martin"), ("Noé", ""), ("Anna-Lou", "De Sousa")]);
        assert!(s.iter().all(|x| !x.photo_allowed));
        assert!(s[0].id != s[1].id && s[1].id != s[2].id);
    }

    #[test]
    fn sort_ignores_accents_and_case() {
        let mut s = vec![Student::new("émile", ""), Student::new("Eva", ""), Student::new("Adam", "")];
        sort_students(&mut s);
        assert_eq!(names(&s).iter().map(|n| n.0).collect::<Vec<_>>(), ["Adam", "émile", "Eva"]);
    }

    #[test]
    fn roundtrip_keeps_students_and_photos() {
        let asset = crate::photo::import_bytes(&png()).unwrap();
        let mut store = AssetStore::default();
        store.insert(asset.clone()).unwrap();
        let mut class = Class::new("Moyenne section", "2026-2027");
        let mut lea = Student::new("Léa", "Martin");
        lea.photo = Some(asset.meta.id.clone());
        lea.photo_allowed = true;
        lea.level = Some(Level::MS);
        lea.symbol = Some("1F98A".into());
        class.photos.insert(asset.meta.id.clone(), asset.meta.clone());
        class.students = vec![lea, Student::new("Noé", "")];
        let mut buf = Cursor::new(Vec::new());
        save(&class, &store, &mut buf).unwrap();
        buf.set_position(0);
        let back = load(buf).unwrap();
        assert_eq!(back.class, class);
        assert_eq!(back.assets.len(), 1);
        assert_eq!(*back.assets[0].bytes, *asset.bytes);
    }

    #[test]
    fn unused_photos_are_not_saved() {
        let asset = crate::photo::import_bytes(&png()).unwrap();
        let mut store = AssetStore::default();
        store.insert(asset.clone()).unwrap();
        let mut class = Class::new("MS", "2026-2027");
        class.photos.insert(asset.meta.id.clone(), asset.meta.clone());
        let mut buf = Cursor::new(Vec::new());
        save(&class, &store, &mut buf).unwrap();
        buf.set_position(0);
        assert!(load(buf).unwrap().class.photos.is_empty());
    }

    #[test]
    fn newer_class_file_is_refused() {
        let json = serde_json::to_string(&Class { format_version: 2, ..Class::new("MS", "2026-2027") }).unwrap();
        assert_eq!(load(zip_with(&json)).err(), Some(NativeError::NewerVersion(2)));
    }

    #[test]
    fn unsafe_photo_id_is_refused() {
        let mut class = Class::new("MS", "2026-2027");
        let mut s = Student::new("Léa", "");
        s.photo = Some("../x".into());
        class.students.push(s);
        let json = serde_json::to_string(&class).unwrap();
        assert_eq!(load(zip_with(&json)).err(), Some(NativeError::Corrupt));
    }

    #[test]
    fn a_photo_missing_from_the_file_is_dropped() {
        let mut class = Class::new("MS", "2026-2027");
        let mut s = Student::new("Léa", "");
        s.photo = Some("absente".into());
        s.photo_allowed = true;
        class.students.push(s);
        let back = load(zip_with(&serde_json::to_string(&class).unwrap())).unwrap();
        assert_eq!(back.class.students[0].photo, None);
    }

    #[test]
    fn read_info_reads_only_the_list() {
        let mut class = Class::new("Moyenne section", "2026-2027");
        class.students = parse_list("Léa\nNoé");
        let info = read_info(zip_with(&serde_json::to_string(&class).unwrap())).unwrap();
        assert_eq!((info.name.as_str(), info.year.as_str(), info.students), ("Moyenne section", "2026-2027", 2));
    }

    #[test]
    fn default_file_name_is_safe() {
        assert_eq!(default_file_name(&Class::new("MS/GS", "2026-2027")), "MS-GS 2026-2027.cahutin-classe");
    }
}
