//! Export ODG (LibreOffice Draw) : chaque élément devient un objet natif, les polices sont intégrées.

mod content;
mod package;
mod styles;
mod xml;

use crate::model::{Document, Frame, Id};
use crate::photo::AssetStore;
use package::Part;
use std::io::{Seek, Write};
use std::path::Path;
use styles::{Registry, mm};

/// Marge intérieure des zones de texte, identique à l'écran (CSS `padding`).
pub const TEXT_PADDING_MM: f64 = 1.5;

pub(crate) const NS: [(&str, &str); 11] = [
    ("xmlns:office", "urn:oasis:names:tc:opendocument:xmlns:office:1.0"),
    ("xmlns:style", "urn:oasis:names:tc:opendocument:xmlns:style:1.0"),
    ("xmlns:text", "urn:oasis:names:tc:opendocument:xmlns:text:1.0"),
    ("xmlns:draw", "urn:oasis:names:tc:opendocument:xmlns:drawing:1.0"),
    ("xmlns:fo", "urn:oasis:names:tc:opendocument:xmlns:xsl-fo-compatible:1.0"),
    ("xmlns:svg", "urn:oasis:names:tc:opendocument:xmlns:svg-compatible:1.0"),
    ("xmlns:xlink", "http://www.w3.org/1999/xlink"),
    ("xmlns:config", "urn:oasis:names:tc:opendocument:xmlns:config:1.0"),
    ("xmlns:meta", "urn:oasis:names:tc:opendocument:xmlns:meta:1.0"),
    ("xmlns:dc", "http://purl.org/dc/elements/1.1/"),
    ("xmlns:loext", "urn:org:documentfoundation:names:experimental:office:xmlns:loext:1.0"),
];

#[derive(Debug)]
pub enum OdgError {
    MissingAsset(Id),
    Image,
    Io,
}

impl OdgError {
    pub fn message_fr(&self) -> String {
        match self {
            OdgError::MissingAsset(_) => "Une photo du document est introuvable.",
            OdgError::Image => "Une photo du document n'a pas pu être préparée.",
            OdgError::Io => "Impossible d'écrire le fichier.",
        }
        .to_string()
    }
}

impl From<std::io::Error> for OdgError {
    fn from(_: std::io::Error) -> Self {
        OdgError::Io
    }
}

impl From<zip::result::ZipError> for OdgError {
    fn from(_: zip::result::ZipError) -> Self {
        OdgError::Io
    }
}

/// `draw:transform` d'un cadre tourné. L'écran tourne autour du centre, dans le sens horaire ;
/// l'ODF tourne autour du coin haut gauche, dans le sens antihoraire, puis translate ce coin.
pub(crate) fn odf_transform(f: &Frame) -> Option<String> {
    if f.rotation_deg == 0.0 {
        return None;
    }
    let t = f.rotation_deg.to_radians();
    let (cx, cy) = (f.x_mm + f.w_mm / 2.0, f.y_mm + f.h_mm / 2.0);
    let x = cx - (f.w_mm / 2.0) * t.cos() + (f.h_mm / 2.0) * t.sin();
    let y = cy - (f.w_mm / 2.0) * t.sin() - (f.h_mm / 2.0) * t.cos();
    Some(format!("rotate({:.6}) translate({} {})", -t, mm(x), mm(y)))
}

pub fn export_odg(doc: &Document, store: &AssetStore, out: impl Write + Seek) -> Result<(), OdgError> {
    let mut reg = Registry::default();
    let (content, pictures) = content::content_xml(doc, store, &mut reg)?;
    let mut parts = vec![
        Part { path: "content.xml".into(), media_type: "text/xml", bytes: content.into_bytes(), compress: true },
        Part { path: "styles.xml".into(), media_type: "text/xml", bytes: reg.styles_xml().into_bytes(), compress: true },
        Part { path: "meta.xml".into(), media_type: "text/xml", bytes: package::meta_xml().into_bytes(), compress: true },
        Part { path: "settings.xml".into(), media_type: "text/xml", bytes: package::settings_xml().into_bytes(), compress: true },
    ];
    for p in pictures {
        parts.push(Part { path: p.path, media_type: p.media_type, bytes: p.bytes, compress: false });
    }
    for f in &reg.faces {
        let media_type = if f.format == "opentype" { "application/x-font-otf" } else { "application/x-font-ttf" };
        parts.push(Part { path: format!("Fonts/{}", f.file_name), media_type, bytes: f.bytes.to_vec(), compress: true });
    }
    package::write(parts, out)
}

pub fn export_odg_to_path(doc: &Document, store: &AssetStore, path: &Path) -> Result<(), OdgError> {
    let file = std::fs::File::create(path)?;
    export_odg(doc, store, file)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::*;
    use crate::photo::{Asset, AssetStore};
    use image::{ImageFormat, Rgb, RgbImage};
    use std::io::{Cursor, Read};
    use std::sync::Arc;
    use zip::ZipArchive;

    fn frame(x: f64, y: f64, w: f64, h: f64) -> Frame {
        Frame { x_mm: x, y_mm: y, w_mm: w, h_mm: h, rotation_deg: 0.0 }
    }

    fn raw_asset(w: u32, h: u32) -> Asset {
        let mut out = Cursor::new(Vec::new());
        RgbImage::from_pixel(w, h, Rgb([90, 140, 200])).write_to(&mut out, ImageFormat::Jpeg).unwrap();
        let bytes = Arc::new(out.into_inner());
        Asset { meta: AssetMeta { id: new_id(), width_px: w, height_px: h, mime: Mime::Jpeg }, thumb: bytes.clone(), bytes }
    }

    fn photo(asset: Option<&str>, f: Frame, radius: f64) -> Element {
        Element {
            id: new_id(),
            frame: f,
            locked: false,
            zone: true,
            kind: ElementKind::Photo(PhotoProps {
                asset: asset.map(String::from),
                crop: Crop::COVER,
                corner_radius_mm: radius,
                border: None,
                shadow: false,
            }),
        }
    }

    fn text(content: RichText, prompt: Option<&str>) -> Element {
        Element {
            id: new_id(),
            frame: frame(10.0, 200.0, 190.0, 40.0),
            locked: false,
            zone: true,
            kind: ElementKind::Text(TextProps { content, style: TextStyle::default_zone(), prompt: prompt.map(String::from) }),
        }
    }

    struct Exported {
        zip: ZipArchive<Cursor<Vec<u8>>>,
    }

    impl Exported {
        fn read(&mut self, name: &str) -> Vec<u8> {
            let mut buf = Vec::new();
            self.zip.by_name(name).unwrap().read_to_end(&mut buf).unwrap();
            buf
        }
        fn xml(&mut self, name: &str) -> String {
            String::from_utf8(self.read(name)).unwrap()
        }
        fn names(&self) -> Vec<String> {
            self.zip.file_names().map(String::from).collect()
        }
    }

    fn export(doc: &Document, store: &AssetStore) -> Exported {
        let mut buf = Cursor::new(Vec::new());
        export_odg(doc, store, &mut buf).unwrap();
        Exported { zip: ZipArchive::new(Cursor::new(buf.into_inner())).unwrap() }
    }

    fn doc_with(elements: Vec<Element>) -> Document {
        let mut d = Document::new("essai");
        d.pages[0].elements = elements;
        d
    }

    fn all_text(xml: &str) -> String {
        let doc = roxmltree::Document::parse(xml).unwrap();
        doc.descendants().filter(|n| n.is_text()).map(|n| n.text().unwrap()).collect()
    }

    #[test]
    fn mimetype_comes_first_and_is_stored() {
        let mut e = export(&Document::new("vide"), &AssetStore::default());
        let first = e.zip.by_index(0).unwrap();
        assert_eq!(first.name(), "mimetype");
        assert_eq!(first.compression(), zip::CompressionMethod::Stored);
        drop(first);
        assert_eq!(e.read("mimetype"), b"application/vnd.oasis.opendocument.graphics");
    }

    #[test]
    fn manifest_lists_every_file() {
        let mut store = AssetStore::default();
        let a = raw_asset(400, 300);
        let d = doc_with(vec![photo(Some(&a.meta.id), frame(10.0, 10.0, 50.0, 50.0), 0.0), text(RichText::plain("Bonjour"), None)]);
        store.insert(a);
        let mut e = export(&d, &store);
        let manifest = e.xml("META-INF/manifest.xml");
        for name in e.names() {
            if name == "mimetype" || name == "META-INF/manifest.xml" {
                continue;
            }
            assert!(manifest.contains(&format!("manifest:full-path=\"{name}\"")), "{name} absent du manifeste");
        }
        assert!(e.names().iter().any(|n| n.starts_with("Pictures/")));
        assert!(e.names().iter().any(|n| n.starts_with("Fonts/")));
    }

    #[test]
    fn one_draw_page_per_page() {
        let mut d = Document::new("deux pages");
        d.pages.push(Page::new(A4_PORTRAIT));
        let mut e = export(&d, &AssetStore::default());
        assert_eq!(e.xml("content.xml").matches("<draw:page ").count(), 2);
    }

    #[test]
    fn empty_zones_are_not_exported() {
        let d = doc_with(vec![photo(None, frame(10.0, 10.0, 50.0, 50.0), 0.0), text(RichText::default(), Some("Texte"))]);
        let mut e = export(&d, &AssetStore::default());
        let content = e.xml("content.xml");
        assert!(!content.contains("draw:image"));
        assert!(!content.contains("draw:frame"));
        assert!(!all_text(&content).contains("Texte"));
    }

    #[test]
    fn photo_is_precropped() {
        let mut store = AssetStore::default();
        let a = raw_asset(4000, 3000);
        let el = photo(Some(&a.meta.id), frame(10.0, 10.0, 100.0, 100.0), 0.0);
        let id = el.id.clone();
        store.insert(a);
        let mut e = export(&doc_with(vec![el]), &store);
        let pic = image::load_from_memory(&e.read(&format!("Pictures/{id}.jpg"))).unwrap();
        assert_eq!((pic.width(), pic.height()), (3000, 3000));
        assert!(e.xml("content.xml").contains("<draw:image"));
    }

    #[test]
    fn rounded_photo_uses_bitmap_fill() {
        let mut store = AssetStore::default();
        let a = raw_asset(400, 300);
        let d = doc_with(vec![photo(Some(&a.meta.id), frame(10.0, 10.0, 50.0, 50.0), 3.0)]);
        store.insert(a);
        let mut e = export(&d, &store);
        let content = e.xml("content.xml");
        assert!(content.contains("<draw:rect "));
        assert!(content.contains("draw:corner-radius=\"3.000mm\""));
        assert!(content.contains("draw:fill=\"bitmap\""));
        assert!(e.xml("styles.xml").contains("<draw:fill-image "));
    }

    #[test]
    fn rotation_matches_formula() {
        let f = Frame { x_mm: 50.0, y_mm: 50.0, w_mm: 40.0, h_mm: 20.0, rotation_deg: 90.0 };
        assert_eq!(odf_transform(&f).unwrap(), "rotate(-1.570796) translate(80.000mm 40.000mm)");
        assert!(odf_transform(&frame(1.0, 2.0, 3.0, 4.0)).is_none());
    }

    #[test]
    fn text_is_escaped() {
        let d = doc_with(vec![text(RichText::plain("Ça & là <ok> « œuf »"), None)]);
        let mut e = export(&d, &AssetStore::default());
        assert!(all_text(&e.xml("content.xml")).contains("Ça & là <ok> « œuf »"));
    }

    #[test]
    fn repeated_spaces_are_kept() {
        let d = doc_with(vec![text(RichText::plain("a   b"), None)]);
        let mut e = export(&d, &AssetStore::default());
        assert!(e.xml("content.xml").contains("a <text:s text:c=\"2\"/>b"));
    }

    #[test]
    fn control_characters_never_break_the_xml() {
        // U+000B est le retour à la ligne manuel de Word dans le texte brut copié.
        let d = doc_with(vec![text(RichText::plain("avant\u{b}après\u{c}fin\u{1}\u{7f}!"), None)]);
        let mut e = export(&d, &AssetStore::default());
        let content = e.xml("content.xml");
        roxmltree::Document::parse(&content).unwrap();
        assert!(content.contains("avant<text:line-break/>après<text:line-break/>fin!"));
    }

    #[test]
    fn bold_run_gets_bold_style() {
        let mut rt = RichText::plain("Fort");
        rt.paragraphs[0].runs[0].bold = true;
        let d = doc_with(vec![text(rt, None)]);
        let mut e = export(&d, &AssetStore::default());
        let content = e.xml("content.xml");
        let doc = roxmltree::Document::parse(&content).unwrap();
        let span = doc.descendants().find(|n| n.has_tag_name("span")).unwrap();
        let style_name = span.attributes().find(|a| a.name() == "style-name").unwrap().value();
        let style = doc
            .descendants()
            .find(|n| n.has_tag_name("style") && n.attributes().any(|a| a.name() == "name" && a.value() == style_name))
            .unwrap();
        let props = style.children().find(|n| n.has_tag_name("text-properties")).unwrap();
        assert_eq!(props.attributes().find(|a| a.name() == "font-weight").unwrap().value(), "bold");
    }

    #[test]
    fn used_fonts_are_embedded() {
        let d = doc_with(vec![text(RichText::plain("Bonjour"), None)]);
        let mut e = export(&d, &AssetStore::default());
        assert!(e.names().contains(&"Fonts/Andika-Regular.ttf".to_string()));
        assert!(e.xml("content.xml").contains("xlink:href=\"Fonts/Andika-Regular.ttf\""));
        let settings = e.xml("settings.xml");
        assert!(settings.contains("config:name=\"EmbedFonts\" config:type=\"boolean\">true<"));
    }

    #[test]
    fn unused_fonts_are_not_embedded() {
        let d = doc_with(vec![text(RichText::plain("Bonjour"), None)]);
        let e = export(&d, &AssetStore::default());
        assert!(!e.names().iter().any(|n| n.contains("OpenDyslexic")));
    }

    #[test]
    fn missing_asset_is_an_error() {
        let d = doc_with(vec![photo(Some("absent"), frame(10.0, 10.0, 50.0, 50.0), 0.0)]);
        let mut buf = Cursor::new(Vec::new());
        assert!(matches!(export_odg(&d, &AssetStore::default(), &mut buf), Err(OdgError::MissingAsset(_))));
    }

    #[test]
    fn all_xml_parts_are_well_formed() {
        let mut store = AssetStore::default();
        let a = raw_asset(400, 300);
        let mut rounded = photo(Some(&a.meta.id), frame(10.0, 10.0, 50.0, 50.0), 2.0);
        rounded.frame.rotation_deg = 15.0;
        let d = doc_with(vec![rounded, text(RichText::plain("Bonjour"), None)]);
        store.insert(a);
        let mut e = export(&d, &store);
        for part in ["content.xml", "styles.xml", "meta.xml", "settings.xml", "META-INF/manifest.xml"] {
            roxmltree::Document::parse(&e.xml(part)).unwrap_or_else(|err| panic!("{part}: {err}"));
        }
    }
}
