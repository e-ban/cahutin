//! Champs de la classe : des mots entre accolades, sans accents, que le remplissage avec la classe remplace
//! (`{prenom}`, `{age}`, `{PRENOM_1}`, `{anniversaires_mars}`…). Les anciennes écritures avec accents
//! (`{prénom}`, `{âge}`, `{anniversaires février}`) sont lues comme synonymes, jamais proposées.

use crate::class::{self, Student};
use crate::model::TextProps;

/// Les champs d'un élève, dans l'ordre où l'interface les propose (hors lettres et anniversaires du mois).
pub const KEYS: [&str; 6] = ["prenom", "PRENOM", "nom", "groupe", "age", "anniversaire"];

/// Anciennes écritures lues comme synonymes.
const ALIASES: [(&str, &str); 3] = [("prénom", "prenom"), ("PRÉNOM", "PRENOM"), ("âge", "age")];

/// Sans accents : « février » → « fevrier », « août » → « aout ».
pub fn fold(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'à' | 'â' => 'a',
            'û' | 'ù' => 'u',
            'ô' => 'o',
            'î' | 'ï' => 'i',
            'ç' => 'c',
            c => c,
        })
        .collect()
}

fn canonical(field: &str) -> &str {
    ALIASES.iter().find(|(old, _)| *old == field).map_or(field, |(_, new)| new)
}

/// `(n, capitale)` pour `PRENOM_n` (la lettre en capitale) ou `prenom_n` (telle qu'elle s'écrit), n ≥ 1.
pub fn letter_index(field: &str) -> Option<(usize, bool)> {
    let (n, capital) = match field.strip_prefix("PRENOM_") {
        Some(n) => (n, true),
        None => (field.strip_prefix("prenom_")?, false),
    };
    if !n.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let n: usize = n.parse().ok().filter(|&n| n >= 1)?;
    Some((n, capital))
}

/// Le mois (1 à 12) de `anniversaires_mars`, ou de l'ancienne écriture `anniversaires mars`.
pub fn month_of(field: &str) -> Option<usize> {
    let m = field.strip_prefix("anniversaires_").or_else(|| field.strip_prefix("anniversaires "))?;
    class::MONTHS.iter().position(|x| fold(x) == fold(m)).map(|i| i + 1)
}

/// `anniversaires_aout` pour le mois 8.
pub fn month_key(month: usize) -> String {
    format!("anniversaires_{}", fold(class::MONTHS[month - 1]))
}

/// Ce que devient le champ pour l'élève ; `None` pour un champ inconnu ou un champ de toute la classe.
pub fn value(field: &str, s: &Student) -> Option<String> {
    if let Some((n, capital)) = letter_index(field) {
        let c = s.first_name.chars().filter(|c| !c.is_whitespace()).nth(n - 1);
        return Some(c.map(|c| if capital { c.to_uppercase().collect() } else { c.to_string() }).unwrap_or_default());
    }
    match canonical(field) {
        "prenom" => Some(s.first_name.clone()),
        "PRENOM" => Some(s.first_name.to_uppercase()),
        "nom" => Some(s.last_name.clone()),
        "groupe" => Some(s.group.clone()),
        // Sans date de naissance : vide.
        "age" => Some(s.birth_date.as_deref().and_then(|b| class::age_on(b, class::today())).map(|a| format!("{a} ans")).unwrap_or_default()),
        "anniversaire" => Some(s.birth_date.as_deref().and_then(class::birthday_text).unwrap_or_default()),
        _ => None,
    }
}

fn is_student_field(field: &str) -> bool {
    value(field, &Student::new("", "")).is_some()
}

/// Ce qu'on lit à la place d'un champ non rempli : sur la pastille, à l'impression, à l'export.
pub fn label(field: &str) -> Option<String> {
    if let Some((n, capital)) = letter_index(field) {
        return Some(format!("{} {n}", if capital { "Lettre" } else { "lettre" }));
    }
    if let Some(m) = month_of(field) {
        let name = class::MONTHS[m - 1];
        let of = if name.starts_with(['a', 'o']) { "d'" } else { "de " };
        return Some(format!("Anniversaires {of}{name}"));
    }
    Some(
        match canonical(field) {
            "prenom" => "Prénom",
            "PRENOM" => "PRÉNOM",
            "nom" => "Nom",
            "groupe" => "Groupe",
            "age" => "Âge",
            "anniversaire" => "Anniversaire",
            _ => return None,
        }
        .into(),
    )
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Piece<'a> {
    Text(&'a str),
    /// Le nom du champ, sans ses accolades.
    Field(&'a str),
}

/// Le texte découpé en morceaux de texte et en champs reconnus ; des accolades inconnues restent du texte.
pub fn split(text: &str) -> Vec<Piece<'_>> {
    let mut out = Vec::new();
    let (mut start, mut at) = (0, 0);
    while let Some(open) = text[at..].find('{').map(|i| at + i) {
        let Some(close) = text[open + 1..].find('}').map(|i| open + 1 + i) else { break };
        let inner = &text[open + 1..close];
        if is_student_field(inner) || month_of(inner).is_some() {
            if open > start {
                out.push(Piece::Text(&text[start..open]));
            }
            out.push(Piece::Field(inner));
            start = close + 1;
            at = start;
        } else {
            at = open + 1;
        }
    }
    if start < text.len() {
        out.push(Piece::Text(&text[start..]));
    }
    out
}

fn fields_in(t: &TextProps) -> impl Iterator<Item = &str> {
    t.content.paragraphs.iter().flat_map(|p| &p.runs).flat_map(|r| split(&r.text)).filter_map(|p| match p {
        Piece::Field(f) => Some(f),
        Piece::Text(_) => None,
    })
}

/// Le texte a un champ d'élève (lettre comprise).
pub fn has_fields_text(t: &TextProps) -> bool {
    fields_in(t).any(is_student_field)
}

/// Le texte a un champ de toute la classe (anniversaires d'un mois).
pub fn has_class_fields_text(t: &TextProps) -> bool {
    fields_in(t).any(|f| month_of(f).is_some())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::class::Student;

    #[test]
    fn values_and_aliases() {
        let mut s = Student::new("Léa Rose", "Martin");
        s.group = "Loups".into();
        assert_eq!(value("prenom", &s).as_deref(), Some("Léa Rose"));
        assert_eq!(value("PRENOM", &s).as_deref(), Some("LÉA ROSE"));
        assert_eq!(value("prénom", &s), value("prenom", &s));
        assert_eq!(value("PRÉNOM", &s), value("PRENOM", &s));
        assert_eq!(value("PRENOM_4", &s).as_deref(), Some("R"));
        assert_eq!(value("prenom_1", &s).as_deref(), Some("L"));
        assert_eq!(value("prenom_9", &s).as_deref(), Some(""));
        assert_eq!(value("nom", &s).as_deref(), Some("Martin"));
        assert_eq!(value("groupe", &s).as_deref(), Some("Loups"));
        assert_eq!(value("age", &s).as_deref(), Some(""));
        for unknown in ["toto", "prenom_0", "PRENOM_x", "Prénom", "anniversaires_mars"] {
            assert_eq!(value(unknown, &s), None, "{unknown}");
        }
    }

    #[test]
    fn labels() {
        assert_eq!(label("prenom").as_deref(), Some("Prénom"));
        assert_eq!(label("PRENOM").as_deref(), Some("PRÉNOM"));
        assert_eq!(label("age").as_deref(), Some("Âge"));
        assert_eq!(label("PRENOM_2").as_deref(), Some("Lettre 2"));
        assert_eq!(label("prenom_2").as_deref(), Some("lettre 2"));
        assert_eq!(label("anniversaires_avril").as_deref(), Some("Anniversaires d'avril"));
        assert_eq!(label("anniversaires_aout").as_deref(), Some("Anniversaires d'août"));
        assert_eq!(label("anniversaires_mars").as_deref(), Some("Anniversaires de mars"));
        assert_eq!(label("toto"), None);
    }

    #[test]
    fn months_with_or_without_accents() {
        assert_eq!(month_of("anniversaires_fevrier"), Some(2));
        assert_eq!(month_of("anniversaires février"), Some(2));
        assert_eq!(month_of("anniversaires_decembre"), Some(12));
        assert_eq!(month_of("anniversaires_lundi"), None);
        assert_eq!(month_key(8), "anniversaires_aout");
    }

    #[test]
    fn split_keeps_unknown_braces_as_text() {
        assert_eq!(split("Bonjour {prenom}, {toto} !"), [Piece::Text("Bonjour "), Piece::Field("prenom"), Piece::Text(", {toto} !")]);
        assert_eq!(split("{prenom}{age}"), [Piece::Field("prenom"), Piece::Field("age")]);
        assert_eq!(split("{ {prenom}"), [Piece::Text("{ "), Piece::Field("prenom")]);
        assert_eq!(split(""), Vec::<Piece>::new());
    }

    #[test]
    fn letter_indexes() {
        assert_eq!(letter_index("PRENOM_3"), Some((3, true)));
        assert_eq!(letter_index("prenom_12"), Some((12, false)));
        assert_eq!(letter_index("prenom_0"), None);
        assert_eq!(letter_index("prenom"), None);
    }
}
