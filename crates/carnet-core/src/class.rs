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
    /// Date de naissance, « 2022-03-12 » ; facultative (âge, anniversaires).
    #[serde(default)]
    #[ts(optional = nullable)]
    pub birth_date: Option<String>,
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
            birth_date: None,
        }
    }
}

/// (année, mois, jour) d'une date « AAAA-MM-JJ ».
pub fn ymd(date: &str) -> Option<(i32, u32, u32)> {
    let mut it = date.split('-');
    let (y, m, d) = (it.next()?.parse().ok()?, it.next()?.parse().ok()?, it.next()?.parse().ok()?);
    (it.next().is_none() && (1..=12).contains(&m) && (1..=31).contains(&d)).then_some((y, m, d))
}

/// Âge en années révolues au jour donné.
pub fn age_on(birth: &str, today: (i32, u32, u32)) -> Option<u32> {
    let (y, m, d) = ymd(birth)?;
    let age = today.0 - y - i32::from((today.1, today.2) < (m, d));
    u32::try_from(age).ok()
}

/// La date du jour (heure universelle), sans dépendance de calendrier.
pub fn today() -> (i32, u32, u32) {
    let days = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() / 86_400).unwrap_or(0) as i64;
    // Algorithme des jours civils (H. Hinnant).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    ((yoe + era * 400 + i64::from(m <= 2)) as i32, m, d)
}

pub const MONTHS: [&str; 12] = ["janvier", "février", "mars", "avril", "mai", "juin", "juillet", "août", "septembre", "octobre", "novembre", "décembre"];

/// « 12 mars », « 1er avril ».
pub fn birthday_text(birth: &str) -> Option<String> {
    let (_, m, d) = ymd(birth)?;
    let day = if d == 1 { "1er".to_string() } else { d.to_string() };
    Some(format!("{day} {}", MONTHS[m as usize - 1]))
}

/// Une date écrite à la française (« 12/03/2022 », « 12-03-2022 », « 12.03.22 ») ou au format ISO, en
/// « AAAA-MM-JJ ».
pub fn parse_date(text: &str) -> Option<String> {
    let t = text.trim();
    if ymd(t).is_some() {
        return Some(t.to_string());
    }
    let parts: Vec<&str> = t.split(['/', '-', '.']).collect();
    let [d, m, y] = parts[..] else { return None };
    let (d, m, mut y): (u32, u32, i32) = (d.parse().ok()?, m.parse().ok()?, y.parse().ok()?);
    if y < 100 {
        y += 2000;
    }
    let iso = format!("{y:04}-{m:02}-{d:02}");
    ymd(&iso).map(|_| iso)
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

/// Colonne d'un tableau d'élèves, reconnue à son titre.
#[derive(Clone, Copy, PartialEq)]
enum Column {
    FirstName,
    LastName,
    Birth,
    Group,
    Level,
    Other,
}

fn column_of(title: &str) -> Column {
    let t = fold(title.trim());
    if t.contains("prenom") {
        Column::FirstName
    } else if t.contains("naissance") || t == "date" || t.contains("ne(e) le") || t.contains("ne le") {
        Column::Birth
    } else if t.contains("groupe") || t.contains("equipe") {
        Column::Group
    } else if t.contains("niveau") || t == "classe" || t == "section" {
        Column::Level
    } else if t == "nom" || t.starts_with("nom ") || t.contains("nom de famille") {
        Column::LastName
    } else {
        Column::Other
    }
}

/// Séparateur des colonnes d'un tableau collé ou d'un CSV : tabulation, point-virgule ou virgule ; aucun
/// pour une simple liste de prénoms.
fn separator(lines: &[&str]) -> Option<char> {
    ['\t', ';', ','].into_iter().find(|&c| lines.iter().filter(|l| l.contains(c)).count() * 2 >= lines.len().max(1))
}

fn cell(c: &str) -> String {
    c.trim().trim_matches('"').trim().to_string()
}

/// La liste de la classe, collée ou lue dans un fichier. Une colonne de prénoms : une ligne par élève, le
/// prénom puis le nom après le premier blanc. Un tableau (tableur, CSV, export ONDE) : ses colonnes sont
/// reconnues à leur titre (prénom, nom, date de naissance, groupe, niveau) ; sans ligne de titres, prénom
/// puis nom. Lignes vides ignorées.
pub fn parse_list(text: &str) -> Vec<Student> {
    let lines: Vec<&str> = text.lines().filter(|l| !l.trim().trim_matches([';', ',', '\t']).trim().is_empty()).collect();
    let Some(sep) = separator(&lines) else {
        return lines.iter().map(|l| l.split_whitespace().collect::<Vec<_>>()).map(|w| Student::new(w[0], &w[1..].join(" "))).collect();
    };
    let rows: Vec<Vec<String>> = lines.iter().map(|l| l.split(sep).map(cell).collect()).collect();
    let titles: Vec<Column> = rows[0].iter().map(|t| column_of(t)).collect();
    let (columns, data) = if titles.contains(&Column::FirstName) {
        (titles, &rows[1..])
    } else {
        (vec![Column::FirstName, Column::LastName], &rows[..])
    };
    data.iter()
        .filter_map(|row| {
            let get = |c: Column| columns.iter().position(|&x| x == c).and_then(|i| row.get(i)).map(String::as_str).unwrap_or("");
            let first = get(Column::FirstName);
            if first.is_empty() {
                return None;
            }
            let mut s = Student::new(first, get(Column::LastName));
            s.birth_date = parse_date(get(Column::Birth));
            s.group = get(Column::Group).to_string();
            s.level = match fold(get(Column::Level)).trim() {
                l if l.contains("ps") || l.contains("petit") => Some(Level::PS),
                l if l.contains("ms") || l.contains("moyen") => Some(Level::MS),
                l if l.contains("gs") || l.contains("grand") => Some(Level::GS),
                _ => None,
            };
            Some(s)
        })
        .collect()
}

/// Texte d'un fichier de liste : UTF-8, sinon Windows-1252 (exports de tableurs et d'ONDE sous Windows).
pub fn decode_list_file(bytes: &[u8]) -> String {
    let bytes = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(bytes);
    match std::str::from_utf8(bytes) {
        Ok(s) => s.to_string(),
        Err(_) => bytes.iter().map(|&b| char::from(b)).collect(),
    }
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
    fn a_spreadsheet_is_read_by_column_titles() {
        let csv = "Nom;Prénom;Né(e) le;Niveau;Groupe\nDurand;Léa;12/03/2022;MS;Les renards\n\nMartin;Noé;2021-11-01;Grande section;\n";
        let s = parse_list(csv);
        assert_eq!(s.len(), 2);
        assert_eq!((s[0].first_name.as_str(), s[0].last_name.as_str()), ("Léa", "Durand"));
        assert_eq!(s[0].birth_date.as_deref(), Some("2022-03-12"));
        assert_eq!((s[0].level, s[0].group.as_str()), (Some(Level::MS), "Les renards"));
        assert_eq!((s[1].birth_date.as_deref(), s[1].level), (Some("2021-11-01"), Some(Level::GS)));
        // Collé d'un tableur, sans titres : prénom puis nom.
        let s = parse_list("Léa\tDurand\nNoé\tMartin\n");
        assert_eq!((s[1].first_name.as_str(), s[1].last_name.as_str()), ("Noé", "Martin"));
    }

    #[test]
    fn a_windows_file_is_decoded() {
        assert_eq!(decode_list_file(b"Pr\xe9nom\nL\xe9a"), "Prénom\nLéa");
        assert_eq!(decode_list_file("\u{feff}Léa".as_bytes()), "Léa");
    }

    #[test]
    fn age_and_birthday() {
        assert_eq!(age_on("2022-03-12", (2026, 3, 11)), Some(3));
        assert_eq!(age_on("2022-03-12", (2026, 3, 12)), Some(4));
        assert_eq!(birthday_text("2022-04-01").as_deref(), Some("1er avril"));
        assert_eq!(parse_date("12.03.22").as_deref(), Some("2022-03-12"));
        assert!(parse_date("bonjour").is_none());
        let (y, m, d) = today();
        assert!(y >= 2026 && (1..=12).contains(&m) && (1..=31).contains(&d));
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
