//! Format natif : archive zip avec `document.json` et les images utilisées.

pub use crate::model::FORMAT_VERSION;
use crate::model::{Document, ElementKind, Mime};
use crate::photo::{Asset, AssetStore};
use std::collections::BTreeSet;
use std::io::{Read, Seek, Write};
use std::path::Path;
use std::sync::Arc;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

pub const DOC_EXT: &str = "carnet";
pub const TEMPLATE_EXT: &str = "carnet-modele";

const DOCUMENT_ENTRY: &str = "document.json";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeError {
    NewerVersion(u32),
    Corrupt,
    /// Une photo du document n'est plus en mémoire.
    MissingAsset,
    Io,
}

impl NativeError {
    pub fn message_fr(&self) -> String {
        match self {
            NativeError::NewerVersion(_) => "Ce fichier a été créé par une version plus récente de l'application.",
            NativeError::Corrupt => "Ce fichier est abîmé ou n'est pas un document de l'application.",
            NativeError::MissingAsset => "Une photo du document est introuvable : le document n'a pas été enregistré.",
            NativeError::Io => "Impossible de lire ou d'écrire le fichier.",
        }
        .to_string()
    }
}

impl From<std::io::Error> for NativeError {
    fn from(_: std::io::Error) -> Self {
        NativeError::Io
    }
}

impl From<zip::result::ZipError> for NativeError {
    fn from(e: zip::result::ZipError) -> Self {
        match e {
            zip::result::ZipError::Io(_) => NativeError::Io,
            _ => NativeError::Corrupt,
        }
    }
}

fn extension(mime: Mime) -> &'static str {
    match mime {
        Mime::Jpeg => "jpg",
        Mime::Png => "png",
    }
}

fn used_assets(doc: &Document) -> BTreeSet<String> {
    doc.pages
        .iter()
        .flat_map(|p| &p.elements)
        .filter_map(|e| match &e.kind {
            ElementKind::Photo(p) => p.asset.clone(),
            _ => None,
        })
        .collect()
}

/// Seules les images utilisées par un élément sont enregistrées.
pub fn save(doc: &Document, store: &AssetStore, out: impl Write + Seek) -> Result<(), NativeError> {
    let used = used_assets(doc);
    let mut doc = doc.clone();
    doc.assets.retain(|id, _| used.contains(id));

    let mut zip = ZipWriter::new(out);
    let deflated = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    let stored = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
    zip.start_file(DOCUMENT_ENTRY, deflated)?;
    zip.write_all(&serde_json::to_vec_pretty(&doc).map_err(|_| NativeError::Corrupt)?)?;
    for meta in doc.assets.values() {
        let asset = store.get(&meta.id).ok_or(NativeError::MissingAsset)?;
        let ext = extension(meta.mime);
        zip.start_file(format!("assets/{}.{ext}", meta.id), stored)?;
        zip.write_all(&asset.bytes)?;
        zip.start_file(format!("assets/{}.thumb.{ext}", meta.id), stored)?;
        zip.write_all(&asset.thumb)?;
    }
    zip.finish()?;
    Ok(())
}

/// Écriture sûre : un échec ne détruit pas l'ancien fichier.
pub fn save_to_path(doc: &Document, store: &AssetStore, path: &Path) -> Result<(), NativeError> {
    crate::fsutil::write_atomically(path, |f| save(doc, store, f))
}

fn read_entry<R: Read + Seek>(zip: &mut ZipArchive<R>, name: &str) -> Result<Vec<u8>, NativeError> {
    let mut entry = zip.by_name(name)?;
    let mut buf = Vec::new();
    entry.read_to_end(&mut buf)?;
    Ok(buf)
}

pub fn load(input: impl Read + Seek) -> Result<(Document, Vec<Asset>), NativeError> {
    let mut zip = ZipArchive::new(input)?;
    let json = read_entry(&mut zip, DOCUMENT_ENTRY)?;
    let version: serde_json::Value = serde_json::from_slice(&json).map_err(|_| NativeError::Corrupt)?;
    let found = version["format_version"].as_u64().ok_or(NativeError::Corrupt)? as u32;
    if found > FORMAT_VERSION {
        return Err(NativeError::NewerVersion(found));
    }
    let doc: Document = serde_json::from_slice(&json).map_err(|_| NativeError::Corrupt)?;
    let mut assets = Vec::new();
    for meta in doc.assets.values() {
        let ext = extension(meta.mime);
        assets.push(Asset {
            meta: meta.clone(),
            bytes: Arc::new(read_entry(&mut zip, &format!("assets/{}.{ext}", meta.id))?),
            thumb: Arc::new(read_entry(&mut zip, &format!("assets/{}.thumb.{ext}", meta.id))?),
        });
    }
    Ok((doc, assets))
}

pub fn load_from_path(path: &Path) -> Result<(Document, Vec<Asset>), NativeError> {
    load(std::fs::File::open(path)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::*;
    use crate::photo::import_bytes;
    use image::{ImageFormat, Rgb, RgbImage};
    use std::io::Cursor;

    fn photo_asset() -> Asset {
        let mut out = Cursor::new(Vec::new());
        RgbImage::from_pixel(30, 20, Rgb([10, 20, 30])).write_to(&mut out, ImageFormat::Jpeg).unwrap();
        import_bytes(&out.into_inner()).unwrap()
    }

    fn photo_element(asset: &str) -> Element {
        Element {
            id: new_id(),
            frame: Frame { x_mm: 10.0, y_mm: 10.0, w_mm: 50.0, h_mm: 40.0, rotation_deg: 0.0 },
            locked: false,
            zone: true,
            kind: ElementKind::Photo(PhotoProps {
                asset: Some(asset.into()),
                crop: Crop::COVER,
                corner_radius_mm: 0.0,
                border: None,
                shadow: false,
            }),
        }
    }

    /// Document avec deux photos utilisées et une photo inutilisée.
    fn sample() -> (Document, AssetStore, String) {
        let mut doc = Document::new("Cahier été");
        let mut store = AssetStore::default();
        let mut unused = String::new();
        for i in 0..3 {
            let a = photo_asset();
            if i < 2 {
                doc.pages[0].elements.push(photo_element(&a.meta.id));
            } else {
                unused = a.meta.id.clone();
            }
            doc.assets.insert(a.meta.id.clone(), a.meta.clone());
            store.insert(a).unwrap();
        }
        (doc, store, unused)
    }

    #[test]
    fn roundtrip_document_and_assets() {
        let (doc, store, unused) = sample();
        let mut buf = Cursor::new(Vec::new());
        save(&doc, &store, &mut buf).unwrap();
        buf.set_position(0);
        let (back, assets) = load(buf).unwrap();
        let mut expected = doc.clone();
        expected.assets.remove(&unused);
        assert_eq!(back, expected);
        assert_eq!(assets.len(), 2);
        for a in &assets {
            let original = store.get(&a.meta.id).unwrap();
            assert_eq!(a.bytes, original.bytes);
            assert_eq!(a.thumb, original.thumb);
            assert_eq!(a.meta, original.meta);
        }
    }

    #[test]
    fn unused_assets_are_not_saved() {
        let (doc, store, unused) = sample();
        let mut buf = Cursor::new(Vec::new());
        save(&doc, &store, &mut buf).unwrap();
        buf.set_position(0);
        let (back, assets) = load(buf).unwrap();
        assert!(!back.assets.contains_key(&unused));
        assert!(assets.iter().all(|a| a.meta.id != unused));
    }

    #[test]
    fn newer_version_is_refused() {
        let mut doc = Document::new("futur");
        doc.format_version = 2;
        let mut buf = Cursor::new(Vec::new());
        save(&doc, &AssetStore::default(), &mut buf).unwrap();
        buf.set_position(0);
        let err = load(buf).err().unwrap();
        assert_eq!(err, NativeError::NewerVersion(2));
        assert!(err.message_fr().contains("plus récente"));
    }

    #[test]
    fn missing_image_is_reported_as_such() {
        let (doc, _, _) = sample();
        let mut buf = Cursor::new(Vec::new());
        let err = save(&doc, &AssetStore::default(), &mut buf).err().unwrap();
        assert!(matches!(err, NativeError::MissingAsset));
        assert!(err.message_fr().contains("photo"));
    }

    #[test]
    fn garbage_is_corrupt() {
        let buf = Cursor::new(b"pas une archive du tout".to_vec());
        assert_eq!(load(buf).err(), Some(NativeError::Corrupt));
    }

    #[test]
    fn saves_to_path_with_accents_and_spaces() {
        let dir = tempfile::tempdir().unwrap();
        let folder = dir.path().join("Élodie dossier");
        std::fs::create_dir(&folder).unwrap();
        let path = folder.join("cahier été.carnet");
        let (doc, store, _) = sample();
        save_to_path(&doc, &store, &path).unwrap();
        let (back, assets) = load_from_path(&path).unwrap();
        assert_eq!(back.title, "Cahier été");
        assert_eq!(assets.len(), 2);
        let leftovers: Vec<_> = std::fs::read_dir(&folder).unwrap().map(|e| e.unwrap().file_name()).collect();
        assert_eq!(leftovers, vec![std::ffi::OsString::from("cahier été.carnet")]);
    }
}
