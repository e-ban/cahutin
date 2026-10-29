//! Lecture et écriture d'un document Cahutin pour le serveur : seule l'entrée `document.json` est
//! réécrite ; photos et polices sont recopiées sans être décompressées.

use carnet_core::{Document, native};
use std::fs::File;
use std::io::Write;
use std::path::Path;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

const DOCUMENT_ENTRY: &str = "document.json";

pub fn read(path: &Path) -> Result<Document, String> {
    native::load_from_path(path).map(|l| l.doc).map_err(|e| e.message_fr())
}

/// Remplace `document.json` dans un fichier voisin, renommé à la fin : une erreur laisse l'original intact.
pub fn write(path: &Path, doc: &Document) -> Result<(), String> {
    let err = |e: &dyn std::fmt::Display| format!("écriture impossible : {e}");
    let mut doc = doc.clone();
    doc.format_version = native::written_version(&doc);
    let json = serde_json::to_vec_pretty(&doc).map_err(|e| err(&e))?;
    let mut source = ZipArchive::new(File::open(path).map_err(|e| err(&e))?).map_err(|e| err(&e))?;
    let tmp = path.with_extension("carnet.tmp");
    let result = (|| -> Result<(), String> {
        let mut out = ZipWriter::new(File::create(&tmp).map_err(|e| err(&e))?);
        out.start_file(DOCUMENT_ENTRY, SimpleFileOptions::default().compression_method(CompressionMethod::Deflated)).map_err(|e| err(&e))?;
        out.write_all(&json).map_err(|e| err(&e))?;
        for i in 0..source.len() {
            let entry = source.by_index_raw(i).map_err(|e| err(&e))?;
            if entry.name().map_err(|e| err(&e))? != DOCUMENT_ENTRY {
                out.raw_copy_file(entry).map_err(|e| err(&e))?;
            }
        }
        out.finish().map_err(|e| err(&e))?.sync_all().map_err(|e| err(&e))
    })();
    match result {
        Ok(()) => std::fs::rename(&tmp, path).map_err(|e| err(&e)),
        Err(e) => {
            let _ = std::fs::remove_file(&tmp);
            Err(e)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use carnet_core::fonts::FontSet;
    use carnet_core::photo::{AssetStore, import_bytes};
    use carnet_core::*;
    use image::{ImageFormat, Rgb, RgbImage};
    use std::io::Cursor;

    fn saved(dir: &Path) -> (std::path::PathBuf, Vec<u8>) {
        let mut jpeg = Cursor::new(Vec::new());
        RgbImage::from_pixel(30, 20, Rgb([200, 10, 10])).write_to(&mut jpeg, ImageFormat::Jpeg).unwrap();
        let asset = import_bytes(&jpeg.into_inner()).unwrap();
        let bytes = asset.bytes.to_vec();
        let mut doc = Document::new("essai");
        doc.pages[0].elements.push(Element {
            id: "p1".into(),
            frame: Frame { x_mm: 10.0, y_mm: 10.0, w_mm: 60.0, h_mm: 40.0, rotation_deg: 0.0 },
            locked: false,
            zone: true,
            group: None,
            kind: ElementKind::Photo(PhotoProps { asset: Some(asset.meta.id.clone()), crop: Crop::COVER, corner_radius_mm: 0.0, border: None, shadow: false }),
        });
        doc.assets.insert(asset.meta.id.clone(), asset.meta.clone());
        let mut store = AssetStore::default();
        store.insert(asset).unwrap();
        let path = dir.join("essai.carnet");
        native::save_to_path(&doc, &store, &FontSet::builtin(), &path).unwrap();
        (path, bytes)
    }

    #[test]
    fn writing_changes_the_layout_and_keeps_the_photos_byte_for_byte() {
        let dir = tempfile::tempdir().unwrap();
        let (path, photo) = saved(dir.path());
        let mut doc = read(&path).unwrap();
        doc.pages[0].elements[0].frame.x_mm = 99.0;
        write(&path, &doc).unwrap();
        let loaded = native::load_from_path(&path).unwrap();
        assert_eq!(loaded.doc.pages[0].elements[0].frame.x_mm, 99.0);
        assert_eq!(loaded.assets.len(), 1);
        assert_eq!(*loaded.assets[0].bytes, photo);
        assert!(!dir.path().read_dir().unwrap().any(|e| e.unwrap().file_name().to_string_lossy().contains(".tmp")));
    }

    #[test]
    fn a_damaged_file_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("abime.carnet");
        std::fs::write(&path, b"pas un zip").unwrap();
        assert!(read(&path).is_err());
    }
}
