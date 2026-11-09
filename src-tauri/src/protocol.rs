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
    if let Some(name) = path.strip_prefix("mine/") {
        return my_image_response(&ctx.app_handle().state::<AppState>(), name);
    }
    if let Some(rest) = path.strip_prefix("lib/") {
        return library_response(&ctx.app_handle().state::<AppState>(), rest);
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

/// `lib/<id>` ou `lib/<id>/thumb` : une photo de la photothèque ouverte, lue dans son dossier.
fn library_response(state: &AppState, rest: &str) -> Response<Cow<'static, [u8]>> {
    let (id, thumb) = rest.strip_suffix("/thumb").map_or((rest, false), |id| (id, true));
    let file = {
        let lib = state.phototheque.lock().unwrap();
        let Some((dir, index)) = lib.as_ref() else { return not_found() };
        index.photos.iter().find(|p| p.id == id).and_then(|p| carnet_core::phototheque::file_of(dir, &p.id, p.mime, thumb).map(|f| (f, p.mime)))
    };
    let Some((file, mime)) = file else { return not_found() };
    let Ok(bytes) = std::fs::read(file) else { return not_found() };
    let mime = match mime {
        Mime::Jpeg => "image/jpeg",
        Mime::Png => "image/png",
    };
    Response::builder().header(header::CONTENT_TYPE, mime).header(header::CACHE_CONTROL, "max-age=31536000, immutable").body(Cow::Owned(bytes)).unwrap()
}

/// `mine/<nom>` : une image de « Mes images », telle quelle (le nom vient de l'adresse, encodé).
fn my_image_response(state: &AppState, name: &str) -> Response<Cow<'static, [u8]>> {
    let Ok(name) = percent_decode(name) else { return not_found() };
    // Une image retirée se montre aussi (pour la remettre) : seul ce sous-dossier est permis.
    let dir = crate::images_commands::images_dir(state);
    let (dir, name) = match name.strip_prefix(crate::images_commands::REMOVED).and_then(|r| r.strip_prefix('/')) {
        Some(rest) => (dir.join(crate::images_commands::REMOVED), rest.to_string()),
        None => (dir, name),
    };
    if !crate::images_commands::is_image_name(&name) {
        return not_found();
    }
    let Ok(bytes) = std::fs::read(dir.join(&name)) else { return not_found() };
    let mime = match name.rsplit('.').next().map(str::to_ascii_lowercase).as_deref() {
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        Some("webp") => "image/webp",
        _ => "image/jpeg",
    };
    Response::builder().header(header::CONTENT_TYPE, mime).header(header::CACHE_CONTROL, "no-cache").body(Cow::Owned(bytes)).unwrap()
}

/// Décodage des « %XX » d'une adresse (noms avec espaces ou accents).
fn percent_decode(s: &str) -> Result<String, ()> {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' {
            let hex = s.get(i + 1..i + 3).ok_or(())?;
            out.push(u8::from_str_radix(hex, 16).map_err(|_| ())?);
            i += 3;
        } else {
            out.push(b[i]);
            i += 1;
        }
    }
    String::from_utf8(out).map_err(|_| ())
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

/// `picto/<code>.svg` : l'image en couleur ; `picto/contour/<code>.svg` : son contour noir, à colorier.
fn picto_response(file: &str) -> Response<Cow<'static, [u8]>> {
    let (file, outline) = match file.strip_prefix("contour/") {
        Some(f) => (f, true),
        None => (file, false),
    };
    let code = file.strip_suffix(".svg").filter(|c| !c.is_empty() && c.chars().all(|ch| ch.is_ascii_digit() || ch.is_ascii_uppercase() || ch == '-'));
    let svg = |c: &str| if outline { carnet_core::picto::outline_svg(c) } else { carnet_core::picto::svg(c) };
    match code.and_then(svg) {
        Some(bytes) => Response::builder()
            .header(header::CONTENT_TYPE, "image/svg+xml")
            .header(header::CACHE_CONTROL, "max-age=31536000, immutable")
            .body(Cow::Owned(bytes))
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

    /// Les noms de « Mes images » arrivent encodés ; un chemin caché dans l'encodage est refusé ensuite.
    #[test]
    fn my_image_names_are_decoded() {
        assert_eq!(percent_decode("Citrouille%20d'automne%20%C3%A9t%C3%A9.svg").as_deref(), Ok("Citrouille d'automne été.svg"));
        assert!(percent_decode("a%2").is_err());
        assert!(percent_decode("a%zz.png").is_err());
        let hidden = percent_decode("..%2F..%2Fsecret.png").unwrap();
        assert!(!crate::images_commands::is_image_name(&hidden));
    }

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
        let state = AppState::new(Default::default(), carnet_core::templates::Library::open(dir.path().join("m")).unwrap(), dir.path().join("p"), dir.path().join("c"), dir.path().join("s"));
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
        // Le contour à colorier ; « contour/ » une seule fois, et pas d'autre dossier.
        assert_eq!(picto_response("contour/1F98A.svg").body().as_ref(), carnet_core::picto::outline_svg("1F98A").unwrap());
        assert_eq!(picto_response("contour/1F9D2-1F3FD.svg").status(), StatusCode::OK);
        assert_eq!(picto_response("contour/contour/1F98A.svg").status(), StatusCode::NOT_FOUND);
        assert_eq!(picto_response("contour/../1F98A.svg").status(), StatusCode::NOT_FOUND);
    }
}
