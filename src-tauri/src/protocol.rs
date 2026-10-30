//! Protocole `carnet-asset` : sert les images importées à l'interface.
//! `/<id>` donne l'image (relue depuis le cache), `/<id>/thumb` sa vignette (en mémoire),
//! `/font/<fichier>` une police ajoutée par l'utilisateur, `/sysfont/<indice>` une police du poste,
//! `/picto/<code>.svg` une image de la banque OpenMoji.

use crate::state::AppState;
use carnet_core::{Mime, fonts};
use std::borrow::Cow;
use std::path::Path;
use tauri::http::{Request, Response, StatusCode, header};
use tauri::{Manager, Runtime, UriSchemeContext};

pub const SCHEME: &str = "carnet-asset";

pub fn handle<R: Runtime>(ctx: UriSchemeContext<'_, R>, request: Request<Vec<u8>>) -> Response<Cow<'static, [u8]>> {
    let path = request.uri().path().trim_start_matches('/');
    if let Some(file) = path.strip_prefix("font/") {
        return font_response(&ctx.app_handle().state::<AppState>().fonts_dir, file);
    }
    if let Some(file) = path.strip_prefix("picto/") {
        return picto_response(file);
    }
    if let Some(index) = path.strip_prefix("sysfont/") {
        return system_font_response(&ctx.app_handle().state::<AppState>(), index);
    }
    let (id, thumb) = match path.strip_suffix("/thumb") {
        Some(id) => (id, true),
        None => (path, false),
    };
    let state = ctx.app_handle().state::<AppState>();
    let store = state.store.lock().unwrap();
    // La lecture du disque se fait après avoir rendu le verrou du magasin.
    let found = if thumb {
        store.thumb(id)
    } else {
        let source = store.locate(id);
        drop(store);
        source.and_then(|s| s.load()).map(|a| (a.bytes, a.meta.mime))
    };
    let Some((bytes, mime)) = found else {
        return Response::builder().status(StatusCode::NOT_FOUND).body(Cow::Borrowed(&[][..])).unwrap();
    };
    let mime = match mime {
        Mime::Jpeg => "image/jpeg",
        Mime::Png => "image/png",
    };
    Response::builder()
        .header(header::CONTENT_TYPE, mime)
        .header(header::CACHE_CONTROL, "max-age=31536000, immutable")
        .body(Cow::Owned(bytes.to_vec()))
        .unwrap()
}

fn not_found() -> Response<Cow<'static, [u8]>> {
    Response::builder().status(StatusCode::NOT_FOUND).body(Cow::Borrowed(&[][..])).unwrap()
}

fn font_response(dir: &Path, file: &str) -> Response<Cow<'static, [u8]>> {
    if !fonts::is_safe_file_name(file) {
        return not_found();
    }
    font_file_response(&dir.join(file))
}

/// Seul un indice de la liste des polices du poste est accepté : jamais un chemin venu de la page.
fn system_font_response(state: &AppState, index: &str) -> Response<Cow<'static, [u8]>> {
    match index.parse::<usize>().ok().and_then(|i| state.system_font_path(i)) {
        Some(path) => font_file_response(&path),
        None => not_found(),
    }
}

fn picto_response(file: &str) -> Response<Cow<'static, [u8]>> {
    let code = file.strip_suffix(".svg").filter(|c| !c.is_empty() && c.chars().all(|ch| ch.is_ascii_digit() || ch.is_ascii_uppercase() || ch == '-'));
    match code.and_then(carnet_core::picto::svg) {
        Some(bytes) => Response::builder()
            .header(header::CONTENT_TYPE, "image/svg+xml")
            .header(header::CACHE_CONTROL, "max-age=31536000, immutable")
            .body(Cow::Borrowed(bytes))
            .unwrap(),
        None => not_found(),
    }
}

fn font_file_response(path: &Path) -> Response<Cow<'static, [u8]>> {
    let Ok(bytes) = std::fs::read(path) else { return not_found() };
    let mime = if bytes.starts_with(b"OTTO") { "font/otf" } else { "font/ttf" };
    Response::builder()
        .header(header::CONTENT_TYPE, mime)
        .header(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")
        .body(Cow::Owned(bytes))
        .unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Sous Windows, la page (http://tauri.localhost) charge les polices depuis http://carnet-asset.localhost :
    /// une requête de police d'une autre origine exige l'en-tête CORS.
    #[test]
    fn font_response_allows_cross_origin_loading() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("Essai.ttf"), b"\0\x01\0\0").unwrap();
        let r = font_response(dir.path(), "Essai.ttf");
        assert_eq!(r.status(), StatusCode::OK);
        assert_eq!(r.headers()[header::ACCESS_CONTROL_ALLOW_ORIGIN], "*");
        assert_eq!(font_response(dir.path(), "../secret.ttf").status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn system_font_is_served_by_index_only() {
        let dir = tempfile::tempdir().unwrap();
        let sys = dir.path().join("systeme");
        std::fs::create_dir(&sys).unwrap();
        let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("../crates/carnet-core/tests/fixtures/CarnetTest-Regular.ttf");
        std::fs::copy(fixture, sys.join("carnettest.ttf")).unwrap();
        let state = AppState::new(Default::default(), carnet_core::templates::Library::open(dir.path().join("m")).unwrap(), dir.path().join("p"), dir.path().join("c"));
        state.set_system_fonts(fonts::scan_font_dirs(&[sys.as_path()]));
        let r = system_font_response(&state, "0");
        assert_eq!(r.status(), StatusCode::OK);
        assert_eq!(r.headers()[header::ACCESS_CONTROL_ALLOW_ORIGIN], "*");
        assert_eq!(system_font_response(&state, "1").status(), StatusCode::NOT_FOUND);
        assert_eq!(system_font_response(&state, "../x").status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn picto_route_serves_svg_and_refuses_paths() {
        let r = picto_response("1F98A.svg");
        assert_eq!(r.status(), StatusCode::OK);
        assert_eq!(r.headers()[header::CONTENT_TYPE], "image/svg+xml");
        assert_eq!(picto_response("1f98a.svg").status(), StatusCode::NOT_FOUND);
        assert_eq!(picto_response("../1F98A.svg").status(), StatusCode::NOT_FOUND);
        assert_eq!(picto_response("1F98A").status(), StatusCode::NOT_FOUND);
    }
}
