//! Noms français des images OpenMoji, pour que l'assistant sache ce qu'une image montre et en trouve une.

use serde_json::Value;
use std::sync::OnceLock;

static INDEX: &str = include_str!("../../../assets/openmoji/index.json");

/// (code, mots-clés français), dans l'ordre de l'index ; le premier mot-clé est le nom de l'image.
fn index() -> &'static [(String, Vec<String>)] {
    static PARSED: OnceLock<Vec<(String, Vec<String>)>> = OnceLock::new();
    PARSED.get_or_init(|| {
        let v: Value = serde_json::from_str(INDEX).expect("index OpenMoji embarqué");
        v.as_array()
            .into_iter()
            .flatten()
            .filter_map(|e| {
                let code = e["code"].as_str()?.to_string();
                let fr = e["fr"].as_array()?.iter().filter_map(|k| k.as_str().map(String::from)).collect();
                Some((code, fr))
            })
            .collect()
    })
}

/// Nom français de l'image, ou `None` pour un code inconnu.
pub fn name(code: &str) -> Option<&'static str> {
    index().iter().find(|(c, _)| c == code).and_then(|(_, fr)| fr.first()).map(String::as_str)
}

/// Sans casse, sans accents, apostrophes unifiées.
fn fold(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .map(|c| match c {
            'à' | 'â' | 'ä' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'î' | 'ï' => 'i',
            'ô' | 'ö' => 'o',
            'ù' | 'û' | 'ü' => 'u',
            'ç' => 'c',
            '’' => '\'',
            c => c,
        })
        .collect::<String>()
        .trim()
        .to_string()
}

/// Image pour une requête en français : (code, nom). Par ordre de préférence : le nom de l'image égal à la
/// requête, puis un nom qui la contient, puis un mot-clé égal, puis un mot-clé qui la contient.
pub fn search(query: &str) -> Option<(&'static str, &'static str)> {
    let q = fold(query);
    if q.is_empty() {
        return None;
    }
    let rank = |fr: &[String]| -> Option<u8> {
        let name = fold(fr.first()?);
        if name == q {
            Some(0)
        } else if name.contains(&q) {
            Some(1)
        } else if fr.iter().any(|k| fold(k) == q) {
            Some(2)
        } else if fr.iter().any(|k| fold(k).contains(&q)) {
            Some(3)
        } else {
            None
        }
    };
    let (_, _, hit) = index().iter().enumerate().filter_map(|(i, e)| rank(&e.1).map(|r| (r, i, e))).min_by_key(|(r, i, _)| (*r, *i))?;
    Some((hit.0.as_str(), hit.1.first().map(String::as_str).unwrap_or("")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_image_has_its_french_name() {
        assert_eq!(name("1F382"), Some("gâteau d’anniversaire"));
        assert_eq!(name("pas-un-code"), None);
    }

    #[test]
    fn search_ignores_case_and_accents() {
        assert_eq!(search("Gateau d'anniversaire").map(|r| r.0), Some("1F382"));
        assert_eq!(search("tulipe").map(|r| r.0), Some("1F337"));
        assert_eq!(search("xqzwv"), None);
        assert_eq!(search("  "), None);
    }

    #[test]
    fn search_prefers_the_name_of_the_image() {
        // « feuille » est aussi un mot-clé de la main levée (pierre-feuille-ciseaux).
        let (_, name) = search("feuille").unwrap();
        assert!(name.contains("feuille"), "{name}");
    }

    #[test]
    fn search_finds_a_keyword_containing_the_query() {
        assert!(search("tulip").is_some());
    }
}
