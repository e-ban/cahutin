//! Banque d'images OpenMoji (CC BY-SA 4.0) embarquée : un zip de fichiers `<code>.svg`, construit par
//! `scripts/build-openmoji.py`. Décompressée image par image, à la demande : seul l'index des noms
//! reste en mémoire.

use std::collections::HashMap;
use std::io::{Cursor, Read};
use std::sync::{Mutex, OnceLock};
use zip::ZipArchive;

static BANK_ZIP: &[u8] = include_bytes!("../../../assets/openmoji/openmoji.zip");

type Bank = ZipArchive<Cursor<&'static [u8]>>;

/// L'archive, ouverte une fois.
fn archive() -> &'static Mutex<Bank> {
    static ARCHIVE: OnceLock<Mutex<Bank>> = OnceLock::new();
    ARCHIVE.get_or_init(|| Mutex::new(ZipArchive::new(Cursor::new(BANK_ZIP)).expect("banque OpenMoji embarquée")))
}

/// Code de l'image vers le rang de son entrée dans l'archive, lu sans rien décompresser.
fn index() -> &'static HashMap<String, usize> {
    static INDEX: OnceLock<HashMap<String, usize>> = OnceLock::new();
    INDEX.get_or_init(|| {
        let mut zip = archive().lock().unwrap();
        let mut out = HashMap::new();
        for i in 0..zip.len() {
            let entry = zip.by_index_raw(i).expect("entrée de la banque");
            if let Some(code) = entry.name().ok().and_then(|n| n.strip_suffix(".svg").map(String::from)) {
                out.insert(code, i);
            }
        }
        out
    })
}

/// Fichier SVG de l'image ; `None` pour un code inconnu.
pub fn svg(code: &str) -> Option<Vec<u8>> {
    let i = *index().get(code)?;
    let mut zip = archive().lock().unwrap();
    let mut entry = zip.by_index(i).ok()?;
    let mut bytes = Vec::new();
    entry.read_to_end(&mut bytes).ok()?;
    Some(bytes)
}

/// L'image retournée (de gauche à droite `h`, de haut en bas `v`) : son contenu dans un groupe qui la retourne
/// à l'intérieur de son `viewBox`. LibreOffice n'applique pas le miroir d'une image SVG : l'export en écrit une
/// copie retournée.
pub fn mirrored_svg(svg: &[u8], h: bool, v: bool) -> Option<Vec<u8>> {
    let text = std::str::from_utf8(svg).ok()?;
    let open = text.find("<svg")?;
    let open_end = open + text[open..].find('>')? + 1;
    let close = text.rfind("</svg>")?;
    let vb = text[open..open_end].split("viewBox=\"").nth(1)?.split('"').next()?;
    let n: Vec<f64> = vb.split([' ', ',']).filter(|s| !s.is_empty()).filter_map(|s| s.parse().ok()).collect();
    let [x, y, w, hh] = n[..] else { return None };
    let (a, d) = (if h { -1.0 } else { 1.0 }, if v { -1.0 } else { 1.0 });
    let (e, f) = (if h { 2.0 * x + w } else { 0.0 }, if v { 2.0 * y + hh } else { 0.0 });
    Some(format!("{}<g transform=\"matrix({a} 0 0 {d} {e} {f})\">{}</g>{}", &text[..open_end], &text[open_end..close], &text[close..]).into_bytes())
}

/// Version au trait noir (à colorier) de l'image ; une couleur de peau n'en a pas d'autre que celle de
/// l'image de base.
pub fn outline_svg(code: &str) -> Option<Vec<u8>> {
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
    index().len()
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
        let outline: Vec<u8> = outline_svg("1F98A").unwrap();
        assert!(std::str::from_utf8(&outline).unwrap().contains("<svg"));
        assert_eq!(outline_svg("1F9D2-1F3FD"), outline_svg("1F9D2"));
        assert!(has_skin_tone("1F9D2-1F3FD") && !has_skin_tone("1F98A"));
        assert_eq!(base_code("1F469-1F3FE-200D-1F3EB"), "1F469-200D-1F3EB");
    }

    #[test]
    fn known_picture_is_an_svg() {
        // Chaque image est décompressée à la demande : l'appelant reçoit ses propres octets.
        let fox: Vec<u8> = svg("1F98A").unwrap();
        let fox = std::str::from_utf8(&fox).unwrap();
        assert!(fox.trim_start().starts_with("<svg"), "{}", &fox[..80]);
    }

    #[test]
    fn unknown_code_is_none() {
        assert!(svg("ZZZZ").is_none());
        assert!(svg("../1F98A").is_none());
    }
}
