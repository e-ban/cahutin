//! Polices : celles fournies avec l'application (licence OFL) et celles ajoutées par l'utilisateur.
//! L'écran et l'export utilisent les mêmes fichiers.

use serde::Serialize;
use std::path::Path;
use std::sync::{Arc, OnceLock};
use ts_rs::TS;

pub const DEFAULT_FAMILY: &str = "Andika";

#[derive(Debug, Clone)]
pub struct FontFace {
    pub family: String,
    pub bold: bool,
    pub italic: bool,
    pub file_name: String,
    /// "truetype" ou "opentype", tel qu'écrit dans `svg:font-face-format`.
    pub format: &'static str,
    pub bytes: Arc<[u8]>,
    /// Faux si la licence interdit d'intégrer la police à un fichier (`OS/2.fsType` « restricted »).
    pub embeddable: bool,
    pub user: bool,
}

#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[ts(export)]
pub struct FontFamilyInfo {
    pub name: String,
    pub has_bold: bool,
    pub has_italic: bool,
    pub user: bool,
    pub embeddable: bool,
    /// Un mot pour les polices fournies (« scripte », « cursive »…), rien pour celles de l'utilisateur.
    pub kind: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontError {
    Unreadable,
    Collection,
    BuiltinName,
    Io,
}

impl FontError {
    pub fn message_fr(&self) -> String {
        match self {
            FontError::Unreadable => "Ce fichier n'est pas une police lisible (.ttf ou .otf attendu).",
            FontError::Collection => "Les collections de polices (.ttc) ne sont pas prises en charge : choisissez un fichier .ttf ou .otf.",
            FontError::BuiltinName => "Cette police porte le nom d'une police fournie avec l'application : elle n'a pas été ajoutée.",
            FontError::Io => "Impossible de copier la police dans le dossier de l'application.",
        }
        .to_string()
    }
}

macro_rules! face {
    ($family:literal, $bold:literal, $italic:literal, $dir:literal, $file:literal, $format:literal) => {
        FontFace {
            family: $family.to_string(),
            bold: $bold,
            italic: $italic,
            file_name: $file.to_string(),
            format: $format,
            bytes: Arc::from(&include_bytes!(concat!("../../../assets/fonts/", $dir, "/", $file))[..]),
            embeddable: true,
            user: false,
        }
    };
}

fn builtin_faces() -> &'static [FontFace] {
    static CATALOG: OnceLock<Vec<FontFace>> = OnceLock::new();
    CATALOG.get_or_init(|| {
        vec![
            face!("Andika", false, false, "andika", "Andika-Regular.ttf", "truetype"),
            face!("Andika", true, false, "andika", "Andika-Bold.ttf", "truetype"),
            face!("Andika", false, true, "andika", "Andika-Italic.ttf", "truetype"),
            face!("Andika", true, true, "andika", "Andika-BoldItalic.ttf", "truetype"),
            face!("Atkinson Hyperlegible", false, false, "atkinson-hyperlegible", "AtkinsonHyperlegible-Regular.ttf", "truetype"),
            face!("Atkinson Hyperlegible", true, false, "atkinson-hyperlegible", "AtkinsonHyperlegible-Bold.ttf", "truetype"),
            face!("Atkinson Hyperlegible", false, true, "atkinson-hyperlegible", "AtkinsonHyperlegible-Italic.ttf", "truetype"),
            face!("Atkinson Hyperlegible", true, true, "atkinson-hyperlegible", "AtkinsonHyperlegible-BoldItalic.ttf", "truetype"),
            face!("OpenDyslexic", false, false, "opendyslexic", "OpenDyslexic-Regular.otf", "opentype"),
            face!("OpenDyslexic", true, false, "opendyslexic", "OpenDyslexic-Bold.otf", "opentype"),
            face!("OpenDyslexic", false, true, "opendyslexic", "OpenDyslexic-Italic.otf", "opentype"),
            face!("OpenDyslexic", true, true, "opendyslexic", "OpenDyslexic-Bold-Italic.otf", "opentype"),
            face!("Playwrite FR Moderne", false, false, "playwrite-fr-moderne", "PlaywriteFRModerne-Regular.ttf", "truetype"),
        ]
    })
}

fn builtin_kind(family: &str) -> Option<String> {
    let kind = match family {
        "Andika" => "scripte",
        "Playwrite FR Moderne" => "cursive",
        "Atkinson Hyperlegible" => "lisible",
        "OpenDyslexic" => "dyslexie",
        _ => return None,
    };
    Some(kind.to_string())
}

fn is_builtin_family(family: &str) -> bool {
    builtin_faces().iter().any(|f| f.family.eq_ignore_ascii_case(family))
}

/// Lit une police : nom de famille typographique (sinon le nom de famille), gras, italique, licence.
fn parse_face(bytes: &[u8], file_name: &str) -> Result<FontFace, FontError> {
    if bytes.starts_with(b"ttcf") {
        return Err(FontError::Collection);
    }
    let face = ttf_parser::Face::parse(bytes, 0).map_err(|_| FontError::Unreadable)?;
    let name = |id: u16| face.names().into_iter().filter(|n| n.name_id == id).find_map(|n| n.to_string());
    let family = name(ttf_parser::name_id::TYPOGRAPHIC_FAMILY)
        .or_else(|| name(ttf_parser::name_id::FAMILY))
        .filter(|f| !f.trim().is_empty())
        .ok_or(FontError::Unreadable)?;
    Ok(FontFace {
        family: family.trim().to_string(),
        bold: face.weight().to_number() >= 600,
        italic: face.is_italic(),
        file_name: file_name.to_string(),
        format: if bytes.starts_with(b"OTTO") { "opentype" } else { "truetype" },
        bytes: Arc::from(bytes),
        embeddable: face.permissions() != Some(ttf_parser::Permissions::Restricted),
        user: true,
    })
}

/// Polices utilisateur lisibles du dossier ; celles qui portent le nom d'une police fournie sont écartées.
fn read_user_dir(dir: &Path) -> Vec<FontFace> {
    let Ok(entries) = std::fs::read_dir(dir) else { return vec![] };
    let mut faces: Vec<FontFace> = entries
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file())
        .filter_map(|e| {
            let bytes = std::fs::read(e.path()).ok()?;
            parse_face(&bytes, &e.file_name().to_string_lossy()).ok()
        })
        .filter(|f| !is_builtin_family(&f.family))
        .collect();
    faces.sort_by(|a, b| a.family.cmp(&b.family).then(a.file_name.cmp(&b.file_name)));
    faces
}

/// Polices fournies, puis polices de l'utilisateur.
#[derive(Clone, Debug)]
pub struct FontSet {
    faces: Vec<FontFace>,
}

impl FontSet {
    pub fn builtin() -> FontSet {
        FontSet { faces: builtin_faces().to_vec() }
    }

    /// Les fichiers illisibles du dossier sont ignorés.
    pub fn with_user_dir(dir: &Path) -> FontSet {
        let mut set = FontSet::builtin();
        set.faces.extend(read_user_dir(dir));
        set
    }

    /// Fournies d'abord, puis celles de l'utilisateur par nom.
    pub fn families(&self) -> Vec<FontFamilyInfo> {
        let mut out: Vec<FontFamilyInfo> = Vec::new();
        for f in &self.faces {
            if out.iter().any(|i| i.name == f.family) {
                continue;
            }
            let faces = || self.faces.iter().filter(|g| g.family == f.family);
            out.push(FontFamilyInfo {
                name: f.family.clone(),
                has_bold: faces().any(|g| g.bold),
                has_italic: faces().any(|g| g.italic),
                user: f.user,
                embeddable: faces().all(|g| g.embeddable),
                kind: if f.user { None } else { builtin_kind(&f.family) },
            });
        }
        out
    }

    /// Variante absente : le régulier de la famille (ou sa première variante). Famille inconnue : `None`.
    pub fn face(&self, family: &str, bold: bool, italic: bool) -> Option<&FontFace> {
        let in_family = || self.faces.iter().filter(move |f| f.family == family);
        in_family()
            .find(|f| f.bold == bold && f.italic == italic)
            .or_else(|| in_family().find(|f| !f.bold && !f.italic))
            .or_else(|| in_family().next())
    }

    pub fn faces(&self) -> &[FontFace] {
        &self.faces
    }
}

fn safe_file_name(name: &str) -> String {
    let cleaned: String =
        name.chars().map(|c| if c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-') { c } else { '_' }).collect();
    let cleaned = cleaned.trim_start_matches('.').to_string();
    if cleaned.is_empty() { "police.ttf".to_string() } else { cleaned }
}

/// Copie la police dans le dossier des polices utilisateur ; renvoie sa famille telle que listée.
pub fn add_user_font(dir: &Path, src: &Path) -> Result<FontFamilyInfo, FontError> {
    let bytes = std::fs::read(src).map_err(|_| FontError::Unreadable)?;
    let original = src.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    add_user_font_bytes(dir, &original, &bytes)
}

/// Comme `add_user_font`, pour une police reçue dans un document.
pub fn add_user_font_bytes(dir: &Path, file_name: &str, bytes: &[u8]) -> Result<FontFamilyInfo, FontError> {
    let face = parse_face(bytes, file_name)?;
    if is_builtin_family(&face.family) {
        return Err(FontError::BuiltinName);
    }
    std::fs::create_dir_all(dir).map_err(|_| FontError::Io)?;
    std::fs::write(dir.join(safe_file_name(file_name)), bytes).map_err(|_| FontError::Io)?;
    FontSet::with_user_dir(dir).families().into_iter().find(|f| f.user && f.name == face.family).ok_or(FontError::Io)
}

/// Nom de famille d'un fichier de police, s'il est lisible.
pub fn family_of(bytes: &[u8]) -> Option<String> {
    parse_face(bytes, "").ok().map(|f| f.family)
}

/// Nom de fichier servi par `/font/<nom>` : seulement des caractères sûrs, jamais de chemin.
pub fn is_safe_file_name(name: &str) -> bool {
    !name.is_empty() && !name.starts_with('.') && name.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
}

/// Supprime tous les fichiers de cette famille dans le dossier des polices utilisateur.
pub fn remove_user_family(dir: &Path, family: &str) -> std::io::Result<()> {
    for f in read_user_dir(dir).iter().filter(|f| f.family == family) {
        std::fs::remove_file(dir.join(&f.file_name))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_face_parses_and_has_its_family_name() {
        let set = FontSet::builtin();
        assert!(!set.faces.is_empty());
        for f in &set.faces {
            let face = ttf_parser::Face::parse(&f.bytes, 0).unwrap_or_else(|_| panic!("{} illisible", f.file_name));
            let names: Vec<String> = face
                .names()
                .into_iter()
                .filter(|n| n.name_id == ttf_parser::name_id::FAMILY)
                .filter_map(|n| n.to_string())
                .collect();
            assert!(names.iter().any(|n| n.contains(&f.family)), "{}: {names:?}", f.file_name);
        }
    }

    #[test]
    fn every_family_ships_its_licence() {
        let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets/fonts");
        for dir in std::fs::read_dir(root).unwrap() {
            let licence = std::fs::read_to_string(dir.unwrap().path().join("OFL.txt")).unwrap();
            assert!(licence.contains("SIL Open Font License"));
        }
    }

    #[test]
    fn missing_variant_falls_back_to_regular() {
        let set = FontSet::builtin();
        let f = set.face("Playwrite FR Moderne", true, false).unwrap();
        assert!(!f.bold && !f.italic);
        assert!(set.face("Inconnue", false, false).is_none());
    }

    #[test]
    fn families_report_variants() {
        let fams = FontSet::builtin().families();
        let andika = fams.iter().find(|f| f.name == "Andika").unwrap();
        assert!(andika.has_bold && andika.has_italic);
        let playwrite = fams.iter().find(|f| f.name == "Playwrite FR Moderne").unwrap();
        assert!(!playwrite.has_bold && !playwrite.has_italic);
        assert_eq!(fams.len(), 4);
    }

    const FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures");

    fn fixture(name: &str) -> std::path::PathBuf {
        std::path::Path::new(FIXTURES).join(name)
    }

    #[test]
    fn user_font_is_added_and_listed() {
        let dir = tempfile::tempdir().unwrap();
        let info = add_user_font(dir.path(), &fixture("CarnetTest-Regular.ttf")).unwrap();
        assert_eq!(info.name, "Carnet Test");
        assert!(info.user && info.embeddable);
        assert_eq!(info.kind, None);
        let fams = FontSet::with_user_dir(dir.path()).families();
        assert_eq!(fams.len(), 5);
        assert_eq!(fams[4].name, "Carnet Test");
        assert!(fams[..4].iter().all(|f| !f.user && f.kind.is_some()));
        let face = FontSet::with_user_dir(dir.path()).face("Carnet Test", false, false).cloned().unwrap();
        assert!(face.user && face.embeddable);
        assert_eq!(face.file_name, "CarnetTest-Regular.ttf");
    }

    #[test]
    fn restricted_font_is_marked_not_embeddable() {
        let dir = tempfile::tempdir().unwrap();
        let info = add_user_font(dir.path(), &fixture("CarnetTestRestricted-Regular.ttf")).unwrap();
        assert!(info.user);
        assert!(!info.embeddable);
    }

    #[test]
    fn user_font_cannot_shadow_a_builtin_family() {
        let dir = tempfile::tempdir().unwrap();
        let src = std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets/fonts/andika/Andika-Regular.ttf"));
        assert_eq!(add_user_font(dir.path(), src).err(), Some(FontError::BuiltinName));
        assert!(FontError::BuiltinName.message_fr().contains("fournie"));
        // Même posé à la main dans le dossier, il ne remplace pas la police fournie.
        std::fs::copy(src, dir.path().join("Andika-Regular.ttf")).unwrap();
        let set = FontSet::with_user_dir(dir.path());
        assert!(!set.face("Andika", false, false).unwrap().user);
        assert_eq!(set.families().len(), 4);
    }

    #[test]
    fn corrupt_font_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let bad = dir.path().join("abîmée.ttf");
        std::fs::write(&bad, b"pas une police").unwrap();
        let fonts = dir.path().join("polices");
        assert_eq!(add_user_font(&fonts, &bad).err(), Some(FontError::Unreadable));
        let ttc = dir.path().join("lot.ttc");
        std::fs::write(&ttc, b"ttcf\0\x01\0\0\0\0\0\0").unwrap();
        assert_eq!(add_user_font(&fonts, &ttc).err(), Some(FontError::Collection));
        assert!(!FontError::Unreadable.message_fr().is_empty());
        assert_eq!(std::fs::read_dir(&fonts).map(|d| d.count()).unwrap_or(0), 0);
    }

    #[test]
    fn remove_user_family_deletes_its_files() {
        let dir = tempfile::tempdir().unwrap();
        add_user_font(dir.path(), &fixture("CarnetTest-Regular.ttf")).unwrap();
        add_user_font(dir.path(), &fixture("CarnetTestRestricted-Regular.ttf")).unwrap();
        remove_user_family(dir.path(), "Carnet Test").unwrap();
        let names: Vec<String> = FontSet::with_user_dir(dir.path()).families().into_iter().map(|f| f.name).collect();
        assert!(!names.contains(&"Carnet Test".to_string()));
        assert!(names.contains(&"Carnet Test Restricted".to_string()));
    }
}
