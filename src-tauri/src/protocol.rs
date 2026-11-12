//! Protocole `carnet-asset` : sert les images importées à l'interface.
//! `/<id>` donne l'image (relue depuis le cache), `/<id>/thumb` sa vignette (en mémoire),
//! `/lib/<id>` une photo de la photothèque et `/lib/<id>/thumb` sa vignette (faite si elle manque),
//! `/font/<fichier>` une police ajoutée par l'utilisateur, `/sysfont/<indice>` une police du poste,
//! `/picto/<code>.svg` une image de la banque OpenMoji.

use crate::state::AppState;
use carnet_core::phototheque::thumbs;
use carnet_core::{Mime, fonts};
use std::borrow::Cow;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::OnceLock;
use tauri::http::{Request, Response, StatusCode, header};
use tauri::{AppHandle, Manager, Runtime, UriSchemeContext, UriSchemeResponder};

pub const SCHEME: &str = "carnet-asset";

type Body = Response<Cow<'static, [u8]>>;

/// Une demande pour le fil de travail : le travail qui fait la réponse, et l'envoi de la réponse.
struct Job {
    work: Box<dyn FnOnce() -> Body + Send>,
    reply: Box<dyn FnOnce(Body) + Send>,
}

/// La boucle du fil de travail : chaque demande reçoit une réponse, et le fil survit à une demande qui
/// échoue (un décodeur qui panique sur un fichier abîmé, un verrou empoisonné) ; elle reçoit alors une erreur.
fn run_jobs(jobs: Receiver<Job>) {
    let mut failures = 0u64;
    for Job { work, reply } in jobs {
        let response = catch_unwind(AssertUnwindSafe(work)).unwrap_or_else(|_| {
            failures += 1;
            log::error!("library_job_panicked count={failures}");
            server_error()
        });
        // Une réponse qui ne peut pas partir (page fermée) n'arrête pas le fil non plus.
        let _ = catch_unwind(AssertUnwindSafe(|| reply(response)));
    }
}

/// Fil de travail de la photothèque : lire une photo ou faire sa vignette prend du temps, jamais sur le
/// fil de l'interface. Un seul fil, qui sert les demandes dans l'ordre : jamais des centaines de photos
/// décodées à la fois.
fn library_jobs() -> &'static Sender<Job> {
    static JOBS: OnceLock<Sender<Job>> = OnceLock::new();
    JOBS.get_or_init(|| {
        let (tx, rx) = channel::<Job>();
        std::thread::spawn(move || run_jobs(rx));
        tx
    })
}

fn server_error() -> Body {
    Response::builder().status(StatusCode::INTERNAL_SERVER_ERROR).body(Cow::Borrowed(&[][..])).unwrap()
}

/// Protocole asynchrone : les routes `lib/` sont servies par le fil de travail, les autres tout de suite.
pub fn handle<R: Runtime>(ctx: UriSchemeContext<'_, R>, request: Request<Vec<u8>>, responder: UriSchemeResponder) {
    let path = request.uri().path().trim_start_matches('/');
    if let Some(rest) = path.strip_prefix("lib/") {
        let app = ctx.app_handle().clone();
        let rest = rest.to_string();
        let job = Job { work: Box::new(move || library_response(&app.state::<AppState>(), &rest)), reply: Box::new(move |r| responder.respond(r)) };
        // Le fil de travail ne s'arrête pas (voir `run_jobs`). S'il avait disparu malgré tout, la demande
        // reçoit une erreur : le travail ne se fait jamais sur le fil de l'interface.
        if let Err(e) = library_jobs().send(job) {
            log::error!("library_worker_gone");
            (e.0.reply)(server_error());
        }
        return;
    }
    responder.respond(respond(ctx.app_handle(), path));
}

fn respond<R: Runtime>(app: &AppHandle<R>, path: &str) -> Response<Cow<'static, [u8]>> {
    if let Some(file) = path.strip_prefix("font/") {
        return font_response(&app.state::<AppState>().fonts_dir, file);
    }
    if let Some(file) = path.strip_prefix("picto/") {
        return picto_response(file);
    }
    if let Some(name) = path.strip_prefix("mine/") {
        return my_image_response(&app.state::<AppState>(), name);
    }
    if let Some(index) = path.strip_prefix("sysfont/") {
        return system_font_response(&app.state::<AppState>(), index);
    }
    let (id, thumb) = match path.strip_suffix("/thumb") {
        Some(id) => (id, true),
        None => (path, false),
    };
    let state = app.state::<AppState>();
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

/// `lib/<id>` ou `lib/<id>/thumb` : une photo de la photothèque ouverte, lue dans son dossier suivi, ou
/// sa vignette du cache (faite si elle manque). `no-cache` : la vignette change quand le fichier change.
fn library_response(state: &AppState, rest: &str) -> Response<Cow<'static, [u8]>> {
    let (id, thumb) = rest.strip_suffix("/thumb").map_or((rest, false), |id| (id, true));
    let found = {
        let lib = state.phototheque.lock().unwrap();
        let Some((dir, index)) = lib.as_ref() else { return not_found() };
        index.photos.iter().find(|p| p.id == id).map(|p| (dir.clone(), p.clone()))
    };
    let Some((dir, photo)) = found else { return not_found() };
    let (file, mime) = if thumb {
        let Ok(t) = thumbs::thumb(&dir, &photo) else { return not_found() };
        (t, "image/jpeg")
    } else {
        (PathBuf::from(&photo.path), original_type(&photo.path))
    };
    let Ok(bytes) = std::fs::read(file) else { return not_found() };
    Response::builder().header(header::CONTENT_TYPE, mime).header(header::CACHE_CONTROL, "no-cache").body(Cow::Owned(bytes)).unwrap()
}

/// Le type d'un original d'après l'extension de son fichier : un WebP est noté JPEG dans l'index (son
/// format une fois posé dans un document), mais son fichier reste un WebP.
fn original_type(path: &str) -> &'static str {
    match Path::new(path).extension().and_then(|e| e.to_str()).map(str::to_ascii_lowercase).as_deref() {
        Some("png") => "image/png",
        Some("webp") => "image/webp",
        _ => "image/jpeg",
    }
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

    /// Une photothèque ouverte qui suit un dossier d'un JPEG 800 x 600 et d'un WebP.
    fn library() -> (tempfile::TempDir, AppState, std::path::PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let folder = dir.path().join("Sortie");
        std::fs::create_dir_all(&folder).unwrap();
        image::RgbImage::from_pixel(800, 600, image::Rgb([10, 120, 200])).save(folder.join("a.jpg")).unwrap();
        image::RgbImage::from_pixel(40, 30, image::Rgb([200, 120, 10])).save(folder.join("b.webp")).unwrap();
        let library = dir.path().join("Classe");
        let mut lib = carnet_core::phototheque::Phototheque { format_version: 2, folders: vec![], photos: vec![] };
        carnet_core::phototheque::follow(&mut lib, &folder).unwrap();
        carnet_core::phototheque::scan(&mut lib, &library, &mut |_| {});
        let state = AppState::new(Default::default(), carnet_core::templates::Library::open(dir.path().join("m")).unwrap(), dir.path().join("p"), dir.path().join("c"), dir.path().join("s"));
        *state.phototheque.lock().unwrap() = Some((library.clone(), lib));
        (dir, state, library)
    }

    fn id_of(state: &AppState, name: &str) -> String {
        let lib = state.phototheque.lock().unwrap();
        lib.as_ref().unwrap().1.photos.iter().find(|p| p.path.ends_with(name)).unwrap().id.clone()
    }

    #[test]
    fn a_library_thumbnail_is_made_on_demand_then_served_from_the_cache() {
        let (_dir, state, library) = library();
        let id = id_of(&state, "a.jpg");
        let r = library_response(&state, &format!("{id}/thumb"));
        assert_eq!(r.status(), StatusCode::OK);
        assert_eq!(r.headers()[header::CONTENT_TYPE], "image/jpeg");
        assert_eq!(r.headers()[header::CACHE_CONTROL], "no-cache");
        let img = image::load_from_memory(r.body()).unwrap();
        assert_eq!((img.width(), img.height()), (256, 192));
        assert_eq!(std::fs::read_dir(library.join(".vignettes")).unwrap().count(), 1, "gardée dans le cache");
        assert_eq!(library_response(&state, "inconnue/thumb").status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn a_library_original_has_the_type_of_its_file() {
        let (_dir, state, _library) = library();
        let r = library_response(&state, &id_of(&state, "a.jpg"));
        assert_eq!((r.headers()[header::CONTENT_TYPE].to_str().unwrap(), r.headers()[header::CACHE_CONTROL].to_str().unwrap()), ("image/jpeg", "no-cache"));
        // Un WebP est noté JPEG dans l'index (son format une fois posé dans un document) : servi en WebP.
        let r = library_response(&state, &id_of(&state, "b.webp"));
        assert_eq!(r.headers()[header::CONTENT_TYPE], "image/webp");
        assert!(r.body().starts_with(b"RIFF"));
        assert_eq!(library_response(&state, "inconnue").status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn a_job_that_panics_gets_an_error_and_the_worker_keeps_serving() {
        let (jobs, rx) = channel::<Job>();
        std::thread::spawn(move || run_jobs(rx));
        let (replies, answers) = channel::<StatusCode>();
        let send = |work: Box<dyn FnOnce() -> Body + Send>| {
            let replies = replies.clone();
            jobs.send(Job { work, reply: Box::new(move |r| replies.send(r.status()).unwrap()) }).unwrap();
        };
        send(Box::new(|| panic!("décodeur en panne")));
        send(Box::new(not_found));
        let wait = std::time::Duration::from_secs(5);
        assert_eq!(answers.recv_timeout(wait), Ok(StatusCode::INTERNAL_SERVER_ERROR), "la demande qui a échoué reçoit une erreur");
        assert_eq!(answers.recv_timeout(wait), Ok(StatusCode::NOT_FOUND), "la suivante est encore servie");
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
