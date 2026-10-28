//! Export ODG (LibreOffice Draw) : chaque élément devient un objet natif, les polices sont intégrées.

mod content;
mod package;
mod styles;
mod xml;

use crate::fonts::FontSet;
use crate::model::{Document, Frame, Id};
use crate::photo::AssetStore;
use package::Part;
use std::io::{Seek, Write};
use std::path::Path;
use styles::{Registry, mm};

/// Marge intérieure des zones de texte, identique à l'écran (CSS `padding`).
pub use crate::model::TEXT_PADDING_MM;

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

/// Mention de licence des images OpenMoji (propriétés du fichier, écran À propos).
pub const OPENMOJI_CREDIT: &str = "Pictogrammes : OpenMoji (openmoji.org), licence CC BY-SA 4.0";
/// Mention écrite en bas de chaque page qui contient une image OpenMoji.
pub const OPENMOJI_CREDIT_PAGE: &str = "Pictogrammes : OpenMoji, CC BY-SA 4.0";

#[derive(Debug)]
pub enum OdgError {
    MissingAsset(Id),
    MissingPicture(String),
    Image,
    Io,
}

impl OdgError {
    /// Nom de l'erreur pour le journal, sans rien du document (ni identifiant, ni code d'image).
    pub fn kind(&self) -> &'static str {
        match self {
            OdgError::MissingAsset(_) => "MissingAsset",
            OdgError::MissingPicture(_) => "MissingPicture",
            OdgError::Image => "Image",
            OdgError::Io => "Io",
        }
    }

    pub fn message_fr(&self) -> String {
        match self {
            OdgError::MissingAsset(_) => "Une photo du document est introuvable.",
            OdgError::MissingPicture(_) => "Une image décorative du document est introuvable.",
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

pub fn export_odg(doc: &Document, store: &AssetStore, fonts: &FontSet, out: impl Write + Seek) -> Result<(), OdgError> {
    let mut reg = Registry::new(fonts);
    let (content, pictures) = content::content_xml(doc, store, &mut reg)?;
    let mut parts = vec![
        Part { path: "content.xml".into(), media_type: "text/xml", bytes: content.into_bytes(), compress: true },
        Part { path: "styles.xml".into(), media_type: "text/xml", bytes: reg.styles_xml().into_bytes(), compress: true },
        Part { path: "meta.xml".into(), media_type: "text/xml", bytes: package::meta_xml(content::has_pictures(doc)).into_bytes(), compress: true },
        Part { path: "settings.xml".into(), media_type: "text/xml", bytes: package::settings_xml().into_bytes(), compress: true },
    ];
    for p in pictures {
        parts.push(Part { path: p.path, media_type: p.media_type, bytes: p.bytes, compress: false });
    }
    for (f, name) in reg.faces.iter().zip(reg.archive_names()).filter(|(f, _)| f.embeddable) {
        let media_type = if f.format == "opentype" { "application/x-font-otf" } else { "application/x-font-ttf" };
        let bytes = f.bytes().ok_or(OdgError::Io)?;
        parts.push(Part { path: format!("Fonts/{name}"), media_type, bytes: bytes.to_vec(), compress: true });
    }
    package::write(parts, out)
}

/// Écriture sûre : un export qui échoue n'écrase pas un export précédent du même nom.
pub fn export_odg_to_path(doc: &Document, store: &AssetStore, fonts: &FontSet, path: &Path) -> Result<(), OdgError> {
    crate::fsutil::write_atomically(path, |f| export_odg(doc, store, fonts, f))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::*;
    use crate::fonts::FontSet;
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
            group: None,
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
            group: None,
            kind: ElementKind::Text(TextProps { content, style: TextStyle::default_zone(), prompt: prompt.map(String::from), frame: None }),
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
        export_with(doc, store, &FontSet::builtin())
    }

    fn export_with(doc: &Document, store: &AssetStore, fonts: &FontSet) -> Exported {
        let mut buf = Cursor::new(Vec::new());
        export_odg(doc, store, fonts, &mut buf).unwrap();
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
        store.insert(a).unwrap();
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
        store.insert(a).unwrap();
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
        store.insert(a).unwrap();
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
    fn vertical_alignment_and_padding_are_exported() {
        let mut t = text(RichText::plain("milieu"), None);
        if let ElementKind::Text(p) = &mut t.kind {
            p.style.valign = Some(VAlign::Bottom);
            p.style.padding_mm = Some(5.0);
        }
        let mut e = export(&doc_with(vec![t, text(RichText::plain("haut"), None)]), &AssetStore::default());
        let content = e.xml("content.xml");
        assert!(content.contains(r#"draw:textarea-vertical-align="bottom""#), "{content}");
        assert!(content.contains(&format!(r#"fo:padding-left="{}""#, mm(5.0))));
        // Un texte sans réglage garde le haut et la marge par défaut.
        assert!(content.contains(r#"draw:textarea-vertical-align="top""#));
        assert!(content.contains(&format!(r#"fo:padding-left="{}""#, mm(TEXT_PADDING_MM))));
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
        assert!(matches!(export_odg(&d, &AssetStore::default(), &FontSet::builtin(), &mut buf), Err(OdgError::MissingAsset(_))));
    }

    #[test]
    fn failed_export_keeps_the_previous_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("cahier.odg");
        std::fs::write(&path, b"export precedent").unwrap();
        let d = doc_with(vec![photo(Some("absent"), frame(10.0, 10.0, 50.0, 50.0), 0.0)]);
        assert!(export_odg_to_path(&d, &AssetStore::default(), &FontSet::builtin(), &path).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), b"export precedent");
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
    }

    #[test]
    fn all_xml_parts_are_well_formed() {
        let mut store = AssetStore::default();
        let a = raw_asset(400, 300);
        let mut rounded = photo(Some(&a.meta.id), frame(10.0, 10.0, 50.0, 50.0), 2.0);
        rounded.frame.rotation_deg = 15.0;
        let d = doc_with(vec![rounded, text(RichText::plain("Bonjour"), None)]);
        store.insert(a).unwrap();
        let mut e = export(&d, &store);
        for part in ["content.xml", "styles.xml", "meta.xml", "settings.xml", "META-INF/manifest.xml"] {
            roxmltree::Document::parse(&e.xml(part)).unwrap_or_else(|err| panic!("{part}: {err}"));
        }
    }

    #[test]
    fn restricted_user_font_is_declared_but_not_embedded() {
        let dir = tempfile::tempdir().unwrap();
        let fixtures = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures");
        for f in ["CarnetTest-Regular.ttf", "CarnetTestRestricted-Regular.ttf"] {
            crate::fonts::add_user_font(dir.path(), &std::path::Path::new(fixtures).join(f)).unwrap();
        }
        let fonts = FontSet::with_user_dir(dir.path());
        let mut free = text(RichText::plain("libre"), None);
        let mut restricted = text(RichText::plain("restreinte"), None);
        if let ElementKind::Text(t) = &mut free.kind {
            t.style.font = "Carnet Test".into();
        }
        if let ElementKind::Text(t) = &mut restricted.kind {
            t.style.font = "Carnet Test Restricted".into();
        }
        let mut e = export_with(&doc_with(vec![free, restricted]), &AssetStore::default(), &fonts);
        let names = e.names();
        assert!(names.contains(&"Fonts/CarnetTest-Regular.ttf".to_string()), "{names:?}");
        assert!(!names.iter().any(|n| n.starts_with("Fonts/CarnetTestRestricted")), "{names:?}");
        let content = e.xml("content.xml");
        assert!(content.contains(r#"style:font-face style:name="Carnet Test Restricted""#), "{content}");
        assert!(content.contains(r#"style:font-name="Carnet Test Restricted""#));
    }

    #[test]
    fn fonts_sharing_a_file_name_are_both_embedded() {
        let dir = tempfile::tempdir().unwrap();
        let src = tempfile::tempdir().unwrap();
        // Police ajoutée dont le fichier porte le nom d'un fichier fourni.
        let named = src.path().join("Andika-Regular.ttf");
        std::fs::copy(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/CarnetTest-Regular.ttf"), &named).unwrap();
        crate::fonts::add_user_font(dir.path(), &named).unwrap();
        let fonts = FontSet::with_user_dir(dir.path());
        let mut user_text = text(RichText::plain("ajoutée"), None);
        if let ElementKind::Text(t) = &mut user_text.kind {
            t.style.font = "Carnet Test".into();
        }
        let mut e = export_with(&doc_with(vec![text(RichText::plain("fournie"), None), user_text]), &AssetStore::default(), &fonts);
        let embedded: Vec<String> = e.names().into_iter().filter(|n| n.starts_with("Fonts/")).collect();
        assert_eq!(embedded.len(), 2, "{embedded:?}");
        let styles = e.xml("content.xml");
        for name in &embedded {
            assert!(styles.contains(&format!("xlink:href=\"{name}\"")), "{name} non déclaré");
        }
    }

    fn shape_el(kind: ShapeKind, f: Frame) -> Element {
        Element {
            id: new_id(),
            frame: f,
            locked: false,
            zone: false,
            group: None,
            kind: ElementKind::Shape(ShapeProps {
                shape: kind,
                fill: Some("#f1c40f".into()),
                stroke: Some(Stroke { width_mm: 1.0, color: "#333333".into(), style: None }),
                corner_radius_mm: 0.0,
            }),
        }
    }

    #[test]
    fn drawn_shapes_are_custom_shapes_with_the_screen_geometry() {
        let mut e = export(&doc_with(vec![shape_el(ShapeKind::Star, frame(20.0, 20.0, 40.0, 40.0))]), &AssetStore::default());
        let content = e.xml("content.xml");
        assert!(content.contains("<draw:custom-shape"), "{content}");
        assert!(content.contains(r#"svg:viewBox="0 0 1000 1000""#));
        let path = crate::shapes::path(ShapeKind::Star).unwrap();
        assert!(content.contains(&format!(r#"draw:enhanced-path="{path} N""#)), "{content}");
    }

    #[test]
    fn arrow_is_a_line_with_an_end_marker() {
        let mut e = export(&doc_with(vec![shape_el(ShapeKind::Arrow, frame(10.0, 20.0, 60.0, 8.0))]), &AssetStore::default());
        let content = e.xml("content.xml");
        assert!(content.contains(r#"svg:x1="10.000mm" svg:y1="24.000mm" svg:x2="70.000mm" svg:y2="24.000mm""#), "{content}");
        assert!(content.contains("draw:marker-end="), "{content}");
        assert!(content.contains(r#"draw:marker-end-width="4.000mm""#), "{content}");
        assert!(e.xml("styles.xml").contains("<draw:marker "));
    }

    #[test]
    fn a_rotated_line_turns_around_its_centre() {
        let mut f = frame(10.0, 20.0, 60.0, 8.0);
        f.rotation_deg = 90.0;
        let mut e = export(&doc_with(vec![shape_el(ShapeKind::Line, f)]), &AssetStore::default());
        let content = e.xml("content.xml");
        assert!(content.contains(r#"svg:x1="40.000mm" svg:y1="-6.000mm" svg:x2="40.000mm" svg:y2="54.000mm""#), "{content}");
        assert!(!e.xml("styles.xml").contains("<draw:marker "));
    }

    fn picture_el(code: &str, group: Option<&str>) -> Element {
        Element {
            id: new_id(),
            frame: frame(20.0, 20.0, 30.0, 30.0),
            locked: false,
            zone: false,
            group: group.map(String::from),
            kind: ElementKind::Image(ImageProps { code: code.into() }),
        }
    }

    #[test]
    fn picture_is_embedded_as_svg() {
        let mut e = export(&doc_with(vec![picture_el("1F98A", None), picture_el("1F98A", None)]), &AssetStore::default());
        let names = e.names();
        assert_eq!(names.iter().filter(|n| *n == "Pictures/openmoji-1F98A.svg").count(), 1, "{names:?}");
        assert_eq!(e.read("Pictures/openmoji-1F98A.svg"), crate::picto::svg("1F98A").unwrap());
        let content = e.xml("content.xml");
        assert_eq!(content.matches(r#"xlink:href="Pictures/openmoji-1F98A.svg""#).count(), 2);
        assert!(e.xml("META-INF/manifest.xml").contains("image/svg+xml"));
    }

    #[test]
    fn group_members_are_wrapped_in_draw_g() {
        let d = doc_with(vec![picture_el("1F98A", Some("g")), shape_el(ShapeKind::Star, frame(60.0, 20.0, 20.0, 20.0)), picture_el("2600", None)]);
        let mut grouped = d.clone();
        grouped.pages[0].elements[1].group = Some("g".into());
        let content = export(&grouped, &AssetStore::default()).xml("content.xml");
        let g = content.split("<draw:g>").nth(1).expect("draw:g").split("</draw:g>").next().unwrap();
        assert!(g.contains("openmoji-1F98A") && g.contains("draw:custom-shape") && !g.contains("openmoji-2600"));
    }

    #[test]
    fn credit_line_only_on_pages_with_pictures() {
        let mut d = doc_with(vec![picture_el("1F98A", None)]);
        d.pages.push(Page::new(A4_PORTRAIT));
        let content = export(&d, &AssetStore::default()).xml("content.xml");
        let pages: Vec<&str> = content.split("<draw:page ").skip(1).collect();
        assert_eq!(pages.len(), 2);
        assert!(pages[0].contains(OPENMOJI_CREDIT_PAGE) && pages[0].contains(r#"draw:protect="position size""#));
        assert!(!pages[1].contains(OPENMOJI_CREDIT_PAGE));
    }

    #[test]
    fn meta_credits_only_when_pictures() {
        assert!(export(&doc_with(vec![picture_el("1F98A", None)]), &AssetStore::default()).xml("meta.xml").contains(OPENMOJI_CREDIT));
        assert!(!export(&doc_with(vec![]), &AssetStore::default()).xml("meta.xml").contains("OpenMoji"));
    }

    #[test]
    fn unknown_picture_code_is_reported() {
        let mut buf = Cursor::new(Vec::new());
        let err = export_odg(&doc_with(vec![picture_el("ZZZZ", None)]), &AssetStore::default(), &FontSet::builtin(), &mut buf).err().unwrap();
        assert!(matches!(err, OdgError::MissingPicture(_)));
        assert_eq!(err.message_fr(), "Une image décorative du document est introuvable.");
    }

    #[test]
    fn error_kind_never_carries_document_text() {
        assert_eq!(OdgError::MissingPicture("code venu du document".into()).kind(), "MissingPicture");
        assert_eq!(OdgError::MissingAsset("id".into()).kind(), "MissingAsset");
    }

    fn with_background(mut d: Document, color: &str, pattern: Option<Pattern>) -> Document {
        d.pages[0].background = Some(Background { color: color.into(), pattern });
        d
    }

    #[test]
    fn background_color_fills_the_page() {
        let mut e = export(&with_background(doc_with(vec![]), crate::background::KRAFT, None), &AssetStore::default());
        let styles = e.xml("styles.xml");
        assert!(styles.contains(r#"draw:fill="solid""#) && styles.contains(r##"draw:fill-color="#d8bc8a""##), "{styles}");
    }

    #[test]
    fn pattern_is_drawn_on_the_master_page() {
        let mut e = export(&with_background(doc_with(vec![]), "#ffffff", Some(Pattern::Seyes)), &AssetStore::default());
        let styles = e.xml("styles.xml");
        let master = styles.split("<style:master-page").nth(1).unwrap();
        assert_eq!(master.matches("<draw:line").count(), 148 + 26 + 1);
        assert!(!e.xml("content.xml").contains("<draw:line"));
    }

    #[test]
    fn pages_with_different_backgrounds_get_their_own_master() {
        let mut d = with_background(doc_with(vec![]), "#ffffff", Some(Pattern::Lines));
        d.pages.push(Page { background: Some(Background { color: "#ffeecc".into(), pattern: None }), ..Page::new(A4_PORTRAIT) });
        d.pages.push(Page::new(A4_PORTRAIT));
        let mut e = export(&d, &AssetStore::default());
        let content = e.xml("content.xml");
        let masters: Vec<&str> = content.split(r#"draw:master-page-name=""#).skip(1).map(|s| s.split('"').next().unwrap()).collect();
        assert_eq!(masters.len(), 3);
        assert!(masters[0] != masters[1] && masters[1] != masters[2] && masters[0] != masters[2], "{masters:?}");
    }
}
