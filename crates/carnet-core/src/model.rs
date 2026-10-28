//! Modèle de document. Toutes les géométries sont en millimètres.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use ts_rs::TS;

pub type Id = String;

pub fn new_id() -> Id {
    uuid::Uuid::new_v4().to_string()
}

/// Version la plus récente que cette application sait lire (2 : élément image ; 3 : cadre de texte,
/// alignement vertical, marge et bordures en tirets ou pointillés).
pub const FORMAT_VERSION: u32 = 3;
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
            format_version: 1,
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
    /// Fond de la page (couleur et motif d'école) ; absent : blanc.
    #[serde(default)]
    #[ts(optional = nullable)]
    pub background: Option<Background>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, TS)]
#[ts(export)]
pub struct Background {
    pub color: String,
    pub pattern: Option<Pattern>,
}

/// Motifs d'école dessinés par-dessus la couleur du fond.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, TS)]
#[ts(export)]
pub enum Pattern {
    Seyes,
    SmallSquares,
    Lines,
    Dots,
}

impl Page {
    pub fn new(size: PageSize) -> Page {
        Page { id: new_id(), size, elements: vec![], grid: None, background: None }
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
    /// Groupe : les éléments d'une page qui partagent cet identifiant se manient ensemble.
    #[serde(default)]
    #[ts(optional = nullable)]
    pub group: Option<Id>,
    pub kind: ElementKind,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, TS)]
#[serde(tag = "type")]
#[ts(export)]
pub enum ElementKind {
    Photo(PhotoProps),
    Text(TextProps),
    Shape(ShapeProps),
    Image(ImageProps),
}

/// Image de la banque OpenMoji embarquée, désignée par son code (par exemple `1F98A`).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, TS)]
#[ts(export)]
pub struct ImageProps {
    pub code: String,
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
    /// Absent : trait plein (documents antérieurs).
    #[serde(default)]
    #[ts(optional = nullable)]
    pub style: Option<StrokeStyle>,
}

/// Tirets de 3 épaisseurs séparés de 2 ; pointillés d'une épaisseur séparés d'une (écran et ODG).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default, TS)]
#[ts(export)]
pub enum StrokeStyle {
    #[default]
    Solid,
    Dashed,
    Dotted,
}

impl Stroke {
    pub fn is_solid(&self) -> bool {
        self.style.unwrap_or_default() == StrokeStyle::Solid
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, TS)]
#[ts(export)]
pub struct TextProps {
    pub content: RichText,
    pub style: TextStyle,
    pub prompt: Option<String>,
    /// Fond, bordure, coins et image de fond du cadre ; absent : texte sans cadre.
    #[serde(default)]
    #[ts(optional = nullable)]
    pub frame: Option<TextFrame>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default, TS)]
#[ts(export)]
pub struct TextFrame {
    pub fill: Option<String>,
    pub stroke: Option<Stroke>,
    pub corner_radius_mm: f64,
    /// Image de la photothèque qui couvre le cadre, sous le fond.
    #[serde(default)]
    #[ts(optional = nullable)]
    pub image: Option<Id>,
}

impl TextFrame {
    pub fn is_empty(&self) -> bool {
        self.fill.is_none() && self.stroke.is_none() && self.image.is_none()
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, TS)]
#[ts(export)]
pub struct TextStyle {
    pub font: String,
    pub size_pt: f64,
    pub color: String,
    pub align: Align,
    /// Absent : en haut.
    #[serde(default)]
    #[ts(optional = nullable)]
    pub valign: Option<VAlign>,
    /// Marge intérieure ; absente : `TEXT_PADDING_MM`.
    #[serde(default)]
    #[ts(optional = nullable)]
    pub padding_mm: Option<f64>,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default, TS)]
#[ts(export)]
pub enum VAlign {
    #[default]
    Top,
    Middle,
    Bottom,
}

/// Marge intérieure des textes sans réglage, identique à l'écran et dans l'ODG.
pub const TEXT_PADDING_MM: f64 = 1.5;

impl TextStyle {
    pub fn padding(&self) -> f64 {
        self.padding_mm.unwrap_or(TEXT_PADDING_MM)
    }

    pub fn default_zone() -> TextStyle {
        TextStyle { font: "Andika".into(), size_pt: 14.0, color: "#222222".into(), align: Align::Left, valign: None, padding_mm: None }
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
    Line,
    Arrow,
    Star,
    Heart,
    Bubble,
    Cloud,
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
            group: None,
            kind: ElementKind::Photo(PhotoProps {
                asset: Some("a1".into()),
                crop: Crop::COVER,
                corner_radius_mm: 3.0,
                border: Some(Stroke { width_mm: 0.5, color: "#336699".into(), style: None }),
                shadow: true,
            }),
        });
        page.elements.push(Element {
            id: new_id(),
            frame: Frame { x_mm: 10.0, y_mm: 90.0, w_mm: 190.0, h_mm: 30.0, rotation_deg: 5.0 },
            locked: false,
            zone: true,
            group: None,
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
                frame: None,
            }),
        });
        page.elements.push(Element {
            id: new_id(),
            frame: Frame { x_mm: 0.0, y_mm: 0.0, w_mm: 210.0, h_mm: 15.0, rotation_deg: 0.0 },
            locked: true,
            zone: false,
            group: None,
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

    #[test]
    fn old_documents_load_without_group() {
        let json = r#"{"id":"e","frame":{"x_mm":0,"y_mm":0,"w_mm":1,"h_mm":1,"rotation_deg":0},"locked":false,"zone":false,"kind":{"type":"Image","code":"1F98A"}}"#;
        let e: Element = serde_json::from_str(json).unwrap();
        assert_eq!(e.group, None);
        assert_eq!(e.kind, ElementKind::Image(ImageProps { code: "1F98A".into() }));
    }
}
