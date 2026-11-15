//! Pages lues dans l'ODG que produit l'import PDF de LibreOffice (`draw_pdf_import`) : textes,
//! images et formes simples deviennent des éléments Carnet ; le reste est compté comme non repris.

use super::paragraphs::{Line, block_element, merge_lines};
use super::vector::{self, Drawing};
use super::{ImportError, ImportReport, Imported};
use crate::model::*;
use crate::photo::{Asset, import_bytes_many, looks_importable};
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
const META: &str = "urn:oasis:names:tc:opendocument:xmlns:meta:1.0";
const TABLE: &str = "urn:oasis:names:tc:opendocument:xmlns:table:1.0";
const LOEXT: &str = "urn:org:documentfoundation:names:experimental:office:xmlns:loext:1.0";

/// Police mise à la place d'une police absente du poste.
const FALLBACK_FONT: &str = "Andika";
/// Au plus autant de tracés sur une page pour les reprendre en formes modifiables ; au-delà, une image fixe.
const EDITABLE_TRACES: usize = 60;
/// Trait « fin » d'une forme dont l'épaisseur vaut 0.
const HAIRLINE_MM: f64 = 0.2;
use crate::odg::CELL_BORDER_DRAWN;
/// Épaisseurs du trait d'un tableau relu : de 0,5 à 3 mm (celles du réglage « Traits »), au 1/20 de mm (un tableau
/// de Cahutin peut en avoir une autre que le réglage, comme les 0,6 mm de l'exemple « Notre semaine », qui revient
/// tel quel de son export : 0,340 mm écrit, 0,5998 mm dessiné).
const TABLE_BORDER_STEP_MM: f64 = 0.05;
const TABLE_BORDER_MIN_MM: f64 = 0.5;
const TABLE_BORDER_MAX_MM: f64 = 3.0;
/// Une case vide plus petite (l'astuce qui tient basse une ligne vide) prend la taille d'un texte ordinaire.
const MIN_CELL_TEXT_PT: f64 = 6.0;

/// Trait d'un tableau relu : le plus fréquent des côtés de ses cases (épaisseur et couleur ; le premier en cas
/// d'égalité), à l'épaisseur que dessine LibreOffice ramenée à celles de Cahutin (un trait visible garde au moins la
/// plus fine), toujours plein ; aucun trait si la plupart des côtés n'en ont pas. Les traits différents d'un côté à
/// l'autre, les tirets et les pointillés sont perdus.
fn table_border(sides: &[Option<CellBorder>]) -> Option<Stroke> {
    let key = |b: &Option<CellBorder>| b.as_ref().map(|b| ((b.width_mm * 1000.0).round() as i64, b.color.clone()));
    let mut counts: Vec<(Option<(i64, String)>, usize)> = Vec::new();
    for s in sides {
        let k = key(s);
        match counts.iter_mut().find(|(c, _)| *c == k) {
            Some((_, n)) => *n += 1,
            None => counts.push((k, 1)),
        }
    }
    let mut best: Option<&(Option<(i64, String)>, usize)> = None;
    for c in &counts {
        if best.is_none_or(|b| c.1 > b.1) {
            best = Some(c);
        }
    }
    let (width, color) = best?.0.clone()?;
    let drawn = width as f64 / 1000.0 * CELL_BORDER_DRAWN;
    let width_mm = (((drawn / TABLE_BORDER_STEP_MM).round() * TABLE_BORDER_STEP_MM * 1000.0).round() / 1000.0).clamp(TABLE_BORDER_MIN_MM, TABLE_BORDER_MAX_MM);
    Some(Stroke { width_mm, color, style: None })
}

/// Hauteur de ligne estimée d'un texte de tableau relu, en multiple de sa taille (Liberation Sans, la police d'un
/// tableau neuf de LibreOffice : 1,149 ; Andika telle que LibreOffice la pose : 1,36) : une estimation, plutôt basse pour
/// les polices de Cahutin, qui ne sert qu'aux tableaux dont le cadre et les lignes se contredisent.
const CELL_LINE_EM: f64 = 1.15;

/// Hauteurs des lignes d'un tableau dont le cadre (`frame_mm`) ne vaut pas la somme des hauteurs écrites (`written`),
/// comme les pose LibreOffice (`svx/source/table/tablelayouter.cxx`, `LayoutTableHeight` puis `distribute`, au 1/100 mm
/// comme lui) : chaque ligne d'abord agrandie à ce que demande son contenu (`mins`), puis la différence avec le cadre
/// répartie au prorata des hauteurs, sur toutes les lignes quand le cadre est plus grand, sur celles qui dépassent leur
/// contenu quand il est plus petit ; une ligne passée sous son contenu y est remise, et la répartition recommence
/// avec ce qu'elle a coûté en plus (LibreOffice la refait sur toute la différence : le tableau peut finir plus petit que
/// son cadre, et c'est ce qu'il montre). Mesuré sur le poste (LibreOffice 26.8) : lignes de 10 et 20 mm dans un cadre
/// de 50 mm, 16,67 et 33,33 ; de 20 mm, 6,67 et 13,33 ; lignes de 20 et 20 mm dont la première demande 36,65 mm, dans
/// un cadre de 80 mm, 51,76 et 28,24 ; de 40 mm, 36,65 et 3,35 ; lignes de 10 et 20 mm demandant chacune 9,73 mm, dans
/// un cadre de 20 mm, 9,73 et 9,75.
fn fit_rows(written: &[f64], mins: &[f64], frame_mm: f64) -> Vec<f64> {
    let hundredths = |v: f64| (v * 100.0).round() as i64;
    let mins: Vec<i64> = mins.iter().map(|m| hundredths(*m)).collect();
    let mut sizes: Vec<i64> = written.iter().zip(&mins).map(|(h, m)| hundredths(*h).max(*m)).collect();
    let mut distribute = hundredths(frame_mm) - sizes.iter().sum::<i64>();
    let last = sizes.len().saturating_sub(1);
    for _ in 0..100 {
        if distribute == 0 {
            break;
        }
        for (s, m) in sizes.iter_mut().zip(&mins) {
            if *s < *m {
                distribute -= *m - *s;
                *s = *m;
            }
        }
        let eligible = |s: i64, m: i64, d: i64| d > 0 || s > m;
        let pool: i64 = sizes.iter().zip(&mins).filter(|(s, m)| eligible(**s, **m, distribute)).map(|(s, _)| *s).sum();
        if pool == 0 || distribute == 0 {
            break;
        }
        let (mut left, mut broken) = (distribute, false);
        for (i, (s, m)) in sizes.iter_mut().zip(&mins).enumerate() {
            if eligible(*s, *m, distribute) {
                let n = if i == last { left } else { distribute * *s / pool };
                *s += n;
                left -= n;
                broken |= *s < *m;
            }
        }
        if !broken {
            break;
        }
    }
    sizes.into_iter().map(|s| s as f64 / 100.0).collect()
}

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
    /// Encre transparente (`loext:opacity="0%"`) : un champ non rempli de Cahutin, écrit sous son nom.
    transparent: bool,
}

#[derive(Clone, Default)]
struct GraphicStyle {
    fill: Option<String>,
    /// Remplie d'une image (photo dans une forme) : le nom de sa définition `draw:fill-image`.
    fill_image: Option<String>,
    stroke: Option<Stroke>,
    /// Motif du pointillé (trait, espace, en mm), d'après sa définition `draw:stroke-dash`.
    dash: Option<(f64, f64)>,
    valign: Option<VAlign>,
    padding_mm: Option<f64>,
    /// Pointe au bout du trait (`draw:marker-end`) : sa largeur en mm (4 mm sans largeur écrite).
    marker_end: Option<f64>,
    /// Pointe au début du trait (`draw:marker-start`).
    marker_start: bool,
    /// Ombre visible (`draw:shadow`).
    shadow: bool,
    /// Place ou taille protégée (`style:protect`) : l'objet est verrouillé.
    protect: bool,
    /// Marge du haut (`fo:padding-top`), en mm.
    padding_top_mm: Option<f64>,
    /// Allure du texte tapé dans la forme (`style:text-properties` du style et de ses parents) ; sans couleur : la
    /// couleur automatique de LibreOffice (voir `auto_text_color`).
    text: SpanStyle,
}

struct Styles {
    text: HashMap<String, SpanStyle>,
    graphic: HashMap<String, GraphicStyle>,
    /// Alignement des styles de paragraphe.
    align: HashMap<String, Align>,
    /// Interligne des styles de paragraphe (`fo:line-height` en pour cent), en multiple de la hauteur naturelle.
    line_spacing: HashMap<String, f64>,
    /// Styles de paragraphe écrits sur des lignes d'écriture : une hauteur de ligne d'au moins un multiple de l'écart des
    /// lignes (`style:line-height-at-least`, ce qu'écrit l'export pour un texte sur les lignes).
    on_lines: std::collections::HashSet<String>,
    /// Styles de liste : vrai quand leur premier niveau est une puce (un caractère ou une image), faux pour des numéros.
    bullet_lists: HashMap<String, bool>,
    /// Styles de case de tableau (famille `table-cell`), de `content.xml` puis de `styles.xml`.
    cells: HashMap<String, CellStyle>,
    /// Largeurs des styles de colonne et hauteurs des styles de ligne, en mm.
    columns: HashMap<String, f64>,
    rows: HashMap<String, f64>,
}

/// Trait d'un côté de case : épaisseur écrite (mm) et couleur ; son genre (plein, tirets...) n'est pas gardé.
#[derive(Clone, Debug, PartialEq)]
struct CellBorder {
    width_mm: f64,
    color: String,
}

/// `fo:border` (« 0.28pt solid #000000 », « none ») ; sans épaisseur lisible : aucun trait ; sans couleur : noir.
fn cell_border(v: &str) -> Option<CellBorder> {
    let mut width_mm = None;
    let (mut kind, mut color) = (None, None);
    for part in v.split_whitespace() {
        if part.starts_with('#') {
            color = Some(part.to_lowercase());
        } else if let Some(w) = length_mm(part).filter(|_| part.starts_with(|c: char| c.is_ascii_digit() || c == '.')) {
            width_mm = Some(w);
        } else {
            kind = Some(part);
        }
    }
    (!matches!(kind, Some("none" | "hidden"))).then_some(CellBorder { width_mm: width_mm.filter(|w| *w > 0.0)?, color: color.unwrap_or_else(|| "#000000".into()) })
}

/// Style de case de tableau, tel qu'écrit : chaque réglage absent vient du style parent (`style:parent-style-name`).
#[derive(Clone, Default)]
struct CellStyle {
    parent: Option<String>,
    /// Fond : `Some(None)` pour `draw:fill="none"`, qui l'emporte sur le fond du parent.
    fill: Option<Option<String>>,
    /// Image de remplissage : son nom et `style:repeat` (`repeat` quand il manque, comme le veut l'ODF).
    fill_image: Option<Option<(String, String)>>,
    padding_mm: Option<f64>,
    /// Marges du haut et du bas, ensemble (la hauteur que la case prend en plus de son texte).
    padding_v_mm: Option<f64>,
    valign: Option<VAlign>,
    /// Traits gauche, droit, haut, bas : `Some(None)` pour un côté sans trait.
    borders: [Option<Option<CellBorder>>; 4],
    /// Allure du texte de la case (police, taille, couleur), sous celle des paragraphes.
    text: Option<SpanStyle>,
}

impl CellStyle {
    fn read(s: Node, fonts: &Fonts) -> CellStyle {
        let mut c = CellStyle { parent: s.attribute((STYLE, "parent-style-name")).map(String::from), ..CellStyle::default() };
        // Les réglages de la case sont dans `loext:graphic-properties` (ou, dans les fichiers anciens, `style:graphic-properties`).
        if let Some(g) = s.children().find(|n| n.has_tag_name((LOEXT, "graphic-properties")) || n.has_tag_name((STYLE, "graphic-properties"))) {
            c.fill = g.attribute((DRAW, "fill")).map(|f| (f == "solid").then(|| g.attribute((DRAW, "fill-color")).unwrap_or("#ffffff").to_string()));
            c.fill_image = g.attribute((DRAW, "fill")).map(|f| {
                (f == "bitmap").then(|| g.attribute((DRAW, "fill-image-name")).map(|n| (n.to_string(), g.attribute((STYLE, "repeat")).unwrap_or("repeat").to_string()))).flatten()
            });
            c.padding_mm = attr_mm(g, FO, "padding-left").or(attr_mm(g, FO, "padding"));
            let all = attr_mm(g, FO, "padding");
            c.padding_v_mm = match (attr_mm(g, FO, "padding-top").or(all), attr_mm(g, FO, "padding-bottom").or(all)) {
                (None, None) => None,
                (t, b) => Some(t.unwrap_or(0.0) + b.unwrap_or(0.0)),
            };
            c.valign = g.attribute((DRAW, "textarea-vertical-align")).map(|v| match v {
                "middle" => VAlign::Middle,
                "bottom" => VAlign::Bottom,
                _ => VAlign::Top,
            });
        }
        if let Some(p) = s.children().find(|n| n.has_tag_name((STYLE, "paragraph-properties"))) {
            let all = p.attribute((FO, "border")).map(cell_border);
            for (i, side) in ["border-left", "border-right", "border-top", "border-bottom"].into_iter().enumerate() {
                c.borders[i] = p.attribute((FO, side)).map(cell_border).or(all.clone());
            }
        }
        c.text = s.children().find(|n| n.has_tag_name((STYLE, "text-properties"))).map(|t| fonts.span_style(t));
        c
    }
}

/// Styles de case, de colonne et de ligne des tableaux, de `content.xml` puis de `styles.xml` (le style par défaut
/// d'un tableau fait dans LibreOffice, `default`, y est rangé).
fn table_styles(docs: &[&roxmltree::Document], known_fonts: &[String]) -> (HashMap<String, CellStyle>, HashMap<String, f64>, HashMap<String, f64>) {
    let fonts = Fonts::new(docs, known_fonts);
    let (mut cells, mut columns, mut rows) = (HashMap::new(), HashMap::new(), HashMap::new());
    let mut drawing: HashMap<String, CellStyle> = HashMap::new();
    for s in docs.iter().flat_map(|d| d.descendants()).filter(|n| n.has_tag_name((STYLE, "style"))) {
        let Some(name) = s.attribute((STYLE, "name")) else { continue };
        let props = |tag: &str| s.children().find(|n| n.has_tag_name((STYLE, tag)));
        match s.attribute((STYLE, "family")) {
            Some("table-cell") => {
                cells.entry(name.to_string()).or_insert_with(|| CellStyle::read(s, &fonts));
            }
            // Une ligne ajoutée dans LibreOffice a pour style de case par défaut un style de dessin (`standard` :
            // fond vert, marges, Liberation Sans 18 pt), que LibreOffice applique à ses cases.
            Some("graphic") => {
                drawing.entry(name.to_string()).or_insert_with(|| CellStyle::read(s, &fonts));
            }
            Some("table-column") => {
                if let Some(w) = props("table-column-properties").and_then(|p| attr_mm(p, STYLE, "column-width")) {
                    columns.entry(name.to_string()).or_insert(w);
                }
            }
            Some("table-row") => {
                if let Some(h) = props("table-row-properties").and_then(|p| attr_mm(p, STYLE, "row-height").or(attr_mm(p, STYLE, "min-row-height"))) {
                    rows.entry(name.to_string()).or_insert(h);
                }
            }
            _ => {}
        }
    }
    for (name, style) in drawing {
        cells.entry(name).or_insert(style);
    }
    (cells, columns, rows)
}

impl Styles {
    /// Le style de case `name`, ses réglages absents pris dans ses parents.
    fn cell(&self, name: Option<&str>) -> CellStyle {
        let mut out = CellStyle::default();
        let mut next = name.map(String::from);
        // Une chaîne de parents qui boucle (fichier abîmé) s'arrête.
        for _ in 0..16 {
            let Some(s) = next.take().and_then(|n| self.cells.get(&n)) else { break };
            out.fill = out.fill.or(s.fill.clone());
            out.fill_image = out.fill_image.or(s.fill_image.clone());
            out.padding_mm = out.padding_mm.or(s.padding_mm);
            out.padding_v_mm = out.padding_v_mm.or(s.padding_v_mm);
            out.valign = out.valign.or(s.valign);
            for i in 0..4 {
                out.borders[i] = out.borders[i].take().or(s.borders[i].clone());
            }
            out.text = match (out.text.take(), &s.text) {
                (Some(t), Some(p)) => Some(SpanStyle { font: t.font.or(p.font.clone()), size_pt: t.size_pt.or(p.size_pt), color: t.color.or(p.color.clone()), ..t }),
                (t, p) => t.or(p.clone()),
            };
            next = s.parent.clone();
        }
        out
    }
}

/// Styles de liste (`text:list-style`) de `content.xml`, puis de `styles.xml` : le premier niveau est-il une puce ?
fn bullet_lists(docs: &[&roxmltree::Document]) -> HashMap<String, bool> {
    let mut out = HashMap::new();
    for l in docs.iter().flat_map(|d| d.descendants()).filter(|n| n.has_tag_name((TEXT, "list-style"))) {
        let Some(name) = l.attribute((STYLE, "name")) else { continue };
        let first = l.children().find(|n| n.is_element() && n.attribute((TEXT, "level")) == Some("1"));
        let bullet = first.is_some_and(|n| n.has_tag_name((TEXT, "list-level-style-bullet")) || n.has_tag_name((TEXT, "list-level-style-image")));
        out.entry(name.to_string()).or_insert(bullet);
    }
    out
}

/// Images de remplissage (`draw:fill-image`, le plus souvent dans `styles.xml`) : leur nom, et le chemin de
/// l'image dans le fichier.
fn fill_images(docs: &[&roxmltree::Document]) -> HashMap<String, String> {
    docs.iter()
        .flat_map(|d| d.descendants())
        .filter(|n| n.has_tag_name((DRAW, "fill-image")))
        .filter_map(|n| Some((n.attribute((DRAW, "name"))?.to_string(), n.attribute((XLINK, "href"))?.to_string())))
        .collect()
}

/// Motifs de pointillé (`draw:stroke-dash`, dans `styles.xml` ou `content.xml`) : trait et espace en mm.
/// Les longueurs en pourcentage de l'épaisseur ne sont pas reprises.
fn stroke_dashes(docs: &[&roxmltree::Document]) -> HashMap<String, (f64, f64)> {
    docs.iter()
        .flat_map(|d| d.descendants())
        .filter(|n| n.has_tag_name((DRAW, "stroke-dash")))
        .filter_map(|n| {
            let on = n.attribute((DRAW, "dots1-length")).and_then(length_mm)?;
            Some((n.attribute((DRAW, "name"))?.to_string(), (on, n.attribute((DRAW, "distance")).and_then(length_mm)?)))
        })
        .collect()
}

/// Genre de chaque motif de pointillé (`draw:stroke-dash`) : des points quand le trait est court (au plus 1,5 fois
/// l'épaisseur, ou 0,5 mm, ou des points sans longueur), des tirets sinon. L'export de Cahutin écrit 100 % pour les
/// points et 300 % pour les tirets.
fn dash_styles(docs: &[&roxmltree::Document]) -> HashMap<String, StrokeStyle> {
    docs.iter()
        .flat_map(|d| d.descendants())
        .filter(|n| n.has_tag_name((DRAW, "stroke-dash")))
        .filter_map(|n| {
            let on = n.attribute((DRAW, "dots1-length")).map(str::trim);
            let dotted = match on {
                None => true,
                Some(v) => match v.strip_suffix('%') {
                    Some(p) => p.parse::<f64>().is_ok_and(|p| p <= 150.0),
                    None => length_mm(v).is_some_and(|mm| mm <= 0.5),
                },
            };
            Some((n.attribute((DRAW, "name"))?.to_string(), if dotted { StrokeStyle::Dotted } else { StrokeStyle::Dashed }))
        })
        .collect()
}

/// Polices nommées par les styles (`style:font-face`) et polices du poste : la police retenue pour un nom.
struct Fonts<'a> {
    faces: HashMap<String, String>,
    known: &'a [String],
}

impl<'a> Fonts<'a> {
    fn new(docs: &[&roxmltree::Document], known: &'a [String]) -> Fonts<'a> {
        let mut faces: HashMap<String, String> = HashMap::new();
        for f in docs.iter().flat_map(|d| d.descendants()).filter(|n| n.has_tag_name((STYLE, "font-face"))) {
            if let (Some(name), Some(family)) = (f.attribute((STYLE, "name")), f.attribute((SVG, "font-family"))) {
                faces.entry(name.to_string()).or_insert_with(|| family.trim_matches(|c| c == '\'' || c == '"').to_string());
            }
        }
        Fonts { faces, known }
    }

    /// La police elle-même, sinon une police de même largeur (les textes gardent leur longueur), sinon la police de repli.
    /// Police retenue, remplacée par la police de repli, remplacée par une police de même largeur.
    fn resolve(&self, name: &str) -> (String, bool, bool) {
        let family = self.faces.get(name).map(String::as_str).unwrap_or(name);
        let known = |f: &str| self.known.iter().find(|k| k.eq_ignore_ascii_case(f)).cloned();
        if let Some(f) = known(family) {
            return (f, false, false);
        }
        match same_width_fonts(family).iter().find_map(|f| known(f)) {
            Some(f) => (f, false, true),
            None => (FALLBACK_FONT.to_string(), !family.eq_ignore_ascii_case(FALLBACK_FONT), false),
        }
    }

    /// Allure d'un `style:text-properties`.
    fn span_style(&self, t: Node) -> SpanStyle {
        self.span_style_by(|ns, attr| t.attribute((ns, attr)).map(String::from))
    }

    /// Allure d'après ses réglages, lus par `a` (espace de noms, nom).
    fn span_style_by(&self, a: impl Fn(&str, &str) -> Option<String>) -> SpanStyle {
        let font = a(STYLE, "font-name").map(|n| self.resolve(&n));
        SpanStyle {
            font: font.as_ref().map(|f| f.0.clone()),
            substituted: font.as_ref().is_some_and(|f| f.1),
            equivalent: font.as_ref().is_some_and(|f| f.2),
            size_pt: a(FO, "font-size").as_deref().and_then(|v| v.strip_suffix("pt")).and_then(|v| v.parse().ok()),
            color: a(FO, "color"),
            bold: a(FO, "font-weight").as_deref() == Some("bold"),
            italic: a(FO, "font-style").as_deref() == Some("italic"),
            underline: a(STYLE, "text-underline-style").is_some_and(|u| u != "none"),
            outline: a(STYLE, "text-outline").as_deref() == Some("true"),
            transparent: a(LOEXT, "opacity").as_deref() == Some("0%"),
        }
    }
}

/// Styles de `content.xml` (`doc`) ; `common` : `styles.xml`. Un style de dessin qui nomme un parent
/// (`style:parent-style-name`, le plus souvent `standard` dans `styles.xml`) en prend chaque réglage qu'il n'écrit pas :
/// une forme dessinée dans LibreOffice n'écrit que ce qui diffère de `standard`. Sans parent nommé, rien n'est hérité.
fn read_styles(doc: &roxmltree::Document, common: Option<&roxmltree::Document>, known_fonts: &[String], dashes: &HashMap<String, (f64, f64)>, dash_kinds: &HashMap<String, StrokeStyle>) -> Styles {
    let fonts = Fonts::new(&[doc], known_fonts);
    let mut styles = Styles { text: HashMap::new(), graphic: HashMap::new(), align: HashMap::new(), line_spacing: HashMap::new(), on_lines: Default::default(), bullet_lists: HashMap::new(), cells: HashMap::new(), columns: HashMap::new(), rows: HashMap::new() };
    let is_graphic = |n: Node| n.attribute((STYLE, "family")) == Some("graphic");
    // Styles de dessin par nom, ceux de `content.xml` d'abord.
    let mut by_name: HashMap<&str, Node> = HashMap::new();
    for s in [Some(doc), common].into_iter().flatten().flat_map(|d| d.descendants()).filter(|n| n.has_tag_name((STYLE, "style")) && is_graphic(*n)) {
        if let Some(name) = s.attribute((STYLE, "name")) {
            by_name.entry(name).or_insert(s);
        }
    }
    // Un réglage du style (dans ses `props` : `graphic-properties`, `text-properties`), sinon de ses parents (une chaîne
    // qui boucle, fichier abîmé, s'arrête).
    let get = |s: Node<'_, '_>, props: &str, ns: &str, attr: &str| -> Option<String> {
        let mut cur = Some(s);
        for _ in 0..16 {
            let c = cur?;
            if let Some(v) = c.children().find(|n| n.has_tag_name((STYLE, props))).and_then(|g| g.attribute((ns, attr))) {
                return Some(v.to_string());
            }
            cur = c.attribute((STYLE, "parent-style-name")).and_then(|p| by_name.get(p).copied());
        }
        None
    };
    // Couleur du texte : celle du style le plus proche qui en dit une ; `use-window-font-color` (la couleur automatique
    // de LibreOffice, que `standard` écrit) ou rien d'écrit : aucune.
    let text_color = |s: Node<'_, '_>| -> Option<String> {
        let mut cur = Some(s);
        for _ in 0..16 {
            let c = cur?;
            if let Some(t) = c.children().find(|n| n.has_tag_name((STYLE, "text-properties"))) {
                if t.attribute((STYLE, "use-window-font-color")) == Some("true") {
                    return None;
                }
                if let Some(v) = t.attribute((FO, "color")) {
                    return Some(v.to_string());
                }
            }
            cur = c.attribute((STYLE, "parent-style-name")).and_then(|p| by_name.get(p).copied());
        }
        None
    };
    // Les polices nommées par les styles parents sont déclarées dans `styles.xml`.
    let all_fonts = Fonts::new(&[Some(doc), common].into_iter().flatten().collect::<Vec<_>>(), known_fonts);
    let graphic_of = |s: Node| -> GraphicStyle {
        let a = |ns: &str, attr: &str| get(s, "graphic-properties", ns, attr);
        let mm = |ns: &str, attr: &str| a(ns, attr).as_deref().and_then(length_mm);
        let (fill_kind, stroke_kind) = (a(DRAW, "fill"), a(DRAW, "stroke"));
        let fill = (fill_kind.as_deref() == Some("solid")).then(|| a(DRAW, "fill-color").unwrap_or_else(|| "#ffffff".into()));
        let stroke = matches!(stroke_kind.as_deref(), Some("solid") | Some("dash")).then(|| Stroke {
            width_mm: mm(SVG, "stroke-width").filter(|w| *w > 0.0).unwrap_or(HAIRLINE_MM),
            color: a(SVG, "stroke-color").unwrap_or_else(|| "#000000".into()),
            style: (stroke_kind.as_deref() == Some("dash")).then(|| a(DRAW, "stroke-dash").and_then(|d| dash_kinds.get(&d)).copied().unwrap_or(StrokeStyle::Dashed)),
        });
        let valign = a(DRAW, "textarea-vertical-align").map(|v| match v.as_str() {
            "middle" => VAlign::Middle,
            "bottom" => VAlign::Bottom,
            _ => VAlign::Top,
        });
        let dash = (stroke_kind.as_deref() == Some("dash")).then(|| a(DRAW, "stroke-dash").and_then(|d| dashes.get(&d)).copied()).flatten();
        let fill_image = (fill_kind.as_deref() == Some("bitmap")).then(|| a(DRAW, "fill-image-name")).flatten();
        let marker_end = a(DRAW, "marker-end").filter(|m| !m.is_empty()).map(|_| mm(DRAW, "marker-end-width").unwrap_or(4.0));
        let marker_start = a(DRAW, "marker-start").is_some_and(|m| !m.is_empty());
        let shadow = a(DRAW, "shadow").as_deref() == Some("visible");
        let protect = a(STYLE, "protect").is_some_and(|p| p.contains("position") || p.contains("size"));
        // L'encre transparente de `standard` (`loext:opacity="0%"`, sans effet sous la couleur automatique) n'est pas la
        // marque d'un champ de Cahutin.
        let text = SpanStyle { color: text_color(s), transparent: false, ..all_fonts.span_style_by(|ns, attr| get(s, "text-properties", ns, attr)) };
        GraphicStyle { fill, fill_image, stroke, dash, valign, padding_mm: mm(FO, "padding-left"), marker_end, marker_start, shadow, protect, padding_top_mm: mm(FO, "padding-top"), text }
    };
    for s in doc.descendants().filter(|n| n.has_tag_name((STYLE, "style"))) {
        let Some(name) = s.attribute((STYLE, "name")) else { continue };
        if let Some(t) = s.children().find(|n| n.has_tag_name((STYLE, "text-properties"))) {
            styles.text.insert(name.to_string(), fonts.span_style(t));
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
        if let Some(p) = s.children().find(|n| n.has_tag_name((STYLE, "paragraph-properties"))).and_then(|p| p.attribute((FO, "line-height"))).and_then(|v| v.strip_suffix('%')).and_then(|v| v.parse::<f64>().ok()) {
            styles.line_spacing.insert(name.to_string(), p / 100.0);
        }
        let step = crate::background::WRITING_LINE_MM;
        if s.children().find(|n| n.has_tag_name((STYLE, "paragraph-properties"))).and_then(|p| attr_mm(p, STYLE, "line-height-at-least")).is_some_and(|h| h > 0.0 && ((h / step).round() * step - h).abs() < 0.02) {
            styles.on_lines.insert(name.to_string());
        }
        if s.children().any(|n| n.has_tag_name((STYLE, "graphic-properties"))) || (is_graphic(s) && s.has_attribute((STYLE, "parent-style-name"))) {
            styles.graphic.insert(name.to_string(), graphic_of(s));
        }
    }
    // Un objet peut aussi porter un style de `styles.xml` (`standard`, `objectwithoutfill`...).
    for s in common.iter().flat_map(|d| d.descendants()).filter(|n| n.has_tag_name((STYLE, "style")) && is_graphic(*n)) {
        if let Some(name) = s.attribute((STYLE, "name")).filter(|n| !styles.graphic.contains_key(*n)) {
            styles.graphic.insert(name.to_string(), graphic_of(s));
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
    let fills = drawing_page_fills(&doc);
    for m in doc.descendants().filter(|n| n.has_tag_name((STYLE, "master-page"))) {
        if let (Some(name), Some(size)) = (m.attribute((STYLE, "name")), m.attribute((STYLE, "page-layout-name")).and_then(|l| layouts.get(l))) {
            sizes.by_master.insert(name.to_string(), *size);
        }
        if let (Some(name), Some(color)) = (m.attribute((STYLE, "name")), m.attribute((DRAW, "style-name")).and_then(|d| fills.get(d))) {
            let size = sizes.by_master.get(name).copied().unwrap_or(sizes.default);
            sizes.background.insert(name.to_string(), Background { color: color.clone(), pattern: master_pattern(&doc, m, size) });
        }
    }
    sizes
}

/// Trait ou point d'un motif lu : extrémités (ou centre et diamètre, en double), épaisseur, couleur.
type ReadMark = (bool, [f64; 4], f64, String);

/// Un motif tel que l'écrit l'export : ses traits (`Mark::Line`) et ses points (`Mark::Dot`), dans le repère `at`.
fn expected_marks(marks: Vec<crate::background::Mark>, at: impl Fn(f64, f64) -> (f64, f64)) -> Vec<ReadMark> {
    use crate::background::Mark;
    marks
        .into_iter()
        .map(|m| match m {
            Mark::Line { x1, y1, x2, y2, width_mm, color } => {
                let ((ax, ay), (bx, by)) = (at(x1, y1), at(x2, y2));
                (true, [ax, ay, bx, by], width_mm, color.to_lowercase())
            }
            Mark::Dot { x, y, d_mm, color } => {
                let (cx, cy) = at(x, y);
                (false, [cx, cy, d_mm, d_mm], 0.0, color.to_lowercase())
            }
        })
        .collect()
}

/// Les traits et points lus sont exactement ceux attendus (au 1/20 de mm près, LibreOffice écrit au 1/100 de mm), dans
/// n'importe quel ordre.
fn same_marks(read: Vec<ReadMark>, expected: Vec<ReadMark>) -> bool {
    if read.len() != expected.len() {
        return false;
    }
    let mut used = vec![false; read.len()];
    expected.iter().all(|b| {
        let close = |a: &ReadMark| a.0 == b.0 && a.3 == b.3 && (a.2 - b.2).abs() < 0.02 && a.1.iter().zip(&b.1).all(|(x, y)| (x - y).abs() < 0.05);
        match (0..read.len()).find(|&i| !used[i] && close(&read[i])) {
            Some(i) => {
                used[i] = true;
                true
            }
            None => false,
        }
    })
}

/// Formats de page de Cahutin, petit et grand côté en mm (A5, A4, A3 : `FORMATS` de ui/src/lib/pageFormat.ts).
const CAHUTIN_FORMATS: [(f64, f64); 3] = [(148.0, 210.0), (210.0, 297.0), (297.0, 420.0)];

/// Motif d'école d'une page maîtresse : ses traits (`draw:line`) et points (`draw:ellipse`, que LibreOffice réécrit
/// en `draw:circle`) sont exactement ceux d'un motif de Cahutin à cette taille de page (`background::marks`), ou à un
/// format de Cahutin, en portrait ou en paysage : LibreOffice 24.2 réenregistre toutes les pages maîtresses sur une
/// seule mise en page (A4 portrait, même quand la première page est A5), et les marques d'une page d'un autre
/// format, inchangées, ne sont plus à la taille lue. Sinon aucun (la couleur du fond reste).
fn master_pattern(doc: &roxmltree::Document, master: Node, size: PageSize) -> Option<Pattern> {
    let styles: HashMap<&str, Node> = doc.descendants().filter(|n| n.has_tag_name((STYLE, "style"))).filter_map(|n| Some((n.attribute((STYLE, "name"))?, n))).collect();
    let props = |n: Node| n.attribute((DRAW, "style-name")).and_then(|s| styles.get(s)).and_then(|s| s.children().find(|c| c.has_tag_name((STYLE, "graphic-properties"))));
    let mut read: Vec<ReadMark> = Vec::new();
    for n in master.children().filter(|n| n.is_element()) {
        let g = props(n);
        let attr = |ns, a| g.and_then(|g| g.attribute((ns, a))).map(str::to_lowercase).unwrap_or_default();
        if n.has_tag_name((DRAW, "line")) {
            let p = [attr_mm(n, SVG, "x1")?, attr_mm(n, SVG, "y1")?, attr_mm(n, SVG, "x2")?, attr_mm(n, SVG, "y2")?];
            read.push((true, p, g.and_then(|g| attr_mm(g, SVG, "stroke-width")).unwrap_or(0.0), attr(SVG, "stroke-color")));
        } else if n.has_tag_name((DRAW, "ellipse")) || n.has_tag_name((DRAW, "circle")) {
            let (w, h) = (attr_mm(n, SVG, "width")?, attr_mm(n, SVG, "height")?);
            let (x, y) = (attr_mm(n, SVG, "x")?, attr_mm(n, SVG, "y")?);
            read.push((false, [x + w / 2.0, y + h / 2.0, w, h], 0.0, attr(DRAW, "fill-color")));
        } else if n.has_tag_name((DRAW, "frame")) || n.has_tag_name((DRAW, "rect")) || n.has_tag_name((DRAW, "custom-shape")) {
            return None;
        }
    }
    if read.is_empty() {
        return None;
    }
    let at = |size: PageSize| [Pattern::Seyes, Pattern::SmallSquares, Pattern::Lines, Pattern::Dots].into_iter().find(|p| same_marks(read.clone(), expected_marks(crate::background::marks(size, *p), |x, y| (x, y))));
    let formats = CAHUTIN_FORMATS.into_iter().flat_map(|(s, l)| [(s, l), (l, s)]).map(|(width_mm, height_mm)| PageSize { width_mm, height_mm });
    at(size).or_else(|| formats.filter(|f| *f != size).find_map(at))
}

/// Couleur de chaque style de page rempli d'une couleur (`style:drawing-page-properties`, `draw:fill="solid"`).
fn drawing_page_fills(doc: &roxmltree::Document) -> HashMap<String, String> {
    doc.descendants()
        .filter(|n| n.has_tag_name((STYLE, "style")) && n.attribute((STYLE, "family")) == Some("drawing-page"))
        .filter_map(|n| {
            let p = n.children().find(|c| c.has_tag_name((STYLE, "drawing-page-properties")))?;
            (p.attribute((DRAW, "fill")) == Some("solid")).then_some(())?;
            Some((n.attribute((STYLE, "name"))?.to_string(), p.attribute((DRAW, "fill-color"))?.to_string()))
        })
        .collect()
}

/// Élément posé, ou place réservée à une ligne de texte (remplacée par son texte recollé).
enum Item {
    Element(Box<Element>),
    Line(usize),
}

struct PageReader<'a, R: Read + Seek> {
    styles: &'a Styles,
    /// Images de remplissage, par nom (voir `fill_images`).
    fill_images: &'a HashMap<String, String>,
    /// Images déjà lues sur les pages précédentes : leur chemin dans le fichier, et leur image dans l'application.
    known: &'a HashMap<String, String>,
    /// Images de la page à décoder (chemin, octets, proportions de leur cadre), toutes ensemble à la fin de la
    /// page (voir `decode_page`).
    pending: Vec<(String, Vec<u8>, f64)>,
    zip: &'a mut ZipArchive<R>,
    items: Vec<Item>,
    lines: Vec<Line>,
    assets: Vec<Asset>,
    skipped: usize,
    /// Parmi `skipped`, les formes toutes faites de LibreOffice (smiley, flèche de liaison...) que Cahutin ne redessine pas.
    ready_made: usize,
    drawings: Vec<Drawing>,
    /// Lignes rétrécies parce que leur police manquait.
    shrunk: usize,
    /// Un dessin LibreOffice lu tel quel (dont l'export de Cahutin) : chaque cadre de texte reste un texte, avec
    /// ses alignements, sa marge et son fond. Sinon (import d'un PDF), un cadre par ligne, recollées ensuite.
    whole_frames: bool,
    /// Premier objet posé dans le groupe (`draw:g`) en cours de lecture : un cadre à lignes d'écriture ne reprend rien
    /// d'avant (voir `lined_frame`).
    group_floor: usize,
    /// Fichier écrit par Cahutin avant `odg::MIRROR_AFTER_ROTATION` : une forme retournée d'un seul sens y porte
    /// l'angle de l'écran, et non celui de LibreOffice.
    screen_mirror_angles: bool,
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

/// Commandes M, L, C et Z, chacune devant ses coordonnées : LibreOffice, en réenregistrant, ne répète pas la lettre
/// d'une commande qui se suit (« L 500 0 0 1000 »). Des coordonnées de plus après un M sont des L.
fn spelled_out(path: &str) -> Option<String> {
    let mut out: Vec<&str> = Vec::new();
    let mut last: Option<(&str, usize)> = None;
    let mut tokens = path.split_whitespace();
    while let Some(t) = tokens.next() {
        let (cmd, n, first) = match t {
            "M" | "L" => (t, 2, None),
            "C" => (t, 6, None),
            "Z" => (t, 0, None),
            _ => match last {
                Some(("M" | "L", n)) => ("L", n, Some(t)),
                Some(("C", n)) => ("C", n, Some(t)),
                _ => return None,
            },
        };
        out.push(cmd);
        out.extend(first);
        for _ in first.iter().count()..n {
            out.push(tokens.next()?);
        }
        last = Some((cmd, n));
    }
    Some(out.join(" "))
}

/// Tracé d'un `draw:custom-shape`, commandes épelées (voir `spelled_out`).
fn enhanced_path(g: Node) -> Option<String> {
    spelled_out(g.attribute((DRAW, "enhanced-path"))?.trim_end_matches(" N").trim())
}

/// Tracé libre d'un `draw:custom-shape` écrit par Cahutin : repère de 0 à 1000, commandes M, L, C et Z.
fn free_path(n: Node) -> Option<String> {
    let g = n.children().find(|c| c.has_tag_name((DRAW, "enhanced-geometry")))?;
    if g.attribute((SVG, "viewBox")) != Some("0 0 1000 1000") {
        return None;
    }
    let path = enhanced_path(g)?;
    crate::shapes::valid_path(&path).then_some(path)
}

/// Forme d'un `draw:custom-shape` : une forme de Cahutin reconnue à son tracé (ce qu'écrit l'export), ou une
/// forme toute faite de LibreOffice d'après son type.
fn custom_shape_kind(n: Node) -> Option<ShapeKind> {
    let g = n.children().find(|c| c.has_tag_name((DRAW, "enhanced-geometry")))?;
    let path = enhanced_path(g);
    let ours = [ShapeKind::Star, ShapeKind::Heart, ShapeKind::Bubble, ShapeKind::Cloud].into_iter().find(|k| crate::shapes::path(*k).is_some_and(|sp| Some(sp) == path.as_deref()));
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
    Element { id: new_id(), frame, locked: false, zone: false, group: None, kind, subgroups: Vec::new(), flip_h: false, flip_v: false }
}

/// Caractères de la zone privée (puces des polices de symboles : OpenSymbol, Symbol, Wingdings) : « • ».
fn readable(text: &str) -> String {
    text.chars().map(|c| if ('\u{E000}'..='\u{F8FF}').contains(&c) { '•' } else { c }).collect()
}

/// Texte d'un nœud, espaces ODF (`text:s`, `text:tab`, `text:line-break`) compris. `exact` : une tabulation et un
/// retour à la ligne restent ce qu'ils sont (un dessin lu tel quel, comme l'export de Cahutin les écrit) ; sinon (import
/// d'un PDF), une espace.
fn node_text(n: Node, out: &mut String, exact: bool) {
    for c in n.children() {
        push_text(c, out, exact);
    }
}

/// Le texte d'un nœud lui-même (voir `node_text`) : un texte, une espace ODF, ou le texte de ses enfants.
fn push_text(c: Node, out: &mut String, exact: bool) {
    if c.is_text() {
        out.push_str(&readable(c.text().unwrap_or("")));
    } else if c.has_tag_name((TEXT, "s")) {
        let count = c.attribute((TEXT, "c")).and_then(|v| v.parse().ok()).unwrap_or(1);
        out.push_str(&" ".repeat(count));
    } else if exact && c.has_tag_name((TEXT, "tab")) {
        out.push('\t');
    } else if exact && c.has_tag_name((TEXT, "line-break")) {
        out.push('\n');
    } else if c.has_tag_name((TEXT, "tab")) || c.has_tag_name((TEXT, "line-break")) {
        out.push(' ');
    } else {
        node_text(c, out, exact);
    }
}

/// Image de la banque OpenMoji, telle que l'export de Cahutin la range : elle redevient une image de la banque, avec
/// son miroir de gauche à droite et de haut en bas. Une image à colorier est rangée sous `openmoji-contour-<code>.svg` ;
/// en miroir, l'export écrit une copie retournée nommée `…-miroir-h.svg` (`-v`, `-hv`).
fn openmoji(href: &str) -> Option<(ImageProps, bool, bool)> {
    let name = href.strip_prefix("Pictures/openmoji-")?.strip_suffix(".svg")?;
    let (name, flip_h, flip_v) = match name.rsplit_once("-miroir-") {
        Some((n, m)) if matches!(m, "h" | "v" | "hv") => (n, m.contains('h'), m.contains('v')),
        _ => (name, false, false),
    };
    let (code, outline) = match name.strip_prefix("contour-") {
        Some(c) => (c, Some(true)),
        None => (name, None),
    };
    crate::picto::svg(code).is_some().then(|| (ImageProps { code: code.to_string(), outline }, flip_h, flip_v))
}

/// Pictogramme reconnu à ses octets, quel que soit le nom que LibreOffice a donné au fichier : le commentaire qui ouvre
/// le SVG exporté par Cahutin le nomme (`odg::PICTO_MARK`), et le SVG doit être celui que Cahutin écrirait pour ce nom
/// (une image remplacée ou retouchée reste une photo). Un SVG d'un fichier plus ancien, sans commentaire, est cherché
/// parmi les images de la banque et leurs versions à colorier (voir `bank_svg`) ; retourné, il n'y est pas.
fn openmoji_bytes(bytes: &[u8]) -> Option<(ImageProps, bool, bool)> {
    let Some(rest) = bytes.strip_prefix(crate::odg::PICTO_MARK.as_bytes()) else { return bank_svg(bytes).map(|p| (p, false, false)) };
    let end = rest.windows(4).position(|w| w == b" -->")?;
    let stem = std::str::from_utf8(&rest[..end]).ok()?;
    let (props, h, v) = openmoji(&format!("Pictures/{stem}.svg"))?;
    (crate::odg::picto_file(&props.code, props.outline == Some(true), h, v)? == bytes).then_some((props, h, v))
}

/// L'image de la banque (ou sa version à colorier) dont le SVG est exactement `bytes`. L'empreinte de toutes les
/// images n'est calculée qu'une fois, au premier SVG qui ressemble à une image OpenMoji (`id="emoji"`).
fn bank_svg(bytes: &[u8]) -> Option<ImageProps> {
    use std::hash::{DefaultHasher, Hash, Hasher};
    static BANK: std::sync::OnceLock<HashMap<u64, Vec<String>>> = std::sync::OnceLock::new();
    if !bytes.windows(10).any(|w| w == b"id=\"emoji\"") {
        return None;
    }
    let hash = |b: &[u8]| {
        let mut h = DefaultHasher::new();
        b.hash(&mut h);
        h.finish()
    };
    let bank = BANK.get_or_init(|| {
        let mut out: HashMap<u64, Vec<String>> = HashMap::new();
        for name in crate::picto::names() {
            if let Some(svg) = crate::picto::svg(&name) {
                out.entry(hash(&svg)).or_default().push(name);
            }
        }
        out
    });
    let name = bank.get(&hash(bytes))?.iter().find(|n| crate::picto::svg(n).as_deref() == Some(bytes))?;
    Some(match name.strip_prefix("contour/") {
        Some(code) => ImageProps { code: code.to_string(), outline: Some(true) },
        None => ImageProps { code: name.clone(), outline: None },
    })
}

/// Image de remplissage d'une case pictogramme exportée par Cahutin (`odg::PICTO_CELL_FILL`), même quand LibreOffice a
/// renommé son fichier en réenregistrant : son nom garde celui du SVG (`_` y est écrit `_5f_`).
fn openmoji_cell_fill(name: &str) -> Option<(ImageProps, bool, bool)> {
    let name = name.replace("_5f_", "_");
    openmoji(&format!("Pictures/{}.svg", name.strip_prefix(crate::odg::PICTO_CELL_FILL)?))
}

/// Les champs que l'export de Cahutin nomme dans la description d'un cadre de texte (`svg:desc`, « {PRENOM_1}
/// {prenom_2} ») : ceux qui s'affichent comme un autre (`fields::shown_is_shared`), dans l'ordre du texte. Une
/// description qui dit autre chose n'en nomme aucun.
fn field_marks(frame: Node) -> Vec<String> {
    let Some(desc) = frame.children().find(|c| c.has_tag_name((SVG, "desc"))).and_then(|d| d.text()) else { return Vec::new() };
    let marks: Option<Vec<String>> = desc.split_whitespace().map(|t| t.strip_prefix('{')?.strip_suffix('}').filter(|f| crate::fields::shown_is_shared(f)).map(String::from)).collect();
    marks.unwrap_or_default()
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

/// Un paragraphe lu : son alignement s'il en a un, ses passages, le passage qui donne l'allure d'une ligne vide, sa puce,
/// son interligne s'il en a un.
struct ReadParagraph {
    align: Option<Align>,
    runs: Vec<Run>,
    first: Run,
    bullet: bool,
    line_spacing: Option<f64>,
    /// Écrit sur des lignes d'écriture (voir `Styles::on_lines`).
    on_lines: bool,
}

/// Couleur automatique du texte d'une forme (LibreOffice : `style:use-window-font-color`), d'après son fond plein :
/// blanche sur un fond foncé, noire sur un fond clair ou sans fond plein. LibreOffice 26.8 la rend blanche jusqu'à une
/// luminance relative (sRGB) entre 0,342 et 0,352 (relevé sur un rendu), d'où 0,345. LibreOffice 24.2 compare une autre
/// luminance à 156 : les deux s'accordent sur les gris et les fonds foncés, pas sur un vert, un orange ou un cyan vifs,
/// que 24.2 écrit en blanc.
fn auto_text_color(fill: Option<&str>) -> &'static str {
    let channel = |h: &str, i: usize| -> Option<f64> {
        let c = f64::from(u8::from_str_radix(h.get(i..i + 2)?, 16).ok()?) / 255.0;
        Some(if c <= 0.04045 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) })
    };
    let luminance = fill.and_then(|f| f.strip_prefix('#')).filter(|h| h.len() == 6).and_then(|h| Some(0.2126 * channel(h, 0)? + 0.7152 * channel(h, 2)? + 0.0722 * channel(h, 4)?));
    match luminance {
        Some(l) if l <= 0.345 => "#ffffff",
        _ => "#000000",
    }
}

/// Style d'un texte importé : la police, la taille et la couleur de son premier passage, l'alignement de son premier
/// paragraphe.
fn text_style(first: &ReadParagraph, outline: bool, valign: Option<VAlign>, padding_mm: Option<f64>) -> TextStyle {
    TextStyle {
        font: first.first.font.clone().unwrap_or_else(|| FALLBACK_FONT.into()),
        size_pt: first.first.size_pt.unwrap_or(12.0),
        color: first.first.color.clone().unwrap_or_else(|| "#000000".into()),
        align: first.align.unwrap_or(Align::Left),
        valign,
        padding_mm,
        outline: outline.then_some(true),
        named: None,
        // L'interligne du premier paragraphe ; 100 % (ce qu'écrit l'export sans réglage) : aucun.
        line_spacing: first.line_spacing.filter(|v| (v - 1.0).abs() > 1e-6),
    }
}

/// Ce qui ne fait que répéter le style du texte est retiré des morceaux, comme dans un texte de Cahutin.
fn rich_text(paragraphs: Vec<ReadParagraph>, style: &TextStyle) -> RichText {
    let paragraphs = paragraphs
        .into_iter()
        .map(|p| Paragraph {
            align: p.align.filter(|a| *a != style.align),
            runs: p
                .runs
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
            list: p.bullet.then_some(ListKind::Bullet),
        })
        .collect();
    RichText { paragraphs }
}

impl<R: Read + Seek> PageReader<'_, R> {
    fn shape(&mut self, n: Node, frame: Frame, shape: ShapeKind) {
        let g = n.attribute((DRAW, "style-name")).and_then(|s| self.styles.graphic.get(s)).cloned().unwrap_or_default();
        let (fill, stroke) = if crate::shapes::is_stroke_only(shape) {
            (None, Some(g.stroke.unwrap_or(Stroke { width_mm: HAIRLINE_MM, color: "#000000".into(), style: None })))
        } else {
            (g.fill, g.stroke)
        };
        let corner_radius_mm = if shape == ShapeKind::Rect { attr_mm(n, DRAW, "corner-radius").unwrap_or(0.0) } else { 0.0 };
        self.items.push(Item::Element(Box::new(element(frame, ElementKind::Shape(ShapeProps { shape, fill, stroke, corner_radius_mm, path: None, graphisme: None })))));
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
                    node_text(c, &mut t, false);
                    push(t, &st);
                } else {
                    let mut t = String::new();
                    if c.is_text() {
                        t.push_str(&readable(c.text().unwrap_or("")));
                    } else {
                        node_text(c, &mut t, false);
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

    /// Premier paragraphe d'un élément d'une liste à puces d'un seul niveau : une liste numérotée, une liste dans une
    /// liste ou qui en contient une, un titre de liste (`text:list-header`) et les paragraphes suivants d'un élément
    /// restent des paragraphes simples (leur texte gardé, sans marque).
    fn bulleted(&self, p: Node) -> bool {
        let Some(item) = p.parent().filter(|n| n.has_tag_name((TEXT, "list-item"))) else { return false };
        let Some(list) = item.parent().filter(|n| n.has_tag_name((TEXT, "list"))) else { return false };
        item.children().find(|n| n.is_element()) == Some(p)
            && !list.ancestors().skip(1).any(|n| n.has_tag_name((TEXT, "list")))
            && !list.descendants().skip(1).any(|n| n.has_tag_name((TEXT, "list")))
            && list.attribute((TEXT, "style-name")).and_then(|s| self.styles.bullet_lists.get(s)).copied().unwrap_or(false)
    }

    /// Paragraphes d'un cadre de texte ou d'une case, avec leur alignement et leur marque de puce, et vrai si des lettres
    /// sont creuses. `fallback` donne ce que ni le paragraphe ni le passage ne disent (l'allure du texte d'une case) ;
    /// `zero_width` retire les espaces sans largeur (U+200B), qui tiennent basse une ligne de tableau vide.
    /// Un passage à l'encre transparente qui dit exactement le nom d'un champ redevient ce champ ; une lettre (qui ne
    /// montre que son numéro) ne le redevient que si `marks` (voir `field_marks`) nomme dans l'ordre les lettres que
    /// montrent ces passages. Un autre passage transparent reste du texte, à l'encre de sa couleur.
    fn paragraphs<'n>(&self, ps: impl Iterator<Item = Node<'n, 'n>>, fallback: &SpanStyle, zero_width: bool, marks: &[String]) -> (Vec<ReadParagraph>, bool) {
        let mut paragraphs: Vec<ReadParagraph> = Vec::new();
        let mut outline = false;
        // Passages transparents qui montrent un nom de champ partagé (paragraphe, passage).
        let mut shared: Vec<(usize, usize)> = Vec::new();
        for p in ps {
            let p_style = p.attribute((TEXT, "style-name"));
            let base = match p_style.and_then(|s| self.styles.text.get(s)) {
                Some(b) => SpanStyle { font: b.font.clone().or(fallback.font.clone()), size_pt: b.size_pt.or(fallback.size_pt), color: b.color.clone().or(fallback.color.clone()), ..b.clone() },
                None => fallback.clone(),
            };
            let mut runs: Vec<Run> = Vec::new();
            for c in p.children() {
                let st = if c.has_tag_name((TEXT, "span")) { c.attribute((TEXT, "style-name")).and_then(|s| self.styles.text.get(s)).cloned().unwrap_or_else(|| base.clone()) } else { base.clone() };
                let mut t = String::new();
                push_text(c, &mut t, self.whole_frames);
                if zero_width {
                    t.retain(|c| c != '\u{200B}');
                }
                if t.is_empty() {
                    continue;
                }
                outline |= st.outline;
                if st.transparent {
                    match crate::fields::from_shown_run(&t) {
                        Some(fields) => t = fields.iter().map(|f| format!("{{{f}}}")).collect(),
                        None => shared.push((paragraphs.len(), runs.len())),
                    }
                }
                runs.push(Run {
                    text: t,
                    bold: st.bold,
                    italic: st.italic,
                    underline: st.underline,
                    color: Some(st.color.clone().or(fallback.color.clone()).unwrap_or_else(|| "#000000".into())),
                    font: Some(st.font.clone().or(base.font.clone()).unwrap_or_else(|| FALLBACK_FONT.into())),
                    size_pt: Some(st.size_pt.or(base.size_pt).unwrap_or(12.0)),
                });
            }
            // Le style du paragraphe donne la police d'une ligne vide.
            let first = runs.first().cloned().unwrap_or(Run { font: base.font.clone(), size_pt: base.size_pt, color: base.color.clone(), ..Run::plain("") });
            let line_spacing = p_style.and_then(|s| self.styles.line_spacing.get(s)).copied();
            let on_lines = p_style.is_some_and(|s| self.styles.on_lines.contains(s));
            paragraphs.push(ReadParagraph { align: p_style.and_then(|s| self.styles.align.get(s)).copied(), runs, first, bullet: self.bulleted(p), line_spacing, on_lines });
        }
        // Chaque passage prend les lettres suivantes de la description tant qu'elles montrent son texte (LibreOffice recolle
        // deux lettres voisines en un passage : « 12 ») ; toutes doivent servir, et chaque passage en avoir.
        let mut next = 0;
        let mut taken: Vec<String> = Vec::new();
        for &(p, r) in &shared {
            let text = &paragraphs[p].runs[r].text;
            let (mut shown, mut tokens) = (String::new(), String::new());
            while shown.len() < text.len() && next < marks.len() {
                shown.push_str(&crate::fields::shown(&marks[next]).unwrap_or_default());
                tokens.push_str(&format!("{{{}}}", marks[next]));
                next += 1;
            }
            if shown != *text {
                taken.clear();
                break;
            }
            taken.push(tokens);
        }
        if next == marks.len() && taken.len() == shared.len() {
            for (&(p, r), tokens) in shared.iter().zip(taken) {
                paragraphs[p].runs[r].text = tokens;
            }
        }
        (paragraphs, outline)
    }

    /// Un cadre de texte entier : ses paragraphes et leur alignement, l'alignement vertical, la marge, le fond,
    /// la bordure et l'arrondi du cadre. La mention de licence OpenMoji, ajoutée par l'export, est laissée.
    fn whole_text_box(&mut self, frame: Frame, frame_node: Node, text_box: Node) {
        let g = frame_node.attribute((DRAW, "style-name")).and_then(|s| self.styles.graphic.get(s)).cloned().unwrap_or_default();
        let (paragraphs, outline) = self.paragraphs(text_box.descendants().filter(|n| n.has_tag_name((TEXT, "p"))), &SpanStyle::default(), false, &field_marks(frame_node));
        let plain: String = paragraphs.iter().flat_map(|p| p.runs.iter().map(|r| r.text.as_str())).collect();
        if plain.trim() == crate::odg::OPENMOJI_CREDIT_PAGE || plain.trim() == crate::odg::OPENMOJI_CREDIT {
            return;
        }
        let radius = text_box.attribute((DRAW, "corner-radius")).and_then(length_mm).unwrap_or(0.0);
        // Cadre à lignes d'écriture : son fond, sa bordure et ses lignes, dessinés juste avant par l'export, le redeviennent.
        let visible = g.fill.is_some() || g.stroke.is_some() || g.fill_image.is_some();
        let lined = if visible { None } else { self.lined_frame(frame) };
        // Sans paragraphe ni cadre visible, rien ne se voit : rien n'est repris.
        if paragraphs.is_empty() && !visible && lined.is_none() {
            return;
        }
        let text_frame = match lined {
            Some(mut lined) => {
                // Écrit sur les lignes : la hauteur de ligne du paragraphe, ou (une étiquette vide, dont LibreOffice ne
                // garde pas le style du paragraphe) la marge du haut, la descendante de la police, qui n'est alors pas
                // celle des côtés.
                let top_differs = matches!((g.padding_top_mm, g.padding_mm), (Some(t), Some(l)) if (t - l).abs() > 0.01);
                let on_lines = paragraphs.first().is_some_and(|p| p.on_lines) || top_differs;
                lined.write_on_lines = (on_lines && matches!(lined.pattern, Some(Pattern::Seyes | Pattern::Lines))).then_some(true);
                Some(lined)
            }
            None => {
                // Image de fond : une photo de la photothèque, décodée à la fin de la page comme les autres (`decode_page`).
                let image = g.fill_image.as_ref().and_then(|name| Some((name.clone(), self.fill_images.get(name)?.clone()))).and_then(|(name, href)| self.fill_picture(frame, &name, &href, radius)).and_then(|e| match e.kind {
                    ElementKind::Photo(p) => p.asset,
                    _ => None,
                });
                (visible || radius > 0.0 || image.is_some()).then(|| TextFrame { fill: g.fill.clone(), stroke: g.stroke.clone(), corner_radius_mm: radius, image, pattern: None, write_on_lines: None })
            }
        };
        // Une étiquette encadrée vide : l'allure de son paragraphe vide (ce qu'écrit l'export) ; sans elle (un paragraphe
        // vide sans style, ou aucun), celle d'une zone de texte.
        // L'allure du cadre : son style graphique (l'export), ou son style de texte (LibreOffice l'y range en
        // réenregistrant).
        let frame_text = [(DRAW, "style-name"), (DRAW, "text-style-name")].into_iter().filter_map(|a| frame_node.attribute(a).and_then(|s| self.styles.text.get(s))).find(|t| t.font.is_some());
        let style = match (paragraphs.first(), frame_text) {
            (Some(first), _) if first.first.font.is_some() || !first.runs.is_empty() => text_style(first, outline, g.valign, g.padding_mm),
            // Le paragraphe vide a perdu son style en passant par LibreOffice : celui du cadre (voir `odg::content`).
            (_, Some(t)) => {
                let first = Run { font: t.font.clone(), size_pt: t.size_pt, color: t.color.clone(), ..Run::plain("") };
                text_style(&ReadParagraph { align: paragraphs.first().and_then(|p| p.align), runs: Vec::new(), first, bullet: false, line_spacing: None, on_lines: false }, outline, g.valign, g.padding_mm)
            }
            _ => TextStyle { valign: g.valign, padding_mm: g.padding_mm, ..TextStyle::default_zone() },
        };
        // Rien d'écrit : aucun paragraphe, comme un texte vide de Cahutin.
        let blank = paragraphs.iter().all(|p| p.runs.iter().all(|r| r.text.trim().is_empty()));
        let content = if blank { RichText::default() } else { rich_text(paragraphs, &style) };
        let props = TextProps { content, style, prompt: None, frame: text_frame };
        self.items.push(Item::Element(Box::new(element(frame, ElementKind::Text(props)))));
    }

    /// Le fond, la bordure et les lignes (ou les points) d'un cadre de texte à lignes d'écriture tels que l'export les
    /// écrit (`odg::content`, `frame_lines`), juste avant le cadre de texte `frame` : un rectangle (ou, avec une image
    /// de fond, une photo) au même cadre, puis exactement les traits et les points d'un motif (`background::frame_marks`
    /// à la taille du cadre, tournés avec lui), hors de tout groupe et dans le même groupe LibreOffice que le texte. Ils
    /// sont retirés de la page et deviennent le cadre du texte. Si quelque chose ne colle pas (ou si le motif n'a aucun
    /// trait à cette taille), rien n'est retiré.
    fn lined_frame(&mut self, frame: Frame) -> Option<TextFrame> {
        let same = |a: &Frame| [a.x_mm - frame.x_mm, a.y_mm - frame.y_mm, a.w_mm - frame.w_mm, a.h_mm - frame.h_mm].iter().all(|d| d.abs() < 0.05) && ((a.rotation_deg - frame.rotation_deg + 180.0).rem_euclid(360.0) - 180.0).abs() < 0.01;
        let (cx, cy) = (frame.x_mm + frame.w_mm / 2.0, frame.y_mm + frame.h_mm / 2.0);
        let (sin, cos) = frame.rotation_deg.to_radians().sin_cos();
        let at = |x: f64, y: f64| {
            let (dx, dy) = (x - frame.w_mm / 2.0, y - frame.h_mm / 2.0);
            (cx + dx * cos - dy * sin, cy + dx * sin + dy * cos)
        };
        for pattern in [Pattern::Seyes, Pattern::SmallSquares, Pattern::Lines, Pattern::Dots] {
            let expected = expected_marks(crate::background::frame_marks(PageSize { width_mm: frame.w_mm, height_mm: frame.h_mm }, pattern), at);
            let n = expected.len();
            // Un cadre trop petit pour une seule ligne : rien ne dit que le rectangle d'avant est le sien.
            if n == 0 {
                continue;
            }
            let Some(back) = self.items.len().checked_sub(n + 1).filter(|b| *b >= self.group_floor) else { continue };
            let mut read: Vec<ReadMark> = Vec::new();
            for it in &self.items[back + 1..] {
                let Item::Element(e) = it else { break };
                let ElementKind::Shape(s) = &e.kind else { break };
                let f = e.frame;
                match (s.shape, &s.stroke, &s.fill) {
                    (ShapeKind::Line, Some(st), None) if e.group.is_none() => {
                        let (sn, cs) = f.rotation_deg.to_radians().sin_cos();
                        let (mx, my, hw) = (f.x_mm + f.w_mm / 2.0, f.y_mm + f.h_mm / 2.0, f.w_mm / 2.0);
                        read.push((true, [mx - hw * cs, my - hw * sn, mx + hw * cs, my + hw * sn], st.width_mm, st.color.to_lowercase()));
                    }
                    (ShapeKind::Ellipse, None, Some(fill)) if e.group.is_none() && f.rotation_deg == 0.0 => read.push((false, [f.x_mm + f.w_mm / 2.0, f.y_mm + f.h_mm / 2.0, f.w_mm, f.h_mm], 0.0, fill.to_lowercase())),
                    _ => break,
                }
            }
            if read.len() != n || !same_marks(read, expected) {
                continue;
            }
            let lined = match &self.items[back] {
                Item::Element(e) if e.group.is_none() && same(&e.frame) => match &e.kind {
                    ElementKind::Shape(s) if s.shape == ShapeKind::Rect => TextFrame { fill: s.fill.clone(), stroke: s.stroke.clone(), corner_radius_mm: s.corner_radius_mm, image: None, pattern: Some(pattern), write_on_lines: None },
                    ElementKind::Photo(p) => TextFrame { fill: None, stroke: p.border.clone(), corner_radius_mm: p.corner_radius_mm, image: p.asset.clone(), pattern: Some(pattern), write_on_lines: None },
                    _ => continue,
                },
                _ => continue,
            };
            self.items.truncate(back);
            return Some(lined);
        }
        None
    }

    /// Les images d'un cadre : la première que Cahutin sait lire. LibreOffice range souvent une image en deux
    /// exemplaires, un dessin (WMF, EMF, SVG) puis une copie en PNG pour les logiciels qui ne lisent pas le premier.
    fn image(&mut self, frame: Frame, images: Vec<Node>) {
        let href = images[0].attribute((XLINK, "href"));
        let picto = href.and_then(openmoji).or_else(|| href.filter(|h| h.ends_with(".svg")).and_then(|h| read_entry(self.zip, h)).and_then(|b| openmoji_bytes(&b)));
        if let Some((picto, flip_h, flip_v)) = picto {
            self.items.push(Item::Element(Box::new(Element { flip_h, flip_v, ..element(frame, ElementKind::Image(picto)) })));
            return;
        }
        if !images.iter().filter_map(|i| i.attribute((XLINK, "href"))).any(|href| self.photo(frame, href, 0.0)) {
            self.skipped += 1;
        }
    }

    /// Une photo dans `frame`, tirée de l'image `href` du fichier ; faux si ce n'est pas une image lisible. Elle
    /// est décodée à la fin de la page (`decode_page`) ; d'ici là, son image est un repère `PENDING`.
    fn photo(&mut self, frame: Frame, href: &str, corner_radius_mm: f64) -> bool {
        self.photo_from(frame, href, corner_radius_mm, None)
    }

    /// Comme `photo`, avec l'image déjà lue (`bytes`), rangée sous `href`.
    fn photo_from(&mut self, frame: Frame, href: &str, corner_radius_mm: f64, bytes: Option<Vec<u8>>) -> bool {
        let Some(e) = self.photo_element(frame, href, corner_radius_mm, bytes) else { return false };
        self.items.push(Item::Element(Box::new(e)));
        true
    }

    /// La photo que poserait `photo_from`, sans la poser.
    fn photo_element(&mut self, frame: Frame, href: &str, corner_radius_mm: f64, bytes: Option<Vec<u8>>) -> Option<Element> {
        let asset = match self.known.get(href) {
            Some(id) => id.clone(),
            None => {
                let at = match self.pending.iter().position(|(h, _, _)| h == href) {
                    Some(i) => i,
                    None => {
                        let bytes = bytes.or_else(|| read_entry(self.zip, href)).filter(|b| looks_importable(b))?;
                        self.pending.push((href.to_string(), bytes, frame.w_mm / frame.h_mm));
                        self.pending.len() - 1
                    }
                };
                format!("{PENDING}{at}")
            }
        };
        let photo = PhotoProps { asset: Some(asset), crop: Crop::COVER, corner_radius_mm, border: None, shadow: false, mirrored: false };
        Some(element(frame, ElementKind::Photo(photo)))
    }

    /// Le miroir d'une forme LibreOffice (`draw:mirror-horizontal`, `draw:mirror-vertical` de sa géométrie)
    /// passe à la forme qui vient d'être lue.
    fn mirror_last_shape(&mut self, n: Node) {
        let Some(g) = n.children().find(|c| c.has_tag_name((DRAW, "enhanced-geometry"))) else { return };
        let on = |a: &str| g.attribute((DRAW, a)) == Some("true");
        if let Some(Item::Element(e)) = self.items.last_mut() {
            e.flip_h = on("mirror-horizontal");
            e.flip_v = on("mirror-vertical");
            // LibreOffice retourne la forme après l'avoir tournée, l'écran avant : retournée d'un seul sens, elle est
            // tournée dans l'autre sens à l'écran (l'export écrit l'angle opposé, `odg::content`).
            if e.flip_h != e.flip_v && e.frame.rotation_deg != 0.0 && !self.screen_mirror_angles {
                e.frame.rotation_deg = 360.0 - e.frame.rotation_deg;
            }
        }
    }

    /// Une forme remplie d'une image (la façon de LibreOffice de poser une photo dans un cadre, arrondi ou non) :
    /// elle devient une photo, à sa place, avec le trait de la forme pour bordure et son ombre. Faux si la forme n'a
    /// pas d'image de remplissage lisible.
    fn filled_with_image(&mut self, n: Node) -> bool {
        let at = self.items.len();
        let placed = self.filled_photo(n);
        let g = n.attribute((DRAW, "style-name")).and_then(|s| self.styles.graphic.get(s));
        if let (true, Some(g), Some(Item::Element(e))) = (placed, g, self.items.get_mut(at))
            && let ElementKind::Photo(p) = &mut e.kind
        {
            p.border = g.stroke.clone();
            p.shadow = g.shadow;
        }
        placed
    }

    fn filled_photo(&mut self, n: Node) -> bool {
        let name = n.attribute((DRAW, "style-name")).and_then(|s| self.styles.graphic.get(s)).and_then(|g| g.fill_image.clone());
        let href = name.as_ref().and_then(|name| self.fill_images.get(name)).cloned();
        let (Some(href), Some(frame)) = (href, frame_of(n)) else { return false };
        let radius = if n.has_tag_name((DRAW, "rect")) { n.attribute((DRAW, "corner-radius")).and_then(length_mm).unwrap_or(0.0) } else { 0.0 };
        let Some(e) = self.fill_picture(frame, name.as_deref().unwrap_or(""), &href, radius) else { return false };
        self.items.push(Item::Element(Box::new(e)));
        true
    }

    /// La photo que donne l'image de remplissage `name` (rangée sous `href`) d'une forme ou d'un cadre de texte aux coins
    /// arrondis de `radius`, sans la poser. Une image posée tournée par l'export de Cahutin (`odg::turned_box_mm`, son
    /// nom finit par `odg::TURNED_FILL`) est redressée dans le cadre.
    fn fill_picture(&mut self, frame: Frame, name: &str, href: &str, radius: f64) -> Option<Element> {
        if frame.rotation_deg != 0.0 && name.ends_with(crate::odg::TURNED_FILL) {
            let upright = read_entry(self.zip, href).and_then(|b| image::load_from_memory(&b).ok()).and_then(|img| {
                let (bw, bh) = crate::odg::turned_box_mm(&frame, radius);
                let px_per_mm = (img.width() as f64 / bw).max(img.height() as f64 / bh);
                let out = crate::odg::turned_image(&img.to_rgba8(), (bw, bh), (frame.w_mm, frame.h_mm), px_per_mm, -frame.rotation_deg);
                crate::photo::encode(&image::DynamicImage::ImageRgba8(out), Mime::Png).ok()
            });
            return upright.and_then(|b| self.photo_element(frame, &format!("{href}{}", crate::odg::TURNED_FILL), radius, Some(b)));
        }
        self.photo_element(frame, href, radius, None)
    }

    /// Un tableau de LibreOffice (`table:table` dans un `draw:frame`) : un tableau de Cahutin à la place du cadre, ses
    /// cases des éléments du même groupe placés par `table::layout`. Colonnes aux largeurs écrites ; lignes aux hauteurs
    /// écrites, sauf quand le cadre ne vaut pas leur somme : LibreOffice écrit la hauteur demandée d'une ligne, pas
    /// celle qu'il lui donne (agrandie pour son texte), et le cadre à la hauteur rendue ; les lignes sont alors posées
    /// comme il les pose (voir `fit_rows`, contenu estimé) ; cases fusionnées ;
    /// texte, fond, marge et alignement vertical de chaque case ; un trait pour tout le tableau (voir `table_border`).
    /// Une image de fond non étirée d'une case vide devient le pictogramme ou la photo de la case ; une autre image de
    /// fond, un tableau dans une case et ce qui dépasse `table::MAX_LINES` sont comptés comme non repris. L'aperçu
    /// (`Pictures/TablePreviewN.svm`) n'est pas lu.
    fn table(&mut self, frame_node: Node, table: Node) {
        let own = |n: &Node| n.ancestors().skip(1).find(|a| a.has_tag_name((TABLE, "table"))) == Some(table);
        let count = |n: Node, attr: &str| n.attribute((TABLE, attr)).and_then(|v| v.parse::<usize>().ok()).unwrap_or(1).clamp(1, 1000);
        // Le cadre tourné (`draw:transform`) est remis droit : un tableau de Cahutin ne tourne pas.
        let at = frame_of(frame_node).map(|f| (f.x_mm, f.y_mm)).or_else(|| Some((attr_mm(frame_node, SVG, "x")?, attr_mm(frame_node, SVG, "y")?)));
        let Some((x, y)) = at else { return self.skipped += 1 };
        let mut cols: Vec<(Option<f64>, Option<&str>)> = Vec::new();
        for c in table.descendants().filter(|n| n.has_tag_name((TABLE, "table-column")) && own(n)) {
            let width = c.attribute((TABLE, "style-name")).and_then(|s| self.styles.columns.get(s)).copied();
            cols.extend(std::iter::repeat_n((width, c.attribute((TABLE, "default-cell-style-name"))), count(c, "number-columns-repeated")));
        }
        let rows: Vec<Node> = table.descendants().filter(|n| n.has_tag_name((TABLE, "table-row")) && own(n)).collect();
        let is_cell = |n: &Node| n.has_tag_name((TABLE, "table-cell")) || n.has_tag_name((TABLE, "covered-table-cell"));
        let widest = rows.iter().map(|r| r.children().filter(is_cell).map(|c| count(c, "number-columns-repeated")).sum::<usize>()).max().unwrap_or(0);
        let max = crate::table::MAX_LINES as usize;
        let (ncols, nrows) = (cols.len().max(widest).min(max), rows.len().min(max));
        if ncols == 0 || nrows == 0 {
            return;
        }
        let mut truncated = cols.len().max(widest) > max || rows.len() > max;
        // Une colonne sans largeur lisible : la part du cadre qui reste.
        let known: f64 = cols.iter().take(ncols).filter_map(|c| c.0).sum();
        let missing = (0..ncols).filter(|&i| cols.get(i).and_then(|c| c.0).is_none()).count();
        let frame_w = attr_mm(frame_node, SVG, "width").unwrap_or(0.0);
        let spare = if missing > 0 { ((frame_w - known) / missing as f64).max(5.0) } else { 0.0 };
        let widths: Vec<f64> = (0..ncols).map(|i| cols.get(i).and_then(|c| c.0).filter(|w| *w > 0.0).unwrap_or(spare)).collect();
        let row_h = attr_mm(frame_node, SVG, "height").unwrap_or(0.0) / nrows as f64;
        // Hauteur écrite de chaque ligne ; une ligne sans hauteur (ou de hauteur nulle, comme l'écrit LibreOffice pour une
        // ligne ajoutée) prend en attendant une part du cadre, puis sa hauteur rendue (voir plus bas, `fit_rows`).
        let row_written: Vec<Option<f64>> = rows.iter().take(nrows).map(|r| r.attribute((TABLE, "style-name")).and_then(|s| self.styles.rows.get(s)).copied().filter(|h| *h > 0.0)).collect();
        let heights: Vec<f64> = row_written.iter().map(|h| h.unwrap_or(row_h.max(5.0))).collect();
        // Les cases écrites, à leur place dans la grille ; une case couverte par une fusion n'est qu'une place.
        let mut taken = vec![vec![false; ncols]; nrows];
        let mut cells: Vec<(GridCell, Option<(Node, Option<&str>)>)> = Vec::new();
        for (r, row) in rows.iter().take(nrows).enumerate() {
            let mut c = 0;
            for n in row.children().filter(is_cell) {
                for _ in 0..count(n, "number-columns-repeated") {
                    if c >= ncols {
                        truncated |= n.has_tag_name((TABLE, "table-cell"));
                        break;
                    }
                    if n.has_tag_name((TABLE, "table-cell")) && !taken[r][c] {
                        let mut cs = count(n, "number-columns-spanned").min(ncols - c);
                        let mut rs = count(n, "number-rows-spanned").min(nrows - r);
                        if (r..r + rs).any(|rr| (c..c + cs).any(|cc| taken[rr][cc])) {
                            (cs, rs) = (1, 1);
                        }
                        for line in taken.iter_mut().skip(r).take(rs) {
                            line[c..c + cs].fill(true);
                        }
                        let style = n.attribute((TABLE, "style-name")).or(row.attribute((TABLE, "default-cell-style-name"))).or(cols.get(c).and_then(|k| k.1));
                        cells.push((GridCell { element_id: String::new(), col: c as u8, row: r as u8, col_span: cs as u8, row_span: rs as u8 }, Some((n, style))));
                    }
                    c += 1;
                }
            }
        }
        // Une place qu'aucune case ne couvre (fichier abîmé) : une case vide.
        for (r, line) in taken.iter().enumerate() {
            for (c, _) in line.iter().enumerate().filter(|(_, t)| !**t) {
                cells.push((GridCell { element_id: String::new(), col: c as u8, row: r as u8, col_span: 1, row_span: 1 }, None));
            }
        }
        cells.sort_by_key(|(g, _)| (g.row, g.col));
        let xs: Vec<f64> = std::iter::once(x).chain(widths.iter().scan(x, |a, w| { *a += w; Some(*a) })).collect();
        let ys: Vec<f64> = std::iter::once(y).chain(heights.iter().scan(y, |a, h| { *a += h; Some(*a) })).collect();
        let group = new_id();
        let mut sides: Vec<Option<CellBorder>> = Vec::new();
        let mut elements: Vec<Element> = Vec::new();
        let mut blank = Vec::new();
        // Ce que demande le contenu de chaque case (marges et lignes de texte estimées) : (ligne, lignes couvertes, mm).
        let mut needs: Vec<(usize, usize, f64)> = Vec::new();
        for (g, node) in &mut cells {
            let (c, r, cs, rs) = (g.col as usize, g.row as usize, g.col_span as usize, g.row_span as usize);
            let rect = Frame { x_mm: xs[c], y_mm: ys[r], w_mm: xs[c + cs] - xs[c], h_mm: ys[r + rs] - ys[r], rotation_deg: 0.0 };
            let style = self.styles.cell(node.and_then(|n| n.1));
            if node.is_some() {
                sides.extend(style.borders.iter().map(|b| b.clone().flatten()));
            }
            let fallback = style.text.clone().unwrap_or_default();
            let (paragraphs, outline) = match node {
                Some((n, _)) => {
                    // Un tableau dans une case (que LibreOffice Draw ne fait pas) : son texte manquera.
                    if n.descendants().skip(1).any(|d| d.has_tag_name((TABLE, "table"))) {
                        self.skipped += 1;
                    }
                    let cell = *n;
                    let ps = n.descendants().filter(|p| p.has_tag_name((TEXT, "p")) && p.ancestors().find(|a| a.has_tag_name((TABLE, "table-cell"))) == Some(cell));
                    self.paragraphs(ps, &fallback, true, &[])
                }
                None => (Vec::new(), false),
            };
            let empty = paragraphs.iter().all(|p| p.runs.iter().all(|r| r.text.trim().is_empty()));
            let first = paragraphs.first().map(|p| ReadParagraph { align: p.align, runs: Vec::new(), first: p.first.clone(), bullet: false, line_spacing: p.line_spacing, on_lines: false });
            let first = first.unwrap_or(ReadParagraph { align: None, runs: Vec::new(), first: Run { font: fallback.font.clone(), size_pt: fallback.size_pt, color: fallback.color.clone(), ..Run::plain("") }, bullet: false, line_spacing: None, on_lines: false });
            let text_style = text_style(&first, outline, style.valign, style.padding_mm);
            // Une case sans paragraphe a la hauteur d'une ligne vide (LibreOffice : celle de sa police).
            let line_mm = |size_pt: f64| size_pt * CELL_LINE_EM * 25.4 / 72.0;
            let lines: f64 = if paragraphs.is_empty() {
                line_mm(text_style.size_pt)
            } else {
                paragraphs
                    .iter()
                    .map(|p| {
                        let size = p.runs.iter().filter_map(|r| r.size_pt).fold(p.first.size_pt.unwrap_or(text_style.size_pt), f64::max);
                        (1 + p.runs.iter().map(|r| r.text.matches('\n').count()).sum::<usize>()) as f64 * line_mm(size)
                    })
                    .sum()
            };
            needs.push((r, rs, style.padding_v_mm.unwrap_or(0.0) + lines));
            let content = if empty { RichText::default() } else { rich_text(paragraphs, &text_style) };
            let fill = style.fill.clone().flatten();
            let frame = fill.map(|f| TextFrame { fill: Some(f), stroke: None, corner_radius_mm: 0.0, image: None, pattern: None, write_on_lines: None });
            let mut kind = ElementKind::Text(TextProps { content, style: text_style, prompt: None, frame });
            let (mut flip_h, mut flip_v) = (false, false);
            if let Some(Some((name, repeat))) = &style.fill_image {
                let href = self.fill_images.get(name).cloned();
                let picture = match href {
                    Some(href) if empty && repeat == "no-repeat" => match openmoji(&href).or_else(|| openmoji_cell_fill(name)) {
                        Some((picto, h, v)) => {
                            (flip_h, flip_v) = (h, v);
                            Some(ElementKind::Image(picto))
                        }
                        None => self.photo_element(rect, &href, 0.0, None).map(|e| e.kind),
                    },
                    _ => None,
                };
                match picture {
                    Some(k) => kind = k,
                    None => self.skipped += 1,
                }
            }
            if empty && matches!(kind, ElementKind::Text(_)) {
                // Une case vide de Cahutin porte une espace sans largeur à sa police et à sa taille : elle les garde,
                // sauf en toute petite taille (une ligne trop basse pour elles).
                let marked = node.is_some_and(|(n, _)| n.descendants().any(|d| d.text().is_some_and(|t| t.contains('\u{200B}'))));
                let keeps = marked && matches!(&kind, ElementKind::Text(t) if t.style.size_pt >= MIN_CELL_TEXT_PT);
                blank.push((elements.len(), keeps));
            }
            let e = Element { group: Some(group.clone()), flip_h, flip_v, ..element(rect, kind) };
            g.element_id = e.id.clone();
            elements.push(e);
        }
        // Une autre case vide prend la police, la taille et la couleur de la première case écrite : le texte qu'on y
        // tapera ressemble aux autres, et non à la toute petite taille qui tient basse une ligne vide.
        let written = elements.iter().enumerate().find_map(|(i, e)| match &e.kind {
            ElementKind::Text(t) if !blank.iter().any(|b| b.0 == i) => Some(t.style.clone()),
            _ => None,
        });
        for &(i, _) in blank.iter().filter(|b| !b.1) {
            if let ElementKind::Text(t) = &mut elements[i].kind {
                match &written {
                    Some(w) => (t.style.font, t.style.size_pt, t.style.color) = (w.font.clone(), w.size_pt, w.color.clone()),
                    None if t.style.size_pt < MIN_CELL_TEXT_PT => t.style.size_pt = TextStyle::default_zone().size_pt,
                    None => {}
                }
            }
        }
        if truncated {
            self.skipped += 1;
        }
        let mut heights = heights;
        let missing = row_written.iter().any(Option::is_none);
        let written_sum: f64 = row_written.iter().flatten().sum();
        // LibreOffice écrit souvent le cadre au 1/100 mm près de la somme (11,999 cm pour 12 cm) : ce n'est pas un désaccord.
        if let Some(frame_h) = attr_mm(frame_node, SVG, "height").filter(|h| (missing || (h - written_sum).abs() > 0.05) && !truncated) {
            let heights_written: Vec<f64> = row_written.iter().map(|h| h.unwrap_or(0.0)).collect();
            let mut mins = vec![0.0; nrows];
            for &(r, _, need) in needs.iter().filter(|n| n.1 == 1) {
                mins[r] = f64::max(mins[r], need);
            }
            // Une case fusionnée sur plusieurs lignes : ce qui manque aux premières, à la dernière.
            for &(r, rs, need) in needs.iter().filter(|n| n.1 > 1) {
                let last = r + rs - 1;
                mins[last] = f64::max(mins[last], need - heights_written[r..last].iter().sum::<f64>());
            }
            heights = fit_rows(&heights_written, &mins, frame_h);
        }
        let ys: Vec<f64> = std::iter::once(y).chain(heights.iter().scan(y, |a, h| { *a += h; Some(*a) })).collect();
        let frame = Frame { x_mm: x, y_mm: y, w_mm: xs[ncols] - x, h_mm: ys[nrows] - y, rotation_deg: 0.0 };
        let props = TableProps { cols: widths, rows: heights, cells: cells.into_iter().map(|(g, _)| g).collect(), border: table_border(&sides) };
        let table = Element { group: Some(group), ..element(frame, ElementKind::Table(props)) };
        let id = table.id.clone();
        // Les cases, puis le tableau par-dessus (ses traits ne sont pas cachés par leur fond), comme `table::add_table`.
        let mut page = Page::new(A4_PORTRAIT);
        page.elements = elements;
        page.elements.push(table);
        crate::table::layout(&mut page, &id);
        self.items.extend(page.elements.into_iter().map(|e| Item::Element(Box::new(e))));
    }

    /// Un objet de la page ; protégé dans LibreOffice (`style:protect` de son style), ce qu'il donne est verrouillé.
    /// Un vrai tableau protégé : le tableau verrouillé, pas ses cases (comme l'export, qui ne protège que le tableau).
    fn node(&mut self, n: Node) {
        let start = self.items.len();
        self.node_kind(n);
        if self.whole_frames && self.items.len() == start + 1 {
            self.shape_text(n);
        }
        // Un cadre à lignes d'écriture a repris les objets posés avant lui (`lined_frame`) : il est le dernier.
        let start = if self.items.len() < start { self.items.len() - 1 } else { start };
        if n.attribute((DRAW, "style-name")).and_then(|s| self.styles.graphic.get(s)).is_some_and(|g| g.protect) {
            let table = n.children().any(|c| c.has_tag_name((TABLE, "table")));
            for it in &mut self.items[start..] {
                if let Item::Element(e) = it
                    && (!table || matches!(e.kind, ElementKind::Table(_)))
                {
                    e.locked = true;
                }
            }
        }
    }

    fn node_kind(&mut self, n: Node) {
        if !n.is_element() {
            return;
        }
        if n.has_tag_name((DRAW, "g")) {
            let start = self.items.len();
            let floor = std::mem::replace(&mut self.group_floor, start);
            for c in n.children() {
                self.node(c);
            }
            self.group_floor = floor;
            if self.whole_frames {
                self.group_from(start);
            }
            return;
        }
        let tag = n.tag_name();
        if tag.namespace() != Some(DRAW) {
            return;
        }
        if matches!(tag.name(), "rect" | "polygon" | "path" | "custom-shape" | "ellipse" | "circle") && self.filled_with_image(n) {
            return;
        }
        match tag.name() {
            "frame" => {
                if let Some(t) = n.children().find(|c| c.has_tag_name((TABLE, "table"))) {
                    return self.table(n, t);
                }
                let Some(frame) = frame_of(n) else { return self.skipped += 1 };
                if let Some(tb) = n.children().find(|c| c.has_tag_name((DRAW, "text-box"))) {
                    self.text_box(frame, n, tb);
                } else if n.children().any(|c| c.has_tag_name((DRAW, "image"))) {
                    self.image(frame, n.children().filter(|c| c.has_tag_name((DRAW, "image"))).collect());
                } else if n.children().any(|c| c.is_element()) {
                    // Un objet (graphique, formule) : il manquera.
                    self.skipped += 1;
                }
                // Un cadre vide ne montre rien : rien ne manque.
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
                    Some(d) => self.drawings.push(d.with_dash(g.dash)),
                    None => self.skipped += 1,
                }
            }
            "custom-shape" => match (frame_of(n), custom_shape_kind(n), free_path(n)) {
                (Some(f), Some(kind), _) => {
                    self.shape(n, f, kind);
                    self.mirror_last_shape(n);
                }
                (Some(f), None, Some(path)) => {
                    self.shape(n, f, ShapeKind::Path);
                    self.mirror_last_shape(n);
                    if let Some(Item::Element(e)) = self.items.last_mut()
                        && let ElementKind::Shape(s) = &mut e.kind
                    {
                        s.path = Some(path);
                        // Un tracé ouvert (ligne de graphisme, chemin) n'a pas de remplissage.
                        if !s.path.as_deref().is_some_and(|p| p.trim_end().ends_with('Z')) {
                            s.fill = None;
                        }
                    }
                }
                // Une forme toute faite de LibreOffice que Cahutin ne sait pas redessiner (smiley, soleil, lune...).
                _ => {
                    self.skipped += 1;
                    self.ready_made += 1;
                }
            },
            "line" => {
                let (Some(x1), Some(y1), Some(x2), Some(y2)) = (attr_mm(n, SVG, "x1"), attr_mm(n, SVG, "y1"), attr_mm(n, SVG, "x2"), attr_mm(n, SVG, "y2")) else {
                    return self.skipped += 1;
                };
                // Trait Carnet : du milieu du bord gauche au milieu du bord droit du cadre, tourné. Une pointe au bout
                // (`draw:marker-end`, ce qu'écrit l'export) en fait une flèche, haute de sa pointe ; une pointe au
                // début seulement (dessinée dans LibreOffice) aussi, le trait pris dans l'autre sens. Une flèche
                // retournée revient tournée d'un demi-tour : à l'écran, c'est la même.
                let g = n.attribute((DRAW, "style-name")).and_then(|s| self.styles.graphic.get(s));
                let start_only = g.is_some_and(|g| g.marker_end.is_none() && g.marker_start);
                let ((x1, y1), (x2, y2)) = if start_only { ((x2, y2), (x1, y1)) } else { ((x1, y1), (x2, y2)) };
                let head = g.and_then(|g| g.marker_end).or(start_only.then_some(4.0));
                let len = (x2 - x1).hypot(y2 - y1);
                let (cx, cy, h) = ((x1 + x2) / 2.0, (y1 + y2) / 2.0, head.unwrap_or(2.0).max(2.0));
                let rotation_deg = (y2 - y1).atan2(x2 - x1).to_degrees();
                self.shape(n, Frame { x_mm: cx - len / 2.0, y_mm: cy - h / 2.0, w_mm: len, h_mm: h, rotation_deg }, if head.is_some() { ShapeKind::Arrow } else { ShapeKind::Line });
            }
            // Une flèche de liaison (`draw:connector`), une cote, une légende... : des objets de LibreOffice que Cahutin
            // ne sait pas redessiner.
            _ => {
                self.skipped += 1;
                self.ready_made += 1;
            }
        }
    }

    /// Le texte tapé dans une forme dans LibreOffice (ses `text:p`, que l'export de Cahutin n'écrit jamais) : un texte
    /// posé sur la forme, au même cadre, centré comme LibreOffice le pose (sauf réglage de la forme), groupé avec elle.
    /// Une forme sans texte (LibreOffice lui ajoute un paragraphe vide en réenregistrant) reste seule.
    fn shape_text(&mut self, n: Node) {
        let shaped = matches!(n.tag_name().name(), "rect" | "ellipse" | "circle" | "custom-shape" | "polygon") && n.tag_name().namespace() == Some(DRAW);
        let Some(Item::Element(shape)) = self.items.last() else { return };
        if !shaped || !matches!(shape.kind, ElementKind::Shape(_) | ElementKind::Photo(_)) {
            return;
        }
        let (frame, group) = (shape.frame, crate::groups::path(shape));
        // Ce que le paragraphe ne dit pas vient du style de la forme et de ses parents ; la couleur automatique, du fond.
        let g = n.attribute((DRAW, "style-name")).and_then(|s| self.styles.graphic.get(s)).cloned().unwrap_or_default();
        let mut fallback = g.text.clone();
        fallback.color = fallback.color.or_else(|| Some(auto_text_color(g.fill.as_deref()).into()));
        let (paragraphs, outline) = self.paragraphs(n.children().filter(|c| c.has_tag_name((TEXT, "p"))), &fallback, false, &[]);
        if paragraphs.iter().all(|p| p.runs.iter().all(|r| r.text.trim().is_empty())) {
            return;
        }
        let first = &paragraphs[0];
        let mut style = text_style(first, outline, Some(g.valign.unwrap_or(VAlign::Middle)), g.padding_mm);
        if first.align.is_none() {
            style.align = Align::Center;
        }
        let content = rich_text(paragraphs, &style);
        let id = new_id();
        let mut text = element(frame, ElementKind::Text(TextProps { content, style, prompt: None, frame: None }));
        crate::groups::set_path(&mut text, [group.clone(), vec![id.clone()]].concat());
        if let Some(Item::Element(shape)) = self.items.last_mut() {
            crate::groups::set_path(shape, [group, vec![id]].concat());
        }
        self.items.push(Item::Element(Box::new(text)));
    }

    /// Un groupe LibreOffice (`draw:g`) lu dans un dessin : les éléments posés depuis `start` en deviennent un groupe,
    /// autour des groupes qu'ils forment déjà (un groupe dans un groupe). Un tableau et ses cases gardent leur seul
    /// groupe, comme dans Cahutin, où un tableau n'entre dans aucun groupe. Les tracés redessinés au fond de la page ne
    /// sont pas des éléments posés ici : ils quittent le groupe. Un groupe d'un seul membre ou vide est défait à la fin
    /// de la page (`groups::dissolve_small_groups`).
    fn group_from(&mut self, start: usize) {
        let tables: Vec<Id> = self.items[start..]
            .iter()
            .filter_map(|it| match it {
                Item::Element(e) if matches!(e.kind, ElementKind::Table(_)) => e.group.clone(),
                _ => None,
            })
            .collect();
        let id = new_id();
        for it in &mut self.items[start..] {
            if let Item::Element(e) = it
                && !e.group.as_ref().is_some_and(|g| tables.contains(g))
            {
                let mut path = crate::groups::path(e);
                path.insert(0, id.clone());
                crate::groups::set_path(e, path);
            }
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

    fn finish(mut self) -> (Vec<Element>, Vec<Asset>, usize, usize, Vec<Drawing>, usize, Vec<(String, Vec<u8>, f64)>) {
        self.attach_bullets();
        attach_underlines(&mut self.lines, &mut self.items);
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
        (elements, self.assets, self.skipped, self.ready_made, self.drawings, self.shrunk, self.pending)
    }
}

/// Plus grande épaisseur d'un trait de soulignement.
const UNDERLINE_MAX_MM: f64 = 0.6;

/// Un soulignement, dans un PDF, est un trait à part, juste sous sa ligne de texte : il devient le soulignement
/// des mots qu'il couvre, et le trait disparaît. La place de chaque lettre est estimée d'après leur nombre (une
/// police de l'ordinateur n'est pas mesurable ici) ; un mot est souligné si son milieu est sous le trait.
fn attach_underlines(lines: &mut [Line], items: &mut Vec<Item>) {
    items.retain(|it| {
        let Item::Element(e) = it else { return true };
        let ElementKind::Shape(s) = &e.kind else { return true };
        let f = &e.frame;
        let horizontal = f.rotation_deg.abs() < 0.5 || (f.rotation_deg.abs() - 180.0).abs() < 0.5;
        let thin = s.stroke.as_ref().is_some_and(|st| st.width_mm <= UNDERLINE_MAX_MM);
        if s.shape != ShapeKind::Line || !horizontal || !thin || f.w_mm < 1.0 {
            return true;
        }
        let (x0, x1, y) = (f.x_mm, f.x_mm + f.w_mm, f.y_mm + f.h_mm / 2.0);
        let under = lines.iter_mut().find(|l| x0 >= l.x_mm - 1.0 && x1 <= l.x_mm + l.w_mm + 1.0 && y >= l.y_mm + l.h_mm * 0.5 && y <= l.y_mm + l.h_mm + 1.0);
        let Some(line) = under else { return true };
        underline_span(line, x0, x1);
        false
    });
}

/// Deux morceaux de texte de même allure (hors soulignement) : ils se recollent.
fn same_look(a: &Run, b: &Run) -> bool {
    a.bold == b.bold && a.italic == b.italic && a.color == b.color && a.font == b.font && a.size_pt == b.size_pt
}

/// Souligne les mots de `line` dont le milieu tombe entre `x0` et `x1`.
fn underline_span(line: &mut Line, x0: f64, x1: f64) {
    let chars: usize = line.runs.iter().map(|r| r.text.chars().count()).sum();
    if chars == 0 {
        return;
    }
    let step = line.w_mm / chars as f64;
    let mut at = 0usize;
    let mut out: Vec<Run> = Vec::new();
    for run in std::mem::take(&mut line.runs) {
        // Mots et espaces, chacun à sa place sur la ligne.
        let mut pieces: Vec<(String, bool)> = Vec::new();
        let mut word = String::new();
        let flush = |word: &mut String, at: &mut usize, pieces: &mut Vec<(String, bool)>| {
            if word.is_empty() {
                return;
            }
            let n = word.chars().count();
            let mid = line.x_mm + (*at as f64 + n as f64 / 2.0) * step;
            let is_space = word.chars().all(char::is_whitespace);
            pieces.push((std::mem::take(word), !is_space && mid >= x0 && mid <= x1));
            *at += n;
        };
        for c in run.text.chars() {
            if c.is_whitespace() != word.chars().last().is_some_and(char::is_whitespace) && !word.is_empty() {
                flush(&mut word, &mut at, &mut pieces);
            }
            word.push(c);
        }
        flush(&mut word, &mut at, &mut pieces);
        // Une espace entre deux mots soulignés l'est aussi : le trait reste continu.
        for i in 1..pieces.len().saturating_sub(1) {
            if !pieces[i].1 && pieces[i].0.trim().is_empty() && pieces[i - 1].1 && pieces[i + 1].1 {
                pieces[i].1 = true;
            }
        }
        for (text, under) in pieces {
            let underline = run.underline || under;
            match out.last_mut() {
                Some(last) if last.underline == underline && same_look(last, &run) => last.text.push_str(&text),
                _ => out.push(Run { text, underline, ..run.clone() }),
            }
        }
    }
    line.runs = out;
}

/// Repère d'une image en attente de décodage, suivi de son rang dans la page (voir `PageReader::photo`).
const PENDING: &str = "\u{0}en-attente-";
/// Fils pour décoder les images d'une page. Une grande photo (4000 × 3000) prend près de 150 Mo le temps d'être
/// décodée et réduite : deux à la fois divisent le temps par deux (84 photos : 13 s au lieu de 24) sans dépasser
/// 600 Mo, ce que supporte un ordinateur d'école.
const DECODE_THREADS: usize = 2;

/// Décode ensemble les images d'une page (plusieurs fils, une page à la fois pour ne pas tout garder en mémoire),
/// puis donne aux photos leur image ; une photo dont l'image ne se décode pas est retirée (une case de tableau reste,
/// photo vide). Rend le nombre de photos retirées ou vidées.
fn decode_page(elements: &mut Vec<Element>, pending: Vec<(String, Vec<u8>, f64)>, known: &mut HashMap<String, String>, assets: &mut Vec<Asset>) -> usize {
    if pending.is_empty() {
        return 0;
    }
    let threads = std::thread::available_parallelism().map_or(1, |n| n.get()).min(DECODE_THREADS);
    // Chaque image tournée dans le sens de son cadre (voir `photo::import_bytes_many`).
    let (hrefs, blobs): (Vec<String>, Vec<(Vec<u8>, Option<f64>)>) = pending.into_iter().map(|(h, b, aspect)| (h, (b, Some(aspect)))).unzip();
    let ids: Vec<Option<String>> = import_bytes_many(&blobs, threads)
        .into_iter()
        .zip(hrefs)
        .map(|(r, href)| {
            let a = r.ok()?;
            let id = a.meta.id.clone();
            known.insert(href, id.clone());
            assets.push(a);
            Some(id)
        })
        .collect();
    let cells: Vec<String> = elements.iter().filter_map(|e| if let ElementKind::Table(t) = &e.kind { Some(t.cells.iter().map(|c| c.element_id.clone())) } else { None }).flatten().collect();
    let mut lost = 0;
    let decoded = |asset: &str| asset.strip_prefix(PENDING).and_then(|i| i.parse::<usize>().ok()).map(|at| ids.get(at).cloned().flatten());
    for e in elements.iter_mut() {
        if let ElementKind::Text(t) = &mut e.kind
            && let Some(f) = &mut t.frame
            && let Some(id) = f.image.as_deref().and_then(decoded)
        {
            lost += usize::from(id.is_none());
            f.image = id;
        }
    }
    elements.retain_mut(|e| {
        let ElementKind::Photo(p) = &mut e.kind else { return true };
        let Some(at) = p.asset.as_deref().and_then(|a| a.strip_prefix(PENDING)).and_then(|i| i.parse::<usize>().ok()) else { return true };
        p.asset = ids.get(at).cloned().flatten();
        lost += usize::from(p.asset.is_none());
        p.asset.is_some() || cells.contains(&e.id)
    });
    lost
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

/// Vrai si le fichier porte la marque de Cahutin dans ses propriétés (`odg::CAHUTIN_MARK`), même réenregistré par
/// LibreOffice, qui réécrit le générateur mais garde les propriétés.
pub fn from_cahutin(input: impl Read + Seek) -> bool {
    let Ok(mut zip) = ZipArchive::new(input) else { return false };
    let Some(meta) = read_entry(&mut zip, "meta.xml").and_then(|b| String::from_utf8(b).ok()) else { return false };
    let Ok(meta) = roxmltree::Document::parse(&meta) else { return false };
    meta.descendants().any(|n| n.has_tag_name((META, "user-defined")) && n.attribute((META, "name")) == Some(crate::odg::CAHUTIN_MARK_NAME) && n.text().map(str::trim) == Some(crate::odg::CAHUTIN_MARK))
}

fn read(input: impl Read + Seek, known_fonts: &[String], whole_frames: bool) -> Result<Imported, ImportError> {
    let mut zip = ZipArchive::new(input).map_err(|_| ImportError::Unreadable)?;
    let content = read_entry(&mut zip, "content.xml").and_then(|b| String::from_utf8(b).ok()).ok_or(ImportError::Unreadable)?;
    let styles_xml = read_entry(&mut zip, "styles.xml").and_then(|b| String::from_utf8(b).ok());
    let doc = roxmltree::Document::parse(&content).map_err(|_| ImportError::Unreadable)?;
    let styles_doc = styles_xml.as_deref().and_then(|x| roxmltree::Document::parse(x).ok());
    let docs = [Some(&doc), styles_doc.as_ref()].into_iter().flatten().collect::<Vec<_>>();
    let dashes = stroke_dashes(&docs);
    let fills = fill_images(&docs);
    let mut styles = read_styles(&doc, styles_doc.as_ref(), known_fonts, &dashes, &dash_styles(&docs));
    styles.bullet_lists = bullet_lists(&docs);
    (styles.cells, styles.columns, styles.rows) = table_styles(&docs, known_fonts);
    let sizes = page_sizes(styles_xml.as_deref());
    let page_fills = drawing_page_fills(&doc);
    let generator = read_entry(&mut zip, "meta.xml").and_then(|b| String::from_utf8(b).ok()).and_then(|x| {
        let meta = roxmltree::Document::parse(&x).ok()?;
        meta.descendants().find(|n| n.has_tag_name((META, "generator"))).and_then(|n| n.text()).map(str::to_string)
    });
    let screen_mirror_angles = generator.is_some_and(|g| g.starts_with("cahutin/") && !g.contains(crate::odg::MIRROR_AFTER_ROTATION));
    let mut pages = Vec::new();
    let mut assets = Vec::new();
    let (mut skipped, mut ready_made) = (0, 0);
    let (mut drawn, mut drawn_pages, mut shrunk) = (0, 0, 0);
    // Images déjà décodées, par chemin dans le fichier : une image reprise sur plusieurs pages ne l'est qu'une fois.
    let mut known: HashMap<String, String> = HashMap::new();
    for page_node in doc.descendants().filter(|n| n.has_tag_name((DRAW, "page"))) {
        let mut reader = PageReader { styles: &styles, fill_images: &fills, known: &known, pending: Vec::new(), zip: &mut zip, items: Vec::new(), lines: Vec::new(), assets: Vec::new(), skipped: 0, ready_made: 0, drawings: Vec::new(), shrunk: 0, whole_frames, screen_mirror_angles, group_floor: 0 };
        for c in page_node.children() {
            reader.node(c);
        }
        let (mut elements, page_assets, page_skipped, page_ready_made, drawings, page_shrunk, pending) = reader.finish();
        ready_made += page_ready_made;
        shrunk += page_shrunk;
        skipped += decode_page(&mut elements, pending, &mut known, &mut assets);
        let size = page_node.attribute((DRAW, "master-page-name")).and_then(|m| sizes.by_master.get(m)).copied().unwrap_or(sizes.default);
        let mut background = page_node.attribute((DRAW, "master-page-name")).and_then(|m| sizes.background.get(m)).cloned();
        // Un fond donné à la page elle-même dans LibreOffice (son style, dans `content.xml`) l'emporte sur celui de la
        // page maîtresse.
        if let Some(color) = page_node.attribute((DRAW, "style-name")).and_then(|n| page_fills.get(n)) {
            background = Some(Background { color: color.clone(), pattern: background.and_then(|b| b.pattern) });
        }
        let mut page = Page { elements, background, ..Page::new(size) };
        super::tidy::full_page_rect_to_background(&mut page);
        super::tidy::group_fragments(&mut page);
        if whole_frames {
            crate::groups::dissolve_small_groups(&mut page);
        }
        // Les tracés au fond de la page, sous les textes et les photos. Peu nombreux (une fiche de graphisme) : des
        // formes modifiables ; nombreux (une illustration) : une image fixe, verrouillée, plus fidèle.
        let editable = (!drawings.is_empty() && drawings.len() <= EDITABLE_TRACES).then(|| drawings.iter().map(Drawing::to_shape).collect::<Option<Vec<_>>>()).flatten();
        if let Some(shapes) = editable {
            for (i, (frame, props)) in shapes.into_iter().enumerate() {
                page.elements.insert(i, element(frame, ElementKind::Shape(props)));
            }
        } else {
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
        }
        pages.push(page);
        assets.extend(page_assets);
        skipped += page_skipped;
    }
    let counted = ImportReport::count(&pages);
    let report = ImportReport { skipped, ready_made, drawn, shrunk, images: counted.images - drawn_pages, ..counted };
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
        odg_with_styles(pages, layouts, masters, pictures, "", "")
    }

    /// Comme `odg_raw`, avec des styles en plus : `auto` dans les styles automatiques de `content.xml`, `common` dans
    /// les styles de `styles.xml`.
    fn odg_with_styles(pages: &str, layouts: &str, masters: &str, pictures: &[(&str, Vec<u8>)], auto: &str, common: &str) -> Vec<u8> {
        let content = format!(
            r##"<?xml version="1.0" encoding="UTF-8"?>
<office:document-content xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0" xmlns:style="urn:oasis:names:tc:opendocument:xmlns:style:1.0" xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0" xmlns:draw="urn:oasis:names:tc:opendocument:xmlns:drawing:1.0" xmlns:fo="urn:oasis:names:tc:opendocument:xmlns:xsl-fo-compatible:1.0" xmlns:svg="urn:oasis:names:tc:opendocument:xmlns:svg-compatible:1.0" xmlns:xlink="http://www.w3.org/1999/xlink" xmlns:loext="urn:org:documentfoundation:names:experimental:office:xmlns:loext:1.0" xmlns:table="urn:oasis:names:tc:opendocument:xmlns:table:1.0" office:version="1.3">
<office:font-face-decls>
<style:font-face style:name="Liberation Serif" svg:font-family="&apos;Liberation Serif&apos;"/>
<style:font-face style:name="Liberation Sans" svg:font-family="&apos;Liberation Sans&apos;" style:font-family-generic="roman"/>
<style:font-face style:name="Comic Inconnue" svg:font-family="&apos;Comic Inconnue&apos;"/>
<style:font-face style:name="Helvetica" svg:font-family="Helvetica"/>
</office:font-face-decls>
<office:automatic-styles>
<style:style style:name="gr1" style:family="graphic"><style:graphic-properties draw:stroke="solid" svg:stroke-width="0cm" svg:stroke-color="#3465a4" draw:fill="solid" draw:fill-color="#729fcf"/></style:style>
<style:style style:name="gr2" style:family="graphic"><style:graphic-properties draw:stroke="none" draw:fill="none"/></style:style>
<style:style style:name="grPhoto" style:family="graphic"><style:graphic-properties draw:stroke="none" draw:fill="bitmap" draw:fill-image-name="Bitmap_5f_1" style:repeat="stretch"/></style:style>
<style:style style:name="gr3" style:family="graphic"><style:graphic-properties draw:stroke="none" draw:fill="solid" draw:fill-color="#ffffff"/></style:style>
<style:style style:name="T1" style:family="text"><style:text-properties fo:color="#000000" style:font-name="Liberation Serif" fo:font-size="12pt" fo:font-weight="normal"/></style:style>
<style:style style:name="T2" style:family="text"><style:text-properties fo:color="#c0392b" style:font-name="Liberation Sans" fo:font-size="24pt" fo:font-weight="bold"/></style:style>
<style:style style:name="T3" style:family="text"><style:text-properties fo:color="#000000" style:font-name="Liberation Serif" fo:font-size="12pt" fo:font-weight="bold"/></style:style>
<style:style style:name="T4" style:family="text"><style:text-properties fo:color="#000000" style:font-name="Comic Inconnue" fo:font-size="12pt" fo:font-style="italic" style:text-underline-style="solid"/></style:style>
<style:style style:name="T5" style:family="text"><style:text-properties fo:color="#000000" style:font-name="Helvetica" fo:font-size="12pt"/></style:style>
<text:list-style style:name="LB"><text:list-level-style-bullet text:level="1" text:bullet-char="●"><style:list-level-properties text:min-label-width="0.6cm"/><style:text-properties fo:font-family="OpenSymbol" style:use-window-font-color="true" fo:font-size="45%"/></text:list-level-style-bullet><text:list-level-style-bullet text:level="2" text:bullet-char="●"><style:list-level-properties text:space-before="0.6cm" text:min-label-width="0.6cm"/><style:text-properties fo:font-family="OpenSymbol" style:use-window-font-color="true" fo:font-size="45%"/></text:list-level-style-bullet></text:list-style>
<text:list-style style:name="LN"><text:list-level-style-number text:level="1" style:num-suffix="." style:num-format="1"><style:list-level-properties text:min-label-width="0.6cm"/></text:list-level-style-number></text:list-style>
{auto}</office:automatic-styles>
<office:body><office:drawing>{pages}</office:drawing></office:body></office:document-content>"##
        );
        let styles = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document-styles xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0" xmlns:style="urn:oasis:names:tc:opendocument:xmlns:style:1.0" xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0" xmlns:draw="urn:oasis:names:tc:opendocument:xmlns:drawing:1.0" xmlns:fo="urn:oasis:names:tc:opendocument:xmlns:xsl-fo-compatible:1.0" xmlns:xlink="http://www.w3.org/1999/xlink" xmlns:svg="urn:oasis:names:tc:opendocument:xmlns:svg-compatible:1.0" xmlns:loext="urn:org:documentfoundation:names:experimental:office:xmlns:loext:1.0" xmlns:table="urn:oasis:names:tc:opendocument:xmlns:table:1.0">
<office:styles><draw:fill-image draw:name="Bitmap_5f_1" xlink:href="Pictures/remplissage.png" xlink:type="simple"/><text:list-style style:name="LS"><text:list-level-style-bullet text:level="1" text:bullet-char="◦"/></text:list-style>{common}</office:styles>
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

    fn read_whole(body: &str) -> Imported {
        read_drawing(Cursor::new(odg_fixture(body, A4_LAYOUT, &[])), &known()).unwrap()
    }

    /// Groupes d'un dessin LibreOffice : un groupe dans un groupe revient imbriqué ; un groupe d'un seul membre (un
    /// élément ou un groupe) et un groupe vide sont défaits. Dans l'import d'un PDF, rien n'est groupé.
    #[test]
    fn libreoffice_groups_become_groups_and_lonely_ones_are_undone() {
        let rect = |x: u32| format!(r#"<draw:rect draw:style-name="gr1" svg:x="{x}mm" svg:y="10mm" svg:width="5mm" svg:height="5mm"/>"#);
        let body = format!("<draw:g><draw:g>{}{}</draw:g>{}</draw:g><draw:g><draw:g>{}{}</draw:g></draw:g><draw:g>{}</draw:g><draw:g/>", rect(10), rect(20), rect(30), rect(40), rect(50), rect(60));
        let page = &read_whole(&body).pages[0];
        let paths: Vec<usize> = page.elements.iter().map(|e| crate::groups::path(e).len()).collect();
        assert_eq!(paths, [2, 2, 1, 1, 1, 0]);
        let g = |i: usize| crate::groups::path(&page.elements[i]);
        assert!(g(0) == g(1) && g(0)[0] == g(2)[0] && g(3) == g(4) && g(3) != g(0)[..1], "{:?}", page.elements);
        assert!(read(&body).pages[0].elements.iter().all(|e| e.group.is_none()));
    }

    /// Un champ non rempli tel que LibreOffice le réenregistre (relevé dans `lo2/p09_champs.odg`, LibreOffice 26.8 : les
    /// passages gardent leur style, l'encre transparente `loext:opacity="0%"`, les autres `100%`). Le nom d'un champ
    /// redevient le champ ; une lettre ne le redevient que si la description du cadre la nomme encore ; un autre texte
    /// transparent reste du texte.
    #[test]
    fn transparent_field_names_become_fields_again() {
        let auto = r##"<style:style style:name="Tv" style:family="text"><style:text-properties fo:color="#222222" loext:opacity="100%" style:font-name="Liberation Sans" fo:font-size="14pt"/></style:style><style:style style:name="Tt" style:family="text"><style:text-properties fo:color="#222222" loext:opacity="0%" style:font-name="Liberation Sans" fo:font-size="14pt"/></style:style>"##;
        let frame = |y: u32, spans: &str, desc: &str| format!(r#"<draw:frame draw:style-name="gr2" svg:width="19cm" svg:height="2cm" svg:x="1cm" svg:y="{y}cm"><draw:text-box><text:p>{spans}</text:p></draw:text-box>{desc}</draw:frame>"#);
        let body = [
            frame(1, r#"<text:span text:style-name="Tv">Bonjour </text:span><text:span text:style-name="Tt">Prénom</text:span><text:span text:style-name="Tv"> !</text:span>"#, ""),
            frame(4, r#"<text:span text:style-name="Tt">2</text:span><text:span text:style-name="Tt">3</text:span>"#, "<svg:desc>{PRENOM_2} {prenom_3}</svg:desc>"),
            frame(7, r#"<text:span text:style-name="Tt">2</text:span>"#, "<svg:desc>{PRENOM_4}</svg:desc>"),
            frame(10, r#"<text:span text:style-name="Tt">2</text:span>"#, ""),
            frame(13, r#"<text:span text:style-name="Tt">Coucou</text:span><text:span text:style-name="Tv">Prénom</text:span>"#, ""),
            // Deux champs voisins recollés par LibreOffice en un passage.
            frame(16, r#"<text:span text:style-name="Tt">12</text:span>"#, "<svg:desc>{PRENOM_1} {prenom_2}</svg:desc>"),
            frame(19, r#"<text:span text:style-name="Tt">PRÉNOMÂge</text:span>"#, ""),
        ]
        .concat();
        let odg = odg_with_styles(&format!(r#"<draw:page draw:name="page1">{body}</draw:page>"#), &format!(r#"<style:page-layout style:name="PM1"><style:page-layout-properties {A4_LAYOUT}/></style:page-layout>"#), "", &[], auto, "");
        let page = &read_drawing(Cursor::new(odg), &known()).unwrap().pages[0];
        let got: Vec<String> = texts(page).into_iter().map(plain).collect();
        assert_eq!(got, ["Bonjour {prenom} !", "{PRENOM_2}{prenom_3}", "2", "2", "CoucouPrénom", "{PRENOM_1}{prenom_2}", "{PRENOM}{age}"]);
    }

    /// Un fond donné à la page dans LibreOffice (relevé dans un fichier retouché par UNO, LibreOffice 26.8 : le style
    /// `dp1` de la page dans `content.xml`) l'emporte sur celui de la page maîtresse ; un style de page sans fond le
    /// laisse.
    #[test]
    fn a_page_colour_set_in_libreoffice_is_the_page_background() {
        let auto = r##"<style:style style:name="dp1" style:family="drawing-page"><style:drawing-page-properties draw:fill="solid" draw:fill-color="#ccffcc"/></style:style><style:style style:name="dp2" style:family="drawing-page"/>"##;
        let pages = r#"<draw:page draw:name="page1" draw:style-name="dp1" draw:master-page-name="M1"/><draw:page draw:name="page2" draw:style-name="dp2" draw:master-page-name="M1"/>"#;
        let layouts = format!(r##"<style:page-layout style:name="PM1"><style:page-layout-properties {A4_LAYOUT}/></style:page-layout><style:style style:name="Mdp1" style:family="drawing-page"><style:drawing-page-properties draw:fill="solid" draw:fill-color="#d8bc8a"/></style:style>"##);
        let masters = r#"<style:master-page style:name="M1" style:page-layout-name="PM1" draw:style-name="Mdp1"/>"#;
        let odg = odg_with_styles(pages, &layouts, masters, &[], auto, "");
        let back = read_drawing(Cursor::new(odg), &known()).unwrap();
        let colours: Vec<Option<String>> = back.pages.iter().map(|p| p.background.as_ref().map(|b| b.color.clone())).collect();
        assert_eq!(colours, [Some("#ccffcc".to_string()), Some("#d8bc8a".to_string())]);
    }

    /// Une étiquette encadrée vide telle que LibreOffice la réenregistre (relevé dans `lo2/ex_prenom.odg`, LibreOffice
    /// 26.8) : un paragraphe vide sans style, l'allure du cadre dans son style de texte (`draw:text-style-name`).
    #[test]
    fn an_empty_label_rewritten_by_libreoffice_keeps_its_font() {
        let auto = r##"<style:style style:name="grE" style:family="graphic"><style:graphic-properties draw:stroke="solid" svg:stroke-width="0.05cm" svg:stroke-color="#9aa5b1" draw:fill="none"/></style:style><style:style style:name="P4" style:family="paragraph"><loext:graphic-properties draw:fill="none"/><style:text-properties fo:color="#c0392b" loext:opacity="100%" style:font-name="Liberation Sans" fo:font-size="28pt"/></style:style>"##;
        let body = r#"<draw:frame draw:style-name="grE" draw:text-style-name="P4" svg:width="18cm" svg:height="3.6cm" svg:x="1.5cm" svg:y="15.4cm"><draw:text-box><text:p/></draw:text-box></draw:frame>"#;
        let odg = odg_with_styles(&format!(r#"<draw:page draw:name="page1">{body}</draw:page>"#), &format!(r#"<style:page-layout style:name="PM1"><style:page-layout-properties {A4_LAYOUT}/></style:page-layout>"#), "", &[], auto, "");
        let page = &read_drawing(Cursor::new(odg), &known()).unwrap().pages[0];
        let t = texts(page)[0];
        assert!(t.content.paragraphs.is_empty() && t.frame.as_ref().is_some_and(|f| f.stroke.is_some()));
        assert_eq!((t.style.font.as_str(), t.style.size_pt, t.style.color.as_str()), ("Liberation Sans", 28.0, "#c0392b"));
    }

    /// Ce que l'enseignant ajoute dans LibreOffice (relevé dans un fichier retouché par UNO, LibreOffice 26.8) : le texte
    /// tapé dans un rectangle devient un texte posé dessus, centré, groupé avec lui ; une forme au paragraphe vide reste
    /// seule ; un smiley, un octogone (formes toutes faites inconnues) et un connecteur ne sont pas repris, et comptés.
    #[test]
    fn what_a_teacher_adds_in_libreoffice_is_read_or_counted() {
        let auto = r##"<style:style style:name="gr12" style:family="graphic"><style:graphic-properties draw:textarea-horizontal-align="center" draw:textarea-vertical-align="middle"/></style:style><style:style style:name="P10" style:family="paragraph"><style:paragraph-properties fo:text-align="center"/></style:style>"##;
        let body = concat!(
            r#"<draw:custom-shape draw:style-name="gr12" svg:width="3cm" svg:height="3cm" svg:x="2cm" svg:y="24cm"><text:p/><draw:enhanced-geometry svg:viewBox="0 0 21600 21600" draw:type="smiley" draw:enhanced-path="U 10800 10800 10800 10800 0 360 Z N"/></draw:custom-shape>"#,
            r#"<draw:custom-shape draw:style-name="gr12" svg:width="3cm" svg:height="3cm" svg:x="6cm" svg:y="24cm"><text:p/><draw:enhanced-geometry svg:viewBox="0 0 21600 21600" draw:type="octagon" draw:modifiers="6326" draw:enhanced-path="M ?f0 0 L ?f2 0 Z N"/></draw:custom-shape>"#,
            r#"<draw:rect draw:style-name="gr12" draw:text-style-name="P10" svg:width="5cm" svg:height="2cm" svg:x="10cm" svg:y="24cm"><text:p text:style-name="P10">Texte dans forme</text:p></draw:rect>"#,
            r#"<draw:ellipse draw:style-name="gr12" svg:width="2cm" svg:height="2cm" svg:x="10cm" svg:y="20cm"><text:p/></draw:ellipse>"#,
            r#"<draw:connector draw:style-name="gr12" svg:x1="16cm" svg:y1="24cm" svg:x2="19cm" svg:y2="27cm" svg:d="M16000 24000v1500h3000v1500" svg:viewBox="0 0 3001 3001"><text:p/></draw:connector>"#,
        );
        let odg = odg_with_styles(&format!(r#"<draw:page draw:name="page1">{body}</draw:page>"#), &format!(r#"<style:page-layout style:name="PM1"><style:page-layout-properties {A4_LAYOUT}/></style:page-layout>"#), "", &[], auto, "");
        let back = read_drawing(Cursor::new(odg), &known()).unwrap();
        let els = &back.pages[0].elements;
        assert_eq!(back.report.skipped, 3, "{:?}", back.report);
        assert_eq!(els.len(), 3, "{els:?}");
        let ElementKind::Text(t) = &els[1].kind else { panic!("{els:?}") };
        assert_eq!((plain(t), t.style.align, t.style.valign), ("Texte dans forme".to_string(), Align::Center, Some(VAlign::Middle)));
        assert_eq!(els[1].frame, els[0].frame);
        assert!(els[0].group.is_some() && els[0].group == els[1].group && els[2].group.is_none());
    }

    /// Une forme dessinée dans LibreOffice (relevé dans un fichier retouché par UNO et dans un dessin neuf, LibreOffice
    /// 26.8) : son style automatique n'écrit que ce qui change, le reste vient de son style parent (`standard`, dans
    /// `styles.xml` : fond vert plein, trait vert). Un réglage écrit l'emporte sur celui du parent ; sans parent nommé,
    /// rien n'est hérité (les exports de Cahutin).
    #[test]
    fn a_shape_drawn_in_libreoffice_takes_what_its_style_does_not_say_from_its_parent() {
        let common = r##"<style:style style:name="standard" style:family="graphic"><style:graphic-properties draw:stroke="solid" svg:stroke-width="0.106cm" svg:stroke-color="#127a02" draw:fill="solid" draw:fill-color="#18a303"/></style:style><style:style style:name="objectwithoutfill" style:family="graphic" style:parent-style-name="standard"><style:graphic-properties draw:fill="none"/></style:style>"##;
        let auto = r##"<style:style style:name="grColour" style:family="graphic" style:parent-style-name="standard"><style:graphic-properties draw:fill-color="#ffee99"/></style:style><style:style style:name="grNone" style:family="graphic" style:parent-style-name="standard"><style:graphic-properties draw:fill="none" draw:stroke="none"/></style:style><style:style style:name="grChain" style:family="graphic" style:parent-style-name="objectwithoutfill"><style:graphic-properties/></style:style><style:style style:name="grOrphan" style:family="graphic"><style:graphic-properties draw:fill-color="#ffee99"/></style:style>"##;
        let rect = |style: &str, y: u32| format!(r#"<draw:rect draw:style-name="{style}" svg:width="5cm" svg:height="2cm" svg:x="2cm" svg:y="{y}cm"><text:p/></draw:rect>"#);
        let body = [rect("grColour", 2), rect("grNone", 6), rect("grChain", 10), rect("grOrphan", 14), rect("standard", 18)].concat();
        let odg = odg_with_styles(&format!(r#"<draw:page draw:name="page1">{body}</draw:page>"#), &format!(r#"<style:page-layout style:name="PM1"><style:page-layout-properties {A4_LAYOUT}/></style:page-layout>"#), "", &[], auto, common);
        let page = &read_drawing(Cursor::new(odg), &known()).unwrap().pages[0];
        let got: Vec<(Option<String>, Option<(f64, String)>)> = page
            .elements
            .iter()
            .map(|e| {
                let ElementKind::Shape(s) = &e.kind else { panic!("{e:?}") };
                (s.fill.clone(), s.stroke.as_ref().map(|st| ((st.width_mm * 100.0).round() / 100.0, st.color.clone())))
            })
            .collect();
        let green = Some((1.06, "#127a02".to_string()));
        assert_eq!(got, [(Some("#ffee99".into()), green.clone()), (None, None), (None, green.clone()), (None, None), (Some("#18a303".into()), green)]);
    }

    /// Le texte tapé dans une forme dans LibreOffice prend de la chaîne des styles parents ce que son style ne dit pas :
    /// taille, police, gras, couleur (XML relevé dans deux fichiers de LibreOffice 26.8 : `standard` d'un export
    /// réenregistré, la chaîne `Filled Blue` → `Filled` → `Shapes` → `Graphic` d'un dessin neuf). La couleur
    /// automatique (`style:use-window-font-color`, ou aucune couleur écrite) est blanche sur un fond foncé, noire sur un
    /// fond clair ou sans fond plein, comme LibreOffice 26.8 la rend. Un passage qui écrit sa taille et sa couleur les
    /// garde ; un style sans parent nommé n'hérite de rien (sa couleur, qu'il n'écrit pas, reste automatique).
    #[test]
    fn text_typed_in_a_libreoffice_shape_takes_size_font_and_colour_from_its_parent_styles() {
        let common = r##"<style:style style:name="standard" style:family="graphic"><style:graphic-properties draw:stroke="solid" svg:stroke-width="0.106cm" svg:stroke-color="#127a02" draw:fill="solid" draw:fill-color="#18a303" draw:textarea-horizontal-align="justify" fo:padding-top="0.125cm" fo:padding-bottom="0.125cm" fo:padding-left="0.25cm" fo:padding-right="0.25cm" fo:wrap-option="wrap" draw:shadow="hidden"/><style:paragraph-properties fo:margin-left="0cm" fo:margin-right="0cm" fo:margin-top="0cm" fo:margin-bottom="0cm" fo:line-height="100%" fo:text-indent="0cm"/><style:text-properties fo:font-variant="normal" fo:text-transform="none" style:use-window-font-color="true" loext:opacity="0%" style:text-outline="false" style:text-line-through-style="none" style:text-line-through-type="none" style:font-name="Liberation Sans" fo:font-family="&apos;Liberation Sans&apos;" style:font-family-generic="roman" style:font-pitch="variable" fo:font-size="18pt" fo:font-style="normal" fo:text-shadow="none" style:text-underline-style="none" fo:font-weight="normal" style:letter-kerning="true" style:font-size-asian="18pt" style:font-size-complex="18pt" style:text-emphasize="none" style:font-relief="none" loext:font-optical-sizing="auto" style:text-overline-style="none" style:text-overline-color="font-color"/></style:style><style:style style:name="Graphic" style:family="graphic"><style:graphic-properties draw:fill="solid" draw:fill-color="#ffffff"/><style:text-properties style:font-name="Liberation Sans" fo:font-family="&apos;Liberation Sans&apos;" style:font-family-generic="roman" style:font-pitch="variable" fo:font-size="18pt"/></style:style><style:style style:name="Shapes" style:family="graphic" style:parent-style-name="Graphic"><style:graphic-properties draw:stroke="none" draw:fill="gradient" draw:fill-gradient-name="Shapes"/><style:text-properties fo:font-size="14pt" fo:font-weight="bold"/></style:style><style:style style:name="Filled" style:family="graphic" style:parent-style-name="Shapes"><style:graphic-properties draw:fill="gradient" draw:fill-gradient-name="Filled"/></style:style><style:style style:name="Filled_20_Blue" style:display-name="Filled Blue" style:family="graphic" style:parent-style-name="Filled"><style:graphic-properties draw:fill-gradient-name="Filled_20_Blue"/><style:text-properties fo:color="#ffffff" loext:opacity="100%"/></style:style>"##;
        let auto = r##"<style:style style:name="gr8" style:family="graphic" style:parent-style-name="standard"><style:graphic-properties draw:fill-color="#ffee99" draw:textarea-horizontal-align="center" draw:textarea-vertical-align="middle" loext:decorative="false"/><style:paragraph-properties style:writing-mode="lr-tb"/></style:style><style:style style:name="gr4" style:family="graphic" style:parent-style-name="standard"><style:graphic-properties draw:textarea-horizontal-align="center" draw:textarea-vertical-align="middle" loext:decorative="false"/><style:paragraph-properties style:writing-mode="lr-tb"/></style:style><style:style style:name="grNavy" style:family="graphic" style:parent-style-name="standard"><style:graphic-properties draw:fill-color="#000080"/></style:style><style:style style:name="grNoFill" style:family="graphic" style:parent-style-name="standard"><style:graphic-properties draw:fill="none"/></style:style><style:style style:name="grBlue" style:family="graphic" style:parent-style-name="Filled_20_Blue"><style:graphic-properties loext:decorative="false"/></style:style><style:style style:name="grOrphan" style:family="graphic"><style:graphic-properties draw:fill="solid" draw:fill-color="#000080"/></style:style><style:style style:name="P6" style:family="paragraph"><loext:graphic-properties draw:fill-color="#ffee99"/><style:paragraph-properties fo:text-align="center" loext:paragraph-composer="false"/></style:style><style:style style:name="TRed" style:family="text"><style:text-properties fo:color="#c0392b" fo:font-size="30pt"/></style:style>"##;
        let rect = |style: &str, y: u32, p: &str| format!(r#"<draw:rect draw:style-name="{style}" draw:text-style-name="P6" draw:layer="layout" svg:width="6cm" svg:height="2cm" svg:x="2cm" svg:y="{y}cm">{p}</draw:rect>"#);
        let body = [
            rect("gr8", 1, "<text:p>Tapé dans LibreOffice</text:p>"),
            rect("gr4", 4, "<text:p>Dans le cadre</text:p>"),
            rect("grNavy", 7, "<text:p>Nuit</text:p>"),
            rect("grNoFill", 10, "<text:p>Sans fond</text:p>"),
            rect("grBlue", 13, "<text:p>Bleu</text:p>"),
            rect("gr8", 16, r#"<text:p><text:span text:style-name="TRed">Rouge</text:span></text:p>"#),
            rect("grOrphan", 19, "<text:p>Seul</text:p>"),
        ]
        .concat();
        let odg = odg_with_styles(&format!(r#"<draw:page draw:name="page1">{body}</draw:page>"#), &format!(r#"<style:page-layout style:name="PM1"><style:page-layout-properties {A4_LAYOUT}/></style:page-layout>"#), "", &[], auto, common);
        let page = &read_drawing(Cursor::new(odg), &known()).unwrap().pages[0];
        let got: Vec<(String, f64, String, String, bool)> = texts(page).iter().map(|t| (plain(t), t.style.size_pt, t.style.font.clone(), t.style.color.clone(), t.content.paragraphs[0].runs[0].bold)).collect();
        let sans = "Liberation Sans".to_string();
        assert_eq!(
            got,
            [
                ("Tapé dans LibreOffice".into(), 18.0, sans.clone(), "#000000".into(), false),
                ("Dans le cadre".into(), 18.0, sans.clone(), "#ffffff".into(), false),
                ("Nuit".into(), 18.0, sans.clone(), "#ffffff".into(), false),
                ("Sans fond".into(), 18.0, sans.clone(), "#000000".into(), false),
                ("Bleu".into(), 14.0, sans.clone(), "#ffffff".into(), true),
                ("Rouge".into(), 30.0, sans.clone(), "#c0392b".into(), false),
                ("Seul".into(), 12.0, FALLBACK_FONT.to_string(), "#ffffff".into(), false),
            ]
        );
    }

    /// Couleur automatique du texte d'une forme, d'après son fond, comme LibreOffice 26.8 la rend (relevé sur un rendu) :
    /// blanche jusqu'à une luminance relative (sRGB) de 0,345, noire au-delà.
    #[test]
    fn automatic_text_colour_follows_the_fill_like_libreoffice() {
        let auto = |fill: &str| auto_text_color(Some(fill));
        for white in ["#141414", "#3f3f3f", "#808080", "#9e9e9e", "#0000ff", "#ff0000", "#18a303", "#c000c0", "#ff7070", "#9090ff"] {
            assert_eq!(auto(white), "#ffffff", "{white}");
        }
        for black in ["#a0a0a0", "#ffee99", "#00ff00", "#ff8000", "#00c0c0", "#40b0b0", "#ff7878", "#ffffff"] {
            assert_eq!(auto(black), "#000000", "{black}");
        }
        assert_eq!(auto_text_color(None), "#000000");
        assert_eq!(auto("pas une couleur"), "#000000");
    }

    /// Un export de Cahutin retouché dans LibreOffice 26.8 par UNO (`tests/fixtures/retouche-libreoffice.odg` : le
    /// groupe déplacé de 3 mm, puis dissocié et regroupé ; formes et photo générées, sans texte) se rouvre avec son
    /// groupe dans un groupe, sa forme verrouillée, sa flèche, ses tirets, sa photo bordée et ombrée, son fond à lignes.
    #[test]
    fn a_drawing_retouched_in_libreoffice_comes_back() {
        let back = read_drawing(Cursor::new(include_bytes!("../../tests/fixtures/retouche-libreoffice.odg").to_vec()), &known()).unwrap();
        let page = &back.pages[0];
        assert_eq!(back.report.skipped, 0, "{:?}", back.report);
        assert_eq!(page.background, Some(Background { color: "#fff8dc".into(), pattern: Some(Pattern::Lines) }));
        let stars: Vec<&Element> = page.elements.iter().filter(|e| matches!(&e.kind, ElementKind::Shape(s) if s.shape == ShapeKind::Star)).collect();
        let paths: Vec<Vec<Id>> = stars.iter().map(|e| crate::groups::path(e)).collect();
        assert_eq!(paths.iter().map(Vec::len).collect::<Vec<_>>(), [2, 2, 1]);
        assert!(paths[0] == paths[1] && paths[0][0] == paths[2][0]);
        assert!((stars[0].frame.x_mm - 23.0).abs() < 0.01, "déplacé de 3 mm : {:?}", stars[0].frame);
        let shape = |k: ShapeKind| page.elements.iter().find(|e| matches!(&e.kind, ElementKind::Shape(s) if s.shape == k)).unwrap();
        assert!(shape(ShapeKind::Heart).locked && !stars[0].locked);
        let ElementKind::Shape(rect) = &shape(ShapeKind::Rect).kind else { unreachable!() };
        assert_eq!(rect.stroke.as_ref().and_then(|s| s.style), Some(StrokeStyle::Dashed));
        shape(ShapeKind::Arrow);
        let photo = page.elements.iter().find_map(|e| if let ElementKind::Photo(p) = &e.kind { Some(p) } else { None }).unwrap();
        assert_eq!((photo.border.as_ref().and_then(|b| b.style), photo.shadow), (Some(StrokeStyle::Dotted), true));
    }

    /// Les espaces ODF posés directement dans le paragraphe (hors d'un passage) comptent, tabulation et retour à la ligne
    /// compris.
    #[test]
    fn spaces_written_in_the_paragraph_itself_are_kept() {
        let body = r#"<draw:frame draw:style-name="gr2" svg:width="10cm" svg:height="2cm" svg:x="1cm" svg:y="1cm"><draw:text-box><text:p>Lundi<text:tab/>Mardi<text:s text:c="2"/>!<text:line-break/>Jeudi</text:p></draw:text-box></draw:frame>"#;
        let page = &read_whole(body).pages[0];
        assert_eq!(plain(texts(page)[0]), "Lundi\tMardi  !\nJeudi");
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

    /// LibreOffice range une image en deux exemplaires, un dessin WMF puis sa copie PNG : la copie est reprise.
    #[test]
    fn a_picture_stored_twice_is_read_from_the_copy_cahutin_can_read() {
        let body = r#"<draw:frame svg:width="4cm" svg:height="3cm" svg:x="2cm" svg:y="2cm"><draw:image xlink:href="Pictures/dessin.wmf" draw:mime-type="image/x-wmf"><text:p/></draw:image><draw:image xlink:href="Pictures/copie.png" draw:mime-type="image/png"/></draw:frame>"#;
        let wmf = b"\xd7\xcd\xc6\x9a\0\0 pas une image lisible".to_vec();
        let imported = read_drawing(Cursor::new(odg_fixture(body, A4_LAYOUT, &[("dessin.wmf", wmf), ("copie.png", png())])), &known()).unwrap();
        assert_eq!((imported.assets.len(), imported.report.skipped), (1, 0));
        assert!(matches!(imported.pages[0].elements[0].kind, ElementKind::Photo(_)));
    }

    /// Une photo posée dans une forme (un rectangle arrondi, un polygone) est une forme remplie d'une image : elle
    /// devient une photo à sa place, avec ses coins arrondis ; la même image sur deux pages n'est lue qu'une fois.
    #[test]
    fn a_shape_filled_with_a_picture_becomes_a_photo() {
        let rect = r#"<draw:rect draw:style-name="grPhoto" draw:corner-radius="0.5cm" svg:width="6cm" svg:height="4cm" svg:x="2cm" svg:y="3cm"><text:p/></draw:rect>"#;
        let poly = r#"<draw:polygon draw:style-name="grPhoto" svg:width="5cm" svg:height="5cm" svg:x="10cm" svg:y="3cm" svg:viewBox="0 0 5000 5000" draw:points="0,0 5000,0 5000,5000 0,5000"/>"#;
        let pages = format!(r#"<draw:page draw:name="page1">{rect}{poly}</draw:page><draw:page draw:name="page2">{rect}</draw:page>"#);
        let layouts = format!(r#"<style:page-layout style:name="PM1"><style:page-layout-properties fo:margin-top="2cm" {A4_LAYOUT}/></style:page-layout>"#);
        let imported = read_drawing(Cursor::new(odg_raw(&pages, &layouts, "", &[("remplissage.png", png())])), &known()).unwrap();
        assert_eq!(imported.assets.len(), 1, "une seule image, lue une fois");
        let photos: Vec<(&Element, &PhotoProps)> = imported.pages.iter().flat_map(|p| &p.elements).filter_map(|e| if let ElementKind::Photo(p) = &e.kind { Some((e, p)) } else { None }).collect();
        assert_eq!(photos.len(), 3);
        assert!(photos.iter().all(|(_, p)| p.asset.as_deref() == Some(imported.assets[0].meta.id.as_str())));
        let (rect_el, rect_photo) = photos[0];
        assert!((rect_photo.corner_radius_mm - 5.0).abs() < 0.01);
        assert!((rect_el.frame.x_mm - 20.0).abs() < 0.01 && (rect_el.frame.w_mm - 60.0).abs() < 0.01);
        assert_eq!(photos[1].1.corner_radius_mm, 0.0);
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
    fn a_few_curves_become_editable_paths_and_rotated_shapes_are_read() {
        let body = r#"<draw:polygon draw:style-name="gr1" svg:width="2cm" svg:height="2cm" svg:x="2cm" svg:y="2cm" svg:viewBox="0 0 2000 2000" draw:points="0,2000 1000,0 2000,2000"/>
<draw:path draw:style-name="gr1" svg:width="2cm" svg:height="2cm" svg:x="5cm" svg:y="2cm" svg:viewBox="0 0 2000 2000" svg:d="M 0 0 C 500 2000 1500 2000 2000 0"/>
<draw:rect draw:style-name="gr1" svg:width="2cm" svg:height="1cm" draw:transform="rotate (0.5) translate (8cm 2cm)"/>
<draw:g><draw:ellipse draw:style-name="gr1" svg:width="2cm" svg:height="1cm" svg:x="2cm" svg:y="8cm"/><draw:line draw:style-name="gr1" svg:x1="2cm" svg:y1="12cm" svg:x2="6cm" svg:y2="12cm"/></draw:g>"#;
        let imported = read(body);
        // Le triangle et la courbe : deux tracés libres modifiables, au fond de la page, à leur place.
        assert_eq!((imported.report.skipped, imported.report.drawn, imported.report.images, imported.assets.len()), (0, 0, 0, 0));
        let kinds: Vec<ShapeKind> = imported.pages[0].elements.iter().filter_map(|e| if let ElementKind::Shape(s) = &e.kind { Some(s.shape) } else { None }).collect();
        assert_eq!(kinds, [ShapeKind::Path, ShapeKind::Path, ShapeKind::Rect, ShapeKind::Ellipse, ShapeKind::Line]);
        let (tri, curve) = (&imported.pages[0].elements[0], &imported.pages[0].elements[1]);
        assert!((tri.frame.x_mm - 20.0).abs() < 0.1 && (tri.frame.w_mm - 20.0).abs() < 0.1, "{:?}", tri.frame);
        assert!((curve.frame.x_mm - 50.0).abs() < 0.1, "{:?}", curve.frame);
        let ElementKind::Shape(t) = &tri.kind else { unreachable!() };
        assert_eq!(t.path.as_deref(), Some("M 0 1000 L 500 0 L 1000 1000 Z"));
        assert!(crate::shapes::valid_path(match &curve.kind { ElementKind::Shape(c) => c.path.as_deref().unwrap(), _ => unreachable!() }));
        // Tourné de 0,5 radian dans le sens ODF (inverse des aiguilles) : 331,4° dans Cahutin.
        let rect = &imported.pages[0].elements[2].frame;
        assert!((rect.rotation_deg - (360.0 - 0.5f64.to_degrees())).abs() < 0.01 && (rect.w_mm - 20.0).abs() < 0.01, "{rect:?}");
        let line = &imported.pages[0].elements[4].frame;
        assert!((line.x_mm - 20.0).abs() < 0.01 && (line.w_mm - 40.0).abs() < 0.01 && (line.y_mm + line.h_mm / 2.0 - 120.0).abs() < 0.01, "{line:?}");
    }

    /// Un tracé réenregistré par LibreOffice, sans la lettre des commandes qui se suivent, se relit épelé.
    #[test]
    fn repeated_path_commands_are_spelled_out() {
        assert_eq!(spelled_out("M 0 0 L 500 0 0 1000 Z").as_deref(), Some("M 0 0 L 500 0 L 0 1000 Z"));
        assert_eq!(spelled_out("M 0 0 10 10 C 1 2 3 4 5 6 7 8 9 10 11 12").as_deref(), Some("M 0 0 L 10 10 C 1 2 3 4 5 6 C 7 8 9 10 11 12"));
        assert_eq!(spelled_out("M 0 0 L 1"), None);
        assert_eq!(spelled_out("Z 1 2"), None);
    }

    /// Le même ODG, avec un `meta.xml` qui nomme son générateur.
    fn with_generator(odg: Vec<u8>, generator: &str) -> Vec<u8> {
        let mut zin = ZipArchive::new(Cursor::new(odg)).unwrap();
        let mut buf = Cursor::new(Vec::new());
        {
            let mut zip = zip::ZipWriter::new(&mut buf);
            for i in 0..zin.len() {
                zip.raw_copy_file(zin.by_index(i).unwrap()).unwrap();
            }
            zip.start_file("meta.xml", zip::write::SimpleFileOptions::default()).unwrap();
            let meta = format!(r#"<?xml version="1.0" encoding="UTF-8"?><office:document-meta xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0" xmlns:meta="{META}"><office:meta><meta:generator>{generator}</meta:generator></office:meta></office:document-meta>"#);
            zip.write_all(meta.as_bytes()).unwrap();
            zip.finish().unwrap();
        }
        buf.into_inner()
    }

    /// Le même ODG, avec un `meta.xml` donné.
    fn with_meta(odg: Vec<u8>, office_meta: &str) -> Vec<u8> {
        let mut zin = ZipArchive::new(Cursor::new(odg)).unwrap();
        let mut buf = Cursor::new(Vec::new());
        {
            let mut zip = zip::ZipWriter::new(&mut buf);
            for i in 0..zin.len() {
                zip.raw_copy_file(zin.by_index(i).unwrap()).unwrap();
            }
            zip.start_file("meta.xml", zip::write::SimpleFileOptions::default()).unwrap();
            let meta = format!(r#"<?xml version="1.0" encoding="UTF-8"?><office:document-meta xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0" xmlns:meta="{META}"><office:meta>{office_meta}</office:meta></office:document-meta>"#);
            zip.write_all(meta.as_bytes()).unwrap();
            zip.finish().unwrap();
        }
        buf.into_inner()
    }

    /// La marque de Cahutin, telle que LibreOffice la réécrit (sans `meta:value-type`) ou telle que l'export l'écrit ; ni
    /// le générateur (réécrit par LibreOffice), ni la mention OpenMoji seule, ni la marque sous un autre nom ne suffisent.
    #[test]
    fn the_cahutin_mark_is_found_in_the_file_properties() {
        let odg = odg_fixture("", A4_LAYOUT, &[]);
        let marked = |meta: &str| from_cahutin(Cursor::new(with_meta(odg.clone(), meta)));
        assert!(marked(r#"<meta:generator>LibreOffice/24.2.7.2</meta:generator><meta:user-defined meta:name="Logiciel">Cahutin</meta:user-defined>"#));
        assert!(marked(r#"<meta:user-defined meta:name="Logiciel" meta:value-type="string">Cahutin</meta:user-defined>"#));
        assert!(!marked(r#"<meta:generator>cahutin/0.17.0 miroir-apres-rotation</meta:generator>"#));
        assert!(!marked(r#"<meta:user-defined meta:name="Crédits">Pictogrammes : OpenMoji (openmoji.org), licence CC BY-SA 4.0</meta:user-defined>"#));
        assert!(!marked(r#"<meta:user-defined meta:name="Auteur">Cahutin</meta:user-defined><meta:user-defined meta:name="Logiciel">Autre</meta:user-defined>"#));
        assert!(!from_cahutin(Cursor::new(odg.clone())));
        assert!(!from_cahutin(Cursor::new(b"pas un zip".to_vec())));
        // Un export de Cahutin la porte.
        let mut buf = Cursor::new(Vec::new());
        crate::odg::export_odg(&crate::model::Document::new("Fiche"), &crate::photo::AssetStore::default(), &crate::fonts::FontSet::builtin(), &mut buf).unwrap();
        assert!(from_cahutin(Cursor::new(buf.into_inner())));
    }

    /// Les formes toutes faites de LibreOffice que Cahutin ne redessine pas (un smiley, une flèche de liaison) sont
    /// comptées à part parmi les dessins non repris ; un objet (graphique) manque aussi, sans en être une.
    #[test]
    fn ready_made_shapes_are_counted_apart() {
        let smiley = r#"<draw:custom-shape draw:style-name="gr1" svg:width="3cm" svg:height="3cm" svg:x="1cm" svg:y="1cm"><draw:enhanced-geometry svg:viewBox="0 0 21600 21600" draw:type="smiley" draw:enhanced-path="U 10800 10800 10800 10800 0 360 Z N"/></draw:custom-shape>"#;
        let connector = r#"<draw:connector draw:style-name="gr1" svg:x1="1cm" svg:y1="6cm" svg:x2="8cm" svg:y2="6cm"/>"#;
        let chart = r#"<draw:frame svg:width="3cm" svg:height="3cm" svg:x="1cm" svg:y="9cm"><draw:object xlink:href="./Object 1"/></draw:frame>"#;
        let rect = r#"<draw:rect draw:style-name="gr1" svg:width="3cm" svg:height="3cm" svg:x="10cm" svg:y="1cm"/>"#;
        let r = read_whole(&format!("{smiley}{connector}{chart}{rect}")).report;
        assert_eq!((r.shapes, r.skipped, r.ready_made, r.fallback), (1, 3, 2, false), "{r:?}");
    }

    /// Forme retournée de gauche à droite et tournée : LibreOffice la retourne après l'avoir tournée, l'angle lu est
    /// donc l'opposé de celui de l'écran (fichier de LibreOffice, ou de Cahutin avec la marque). Un fichier écrit par
    /// Cahutin avant la marque portait l'angle de l'écran : il le garde.
    #[test]
    fn a_mirrored_turned_shape_reads_the_opposite_angle_except_in_old_cahutin_files() {
        let body = r#"<draw:custom-shape draw:style-name="gr1" svg:width="4cm" svg:height="2cm" draw:transform="rotate (-0.523598775598) translate (5cm 5cm)"><draw:enhanced-geometry svg:viewBox="0 0 1000 1000" draw:enhanced-path="M 0 0 L 500 0 L 0 1000 Z N" draw:mirror-horizontal="true"/></draw:custom-shape>"#;
        let odg = odg_fixture(body, A4_LAYOUT, &[]);
        let read_with = |generator: Option<&str>| {
            let bytes = generator.map_or(odg.clone(), |g| with_generator(odg.clone(), g));
            let e = read_drawing(Cursor::new(bytes), &known()).unwrap().pages[0].elements[0].clone();
            assert!(e.flip_h && !e.flip_v, "{e:?}");
            (e.frame.rotation_deg * 1000.0).round() / 1000.0
        };
        assert_eq!(read_with(None), 330.0);
        assert_eq!(read_with(Some("LibreOffice/26.8.2.2$Linux_X86_64 LibreOffice_project/c377f90")), 330.0);
        assert_eq!(read_with(Some(&format!("cahutin/0.18.0 {}", crate::odg::MIRROR_AFTER_ROTATION))), 330.0);
        assert_eq!(read_with(Some("cahutin/0.17.0")), 30.0);
    }

    /// Beaucoup de tracés (une illustration) : redessinés ensemble dans une image fixe, au fond de la page.
    #[test]
    fn many_curves_become_one_fixed_picture() {
        let body: String = (0..EDITABLE_TRACES + 1)
            .map(|i| format!(r#"<draw:path draw:style-name="gr1" svg:width="1cm" svg:height="1cm" svg:x="{}cm" svg:y="2cm" svg:viewBox="0 0 1000 1000" svg:d="M 0 0 C 300 1000 700 1000 1000 0"/>"#, 1 + i % 15))
            .collect();
        let imported = read(&body);
        assert_eq!((imported.report.drawn, imported.report.shapes, imported.assets.len()), (EDITABLE_TRACES + 1, 0, 1));
        let back = &imported.pages[0].elements[0];
        assert!(back.locked && matches!(back.kind, ElementKind::Photo(_)), "{back:?}");
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

    /// Une page A5 à points et une page A4 paysage à lignes, réenregistrées par LibreOffice (éléments, styles et unités
    /// tels qu'il les écrit). LibreOffice 26.8 garde une mise en page par page maîtresse ; LibreOffice 24.2 les met
    /// toutes sur une seule (ici A4 portrait) : les marques, dessinées pour leur format,
    /// ne sont plus à la taille lue, et restent pourtant reconnues. Une marque en moins ou déplacée ne l'est pas.
    #[test]
    fn a_page_pattern_comes_back_when_libreoffice_keeps_one_page_layout() {
        let cm = |mm: f64| format!("{}cm", (mm * 1000.0).round() / 10000.0);
        let dot = |x: f64, y: f64| format!(r#"<draw:circle draw:style-name="Mgr4" draw:text-style-name="MP2" draw:layer="backgroundobjects" svg:width="0.06cm" svg:height="0.06cm" svg:x="{}" svg:y="{}"><text:p/></draw:circle>"#, cm(x - 0.3), cm(y - 0.3));
        let line = |y: f64| format!(r#"<draw:line draw:style-name="Mgr5" draw:text-style-name="MP1" draw:layer="backgroundobjects" svg:x1="0cm" svg:y1="{0}" svg:x2="29.7cm" svg:y2="{0}"><text:p/></draw:line>"#, cm(y));
        let styles = r##"<style:style style:name="Mgr4" style:family="graphic" style:parent-style-name="standard"><style:graphic-properties draw:stroke="none" draw:fill="solid" draw:fill-color="#bdbdbd" loext:decorative="false"/></style:style><style:style style:name="Mgr5" style:family="graphic" style:parent-style-name="standard"><style:graphic-properties draw:stroke="solid" svg:stroke-width="0.02cm" svg:stroke-color="#c8c8c8" draw:fill="none" fo:padding-top="0.135cm" fo:padding-bottom="0.135cm" fo:padding-left="0.26cm" fo:padding-right="0.26cm" loext:decorative="false"/></style:style><style:style style:name="Mdp1" style:family="drawing-page"><style:drawing-page-properties draw:background-size="border" draw:fill="solid" draw:fill-color="#ffffff"/></style:style><style:style style:name="Mdp2" style:family="drawing-page"><style:drawing-page-properties draw:background-size="border" draw:fill="solid" draw:fill-color="#fff8dc"/></style:style>"##;
        let layout = |name: &str, w: &str, h: &str, o: &str| format!(r#"<style:page-layout style:name="{name}"><style:page-layout-properties fo:margin-top="0cm" fo:margin-bottom="0cm" fo:margin-left="0cm" fo:margin-right="0cm" fo:page-width="{w}" fo:page-height="{h}" style:print-orientation="{o}"/></style:page-layout>"#);
        let pages = r#"<draw:page draw:name="page2" draw:style-name="dp1" draw:master-page-name="M2"/><draw:page draw:name="page3" draw:style-name="dp1" draw:master-page-name="M3"/>"#;
        // Les points de 10 en 10 mm sur 148 × 210 (14 × 20, ligne par ligne), les lignes de 8 en 8 mm sur 297 × 210.
        let grid = |rows: usize| -> Vec<(f64, f64)> { (1..=rows).flat_map(|j| (1..=14).map(move |i| (10.0 * i as f64, 10.0 * j as f64))).collect() };
        let read_back = |dots: &[(f64, f64)], lines: usize, one_layout: bool| {
            let dots: String = dots.iter().map(|&(x, y)| dot(x, y)).collect();
            let lines: String = (1..=lines).map(|k| line(8.0 * k as f64)).collect();
            let (m2, m3) = if one_layout { ("PM0", "PM0") } else { ("PM1", "PM2") };
            let masters = format!(r#"<style:master-page style:name="M2" style:page-layout-name="{m2}" draw:style-name="Mdp2">{dots}</style:master-page><style:master-page style:name="M3" style:page-layout-name="{m3}" draw:style-name="Mdp1">{lines}</style:master-page>"#);
            let mut layouts = layout("PM0", "21cm", "29.7cm", "portrait");
            if !one_layout {
                layouts += &(layout("PM1", "14.8cm", "21cm", "portrait") + &layout("PM2", "29.7cm", "21cm", "landscape"));
            }
            let imported = read_odg(Cursor::new(odg_with_styles(pages, &format!("{layouts}{styles}"), &masters, &[], "", "")), &known()).unwrap();
            imported.pages.iter().map(|p| (p.background.clone().unwrap().pattern, p.size.width_mm, p.size.height_mm)).collect::<Vec<_>>()
        };
        let a5 = grid(20);
        assert_eq!(read_back(&a5, 26, false), [(Some(Pattern::Dots), 148.0, 210.0), (Some(Pattern::Lines), 297.0, 210.0)]);
        // LibreOffice 24.2 : la taille des pages est perdue, pas leur motif.
        assert_eq!(read_back(&a5, 26, true), [(Some(Pattern::Dots), 210.0, 297.0), (Some(Pattern::Lines), 210.0, 297.0)]);
        let mut shifted = a5.clone();
        shifted[150].0 += 0.1;
        for (dots, lines) in [(grid(19), 25), (a5[1..].to_vec(), 25), (shifted, 25)] {
            assert_eq!(read_back(&dots, lines, true), [(None, 210.0, 297.0), (None, 210.0, 297.0)]);
        }
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

    /// Une liste à puces de LibreOffice (style de l'ODG ou de `styles.xml`, quelle que soit la puce) redonne des
    /// paragraphes à puce, son texte gardé tel quel ; une liste numérotée, une liste imbriquée, un titre de liste et
    /// le deuxième paragraphe d'un élément restent des paragraphes simples.
    #[test]
    fn a_bullet_list_of_a_drawing_gives_bulleted_paragraphs() {
        let p = |t: &str| format!(r#"<text:p><text:span text:style-name="T1">{t}</text:span></text:p>"#);
        let item = |t: &str| format!("<text:list-item>{}</text:list-item>", p(t));
        let body = format!(
            r#"<draw:frame draw:style-name="gr2" svg:width="10cm" svg:height="8cm" svg:x="2cm" svg:y="2cm"><draw:text-box>{}<text:list text:style-name="LB">{}<text:list-header>{}</text:list-header><text:list-item>{}{}</text:list-item></text:list><text:list text:style-name="LN">{}</text:list><text:list text:style-name="LB"><text:list-item>{}<text:list>{}</text:list></text:list-item></text:list><text:list text:continue-numbering="true" text:style-name="LS">{}</text:list>{}</draw:text-box></draw:frame>"#,
            p("Titre"), item("Colle"), p("Sans puce"), p("Ciseaux"), p("suite"), item("Un"), p("Haut"), item("Dedans"), item("Commun"), p("• tapée")
        );
        let imported = read_drawing(Cursor::new(odg_fixture(&body, A4_LAYOUT, &[])), &known()).unwrap();
        let ElementKind::Text(t) = &imported.pages[0].elements[0].kind else { panic!() };
        let got: Vec<(String, bool)> = t.content.paragraphs.iter().map(|p| (p.runs.iter().map(|r| r.text.as_str()).collect(), p.list == Some(ListKind::Bullet))).collect();
        let want = [("Titre", false), ("Colle", true), ("Sans puce", false), ("Ciseaux", true), ("suite", false), ("Un", false), ("Haut", false), ("Dedans", false), ("Commun", true), ("• tapée", false)];
        assert_eq!(got, want.map(|(t, b)| (t.to_string(), b)));
    }

    /// Le motif d'un pointillé de LibreOffice : trait et espace en mm ; une longueur en pourcentage est ignorée.
    #[test]
    fn stroke_dash_patterns_are_read_in_mm() {
        let xml = r#"<office:styles xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0" xmlns:draw="urn:oasis:names:tc:opendocument:xmlns:drawing:1.0">
<draw:stroke-dash draw:name="d1" draw:style="rect" draw:dots1="1" draw:dots1-length="0.105cm" draw:distance="0.105cm"/>
<draw:stroke-dash draw:name="d2" draw:style="rect" draw:dots1="1" draw:dots1-length="200%" draw:distance="100%"/></office:styles>"#;
        let doc = roxmltree::Document::parse(xml).unwrap();
        let dashes = stroke_dashes(&[&doc]);
        let (on, off) = dashes["d1"];
        assert!((on - 1.05).abs() < 1e-9 && (off - 1.05).abs() < 1e-9);
        assert!(!dashes.contains_key("d2"));
    }

    #[test]
    fn an_underline_drawn_under_words_underlines_them() {
        // Relevé d'une conversion PDF par LibreOffice : le trait est à part, juste sous la ligne.
        let line = |x1: &str, x2: &str, y: &str| format!(r#"<draw:line draw:style-name="gr2" svg:x1="{x1}" svg:y1="{y}" svg:x2="{x2}" svg:y2="{y}"/>"#);
        let body = [
            text_frame("2.004cm", "2.018cm", "5.01cm", r#"<text:span text:style-name="T1">Écris ton prénom sur la ligne.</text:span>"#),
            line("3.624cm", "4.915cm", "2.444cm"),
            text_frame("2.004cm", "4.505cm", "4.904cm", r#"<text:span text:style-name="T1">Consigne soulignée en entier</text:span>"#),
            line("2.003cm", "6.903cm", "4.931cm"),
        ]
        .concat();
        let imported = read(&body);
        let page = &imported.pages[0];
        assert!(page.elements.iter().all(|e| !matches!(e.kind, ElementKind::Shape(_))), "les traits ne restent pas");
        let runs: Vec<Vec<(String, bool)>> = texts(page).iter().map(|t| t.content.paragraphs.iter().flat_map(|p| &p.runs).map(|r| (r.text.clone(), r.underline)).collect()).collect();
        assert_eq!(runs[0], [("Écris ton ".to_string(), false), ("prénom".to_string(), true), (" sur la ligne.".to_string(), false)]);
        assert_eq!(runs[1], [("Consigne soulignée en entier".to_string(), true)]);
    }

    #[test]
    fn an_empty_frame_is_not_reported_missing() {
        let body = [
            r#"<draw:frame draw:style-name="gr2" svg:width="3cm" svg:height="1cm" svg:x="1cm" svg:y="1cm"/>"#,
            r#"<draw:frame draw:style-name="gr2" svg:width="3cm" svg:height="1cm" svg:x="1cm" svg:y="3cm"><draw:object xlink:href="./Object 1"/></draw:frame>"#,
        ]
        .concat();
        assert_eq!(read(&body).report.skipped, 1, "seul l'objet manque");
    }

    #[test]
    fn a_line_away_from_text_stays_a_line() {
        let body = [
            text_frame("2cm", "2cm", "5cm", r#"<text:span text:style-name="T1">Prénom :</text:span>"#),
            // Une ligne d'écriture, bien plus bas : un trait à garder.
            r#"<draw:line draw:style-name="gr2" svg:x1="2cm" svg:y1="5cm" svg:x2="12cm" svg:y2="5cm"/>"#.to_string(),
        ]
        .concat();
        let page = &read(&body).pages[0];
        assert_eq!(page.elements.iter().filter(|e| matches!(e.kind, ElementKind::Shape(_))).count(), 1);
        assert!(texts(page)[0].content.paragraphs[0].runs.iter().all(|r| !r.underline));
    }

    // Tableaux de LibreOffice (`table:table` dans un `draw:frame`).

    /// Styles de case, colonne et ligne d'un tableau d'essai : traits de 0,567 mm écrits (1 mm rendu), marges de 1,5 mm.
    const TABLE_STYLES: &str = r##"<style:style style:name="co20" style:family="table-column"><style:table-column-properties style:column-width="2cm"/></style:style>
<style:style style:name="co40" style:family="table-column"><style:table-column-properties style:column-width="4cm"/></style:style>
<style:style style:name="ro10" style:family="table-row"><style:table-row-properties style:row-height="1cm"/></style:style>
<style:style style:name="ro25" style:family="table-row"><style:table-row-properties style:row-height="2.5cm"/></style:style>
<style:style style:name="ro3" style:family="table-row"><style:table-row-properties style:row-height="3mm"/></style:style>
<style:style style:name="ce1" style:family="table-cell"><loext:graphic-properties draw:fill="none" fo:padding-top="1.5mm" fo:padding-bottom="1.5mm" fo:padding-left="1.5mm" fo:padding-right="1.5mm" draw:textarea-vertical-align="middle"/><style:paragraph-properties fo:border="0.567mm solid #1F6FB2"/></style:style>
<style:style style:name="cePink" style:family="table-cell" style:parent-style-name="ce1"><loext:graphic-properties draw:fill="solid" draw:fill-color="#ffd6d6"/></style:style>
<style:style style:name="ceOld" style:family="table-cell"><style:graphic-properties draw:fill="solid" draw:fill-color="#ffd6d6" fo:padding-top="6mm" fo:padding-bottom="6mm" fo:padding-left="6mm" fo:padding-right="6mm" draw:textarea-vertical-align="bottom"/><style:paragraph-properties fo:border="0.567mm solid #000000"/></style:style>
<style:style style:name="ceTiny" style:family="table-cell" style:parent-style-name="ce1"><loext:graphic-properties fo:padding-top="0mm" fo:padding-bottom="0mm"/><style:text-properties fo:font-size="1pt"/></style:style>
<style:style style:name="ceSides" style:family="table-cell"><style:paragraph-properties fo:border-left="1mm dashed #ff0000" fo:border-right="none" fo:border-top="0.567mm solid #1f6fb2" fo:border-bottom="0.567mm solid #1f6fb2"/></style:style>
<style:style style:name="PTiny" style:family="paragraph"><style:text-properties fo:font-size="1pt"/></style:style>
<style:style style:name="TTiny" style:family="text"><style:text-properties fo:font-size="1pt"/></style:style>"##;

    /// Un tableau à `x`, `y` (cadre de `frame_h`) : colonnes et lignes par leur style, puis les cases de chaque ligne.
    fn table_frame(frame_h: &str, cols: &[&str], rows: &[(&str, &str)]) -> String {
        let cols: String = cols.iter().map(|c| format!(r#"<table:table-column table:style-name="{c}"/>"#)).collect();
        let rows: String = rows.iter().map(|(style, cells)| format!(r#"<table:table-row table:style-name="{style}" table:default-cell-style-name="ce1">{cells}</table:table-row>"#)).collect();
        format!(r#"<draw:frame draw:style-name="gr2" svg:width="6cm" svg:height="{frame_h}" svg:x="2cm" svg:y="3cm"><table:table>{cols}{rows}</table:table><draw:image xlink:href="Pictures/TablePreview1.svm"/></draw:frame>"#)
    }

    fn cell_p(text: &str) -> String {
        format!(r#"<table:table-cell><text:p><text:span text:style-name="T1">{text}</text:span></text:p></table:table-cell>"#)
    }

    fn read_table_body(body: &str, common: &str, pictures: &[(&str, Vec<u8>)]) -> Imported {
        let pages = format!(r#"<draw:page draw:name="page1">{body}</draw:page>"#);
        let layouts = format!(r#"<style:page-layout style:name="PM1"><style:page-layout-properties {A4_LAYOUT}/></style:page-layout>"#);
        let mut pictures = pictures.to_vec();
        pictures.push(("TablePreview1.svm", b"VCLMTF apercu".to_vec()));
        read_drawing(Cursor::new(odg_with_styles(&pages, &layouts, "", &pictures, TABLE_STYLES, common)), &known()).unwrap()
    }

    fn the_table(p: &Page) -> (&Element, &TableProps) {
        let t: Vec<&Element> = p.elements.iter().filter(|e| matches!(e.kind, ElementKind::Table(_))).collect();
        assert_eq!(t.len(), 1, "{:?}", p.elements);
        let ElementKind::Table(props) = &t[0].kind else { unreachable!() };
        (t[0], props)
    }

    /// L'élément de la case qui commence en (`col`, `row`).
    fn cell_at(p: &Page, col: u8, row: u8) -> &Element {
        let (_, t) = the_table(p);
        let c = t.cells.iter().find(|c| (c.col, c.row) == (col, row)).unwrap_or_else(|| panic!("pas de case en {col},{row} : {:?}", t.cells));
        p.elements.iter().find(|e| e.id == c.element_id).unwrap()
    }

    fn cell_props(p: &Page, col: u8, row: u8) -> &TextProps {
        match &cell_at(p, col, row).kind {
            ElementKind::Text(t) => t,
            k => panic!("case {col},{row} : {k:?}"),
        }
    }

    /// Ce que l'interface attend d'un tableau : ses cases existent, ne sont pas verrouillées (Tab passe d'une case texte
    /// à la suivante), sont du groupe du tableau (et lui seul), avant lui dans la page, couvrent la grille une fois
    /// chacune, déjà placées par `table::layout` ; « ajouter une ligne » fonctionne.
    fn assert_real_table(p: &Page) {
        let (t, props) = the_table(p);
        let group = t.group.clone().expect("groupe");
        let members: Vec<&Element> = p.elements.iter().filter(|e| e.group.as_ref() == Some(&group)).collect();
        assert_eq!(members.len(), props.cells.len() + 1, "le groupe : le tableau et ses cases");
        let table_at = p.elements.iter().position(|e| e.id == t.id).unwrap();
        let mut seen = vec![vec![0; props.cols.len()]; props.rows.len()];
        for c in &props.cells {
            let at = p.elements.iter().position(|e| e.id == c.element_id).unwrap_or_else(|| panic!("case sans élément : {c:?}"));
            assert!(at < table_at, "le tableau par-dessus ses cases");
            assert!(!p.elements[at].locked && !t.locked);
            assert_eq!(p.elements[at].group.as_ref(), Some(&group));
            for row in &mut seen[c.row as usize..(c.row + c.row_span) as usize] {
                for v in &mut row[c.col as usize..(c.col + c.col_span) as usize] {
                    *v += 1;
                }
            }
        }
        assert!(seen.iter().flatten().all(|v| *v == 1), "grille couverte une fois : {seen:?}");
        let (w, h): (f64, f64) = (props.cols.iter().sum(), props.rows.iter().sum());
        assert!((t.frame.w_mm - w).abs() < 1e-6 && (t.frame.h_mm - h).abs() < 1e-6, "{:?} {w} {h}", t.frame);
        let mut laid = p.clone();
        crate::table::layout(&mut laid, &t.id);
        assert_eq!(laid.elements.iter().map(|e| e.frame).collect::<Vec<_>>(), p.elements.iter().map(|e| e.frame).collect::<Vec<_>>(), "déjà placées");
        let first = props.cells.iter().min_by_key(|c| (c.row, c.col)).unwrap().element_id.clone();
        crate::table::insert_row(&mut laid, &first, true).unwrap();
        let ElementKind::Table(after) = &laid.elements.iter().find(|e| e.id == t.id).unwrap().kind else { unreachable!() };
        assert_eq!(after.rows.len(), props.rows.len() + 1);
    }

    fn frame_close(f: Frame, x: f64, y: f64, w: f64, h: f64) -> bool {
        [(f.x_mm, x), (f.y_mm, y), (f.w_mm, w), (f.h_mm, h)].iter().all(|(a, b)| (a - b).abs() < 0.01)
    }

    #[test]
    fn a_libreoffice_table_becomes_a_cahutin_table() {
        let body = table_frame("3.5cm", &["co20", "co40"], &[("ro10", &(cell_p("Lundi") + &cell_p("Mardi"))), ("ro25", &(cell_p("Jeudi") + &cell_p("Vendredi")))]);
        let imported = read_table_body(&body, "", &[]);
        let p = &imported.pages[0];
        assert_real_table(p);
        let (t, props) = the_table(p);
        assert!(frame_close(t.frame, 20.0, 30.0, 60.0, 35.0), "{:?}", t.frame);
        assert_eq!((props.cols.clone(), props.rows.clone()), (vec![20.0, 40.0], vec![10.0, 25.0]));
        let texts: Vec<String> = [(0, 0), (1, 0), (0, 1), (1, 1)].iter().map(|&(c, r)| plain(cell_props(p, c, r))).collect();
        assert_eq!(texts, ["Lundi", "Mardi", "Jeudi", "Vendredi"]);
        assert!(frame_close(cell_at(p, 1, 1).frame, 40.0, 40.0, 40.0, 25.0), "{:?}", cell_at(p, 1, 1).frame);
        let c = cell_props(p, 0, 0);
        assert_eq!((c.style.font.as_str(), c.style.size_pt, c.style.valign, c.style.padding_mm), ("Liberation Serif", 12.0, Some(VAlign::Middle), Some(1.5)));
        assert_eq!(c.frame, None, "sans fond, pas de cadre");
        // L'aperçu .svm n'est pas une image ; rien ne manque.
        assert_eq!((imported.report.texts, imported.report.images, imported.report.skipped), (4, 0, 0));
        assert!(imported.assets.is_empty());
    }

    /// Fusions écrites par LibreOffice (relevé de `uno.odg`) : en largeur sur la première ligne, en hauteur à droite.
    #[test]
    fn merged_cells_keep_their_spans() {
        let covered = r#"<table:covered-table-cell/>"#;
        let body = table_frame(
            "4.5cm",
            &["co20", "co20", "co20"],
            &[
                ("ro10", &(r#"<table:table-cell table:number-columns-spanned="2"><text:p>Alpha</text:p></table:table-cell>"#.to_string() + covered + &cell_p("Bravo"))),
                ("ro25", &(cell_p("Delta") + &cell_p("Echo") + r#"<table:table-cell table:number-rows-spanned="2"><text:p>Golf</text:p></table:table-cell>"#)),
                ("ro10", &(cell_p("Hotel") + &cell_p("Kilo") + covered)),
            ],
        );
        let p = &read_table_body(&body, "", &[]).pages[0];
        assert_real_table(p);
        let (_, props) = the_table(p);
        let spans: Vec<(u8, u8, u8, u8, String)> = props
            .cells
            .iter()
            .map(|c| (c.col, c.row, c.col_span, c.row_span, plain(cell_props(p, c.col, c.row))))
            .collect();
        let want = [(0, 0, 2, 1, "Alpha"), (2, 0, 1, 1, "Bravo"), (0, 1, 1, 1, "Delta"), (1, 1, 1, 1, "Echo"), (2, 1, 1, 2, "Golf"), (0, 2, 1, 1, "Hotel"), (1, 2, 1, 1, "Kilo")];
        assert_eq!(spans, want.map(|(c, r, cs, rs, t)| (c, r, cs, rs, t.to_string())));
        assert!(frame_close(cell_at(p, 2, 1).frame, 60.0, 40.0, 20.0, 35.0), "{:?}", cell_at(p, 2, 1).frame);
    }

    /// LibreOffice écrit la hauteur demandée de chaque ligne et le cadre à la hauteur rendue : un cadre plus petit que
    /// ses lignes les rétrécit, mais jamais sous leur texte (une ligne de 12 pt et ses marges de 1,5 mm).
    #[test]
    fn a_frame_smaller_than_its_rows_shrinks_them_down_to_their_text() {
        let body = table_frame("1cm", &["co20"], &[("ro25", &cell_p("Grand")), ("ro25", &cell_p("Aussi"))]);
        let p = &read_table_body(&body, "", &[]).pages[0];
        assert_real_table(p);
        let need = 3.0 + 12.0 * CELL_LINE_EM * 25.4 / 72.0;
        assert!(frame_close(the_table(p).0.frame, 20.0, 30.0, 20.0, 2.0 * need), "{:?}", the_table(p).0.frame);
        assert!(frame_close(cell_at(p, 0, 1).frame, 20.0, 30.0 + need, 20.0, need), "{:?}", cell_at(p, 0, 1).frame);
    }

    /// Le pictogramme d'une case se retrouve au nom de son image de remplissage, que LibreOffice garde (en écrivant `_`
    /// `_5f_`) quand il renomme le fichier.
    #[test]
    fn a_picto_cell_fill_is_recognized_by_its_name() {
        let (p, h, v) = openmoji_cell_fill("case_5f_openmoji-1F98A-miroir-h").unwrap();
        assert_eq!((p.code.as_str(), p.outline, h, v), ("1F98A", None, true, false));
        assert_eq!(openmoji_cell_fill("case_openmoji-contour-1F98A").map(|x| x.0.outline), Some(Some(true)));
        assert!(openmoji_cell_fill("img_5f_abc").is_none() && openmoji_cell_fill("case_inconnu").is_none());
    }

    /// Une ligne ajoutée dans LibreOffice : hauteur écrite nulle, style de case par défaut `standard` (un style de
    /// dessin : fond vert, marges de 1,25 mm en haut et en bas, 18 pt). Ses cases en prennent l'allure, et la ligne la
    /// hauteur de son texte, le reste du cadre aux autres lignes.
    #[test]
    fn a_row_added_in_libreoffice_takes_its_default_style_and_its_text_height() {
        let standard = r##"<style:style style:name="standard" style:family="graphic"><style:graphic-properties draw:fill="solid" draw:fill-color="#18a303" fo:padding-top="0.125cm" fo:padding-bottom="0.125cm" fo:padding-left="0.25cm" fo:padding-right="0.25cm"/><style:text-properties style:font-name="Liberation Sans" fo:font-size="18pt"/></style:style><style:style style:name="ro0" style:family="table-row"><style:table-row-properties style:row-height="0cm"/></style:style>"##;
        let need = 2.5 + 18.0 * CELL_LINE_EM * 25.4 / 72.0;
        let body = format!(
            r#"<draw:frame draw:style-name="gr2" svg:width="2cm" svg:height="{}mm" svg:x="2cm" svg:y="3cm"><table:table><table:table-column table:style-name="co20"/><table:table-row table:style-name="ro10" table:default-cell-style-name="ce1">{}</table:table-row><table:table-row table:style-name="ro0" table:default-cell-style-name="standard"><table:table-cell><text:p>Ajout</text:p></table:table-cell></table:table-row></table:table></draw:frame>"#,
            10.0 + need,
            cell_p("Haut")
        );
        let p = &read_table_body(&body, standard, &[]).pages[0];
        assert_real_table(p);
        let (t, props) = the_table(p);
        assert!((props.rows[1] - need).abs() < 0.02 && (props.rows[0] - 10.0).abs() < 0.02 && (t.frame.h_mm - 10.0 - need).abs() < 0.02, "{:?}", props.rows);
        let added = cell_props(p, 0, 1);
        assert_eq!((added.style.size_pt, added.frame.as_ref().and_then(|f| f.fill.as_deref())), (18.0, Some("#18a303")));
    }

    /// Lignes et cadre qui se contredisent : comme LibreOffice, chaque ligne agrandie à son contenu, puis la différence
    /// avec le cadre répartie au prorata, sans descendre sous le contenu. Les mesures du poste (voir `fit_rows`).
    #[test]
    fn rows_are_fitted_to_the_frame_as_libreoffice_does() {
        let near = |got: Vec<f64>, want: &[f64]| got.len() == want.len() && got.iter().zip(want).all(|(g, w)| (g - w).abs() < 0.02);
        let fit = |w: &[f64], m: &[f64], f: f64| fit_rows(w, m, f);
        assert!(near(fit(&[10.0, 20.0], &[0.5, 0.5], 50.0), &[16.67, 33.33]));
        assert!(near(fit(&[10.0, 20.0], &[0.5, 0.5], 20.0), &[6.67, 13.33]));
        assert!(near(fit(&[20.0, 20.0], &[36.65, 0.5], 80.0), &[51.76, 28.24]));
        assert!(near(fit(&[20.0, 20.0], &[36.65, 0.5], 58.05), &[37.56, 20.49]));
        assert!(near(fit(&[20.0, 20.0], &[36.65, 0.5], 40.0), &[36.65, 3.35]));
        assert!(near(fit(&[10.0, 20.0], &[9.73, 9.75], 20.0), &[9.73, 9.75]), "{:?}", fit(&[10.0, 20.0], &[9.73, 9.75], 20.0));
        assert!(near(fit(&[10.0, 20.0, 30.0], &[0.0; 3], 90.0), &[15.0, 30.0, 45.0]));
        // Lignes de 1 + 1 cm aux cases vides dans un cadre de 5 cm : 2,5 + 2,5 cm ; le tableau à la hauteur du cadre.
        let tiny = "<table:table-cell table:style-name=\"ceTiny\"><text:p text:style-name=\"PTiny\"><text:span text:style-name=\"TTiny\">\u{200B}</text:span></text:p></table:table-cell>";
        let body = table_frame("5cm", &["co20", "co40"], &[("ro10", &tiny.repeat(2)), ("ro10", &tiny.repeat(2))]);
        let p = &read_table_body(&body, "", &[]).pages[0];
        let (t, props) = the_table(p);
        assert!((t.frame.h_mm - 50.0).abs() < 0.01 && near(props.rows.clone(), &[25.0, 25.0]), "{:?} {:?}", t.frame, props.rows);
        // La première ligne demande cinq lignes de texte (12 pt, marges de 1,5 mm) : elle les garde, la seconde a le reste
        // à sa part.
        let five = format!(r#"<table:table-cell table:style-name="ce1">{}</table:table-cell>"#, ["Un", "Deux", "Trois", "Quatre", "Cinq"].map(|w| format!("<text:p>{w}</text:p>")).concat());
        let body = table_frame("5cm", &["co20", "co40"], &[("ro10", &(five + tiny)), ("ro10", &tiny.repeat(2))]);
        let p = &read_table_body(&body, "", &[]).pages[0];
        let (t, props) = the_table(p);
        let need = 3.0 + 5.0 * 12.0 * CELL_LINE_EM * 25.4 / 72.0;
        assert!((t.frame.h_mm - 50.0).abs() < 0.02, "{:?}", t.frame);
        assert!(near(props.rows.clone(), &[50.0 * need / (need + 10.0), 50.0 * 10.0 / (need + 10.0)]), "{:?}, contenu {need:.2}", props.rows);
        assert_real_table(p);
    }

    #[test]
    fn a_coloured_cell_fills_its_text_frame() {
        let pink = r#"<table:table-cell table:style-name="cePink"><text:p>Rose</text:p></table:table-cell>"#;
        let body = table_frame("1cm", &["co20", "co20"], &[("ro10", &(pink.to_string() + &cell_p("Blanc")))]);
        let p = &read_table_body(&body, "", &[]).pages[0];
        assert_eq!(cell_props(p, 0, 0).frame.as_ref().and_then(|f| f.fill.as_deref()), Some("#ffd6d6"));
        // La marge et l'alignement vertical viennent du style parent.
        assert_eq!((cell_props(p, 0, 0).style.padding_mm, cell_props(p, 0, 0).style.valign), (Some(1.5), Some(VAlign::Middle)));
        assert_eq!(cell_props(p, 1, 0).frame, None);
    }

    #[test]
    fn bullets_in_a_cell_stay_bullets() {
        let list = r#"<table:table-cell><text:list text:style-name="LB"><text:list-item><text:p>Colle</text:p></text:list-item><text:list-item><text:p>Ciseaux</text:p></text:list-item></text:list><text:p>Fin</text:p></table:table-cell>"#;
        let body = table_frame("2.5cm", &["co40"], &[("ro25", list)]);
        let p = &read_table_body(&body, "", &[]).pages[0];
        let got: Vec<(String, bool)> = cell_props(p, 0, 0).content.paragraphs.iter().map(|q| (q.runs.iter().map(|r| r.text.as_str()).collect(), q.list == Some(ListKind::Bullet))).collect();
        assert_eq!(got, [("Colle".to_string(), true), ("Ciseaux".to_string(), true), ("Fin".to_string(), false)]);
    }

    /// Lignes basses de LibreOffice : une case vide tient sa ligne de 3 mm par une espace sans largeur en 1 pt. Elle
    /// redevient une case vide, à la taille de la case écrite (le texte qu'on y tapera ne sera pas minuscule).
    #[test]
    fn an_empty_cell_with_a_zero_width_space_is_empty() {
        let tiny = "<table:table-cell table:style-name=\"ceTiny\"><text:p text:style-name=\"PTiny\"><text:span text:style-name=\"TTiny\">\u{200B}</text:span></text:p></table:table-cell>";
        let body = table_frame("0.6cm", &["co20", "co20"], &[("ro3", &(tiny.to_string() + &cell_p("Mot\u{200B}"))), ("ro3", &(tiny.to_string() + tiny))]);
        let imported = read_table_body(&body, "", &[]);
        let p = &imported.pages[0];
        assert_real_table(p);
        assert_eq!(the_table(p).1.rows, vec![3.0, 3.0]);
        assert_eq!(plain(cell_props(p, 1, 0)), "Mot");
        for (c, r) in [(0, 0), (0, 1), (1, 1)] {
            let t = cell_props(p, c, r);
            assert!(t.content.paragraphs.is_empty(), "{c},{r} : {:?}", t.content);
            assert_eq!((t.style.font.as_str(), t.style.size_pt), ("Liberation Serif", 12.0), "{c},{r}");
            assert_eq!(t.style.padding_mm, Some(1.5));
        }
    }

    /// Le trait du tableau : le plus fréquent des côtés des cases, l'épaisseur écrite fois 1,764 (ce que dessine
    /// LibreOffice) au 1/20 de mm, de 0,5 à 3 mm, plein.
    #[test]
    fn the_table_border_is_the_usual_cell_border_as_drawn() {
        let sides = r#"<table:table-cell table:style-name="ceSides"><text:p>A</text:p></table:table-cell>"#;
        let body = table_frame("1cm", &["co20", "co20", "co20"], &[("ro10", &(cell_p("B") + sides + &cell_p("C")))]);
        let p = &read_table_body(&body, "", &[]).pages[0];
        assert_eq!(the_table(p).1.border, Some(Stroke { width_mm: 1.0, color: "#1f6fb2".into(), style: None }));
        let borders = |sides: &[Option<CellBorder>]| table_border(sides).map(|s| s.width_mm);
        let b = |w: f64| Some(CellBorder { width_mm: w, color: "#000000".into() });
        assert_eq!(borders(&[b(0.0988), b(0.0988)]), Some(0.5), "0,28 pt : le plus fin");
        assert_eq!(borders(&[b(1.0), b(1.0)]), Some(1.75), "1 mm écrit, 1,76 mm dessiné");
        assert_eq!(borders(&[b(0.34)]), Some(0.6), "0,6 mm exporté par Cahutin");
        assert_eq!(borders(&[b(5.0)]), Some(3.0));
        assert_eq!(borders(&[None, None, b(1.0)]), None, "la plupart des côtés sans trait");
        assert_eq!(cell_border("none"), None);
        assert_eq!(cell_border("0.28pt solid #FF0000"), Some(CellBorder { width_mm: 0.28 * 25.4 / 72.0, color: "#ff0000".into() }));
    }

    /// Fichiers anciens : fond, marge et alignement vertical de la case dans `style:graphic-properties`.
    #[test]
    fn the_old_cell_graphic_properties_are_read() {
        let old = r#"<table:table-cell table:style-name="ceOld"><text:p>Ancien</text:p></table:table-cell>"#;
        let body = table_frame("2.5cm", &["co40"], &[("ro25", old)]);
        let p = &read_table_body(&body, "", &[]).pages[0];
        let t = cell_props(p, 0, 0);
        assert_eq!((t.frame.as_ref().and_then(|f| f.fill.as_deref()), t.style.padding_mm, t.style.valign), (Some("#ffd6d6"), Some(6.0), Some(VAlign::Bottom)));
    }

    /// Un tableau fait par l'interface de LibreOffice 26.8 (relevé de `ui_table.odg`, un mot tapé dans la première case) :
    /// modèle `default`, styles de case dans `styles.xml`. `bg-none` (sans fond, trait noir de 0,28 pt) l'emporte sur son
    /// parent `default` (fond blanc, trait blanc) ; marge de 1 mm et texte en Liberation Sans 18 pt viennent de `default`.
    #[test]
    fn a_table_made_in_the_libreoffice_interface_is_read() {
        let common = r##"<style:style style:name="bg-none" style:family="table-cell" style:parent-style-name="default"><loext:graphic-properties draw:fill="none"/><style:paragraph-properties fo:border="0.28pt solid #000000"/></style:style><style:style style:name="default" style:family="table-cell"><loext:graphic-properties draw:fill="solid" draw:fill-color="#ffffff" draw:textarea-vertical-align="top" fo:padding-top="0.1cm" fo:padding-bottom="0.1cm" fo:padding-left="0.1cm" fo:padding-right="0.1cm"/><style:paragraph-properties fo:border="0.28pt solid #ffffff"/><style:text-properties style:use-window-font-color="true" loext:opacity="0%" style:font-name="Liberation Sans" fo:font-family="&apos;Liberation Sans&apos;" style:font-family-generic="roman" style:font-pitch="variable" fo:font-size="18pt" style:letter-kerning="true"/></style:style><table:table-template table:name="default"><table:first-row table:style-name="bg-none"/><table:last-row table:style-name="bg-none"/><table:first-column table:style-name="bg-none"/><table:last-column table:style-name="bg-none"/><table:body table:style-name="bg-none"/><table:odd-rows table:style-name="bg-none"/><table:odd-columns table:style-name="bg-none"/></table:table-template>"##;
        let auto = r#"<style:style style:name="ui-co1" style:family="table-column"><style:table-column-properties style:column-width="4.699cm" style:use-optimal-column-width="false"/></style:style><style:style style:name="ui-co2" style:family="table-column"><style:table-column-properties style:column-width="4.701cm" style:use-optimal-column-width="false"/></style:style><style:style style:name="ui-ro1" style:family="table-row"><style:table-row-properties style:row-height="0.962cm" style:use-optimal-row-height="false"/></style:style>"#;
        let body = r#"<draw:frame draw:style-name="gr2" draw:layer="layout" svg:width="14.098cm" svg:height="1.923cm" svg:x="3.778cm" svg:y="15.185cm"><table:table table:template-name="default" table:use-first-row-styles="true" table:use-banding-rows-styles="true"><table:table-column table:style-name="ui-co1"/><table:table-column table:style-name="ui-co1"/><table:table-column table:style-name="ui-co2"/><table:table-row table:style-name="ui-ro1" table:default-cell-style-name="bg-none"><table:table-cell><text:p>Lundi</text:p></table:table-cell><table:table-cell/><table:table-cell/></table:table-row><table:table-row table:style-name="ui-ro1" table:default-cell-style-name="bg-none"><table:table-cell/><table:table-cell/><table:table-cell/></table:table-row></table:table><draw:image xlink:href="Pictures/TablePreview1.svm" xlink:type="simple" xlink:show="embed" xlink:actuate="onLoad"/></draw:frame>"#;
        let pages = format!(r#"<draw:page draw:name="page1" draw:master-page-name="Default">{body}</draw:page>"#);
        let layouts = format!(r#"<style:page-layout style:name="PM1"><style:page-layout-properties {A4_LAYOUT}/></style:page-layout>"#);
        let odg = odg_with_styles(&pages, &layouts, "", &[("TablePreview1.svm", b"VCLMTF".to_vec())], auto, common);
        let imported = read_drawing(Cursor::new(odg), &known()).unwrap();
        let p = &imported.pages[0];
        assert_real_table(p);
        let (t, props) = the_table(p);
        assert!(frame_close(t.frame, 37.78, 151.85, 140.99, 19.24), "{:?}", t.frame);
        assert_eq!(props.border, Some(Stroke { width_mm: 0.5, color: "#000000".into(), style: None }));
        let first = cell_props(p, 0, 0);
        assert_eq!(plain(first), "Lundi");
        assert_eq!((first.style.font.as_str(), first.style.size_pt, first.style.padding_mm, first.style.valign), ("Liberation Sans", 18.0, Some(1.0), Some(VAlign::Top)));
        assert_eq!(first.frame, None, "bg-none : pas de fond blanc");
        assert!(cell_props(p, 2, 1).content.paragraphs.is_empty());
        assert_eq!((imported.report.texts, imported.report.images, imported.report.skipped), (6, 0, 0));
    }

    /// Image de fond d'une case vide, non étirée : le pictogramme OpenMoji ou la photo de la case. Étirée, ou derrière
    /// un texte : non reprise, comptée.
    #[test]
    fn a_cell_background_picture_becomes_the_cell_picture_when_simple() {
        let fills = r#"<draw:fill-image draw:name="picto" xlink:href="Pictures/openmoji-1F98A.svg"/><draw:fill-image draw:name="photo" xlink:href="Pictures/photo.png"/>"#;
        let auto = r#"<style:style style:name="cePicto" style:family="table-cell" style:parent-style-name="ce1"><loext:graphic-properties draw:fill="bitmap" draw:fill-image-name="picto" style:repeat="no-repeat"/></style:style><style:style style:name="cePhoto" style:family="table-cell" style:parent-style-name="ce1"><loext:graphic-properties draw:fill="bitmap" draw:fill-image-name="photo" style:repeat="no-repeat"/></style:style><style:style style:name="ceStretch" style:family="table-cell" style:parent-style-name="ce1"><loext:graphic-properties draw:fill="bitmap" draw:fill-image-name="photo" style:repeat="stretch"/></style:style>"#;
        let cell = |style: &str, text: &str| format!(r#"<table:table-cell table:style-name="{style}">{text}</table:table-cell>"#);
        let row = [cell("cePicto", ""), cell("cePhoto", ""), cell("ceStretch", ""), cell("cePhoto", "<text:p>Texte</text:p>")].concat();
        let body = table_frame("2.5cm", &["co40", "co40", "co40", "co40"], &[("ro25", &row)]);
        let pages = format!(r#"<draw:page draw:name="page1">{body}</draw:page>"#);
        let layouts = format!(r#"<style:page-layout style:name="PM1"><style:page-layout-properties {A4_LAYOUT}/></style:page-layout>"#);
        let svg = crate::picto::svg("1F98A").unwrap();
        let odg = odg_with_styles(&pages, &layouts, "", &[("photo.png", png()), ("openmoji-1F98A.svg", svg)], &format!("{TABLE_STYLES}{auto}"), fills);
        let imported = read_drawing(Cursor::new(odg), &known()).unwrap();
        let p = &imported.pages[0];
        assert_real_table(p);
        assert!(matches!(&cell_at(p, 0, 0).kind, ElementKind::Image(i) if i.code == "1F98A"), "{:?}", cell_at(p, 0, 0).kind);
        assert!(matches!(&cell_at(p, 1, 0).kind, ElementKind::Photo(ph) if ph.asset.as_deref() == Some(imported.assets[0].meta.id.as_str())));
        assert!(cell_props(p, 2, 0).content.paragraphs.is_empty());
        assert_eq!(plain(cell_props(p, 3, 0)), "Texte");
        assert_eq!((imported.assets.len(), imported.report.images, imported.report.skipped), (1, 2, 2));
    }

    /// Une image de fond de case qui ne se décode pas : la case reste, photo vide, et l'image compte comme non reprise.
    #[test]
    fn an_undecodable_cell_picture_leaves_an_empty_photo_cell() {
        let fills = r#"<draw:fill-image draw:name="photo" xlink:href="Pictures/abimee.png"/>"#;
        let auto = r#"<style:style style:name="cePhoto" style:family="table-cell" style:parent-style-name="ce1"><loext:graphic-properties draw:fill="bitmap" draw:fill-image-name="photo" style:repeat="no-repeat"/></style:style>"#;
        let body = table_frame("2.5cm", &["co40", "co40"], &[("ro25", &(r#"<table:table-cell table:style-name="cePhoto"/>"#.to_string() + &cell_p("Mot")))]);
        let pages = format!(r#"<draw:page draw:name="page1">{body}</draw:page>"#);
        let layouts = format!(r#"<style:page-layout style:name="PM1"><style:page-layout-properties {A4_LAYOUT}/></style:page-layout>"#);
        let broken = [b"\x89PNG\r\n\x1a\n".as_slice(), b"pas une image"].concat();
        let odg = odg_with_styles(&pages, &layouts, "", &[("abimee.png", broken)], &format!("{TABLE_STYLES}{auto}"), fills);
        let imported = read_drawing(Cursor::new(odg), &known()).unwrap();
        let p = &imported.pages[0];
        assert_real_table(p);
        assert!(matches!(&cell_at(p, 0, 0).kind, ElementKind::Photo(ph) if ph.asset.is_none()), "{:?}", cell_at(p, 0, 0).kind);
        assert_eq!((imported.assets.len(), imported.report.skipped), (0, 1));
    }

    #[test]
    fn report_counts_everything() {
        let body = text_frame("2cm", "2cm", "5cm", r#"<text:span text:style-name="T1">Un</text:span>"#)
            + r#"<draw:polygon draw:style-name="gr1" svg:width="2cm" svg:height="2cm" svg:x="2cm" svg:y="9cm" svg:viewBox="0 0 2000 2000" draw:points="0,0 2000,0 2000,2000 0,2000"/>"#
            + r#"<draw:path draw:style-name="gr1" svg:width="2cm" svg:height="2cm" svg:x="5cm" svg:y="2cm" svg:viewBox="0 0 2000 2000" svg:d="M 0 0 L 2000 2000"/>"#
            + r#"<draw:frame svg:width="2cm" svg:height="2cm" svg:x="9cm" svg:y="9cm"><draw:image xlink:href="Pictures/img.png"/></draw:frame>"#;
        let imported = read_odg(Cursor::new(odg_fixture(&body, A4_LAYOUT, &[("img.png", png())])), &known()).unwrap();
        assert_eq!(imported.report, ImportReport { pages: 1, texts: 1, images: 1, shapes: 2, skipped: 0, ready_made: 0, drawn: 0, shrunk: 0, fallback: false });
    }
}
