//! Polices embarquées (toutes sous licence OFL), utilisées à l'écran et intégrées à l'export.

use serde::Serialize;
use ts_rs::TS;

pub const DEFAULT_FAMILY: &str = "Andika";

#[derive(Debug)]
pub struct FontFace {
    pub family: &'static str,
    pub bold: bool,
    pub italic: bool,
    pub file_name: &'static str,
    /// "truetype" ou "opentype", tel qu'écrit dans `svg:font-face-format`.
    pub format: &'static str,
    pub bytes: &'static [u8],
}

#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[ts(export)]
pub struct FontFamilyInfo {
    pub name: String,
    pub has_bold: bool,
    pub has_italic: bool,
}

macro_rules! face {
    ($family:literal, $bold:literal, $italic:literal, $dir:literal, $file:literal, $format:literal) => {
        FontFace {
            family: $family,
            bold: $bold,
            italic: $italic,
            file_name: $file,
            format: $format,
            bytes: include_bytes!(concat!("../../../assets/fonts/", $dir, "/", $file)),
        }
    };
}

static CATALOG: [FontFace; 13] = [
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
];

pub fn catalog() -> &'static [FontFace] {
    &CATALOG
}

pub fn families() -> Vec<FontFamilyInfo> {
    let mut out: Vec<FontFamilyInfo> = Vec::new();
    for f in catalog() {
        if !out.iter().any(|i| i.name == f.family) {
            out.push(FontFamilyInfo {
                name: f.family.to_string(),
                has_bold: catalog().iter().any(|g| g.family == f.family && g.bold),
                has_italic: catalog().iter().any(|g| g.family == f.family && g.italic),
            });
        }
    }
    out
}

/// Variante absente : le régulier de la famille. Famille inconnue : `None`.
pub fn face(family: &str, bold: bool, italic: bool) -> Option<&'static FontFace> {
    let in_family = || catalog().iter().filter(move |f| f.family == family);
    in_family()
        .find(|f| f.bold == bold && f.italic == italic)
        .or_else(|| in_family().find(|f| !f.bold && !f.italic))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_face_parses_and_has_its_family_name() {
        assert!(!catalog().is_empty());
        for f in catalog() {
            let face = ttf_parser::Face::parse(f.bytes, 0).unwrap_or_else(|_| panic!("{} illisible", f.file_name));
            let names: Vec<String> = face
                .names()
                .into_iter()
                .filter(|n| n.name_id == ttf_parser::name_id::FAMILY)
                .filter_map(|n| n.to_string())
                .collect();
            assert!(names.iter().any(|n| n.contains(f.family)), "{}: {names:?}", f.file_name);
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
        let f = face("Playwrite FR Moderne", true, false).unwrap();
        assert!(!f.bold && !f.italic);
        assert!(face("Inconnue", false, false).is_none());
    }

    #[test]
    fn families_report_variants() {
        let fams = families();
        let andika = fams.iter().find(|f| f.name == "Andika").unwrap();
        assert!(andika.has_bold && andika.has_italic);
        let playwrite = fams.iter().find(|f| f.name == "Playwrite FR Moderne").unwrap();
        assert!(!playwrite.has_bold && !playwrite.has_italic);
        assert_eq!(fams.len(), 4);
    }
}
