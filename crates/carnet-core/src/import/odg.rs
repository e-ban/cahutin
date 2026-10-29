//! Pages lues dans l'ODG que produit l'import PDF de LibreOffice (`draw_pdf_import`) : textes,
//! images et formes simples deviennent des éléments Carnet ; le reste est compté comme non repris.

use super::paragraphs::{Line, block_element, merge_lines};
use super::{ImportError, ImportReport, Imported};
use crate::model::*;
use crate::photo::{Asset, import_bytes};
use roxmltree::Node;
use std::collections::HashMap;
use std::io::{Read, Seek};
use zip::ZipArchive;

const DRAW: &str = "urn:oasis:names:tc:opendocument:xmlns:drawing:1.0";
const SVG: &str = "urn:oasis:names:tc:opendocument:xmlns:svg-compatible:1.0";
const TEXT: &str = "urn:oasis:names:tc:opendocument:xmlns:text:1.0";
const STYLE: &str = "urn:oasis:names:tc:opendocument:xmlns:style:1.0";
const FO: &str = "urn:oasis:names:tc:opendocument:xmlns:xsl-fo-compatible:1.0";
const XLINK: &str = "http://www.w3.org/1999/xlink";

/// Police mise à la place d'une police absente du poste.
const FALLBACK_FONT: &str = "Andika";
/// Trait « fin » d'une forme dont l'épaisseur vaut 0.
const HAIRLINE_MM: f64 = 0.2;

/// Longueur ODF (`1.5cm`, `12mm`, `1in`, `12pt`) en millimètres.
fn length_mm(s: &str) -> Option<f64> {
    let s = s.trim();
    let split = s.find(|c: char| c.is_ascii_alphabetic()).unwrap_or(s.len());
    let v: f64 = s[..split].parse().ok()?;
    Some(match &s[split..] {
        "cm" => v * 10.0,
        "mm" | "" => v,
        "in" => v * 25.4,
        "pt" => v * 25.4 / 72.0,
        _ => return None,
    })
}

fn attr_mm(n: Node, ns: &str, name: &str) -> Option<f64> {
    n.attribute((ns, name)).and_then(length_mm)
}

fn read_entry<R: Read + Seek>(zip: &mut ZipArchive<R>, name: &str) -> Option<Vec<u8>> {
    let mut entry = zip.by_name(name).ok()?;
    let mut buf = Vec::new();
    entry.read_to_end(&mut buf).ok()?;
    Some(buf)
}

#[derive(Clone, Default)]
struct SpanStyle {
    font: Option<String>,
    size_pt: Option<f64>,
    color: Option<String>,
    bold: bool,
    italic: bool,
    underline: bool,
}

#[derive(Clone, Default)]
struct GraphicStyle {
    fill: Option<String>,
    stroke: Option<Stroke>,
}

struct Styles {
    text: HashMap<String, SpanStyle>,
    graphic: HashMap<String, GraphicStyle>,
}

fn read_styles(doc: &roxmltree::Document, known_fonts: &[String]) -> Styles {
    let mut faces: HashMap<String, String> = HashMap::new();
    for f in doc.descendants().filter(|n| n.has_tag_name((STYLE, "font-face"))) {
        if let (Some(name), Some(family)) = (f.attribute((STYLE, "name")), f.attribute((SVG, "font-family"))) {
            faces.insert(name.to_string(), family.trim_matches(|c| c == '\'' || c == '"').to_string());
        }
    }
    let resolve = |name: &str| -> String {
        let family = faces.get(name).map(String::as_str).unwrap_or(name);
        known_fonts.iter().find(|k| k.eq_ignore_ascii_case(family)).cloned().unwrap_or_else(|| FALLBACK_FONT.to_string())
    };
    let mut styles = Styles { text: HashMap::new(), graphic: HashMap::new() };
    for s in doc.descendants().filter(|n| n.has_tag_name((STYLE, "style"))) {
        let Some(name) = s.attribute((STYLE, "name")) else { continue };
        if let Some(t) = s.children().find(|n| n.has_tag_name((STYLE, "text-properties"))) {
            styles.text.insert(
                name.to_string(),
                SpanStyle {
                    font: t.attribute((STYLE, "font-name")).map(resolve),
                    size_pt: t.attribute((FO, "font-size")).and_then(|v| v.strip_suffix("pt")).and_then(|v| v.parse().ok()),
                    color: t.attribute((FO, "color")).map(String::from),
                    bold: t.attribute((FO, "font-weight")) == Some("bold"),
                    italic: t.attribute((FO, "font-style")) == Some("italic"),
                    underline: t.attribute((STYLE, "text-underline-style")).is_some_and(|u| u != "none"),
                },
            );
        }
        if let Some(g) = s.children().find(|n| n.has_tag_name((STYLE, "graphic-properties"))) {
            let fill = (g.attribute((DRAW, "fill")) == Some("solid")).then(|| g.attribute((DRAW, "fill-color")).unwrap_or("#ffffff").to_string());
            let stroke = matches!(g.attribute((DRAW, "stroke")), Some("solid") | Some("dash")).then(|| Stroke {
                width_mm: attr_mm(g, SVG, "stroke-width").filter(|w| *w > 0.0).unwrap_or(HAIRLINE_MM),
                color: g.attribute((SVG, "stroke-color")).unwrap_or("#000000").to_string(),
                style: None,
            });
            styles.graphic.insert(name.to_string(), GraphicStyle { fill, stroke });
        }
    }
    styles
}

/// Taille de la première mise en page de `styles.xml`, au dixième de mm ; A4 portrait à défaut.
fn page_size(styles_xml: Option<&str>) -> PageSize {
    let found = styles_xml.and_then(|x| roxmltree::Document::parse(x).ok()).and_then(|doc| {
        let p = doc.descendants().find(|n| n.has_tag_name((STYLE, "page-layout-properties")) && n.attribute((FO, "page-width")).is_some())?;
        Some((attr_mm(p, FO, "page-width")?, attr_mm(p, FO, "page-height")?))
    });
    match found {
        Some((w, h)) => PageSize { width_mm: (w * 10.0).round() / 10.0, height_mm: (h * 10.0).round() / 10.0 },
        None => A4_PORTRAIT,
    }
}

/// Élément posé, ou place réservée à une ligne de texte (remplacée par son texte recollé).
enum Item {
    Element(Box<Element>),
    Line(usize),
}

struct PageReader<'a, R: Read + Seek> {
    styles: &'a Styles,
    zip: &'a mut ZipArchive<R>,
    items: Vec<Item>,
    lines: Vec<Line>,
    assets: Vec<Asset>,
    skipped: usize,
}

fn frame_of(n: Node) -> Option<Frame> {
    Some(Frame { x_mm: attr_mm(n, SVG, "x")?, y_mm: attr_mm(n, SVG, "y")?, w_mm: attr_mm(n, SVG, "width")?, h_mm: attr_mm(n, SVG, "height")?, rotation_deg: 0.0 })
}

fn element(frame: Frame, kind: ElementKind) -> Element {
    Element { id: new_id(), frame, locked: false, zone: false, group: None, kind }
}

/// Texte d'un nœud, espaces ODF (`text:s`, `text:tab`) compris.
fn node_text(n: Node, out: &mut String) {
    for c in n.children() {
        if c.is_text() {
            out.push_str(c.text().unwrap_or(""));
        } else if c.has_tag_name((TEXT, "s")) {
            let count = c.attribute((TEXT, "c")).and_then(|v| v.parse().ok()).unwrap_or(1);
            out.push_str(&" ".repeat(count));
        } else if c.has_tag_name((TEXT, "tab")) || c.has_tag_name((TEXT, "line-break")) {
            out.push(' ');
        } else {
            node_text(c, out);
        }
    }
}

/// Polygone à quatre angles droits, côtés horizontaux et verticaux.
fn is_axis_rectangle(points: &str) -> bool {
    let pts: Vec<(f64, f64)> = points
        .split_whitespace()
        .filter_map(|p| {
            let (x, y) = p.split_once(',')?;
            Some((x.parse().ok()?, y.parse().ok()?))
        })
        .collect();
    pts.len() == 4 && (0..4).all(|i| {
        let (a, b) = (pts[i], pts[(i + 1) % 4]);
        (a.0 == b.0) != (a.1 == b.1)
    })
}

impl<R: Read + Seek> PageReader<'_, R> {
    fn shape(&mut self, n: Node, frame: Frame, shape: ShapeKind) {
        let g = n.attribute((DRAW, "style-name")).and_then(|s| self.styles.graphic.get(s)).cloned().unwrap_or_default();
        let (fill, stroke) = if shape == ShapeKind::Line {
            (None, Some(g.stroke.unwrap_or(Stroke { width_mm: HAIRLINE_MM, color: "#000000".into(), style: None })))
        } else {
            (g.fill, g.stroke)
        };
        self.items.push(Item::Element(Box::new(element(frame, ElementKind::Shape(ShapeProps { shape, fill, stroke, corner_radius_mm: 0.0 })))));
    }

    fn text_box(&mut self, frame: Frame, frame_node: Node, text_box: Node) {
        let paragraphs: Vec<Node> = text_box.descendants().filter(|n| n.has_tag_name((TEXT, "p"))).collect();
        let n = paragraphs.len().max(1) as f64;
        for (i, p) in paragraphs.iter().enumerate() {
            let base = p
                .attribute((TEXT, "style-name"))
                .or(frame_node.attribute((DRAW, "text-style-name")))
                .and_then(|s| self.styles.text.get(s))
                .cloned()
                .unwrap_or_default();
            let mut runs: Vec<Run> = Vec::new();
            let mut push = |text: String, st: &SpanStyle| {
                if text.is_empty() {
                    return;
                }
                runs.push(Run {
                    text,
                    bold: st.bold,
                    italic: st.italic,
                    underline: st.underline,
                    color: Some(st.color.clone().unwrap_or_else(|| "#000000".into())),
                    font: Some(st.font.clone().unwrap_or_else(|| FALLBACK_FONT.into())),
                    size_pt: Some(st.size_pt.unwrap_or(12.0)),
                });
            };
            for c in p.children() {
                if c.has_tag_name((TEXT, "span")) {
                    let st = c.attribute((TEXT, "style-name")).and_then(|s| self.styles.text.get(s)).cloned().unwrap_or_else(|| base.clone());
                    let mut t = String::new();
                    node_text(c, &mut t);
                    push(t, &st);
                } else {
                    let mut t = String::new();
                    if c.is_text() {
                        t.push_str(c.text().unwrap_or(""));
                    } else {
                        node_text(c, &mut t);
                    }
                    push(t, &base);
                }
            }
            if runs.iter().all(|r| r.text.trim().is_empty()) {
                continue;
            }
            let h = frame.h_mm / n;
            let line = Line {
                x_mm: frame.x_mm,
                y_mm: frame.y_mm + i as f64 * h,
                w_mm: frame.w_mm,
                h_mm: h,
                font: runs[0].font.clone().unwrap(),
                size_pt: runs[0].size_pt.unwrap(),
                runs,
            };
            self.items.push(Item::Line(self.lines.len()));
            self.lines.push(line);
        }
    }

    fn image(&mut self, frame: Frame, image: Node) {
        let asset = image.attribute((XLINK, "href")).and_then(|href| read_entry(self.zip, href)).and_then(|bytes| import_bytes(&bytes).ok());
        match asset {
            Some(a) => {
                let photo = PhotoProps { asset: Some(a.meta.id.clone()), crop: Crop::COVER, corner_radius_mm: 0.0, border: None, shadow: false };
                self.items.push(Item::Element(Box::new(element(frame, ElementKind::Photo(photo)))));
                self.assets.push(a);
            }
            None => self.skipped += 1,
        }
    }

    fn node(&mut self, n: Node) {
        if !n.is_element() {
            return;
        }
        if n.has_tag_name((DRAW, "g")) {
            for c in n.children() {
                self.node(c);
            }
            return;
        }
        if n.attribute((DRAW, "transform")).is_some() {
            self.skipped += 1;
            return;
        }
        let tag = n.tag_name();
        if tag.namespace() != Some(DRAW) {
            return;
        }
        match tag.name() {
            "frame" => {
                let Some(frame) = frame_of(n) else { return self.skipped += 1 };
                if let Some(tb) = n.children().find(|c| c.has_tag_name((DRAW, "text-box"))) {
                    self.text_box(frame, n, tb);
                } else if let Some(img) = n.children().find(|c| c.has_tag_name((DRAW, "image"))) {
                    self.image(frame, img);
                } else {
                    self.skipped += 1;
                }
            }
            "polygon" if n.attribute((DRAW, "points")).is_some_and(is_axis_rectangle) => match frame_of(n) {
                Some(f) => self.shape(n, f, ShapeKind::Rect),
                None => self.skipped += 1,
            },
            "rect" | "ellipse" | "circle" => match frame_of(n) {
                Some(f) => self.shape(n, f, if tag.name() == "rect" { ShapeKind::Rect } else { ShapeKind::Ellipse }),
                None => self.skipped += 1,
            },
            "line" => {
                let (Some(x1), Some(y1), Some(x2), Some(y2)) = (attr_mm(n, SVG, "x1"), attr_mm(n, SVG, "y1"), attr_mm(n, SVG, "x2"), attr_mm(n, SVG, "y2")) else {
                    return self.skipped += 1;
                };
                // Trait Carnet : du milieu du bord gauche au milieu du bord droit du cadre, tourné.
                let len = (x2 - x1).hypot(y2 - y1);
                let (cx, cy, h) = ((x1 + x2) / 2.0, (y1 + y2) / 2.0, 2.0);
                let rotation_deg = (y2 - y1).atan2(x2 - x1).to_degrees();
                self.shape(n, Frame { x_mm: cx - len / 2.0, y_mm: cy - h / 2.0, w_mm: len, h_mm: h, rotation_deg }, ShapeKind::Line);
            }
            _ => self.skipped += 1,
        }
    }

    fn finish(self) -> (Vec<Element>, Vec<Asset>, usize) {
        let blocks = merge_lines(self.lines);
        let mut by_first: HashMap<usize, Element> = blocks.iter().map(|b| (b.first, block_element(b))).collect();
        let elements = self
            .items
            .into_iter()
            .filter_map(|it| match it {
                Item::Element(e) => Some(*e),
                Item::Line(i) => by_first.remove(&i),
            })
            .collect();
        (elements, self.assets, self.skipped)
    }
}

/// Pages Carnet tirées de l'ODG produit par `draw_pdf_import`.
pub fn read_odg(input: impl Read + Seek, known_fonts: &[String]) -> Result<Imported, ImportError> {
    let mut zip = ZipArchive::new(input).map_err(|_| ImportError::Unreadable)?;
    let content = read_entry(&mut zip, "content.xml").and_then(|b| String::from_utf8(b).ok()).ok_or(ImportError::Unreadable)?;
    let styles_xml = read_entry(&mut zip, "styles.xml").and_then(|b| String::from_utf8(b).ok());
    let doc = roxmltree::Document::parse(&content).map_err(|_| ImportError::Unreadable)?;
    let styles = read_styles(&doc, known_fonts);
    let size = page_size(styles_xml.as_deref());
    let mut pages = Vec::new();
    let mut assets = Vec::new();
    let mut skipped = 0;
    for page_node in doc.descendants().filter(|n| n.has_tag_name((DRAW, "page"))) {
        let mut reader = PageReader { styles: &styles, zip: &mut zip, items: Vec::new(), lines: Vec::new(), assets: Vec::new(), skipped: 0 };
        for c in page_node.children() {
            reader.node(c);
        }
        let (elements, page_assets, page_skipped) = reader.finish();
        pages.push(Page { elements, ..Page::new(size) });
        assets.extend(page_assets);
        skipped += page_skipped;
    }
    let report = ImportReport { skipped, ..ImportReport::count(&pages) };
    Ok(Imported { pages, assets, fonts: Vec::new(), report })
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageFormat, Rgb, RgbImage};
    use std::io::{Cursor, Write};

    const A4_LAYOUT: &str = r#"fo:page-width="21.001cm" fo:page-height="29.7cm""#;

    /// ODG minimal : styles relevés dans l'essai du 2026-10-29, corps de page fourni par le test.
    fn odg_fixture(content_body: &str, page_layout: &str, pictures: &[(&str, Vec<u8>)]) -> Vec<u8> {
        let content = format!(
            r##"<?xml version="1.0" encoding="UTF-8"?>
<office:document-content xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0" xmlns:style="urn:oasis:names:tc:opendocument:xmlns:style:1.0" xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0" xmlns:draw="urn:oasis:names:tc:opendocument:xmlns:drawing:1.0" xmlns:fo="urn:oasis:names:tc:opendocument:xmlns:xsl-fo-compatible:1.0" xmlns:svg="urn:oasis:names:tc:opendocument:xmlns:svg-compatible:1.0" xmlns:xlink="http://www.w3.org/1999/xlink" xmlns:loext="urn:org:documentfoundation:names:experimental:office:xmlns:loext:1.0" office:version="1.3">
<office:font-face-decls>
<style:font-face style:name="Liberation Serif" svg:font-family="&apos;Liberation Serif&apos;"/>
<style:font-face style:name="Liberation Sans" svg:font-family="&apos;Liberation Sans&apos;" style:font-family-generic="roman"/>
<style:font-face style:name="Comic Inconnue" svg:font-family="&apos;Comic Inconnue&apos;"/>
</office:font-face-decls>
<office:automatic-styles>
<style:style style:name="gr1" style:family="graphic"><style:graphic-properties draw:stroke="solid" svg:stroke-width="0cm" svg:stroke-color="#3465a4" draw:fill="solid" draw:fill-color="#729fcf"/></style:style>
<style:style style:name="gr2" style:family="graphic"><style:graphic-properties draw:stroke="none" draw:fill="none"/></style:style>
<style:style style:name="T1" style:family="text"><style:text-properties fo:color="#000000" style:font-name="Liberation Serif" fo:font-size="12pt" fo:font-weight="normal"/></style:style>
<style:style style:name="T2" style:family="text"><style:text-properties fo:color="#c0392b" style:font-name="Liberation Sans" fo:font-size="24pt" fo:font-weight="bold"/></style:style>
<style:style style:name="T3" style:family="text"><style:text-properties fo:color="#000000" style:font-name="Liberation Serif" fo:font-size="12pt" fo:font-weight="bold"/></style:style>
<style:style style:name="T4" style:family="text"><style:text-properties fo:color="#000000" style:font-name="Comic Inconnue" fo:font-size="12pt" fo:font-style="italic" style:text-underline-style="solid"/></style:style>
</office:automatic-styles>
<office:body><office:drawing><draw:page draw:name="page1">{content_body}</draw:page></office:drawing></office:body></office:document-content>"##
        );
        let styles = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document-styles xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0" xmlns:style="urn:oasis:names:tc:opendocument:xmlns:style:1.0" xmlns:fo="urn:oasis:names:tc:opendocument:xmlns:xsl-fo-compatible:1.0">
<office:automatic-styles><style:page-layout style:name="PM1"><style:page-layout-properties fo:margin-top="2cm" {page_layout}/></style:page-layout></office:automatic-styles>
</office:document-styles>"#
        );
        let mut buf = Cursor::new(Vec::new());
        {
            let mut zip = zip::ZipWriter::new(&mut buf);
            let opts = zip::write::SimpleFileOptions::default();
            for (name, data) in [("content.xml", content.into_bytes()), ("styles.xml", styles.into_bytes())] {
                zip.start_file(name, opts).unwrap();
                zip.write_all(&data).unwrap();
            }
            for (name, data) in pictures {
                zip.start_file(format!("Pictures/{name}"), opts).unwrap();
                zip.write_all(data).unwrap();
            }
            zip.finish().unwrap();
        }
        buf.into_inner()
    }

    fn known() -> Vec<String> {
        ["Andika", "Liberation Serif", "Liberation Sans"].map(String::from).to_vec()
    }

    fn read(body: &str) -> Imported {
        read_odg(Cursor::new(odg_fixture(body, A4_LAYOUT, &[])), &known()).unwrap()
    }

    fn text_frame(x: &str, y: &str, w: &str, spans: &str) -> String {
        format!(r#"<draw:frame draw:style-name="gr2" svg:width="{w}" svg:height="0.471cm" svg:x="{x}" svg:y="{y}"><draw:text-box><text:p>{spans}</text:p></draw:text-box></draw:frame>"#)
    }

    fn texts(p: &Page) -> Vec<&TextProps> {
        p.elements.iter().filter_map(|e| if let ElementKind::Text(t) = &e.kind { Some(t) } else { None }).collect()
    }

    fn plain(t: &TextProps) -> String {
        t.content.paragraphs.iter().flat_map(|p| p.runs.iter().map(|r| r.text.as_str())).collect()
    }

    fn png() -> Vec<u8> {
        let mut out = Cursor::new(Vec::new());
        RgbImage::from_pixel(40, 30, Rgb([240, 200, 120])).write_to(&mut out, ImageFormat::Png).unwrap();
        out.into_inner()
    }

    #[test]
    fn text_frames_become_texts_with_their_styles() {
        let body = text_frame("13.316cm", "2.036cm", "3.882cm", r#"<text:span text:style-name="T2">La pomme</text:span>"#)
            + &text_frame("2.004cm", "2.992cm", "8.661cm", r#"<text:span text:style-name="T3">Consigne : </text:span><text:span text:style-name="T1">observe l&apos;image.</text:span>"#);
        let imported = read(&body);
        let t = texts(&imported.pages[0]);
        assert_eq!(t.len(), 2);
        assert_eq!(plain(t[0]), "La pomme");
        assert_eq!((t[0].style.font.as_str(), t[0].style.size_pt, t[0].style.color.as_str()), ("Liberation Sans", 24.0, "#c0392b"));
        assert!(t[0].content.paragraphs[0].runs[0].bold);
        let runs = &t[1].content.paragraphs[0].runs;
        assert_eq!(runs[0].text, "Consigne : ");
        assert!(runs[0].bold && !runs[1].bold);
        assert_eq!(runs[1].text, "observe l'image.");
    }

    #[test]
    fn lines_are_merged() {
        let body = text_frame("2.004cm", "3.966cm", "4.374cm", r#"<text:span text:style-name="T1">Complète le tableau avec </text:span>"#)
            + &text_frame("2.004cm", "4.452cm", "4.082cm", r#"<text:span text:style-name="T1">les mots : rouge, ronde, </text:span>"#)
            + &text_frame("2.004cm", "4.939cm", "0.837cm", r#"<text:span text:style-name="T1">fruit.</text:span>"#);
        let t = texts(&read(&body).pages[0]).into_iter().map(plain).collect::<Vec<_>>();
        assert_eq!(t, ["Complète le tableau avec les mots : rouge, ronde, fruit."]);
    }

    #[test]
    fn image_becomes_a_photo_in_the_library() {
        let body = r#"<draw:frame draw:style-name="gr2" svg:width="7.998cm" svg:height="6cm" svg:x="6.502cm" svg:y="3.459cm"><draw:image xlink:href="Pictures/img.png" draw:mime-type="image/png"><text:p/></draw:image></draw:frame>"#;
        let imported = read_odg(Cursor::new(odg_fixture(body, A4_LAYOUT, &[("img.png", png())])), &known()).unwrap();
        assert_eq!(imported.assets.len(), 1);
        let e = &imported.pages[0].elements[0];
        let ElementKind::Photo(p) = &e.kind else { panic!("pas une photo : {:?}", e.kind) };
        assert_eq!(p.asset.as_deref(), Some(imported.assets[0].meta.id.as_str()));
        let f = e.frame;
        assert!((f.x_mm - 65.02).abs() < 0.01 && (f.y_mm - 34.59).abs() < 0.01 && (f.w_mm - 79.98).abs() < 0.01 && (f.h_mm - 60.0).abs() < 0.01, "{f:?}");
        assert!(!e.zone && !e.locked);
    }

    #[test]
    fn rectangle_polygon_becomes_a_filled_rect() {
        let body = r#"<draw:polygon draw:style-name="gr1" svg:width="11.999cm" svg:height="2.5cm" svg:x="2cm" svg:y="6.381cm" svg:viewBox="0 0 12000 2501" draw:points="0,0 12000,0 12000,2501 0,2501"><text:p/></draw:polygon>"#;
        let e = &read(body).pages[0].elements[0];
        let ElementKind::Shape(s) = &e.kind else { panic!("pas une forme") };
        assert_eq!(s.shape, ShapeKind::Rect);
        assert_eq!(s.fill.as_deref(), Some("#729fcf"));
        assert_eq!(s.stroke.as_ref().map(|k| (k.width_mm, k.color.as_str())), Some((0.2, "#3465a4")));
        let f = e.frame;
        assert!((f.x_mm - 20.0).abs() < 0.01 && (f.y_mm - 63.81).abs() < 0.01 && (f.w_mm - 119.99).abs() < 0.01 && (f.h_mm - 25.0).abs() < 0.01, "{f:?}");
    }

    #[test]
    fn other_drawings_are_counted_as_skipped() {
        let body = r#"<draw:polygon draw:style-name="gr1" svg:width="2cm" svg:height="2cm" svg:x="2cm" svg:y="2cm" svg:viewBox="0 0 2000 2000" draw:points="0,2000 1000,0 2000,2000"/>
<draw:path draw:style-name="gr1" svg:width="2cm" svg:height="2cm" svg:x="5cm" svg:y="2cm" svg:viewBox="0 0 2000 2000" svg:d="M 0 0 C 500 2000 1500 2000 2000 0"/>
<draw:rect draw:style-name="gr1" svg:width="2cm" svg:height="1cm" draw:transform="rotate (0.5) translate (8cm 2cm)"/>
<draw:g><draw:ellipse draw:style-name="gr1" svg:width="2cm" svg:height="1cm" svg:x="2cm" svg:y="8cm"/><draw:line draw:style-name="gr1" svg:x1="2cm" svg:y1="12cm" svg:x2="6cm" svg:y2="12cm"/></draw:g>"#;
        let imported = read(body);
        assert_eq!(imported.report.skipped, 3);
        let kinds: Vec<ShapeKind> = imported.pages[0].elements.iter().filter_map(|e| if let ElementKind::Shape(s) = &e.kind { Some(s.shape) } else { None }).collect();
        assert_eq!(kinds, [ShapeKind::Ellipse, ShapeKind::Line]);
        let line = &imported.pages[0].elements[1].frame;
        assert!((line.x_mm - 20.0).abs() < 0.01 && (line.w_mm - 40.0).abs() < 0.01 && (line.y_mm + line.h_mm / 2.0 - 120.0).abs() < 0.01, "{line:?}");
    }

    #[test]
    fn unknown_font_falls_back_to_andika() {
        let t = read(&text_frame("2cm", "2cm", "5cm", r#"<text:span text:style-name="T4">Bonjour</text:span>"#));
        let t = texts(&t.pages[0])[0].clone();
        assert_eq!(t.style.font, "Andika");
        let r = &t.content.paragraphs[0].runs[0];
        assert!(r.italic && r.underline);
    }

    #[test]
    fn unreadable_or_oversized_image_is_skipped() {
        let body = r#"<draw:frame svg:width="2cm" svg:height="2cm" svg:x="2cm" svg:y="2cm"><draw:image xlink:href="Pictures/bad.png"/></draw:frame>"#;
        let imported = read_odg(Cursor::new(odg_fixture(body, A4_LAYOUT, &[("bad.png", b"pas une image".to_vec())])), &known()).unwrap();
        assert!(imported.pages[0].elements.is_empty());
        assert_eq!(imported.report.skipped, 1);
        assert!(imported.assets.is_empty());
    }

    #[test]
    fn page_size_comes_from_the_page_layout() {
        let imported = read_odg(Cursor::new(odg_fixture("", r#"fo:page-width="21.59cm" fo:page-height="27.94cm""#, &[])), &known()).unwrap();
        assert_eq!((imported.pages[0].size.width_mm, imported.pages[0].size.height_mm), (215.9, 279.4));
        let a4 = read("");
        assert_eq!((a4.pages[0].size.width_mm, a4.pages[0].size.height_mm), (210.0, 297.0));
    }

    #[test]
    fn report_counts_everything() {
        let body = text_frame("2cm", "2cm", "5cm", r#"<text:span text:style-name="T1">Un</text:span>"#)
            + r#"<draw:polygon draw:style-name="gr1" svg:width="2cm" svg:height="2cm" svg:x="2cm" svg:y="9cm" svg:viewBox="0 0 2000 2000" draw:points="0,0 2000,0 2000,2000 0,2000"/>"#
            + r#"<draw:path draw:style-name="gr1" svg:width="2cm" svg:height="2cm" svg:x="5cm" svg:y="2cm" svg:viewBox="0 0 2000 2000" svg:d="M 0 0 L 2000 2000"/>"#
            + r#"<draw:frame svg:width="2cm" svg:height="2cm" svg:x="9cm" svg:y="9cm"><draw:image xlink:href="Pictures/img.png"/></draw:frame>"#;
        let imported = read_odg(Cursor::new(odg_fixture(&body, A4_LAYOUT, &[("img.png", png())])), &known()).unwrap();
        assert_eq!(imported.report, ImportReport { pages: 1, texts: 1, images: 1, shapes: 1, skipped: 1 });
    }
}
