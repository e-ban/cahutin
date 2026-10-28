//! La configuration garantit l'absence d'accès réseau et une installation sans droits administrateur.

use serde_json::Value;

fn config() -> Value {
    serde_json::from_str(include_str!("../tauri.conf.json")).unwrap()
}

#[test]
fn csp_allows_no_remote_origin() {
    let c = config();
    let csp = c["app"]["security"]["csp"].as_str().expect("CSP définie");
    assert!(!csp.contains("https:"), "{csp}");
    for source in csp.split([' ', ';']).filter(|s| s.starts_with("http://")) {
        let host = source.trim_start_matches("http://");
        assert!(host.ends_with(".localhost") || host == "localhost", "origine distante autorisée : {source}");
    }
}

#[test]
fn csp_lets_user_fonts_load_from_the_asset_protocol() {
    let c = config();
    let csp = c["app"]["security"]["csp"].as_str().unwrap();
    let font_src = csp.split(';').map(str::trim).find(|d| d.starts_with("font-src")).expect("font-src");
    assert!(font_src.contains("carnet-asset:") && font_src.contains("http://carnet-asset.localhost"), "{font_src}");
}

#[test]
fn installer_is_per_user_nsis() {
    let c = config();
    assert_eq!(c["bundle"]["targets"], serde_json::json!(["nsis"]));
    assert_eq!(c["bundle"]["windows"]["nsis"]["installMode"], "currentUser");
}

#[test]
fn no_updater() {
    let c = config();
    assert!(c["plugins"].get("updater").is_none());
    let manifest = include_str!("../Cargo.toml");
    for forbidden in ["tauri-plugin-updater", "reqwest", "tauri-plugin-http"] {
        assert!(!manifest.contains(forbidden), "dépendance réseau : {forbidden}");
    }
}

#[test]
fn html5_drag_and_drop_is_not_captured_by_the_window() {
    let c = config();
    assert_eq!(c["app"]["windows"][0]["dragDropEnabled"], false);
}
