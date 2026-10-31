//! Banque d'images OpenMoji (CC BY-SA 4.0) embarquée : un zip de fichiers `<code>.svg`, construit par
//! `scripts/build-openmoji.py`. Décompressée une fois, au premier besoin.

use std::collections::HashMap;
use std::io::Read;
use std::sync::OnceLock;

static BANK_ZIP: &[u8] = include_bytes!("../../../assets/openmoji/openmoji.zip");

fn bank() -> &'static HashMap<String, Vec<u8>> {
    static BANK: OnceLock<HashMap<String, Vec<u8>>> = OnceLock::new();
    BANK.get_or_init(|| {
        let mut out = HashMap::new();
        let mut zip = zip::ZipArchive::new(std::io::Cursor::new(BANK_ZIP)).expect("banque OpenMoji embarquée");
        for i in 0..zip.len() {
            let mut entry = zip.by_index(i).expect("entrée de la banque");
            let Some(code) = entry.name().ok().and_then(|n| n.strip_suffix(".svg").map(String::from)) else { continue };
            let mut bytes = Vec::new();
            entry.read_to_end(&mut bytes).expect("image de la banque");
            out.insert(code, bytes);
        }
        out
    })
}

/// Fichier SVG de l'image ; `None` pour un code inconnu.
pub fn svg(code: &str) -> Option<&'static [u8]> {
    bank().get(code).map(Vec::as_slice)
}

/// Version au trait noir (à colorier) de l'image ; une couleur de peau n'en a pas d'autre que celle de
/// l'image de base.
pub fn outline_svg(code: &str) -> Option<&'static [u8]> {
    svg(&format!("contour/{}", base_code(code)))
}

/// Une image en couleur de peau (modificateur U+1F3FB à U+1F3FF).
pub fn has_skin_tone(code: &str) -> bool {
    code.split('-').any(|p| matches!(p, "1F3FB" | "1F3FC" | "1F3FD" | "1F3FE" | "1F3FF"))
}

/// Le code sans couleur de peau.
pub fn base_code(code: &str) -> String {
    code.split('-').filter(|p| !matches!(*p, "1F3FB" | "1F3FC" | "1F3FD" | "1F3FE" | "1F3FF")).collect::<Vec<_>>().join("-")
}

pub fn count() -> usize {
    bank().len()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 2 195 images, plus les cinq teintes de peau de 330 d'entre elles.
    #[test]
    fn bank_has_2195_pictures_and_their_skin_tones() {
        assert_eq!(count(), 2 * 2195 + 5 * 330);
        assert!(svg("1F9D2-1F3FD").is_some(), "enfant, peau mate");
    }

    /// Chaque image a sa version à colorier ; une couleur de peau prend celle de l'image de base.
    #[test]
    fn every_picture_has_an_outline_to_colour() {
        assert!(std::str::from_utf8(outline_svg("1F98A").unwrap()).unwrap().contains("<svg"));
        assert_eq!(outline_svg("1F9D2-1F3FD"), outline_svg("1F9D2"));
        assert!(has_skin_tone("1F9D2-1F3FD") && !has_skin_tone("1F98A"));
        assert_eq!(base_code("1F469-1F3FE-200D-1F3EB"), "1F469-200D-1F3EB");
    }

    #[test]
    fn known_picture_is_an_svg() {
        let fox = std::str::from_utf8(svg("1F98A").unwrap()).unwrap();
        assert!(fox.trim_start().starts_with("<svg"), "{}", &fox[..80]);
    }

    #[test]
    fn unknown_code_is_none() {
        assert!(svg("ZZZZ").is_none());
        assert!(svg("../1F98A").is_none());
    }
}
