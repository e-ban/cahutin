//! Rapport de problème : un fichier texte que l'utilisateur envoie lui-même, joint à un mail que sa
//! messagerie ouvre déjà rempli. Cahutin n'envoie rien.

/// Adresse dédiée aux signalements.
pub const REPORT_EMAIL: &str = "cahutin.proofread153@simplelogin.com";

/// Le journal ne contient que des noms d'événements et des compteurs (voir la journalisation de l'application).
pub fn problem_report(app_version: &str, os: &str, generated_at: &str, log_text: &str, max_lines: usize) -> String {
    let lines: Vec<&str> = log_text.lines().collect();
    let tail = &lines[lines.len().saturating_sub(max_lines)..];
    format!(
        "Rapport de problème\nVersion : {app_version}\nSystème : {os}\nDate : {generated_at}\n\n--- Journal ({} dernières lignes) ---\n{}\n",
        tail.len(),
        tail.join("\n")
    )
}

/// Encodage d'un lien `mailto:` : seuls les caractères sans réserve restent tels quels ; un retour à la
/// ligne devient CRLF, comme le veut un mail.
fn mailto_encode(text: &str) -> String {
    let mut out = String::new();
    for c in text.chars() {
        if c.is_ascii_alphanumeric() || "-_.~".contains(c) {
            out.push(c);
        } else if c == '\n' {
            out.push_str("%0D%0A");
        } else {
            let mut buf = [0u8; 4];
            out.extend(c.encode_utf8(&mut buf).bytes().map(|b| format!("%{b:02X}")));
        }
    }
    out
}

/// Mail court à l'adresse des signalements, à relire puis envoyer : la place pour raconter, la version, le
/// système, et le nom du rapport à joindre (un lien `mailto:` ne peut pas joindre de fichier).
pub fn report_mailto(app_version: &str, os: &str, report_file: &str) -> String {
    let subject = format!("Cahutin {app_version} : signalement d'un problème");
    let body = format!(
        "Bonjour,\n\nVoici ce qui s'est passé (ce que je faisais, ce que j'attendais) :\n\n\n\nPièce jointe : le rapport « {report_file} », que Cahutin vient d'enregistrer. Il ne contient ni photo, ni texte de document, ni nom de fichier.\n\nCahutin {app_version}, {os}\n"
    );
    format!("mailto:{REPORT_EMAIL}?subject={}&body={}", mailto_encode(&subject), mailto_encode(&body))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn report_has_header_fields() {
        let r = problem_report("0.1.0", "Windows 11 (26100)", "2026-10-27 22:00", "ligne", 10);
        assert!(r.starts_with("Rapport de problème\nVersion : 0.1.0\nSystème : Windows 11 (26100)\nDate : 2026-10-27 22:00\n\n"));
    }

    #[test]
    fn report_keeps_only_last_lines() {
        let log: String = (1..=10).map(|i| format!("ligne {i}\n")).collect();
        let r = problem_report("0.1.0", "Linux", "maintenant", &log, 3);
        assert!(r.contains("--- Journal (3 dernières lignes) ---\nligne 8\nligne 9\nligne 10"));
        assert!(!r.contains("ligne 7"));
    }

    /// Un mail court, prêt à relire : l'adresse dédiée, un objet, la version et le système, et le nom du
    /// rapport à joindre ; rien d'autre (ni journal, ni chemin).
    #[test]
    fn the_report_mail_is_short_and_names_the_file_to_attach() {
        let url = report_mailto("0.13.0", "windows x86_64", "rapport-carnet.txt");
        assert!(url.starts_with("mailto:cahutin.proofread153@simplelogin.com?subject=Cahutin%200.13.0%20%3A%20signalement%20d%27un%20probl%C3%A8me&body="), "{url}");
        assert!(url.contains("rapport-carnet.txt"), "{url}");
        assert!(url.contains("windows%20x86_64"), "{url}");
        assert!(url.contains("%0D%0A"), "retours à la ligne CRLF : {url}");
        assert!(!url.contains(' ') && !url.contains('\n'));
        assert!(url.len() < 1800);
    }
}
