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

#[derive(Default)]
pub struct AssetStore {
    assets: HashMap<Id, Asset>,
}

impl AssetStore {
    pub fn insert(&mut self, a: Asset) {
        self.assets.insert(a.meta.id.clone(), a);
    }

    pub fn get(&self, id: &str) -> Option<&Asset> {
        self.assets.get(id)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImportError {
    Heic,
    Unsupported,
    Decode,
    Io,
}

impl ImportError {
    pub fn message_fr(&self) -> String {
        match self {
            ImportError::Heic => "format HEIC (iPhone) non pris en charge : enregistrez la photo en JPEG",
            ImportError::Unsupported => "format d'image non pris en charge (JPEG, PNG ou WebP attendu)",
            ImportError::Decode => "fichier illisible ou abîmé",
            ImportError::Io => "impossible de lire le fichier",
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
        store.insert(a);
        assert_eq!(store.get(&id).unwrap().meta.width_px, 10);
        assert!(store.get("autre").is_none());
    }
}
