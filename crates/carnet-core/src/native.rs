//! Format natif : archive zip avec `document.json`, les images utilisées et les polices ajoutées
//! par l'utilisateur qu'il utilise (si leur licence le permet).

pub use crate::model::FORMAT_VERSION;
use crate::fonts::FontSet;
use crate::model::{Document, Element, ElementKind, Mime, Stroke, TEXT_PADDING_MM, VAlign};
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
const FONTS_DIR: &str = "fonts/";

/// Document relu : ses images, et les polices utilisateur qu'il transporte (nom de fichier, octets).
pub struct LoadedDocument {
    pub doc: Document,
    pub assets: Vec<Asset>,
    pub fonts: Vec<(String, Vec<u8>)>,
}

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
    doc.pages.iter().flat_map(|p| &p.elements).flat_map(Element::asset_ids).cloned().collect()
}

/// 2 si le document contient une image (un Carnet plus ancien affiche alors « version plus récente »), sinon 1.
pub fn written_version(doc: &Document) -> u32 {
    let elements = || doc.pages.iter().flat_map(|p| &p.elements);
    if elements().any(uses_version_3) {
        3
    } else if elements().any(|e| matches!(e.kind, ElementKind::Image(_))) {
        2
    } else {
        1
    }
}

/// Réglages inconnus de la version 2 : alignement vertical, marge, cadre de texte, bordure non pleine.
fn uses_version_3(e: &Element) -> bool {
    let dashed = |s: &Option<Stroke>| s.as_ref().is_some_and(|s| !s.is_solid());
    match &e.kind {
        ElementKind::Text(t) => {
            t.style.valign.unwrap_or_default() != VAlign::Top || t.style.padding_mm.is_some_and(|p| p != TEXT_PADDING_MM) || t.frame.is_some()
        }
        ElementKind::Shape(s) => dashed(&s.stroke),
        ElementKind::Photo(p) => dashed(&p.border),
        ElementKind::Image(_) => false,
    }
}

fn used_families(doc: &Document) -> BTreeSet<&str> {
    let mut out = BTreeSet::new();
    for e in doc.pages.iter().flat_map(|p| &p.elements) {
        if let ElementKind::Text(t) = &e.kind {
            out.insert(t.style.font.as_str());
            out.extend(t.content.paragraphs.iter().flat_map(|p| &p.runs).filter_map(|r| r.font.as_deref()));
        }
    }
    out
}

/// Seules les images utilisées par un élément sont enregistrées, et seules les polices ajoutées ou
/// du poste, utilisées et intégrables.
pub fn save(doc: &Document, store: &AssetStore, fonts: &FontSet, out: impl Write + Seek) -> Result<(), NativeError> {
    let used = used_assets(doc);
    let mut doc = doc.clone();
    doc.assets.retain(|id, _| used.contains(id));
    doc.format_version = written_version(&doc);

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
    let families = used_families(&doc);
    let carried: Vec<_> = fonts.faces().iter().filter(|f| (f.user || f.system) && f.embeddable && families.contains(f.family.as_str())).collect();
    let names = crate::fonts::unique_names(carried.iter().map(|f| f.file_name.as_str()));
    for (f, name) in carried.into_iter().zip(names) {
        zip.start_file(format!("{FONTS_DIR}{name}"), deflated)?;
        zip.write_all(&f.bytes().ok_or(NativeError::Io)?)?;
    }
    zip.finish()?;
    Ok(())
}

/// Écriture sûre : un échec ne détruit pas l'ancien fichier.
pub fn save_to_path(doc: &Document, store: &AssetStore, fonts: &FontSet, path: &Path) -> Result<(), NativeError> {
    crate::fsutil::write_atomically(path, |f| save(doc, store, fonts, f))
}

fn read_entry<R: Read + Seek>(zip: &mut ZipArchive<R>, name: &str) -> Result<Vec<u8>, NativeError> {
    let mut entry = zip.by_name(name)?;
    let mut buf = Vec::new();
    entry.read_to_end(&mut buf)?;
    Ok(buf)
}

pub fn load(input: impl Read + Seek) -> Result<LoadedDocument, NativeError> {
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
    let font_entries: Vec<String> = zip
        .file_names()
        .filter_map(|n| n.strip_prefix(FONTS_DIR))
        .filter(|n| !n.is_empty() && !n.contains('/'))
        .map(String::from)
        .collect();
    let mut fonts = Vec::new();
    for name in font_entries {
        fonts.push((name.clone(), read_entry(&mut zip, &format!("{FONTS_DIR}{name}"))?));
    }
    Ok(LoadedDocument { doc, assets, fonts })
}

pub fn load_from_path(path: &Path) -> Result<LoadedDocument, NativeError> {
    load(std::fs::File::open(path)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fonts::FontSet;
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
            group: None,
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
        save(&doc, &store, &FontSet::builtin(), &mut buf).unwrap();
        buf.set_position(0);
        let LoadedDocument { doc: back, assets, .. } = load(buf).unwrap();
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
        save(&doc, &store, &FontSet::builtin(), &mut buf).unwrap();
        buf.set_position(0);
        let LoadedDocument { doc: back, assets, .. } = load(buf).unwrap();
        assert!(!back.assets.contains_key(&unused));
        assert!(assets.iter().all(|a| a.meta.id != unused));
    }

    #[test]
    fn missing_image_is_reported_as_such() {
        let (doc, _, _) = sample();
        let mut buf = Cursor::new(Vec::new());
        let err = save(&doc, &AssetStore::default(), &FontSet::builtin(), &mut buf).err().unwrap();
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
        save_to_path(&doc, &store, &FontSet::builtin(), &path).unwrap();
        let LoadedDocument { doc: back, assets, .. } = load_from_path(&path).unwrap();
        assert_eq!(back.title, "Cahier été");
        assert_eq!(assets.len(), 2);
        let leftovers: Vec<_> = std::fs::read_dir(&folder).unwrap().map(|e| e.unwrap().file_name()).collect();
        assert_eq!(leftovers, vec![std::ffi::OsString::from("cahier été.carnet")]);
    }

    #[test]
    fn embeddable_user_font_travels_in_the_document() {
        let dir = tempfile::tempdir().unwrap();
        let fixtures = std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures"));
        for f in ["CarnetTest-Regular.ttf", "CarnetTestRestricted-Regular.ttf"] {
            crate::fonts::add_user_font(dir.path(), &fixtures.join(f)).unwrap();
        }
        let fonts = FontSet::with_user_dir(dir.path());
        let mut doc = Document::new("polices");
        for family in ["Carnet Test", "Carnet Test Restricted"] {
            let mut style = TextStyle::default_zone();
            style.font = family.into();
            doc.pages[0].elements.push(Element {
                id: new_id(),
                frame: Frame { x_mm: 10.0, y_mm: 10.0, w_mm: 80.0, h_mm: 20.0, rotation_deg: 0.0 },
                locked: false,
                zone: false,
                group: None,
                kind: ElementKind::Text(TextProps { content: RichText::plain("bonjour"), style, prompt: None, frame: None }),
            });
        }
        let mut buf = Cursor::new(Vec::new());
        save(&doc, &AssetStore::default(), &fonts, &mut buf).unwrap();
        buf.set_position(0);
        let loaded = load(buf).unwrap();
        let original = std::fs::read(fixtures.join("CarnetTest-Regular.ttf")).unwrap();
        assert_eq!(loaded.fonts, vec![("CarnetTest-Regular.ttf".to_string(), original)]);
    }

    #[test]
    fn embeddable_system_font_travels_in_the_document() {
        let sys = tempfile::tempdir().unwrap();
        let fixtures = std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures"));
        std::fs::copy(fixtures.join("CarnetTest-Regular.ttf"), sys.path().join("carnettest.ttf")).unwrap();
        let fonts = FontSet::builtin().with_system(&crate::fonts::scan_font_dirs(&[sys.path()]));
        let mut doc = Document::new("poste");
        let mut style = TextStyle::default_zone();
        style.font = "Carnet Test".into();
        doc.pages[0].elements.push(Element {
            id: new_id(),
            frame: Frame { x_mm: 10.0, y_mm: 10.0, w_mm: 80.0, h_mm: 20.0, rotation_deg: 0.0 },
            locked: false,
            zone: false,
            group: None,
            kind: ElementKind::Text(TextProps { content: RichText::plain("bonjour"), style, prompt: None, frame: None }),
        });
        let mut buf = Cursor::new(Vec::new());
        save(&doc, &AssetStore::default(), &fonts, &mut buf).unwrap();
        buf.set_position(0);
        let names: Vec<String> = load(buf).unwrap().fonts.into_iter().map(|f| f.0).collect();
        assert_eq!(names, vec!["carnettest.ttf".to_string()]);
    }

    #[test]
    fn carried_fonts_sharing_a_file_name_are_both_saved() {
        let user = tempfile::tempdir().unwrap();
        let sys = tempfile::tempdir().unwrap();
        let fixtures = std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures"));
        let src = tempfile::tempdir().unwrap();
        std::fs::copy(fixtures.join("CarnetTest-Regular.ttf"), src.path().join("police.ttf")).unwrap();
        crate::fonts::add_user_font(user.path(), &src.path().join("police.ttf")).unwrap();
        std::fs::copy(fixtures.join("CarnetAutre-Regular.ttf"), sys.path().join("police.ttf")).unwrap();
        let fonts = FontSet::with_user_dir(user.path()).with_system(&crate::fonts::scan_font_dirs(&[sys.path()]));
        let mut doc = Document::new("deux");
        for family in ["Carnet Test", "Carnet Autre"] {
            let mut style = TextStyle::default_zone();
            style.font = family.into();
            doc.pages[0].elements.push(Element {
                id: new_id(),
                frame: Frame { x_mm: 10.0, y_mm: 10.0, w_mm: 80.0, h_mm: 20.0, rotation_deg: 0.0 },
                locked: false,
                zone: false,
                group: None,
                kind: ElementKind::Text(TextProps { content: RichText::plain("x"), style, prompt: None, frame: None }),
            });
        }
        let mut buf = Cursor::new(Vec::new());
        save(&doc, &AssetStore::default(), &fonts, &mut buf).unwrap();
        buf.set_position(0);
        let families: Vec<String> = load(buf).unwrap().fonts.iter().filter_map(|(_, b)| crate::fonts::family_of(b)).collect();
        assert_eq!(families.len(), 2, "{families:?}");
    }

    fn picture(group: Option<&str>) -> Element {
        Element {
            id: new_id(),
            frame: Frame { x_mm: 10.0, y_mm: 10.0, w_mm: 20.0, h_mm: 20.0, rotation_deg: 0.0 },
            locked: false,
            zone: false,
            group: group.map(String::from),
            kind: ElementKind::Image(ImageProps { code: "1F98A".into() }),
        }
    }

    #[test]
    fn written_version_is_2_only_with_images() {
        let mut doc = Document::new("v");
        assert_eq!(written_version(&doc), 1);
        doc.pages[0].elements.push(picture(None));
        assert_eq!(written_version(&doc), 2);
        let mut buf = Cursor::new(Vec::new());
        save(&doc, &AssetStore::default(), &FontSet::builtin(), &mut buf).unwrap();
        buf.set_position(0);
        assert_eq!(load(buf).unwrap().doc.format_version, 2);
    }

    #[test]
    fn newer_than_3_is_refused() {
        let mut doc = Document::new("futur");
        doc.format_version = 4;
        let json = serde_json::to_vec(&doc).unwrap();
        let mut buf = Cursor::new(Vec::new());
        {
            let mut zip = ZipWriter::new(&mut buf);
            zip.start_file(DOCUMENT_ENTRY, SimpleFileOptions::default()).unwrap();
            zip.write_all(&json).unwrap();
            zip.finish().unwrap();
        }
        buf.set_position(0);
        let err = load(buf).err().unwrap();
        assert_eq!(err, NativeError::NewerVersion(4));
        assert!(err.message_fr().contains("plus récente"));
    }

    fn text_el(style: TextStyle, frame: Option<TextFrame>) -> Element {
        Element {
            id: new_id(),
            frame: Frame { x_mm: 10.0, y_mm: 10.0, w_mm: 80.0, h_mm: 20.0, rotation_deg: 0.0 },
            locked: false,
            zone: false,
            group: None,
            kind: ElementKind::Text(TextProps { content: RichText::plain("x"), style, prompt: None, frame }),
        }
    }

    #[test]
    fn text_frame_image_is_saved_with_the_document() {
        let mut doc = Document::new("étiquettes");
        let mut store = AssetStore::default();
        let a = photo_asset();
        let id = a.meta.id.clone();
        doc.assets.insert(id.clone(), a.meta.clone());
        store.insert(a).unwrap();
        doc.pages[0].elements.push(text_el(TextStyle::default_zone(), Some(TextFrame { image: Some(id.clone()), ..TextFrame::default() })));
        let mut buf = Cursor::new(Vec::new());
        save(&doc, &store, &FontSet::builtin(), &mut buf).unwrap();
        buf.set_position(0);
        let loaded = load(buf).unwrap();
        assert!(loaded.doc.assets.contains_key(&id));
        assert_eq!(loaded.assets.len(), 1);
    }

    /// Chaque réglage de la version 3 l'impose : la 0.4 le perdrait en silence (champs inconnus ignorés).
    #[test]
    fn written_version_is_3_with_any_frame_or_border_setting() {
        let with = |e: Element| {
            let mut doc = Document::new("v3");
            doc.pages[0].elements.push(picture(None));
            doc.pages[0].elements.push(e);
            written_version(&doc)
        };
        let plain = TextStyle::default_zone();
        assert_eq!(with(text_el(plain.clone(), None)), 2);
        assert_eq!(with(text_el(TextStyle { valign: Some(VAlign::Top), padding_mm: None, ..plain.clone() }, None)), 2);
        assert_eq!(with(text_el(TextStyle { valign: Some(VAlign::Middle), ..plain.clone() }, None)), 3);
        assert_eq!(with(text_el(TextStyle { padding_mm: Some(4.0), ..plain.clone() }, None)), 3);
        // La marge par défaut écrite telle quelle ne perd rien en 0.4.
        assert_eq!(with(text_el(TextStyle { padding_mm: Some(TEXT_PADDING_MM), ..plain.clone() }, None)), 2);
        let frame = TextFrame { fill: Some("#ffeeaa".into()), ..TextFrame::default() };
        assert_eq!(with(text_el(plain.clone(), Some(frame))), 3);
        let dashed = Stroke { width_mm: 1.0, color: "#000000".into(), style: Some(StrokeStyle::Dashed) };
        let mut shape = picture(None);
        shape.kind = ElementKind::Shape(ShapeProps { shape: ShapeKind::Rect, fill: None, stroke: Some(dashed.clone()), corner_radius_mm: 0.0 });
        assert_eq!(with(shape.clone()), 3);
        shape.kind = ElementKind::Shape(ShapeProps { shape: ShapeKind::Rect, fill: None, stroke: Some(Stroke { style: Some(StrokeStyle::Solid), ..dashed.clone() }), corner_radius_mm: 0.0 });
        assert_eq!(with(shape), 2);
        let mut photo = picture(None);
        photo.kind = ElementKind::Photo(PhotoProps { asset: None, crop: Crop::COVER, corner_radius_mm: 0.0, border: Some(dashed), shadow: false });
        assert_eq!(with(photo), 3);
        let mut doc = Document::new("v3");
        doc.pages[0].elements.push(text_el(TextStyle { valign: Some(VAlign::Bottom), ..plain }, None));
        assert_eq!(written_version(&doc), 3);
    }
}
