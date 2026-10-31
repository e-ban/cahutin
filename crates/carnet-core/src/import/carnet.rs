//! Pages d'un autre document Carnet, avec ses photos et ses polices.

use super::{ImportError, ImportReport, Imported};
use crate::model::renew_ids;
use crate::native::{self, LoadedDocument};
use std::io::{Read, Seek};

/// Toutes les pages, aux identifiants renouvelés ; photos et polices suivent.
pub fn pages_from(loaded: LoadedDocument) -> Imported {
    let LoadedDocument { doc, assets, fonts } = loaded;
    let mut pages = doc.pages;
    pages.iter_mut().for_each(renew_ids);
    let report = ImportReport::count(&pages);
    Imported { pages, assets, fonts, report }
}

pub fn read_carnet(input: impl Read + Seek) -> Result<Imported, ImportError> {
    native::load(input).map(pages_from).map_err(ImportError::Native)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::*;
    use crate::native;
    use crate::photo::import_bytes;
    use image::{ImageFormat, Rgb, RgbImage};
    use std::collections::HashSet;
    use std::io::{Cursor, Write};

    fn el(kind: ElementKind, group: Option<&str>) -> Element {
        Element { id: new_id(), frame: Frame { x_mm: 10.0, y_mm: 10.0, w_mm: 50.0, h_mm: 30.0, rotation_deg: 0.0 }, locked: false, zone: false, group: group.map(String::from), kind }
    }

    fn text() -> ElementKind {
        ElementKind::Text(TextProps { content: RichText::plain("Bonjour"), style: TextStyle::default_zone(), prompt: None, frame: None })
    }

    fn rect() -> ElementKind {
        ElementKind::Shape(ShapeProps { shape: ShapeKind::Rect, fill: Some("#ffffff".into()), stroke: None, corner_radius_mm: 0.0 })
    }

    fn photo_asset() -> crate::photo::Asset {
        let mut out = Cursor::new(Vec::new());
        RgbImage::from_pixel(20, 10, Rgb([0, 120, 200])).write_to(&mut out, ImageFormat::Png).unwrap();
        import_bytes(&out.into_inner()).unwrap()
    }

    /// Deux pages : une grille de deux cases, puis un groupe (texte et rectangle) et une image OpenMoji.
    fn loaded() -> LoadedDocument {
        let asset = photo_asset();
        let mut doc = Document::new("autre");
        let mut first = Page::new(A4_PORTRAIT);
        crate::grid::set_grid(&mut first, 2, 1);
        let mut second = Page::new(A4_PORTRAIT);
        second.elements.push(el(text(), Some("g1")));
        second.elements.push(el(rect(), Some("g1")));
        second.elements.push(el(ElementKind::Image(ImageProps { code: "1F600".into() }), None));
        second.elements.push(el(ElementKind::Photo(PhotoProps { asset: Some(asset.meta.id.clone()), crop: Crop::COVER, corner_radius_mm: 0.0, border: None, shadow: false, mirrored: false }), None));
        doc.pages = vec![first, second];
        doc.assets.insert(asset.meta.id.clone(), asset.meta.clone());
        LoadedDocument { doc, assets: vec![asset], fonts: vec![("Maison.ttf".into(), vec![1, 2, 3])] }
    }

    fn ids(pages: &[Page]) -> HashSet<String> {
        pages.iter().flat_map(|p| std::iter::once(p.id.clone()).chain(p.elements.iter().map(|e| e.id.clone()))).collect()
    }

    #[test]
    fn carnet_pages_get_fresh_ids() {
        let original = loaded();
        let before = original.doc.pages.clone();
        let imported = pages_from(original);
        assert_eq!(imported.pages.len(), 2);
        assert!(ids(&imported.pages).is_disjoint(&ids(&before)));
        let grid = imported.pages[0].grid.as_ref().unwrap();
        assert!(grid.cells.iter().all(|c| imported.pages[0].elements.iter().any(|e| e.id == c.element_id)));
        let groups: Vec<&String> = imported.pages[1].elements.iter().filter_map(|e| e.group.as_ref()).collect();
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0], groups[1]);
        assert_ne!(groups[0], "g1");
    }

    #[test]
    fn carnet_assets_and_fonts_come_along() {
        let original = loaded();
        let asset_id = original.assets[0].meta.id.clone();
        let imported = pages_from(original);
        assert_eq!(imported.assets.iter().map(|a| a.meta.id.clone()).collect::<Vec<_>>(), [asset_id]);
        assert_eq!(imported.fonts, [("Maison.ttf".to_string(), vec![1, 2, 3])]);
    }

    #[test]
    fn report_counts_elements() {
        let r = pages_from(loaded()).report;
        // Grille de deux cases (zones photo), un texte, un rectangle, une image OpenMoji, une photo.
        assert_eq!(r, crate::import::ImportReport { pages: 2, texts: 1, images: 4, shapes: 1, skipped: 0 });
    }

    #[test]
    fn newer_carnet_is_refused() {
        let mut doc = Document::new("futur");
        doc.format_version = 99;
        let mut buf = Cursor::new(Vec::new());
        {
            let mut zip = zip::ZipWriter::new(&mut buf);
            zip.start_file("document.json", zip::write::SimpleFileOptions::default()).unwrap();
            zip.write_all(&serde_json::to_vec(&doc).unwrap()).unwrap();
            zip.finish().unwrap();
        }
        buf.set_position(0);
        let err = read_carnet(buf).err().unwrap();
        assert!(matches!(err, crate::import::ImportError::Native(native::NativeError::NewerVersion(99))));
        assert!(err.message_fr().contains("version plus récente"));
    }
}
