//! Rapport de problème : un fichier texte que l'utilisateur envoie lui-même.

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
}
