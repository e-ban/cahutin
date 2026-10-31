//! Avis de nouvelle version : lit le numéro de la dernière version publiée sur GitHub, seulement si
//! l'utilisateur l'a accepté (l'interface décide d'appeler). Rien d'autre ne part : ni identifiant, ni
//! document, ni information sur le poste.

/// Version du Microsoft Store (construite avec `CAHUTIN_STORE=1`) : le Store fait les mises à jour, Cahutin
/// ne consulte jamais GitHub et ne propose pas l'avis de nouvelle version.
pub const STORE_BUILD: bool = option_env!("CAHUTIN_STORE").is_some();

/// Page ouverte par « Télécharger » : fixe, jamais une adresse lue dans la réponse du réseau.
pub const DOWNLOAD_PAGE: &str = "https://github.com/e-ban/cahutin/releases/latest";

/// Dernière version publiée (les versions en préparation n'y figurent pas).
const LATEST_RELEASE_API: &str = "https://api.github.com/repos/e-ban/cahutin/releases/latest";

/// Réponse brute de l'API, ou rien : hors ligne, proxy, pare-feu, réponse trop longue. Certificats du
/// système, pour les réseaux d'établissement qui inspectent les connexions chiffrées.
pub fn fetch_latest_release() -> Option<String> {
    use std::time::Duration;
    use ureq::tls::{RootCerts, TlsConfig};
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(10)))
        .user_agent("Cahutin")
        .tls_config(TlsConfig::builder().root_certs(RootCerts::PlatformVerifier).build())
        .build()
        .into();
    let response = agent.get(LATEST_RELEASE_API).call().ok()?;
    response.into_body().with_config().limit(1_000_000).read_to_string().ok()
}

/// Numéro `X.Y.Z`, avec ou sans `v` devant ; tout autre forme (suffixe, morceau manquant) est refusée.
pub fn parse_version(s: &str) -> Option<[u64; 3]> {
    let mut parts = s.strip_prefix('v').unwrap_or(s).split('.');
    let mut out = [0; 3];
    for slot in &mut out {
        let part = parts.next()?;
        if part.is_empty() || !part.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        *slot = part.parse().ok()?;
    }
    parts.next().is_none().then_some(out)
}

/// Version annoncée par la réponse de l'API (`tag_name`), si elle est plus récente que `current`.
pub fn newer_version(release_json: &str, current: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(release_json).ok()?;
    let latest = parse_version(value.get("tag_name")?.as_str()?)?;
    (latest > parse_version(current)?).then(|| latest.map(|n| n.to_string()).join("."))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions_are_read_with_or_without_v() {
        assert_eq!(parse_version("v0.5.0"), Some([0, 5, 0]));
        assert_eq!(parse_version("1.12.3"), Some([1, 12, 3]));
    }

    #[test]
    fn malformed_versions_are_refused() {
        for s in ["", "v", "1.2", "1.2.3.4", "1.2.3-beta", "1..3", "v1.2.x", "+1.2.3", " 1.2.3"] {
            assert_eq!(parse_version(s), None, "{s:?}");
        }
    }

    fn release(tag: &str) -> String {
        format!(r#"{{"tag_name":"{tag}","html_url":"https://example.com/piege","name":"Cahutin"}}"#)
    }

    #[test]
    fn a_newer_release_is_announced() {
        assert_eq!(newer_version(&release("v0.6.0"), "0.5.0"), Some("0.6.0".into()));
        assert_eq!(newer_version(&release("v1.0.0"), "0.9.9"), Some("1.0.0".into()));
    }

    /// Comparaison par nombres : 0.10 vient après 0.9, ce qu'une comparaison de textes manquerait.
    #[test]
    fn versions_compare_as_numbers() {
        assert_eq!(newer_version(&release("v0.10.0"), "0.9.0"), Some("0.10.0".into()));
        assert_eq!(newer_version(&release("v0.9.0"), "0.10.0"), None);
    }

    #[test]
    fn the_same_or_an_older_release_is_not_announced() {
        assert_eq!(newer_version(&release("v0.5.0"), "0.5.0"), None);
        assert_eq!(newer_version(&release("v0.4.2"), "0.5.0"), None);
    }

    #[test]
    fn an_unreadable_answer_announces_nothing() {
        for body in ["", "pas du json", "{}", r#"{"tag_name":42}"#, &release("v0.6.0-rc1"), r#"{"message":"Not Found"}"#] {
            assert_eq!(newer_version(body, "0.5.0"), None, "{body:?}");
        }
    }
}
