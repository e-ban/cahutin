//! Modèle de document. Toutes les géométries sont en millimètres.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use ts_rs::TS;

pub type Id = String;

pub fn new_id() -> Id {
    uuid::Uuid::new_v4().to_string()
}

pub const FORMAT_VERSION: u32 = 1;
pub const A4_PORTRAIT: PageSize = PageSize { width_mm: 210.0, height_mm: 297.0 };

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, TS)]
#[ts(export)]
pub struct Document {
    pub format_version: u32,
    pub title: String,
    pub pages: Vec<Page>,
    pub assets: BTreeMap<Id, AssetMeta>,
    /// Chemins des photos importées mais non utilisées, pour retrouver la photothèque à la réouverture.
    #[serde(default)]
    pub pending_photos: Vec<String>,
}

impl Document {
    pub fn new(title: &str) -> Document {
        Document {
            format_version: FORMAT_VERSION,
            title: title.to_string(),
            pages: vec![Page::new(A4_PORTRAIT)],
            assets: BTreeMap::new(),
            pending_photos: Vec::new(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, TS)]
#[ts(export)]
pub struct AssetMeta {
    pub id: Id,
    pub width_px: u32,
    pub height_px: u32,
    pub mime: Mime,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, TS)]
#[ts(export)]
pub enum Mime {
    Jpeg,
    Png,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, TS)]
#[ts(export)]
pub struct Page {
    pub id: Id,
    pub size: PageSize,
    pub elements: Vec<Element>,
    pub grid: Option<GridLayout>,
}

impl Page {
    pub fn new(size: PageSize) -> Page {
        Page { id: new_id(), size, elements: vec![], grid: None }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, TS)]
#[ts(export)]
pub struct PageSize {
    pub width_mm: f64,
    pub height_mm: f64,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, TS)]
#[ts(export)]
pub struct Frame {
    pub x_mm: f64,
    pub y_mm: f64,
    pub w_mm: f64,
    pub h_mm: f64,
    pub rotation_deg: f64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, TS)]
#[ts(export)]
pub struct Element {
    pub id: Id,
    pub frame: Frame,
    pub locked: bool,
    /// Zone à remplir, issue d'un modèle.
    pub zone: bool,
    pub kind: ElementKind,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, TS)]
#[serde(tag = "type")]
#[ts(export)]
pub enum ElementKind {
    Photo(PhotoProps),
    Text(TextProps),
    Shape(ShapeProps),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, TS)]
#[ts(export)]
pub struct PhotoProps {
    pub asset: Option<Id>,
    pub crop: Crop,
    pub corner_radius_mm: f64,
    pub border: Option<Stroke>,
    pub shadow: bool,
}

/// `scale` >= 1 ; le centre de la partie visible, en fraction de l'image.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, TS)]
#[ts(export)]
pub struct Crop {
    pub scale: f64,
    pub center_x: f64,
    pub center_y: f64,
}

impl Crop {
    pub const COVER: Crop = Crop { scale: 1.0, center_x: 0.5, center_y: 0.5 };
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, TS)]
#[ts(export)]
pub struct Stroke {
    pub width_mm: f64,
    /// "#rrggbb"
    pub color: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, TS)]
#[ts(export)]
pub struct TextProps {
    pub content: RichText,
    pub style: TextStyle,
    pub prompt: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, TS)]
#[ts(export)]
pub struct TextStyle {
    pub font: String,
    pub size_pt: f64,
    pub color: String,
    pub align: Align,
}

impl TextStyle {
    pub fn default_zone() -> TextStyle {
        TextStyle { font: "Andika".into(), size_pt: 14.0, color: "#222222".into(), align: Align::Left }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, TS)]
#[ts(export)]
pub enum Align {
    Left,
    Center,
    Right,
    Justify,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default, TS)]
#[ts(export)]
pub struct RichText {
    pub paragraphs: Vec<Paragraph>,
}

impl RichText {
    pub fn is_blank(&self) -> bool {
        self.paragraphs.iter().all(|p| p.runs.iter().all(|r| r.text.trim().is_empty()))
    }

    pub fn plain(s: &str) -> RichText {
        RichText {
            paragraphs: s
                .split('\n')
                .map(|line| Paragraph { align: None, runs: vec![Run::plain(line)] })
                .collect(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, TS)]
#[ts(export)]
pub struct Paragraph {
    pub align: Option<Align>,
    pub runs: Vec<Run>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, TS)]
#[ts(export)]
pub struct Run {
    pub text: String,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub color: Option<String>,
    pub font: Option<String>,
    pub size_pt: Option<f64>,
}

impl Run {
    pub fn plain(text: &str) -> Run {
        Run {
            text: text.to_string(),
            bold: false,
            italic: false,
            underline: false,
            color: None,
            font: None,
            size_pt: None,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, TS)]
#[ts(export)]
pub struct ShapeProps {
    pub shape: ShapeKind,
    pub fill: Option<String>,
    pub stroke: Option<Stroke>,
    pub corner_radius_mm: f64,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, TS)]
#[ts(export)]
pub enum ShapeKind {
    Rect,
    Ellipse,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, TS)]
#[ts(export)]
pub enum ZoneKind {
    Photo,
    Text,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, TS)]
#[ts(export)]
pub struct GridLayout {
    pub cols: u8,
    pub rows: u8,
    pub margin_mm: f64,
    pub gap_mm: f64,
    pub cells: Vec<GridCell>,
}

/// Une case de la grille ; identifiée par l'élément qu'elle place.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, TS)]
#[ts(export)]
pub struct GridCell {
    pub element_id: Id,
    pub col: u8,
    pub row: u8,
    pub col_span: u8,
    pub row_span: u8,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_document() -> Document {
        let mut d = Document::new("Sortie à la ferme");
        let page = &mut d.pages[0];
        page.elements.push(Element {
            id: new_id(),
            frame: Frame { x_mm: 10.0, y_mm: 20.0, w_mm: 80.0, h_mm: 60.0, rotation_deg: 0.0 },
            locked: false,
            zone: true,
            kind: ElementKind::Photo(PhotoProps {
                asset: Some("a1".into()),
                crop: Crop::COVER,
                corner_radius_mm: 3.0,
                border: Some(Stroke { width_mm: 0.5, color: "#336699".into() }),
                shadow: true,
            }),
        });
        page.elements.push(Element {
            id: new_id(),
            frame: Frame { x_mm: 10.0, y_mm: 90.0, w_mm: 190.0, h_mm: 30.0, rotation_deg: 5.0 },
            locked: false,
            zone: true,
            kind: ElementKind::Text(TextProps {
                content: RichText {
                    paragraphs: vec![Paragraph {
                        align: Some(Align::Center),
                        runs: vec![Run {
                            text: "Bonjour".into(),
                            bold: true,
                            italic: false,
                            underline: false,
                            color: Some("#cc0000".into()),
                            font: None,
                            size_pt: Some(20.0),
                        }],
                    }],
                },
                style: TextStyle::default_zone(),
                prompt: Some("Titre".into()),
            }),
        });
        page.elements.push(Element {
            id: new_id(),
            frame: Frame { x_mm: 0.0, y_mm: 0.0, w_mm: 210.0, h_mm: 15.0, rotation_deg: 0.0 },
            locked: true,
            zone: false,
            kind: ElementKind::Shape(ShapeProps {
                shape: ShapeKind::Rect,
                fill: Some("#ffeecc".into()),
                stroke: None,
                corner_radius_mm: 0.0,
            }),
        });
        d.assets.insert(
            "a1".into(),
            AssetMeta { id: "a1".into(), width_px: 4000, height_px: 3000, mime: Mime::Jpeg },
        );
        d
    }

    #[test]
    fn new_document_has_one_a4_page() {
        let d = Document::new("Sortie à la ferme");
        assert_eq!(d.format_version, 1);
        assert_eq!(d.pages.len(), 1);
        assert_eq!(d.pages[0].size, A4_PORTRAIT);
    }

    #[test]
    fn json_roundtrip_keeps_everything() {
        let d = sample_document();
        let json = serde_json::to_string(&d).unwrap();
        let back: Document = serde_json::from_str(&json).unwrap();
        assert_eq!(back, d);
    }

    #[test]
    fn element_kind_is_tagged() {
        let d = sample_document();
        let v = serde_json::to_value(&d.pages[0].elements[1]).unwrap();
        assert_eq!(v["kind"]["type"], "Text");
    }

    #[test]
    fn blank_detection() {
        assert!(RichText { paragraphs: vec![] }.is_blank());
        assert!(RichText::plain("  \n ").is_blank());
        assert!(!RichText::plain("Bonjour").is_blank());
    }

    #[test]
    fn document_without_pending_photos_still_loads() {
        let json = r#"{"format_version":1,"title":"Ancien","pages":[],"assets":{}}"#;
        let d: Document = serde_json::from_str(json).unwrap();
        assert!(d.pending_photos.is_empty());
    }
}
