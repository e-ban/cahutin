//! Import des photos : orientation appliquée, réduction, métadonnées retirées.

use crate::model::{AssetMeta, Id, Mime, new_id};
use image::codecs::jpeg::JpegEncoder;
use image::codecs::png::PngEncoder;
use image::imageops::FilterType;
use image::{DynamicImage, ImageDecoder, ImageReader};
use std::collections::HashMap;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub const MAX_LONG_EDGE_PX: u32 = 3000;
pub const THUMB_LONG_EDGE_PX: u32 = 256;
pub const JPEG_QUALITY: u8 = 85;
/// Plus grand côté d'une image à décoder : bien au-delà d'un appareil photo, assez bas pour qu'une
/// image piégée (dimensions énormes annoncées) soit refusée avant d'allouer sa mémoire.
pub const MAX_DECODE_PX: u32 = 12_000;
/// Mémoire que le décodage d'une image peut demander.
const MAX_DECODE_ALLOC: u64 = 256 * 1024 * 1024;

const IMPORTABLE_EXTENSIONS: [&str; 7] = ["jpg", "jpeg", "png", "webp", "svg", "heic", "heif"];

/// Dessin SVG : rendu en image de ce côté (le plus grand), puis traité comme une image.
const SVG_LONG_EDGE_PX: f32 = 2000.0;
/// Plus gros dessin SVG accepté : un dessin téléchargé fait quelques centaines de Ko.
const MAX_SVG_BYTES: usize = 20 * 1024 * 1024;

/// Une image importée. `thumb` est au même format que `bytes`.
#[derive(Clone, Debug)]
pub struct Asset {
    pub meta: AssetMeta,
    pub bytes: Arc<Vec<u8>>,
    pub thumb: Arc<Vec<u8>>,
}

/// Où sont gardés les octets complets d'une image.
#[derive(Clone)]
enum Storage {
    Memory(Arc<Vec<u8>>),
    Disk(PathBuf),
}

#[derive(Clone)]
struct Entry {
    meta: AssetMeta,
    thumb: Arc<Vec<u8>>,
    storage: Storage,
}

/// Image prête à entrer dans le magasin : déjà écrite dans le cache s'il y en a un.
pub struct StagedAsset(Entry);

impl StagedAsset {
    pub fn meta(&self) -> &AssetMeta {
        &self.0.meta
    }
}

/// Où relire une image, sans garder le magasin (et son verrou) pendant la lecture du disque.
pub struct AssetSource(Entry);

impl AssetSource {
    pub fn load(self) -> Option<Asset> {
        let Entry { meta, thumb, storage } = self.0;
        let bytes = match storage {
            Storage::Memory(b) => b,
            Storage::Disk(path) => Arc::new(std::fs::read(path).ok()?),
        };
        Some(Asset { meta, bytes, thumb })
    }
}

/// Écrit l'image dans le dossier de cache (s'il y en a un) ; à faire hors du verrou du magasin.
pub fn stage(cache_dir: Option<&Path>, a: Asset) -> Result<StagedAsset, ImportError> {
    let storage = match cache_dir {
        Some(dir) => {
            if !crate::model::is_safe_id(&a.meta.id) {
                return Err(ImportError::Storage);
            }
            let path = dir.join(format!("{}.{}", a.meta.id, extension(a.meta.mime)));
            std::fs::write(&path, &*a.bytes).map_err(|_| ImportError::Storage)?;
            Storage::Disk(path)
        }
        None => Storage::Memory(a.bytes),
    };
    Ok(StagedAsset(Entry { meta: a.meta, thumb: a.thumb, storage }))
}

/// Images de la session. Avec un dossier de cache, seules les vignettes restent en mémoire.
#[derive(Default)]
pub struct AssetStore {
    entries: HashMap<Id, Entry>,
    cache_dir: Option<PathBuf>,
}

impl AssetStore {
    pub fn with_cache_dir(dir: PathBuf) -> std::io::Result<AssetStore> {
        std::fs::create_dir_all(&dir)?;
        Ok(AssetStore { entries: HashMap::new(), cache_dir: Some(dir) })
    }

    pub fn cache_dir(&self) -> Option<PathBuf> {
        self.cache_dir.clone()
    }

    pub fn commit(&mut self, s: StagedAsset) {
        self.entries.insert(s.0.meta.id.clone(), s.0);
    }

    pub fn insert(&mut self, a: Asset) -> Result<(), ImportError> {
        let staged = stage(self.cache_dir.as_deref(), a)?;
        self.commit(staged);
        Ok(())
    }

    pub fn locate(&self, id: &str) -> Option<AssetSource> {
        self.entries.get(id).cloned().map(AssetSource)
    }

    pub fn get(&self, id: &str) -> Option<Asset> {
        self.locate(id)?.load()
    }

    pub fn thumb(&self, id: &str) -> Option<(Arc<Vec<u8>>, Mime)> {
        self.entries.get(id).map(|e| (e.thumb.clone(), e.meta.mime))
    }
}

fn extension(mime: Mime) -> &'static str {
    match mime {
        Mime::Jpeg => "jpg",
        Mime::Png => "png",
    }
}

/// Supprime les fichiers du dossier de cache ; les erreurs sont ignorées.
pub fn clear_cache_dir(dir: &Path) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for e in entries.filter_map(|e| e.ok()) {
        let _ = std::fs::remove_file(e.path());
    }
}

/// Dossier de cache d'une instance de l'application, sous `root` : `root/<id>/`, verrouillé par
/// `root/<id>.lock` tant que l'instance vit. Une seconde instance ne touche jamais au dossier d'une autre.
pub struct CacheDir {
    path: PathBuf,
    lock_path: PathBuf,
    _lock: std::fs::File,
}

impl CacheDir {
    /// Vide d'abord les dossiers des instances terminées (verrou libre) et les fichiers d'anciennes versions.
    pub fn acquire(root: &Path) -> std::io::Result<CacheDir> {
        std::fs::create_dir_all(root)?;
        clear_stale(root);
        let id = new_id();
        let lock_path = root.join(format!("{id}.lock"));
        let lock = std::fs::File::create(&lock_path)?;
        lock.try_lock().map_err(std::io::Error::other)?;
        let path = root.join(&id);
        std::fs::create_dir_all(&path)?;
        Ok(CacheDir { path, lock_path, _lock: lock })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// À la fermeture : supprime les images et le dossier ; les erreurs sont ignorées.
    pub fn clear(&self) {
        clear_cache_dir(&self.path);
        let _ = std::fs::remove_dir(&self.path);
        let _ = std::fs::remove_file(&self.lock_path);
    }
}

fn clear_stale(root: &Path) {
    let Ok(entries) = std::fs::read_dir(root) else { return };
    for p in entries.filter_map(|e| e.ok()).map(|e| e.path()) {
        if p.extension().is_some_and(|e| e == "lock") {
            let free = std::fs::OpenOptions::new().write(true).open(&p).is_ok_and(|f| f.try_lock().is_ok());
            if free {
                let dir = p.with_extension("");
                clear_cache_dir(&dir);
                let _ = std::fs::remove_dir(&dir);
                let _ = std::fs::remove_file(&p);
            }
        } else if p.is_file() {
            let _ = std::fs::remove_file(&p);
        }
    }
}

/// Vrai si les octets ressemblent à une image que `import_bytes` sait lire (sans la décoder) : de quoi choisir
/// entre deux exemplaires d'une même image, un dessin WMF et sa copie PNG.
pub fn looks_importable(data: &[u8]) -> bool {
    !is_heic(data) && (is_svg(data) || image::guess_format(data).is_ok())
}

/// Vrai si l'image tournée selon son étiquette `orientation` va au moins aussi bien au cadre (largeur sur hauteur
/// `aspect`) que l'image brute.
fn oriented_fits(w: u32, h: u32, orientation: image::metadata::Orientation, aspect: f64) -> bool {
    use image::metadata::Orientation::*;
    let raw = w as f64 / h as f64;
    let turned = if matches!(orientation, Rotate90 | Rotate270 | Rotate90FlipH | Rotate270FlipH) { 1.0 / raw } else { raw };
    (turned.ln() - aspect.ln()).abs() <= (raw.ln() - aspect.ln()).abs() + 1e-9
}

/// Importe des images en mémoire sur `threads` fils, dans l'ordre ; avec les proportions de leur cadre, s'il y en
/// a un (voir `import_with`).
pub fn import_bytes_many(blobs: &[(Vec<u8>, Option<f64>)], threads: usize) -> Vec<Result<Asset, ImportError>> {
    let next = std::sync::atomic::AtomicUsize::new(0);
    let out: Vec<std::sync::Mutex<Option<Result<Asset, ImportError>>>> = blobs.iter().map(|_| std::sync::Mutex::new(None)).collect();
    std::thread::scope(|s| {
        for _ in 0..threads.max(1).min(blobs.len()) {
            s.spawn(|| {
                loop {
                    let i = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    let Some((b, aspect)) = blobs.get(i) else { break };
                    *out[i].lock().unwrap() = Some(import_with(b, *aspect));
                }
            });
        }
    });
    out.into_iter().map(|m| m.into_inner().unwrap().unwrap_or(Err(ImportError::Decode))).collect()
}

/// Importe les fichiers sur `threads` fils ; `on_each` est appelé une fois par fichier, avec son indice, dès qu'il est prêt.
pub fn import_many(paths: &[PathBuf], threads: usize, on_each: impl Fn(usize, Result<Asset, ImportError>) + Sync) {
    let next = std::sync::atomic::AtomicUsize::new(0);
    std::thread::scope(|s| {
        for _ in 0..threads.max(1).min(paths.len()) {
            s.spawn(|| {
                loop {
                    let i = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    let Some(p) = paths.get(i) else { break };
                    on_each(i, import_path(p));
                }
            });
        }
    });
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImportError {
    Heic,
    Unsupported,
    Decode,
    Io,
    /// L'image n'a pas pu être écrite dans le cache.
    Storage,
    /// Plus de `MAX_DECODE_PX` de côté, ou trop de mémoire à décoder.
    TooLarge,
}

impl ImportError {
    pub fn message_fr(&self) -> String {
        match self {
            ImportError::Heic => "format HEIC (iPhone) non pris en charge : enregistrez la photo en JPEG",
            ImportError::Unsupported => "format d'image non pris en charge (JPEG, PNG, WebP ou dessin SVG attendu)",
            ImportError::Decode => "fichier illisible ou abîmé",
            ImportError::Io => "impossible de lire le fichier",
            ImportError::Storage => "impossible d'enregistrer la photo sur le disque (disque plein ?)",
            ImportError::TooLarge => "image trop grande (plus de 12 000 pixels de côté)",
        }
        .to_string()
    }
}

fn is_heic(data: &[u8]) -> bool {
    data.len() >= 12 && &data[4..8] == b"ftyp" && matches!(&data[8..12], b"heic" | b"heix" | b"mif1" | b"msf1" | b"hevc")
}

/// Un dessin SVG : la balise `<svg` dans le début du fichier (après un éventuel prologue XML).
fn is_svg(data: &[u8]) -> bool {
    let head = String::from_utf8_lossy(&data[..data.len().min(2048)]).to_ascii_lowercase();
    head.trim_start_matches('\u{feff}').trim_start().starts_with('<') && head.contains("<svg")
}

/// Un dessin SVG rendu en image transparente, `SVG_LONG_EDGE_PX` sur son plus grand côté. Un fichier
/// téléchargé n'est pas sûr : aucune ressource extérieure n'est lue (ni fichier, ni adresse, ni image
/// incluse), et le texte n'est pas rendu (pas de polices chargées).
fn render_svg(data: &[u8]) -> Result<DynamicImage, ImportError> {
    if data.len() > MAX_SVG_BYTES {
        return Err(ImportError::Decode);
    }
    let options = resvg::usvg::Options {
        resources_dir: None,
        image_href_resolver: resvg::usvg::ImageHrefResolver { resolve_data: Box::new(|_, _, _| None), resolve_string: Box::new(|_, _| None) },
        ..Default::default()
    };
    let tree = resvg::usvg::Tree::from_data(data, &options).map_err(|_| ImportError::Decode)?;
    let size = tree.size();
    let scale = SVG_LONG_EDGE_PX / size.width().max(size.height());
    let (w, h) = (((size.width() * scale).round() as u32).max(1), ((size.height() * scale).round() as u32).max(1));
    let mut pixmap = resvg::tiny_skia::Pixmap::new(w, h).ok_or(ImportError::Decode)?;
    resvg::render(&tree, resvg::tiny_skia::Transform::from_scale(scale, scale), &mut pixmap.as_mut());
    let rgba: Vec<u8> = pixmap
        .pixels()
        .iter()
        .flat_map(|p| {
            let c = p.demultiply();
            [c.red(), c.green(), c.blue(), c.alpha()]
        })
        .collect();
    image::RgbaImage::from_raw(w, h, rgba).map(DynamicImage::ImageRgba8).ok_or(ImportError::Decode)
}

fn has_transparency(img: &DynamicImage) -> bool {
    img.color().has_alpha() && img.to_rgba8().pixels().any(|p| p[3] < 255)
}

pub(crate) fn encode(img: &DynamicImage, mime: Mime) -> Result<Vec<u8>, ImportError> {
    let mut out = Vec::new();
    match mime {
        Mime::Jpeg => img.to_rgb8().write_with_encoder(JpegEncoder::new_with_quality(&mut out, JPEG_QUALITY)),
        Mime::Png => img.to_rgba8().write_with_encoder(PngEncoder::new(&mut out)),
    }
    .map_err(|_| ImportError::Decode)?;
    Ok(out)
}

fn fit(img: &DynamicImage, long_edge: u32) -> DynamicImage {
    if img.width().max(img.height()) <= long_edge {
        img.clone()
    } else {
        img.resize(long_edge, long_edge, FilterType::Lanczos3)
    }
}

/// Le réencodage n'écrit aucune métadonnée : c'est lui qui retire l'EXIF (dont la position GPS).
pub fn import_bytes(data: &[u8]) -> Result<Asset, ImportError> {
    import_with(data, None)
}

/// Avec `frame_aspect` (une image d'un document repris, largeur sur hauteur de son cadre) : l'image est tournée
/// dans le sens qui va le mieux à son cadre, selon son étiquette d'orientation ou telle qu'elle est enregistrée.
/// LibreOffice n'applique pas l'étiquette à une image qui remplit une forme : l'image brute est souvent la bonne,
/// mais pas toujours.
fn import_with(data: &[u8], frame_aspect: Option<f64>) -> Result<Asset, ImportError> {
    if is_heic(data) {
        return Err(ImportError::Heic);
    }
    if is_svg(data) {
        let img = render_svg(data)?;
        let mime = if has_transparency(&img) { Mime::Png } else { Mime::Jpeg };
        return asset_from_image(&img, mime);
    }
    let mut reader = ImageReader::new(Cursor::new(data)).with_guessed_format().map_err(|_| ImportError::Io)?;
    if reader.format().is_none() {
        return Err(ImportError::Unsupported);
    }
    reader.limits(decode_limits());
    let mut decoder = reader.into_decoder().map_err(decode_error)?;
    let orientation = decoder.orientation().map_err(|_| ImportError::Decode)?;
    let mut img = DynamicImage::from_decoder(decoder).map_err(decode_error)?;
    if frame_aspect.is_none_or(|a| oriented_fits(img.width(), img.height(), orientation, a)) {
        img.apply_orientation(orientation);
    }

    let mime = if has_transparency(&img) { Mime::Png } else { Mime::Jpeg };
    let main = fit(&img, MAX_LONG_EDGE_PX);
    let thumb = fit(&main, THUMB_LONG_EDGE_PX);
    Ok(Asset {
        meta: AssetMeta { id: new_id(), width_px: main.width(), height_px: main.height(), mime },
        bytes: Arc::new(encode(&main, mime)?),
        thumb: Arc::new(encode(&thumb, mime)?),
    })
}

fn decode_limits() -> image::Limits {
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(MAX_DECODE_PX);
    limits.max_image_height = Some(MAX_DECODE_PX);
    limits.max_alloc = Some(MAX_DECODE_ALLOC);
    limits
}

/// Une limite de décodage dépassée est dite comme telle ; toute autre erreur, image illisible.
fn decode_error(e: image::ImageError) -> ImportError {
    match e {
        image::ImageError::Limits(_) => ImportError::TooLarge,
        _ => ImportError::Decode,
    }
}

/// Nouvelle image (nouvel id, vignette refaite) à partir d'une image déjà importée et modifiée (flou).
pub(crate) fn asset_from_image(img: &DynamicImage, mime: Mime) -> Result<Asset, ImportError> {
    let main = fit(img, MAX_LONG_EDGE_PX);
    let thumb = fit(&main, THUMB_LONG_EDGE_PX);
    Ok(Asset {
        meta: AssetMeta { id: new_id(), width_px: main.width(), height_px: main.height(), mime },
        bytes: Arc::new(encode(&main, mime)?),
        thumb: Arc::new(encode(&thumb, mime)?),
    })
}

pub fn import_path(path: &Path) -> Result<Asset, ImportError> {
    let data = std::fs::read(path).map_err(|_| ImportError::Io)?;
    import_bytes(&data)
}

/// Images d'un dossier, sans les sous-dossiers, triées par nom. Les HEIC sont listés pour être signalés.
pub fn list_importable(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir) else { return vec![] };
    let mut files: Vec<PathBuf> = entries
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.is_file())
        .filter(|p| {
            p.extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| IMPORTABLE_EXTENSIONS.contains(&e.to_ascii_lowercase().as_str()))
        })
        .collect();
    files.sort_by_key(|p| p.file_name().map(|n| n.to_os_string()));
    files
}

#[cfg(test)]
mod svg_tests {
    use super::*;

    const DRAWING: &str = r##"<?xml version="1.0"?>
<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" viewBox="0 0 100 50" width="100" height="50">
  <circle cx="25" cy="25" r="20" fill="#e67e22"/>
  <image x="50" y="0" width="50" height="50" xlink:href="/etc/passwd"/>
  <image x="50" y="0" width="50" height="50" xlink:href="https://example.org/x.png"/>
</svg>"##;

    /// Un dessin SVG devient une image transparente à ses proportions ; les références extérieures sont ignorées.
    #[test]
    fn an_svg_drawing_becomes_a_transparent_picture() {
        let a = import_bytes(DRAWING.as_bytes()).unwrap();
        assert_eq!((a.meta.mime, a.meta.width_px, a.meta.height_px), (Mime::Png, 2000, 1000));
        let img = image::load_from_memory(&a.bytes).unwrap().to_rgba8();
        assert!(img.get_pixel(500, 500)[3] > 200, "le disque est dessiné");
        assert_eq!(img.get_pixel(1500, 500)[3], 0, "rien n'est lu hors du fichier");
    }

    /// Un très grand dessin reste à la taille d'une photo ; un fichier qui n'est pas un dessin est refusé.
    #[test]
    fn a_huge_svg_stays_small_and_garbage_is_refused() {
        let huge = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 900000 300000"><rect width="900000" height="300000" fill="red"/></svg>"#;
        let a = import_bytes(huge.as_bytes()).unwrap();
        assert!(a.meta.width_px <= MAX_LONG_EDGE_PX && a.meta.height_px <= MAX_LONG_EDGE_PX);
        assert!(import_bytes(b"<svg nope").is_err());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageFormat, Rgb, RgbImage, Rgba, RgbaImage};
    use std::io::Cursor;

    fn jpeg(w: u32, h: u32) -> Vec<u8> {
        let img = RgbImage::from_pixel(w, h, Rgb([120, 160, 200]));
        let mut out = Cursor::new(Vec::new());
        img.write_to(&mut out, ImageFormat::Jpeg).unwrap();
        out.into_inner()
    }

    fn decoded_dims(bytes: &[u8]) -> (u32, u32) {
        let img = image::load_from_memory(bytes).unwrap();
        (img.width(), img.height())
    }

    const FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures");

    #[test]
    fn large_photo_is_downscaled() {
        let a = import_bytes(&jpeg(6000, 4000)).unwrap();
        assert_eq!((a.meta.width_px, a.meta.height_px), (3000, 2000));
        assert_eq!(a.meta.mime, Mime::Jpeg);
        assert_eq!(decoded_dims(&a.bytes), (3000, 2000));
    }

    #[test]
    fn small_photo_is_not_upscaled() {
        let a = import_bytes(&jpeg(800, 600)).unwrap();
        assert_eq!((a.meta.width_px, a.meta.height_px), (800, 600));
    }

    #[test]
    fn orientation_is_applied() {
        let a = import_path(&Path::new(FIXTURES).join("orientation6.jpg")).unwrap();
        assert_eq!((a.meta.width_px, a.meta.height_px), (20, 40));
        assert_eq!(decoded_dims(&a.bytes), (20, 40));
    }

    /// Une image de document repris est tournée dans le sens de son cadre : la photo de 40 × 20 enregistrée,
    /// étiquetée « tourner de 90° », reste brute dans un cadre en paysage et se redresse dans un cadre en portrait.
    #[test]
    fn a_document_picture_is_turned_to_fit_its_frame() {
        let data = std::fs::read(Path::new(FIXTURES).join("orientation6.jpg")).unwrap();
        let dims = |aspect: f64| {
            let a = import_bytes_many(&[(data.clone(), Some(aspect))], 1).pop().unwrap().unwrap();
            (a.meta.width_px, a.meta.height_px)
        };
        assert_eq!(dims(2.0), (40, 20));
        assert_eq!(dims(0.5), (20, 40));
        // Sans cadre, l'étiquette s'applique toujours.
        let a = import_bytes_many(&[(data.clone(), None)], 1).pop().unwrap().unwrap();
        assert_eq!((a.meta.width_px, a.meta.height_px), (20, 40));
    }

    #[test]
    fn orientation_8_is_applied() {
        let a = import_path(&Path::new(FIXTURES).join("orientation8.jpg")).unwrap();
        assert_eq!((a.meta.width_px, a.meta.height_px), (20, 40));
    }

    #[test]
    fn metadata_is_stripped() {
        let src = std::fs::read(Path::new(FIXTURES).join("gps.jpg")).unwrap();
        assert!(exif::Reader::new().read_from_container(&mut Cursor::new(&src)).is_ok());
        let a = import_bytes(&src).unwrap();
        assert!(exif::Reader::new().read_from_container(&mut Cursor::new(&*a.bytes)).is_err());
    }

    #[test]
    fn transparency_keeps_png() {
        let mut img = RgbaImage::from_pixel(50, 50, Rgba([255, 0, 0, 255]));
        img.put_pixel(0, 0, Rgba([0, 0, 0, 0]));
        let mut out = Cursor::new(Vec::new());
        img.write_to(&mut out, ImageFormat::Png).unwrap();
        let a = import_bytes(&out.into_inner()).unwrap();
        assert_eq!(a.meta.mime, Mime::Png);
        let back = image::load_from_memory(&a.bytes).unwrap().to_rgba8();
        assert_eq!(back.get_pixel(0, 0)[3], 0);
    }

    #[test]
    fn heic_is_rejected_with_message() {
        let mut data = vec![0, 0, 0, 24];
        data.extend_from_slice(b"ftypheic");
        data.extend_from_slice(&[0; 32]);
        let err = import_bytes(&data).err().unwrap();
        assert_eq!(err, ImportError::Heic);
        assert!(err.message_fr().contains("HEIC"));
    }

    #[test]
    fn corrupt_file_is_an_error_not_a_panic() {
        assert_eq!(import_bytes(b"\xFF\xD8\xFF garbage").err(), Some(ImportError::Decode));
    }

    #[test]
    fn thumbnail_long_edge_is_256() {
        let a = import_bytes(&jpeg(6000, 4000)).unwrap();
        assert_eq!(decoded_dims(&a.thumb), (256, 171));
    }

    #[test]
    fn lists_importable_files() {
        let dir = tempfile::tempdir().unwrap();
        for name in ["a.JPG", "b.png", "c.txt", "d.HEIC"] {
            std::fs::write(dir.path().join(name), b"x").unwrap();
        }
        std::fs::create_dir(dir.path().join("sous")).unwrap();
        std::fs::write(dir.path().join("sous/e.jpg"), b"x").unwrap();
        let names: Vec<String> = list_importable(dir.path())
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        assert_eq!(names, ["a.JPG", "b.png", "d.HEIC"]);
    }

    #[test]
    fn store_keeps_assets_by_id() {
        let a = import_bytes(&jpeg(10, 10)).unwrap();
        let id = a.meta.id.clone();
        let mut store = AssetStore::default();
        store.insert(a).unwrap();
        assert_eq!(store.get(&id).unwrap().meta.width_px, 10);
        assert!(store.get("autre").is_none());
    }

    #[test]
    fn cached_store_keeps_only_thumbnails_in_memory() {
        let dir = tempfile::tempdir().unwrap();
        let cache = dir.path().join("images");
        let mut store = AssetStore::with_cache_dir(cache.clone()).unwrap();
        let a = import_bytes(&jpeg(40, 30)).unwrap();
        let (id, bytes, thumb) = (a.meta.id.clone(), a.bytes.clone(), a.thumb.clone());
        store.insert(a).unwrap();
        assert!(cache.join(format!("{id}.jpg")).is_file());
        let back = store.get(&id).unwrap();
        assert_eq!(back.bytes, bytes);
        assert_eq!(store.thumb(&id).unwrap(), (thumb, Mime::Jpeg));
    }

    #[test]
    fn clear_cache_dir_empties_the_folder() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = AssetStore::with_cache_dir(dir.path().to_path_buf()).unwrap();
        store.insert(import_bytes(&jpeg(10, 10)).unwrap()).unwrap();
        clear_cache_dir(dir.path());
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
    }

    #[test]
    fn cache_write_failure_is_an_import_error() {
        let dir = tempfile::tempdir().unwrap();
        let cache = dir.path().join("images");
        let mut store = AssetStore::with_cache_dir(cache.clone()).unwrap();
        std::fs::remove_dir(&cache).unwrap();
        let err = store.insert(import_bytes(&jpeg(10, 10)).unwrap()).err().unwrap();
        assert_eq!(err, ImportError::Storage);
        assert!(err.message_fr().contains("disque"));
    }

    #[test]
    fn import_many_reports_every_file_once() {
        let dir = tempfile::tempdir().unwrap();
        let mut paths = Vec::new();
        for i in 0..3 {
            let p = dir.path().join(format!("p{i}.jpg"));
            std::fs::write(&p, jpeg(20, 20)).unwrap();
            paths.push(p);
        }
        let txt = dir.path().join("texte.jpg");
        std::fs::write(&txt, b"pas une image").unwrap();
        paths.push(txt);
        let heic = dir.path().join("iphone.jpg");
        let mut data = vec![0, 0, 0, 24];
        data.extend_from_slice(b"ftypheic");
        data.extend_from_slice(&[0; 32]);
        std::fs::write(&heic, data).unwrap();
        paths.push(heic);

        let seen = std::sync::Mutex::new(Vec::new());
        import_many(&paths, 2, |i, r| seen.lock().unwrap().push((i, r.is_ok())));
        let mut seen = seen.into_inner().unwrap();
        seen.sort();
        assert_eq!(seen.iter().map(|s| s.0).collect::<Vec<_>>(), [0, 1, 2, 3, 4]);
        assert_eq!(seen.iter().filter(|s| s.1).count(), 3);
    }

    #[test]
    fn second_instance_keeps_the_first_instance_cache() {
        let root = tempfile::tempdir().unwrap();
        let first = CacheDir::acquire(root.path()).unwrap();
        let mut store = AssetStore::with_cache_dir(first.path().to_path_buf()).unwrap();
        let a = import_bytes(&jpeg(10, 10)).unwrap();
        let id = a.meta.id.clone();
        store.insert(a).unwrap();
        let second = CacheDir::acquire(root.path()).unwrap();
        assert_ne!(first.path(), second.path());
        assert!(store.get(&id).is_some(), "le cache de la première instance a été vidé");
    }

    #[test]
    fn stale_cache_of_a_finished_instance_is_cleared() {
        let root = tempfile::tempdir().unwrap();
        let old = root.path().join("ancienne");
        std::fs::create_dir(&old).unwrap();
        std::fs::write(old.join("photo.jpg"), b"x").unwrap();
        std::fs::write(root.path().join("ancienne.lock"), b"").unwrap();
        std::fs::write(root.path().join("vieille-version.jpg"), b"x").unwrap();
        let mine = CacheDir::acquire(root.path()).unwrap();
        let left: Vec<String> = std::fs::read_dir(root.path()).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect();
        assert_eq!(left.len(), 2, "{left:?}");
        assert!(mine.path().is_dir());
        mine.clear();
        assert!(!mine.path().exists());
    }

    #[test]
    fn staging_writes_the_file_before_the_store_knows_the_photo() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = AssetStore::with_cache_dir(dir.path().to_path_buf()).unwrap();
        let a = import_bytes(&jpeg(12, 12)).unwrap();
        let (id, bytes) = (a.meta.id.clone(), a.bytes.clone());
        let staged = stage(store.cache_dir().as_deref(), a).unwrap();
        assert!(dir.path().join(format!("{id}.jpg")).is_file());
        assert!(store.get(&id).is_none());
        store.commit(staged);
        let source = store.locate(&id).unwrap();
        drop(store);
        assert_eq!(source.load().unwrap().bytes, bytes);
    }

    #[test]
    fn a_photo_id_never_writes_outside_the_cache() {
        let root = tempfile::tempdir().unwrap();
        let cache = root.path().join("cache");
        std::fs::create_dir(&cache).unwrap();
        let mut a = import_bytes(&jpeg(12, 12)).unwrap();
        a.meta.id = "../dehors".into();
        assert!(stage(Some(&cache), a).is_err());
        assert!(!root.path().join("dehors.jpg").exists());
    }

    /// Une image piégée annonce des dimensions énormes : refusée avant d'allouer sa mémoire.
    #[test]
    fn a_picture_wider_than_the_limit_is_refused() {
        let mut out = Cursor::new(Vec::new());
        image::GrayImage::new(MAX_DECODE_PX + 1, 1).write_to(&mut out, image::ImageFormat::Png).unwrap();
        assert_eq!(import_bytes(&out.into_inner()).err(), Some(ImportError::TooLarge));
        let mut ok = Cursor::new(Vec::new());
        image::GrayImage::new(64, 1).write_to(&mut ok, image::ImageFormat::Png).unwrap();
        assert!(import_bytes(&ok.into_inner()).is_ok());
    }
}
