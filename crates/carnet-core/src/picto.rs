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

pub fn count() -> usize {
    bank().len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bank_has_2195_pictures() {
        assert_eq!(count(), 2195);
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
