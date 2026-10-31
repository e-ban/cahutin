//! Pages lues dans l'ODG que produit l'import PDF de LibreOffice (`draw_pdf_import`) : textes,
//! images et formes simples deviennent des éléments Carnet ; le reste est compté comme non repris.

use super::paragraphs::{Line, block_element, merge_lines};
use super::vector::{self, Drawing};
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
    crate::native::read_capped(&mut entry, crate::native::MAX_ENTRY_BYTES)
}

#[derive(Clone, Default)]
struct SpanStyle {
    font: Option<String>,
    /// La police d'origine manque, ni elle ni un équivalent de même largeur : remplacée par la police de repli.
    substituted: bool,
    /// La police d'origine manque, remplacée par une police de même largeur : ses lignes sont un peu plus hautes.
    equivalent: bool,
    size_pt: Option<f64>,
    color: Option<String>,
    bold: bool,
    italic: bool,
    underline: bool,
    /// Lettres creuses (contour de LibreOffice).
    outline: bool,
}

#[derive(Clone, Default)]
struct GraphicStyle {
    fill: Option<String>,
    stroke: Option<Stroke>,
    valign: Option<VAlign>,
    padding_mm: Option<f64>,
}

struct Styles {
    text: HashMap<String, SpanStyle>,
    graphic: HashMap<String, GraphicStyle>,
    /// Alignement des styles de paragraphe.
    align: HashMap<String, Align>,
}

fn read_styles(doc: &roxmltree::Document, known_fonts: &[String]) -> Styles {
    let mut faces: HashMap<String, String> = HashMap::new();
    for f in doc.descendants().filter(|n| n.has_tag_name((STYLE, "font-face"))) {
        if let (Some(name), Some(family)) = (f.attribute((STYLE, "name")), f.attribute((SVG, "font-family"))) {
            faces.insert(name.to_string(), family.trim_matches(|c| c == '\'' || c == '"').to_string());
        }
    }
    // La police elle-même, sinon une police de même largeur (les textes gardent leur longueur), sinon la police de repli.
    // Police retenue, remplacée par la police de repli, remplacée par une police de même largeur.
    let resolve = |name: &str| -> (String, bool, bool) {
        let family = faces.get(name).map(String::as_str).unwrap_or(name);
        let known = |f: &str| known_fonts.iter().find(|k| k.eq_ignore_ascii_case(f)).cloned();
        if let Some(f) = known(family) {
            return (f, false, false);
        }
        match same_width_fonts(family).iter().find_map(|f| known(f)) {
            Some(f) => (f, false, true),
            None => (FALLBACK_FONT.to_string(), !family.eq_ignore_ascii_case(FALLBACK_FONT), false),
        }
    };
    let mut styles = Styles { text: HashMap::new(), graphic: HashMap::new(), align: HashMap::new() };
    for s in doc.descendants().filter(|n| n.has_tag_name((STYLE, "style"))) {
        let Some(name) = s.attribute((STYLE, "name")) else { continue };
        if let Some(t) = s.children().find(|n| n.has_tag_name((STYLE, "text-properties"))) {
            styles.text.insert(
                name.to_string(),
                SpanStyle {
                    font: t.attribute((STYLE, "font-name")).map(|n| resolve(n).0),
                    substituted: t.attribute((STYLE, "font-name")).is_some_and(|n| resolve(n).1),
                    equivalent: t.attribute((STYLE, "font-name")).is_some_and(|n| resolve(n).2),
                    size_pt: t.attribute((FO, "font-size")).and_then(|v| v.strip_suffix("pt")).and_then(|v| v.parse().ok()),
                    color: t.attribute((FO, "color")).map(String::from),
                    bold: t.attribute((FO, "font-weight")) == Some("bold"),
                    italic: t.attribute((FO, "font-style")) == Some("italic"),
                    underline: t.attribute((STYLE, "text-underline-style")).is_some_and(|u| u != "none"),
                    outline: t.attribute((STYLE, "text-outline")) == Some("true"),
                },
            );
        }
        if let Some(a) = s.children().find(|n| n.has_tag_name((STYLE, "paragraph-properties"))).and_then(|p| p.attribute((FO, "text-align"))) {
            let align = match a {
                "center" => Align::Center,
                "end" | "right" => Align::Right,
                "justify" => Align::Justify,
                _ => Align::Left,
            };
            styles.align.insert(name.to_string(), align);
        }
        if let Some(g) = s.children().find(|n| n.has_tag_name((STYLE, "graphic-properties"))) {
            let fill = (g.attribute((DRAW, "fill")) == Some("solid")).then(|| g.attribute((DRAW, "fill-color")).unwrap_or("#ffffff").to_string());
            let stroke = matches!(g.attribute((DRAW, "stroke")), Some("solid") | Some("dash")).then(|| Stroke {
                width_mm: attr_mm(g, SVG, "stroke-width").filter(|w| *w > 0.0).unwrap_or(HAIRLINE_MM),
                color: g.attribute((SVG, "stroke-color")).unwrap_or("#000000").to_string(),
                style: (g.attribute((DRAW, "stroke")) == Some("dash")).then_some(StrokeStyle::Dashed),
            });
            let valign = g.attribute((DRAW, "textarea-vertical-align")).map(|v| match v {
                "middle" => VAlign::Middle,
                "bottom" => VAlign::Bottom,
                _ => VAlign::Top,
            });
            styles.graphic.insert(name.to_string(), GraphicStyle { fill, stroke, valign, padding_mm: attr_mm(g, FO, "padding-left") });
        }
    }
    styles
}

/// Hauteur de ligne des polices de même largeur (Liberation, Arimo, Tinos : 1,15 ; Carlito : 1,22), en
/// multiple de la taille : le cadre mesuré pour la police d'origine du PDF est souvent plus bas.
const EQUIVALENT_LINE_HEIGHT: f64 = 1.22;

/// Polices de même largeur qu'une police courante absente du poste (familles métriquement compatibles).
fn same_width_fonts(family: &str) -> &'static [&'static str] {
    let f = family.to_lowercase();
    let f = f.split(['-', ',']).next().unwrap_or("").trim();
    match f {
        "helvetica" | "helvetica neue" | "arial" | "arialmt" | "arimo" | "nimbus sans" | "liberation sans" => &["Arial", "Liberation Sans", "Arimo", "Nimbus Sans", "Nimbus Sans L"],
        "times" | "times new roman" | "timesnewroman" | "timesnewromanps" | "tinos" | "liberation serif" => &["Times New Roman", "Liberation Serif", "Tinos", "Nimbus Roman"],
        "courier" | "courier new" | "couriernew" | "cousine" | "liberation mono" => &["Courier New", "Liberation Mono", "Cousine", "Nimbus Mono PS"],
        "calibri" | "carlito" => &["Calibri", "Carlito"],
        "cambria" | "caladea" => &["Cambria", "Caladea"],
        _ => &[],
    }
}

/// Tailles des pages maîtresses de `styles.xml` (page maîtresse → mise en page), au dixième de mm,
/// et la taille par défaut : la première mise en page, ou A4 portrait.
struct PageSizes {
    by_master: HashMap<String, PageSize>,
    /// Couleur de fond d'une page maîtresse (style `drawing-page` rempli).
    background: HashMap<String, Background>,
    default: PageSize,
}

fn page_sizes(styles_xml: Option<&str>) -> PageSizes {
    let mut sizes = PageSizes { by_master: HashMap::new(), background: HashMap::new(), default: A4_PORTRAIT };
    let Some(doc) = styles_xml.and_then(|x| roxmltree::Document::parse(x).ok()) else { return sizes };
    let mut layouts: HashMap<&str, PageSize> = HashMap::new();
    for l in doc.descendants().filter(|n| n.has_tag_name((STYLE, "page-layout"))) {
        let Some(p) = l.children().find(|n| n.has_tag_name((STYLE, "page-layout-properties"))) else { continue };
        let (Some(w), Some(h)) = (attr_mm(p, FO, "page-width"), attr_mm(p, FO, "page-height")) else { continue };
        let size = PageSize { width_mm: (w * 10.0).round() / 10.0, height_mm: (h * 10.0).round() / 10.0 };
        if layouts.is_empty() {
            sizes.default = size;
        }
        if let Some(name) = l.attribute((STYLE, "name")) {
            layouts.insert(name, size);
        }
    }
    let fills: HashMap<&str, String> = doc
        .descendants()
        .filter(|n| n.has_tag_name((STYLE, "style")) && n.attribute((STYLE, "family")) == Some("drawing-page"))
        .filter_map(|n| {
            let p = n.children().find(|c| c.has_tag_name((STYLE, "drawing-page-properties")))?;
            if p.attribute((DRAW, "fill")) != Some("solid") {
                return None;
            }
            Some((n.attribute((STYLE, "name"))?, p.attribute((DRAW, "fill-color"))?.to_string()))
        })
        .collect();
    for m in doc.descendants().filter(|n| n.has_tag_name((STYLE, "master-page"))) {
        if let (Some(name), Some(size)) = (m.attribute((STYLE, "name")), m.attribute((STYLE, "page-layout-name")).and_then(|l| layouts.get(l))) {
            sizes.by_master.insert(name.to_string(), *size);
        }
        if let (Some(name), Some(color)) = (m.attribute((STYLE, "name")), m.attribute((DRAW, "style-name")).and_then(|d| fills.get(d))) {
            sizes.background.insert(name.to_string(), Background { color: color.clone(), pattern: None });
        }
    }
    sizes
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
    drawings: Vec<Drawing>,
    /// Lignes rétrécies parce que leur police manquait.
    shrunk: usize,
    /// Un dessin LibreOffice lu tel quel (dont l'export de Cahutin) : chaque cadre de texte reste un texte, avec
    /// ses alignements, sa marge et son fond. Sinon (import d'un PDF), un cadre par ligne, recollées ensuite.
    whole_frames: bool,
}

/// Cadre d'un objet : `svg:x`, `svg:y`, ou un `draw:transform` fait d'une rotation puis d'une translation
/// (ce qu'écrivent LibreOffice et l'export de Cahutin pour un objet tourné). Toute autre transformation
/// (inclinaison, échelle) : pas de cadre.
fn frame_of(n: Node) -> Option<Frame> {
    let (w, h) = (attr_mm(n, SVG, "width")?, attr_mm(n, SVG, "height")?);
    let Some(t) = n.attribute((DRAW, "transform")) else {
        return Some(Frame { x_mm: attr_mm(n, SVG, "x")?, y_mm: attr_mm(n, SVG, "y")?, w_mm: w, h_mm: h, rotation_deg: 0.0 });
    };
    let (angle, x, y) = rotate_translate(t)?;
    // L'inverse de `odg::odf_transform` : l'angle ODF est en radians, dans le sens inverse des aiguilles.
    let r = -angle;
    let (cx, cy) = (x + (w / 2.0) * r.cos() - (h / 2.0) * r.sin(), y + (w / 2.0) * r.sin() + (h / 2.0) * r.cos());
    let deg = r.to_degrees().rem_euclid(360.0);
    Some(Frame { x_mm: cx - w / 2.0, y_mm: cy - h / 2.0, w_mm: w, h_mm: h, rotation_deg: if deg.abs() < 1e-6 { 0.0 } else { deg } })
}

/// Forme d'un `draw:custom-shape` : une forme de Cahutin reconnue à son tracé (ce qu'écrit l'export), ou une
/// forme toute faite de LibreOffice d'après son type.
fn custom_shape_kind(n: Node) -> Option<ShapeKind> {
    let g = n.children().find(|c| c.has_tag_name((DRAW, "enhanced-geometry")))?;
    let path = g.attribute((DRAW, "enhanced-path")).map(|p| p.trim_end_matches(" N"));
    let ours = [ShapeKind::Star, ShapeKind::Heart, ShapeKind::Bubble, ShapeKind::Cloud].into_iter().find(|k| crate::shapes::path(*k).is_some_and(|sp| Some(sp) == path));
    if ours.is_some() {
        return ours;
    }
    match g.attribute((DRAW, "type"))? {
        "rectangle" | "round-rectangle" => Some(ShapeKind::Rect),
        "ellipse" | "circle" => Some(ShapeKind::Ellipse),
        "star5" => Some(ShapeKind::Star),
        "heart" => Some(ShapeKind::Heart),
        "cloud" => Some(ShapeKind::Cloud),
        "round-rectangular-callout" | "rectangular-callout" => Some(ShapeKind::Bubble),
        _ => None,
    }
}

/// « rotate (a) translate (x y) », la rotation facultative : angle en radians, position en mm.
fn rotate_translate(t: &str) -> Option<(f64, f64, f64)> {
    let mut angle = 0.0;
    let mut at = None;
    for part in t.split(')').map(str::trim).filter(|p| !p.is_empty()) {
        let (name, args) = part.split_once('(')?;
        let args: Vec<&str> = args.split([' ', ',']).filter(|a| !a.is_empty()).collect();
        match (name.trim(), args.as_slice()) {
            ("rotate", [a]) => angle = a.parse().ok()?,
            ("translate", [x, y]) => at = Some((length_mm(x)?, length_mm(y)?)),
            _ => return None,
        }
    }
    at.map(|(x, y)| (angle, x, y))
}

fn element(frame: Frame, kind: ElementKind) -> Element {
    Element { id: new_id(), frame, locked: false, zone: false, group: None, kind }
}

/// Caractères de la zone privée (puces des polices de symboles : OpenSymbol, Symbol, Wingdings) : « • ».
fn readable(text: &str) -> String {
    text.chars().map(|c| if ('\u{E000}'..='\u{F8FF}').contains(&c) { '•' } else { c }).collect()
}

/// Texte d'un nœud, espaces ODF (`text:s`, `text:tab`) compris.
fn node_text(n: Node, out: &mut String) {
    for c in n.children() {
        if c.is_text() {
            out.push_str(&readable(c.text().unwrap_or("")));
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
        let corner_radius_mm = if shape == ShapeKind::Rect { attr_mm(n, DRAW, "corner-radius").unwrap_or(0.0) } else { 0.0 };
        self.items.push(Item::Element(Box::new(element(frame, ElementKind::Shape(ShapeProps { shape, fill, stroke, corner_radius_mm })))));
    }

    fn text_box(&mut self, frame: Frame, frame_node: Node, text_box: Node) {
        if self.whole_frames {
            return self.whole_text_box(frame, frame_node, text_box);
        }
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
            let mut substituted: Vec<bool> = Vec::new();
            let mut equivalent = false;
            let mut push = |text: String, st: &SpanStyle| {
                if text.is_empty() {
                    return;
                }
                substituted.push(st.substituted);
                equivalent |= st.equivalent;
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
                        t.push_str(&readable(c.text().unwrap_or("")));
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
            // Police remplacée par la police de repli, plus large et plus haute. Le texte rétrécit jusqu'à tenir
            // dans la largeur qu'il avait dans l'original (la moitié de sa taille au plus), sinon il passerait à
            // la ligne ; la ligne s'allonge vers le bas à la hauteur de la police de repli.
            let mut line_h_mm = h;
            if substituted.iter().any(|s| *s) {
                let font = |r: &Run| r.font.clone().unwrap_or_else(|| FALLBACK_FONT.into());
                let width: f64 = runs.iter().filter_map(|r| crate::fonts::builtin_text_width_mm(&font(r), r.bold, r.italic, r.size_pt.unwrap_or(12.0), &r.text)).sum();
                let factor = if width > 0.0 { (frame.w_mm / width).clamp(0.5, 1.0) } else { 1.0 };
                if factor < 0.99 {
                    for r in &mut runs {
                        r.size_pt = r.size_pt.map(|s| (s * factor * 10.0).floor() / 10.0);
                    }
                    self.shrunk += 1;
                }
                let line_h = runs.iter().filter_map(|r| crate::fonts::builtin_line_height_mm(&font(r), r.size_pt.unwrap_or(12.0))).fold(0.0, f64::max);
                line_h_mm = h.max(line_h);
            } else if equivalent {
                let size = runs.iter().filter_map(|r| r.size_pt).fold(0.0, f64::max);
                line_h_mm = h.max(size * EQUIVALENT_LINE_HEIGHT * 25.4 / 72.0);
            }
            let line = Line {
                x_mm: frame.x_mm,
                y_mm: frame.y_mm + i as f64 * h,
                w_mm: frame.w_mm,
                h_mm: line_h_mm,
                font: runs[0].font.clone().unwrap(),
                size_pt: runs[0].size_pt.unwrap(),
                runs,
            };
            self.items.push(Item::Line(self.lines.len()));
            self.lines.push(line);
        }
    }

    /// Un cadre de texte entier : ses paragraphes et leur alignement, l'alignement vertical, la marge, le fond,
    /// la bordure et l'arrondi du cadre. La mention de licence OpenMoji, ajoutée par l'export, est laissée.
    fn whole_text_box(&mut self, frame: Frame, frame_node: Node, text_box: Node) {
        let g = frame_node.attribute((DRAW, "style-name")).and_then(|s| self.styles.graphic.get(s)).cloned().unwrap_or_default();
        let mut paragraphs = Vec::new();
        let mut outline = false;
        for p in text_box.descendants().filter(|n| n.has_tag_name((TEXT, "p"))) {
            let p_style = p.attribute((TEXT, "style-name"));
            let base = p_style.and_then(|s| self.styles.text.get(s)).cloned().unwrap_or_default();
            let mut runs: Vec<Run> = Vec::new();
            for c in p.children() {
                let st = if c.has_tag_name((TEXT, "span")) { c.attribute((TEXT, "style-name")).and_then(|s| self.styles.text.get(s)).cloned().unwrap_or_else(|| base.clone()) } else { base.clone() };
                let mut t = String::new();
                if c.is_text() {
                    t.push_str(&readable(c.text().unwrap_or("")));
                } else {
                    node_text(c, &mut t);
                }
                if t.is_empty() {
                    continue;
                }
                outline |= st.outline;
                runs.push(Run {
                    text: t,
                    bold: st.bold,
                    italic: st.italic,
                    underline: st.underline,
                    color: Some(st.color.clone().unwrap_or_else(|| "#000000".into())),
                    font: Some(st.font.clone().or(base.font.clone()).unwrap_or_else(|| FALLBACK_FONT.into())),
                    size_pt: Some(st.size_pt.or(base.size_pt).unwrap_or(12.0)),
                });
            }
            // Le style du paragraphe donne la police d'une ligne vide.
            let first = runs.first().cloned().unwrap_or(Run { font: base.font.clone(), size_pt: base.size_pt, color: base.color.clone(), ..Run::plain("") });
            paragraphs.push((p_style.and_then(|s| self.styles.align.get(s)).copied(), runs, first));
        }
        let plain: String = paragraphs.iter().flat_map(|(_, r, _)| r.iter().map(|r| r.text.as_str())).collect();
        if plain.trim() == crate::odg::OPENMOJI_CREDIT_PAGE || plain.trim() == crate::odg::OPENMOJI_CREDIT {
            return;
        }
        let Some((align, _, first)) = paragraphs.first().cloned() else { return };
        let style = TextStyle {
            font: first.font.clone().unwrap_or_else(|| FALLBACK_FONT.into()),
            size_pt: first.size_pt.unwrap_or(12.0),
            color: first.color.clone().unwrap_or_else(|| "#000000".into()),
            align: align.unwrap_or(Align::Left),
            valign: g.valign,
            padding_mm: g.padding_mm,
            outline: outline.then_some(true),
            named: None,
        };
        // Ce qui ne fait que répéter le style du texte est retiré des morceaux, comme dans un texte de Cahutin.
        let paragraphs = paragraphs
            .into_iter()
            .map(|(a, runs, _)| Paragraph {
                align: a.filter(|a| *a != style.align),
                runs: runs
                    .into_iter()
                    .map(|mut r| {
                        if r.font.as_deref() == Some(style.font.as_str()) {
                            r.font = None;
                        }
                        if r.size_pt.is_some_and(|s| (s - style.size_pt).abs() < 0.01) {
                            r.size_pt = None;
                        }
                        if r.color.as_deref() == Some(style.color.as_str()) {
                            r.color = None;
                        }
                        r
                    })
                    .collect(),
            })
            .collect();
        let radius = text_box.attribute((DRAW, "corner-radius")).and_then(length_mm).unwrap_or(0.0);
        let text_frame = (g.fill.is_some() || g.stroke.is_some() || radius > 0.0).then(|| TextFrame { fill: g.fill.clone(), stroke: g.stroke.clone(), corner_radius_mm: radius, image: None, pattern: None });
        let props = TextProps { content: RichText { paragraphs }, style, prompt: None, frame: text_frame };
        self.items.push(Item::Element(Box::new(element(frame, ElementKind::Text(props)))));
    }

    fn image(&mut self, frame: Frame, image: Node) {
        // Image de la banque OpenMoji, telle que l'export de Cahutin la range : elle redevient une image de la banque.
        let code = image.attribute((XLINK, "href")).and_then(|h| h.strip_prefix("Pictures/openmoji-")).and_then(|h| h.strip_suffix(".svg"));
        if let Some(code) = code.filter(|c| crate::picto::svg(c).is_some()) {
            self.items.push(Item::Element(Box::new(element(frame, ElementKind::Image(ImageProps { code: code.to_string() })))));
            return;
        }
        let asset = image.attribute((XLINK, "href")).and_then(|href| read_entry(self.zip, href)).and_then(|bytes| import_bytes(&bytes).ok());
        match asset {
            Some(a) => {
                let photo = PhotoProps { asset: Some(a.meta.id.clone()), crop: Crop::COVER, corner_radius_mm: 0.0, border: None, shadow: false, mirrored: false };
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
            // Courbes et polygones quelconques : redessinés ensemble dans une image fixe (voir `vector`).
            "path" | "polygon" | "polyline" => {
                let g = n.attribute((DRAW, "style-name")).and_then(|s| self.styles.graphic.get(s)).cloned().unwrap_or_default();
                let view_box = n.attribute((SVG, "viewBox"));
                let drawing = frame_of(n).and_then(|f| match (tag.name(), n.attribute((SVG, "d")), n.attribute((DRAW, "points"))) {
                    ("path", Some(d), _) => Drawing::from_svg_path(d, &f, view_box, g.fill.as_deref(), g.stroke.as_ref()),
                    (kind, _, Some(points)) if kind != "path" => Drawing::from_points(points, kind == "polygon", &f, view_box, g.fill.as_deref(), g.stroke.as_ref()),
                    _ => None,
                });
                match drawing {
                    Some(d) => self.drawings.push(d),
                    None => self.skipped += 1,
                }
            }
            "custom-shape" => match (frame_of(n), custom_shape_kind(n)) {
                (Some(f), Some(kind)) => self.shape(n, f, kind),
                _ => self.skipped += 1,
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

    /// Puce ou numéro d'une liste (une case à part dans l'import PDF) : rattachés au début de la ligne qui
    /// commence juste à sa droite, à la même hauteur, et écrits dans sa police.
    fn attach_bullets(&mut self) {
        let text = |l: &Line| l.runs.iter().map(|r| r.text.as_str()).collect::<String>().trim().to_string();
        let is_marker = |l: &Line| {
            let t = text(l);
            let mut c = t.chars();
            matches!((c.next(), c.next()), (Some('•' | '◦' | '▪' | '■' | '●' | '○' | '\u{2013}' | '✓' | '➢' | '➤' | '-'), None))
                || (t.len() > 1 && t.len() <= 3 && t.ends_with(['.', ')']) && t[..t.len() - 1].chars().all(|c| c.is_ascii_digit() || c.is_ascii_lowercase()))
        };
        let mut gone = vec![false; self.lines.len()];
        for b in 0..self.lines.len() {
            if !is_marker(&self.lines[b]) {
                continue;
            }
            let m = &self.lines[b];
            let (end, mid) = (m.x_mm + m.w_mm, m.y_mm + m.h_mm / 2.0);
            let next = (0..self.lines.len())
                .filter(|&i| i != b && !gone[i] && !is_marker(&self.lines[i]))
                .filter(|&i| {
                    let l = &self.lines[i];
                    l.x_mm >= end - 1.0 && l.x_mm - end < 15.0 && (l.y_mm + l.h_mm / 2.0 - mid).abs() < m.h_mm.max(l.h_mm) / 2.0
                })
                .min_by(|&i, &j| self.lines[i].x_mm.total_cmp(&self.lines[j].x_mm));
            let Some(i) = next else { continue };
            let marker = text(&self.lines[b]);
            let (bx, by, bh) = (self.lines[b].x_mm, self.lines[b].y_mm, self.lines[b].h_mm);
            let line = &mut self.lines[i];
            let mut first = line.runs[0].clone();
            first.text = format!("{marker} ");
            first.bold = false;
            first.italic = false;
            first.underline = false;
            line.runs.insert(0, first);
            let top = line.y_mm.min(by);
            line.h_mm = (line.y_mm + line.h_mm).max(by + bh) - top;
            line.y_mm = top;
            line.w_mm += line.x_mm - bx;
            line.x_mm = bx;
            gone[b] = true;
        }
        // Les puces rattachées quittent la page ; les autres lignes gardent leur ordre.
        let mut new_index = vec![None; self.lines.len()];
        let mut kept = Vec::new();
        for (i, l) in std::mem::take(&mut self.lines).into_iter().enumerate() {
            if !gone[i] {
                new_index[i] = Some(kept.len());
                kept.push(l);
            }
        }
        self.lines = kept;
        self.items = std::mem::take(&mut self.items)
            .into_iter()
            .filter_map(|it| match it {
                Item::Line(i) => new_index[i].map(Item::Line),
                e => Some(e),
            })
            .collect();
    }

    fn finish(mut self) -> (Vec<Element>, Vec<Asset>, usize, Vec<Drawing>, usize) {
        self.attach_bullets();
        let obstacles: Vec<Frame> = self.items.iter().filter_map(|it| if let Item::Element(e) = it { Some(e.frame) } else { None }).collect();
        let blocks = merge_lines(&obstacles, self.lines);
        let mut by_last: HashMap<usize, Element> = blocks.iter().map(|b| (b.last, block_element(b))).collect();
        let elements = self
            .items
            .into_iter()
            .filter_map(|it| match it {
                Item::Element(e) => Some(*e),
                Item::Line(i) => by_last.remove(&i),
            })
            .collect();
        (elements, self.assets, self.skipped, self.drawings, self.shrunk)
    }
}

/// Pages Carnet tirées de l'ODG produit par `draw_pdf_import`.
pub fn read_odg(input: impl Read + Seek, known_fonts: &[String]) -> Result<Imported, ImportError> {
    read(input, known_fonts, false)
}

/// Pages Carnet d'un dessin LibreOffice lu tel quel (dont un export de Cahutin) : chaque cadre de texte reste
/// un texte entier.
pub fn read_drawing(input: impl Read + Seek, known_fonts: &[String]) -> Result<Imported, ImportError> {
    read(input, known_fonts, true)
}

fn read(input: impl Read + Seek, known_fonts: &[String], whole_frames: bool) -> Result<Imported, ImportError> {
    let mut zip = ZipArchive::new(input).map_err(|_| ImportError::Unreadable)?;
    let content = read_entry(&mut zip, "content.xml").and_then(|b| String::from_utf8(b).ok()).ok_or(ImportError::Unreadable)?;
    let styles_xml = read_entry(&mut zip, "styles.xml").and_then(|b| String::from_utf8(b).ok());
    let doc = roxmltree::Document::parse(&content).map_err(|_| ImportError::Unreadable)?;
    let styles = read_styles(&doc, known_fonts);
    let sizes = page_sizes(styles_xml.as_deref());
    let mut pages = Vec::new();
    let mut assets = Vec::new();
    let mut skipped = 0;
    let (mut drawn, mut drawn_pages, mut shrunk) = (0, 0, 0);
    for page_node in doc.descendants().filter(|n| n.has_tag_name((DRAW, "page"))) {
        let mut reader = PageReader { styles: &styles, zip: &mut zip, items: Vec::new(), lines: Vec::new(), assets: Vec::new(), skipped: 0, drawings: Vec::new(), shrunk: 0, whole_frames };
        for c in page_node.children() {
            reader.node(c);
        }
        let (elements, page_assets, page_skipped, drawings, page_shrunk) = reader.finish();
        shrunk += page_shrunk;
        let size = page_node.attribute((DRAW, "master-page-name")).and_then(|m| sizes.by_master.get(m)).copied().unwrap_or(sizes.default);
        let background = page_node.attribute((DRAW, "master-page-name")).and_then(|m| sizes.background.get(m)).cloned();
        let mut page = Page { elements, background, ..Page::new(size) };
        super::tidy::full_page_rect_to_background(&mut page);
        super::tidy::group_fragments(&mut page);
        // Les tracés au fond de la page, sous les textes et les photos, fixés : un clic sur la page ne les déplace pas.
        match (drawings.is_empty(), vector::render(&drawings)) {
            (true, _) => {}
            (false, Some((frame, asset))) => {
                let photo = PhotoProps { asset: Some(asset.meta.id.clone()), crop: Crop::COVER, corner_radius_mm: 0.0, border: None, shadow: false, mirrored: false };
                page.elements.insert(0, Element { locked: true, ..element(frame, ElementKind::Photo(photo)) });
                assets.push(asset);
                drawn += drawings.len();
                drawn_pages += 1;
            }
            (false, None) => skipped += drawings.len(),
        }
        pages.push(page);
        assets.extend(page_assets);
        skipped += page_skipped;
    }
    let counted = ImportReport::count(&pages);
    let report = ImportReport { skipped, drawn, shrunk, images: counted.images - drawn_pages, ..counted };
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
        let pages = format!(r#"<draw:page draw:name="page1">{content_body}</draw:page>"#);
        let layouts = format!(r#"<style:page-layout style:name="PM1"><style:page-layout-properties fo:margin-top="2cm" {page_layout}/></style:page-layout>"#);
        odg_raw(&pages, &layouts, "", pictures)
    }

    /// ODG aux pages, mises en page et pages maîtresses fournies.
    fn odg_raw(pages: &str, layouts: &str, masters: &str, pictures: &[(&str, Vec<u8>)]) -> Vec<u8> {
        let content = format!(
            r##"<?xml version="1.0" encoding="UTF-8"?>
<office:document-content xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0" xmlns:style="urn:oasis:names:tc:opendocument:xmlns:style:1.0" xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0" xmlns:draw="urn:oasis:names:tc:opendocument:xmlns:drawing:1.0" xmlns:fo="urn:oasis:names:tc:opendocument:xmlns:xsl-fo-compatible:1.0" xmlns:svg="urn:oasis:names:tc:opendocument:xmlns:svg-compatible:1.0" xmlns:xlink="http://www.w3.org/1999/xlink" xmlns:loext="urn:org:documentfoundation:names:experimental:office:xmlns:loext:1.0" office:version="1.3">
<office:font-face-decls>
<style:font-face style:name="Liberation Serif" svg:font-family="&apos;Liberation Serif&apos;"/>
<style:font-face style:name="Liberation Sans" svg:font-family="&apos;Liberation Sans&apos;" style:font-family-generic="roman"/>
<style:font-face style:name="Comic Inconnue" svg:font-family="&apos;Comic Inconnue&apos;"/>
<style:font-face style:name="Helvetica" svg:font-family="Helvetica"/>
</office:font-face-decls>
<office:automatic-styles>
<style:style style:name="gr1" style:family="graphic"><style:graphic-properties draw:stroke="solid" svg:stroke-width="0cm" svg:stroke-color="#3465a4" draw:fill="solid" draw:fill-color="#729fcf"/></style:style>
<style:style style:name="gr2" style:family="graphic"><style:graphic-properties draw:stroke="none" draw:fill="none"/></style:style>
<style:style style:name="gr3" style:family="graphic"><style:graphic-properties draw:stroke="none" draw:fill="solid" draw:fill-color="#ffffff"/></style:style>
<style:style style:name="T1" style:family="text"><style:text-properties fo:color="#000000" style:font-name="Liberation Serif" fo:font-size="12pt" fo:font-weight="normal"/></style:style>
<style:style style:name="T2" style:family="text"><style:text-properties fo:color="#c0392b" style:font-name="Liberation Sans" fo:font-size="24pt" fo:font-weight="bold"/></style:style>
<style:style style:name="T3" style:family="text"><style:text-properties fo:color="#000000" style:font-name="Liberation Serif" fo:font-size="12pt" fo:font-weight="bold"/></style:style>
<style:style style:name="T4" style:family="text"><style:text-properties fo:color="#000000" style:font-name="Comic Inconnue" fo:font-size="12pt" fo:font-style="italic" style:text-underline-style="solid"/></style:style>
<style:style style:name="T5" style:family="text"><style:text-properties fo:color="#000000" style:font-name="Helvetica" fo:font-size="12pt"/></style:style>
</office:automatic-styles>
<office:body><office:drawing>{pages}</office:drawing></office:body></office:document-content>"##
        );
        let styles = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document-styles xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0" xmlns:style="urn:oasis:names:tc:opendocument:xmlns:style:1.0" xmlns:draw="urn:oasis:names:tc:opendocument:xmlns:drawing:1.0" xmlns:fo="urn:oasis:names:tc:opendocument:xmlns:xsl-fo-compatible:1.0">
<office:automatic-styles>{layouts}</office:automatic-styles>
<office:master-styles>{masters}</office:master-styles>
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

    /// Fond dessiné entre la ligne 1 et la ligne 2 d'un paragraphe : le texte recollé reste au-dessus.
    #[test]
    fn merged_text_stays_above_fills_drawn_between_its_lines() {
        let body = text_frame("2cm", "4cm", "4.4cm", r#"<text:span text:style-name="T1">Complète le tableau avec </text:span>"#)
            + r#"<draw:polygon draw:style-name="gr1" svg:width="5cm" svg:height="1.2cm" svg:x="1.9cm" svg:y="3.9cm" svg:viewBox="0 0 5000 1200" draw:points="0,0 5000,0 5000,1200 0,1200"/>"#
            + &text_frame("2cm", "4.486cm", "4.1cm", r#"<text:span text:style-name="T1">les mots : rouge, ronde, </text:span>"#);
        let els = &read(&body).pages[0].elements;
        assert_eq!(els.len(), 2);
        assert!(matches!(els[0].kind, ElementKind::Shape(_)), "le texte est sous le fond : {:?}", els.iter().map(|e| std::mem::discriminant(&e.kind)).collect::<Vec<_>>());
        assert!(matches!(els[1].kind, ElementKind::Text(_)));
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

    /// Fond blanc que l'export PDF de LibreOffice dessine sous chaque page : pas un élément cliquable.
    #[test]
    fn the_white_page_rectangle_is_not_imported_as_a_shape() {
        let page = r#"<draw:polygon draw:style-name="gr3" svg:width="21.001cm" svg:height="29.7cm" svg:x="0cm" svg:y="0cm" svg:viewBox="0 0 21001 29700" draw:points="0,0 21001,0 21001,29700 0,29700"><text:p/></draw:polygon>"#;
        let small = r#"<draw:polygon draw:style-name="gr1" svg:width="4cm" svg:height="2cm" svg:x="2cm" svg:y="2cm" svg:viewBox="0 0 4000 2000" draw:points="0,0 4000,0 4000,2000 0,2000"><text:p/></draw:polygon>"#;
        let imported = read(&format!("{page}{small}"));
        let els = &imported.pages[0].elements;
        assert_eq!(els.len(), 1, "{els:?}");
        assert!((els[0].frame.w_mm - 40.0).abs() < 0.01);
        assert_eq!(imported.report.shapes, 1);
    }

    #[test]
    fn curves_become_a_fixed_picture_and_rotated_shapes_are_read() {
        let body = r#"<draw:polygon draw:style-name="gr1" svg:width="2cm" svg:height="2cm" svg:x="2cm" svg:y="2cm" svg:viewBox="0 0 2000 2000" draw:points="0,2000 1000,0 2000,2000"/>
<draw:path draw:style-name="gr1" svg:width="2cm" svg:height="2cm" svg:x="5cm" svg:y="2cm" svg:viewBox="0 0 2000 2000" svg:d="M 0 0 C 500 2000 1500 2000 2000 0"/>
<draw:rect draw:style-name="gr1" svg:width="2cm" svg:height="1cm" draw:transform="rotate (0.5) translate (8cm 2cm)"/>
<draw:g><draw:ellipse draw:style-name="gr1" svg:width="2cm" svg:height="1cm" svg:x="2cm" svg:y="8cm"/><draw:line draw:style-name="gr1" svg:x1="2cm" svg:y1="12cm" svg:x2="6cm" svg:y2="12cm"/></draw:g>"#;
        let imported = read(body);
        // Le triangle et la courbe : redessinés ensemble dans une image fixe, au fond de la page.
        assert_eq!((imported.report.skipped, imported.report.drawn, imported.report.images), (0, 2, 0));
        let back = &imported.pages[0].elements[0];
        assert!(back.locked && matches!(back.kind, ElementKind::Photo(_)), "{back:?}");
        assert!((back.frame.x_mm - 20.0).abs() < 1.0 && (back.frame.w_mm - 50.0).abs() < 2.0, "{:?}", back.frame);
        assert_eq!(imported.assets.len(), 1);
        let kinds: Vec<ShapeKind> = imported.pages[0].elements.iter().filter_map(|e| if let ElementKind::Shape(s) = &e.kind { Some(s.shape) } else { None }).collect();
        assert_eq!(kinds, [ShapeKind::Rect, ShapeKind::Ellipse, ShapeKind::Line]);
        // Tourné de 0,5 radian dans le sens ODF (inverse des aiguilles) : 331,4° dans Cahutin.
        let rect = &imported.pages[0].elements[1].frame;
        assert!((rect.rotation_deg - (360.0 - 0.5f64.to_degrees())).abs() < 0.01 && (rect.w_mm - 20.0).abs() < 0.01, "{rect:?}");
        let line = &imported.pages[0].elements[3].frame;
        assert!((line.x_mm - 20.0).abs() < 0.01 && (line.w_mm - 40.0).abs() < 0.01 && (line.y_mm + line.h_mm / 2.0 - 120.0).abs() < 0.01, "{line:?}");
    }

    /// Puce par défaut de LibreOffice (OpenSymbol, zone privée U+F095) : un « • » lisible dans toute police.
    #[test]
    fn symbol_font_bullets_become_bullets() {
        let t = read(&text_frame("2cm", "2cm", "1cm", "<text:span text:style-name=\"T1\">\u{F095}</text:span>"));
        assert_eq!(plain(texts(&t.pages[0])[0]), "•");
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

    /// PDF A5 paysage puis US Letter : LibreOffice range la mise en page Letter en premier.
    #[test]
    fn each_page_keeps_its_own_size() {
        let layouts = r#"<style:page-layout style:name="PM0"><style:page-layout-properties fo:page-width="21.59cm" fo:page-height="27.94cm"/></style:page-layout><style:page-layout style:name="PM1"><style:page-layout-properties fo:page-width="21cm" fo:page-height="14.8cm"/></style:page-layout>"#;
        let masters = r#"<style:master-page style:name="master-page1" style:page-layout-name="PM1"/><style:master-page style:name="master-page2" style:page-layout-name="PM0"/>"#;
        let pages = r#"<draw:page draw:name="page1" draw:master-page-name="master-page1"/><draw:page draw:name="page2" draw:master-page-name="master-page2"/>"#;
        let imported = read_odg(Cursor::new(odg_raw(pages, layouts, masters, &[])), &known()).unwrap();
        let sizes: Vec<(f64, f64)> = imported.pages.iter().map(|p| (p.size.width_mm, p.size.height_mm)).collect();
        assert_eq!(sizes, [(210.0, 148.0), (215.9, 279.4)]);
    }

    /// Police absente sans équivalent : la police de repli, plus large, rétrécit jusqu'à la largeur d'origine.
    #[test]
    fn a_missing_font_shrinks_to_the_width_it_had() {
        let imported = read(&text_frame("2cm", "2cm", "3cm", r#"<text:span text:style-name="T4">Sortie à la ferme pédagogique</text:span>"#));
        let t = texts(&imported.pages[0])[0];
        assert_eq!(t.style.font, FALLBACK_FONT);
        let size = t.content.paragraphs[0].runs[0].size_pt.unwrap_or(t.style.size_pt);
        assert!((6.0..12.0).contains(&size), "{size}");
        assert_eq!(imported.report.shrunk, 1);
        // Une ligne qui tient déjà garde sa taille.
        let short = read(&text_frame("2cm", "2cm", "8cm", r#"<text:span text:style-name="T4">Oui</text:span>"#));
        assert_eq!((texts(&short.pages[0])[0].style.size_pt, short.report.shrunk), (12.0, 0));
        // Sa ligne s'allonge à la hauteur de la police de repli, plus haute que l'originale.
        let line_h = crate::fonts::builtin_line_height_mm(FALLBACK_FONT, 12.0).unwrap();
        assert!(short.pages[0].elements[0].frame.h_mm >= line_h, "{:?} {line_h}", short.pages[0].elements[0].frame);
    }

    /// Helvetica absente : Liberation Sans, de même largeur, la remplace sans rien rétrécir.
    #[test]
    fn a_missing_font_takes_one_of_the_same_width() {
        let imported = read(&text_frame("2cm", "2cm", "3cm", r#"<text:span text:style-name="T5">Notre semaine à l'école</text:span>"#));
        let t = texts(&imported.pages[0])[0];
        assert_eq!((t.style.font.as_str(), t.style.size_pt, imported.report.shrunk), ("Liberation Sans", 12.0, 0));
        // Sa ligne a la hauteur de Liberation Sans, plus que le cadre mesuré pour Helvetica.
        assert!(imported.pages[0].elements[0].frame.h_mm >= 12.0 * 1.15 * 25.4 / 72.0, "{:?}", imported.pages[0].elements[0].frame);
    }

    /// Une puce de liste, case à part dans le PDF : elle rejoint le début de sa ligne, dans la police de la ligne.
    #[test]
    fn a_list_bullet_joins_its_line() {
        let body = text_frame("2cm", "5cm", "0.4cm", r#"<text:span text:style-name="T4"></text:span>"#)
            + &text_frame("2.6cm", "5cm", "6cm", r#"<text:span text:style-name="T1">un pique-nique et une gourde</text:span>"#);
        let imported = read(&body);
        let t = texts(&imported.pages[0]);
        assert_eq!(t.len(), 1);
        assert_eq!(plain(t[0]), "• un pique-nique et une gourde");
        assert_eq!(t[0].style.font, "Liberation Serif");
        assert!((imported.pages[0].elements[0].frame.x_mm - 20.0).abs() < 2.0, "{:?}", imported.pages[0].elements[0].frame);
    }

    #[test]
    fn report_counts_everything() {
        let body = text_frame("2cm", "2cm", "5cm", r#"<text:span text:style-name="T1">Un</text:span>"#)
            + r#"<draw:polygon draw:style-name="gr1" svg:width="2cm" svg:height="2cm" svg:x="2cm" svg:y="9cm" svg:viewBox="0 0 2000 2000" draw:points="0,0 2000,0 2000,2000 0,2000"/>"#
            + r#"<draw:path draw:style-name="gr1" svg:width="2cm" svg:height="2cm" svg:x="5cm" svg:y="2cm" svg:viewBox="0 0 2000 2000" svg:d="M 0 0 L 2000 2000"/>"#
            + r#"<draw:frame svg:width="2cm" svg:height="2cm" svg:x="9cm" svg:y="9cm"><draw:image xlink:href="Pictures/img.png"/></draw:frame>"#;
        let imported = read_odg(Cursor::new(odg_fixture(&body, A4_LAYOUT, &[("img.png", png())])), &known()).unwrap();
        assert_eq!(imported.report, ImportReport { pages: 1, texts: 1, images: 1, shapes: 1, skipped: 0, drawn: 1, shrunk: 0 });
    }
}
