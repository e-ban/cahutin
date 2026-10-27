//! Protocole `carnet-asset` : sert les images importées à l'interface, sans passer par le disque.
//! `/<id>` donne l'image, `/<id>/thumb` sa vignette.

use crate::state::AppState;
use carnet_core::Mime;
use std::borrow::Cow;
use tauri::http::{Request, Response, StatusCode, header};
use tauri::{Manager, Runtime, UriSchemeContext};

pub const SCHEME: &str = "carnet-asset";

pub fn handle<R: Runtime>(ctx: UriSchemeContext<'_, R>, request: Request<Vec<u8>>) -> Response<Cow<'static, [u8]>> {
    let path = request.uri().path().trim_start_matches('/');
    let (id, thumb) = match path.strip_suffix("/thumb") {
        Some(id) => (id, true),
        None => (path, false),
    };
    let state = ctx.app_handle().state::<AppState>();
    let store = state.store.lock().unwrap();
    let Some(asset) = store.get(id) else {
        return Response::builder().status(StatusCode::NOT_FOUND).body(Cow::Borrowed(&[][..])).unwrap();
    };
    let bytes = if thumb { asset.thumb.clone() } else { asset.bytes.clone() };
    let mime = match asset.meta.mime {
        Mime::Jpeg => "image/jpeg",
        Mime::Png => "image/png",
    };
    Response::builder()
        .header(header::CONTENT_TYPE, mime)
        .header(header::CACHE_CONTROL, "max-age=31536000, immutable")
        .body(Cow::Owned(bytes.to_vec()))
        .unwrap()
}
