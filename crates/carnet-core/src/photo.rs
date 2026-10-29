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

const IMPORTABLE_EXTENSIONS: [&str; 6] = ["jpg", "jpeg", "png", "webp", "heic", "heif"];

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
}

impl ImportError {
    pub fn message_fr(&self) -> String {
        match self {
            ImportError::Heic => "format HEIC (iPhone) non pris en charge : enregistrez la photo en JPEG",
            ImportError::Unsupported => "format d'image non pris en charge (JPEG, PNG ou WebP attendu)",
            ImportError::Decode => "fichier illisible ou abîmé",
            ImportError::Io => "impossible de lire le fichier",
            ImportError::Storage => "impossible d'enregistrer la photo sur le disque (disque plein ?)",
        }
        .to_string()
    }
}

fn is_heic(data: &[u8]) -> bool {
    data.len() >= 12 && &data[4..8] == b"ftyp" && matches!(&data[8..12], b"heic" | b"heix" | b"mif1" | b"msf1" | b"hevc")
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
    if is_heic(data) {
        return Err(ImportError::Heic);
    }
    let reader = ImageReader::new(Cursor::new(data)).with_guessed_format().map_err(|_| ImportError::Io)?;
    if reader.format().is_none() {
        return Err(ImportError::Unsupported);
    }
    let mut decoder = reader.into_decoder().map_err(|_| ImportError::Decode)?;
    let orientation = decoder.orientation().map_err(|_| ImportError::Decode)?;
    let mut img = DynamicImage::from_decoder(decoder).map_err(|_| ImportError::Decode)?;
    img.apply_orientation(orientation);

    let mime = if has_transparency(&img) { Mime::Png } else { Mime::Jpeg };
    let main = fit(&img, MAX_LONG_EDGE_PX);
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
}
