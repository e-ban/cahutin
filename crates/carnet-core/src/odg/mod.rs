//! Export ODG (LibreOffice Draw) : chaque élément devient un objet natif, les polices sont intégrées.

mod content;
pub(crate) use content::{PICTO_CELL_FILL, PICTO_MARK, TURNED_FILL, picto_file, turned_box_mm, turned_image};
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

pub(crate) const NS: [(&str, &str); 12] = [
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
    ("xmlns:table", "urn:oasis:names:tc:opendocument:xmlns:table:1.0"),
];

/// LibreOffice dessine un trait de case de tableau 1,764 fois plus épais que l'épaisseur écrite (0,567 mm écrit :
/// 1 mm rendu ; mesuré en PDF, en image et par UNO sur LibreOffice 26.8 et 24.2, gardé par la sonde du banc
/// `libreoffice_tables_behave_as_measured`). L'export écrit donc l'épaisseur voulue divisée par ce facteur (× 0,5669),
/// l'import multiplie l'épaisseur lue.
pub(crate) const CELL_BORDER_DRAWN: f64 = 1.764;

/// Mention de licence des images OpenMoji (propriétés du fichier, écran À propos).
pub const OPENMOJI_CREDIT: &str = "Pictogrammes : OpenMoji (openmoji.org), licence CC BY-SA 4.0";
/// Mention écrite en bas de chaque page qui contient une image OpenMoji.
pub const OPENMOJI_CREDIT_PAGE: &str = "Pictogrammes : OpenMoji, CC BY-SA 4.0";
/// Marque écrite après la version dans le générateur de `meta.xml` : une forme retournée d'un seul sens y porte
/// l'angle que LibreOffice attend (l'opposé de celui de l'écran, voir `content`). Un fichier de Cahutin sans elle
/// (version 0.17.0 et avant) porte l'angle de l'écran ; l'import le relit tel quel.
pub const MIRROR_AFTER_ROTATION: &str = "miroir-apres-rotation";
/// Propriété du fichier (`meta:user-defined`) qui dit qu'il vient de Cahutin. LibreOffice réécrit le générateur en
/// réenregistrant, mais garde cette propriété : l'import y reconnaît un fichier de Cahutin retouché.
pub const CAHUTIN_MARK_NAME: &str = "Logiciel";
pub const CAHUTIN_MARK: &str = "Cahutin";

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
        Asset { meta: AssetMeta { id: new_id(), width_px: w, height_px: h, mime: Mime::Jpeg, taken: None }, thumb: bytes.clone(), bytes }
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
                mirrored: false,
            }),
            subgroups: Vec::new(),
            flip_h: false, flip_v: false,
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
            subgroups: Vec::new(),
            flip_h: false, flip_v: false,
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
            self.zip.file_names().map(|n| n.unwrap().into_owned()).collect()
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

    /// Un ODG exporté par Cahutin se relit tel quel : mêmes textes (cursive comprise), mêmes images de la
    /// banque, mêmes rotations.
    #[test]
    fn every_example_comes_back_from_its_odg() {
        let fonts = FontSet::builtin();
        let known: Vec<String> = fonts.families().into_iter().map(|f| f.name).collect();
        let texts = |pages: &[Page]| -> Vec<String> {
            let mut v: Vec<String> = pages
                .iter()
                .flat_map(|p| &p.elements)
                .filter_map(|e| match &e.kind {
                    ElementKind::Text(t) => Some(t.content.paragraphs.iter().map(|p| p.runs.iter().map(|r| r.text.as_str()).collect::<String>()).collect::<Vec<_>>().join("\n")),
                    _ => None,
                })
                .filter(|t| !t.trim().is_empty())
                .collect();
            v.sort();
            v
        };
        let pictos = |pages: &[Page]| -> Vec<(String, i64)> {
            let mut v: Vec<(String, i64)> = pages.iter().flat_map(|p| &p.elements).filter_map(|e| if let ElementKind::Image(i) = &e.kind { Some((i.code.clone(), e.frame.rotation_deg.round() as i64)) } else { None }).collect();
            v.sort();
            v
        };
        let mut unexpected: Vec<String> = Vec::new();
        // Les exemples, puis les modèles fournis (leurs photos d'exemple comprises).
        let mut docs: Vec<(String, Document, AssetStore)> = crate::examples::list().into_iter().map(|i| (i.id.clone(), crate::examples::document(&i.id).unwrap(), AssetStore::default())).collect();
        for t in crate::templates::builtin() {
            let (page, assets) = crate::templates::instantiate(&t);
            let mut d = Document::new(&t.info.name);
            d.pages = vec![page];
            let mut store = AssetStore::default();
            for a in assets {
                d.assets.insert(a.meta.id.clone(), a.meta.clone());
                store.insert(a).unwrap();
            }
            docs.push((format!("modèle {}", t.info.id), d, store));
        }
        for (id, doc, store) in &docs {
            let mut buf = Cursor::new(Vec::new());
            export_odg(doc, store, &fonts, &mut buf).unwrap();
            buf.set_position(0);
            let back = crate::import::odg::read_drawing(buf, &known).unwrap();
            assert_eq!(back.report.skipped, 0, "{} : {:?}", id, back.report);
            // Un champ non rempli s'exporte sous son nom (« Âge ») à l'encre transparente, et revient champ.
            let shown = |t: &String| t.clone();
            let (before, after) = (texts(&doc.pages), texts(&back.pages));
            for t in &before {
                assert!(after.contains(t), "{} : {t:?} perdu, relu {after:?}", id);
            }
            assert_eq!(pictos(&back.pages), pictos(&doc.pages), "{}", id);
            // Le crédit OpenMoji ajouté par l'export ne revient pas.
            assert!(!after.iter().any(|t| t.contains("OpenMoji")) || before.iter().any(|t| t.contains("OpenMoji")), "{} : {after:?}", id);
            for (p, q) in doc.pages.iter().zip(&back.pages) {
                assert_eq!(p.background.as_ref().map(|b| &b.color), q.background.as_ref().map(|b| &b.color), "{} : fond de page", id);
            }
            // Chaque texte garde ses alignements, ses lettres creuses, sa police, sa taille et le fond de son cadre.
            let styled = |pages: &[Page]| -> Vec<String> {
                let mut v: Vec<String> = pages
                    .iter()
                    .flat_map(|p| &p.elements)
                    .filter_map(|e| match &e.kind {
                        ElementKind::Text(t) if !t.content.is_blank() => Some(format!(
                            "{:?} {:?} {:?} {:?} {} {} {:?}",
                            t.content.paragraphs.iter().map(|p| shown(&p.runs.iter().map(|r| r.text.as_str()).collect::<String>())).collect::<Vec<_>>(),
                            t.style.align,
                            t.style.valign.unwrap_or_default(),
                            t.style.outline.unwrap_or(false),
                            t.style.font,
                            t.style.size_pt,
                            t.frame.as_ref().and_then(|f| f.fill.clone())
                        )),
                        _ => None,
                    })
                    .filter(|s| !s.contains("OpenMoji"))
                    .collect();
                v.sort();
                v
            };
            assert_eq!(styled(&back.pages), styled(&doc.pages), "{}", id);
            for (p, q) in doc.pages.iter().zip(&back.pages) {
                for (cat, detail) in structural_diff(p, q) {
                    if !STILL_LOST.iter().any(|(c, _)| *c == cat) {
                        unexpected.push(format!("{} : {cat} : {detail}", id));
                    }
                }
            }
        }
        assert!(unexpected.is_empty(), "{} écart(s) :\n{}", unexpected.len(), unexpected.join("\n"));
    }

    /// Ce qu'un exemple perd encore en passant par son ODG, catégorie par catégorie (voir `structural_diff`).
    /// `structural_diff` ne compare pas non plus, et ils ne reviennent pas : la marque de zone d'un élément (`zone`), le
    /// recadrage d'une photo et son miroir (les pixels visibles partent prérecadrés et retournés : la photo revient
    /// entière, recadrage au centre), la hauteur du cadre d'un trait ou d'une flèche (non écrite), les tailles des
    /// colonnes et des lignes d'un tableau et le contenu de ses cases (voir `a_real_table_comes_back_as_the_same_table`),
    /// le titre du document et ses styles nommés. Un tableau qui n'est pas simple revient en cases et traits groupés.
    const STILL_LOST: &[(&str, &str)] = &[
        ("Photo vide perdue", "une zone de photo vide n'est pas exportée (rien à imprimer, pas de zones de modèle dans l'ODG)"),
        ("Texte vide perdu", "une zone de texte vide sans cadre n'est pas exportée (l'invite ne l'est jamais)"),
        ("Texte.invite", "l'invite d'une zone de texte (le mot gris qui dit quoi écrire) n'est jamais exportée : elle s'imprimerait"),
        ("Page.grille", "la grille de mise en page (cases de photos et de textes) n'a pas d'équivalent dans LibreOffice : les cases reviennent, la grille non"),
    ];

    /// Écarts entre une page et sa relecture, champ par champ (dans l'esprit du diff du banc d'aller-retour) : les
    /// éléments sont appariés par nature et par centre du cadre (au plus près, à moins de 3 mm), les identifiants
    /// ignorés. Chaque écart : sa catégorie (`Texte.interligne`, `élément perdu`...) et le détail.
    fn structural_diff(before: &Page, after: &Page) -> Vec<(String, String)> {
        let mut out: Vec<(String, String)> = Vec::new();
        let kind = |e: &Element| match &e.kind {
            ElementKind::Photo(_) => "Photo",
            ElementKind::Text(_) => "Texte",
            ElementKind::Shape(_) => "Forme",
            ElementKind::Image(_) => "Pictogramme",
            ElementKind::Table(_) => "Tableau",
        };
        let centre = |e: &Element| (e.frame.x_mm + e.frame.w_mm / 2.0, e.frame.y_mm + e.frame.h_mm / 2.0);
        let mut pairs: Vec<(f64, usize, usize)> = Vec::new();
        for (i, a) in before.elements.iter().enumerate() {
            for (j, b) in after.elements.iter().enumerate() {
                let ((x1, y1), (x2, y2)) = (centre(a), centre(b));
                let d = (x1 - x2).hypot(y1 - y2);
                if kind(a) == kind(b) && d < 3.0 {
                    pairs.push((d, i, j));
                }
            }
        }
        pairs.sort_by(|a, b| a.0.total_cmp(&b.0));
        let (mut of_before, mut of_after) = (vec![None; before.elements.len()], vec![None; after.elements.len()]);
        for (_, i, j) in pairs {
            if of_before[i].is_none() && of_after[j].is_none() {
                of_before[i] = Some(j);
                of_after[j] = Some(i);
            }
        }
        let mut push = |cat: &str, detail: String| out.push((cat.to_string(), detail));
        let near = |a: f64, b: f64| (a - b).abs() < 0.02;
        let angle = |a: f64| a.rem_euclid(360.0);
        // Les morceaux d'un texte, à leur allure effective, ceux de même allure recollés ; les champs tels quels.
        let looks = |t: &TextProps| -> Vec<(Option<ListKind>, Align, Vec<(String, bool, bool, bool, String, String, i64)>)> {
            t.content
                .paragraphs
                .iter()
                .map(|p| {
                    let mut runs: Vec<(String, bool, bool, bool, String, String, i64)> = Vec::new();
                    for r in &p.runs {
                        let look = (r.bold, r.italic, r.underline, r.color.clone().unwrap_or_else(|| t.style.color.clone()).to_lowercase(), r.font.clone().unwrap_or_else(|| t.style.font.clone()), (r.size_pt.unwrap_or(t.style.size_pt) * 10.0).round() as i64);
                        match runs.last_mut() {
                            Some(last) if (last.1, last.2, last.3, &last.4, &last.5, last.6) == (look.0, look.1, look.2, &look.3, &look.4, look.5) => last.0.push_str(&r.text),
                            _ => runs.push((r.text.clone(), look.0, look.1, look.2, look.3, look.4, look.5)),
                        }
                    }
                    runs.retain(|r| !r.0.is_empty());
                    (p.list, p.align.unwrap_or(t.style.align), runs)
                })
                .collect()
        };
        for (i, a) in before.elements.iter().enumerate() {
            let Some(j) = of_before[i] else {
                let what = match &a.kind {
                    ElementKind::Photo(p) if p.asset.is_none() => "Photo vide perdue".to_string(),
                    ElementKind::Text(t) if t.content.is_blank() && t.frame.as_ref().is_none_or(TextFrame::is_empty) => "Texte vide perdu".to_string(),
                    _ => format!("{} perdu", kind(a)),
                };
                push(&what, format!("{a:?}"));
                continue;
            };
            let b = &after.elements[j];
            let (fa, mut fb) = (a.frame, b.frame);
            // Un trait ou une flèche : sa hauteur n'est pas écrite (le trait passe au milieu du cadre), seul son milieu compte.
            if matches!(&a.kind, ElementKind::Shape(s) if crate::shapes::is_stroke_only(s.shape)) {
                fb = Frame { y_mm: fb.y_mm + (fb.h_mm - fa.h_mm) / 2.0, h_mm: fa.h_mm, ..fb };
            }
            if !(near(fa.x_mm, fb.x_mm) && near(fa.y_mm, fb.y_mm) && near(fa.w_mm, fb.w_mm) && near(fa.h_mm, fb.h_mm) && near(angle(fa.rotation_deg), angle(fb.rotation_deg))) {
                push(&format!("{}.cadre", kind(a)), format!("{fa:?} → {fb:?}"));
            }
            if (a.locked, a.flip_h, a.flip_v) != (b.locked, b.flip_h, b.flip_v) {
                push(&format!("{}.verrou ou miroir", kind(a)), format!("{:?} → {:?}", (a.locked, a.flip_h, a.flip_v), (b.locked, b.flip_h, b.flip_v)));
            }
            // Groupes : deux éléments partagent autant de niveaux de groupe avant et après.
            for (k, c) in before.elements.iter().enumerate().skip(i + 1) {
                let Some(l) = of_before[k] else { continue };
                let shared = |x: &Element, y: &Element| crate::groups::path(x).iter().zip(crate::groups::path(y)).take_while(|(p, q)| *p == q).count();
                if shared(a, c) != shared(b, &after.elements[l]) {
                    push(&format!("{}.groupe", kind(a)), format!("{} et {} : {} niveau(x) communs → {}", i, k, shared(a, c), shared(b, &after.elements[l])));
                }
            }
            match (&a.kind, &b.kind) {
                (ElementKind::Text(x), ElementKind::Text(y)) => {
                    if looks(x) != looks(y) {
                        push("Texte.contenu", format!("{:?} → {:?}", looks(x), looks(y)));
                    }
                    let s = |t: &TextProps| (t.style.align, t.style.valign.unwrap_or_default(), t.style.outline.unwrap_or(false), t.style.font.clone(), (t.style.size_pt * 10.0).round() as i64, t.style.color.to_lowercase(), (t.style.padding() * 100.0).round() as i64);
                    if s(x) != s(y) {
                        push("Texte.style", format!("{:?} → {:?}", s(x), s(y)));
                    }
                    if x.style.line_spacing.map(|v| (v * 100.0).round() as i64) != y.style.line_spacing.map(|v| (v * 100.0).round() as i64) {
                        push("Texte.interligne", format!("{:?} → {:?}", x.style.line_spacing, y.style.line_spacing));
                    }
                    if x.style.named != y.style.named {
                        push("Texte.style nommé", format!("{:?} → {:?}", x.style.named, y.style.named));
                    }
                    if x.prompt != y.prompt {
                        push("Texte.invite", format!("{:?} → {:?}", x.prompt, y.prompt));
                    }
                    let fr = |t: &TextProps| t.frame.clone().filter(|f| !f.is_empty()).map(|f| (f.fill.map(|c| c.to_lowercase()), f.stroke.map(|s| ((s.width_mm * 100.0).round() as i64, s.color.to_lowercase(), s.style.unwrap_or_default())), (f.corner_radius_mm * 100.0).round() as i64, f.image.is_some(), f.pattern, f.write_on_lines.unwrap_or(false)));
                    if fr(x) != fr(y) {
                        push("Texte.cadre", format!("{:?} → {:?}", fr(x), fr(y)));
                    }
                }
                (ElementKind::Shape(x), ElementKind::Shape(y)) => {
                    let s = |s: &ShapeProps| (s.shape, s.fill.clone().map(|c| c.to_lowercase()), s.stroke.clone().map(|s| ((s.width_mm * 100.0).round() as i64, s.color.to_lowercase(), s.style.unwrap_or_default())), (s.corner_radius_mm * 100.0).round() as i64, s.path.clone());
                    if s(x) != s(y) {
                        push("Forme", format!("{:?} → {:?}", s(x), s(y)));
                    }
                    if x.graphisme != y.graphisme {
                        push("Forme.graphisme", format!("{:?}", x.graphisme.as_ref().map(|g| &g.motif)));
                    }
                }
                (ElementKind::Image(x), ElementKind::Image(y)) if x.code != y.code || x.outline.unwrap_or(false) != y.outline.unwrap_or(false) => push("Pictogramme", format!("{x:?} → {y:?}")),
                (ElementKind::Photo(x), ElementKind::Photo(y)) => {
                    let s = |p: &PhotoProps| ((p.corner_radius_mm * 100.0).round() as i64, p.border.clone().map(|s| ((s.width_mm * 100.0).round() as i64, s.color.to_lowercase(), s.style.unwrap_or_default())), p.shadow);
                    if s(x) != s(y) {
                        push("Photo", format!("{:?} → {:?}", s(x), s(y)));
                    }
                }
                (ElementKind::Table(x), ElementKind::Table(y)) if x.cells.len() != y.cells.len() || x.border.as_ref().map(|b| (b.width_mm * 100.0).round() as i64) != y.border.as_ref().map(|b| (b.width_mm * 100.0).round() as i64) => {
                    push("Tableau", format!("{} cases → {}", x.cells.len(), y.cells.len()))
                }
                _ => {}
            }
        }
        for (j, b) in after.elements.iter().enumerate().filter(|(j, _)| of_after[*j].is_none()) {
            push(&format!("{} en plus", kind(b)), format!("{j} {b:?}"));
        }
        let bg = |p: &Page| p.background.clone().filter(|b| !(b.color.eq_ignore_ascii_case("#ffffff") && b.pattern.is_none())).map(|b| (b.color.to_lowercase(), b.pattern));
        if bg(before) != bg(after) {
            push("Page.fond", format!("{:?} → {:?}", bg(before), bg(after)));
        }
        if before.grid.is_some() != after.grid.is_some() {
            push("Page.grille", String::new());
        }
        out
    }

    /// Une page à remplir exportée telle quelle : un champ non rempli sort blanc, comme à l'impression. Son nom
    /// est écrit à l'encre transparente, pour qu'il garde la place qu'il prend à l'impression ; le texte autour
    /// reste à l'encre.
    #[test]
    fn an_unfilled_field_exports_blank() {
        let doc = doc_with(vec![text(RichText::plain("Bonjour {prenom} {toto}, {PRENOM_2}"), None)]);
        let content = export(&doc, &AssetStore::default()).xml("content.xml");
        assert!(!content.contains("{prenom}"));
        // Chaque portion écrite, et si son style la rend transparente.
        let invisible = |style: &str| {
            let def = content.split(&format!(r#"<style:style style:name="{style}""#)).nth(1).unwrap();
            def.split("</style:style>").next().unwrap().contains(r#"loext:opacity="0%""#)
        };
        let shown: Vec<(&str, bool)> = content
            .split(r#"<text:span text:style-name=""#)
            .skip(1)
            .map(|s| {
                let (style, rest) = s.split_once(r#"">"#).unwrap();
                (rest.split("</text:span>").next().unwrap(), invisible(style))
            })
            .collect();
        assert_eq!(shown, [("Bonjour ", false), ("Prénom", true), (" {toto}, ", false), ("2", true)], "{content}");
    }

    /// Un champ non rempli revient champ : d'après son nom quand il est le seul à s'afficher ainsi, d'après la
    /// description du cadre pour une lettre (`{PRENOM_2}` et `{prenom_2}` montrent tous deux « 2 »). La description
    /// ne nomme que les lettres, jamais une donnée d'élève ; un texte sans lettre n'en a pas.
    #[test]
    fn an_unfilled_field_comes_back_as_a_field() {
        let mut letters = text(RichText::plain("{PRENOM_2}{prenom_2} et {prenom_10}"), None);
        letters.frame.y_mm = 100.0;
        let doc = doc_with(vec![text(RichText::plain("Bonjour {prenom} {toto}, {age} ({anniversaires_mars})"), None), letters]);
        let content = export(&doc, &AssetStore::default()).xml("content.xml");
        assert_eq!(content.matches("<svg:desc>").count(), 1, "{content}");
        assert!(content.contains("<svg:desc>{PRENOM_2} {prenom_2} {prenom_10}</svg:desc>"), "{content}");
        let back = round_trip(&doc, &AssetStore::default());
        let plain: Vec<String> = back.pages[0].elements.iter().filter_map(|e| if let ElementKind::Text(t) = &e.kind { Some(t.content.paragraphs.iter().flat_map(|p| &p.runs).map(|r| r.text.as_str()).collect()) } else { None }).collect();
        assert_eq!(plain, ["Bonjour {prenom} {toto}, {age} ({anniversaires_mars})", "{PRENOM_2}{prenom_2} et {prenom_10}"]);
    }

    /// Une image à colorier part en contour noir et revient à colorier.
    /// Une étoile, un pictogramme et une flèche en miroir : LibreOffice reçoit le miroir (géométrie, style de
    /// l'image, bouts de la flèche échangés), et la relecture d'un export retrouve celui des deux premiers.
    #[test]
    fn mirrored_shapes_and_pictures_go_to_libreoffice_and_come_back() {
        let star = Element { flip_h: true, ..shape_el(ShapeKind::Star, frame(20.0, 20.0, 30.0, 30.0)) };
        let picto = Element { flip_v: true, ..picture_el("1F98A", None) };
        let arrow = Element { flip_h: true, ..shape_el(ShapeKind::Arrow, frame(20.0, 100.0, 60.0, 10.0)) };
        let doc = doc_with(vec![star, picto, arrow]);
        assert_eq!(crate::native::written_version(&doc), 7);
        let content = export(&doc, &AssetStore::default()).xml("content.xml");
        assert!(content.contains(r#"draw:mirror-horizontal="true""#), "{content}");
        assert!(content.contains("openmoji-1F98A-miroir-v.svg"));
        // La flèche part de la droite (80 mm) vers la gauche (20 mm).
        assert!(content.contains(r#"svg:x1="80.000mm""#) && content.contains(r#"svg:x2="20.000mm""#), "{content}");
        let mut buf = Cursor::new(Vec::new());
        export_odg(&doc, &AssetStore::default(), &FontSet::builtin(), &mut buf).unwrap();
        buf.set_position(0);
        let back = crate::import::odg::read_drawing(buf, &[]).unwrap();
        let els = &back.pages[0].elements;
        assert!(els.iter().any(|e| matches!(&e.kind, ElementKind::Shape(_)) && e.flip_h && !e.flip_v));
        assert!(els.iter().any(|e| matches!(&e.kind, ElementKind::Image(_)) && e.flip_v && !e.flip_h));
    }

    /// Une forme retournée d'un seul sens puis tournée : LibreOffice la retourne après l'avoir tournée, elle part
    /// donc avec l'angle opposé (retournée dans les deux sens, avec le sien), et le générateur porte la marque qui le
    /// dit. La relecture retrouve l'angle, le miroir et la place de l'écran.
    #[test]
    fn a_mirrored_and_turned_shape_gets_the_opposite_angle_and_comes_back() {
        let f = Frame { rotation_deg: 30.0, ..frame(20.0, 20.0, 40.0, 20.0) };
        let flips = [(true, false), (false, true), (true, true)];
        let doc = doc_with(flips.iter().map(|&(h, v)| Element { flip_h: h, flip_v: v, ..shape_el(ShapeKind::Star, f) }).collect());
        let mut e = export(&doc, &AssetStore::default());
        let content = e.xml("content.xml");
        let angles: Vec<&str> = content.split(r#"draw:transform="rotate("#).skip(1).map(|s| s.split(')').next().unwrap()).collect();
        assert_eq!(angles, ["0.523599", "0.523599", "-0.523599"], "{content}");
        assert!(e.xml("meta.xml").contains(MIRROR_AFTER_ROTATION));
        let mut buf = Cursor::new(Vec::new());
        export_odg(&doc, &AssetStore::default(), &FontSet::builtin(), &mut buf).unwrap();
        buf.set_position(0);
        let back = crate::import::odg::read_drawing(buf, &[]).unwrap();
        let got: Vec<(bool, bool, f64, f64, f64)> = back.pages[0].elements.iter().map(|e| (e.flip_h, e.flip_v, (e.frame.rotation_deg * 1000.0).round() / 1000.0, (e.frame.x_mm * 1000.0).round() / 1000.0, (e.frame.y_mm * 1000.0).round() / 1000.0)).collect();
        assert_eq!(got, flips.map(|(h, v)| (h, v, 30.0, 20.0, 20.0)));
    }

    #[test]
    fn an_outlined_picture_goes_and_comes_back_as_an_outline() {
        let el = Element {
            id: "i".into(),
            frame: Frame { x_mm: 20.0, y_mm: 20.0, w_mm: 40.0, h_mm: 40.0, rotation_deg: 0.0 },
            locked: false,
            zone: false,
            group: None,
            kind: ElementKind::Image(ImageProps { code: "1F98A".into(), outline: Some(true) }),
            subgroups: Vec::new(),
            flip_h: false, flip_v: false,
        };
        let mut e = export(&doc_with(vec![el.clone()]), &AssetStore::default());
        let written = e.read("Pictures/openmoji-contour-1F98A.svg");
        let mark = b"<!-- cahutin:openmoji-contour-1F98A -->\n";
        assert!(written.starts_with(mark) && written[mark.len()..] == crate::picto::outline_svg("1F98A").unwrap()[..]);
        let mut buf = Cursor::new(Vec::new());
        export_odg(&doc_with(vec![el]), &AssetStore::default(), &FontSet::builtin(), &mut buf).unwrap();
        buf.set_position(0);
        let back = crate::import::odg::read_drawing(buf, &[]).unwrap();
        let images: Vec<&ImageProps> = back.pages[0].elements.iter().filter_map(|e| if let ElementKind::Image(i) = &e.kind { Some(i) } else { None }).collect();
        assert_eq!(images, [&ImageProps { code: "1F98A".into(), outline: Some(true) }]);
    }

    /// Un tracé libre part en forme personnalisée et revient tracé libre, ouvert et sans remplissage ; un tracé
    /// invalide n'est pas écrit.
    #[test]
    fn a_free_path_goes_and_comes_back() {
        let bridges = "M 0 1000 C 0 -333 250 -333 250 1000 M 300 1000 C 300 -333 550 -333 550 1000";
        let shape = |path: &str| Element {
            id: "p".into(),
            frame: Frame { x_mm: 15.0, y_mm: 40.0, w_mm: 180.0, h_mm: 20.0, rotation_deg: 0.0 },
            locked: false,
            zone: false,
            group: None,
            kind: ElementKind::Shape(ShapeProps { shape: ShapeKind::Path, fill: None, stroke: Some(Stroke { width_mm: 0.8, color: "#9e9e9e".into(), style: Some(StrokeStyle::Dotted) }), corner_radius_mm: 0.0, path: Some(path.into()), graphisme: None }),
            subgroups: Vec::new(),
            flip_h: false, flip_v: false,
        };
        let mut buf = Cursor::new(Vec::new());
        export_odg(&doc_with(vec![shape(bridges)]), &AssetStore::default(), &FontSet::builtin(), &mut buf).unwrap();
        buf.set_position(0);
        let back = crate::import::odg::read_drawing(buf, &[]).unwrap();
        let ElementKind::Shape(s) = &back.pages[0].elements[0].kind else { panic!("{:?}", back.pages[0].elements) };
        assert_eq!((s.shape, s.path.as_deref(), s.fill.as_deref()), (ShapeKind::Path, Some(bridges), None));
        assert_eq!(s.stroke.as_ref().map(|st| st.width_mm), Some(0.8));
        let content = export(&doc_with(vec![shape("M 0 0 A 1 1 0 0 1 2 2")]), &AssetStore::default()).xml("content.xml");
        assert!(!content.contains("custom-shape") && !content.contains("draw:ellipse"), "{content}");
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
        assert_eq!(first.name().unwrap(), "mimetype");
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

    /// Image de 200 × 100 pixels, moitié gauche rouge, moitié droite bleue, en PNG.
    fn halves_asset() -> Asset {
        let img = RgbImage::from_fn(200, 100, |x, _| if x < 100 { Rgb([220, 30, 30]) } else { Rgb([30, 60, 220]) });
        let mut out = Cursor::new(Vec::new());
        img.write_to(&mut out, ImageFormat::Png).unwrap();
        let bytes = Arc::new(out.into_inner());
        Asset { meta: AssetMeta { id: new_id(), width_px: 200, height_px: 100, mime: Mime::Png, taken: None }, thumb: bytes.clone(), bytes }
    }

    fn is_red(p: &image::Rgba<u8>) -> bool {
        p[0] > 180 && p[2] < 80
    }

    fn is_blue(p: &image::Rgba<u8>) -> bool {
        p[2] > 180 && p[0] < 80
    }

    /// Photo arrondie tournée : LibreOffice étire l'image de remplissage, sans la tourner, sur la boîte droite qui
    /// entoure la forme tournée. Elle part donc déjà tournée, aux proportions de cette boîte (coins arrondis
    /// compris), sous un nom marqué « -tourne » ; la photo arrondie droite ne change pas. Relue, la photo retrouve son
    /// image droite, son angle et ses coins.
    #[test]
    fn a_turned_rounded_photo_fills_with_a_turned_image_and_comes_back() {
        let mut store = AssetStore::default();
        let a = halves_asset();
        let mut turned = photo(Some(&a.meta.id), frame(20.0, 20.0, 60.0, 30.0), 3.0);
        turned.frame.rotation_deg = 90.0;
        let mut slanted = photo(Some(&a.meta.id), frame(20.0, 120.0, 60.0, 30.0), 6.0);
        slanted.frame.rotation_deg = 30.0;
        let flat = photo(Some(&a.meta.id), frame(20.0, 200.0, 60.0, 30.0), 3.0);
        let ids = [turned.id.clone(), slanted.id.clone(), flat.id.clone()];
        store.insert(a).unwrap();
        let doc = doc_with(vec![turned, slanted, flat]);
        let mut e = export(&doc, &store);
        let styles = e.xml("styles.xml");
        assert!(styles.contains(&format!(r#"draw:name="img_{}-tourne""#, ids[0])) && styles.contains(&format!(r#"draw:name="img_{}""#, ids[2])), "{styles}");
        // À 90° : l'image debout, rouge en haut, bleu en bas.
        let up = image::load_from_memory(&e.read(&format!("Pictures/{}.png", ids[0]))).unwrap().to_rgba8();
        assert!(up.height() as f64 > 1.9 * up.width() as f64, "{}x{}", up.width(), up.height());
        assert!(is_red(up.get_pixel(up.width() / 2, up.height() / 4)) && is_blue(up.get_pixel(up.width() / 2, 3 * up.height() / 4)));
        // À 30°, coins de 6 mm : boîte de (48 cos 30° + 18 sin 30° + 12) × (48 sin 30° + 18 cos 30° + 12) mm.
        let s = image::load_from_memory(&e.read(&format!("Pictures/{}.png", ids[1]))).unwrap();
        let (bw, bh) = (48.0 * 30f64.to_radians().cos() + 9.0 + 12.0, 24.0 + 18.0 * 30f64.to_radians().cos() + 12.0);
        assert!((s.width() as f64 / s.height() as f64 - bw / bh).abs() < 0.02, "{}x{} au lieu de {bw:.1} × {bh:.1}", s.width(), s.height());
        let f = image::load_from_memory(&e.read(&format!("Pictures/{}.png", ids[2]))).unwrap();
        assert_eq!((f.width(), f.height()), (200, 100));

        let mut buf = Cursor::new(Vec::new());
        export_odg(&doc, &store, &FontSet::builtin(), &mut buf).unwrap();
        buf.set_position(0);
        let back = crate::import::odg::read_drawing(buf, &[]).unwrap();
        for (i, rot) in [90.0, 30.0, 0.0].into_iter().enumerate() {
            let el = &back.pages[0].elements[i];
            let ElementKind::Photo(p) = &el.kind else { panic!("{el:?}") };
            assert!((el.frame.rotation_deg - rot).abs() < 0.01 && (el.frame.w_mm - 60.0).abs() < 0.01 && (el.frame.x_mm - 20.0).abs() < 0.01, "{:?}", el.frame);
            assert_eq!(p.corner_radius_mm, [3.0, 6.0, 3.0][i]);
            let asset = back.assets.iter().find(|a| Some(&a.meta.id) == p.asset.as_ref()).unwrap();
            let img = image::load_from_memory(&asset.bytes).unwrap().to_rgba8();
            let (w, h) = (img.width(), img.height());
            assert!((w as f64 / h as f64 - 2.0).abs() < 0.05, "photo {i} relue en {w}x{h}");
            assert!(is_red(img.get_pixel(w / 4, h / 2)) && is_blue(img.get_pixel(3 * w / 4, h / 2)), "photo {i} relue : {:?} et {:?}", img.get_pixel(w / 4, h / 2), img.get_pixel(3 * w / 4, h / 2));
        }
    }

    #[test]
    fn text_frame_image_is_a_cover_cropped_bitmap_fill() {
        let mut store = AssetStore::default();
        let a = raw_asset(400, 300);
        let mut t = text(RichText::plain("Léa"), None);
        t.frame = frame(10.0, 10.0, 60.0, 20.0);
        if let ElementKind::Text(p) = &mut t.kind {
            p.frame = Some(TextFrame { fill: Some("#ff0000".into()), stroke: None, corner_radius_mm: 3.0, image: Some(a.meta.id.clone()), pattern: None, write_on_lines: None });
        }
        store.insert(a).unwrap();
        let mut e = export(&doc_with(vec![t]), &store);
        let content = e.xml("content.xml");
        // L'image remplace la couleur de fond.
        assert!(content.contains(r#"draw:fill="bitmap""#) && !content.contains("#ff0000"), "{content}");
        assert!(e.xml("styles.xml").contains("<draw:fill-image "));
        let pic = e.names().into_iter().find(|n| n.starts_with("Pictures/")).expect("image");
        let img = image::load_from_memory(&e.read(&pic)).unwrap();
        // Recadrée aux proportions du cadre (3 pour 1) : 400 × 133 pixels.
        assert_eq!((img.width(), img.height()), (400, 133));
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
    fn line_spacing_is_a_percentage_and_absent_stays_100() {
        let mut t = text(RichText::plain("aéré"), None);
        if let ElementKind::Text(p) = &mut t.kind {
            p.style.line_spacing = Some(1.5);
        }
        let mut tight = text(RichText::plain("serré"), None);
        if let ElementKind::Text(p) = &mut tight.kind {
            p.style.line_spacing = Some(0.8);
        }
        let mut e = export(&doc_with(vec![t, tight]), &AssetStore::default());
        let content = e.xml("content.xml");
        assert!(content.contains(r#"fo:line-height="150%""#), "{content}");
        assert!(content.contains(r#"fo:line-height="80%""#), "{content}");
        let mut e = export(&doc_with(vec![text(RichText::plain("normal"), None)]), &AssetStore::default());
        let content = e.xml("content.xml");
        assert!(content.contains(r#"fo:line-height="100%""#) && !content.contains(r#"fo:line-height="150%""#), "{content}");
    }

    /// Paragraphes « Avant, •Colle, •Ciseaux, Milieu, •Fin » ; le premier mot de « Fin » en `fin_pt`.
    fn bulleted(fin_pt: Option<f64>) -> Element {
        let mut content = RichText::plain("Avant\nColle\nCiseaux\nMilieu\nFin");
        for i in [1, 2, 4] {
            content.paragraphs[i].list = Some(ListKind::Bullet);
        }
        content.paragraphs[4].runs[0].size_pt = fin_pt;
        text(content, None)
    }

    /// Une suite de paragraphes à puce est une liste de LibreOffice : la puce « • » dans la police et à la taille
    /// du bloc (100 %, ou relative au premier mot quand il a sa taille), l'étiquette large du retrait. Un
    /// paragraphe sans puce coupe la liste ; un texte sans puce n'a pas de liste.
    #[test]
    fn bullets_are_a_libreoffice_list_in_the_block_font() {
        const TEXT_NS: &str = "urn:oasis:names:tc:opendocument:xmlns:text:1.0";
        const STYLE_NS: &str = "urn:oasis:names:tc:opendocument:xmlns:style:1.0";
        const FO_NS: &str = "urn:oasis:names:tc:opendocument:xmlns:xsl-fo-compatible:1.0";
        let mut e = export(&doc_with(vec![bulleted(Some(28.0))]), &AssetStore::default());
        let content = e.xml("content.xml");
        let doc = roxmltree::Document::parse(&content).unwrap();
        let tb = doc.descendants().find(|n| n.tag_name().name() == "text-box").unwrap();
        let shape: Vec<String> = tb
            .children()
            .filter(|n| n.is_element())
            .map(|n| match n.tag_name().name() {
                "list" => {
                    let items: Vec<String> = n.children().filter(|i| i.tag_name().name() == "list-item").map(|i| i.descendants().filter(|t| t.is_text()).map(|t| t.text().unwrap()).collect()).collect();
                    format!("list[{}]", items.join(","))
                }
                other => other.to_string(),
            })
            .collect();
        assert_eq!(shape, ["p", "list[Colle,Ciseaux]", "p", "list[Fin]"], "{content}");
        let lists: Vec<_> = doc.descendants().filter(|n| n.tag_name().name() == "list").map(|n| n.attribute((TEXT_NS, "style-name")).unwrap().to_string()).collect();
        let style = |name: &str| doc.descendants().find(|n| n.tag_name().name() == "list-style" && n.attribute((STYLE_NS, "name")) == Some(name)).unwrap();
        for (name, size) in [(&lists[0], "100%"), (&lists[1], "50%")] {
            let level = style(name).children().find(|n| n.is_element()).unwrap();
            assert_eq!((level.tag_name().name(), level.attribute((TEXT_NS, "level")), level.attribute((TEXT_NS, "bullet-char"))), ("list-level-style-bullet", Some("1"), Some("•")));
            let props = level.children().find(|n| n.tag_name().name() == "list-level-properties").unwrap();
            // 1,2 × 14 pt = 5,927 mm.
            assert_eq!(props.attribute((TEXT_NS, "min-label-width")), Some("5.927mm"));
            let t = level.children().find(|n| n.tag_name().name() == "text-properties").unwrap();
            assert_eq!((t.attribute((STYLE_NS, "font-name")), t.attribute((FO_NS, "font-size")), t.attribute((FO_NS, "color"))), (Some("Andika"), Some(size), Some("#222222")));
        }
        let mut e = export(&doc_with(vec![text(RichText::plain("Colle\nCiseaux"), None)]), &AssetStore::default());
        let content = e.xml("content.xml");
        assert!(!content.contains("text:list"), "{content}");
    }

    /// Les puces reviennent de l'ODG : mêmes paragraphes à puce, la taille du premier mot gardée.
    #[test]
    fn bullets_come_back_from_the_odg() {
        let doc = doc_with(vec![bulleted(Some(28.0))]);
        let mut buf = Cursor::new(Vec::new());
        export_odg(&doc, &AssetStore::default(), &FontSet::builtin(), &mut buf).unwrap();
        buf.set_position(0);
        let back = crate::import::odg::read_drawing(buf, &["Andika".to_string()]).unwrap();
        let ElementKind::Text(t) = &back.pages[0].elements[0].kind else { panic!() };
        let lists: Vec<_> = t.content.paragraphs.iter().map(|p| p.list).collect();
        let b = Some(ListKind::Bullet);
        assert_eq!(lists, [None, b, b, None, b]);
        assert_eq!(t.content.paragraphs[4].runs[0].size_pt, Some(28.0));
        assert_eq!(t.content.paragraphs.iter().map(|p| p.runs.iter().map(|r| r.text.as_str()).collect::<String>()).collect::<Vec<_>>(), ["Avant", "Colle", "Ciseaux", "Milieu", "Fin"]);
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
    fn dashed_and_dotted_strokes_use_declared_dashes() {
        let stroke = |style| Some(Stroke { width_mm: 1.0, color: "#333333".into(), style: Some(style) });
        let shape = |style| Element {
            id: new_id(),
            frame: frame(10.0, 10.0, 50.0, 30.0),
            locked: false,
            zone: false,
            group: None,
            kind: ElementKind::Shape(ShapeProps { shape: ShapeKind::Rect, fill: None, stroke: stroke(style), corner_radius_mm: 0.0, path: None, graphisme: None }),
            subgroups: Vec::new(),
            flip_h: false, flip_v: false,
        };
        let mut e = export(&doc_with(vec![shape(StrokeStyle::Dashed), shape(StrokeStyle::Dotted), shape(StrokeStyle::Solid)]), &AssetStore::default());
        let content = e.xml("content.xml");
        assert!(content.contains(r#"draw:stroke="dash" draw:stroke-dash="CarnetDashed""#) || content.contains(r#"draw:stroke-dash="CarnetDashed""#), "{content}");
        assert!(content.contains(r#"draw:stroke-dash="CarnetDotted""#));
        assert!(content.contains(r#"draw:stroke="solid""#));
        let styles = e.xml("styles.xml");
        let doc = roxmltree::Document::parse(&styles).unwrap();
        let dash = |name: &str| doc.descendants().find(|n| n.has_tag_name("stroke-dash") && n.attribute(("urn:oasis:names:tc:opendocument:xmlns:drawing:1.0", "name")) == Some(name)).expect(name);
        let d = "urn:oasis:names:tc:opendocument:xmlns:drawing:1.0";
        // Tirets de 3 épaisseurs séparés de 2 ; points d'une épaisseur séparés d'une (comme à l'écran).
        assert_eq!(dash("CarnetDashed").attribute((d, "dots1-length")), Some("300%"));
        assert_eq!(dash("CarnetDashed").attribute((d, "distance")), Some("200%"));
        assert_eq!(dash("CarnetDotted").attribute((d, "dots1-length")), Some("100%"));
        assert_eq!(dash("CarnetDotted").attribute((d, "distance")), Some("100%"));
        // Sans tirets, rien n'est déclaré.
        let mut plain = export(&doc_with(vec![shape(StrokeStyle::Solid)]), &AssetStore::default());
        assert!(!plain.xml("styles.xml").contains("stroke-dash"));
    }

    #[test]
    fn text_frame_fill_border_and_corners_are_exported() {
        let mut t = text(RichText::plain("étiquette"), None);
        if let ElementKind::Text(p) = &mut t.kind {
            p.frame = Some(TextFrame {
                fill: Some("#fdebd0".into()),
                stroke: Some(Stroke { width_mm: 1.0, color: "#c0392b".into(), style: Some(StrokeStyle::Dashed) }),
                corner_radius_mm: 4.0,
                image: None,
                pattern: None,
                write_on_lines: None,
            });
        }
        let mut e = export(&doc_with(vec![t]), &AssetStore::default());
        let content = e.xml("content.xml");
        assert!(content.contains(r##"draw:fill="solid" draw:fill-color="#fdebd0""##), "{content}");
        assert!(content.contains(r#"draw:stroke-dash="CarnetDashed""#));
        assert!(content.contains(&format!(r#"draw:corner-radius="{}""#, mm(4.0))));
        // Une étiquette encore vide garde son cadre.
        let mut empty = text(RichText::default(), Some("Prénom"));
        if let ElementKind::Text(p) = &mut empty.kind {
            p.frame = Some(TextFrame { fill: Some("#d6eaf8".into()), ..TextFrame::default() });
        }
        let mut e2 = export(&doc_with(vec![empty]), &AssetStore::default());
        let c2 = e2.xml("content.xml");
        assert!(c2.contains("#d6eaf8") && c2.contains("<draw:text-box"), "{c2}");
        assert!(!all_text(&c2).contains("Prénom"));
        // Un texte sans cadre reste transparent et sans trait.
        let mut plain = export(&doc_with(vec![text(RichText::plain("x"), None)]), &AssetStore::default());
        let c = plain.xml("content.xml");
        assert!(c.contains(r#"draw:stroke="none""#) && c.contains(r#"draw:fill="none""#) && !c.contains("corner-radius"));
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

/// Le même prénom dans trois polices (étiquettes) : une police par passage, toutes embarquées.
    #[test]
    fn each_run_keeps_its_font_and_every_font_is_embedded() {
        let mut rt = RichText::plain("Léa\nLéa\nLéa");
        rt.paragraphs[1].runs[0].font = Some("OpenDyslexic".into());
        rt.paragraphs[2].runs[0].font = Some("Playwrite FR Moderne".into());
        rt.paragraphs[2].runs[0].size_pt = Some(32.0);
        let d = doc_with(vec![text(rt, None)]);
        let mut e = export(&d, &AssetStore::default());
        let content = e.xml("content.xml");
        let doc = roxmltree::Document::parse(&content).unwrap();
        let style_of = |name: &str| {
            let style = doc.descendants().find(|n| n.has_tag_name("style") && n.attribute(("urn:oasis:names:tc:opendocument:xmlns:style:1.0", "name")) == Some(name)).unwrap();
            let props = style.children().find(|n| n.has_tag_name("text-properties")).unwrap();
            let attr = |a: &str| props.attributes().find(|x| x.name() == a).map(|x| x.value().to_string());
            (attr("font-name"), attr("font-size"))
        };
        let spans: Vec<_> = doc.descendants().filter(|n| n.has_tag_name("span")).map(|n| style_of(n.attributes().find(|a| a.name() == "style-name").unwrap().value())).collect();
        assert_eq!(spans.len(), 3);
        let fonts: Vec<String> = spans.iter().map(|(f, _)| f.clone().unwrap()).collect();
        assert!(fonts[0].starts_with("Andika") && fonts[1].starts_with("OpenDyslexic") && fonts[2].starts_with("Playwrite"), "{fonts:?}");
        assert_eq!(spans[2].1.as_deref(), Some("32pt"));
        for family in ["Andika", "OpenDyslexic", "Playwrite"] {
            assert!(e.names().iter().any(|n| n.starts_with("Fonts/") && n.contains(family)), "{family} : {:?}", e.names());
        }
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
                path: None,
                graphisme: None,
            }),
            subgroups: Vec::new(),
            flip_h: false, flip_v: false,
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
    fn a_line_without_stroke_exports() {
        let mut el = shape_el(ShapeKind::Line, frame(10.0, 20.0, 60.0, 8.0));
        if let ElementKind::Shape(s) = &mut el.kind {
            s.stroke = None;
        }
        let mut e = export(&doc_with(vec![el]), &AssetStore::default());
        let content = e.xml("content.xml");
        let styles = e.xml("styles.xml");
        assert!(content.contains("draw:stroke=\"none\"") || styles.contains("draw:stroke=\"none\""), "{content}");
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
            kind: ElementKind::Image(ImageProps { code: code.into(), outline: None }),
            subgroups: Vec::new(),
            flip_h: false, flip_v: false,
        }
    }

    #[test]
    fn picture_is_embedded_as_svg() {
        let mut e = export(&doc_with(vec![picture_el("1F98A", None), picture_el("1F98A", None)]), &AssetStore::default());
        let names = e.names();
        assert_eq!(names.iter().filter(|n| *n == "Pictures/openmoji-1F98A.svg").count(), 1, "{names:?}");
        // Le SVG de la banque, précédé du commentaire qui le nomme.
        let mark = b"<!-- cahutin:openmoji-1F98A -->\n";
        assert_eq!(e.read("Pictures/openmoji-1F98A.svg"), [&mark[..], &crate::picto::svg("1F98A").unwrap()].concat());
        let content = e.xml("content.xml");
        assert_eq!(content.matches(r#"xlink:href="Pictures/openmoji-1F98A.svg""#).count(), 2);
        assert!(e.xml("META-INF/manifest.xml").contains("image/svg+xml"));
    }

    #[test]
    fn outlined_text_uses_the_native_outline() {
        let mut e = text(RichText::plain("Léa"), None);
        if let ElementKind::Text(t) = &mut e.kind {
            t.style.outline = Some(true);
        }
        let content = export(&doc_with(vec![e]), &AssetStore::default()).xml("content.xml");
        assert!(content.contains(r#"style:text-outline="true""#), "{content}");
        let plain = export(&doc_with(vec![text(RichText::plain("Léa"), None)]), &AssetStore::default()).xml("content.xml");
        assert!(!plain.contains("text-outline"));
    }

    /// Un tableau de `cols` x `rows` cases à (20, 20), 100 x 50 mm, et l'id de chaque case par (colonne, ligne).
    fn table_doc(cols: u8, rows: u8) -> (Document, Id) {
        let mut d = Document::new("essai");
        let t = crate::table::add_table(&mut d.pages[0], frame(20.0, 20.0, 100.0, 50.0), cols, rows);
        (d, t)
    }

    fn cell_id(d: &Document, t: &str, col: u8, row: u8) -> Id {
        let e = d.pages[0].elements.iter().find(|e| e.id == t).unwrap();
        let ElementKind::Table(p) = &e.kind else { unreachable!() };
        p.cells.iter().find(|c| c.col == col && c.row == row).unwrap().element_id.clone()
    }

    fn el_mut<'a>(d: &'a mut Document, id: &str) -> &'a mut Element {
        d.pages[0].elements.iter_mut().find(|e| e.id == id).unwrap()
    }

    fn table_mut<'a>(d: &'a mut Document, t: &str) -> &'a mut TableProps {
        let ElementKind::Table(p) = &mut el_mut(d, t).kind else { unreachable!() };
        p
    }

    fn set_text(d: &mut Document, id: &str, text: &str) {
        let ElementKind::Text(t) = &mut el_mut(d, id).kind else { panic!("case sans texte") };
        t.content = RichText::plain(text);
    }

    fn is_simple(d: &Document, t: &str) -> bool {
        let e = d.pages[0].elements.iter().find(|e| e.id == t).unwrap();
        content::is_simple(e, &d.pages[0].elements)
    }

    /// La définition d'un style automatique de `content.xml`.
    fn style_def<'a>(content: &'a str, name: &str) -> &'a str {
        let def = content.split(&format!(r#"<style:style style:name="{name}""#)).nth(1).unwrap_or_else(|| panic!("style {name}"));
        def.split("</style:style>").next().unwrap()
    }

    /// Valeurs d'un attribut, dans l'ordre du document.
    fn attr_values<'a>(xml: &'a str, attr: &str) -> Vec<&'a str> {
        xml.split(&format!(r#" {attr}=""#)).skip(1).map(|v| v.split('"').next().unwrap()).collect()
    }

    /// Un tableau simple sort en vrai tableau de LibreOffice : un cadre à la place et à la taille du tableau (pas dans
    /// un groupe), ses colonnes et ses lignes, ses cases et leur texte ; aucun trait ni cadre de texte à part, aucune
    /// image d'aperçu ; le trait écrit divisé par 1,764 ; chaque case sans fond le dit (`draw:fill="none"`).
    #[test]
    fn a_simple_table_is_a_real_table() {
        let (mut d, t) = table_doc(2, 2);
        let a = cell_id(&d, &t, 0, 0);
        set_text(&mut d, &a, "Lundi");
        assert!(is_simple(&d, &t));
        let mut out = export(&d, &AssetStore::default());
        let content = out.xml("content.xml");
        assert!(content.contains(r#"xmlns:table="urn:oasis:names:tc:opendocument:xmlns:table:1.0""#));
        assert!(!content.contains("<draw:line") && !content.contains("<draw:g") && !content.contains("draw:text-box"), "{content}");
        assert!(out.names().iter().all(|n| !n.ends_with(".svm")));
        assert!(content.contains(r#"svg:x="20.000mm" svg:y="20.000mm" svg:width="100.000mm" svg:height="50.000mm"><table:table>"#), "{content}");
        assert_eq!(content.matches("<table:table-column ").count(), 2);
        assert_eq!(content.matches("<table:table-row ").count(), 2);
        assert_eq!(content.matches("<table:table-cell ").count(), 4);
        assert_eq!(attr_values(&content, "style:column-width"), ["50.000mm"]);
        assert_eq!(attr_values(&content, "style:row-height"), ["25.000mm"]);
        assert!(content.contains(r#"style:use-optimal-row-height="false""#) && !content.contains("min-row-height"));
        // Trait par défaut de 0,5 mm : 0,283 mm écrit, que LibreOffice dessine à 0,5 mm.
        assert_eq!(attr_values(&content, "fo:border").into_iter().collect::<std::collections::HashSet<_>>(), ["0.283mm solid #333333"].into());
        let cells: Vec<&str> = attr_values(&content, "table:style-name").into_iter().filter(|s| s.starts_with("ce")).collect();
        for c in &cells {
            let def = style_def(&content, c);
            assert!(def.contains(r#"style:family="table-cell""#) && def.contains(r#"<loext:graphic-properties draw:fill="none""#), "{def}");
            assert!(def.contains(r#"fo:padding-left="1.500mm""#) && def.contains(r#"draw:textarea-vertical-align="middle""#), "{def}");
        }
        let first = content.split("<table:table-cell ").nth(1).unwrap().split("</table:table-cell>").next().unwrap();
        assert!(first.contains(">Lundi</text:span>"), "{first}");
    }

    /// Le trait du tableau est écrit à son épaisseur divisée par 1,764 (`CELL_BORDER_DRAWN`) ; sans trait : `none`.
    #[test]
    fn a_real_table_compensates_the_border_libreoffice_thickens() {
        let (mut d, t) = table_doc(1, 1);
        for (width, want) in [(1.0, "0.567mm"), (2.0, "1.134mm"), (0.6, "0.340mm")] {
            table_mut(&mut d, &t).border = Some(Stroke { width_mm: width, color: "#6b5444".into(), style: Some(StrokeStyle::Solid) });
            let content = export(&d, &AssetStore::default()).xml("content.xml");
            assert_eq!(attr_values(&content, "fo:border"), [format!("{want} solid #6b5444")]);
        }
        table_mut(&mut d, &t).border = None;
        let content = export(&d, &AssetStore::default()).xml("content.xml");
        assert_eq!(attr_values(&content, "fo:border"), ["none"]);
        assert!(content.contains("<table:table>"));
    }

    /// Chaque largeur de colonne et hauteur de ligne est la différence de deux bords arrondis au 1/100 mm, et le cadre
    /// leur somme : LibreOffice garde ses tailles au 1/100 mm, les erreurs ne s'additionnent pas.
    #[test]
    fn a_real_table_rounds_its_edges_to_the_hundredth() {
        let mut d = Document::new("essai");
        let t = crate::table::add_table(&mut d.pages[0], frame(10.004, 20.0, 100.0, 70.0), 3, 3);
        let content = export(&d, &AssetStore::default()).xml("content.xml");
        // Bords 10,004 | 43,337 | 76,671 | 110,004 arrondis : 10 | 43,34 | 76,67 | 110.
        let widths: Vec<String> = attr_values(&content, "table:style-name").into_iter().filter(|s| s.starts_with("co")).map(|s| attr_values(style_def(&content, s), "style:column-width")[0].to_string()).collect();
        assert_eq!(widths, ["33.340mm", "33.330mm", "33.330mm"]);
        // Lignes : 20 | 43,333 | 66,667 | 90 arrondis : 20 | 43,33 | 66,67 | 90.
        let heights: Vec<String> = attr_values(&content, "table:style-name").into_iter().filter(|s| s.starts_with("ro")).map(|s| attr_values(style_def(&content, s), "style:row-height")[0].to_string()).collect();
        assert_eq!(heights, ["23.330mm", "23.340mm", "23.330mm"]);
        assert!(content.contains(r#"svg:x="10.000mm" svg:y="20.000mm" svg:width="100.000mm" svg:height="70.000mm"><table:table>"#), "{content}");
        assert!(is_simple(&d, &t));
    }

    /// Cases fusionnées : la case porte le nombre de colonnes ou de lignes qu'elle couvre, et chaque place couverte
    /// est une `table:covered-table-cell`, à sa place dans la ligne.
    #[test]
    fn merged_cells_span_and_cover_their_places() {
        let (mut d, t) = table_doc(3, 3);
        let (a, b) = (cell_id(&d, &t, 0, 0), cell_id(&d, &t, 1, 0));
        crate::table::merge(&mut d.pages[0], &a, &b).unwrap();
        let (a, b) = (cell_id(&d, &t, 2, 1), cell_id(&d, &t, 2, 2));
        crate::table::merge(&mut d.pages[0], &a, &b).unwrap();
        assert!(is_simple(&d, &t));
        let content = export(&d, &AssetStore::default()).xml("content.xml");
        let rows: Vec<Vec<String>> = content
            .split("<table:table-row ")
            .skip(1)
            .map(|r| {
                let r = r.split("</table:table-row>").next().unwrap();
                r.split('<')
                    .filter(|t| t.starts_with("table:table-cell") || t.starts_with("table:covered-table-cell"))
                    .map(|t| {
                        let spans = attr_values(t, "table:number-columns-spanned").first().map(|v| format!(" c{v}")).unwrap_or_default() + &attr_values(t, "table:number-rows-spanned").first().map(|v| format!(" r{v}")).unwrap_or_default();
                        format!("{}{spans}", if t.starts_with("table:covered") { "couverte" } else { "case" })
                    })
                    .collect()
            })
            .collect();
        assert_eq!(rows, [vec!["case c2", "couverte", "case"], vec!["case", "case", "case r2"], vec!["case", "case", "couverte"]]);
    }

    /// Case vide : une espace sans largeur dans un passage à la police et à la taille de la case, marges gardées, quand
    /// une ligne de ce texte et ses marges tiennent dans la case ; sinon en 1 pt, sans marge en haut ni en bas. Une
    /// case pictogramme est toujours en 1 pt.
    #[test]
    fn an_empty_cell_holds_a_zero_width_space() {
        let style_of_span = |content: &str, cell: &str| -> (String, String) {
            let span = cell.split(r#"<text:span text:style-name=""#).nth(1).unwrap().split('"').next().unwrap();
            let cell_style = cell.split('"').nth(1).unwrap();
            (style_def(content, span).to_string(), style_def(content, cell_style).to_string())
        };
        // Lignes de 25 mm : la police de la case (Andika 14 pt) tient.
        let (d, _) = table_doc(1, 2);
        let content = export(&d, &AssetStore::default()).xml("content.xml");
        let cell = content.split("<table:table-cell ").nth(1).unwrap().split("</table:table-cell>").next().unwrap();
        assert!(cell.contains("\u{200B}</text:span>"), "{cell}");
        let (span, cell_style) = style_of_span(&content, cell);
        assert!(span.contains(r#"style:font-name="Andika""#) && span.contains(r#"fo:font-size="14pt""#), "{span}");
        assert!(cell_style.contains(r#"fo:padding-top="1.500mm""#), "{cell_style}");
        // Lignes de 5 mm : 14 pt n'y tient pas.
        let mut d = Document::new("essai");
        crate::table::add_table(&mut d.pages[0], frame(20.0, 20.0, 100.0, 50.0), 2, 10);
        let content = export(&d, &AssetStore::default()).xml("content.xml");
        let cell = content.split("<table:table-cell ").nth(1).unwrap().split("</table:table-cell>").next().unwrap();
        let (span, cell_style) = style_of_span(&content, cell);
        assert!(span.contains(r#"fo:font-size="1pt""#), "{span}");
        assert!(cell_style.contains(r#"fo:padding-top="0.000mm""#) && cell_style.contains(r#"fo:padding-bottom="0.000mm""#) && cell_style.contains(r#"fo:padding-left="1.500mm""#), "{cell_style}");
        // La règle : une ligne de texte et deux marges dans la hauteur de la case.
        assert!(content::empty_cell_fits(7.96, 1.5, 10.96) && !content::empty_cell_fits(7.96, 1.5, 10.9));
    }

    /// Fond de couleur d'une case : le fond de la case du tableau.
    #[test]
    fn a_colored_cell_fills_its_table_cell() {
        let (mut d, t) = table_doc(2, 1);
        let a = cell_id(&d, &t, 1, 0);
        set_text(&mut d, &a, "Mardi");
        let ElementKind::Text(tx) = &mut el_mut(&mut d, &a).kind else { unreachable!() };
        tx.frame = Some(TextFrame { fill: Some("#d6e8ff".into()), stroke: None, corner_radius_mm: 0.0, image: None, pattern: None, write_on_lines: None });
        assert!(is_simple(&d, &t));
        let content = export(&d, &AssetStore::default()).xml("content.xml");
        let cell = content.split("<table:table-cell ").nth(2).unwrap();
        let def = style_def(&content, cell.split('"').nth(1).unwrap());
        assert!(def.contains(r##"<loext:graphic-properties draw:fill="solid" draw:fill-color="#d6e8ff""##), "{def}");
        assert!(cell.contains(">Mardi</text:span>"));
    }

    /// Pictogramme et photo d'une case : le fond de la case, non étiré, à leur taille dans Cahutin, au centre ; le SVG
    /// du pictogramme reste un SVG, déclaré une fois même s'il revient dans plusieurs cases ; la photo est recadrée
    /// comme à l'écran.
    #[test]
    fn pictures_fill_their_cell_unstretched() {
        let (mut d, t) = table_doc(3, 1);
        let mut store = AssetStore::default();
        let asset = raw_asset(300, 200);
        let id = asset.meta.id.clone();
        d.assets.insert(id.clone(), asset.meta.clone());
        store.insert(asset).unwrap();
        for (col, content) in [(0, crate::table::CellContent::Picto("1F98A".into())), (1, crate::table::CellContent::Picto("1F98A".into())), (2, crate::table::CellContent::Photo)] {
            let c = cell_id(&d, &t, col, 0);
            crate::table::set_cell_content(&mut d.pages[0], &c, content).unwrap();
        }
        let p = cell_id(&d, &t, 2, 0);
        if let ElementKind::Photo(ph) = &mut el_mut(&mut d, &p).kind {
            ph.asset = Some(id);
        }
        assert!(is_simple(&d, &t));
        let c0 = cell_id(&d, &t, 0, 0);
        let picto = el_mut(&mut d, &c0).frame;
        let photo = el_mut(&mut d, &p).frame;
        let mut out = export(&d, &store);
        let content = out.xml("content.xml");
        let styles = out.xml("styles.xml");
        assert!(!content.contains("<draw:image"), "{content}");
        assert_eq!(styles.matches("Pictures/openmoji-1F98A.svg").count(), 1, "{styles}");
        assert!(out.names().contains(&"Pictures/openmoji-1F98A.svg".to_string()));
        let cells: Vec<&str> = content.split("<table:table-cell ").skip(1).map(|c| c.split('"').nth(1).unwrap()).collect();
        for (cell, f) in [(cells[0], picto), (cells[2], photo)] {
            let def = style_def(&content, cell);
            for want in [
                r#"draw:fill="bitmap""#.to_string(),
                r#"style:repeat="no-repeat""#.into(),
                format!(r#"draw:fill-image-width="{}""#, mm(f.w_mm)),
                format!(r#"draw:fill-image-height="{}""#, mm(f.h_mm)),
                r#"draw:fill-image-ref-point="center""#.into(),
            ] {
                assert!(def.contains(&want), "{want} : {def}");
            }
            let name = attr_values(def, "draw:fill-image-name")[0];
            assert!(styles.contains(&format!(r#"draw:name="{name}""#)), "{name}");
        }
        assert_eq!(cells[0], cells[1], "même pictogramme, même style");
        // La photo recadrée a les proportions de son cadre (case moins 1,5 mm de chaque côté).
        let href = styles.split(&format!(r#"draw:name="{}""#, attr_values(style_def(&content, cells[2]), "draw:fill-image-name")[0])).nth(1).unwrap();
        let href = attr_values(href, "xlink:href")[0].to_string();
        let img = image::load_from_memory(&out.read(&href)).unwrap();
        assert!((img.width() as f64 / img.height() as f64 - photo.w_mm / photo.h_mm).abs() < 0.02, "{}x{} pour {photo:?}", img.width(), img.height());
    }

    /// Ce qu'un vrai tableau ne rend pas comme l'écran : le tableau garde ses traits et ses cases à part. Chaque cas,
    /// un à un, sur un tableau simple au départ.
    #[test]
    fn a_table_that_is_not_simple_keeps_its_lines() {
        let base = || {
            let (mut d, t) = table_doc(2, 2);
            let p = cell_id(&d, &t, 1, 1);
            crate::table::set_cell_content(&mut d.pages[0], &p, crate::table::CellContent::Photo).unwrap();
            let a = cell_id(&d, &t, 0, 0);
            set_text(&mut d, &a, "Lundi");
            (d, t, a, p)
        };
        let text_frame = |d: &mut Document, a: &str, f: TextFrame| {
            let ElementKind::Text(tx) = &mut el_mut(d, a).kind else { unreachable!() };
            tx.frame = Some(f);
        };
        let photo = |d: &mut Document, p: &str, change: &dyn Fn(&mut PhotoProps)| {
            let ElementKind::Photo(ph) = &mut el_mut(d, p).kind else { unreachable!() };
            change(ph);
        };
        let plain = TextFrame { fill: None, stroke: None, corner_radius_mm: 0.0, image: None, pattern: None, write_on_lines: None };
        let cases: Vec<(&str, Box<dyn Fn(&mut Document, &str, &str, &str)>)> = vec![
            ("trait en tirets", Box::new(|d, t, _, _| table_mut(d, t).border = Some(Stroke { width_mm: 1.0, color: "#c0392b".into(), style: Some(StrokeStyle::Dashed) }))),
            ("trait en pointillés", Box::new(|d, t, _, _| table_mut(d, t).border = Some(Stroke { width_mm: 1.0, color: "#c0392b".into(), style: Some(StrokeStyle::Dotted) }))),
            ("lignes d'écriture", Box::new({ let f = plain.clone(); move |d, _, a, _| text_frame(d, a, TextFrame { pattern: Some(Pattern::Seyes), ..f.clone() }) })),
            ("bordure de case", Box::new({ let f = plain.clone(); move |d, _, a, _| text_frame(d, a, TextFrame { stroke: Some(Stroke { width_mm: 0.5, color: "#000000".into(), style: None }), ..f.clone() }) })),
            ("case aux coins arrondis", Box::new({ let f = plain.clone(); move |d, _, a, _| text_frame(d, a, TextFrame { fill: Some("#ffd6d6".into()), corner_radius_mm: 2.0, ..f.clone() }) })),
            ("image de fond d'une case texte", Box::new({ let f = plain.clone(); move |d, _, a, _| text_frame(d, a, TextFrame { image: Some("a1".into()), ..f.clone() }) })),
            ("texte retourné", Box::new(|d, _, a, _| el_mut(d, a).flip_h = true)),
            ("photo aux coins arrondis", Box::new(move |d, _, _, p| photo(d, p, &|ph| ph.corner_radius_mm = 3.0))),
            ("photo bordée", Box::new(move |d, _, _, p| photo(d, p, &|ph| ph.border = Some(Stroke { width_mm: 0.5, color: "#000000".into(), style: None })))),
            ("photo ombrée", Box::new(move |d, _, _, p| photo(d, p, &|ph| ph.shadow = true))),
            ("photo en miroir", Box::new(move |d, _, _, p| photo(d, p, &|ph| ph.mirrored = true))),
            ("photo retournée de haut en bas", Box::new(|d, _, _, p| el_mut(d, p).flip_v = true)),
            ("case tournée", Box::new(|d, _, _, p| el_mut(d, p).frame.rotation_deg = 10.0)),
            ("tableau tourné", Box::new(|d, t, _, _| el_mut(d, t).frame.rotation_deg = 15.0)),
            ("case déplacée", Box::new(|d, _, a, _| el_mut(d, a).frame.x_mm += 2.0)),
            ("forme dans une case", Box::new(|d, _, a, _| el_mut(d, a).kind = shape_el(ShapeKind::Rect, frame(0.0, 0.0, 1.0, 1.0)).kind)),
            ("place sans case", Box::new(|d, t, _, _| { table_mut(d, t).cells.pop(); })),
            ("deux cases à la même place", Box::new(|d, t, _, _| { let tp = table_mut(d, t); tp.cells[1].col = 0; })),
            ("élément entre les cases et le tableau", Box::new(|d, _, _, _| { let s = shape_el(ShapeKind::Rect, frame(150.0, 20.0, 10.0, 10.0)); let at = d.pages[0].elements.len() - 1; d.pages[0].elements.insert(at, s); })),
            ("élément étranger dans le groupe", Box::new(|d, t, _, _| { let mut s = shape_el(ShapeKind::Rect, frame(150.0, 20.0, 10.0, 10.0)); s.group = el_mut(d, t).group.clone(); d.pages[0].elements.push(s); })),
        ];
        let (d, t, ..) = base();
        assert!(is_simple(&d, &t), "le tableau de départ est simple");
        let mut store = AssetStore::default();
        let mut asset = raw_asset(40, 30);
        asset.meta.id = "a1".into();
        store.insert(asset).unwrap();
        for (name, change) in &cases {
            let (mut d, t, a, p) = base();
            change(&mut d, &t, &a, &p);
            assert!(!is_simple(&d, &t), "{name} : classé simple");
            let content = export(&d, &store).xml("content.xml");
            assert!(!content.contains("<table:table"), "{name} : vrai tableau");
            if !name.contains("tirets") && !name.contains("pointillés") {
                assert!(content.matches("<draw:line").count() >= 6, "{name} : traits absents");
            }
        }
        // Les tirets restent les traits d'avant : six segments en tirets, la case écrite à part.
        let (mut d, t, ..) = base();
        cases[0].1(&mut d, &t, "", "");
        let content = export(&d, &AssetStore::default()).xml("content.xml");
        let g = content.split("<draw:g>").nth(1).expect("draw:g").split("</draw:g>").next().unwrap();
        assert_eq!(g.matches("<draw:line").count(), 6);
        assert!(g.contains(r#"svg:x1="70.000mm" svg:y1="20.000mm" svg:x2="70.000mm" svg:y2="70.000mm""#), "{g}");
        assert!(g.contains("draw:text-box") && content.contains(r#"draw:stroke-dash="CarnetDashed""#), "{g}");
    }

    /// Aller-retour : un tableau simple exporté puis relu par Cahutin est le même tableau. Même cadre, colonnes, lignes,
    /// fusions et trait ; mêmes textes, puces, fonds, police et taille, alignements et marge ; même pictogramme (miroir
    /// compris) et une photo dans leurs cases ; une case vide garde sa police et sa taille.
    #[test]
    fn a_real_table_comes_back_as_the_same_table() {
        let mut d = Document::new("essai");
        let t = crate::table::add_table(&mut d.pages[0], frame(15.0, 36.0, 267.0, 162.0), 4, 3);
        {
            let tp = table_mut(&mut d, &t);
            tp.cols = vec![1.3, 1.0, 1.0, 1.0];
            tp.rows = vec![0.8, 1.0, 1.0];
            tp.border = Some(Stroke { width_mm: 0.6, color: "#6b5444".into(), style: None });
        }
        crate::table::layout(&mut d.pages[0], &t);
        let (a, b) = (cell_id(&d, &t, 1, 1), cell_id(&d, &t, 3, 1));
        crate::table::merge(&mut d.pages[0], &a, &b).unwrap();
        let mut store = AssetStore::default();
        let asset = raw_asset(300, 200);
        let aid = asset.meta.id.clone();
        d.assets.insert(aid.clone(), asset.meta.clone());
        store.insert(asset).unwrap();
        let (head, list, picto, photo, empty) = (cell_id(&d, &t, 0, 0), cell_id(&d, &t, 0, 1), cell_id(&d, &t, 1, 0), cell_id(&d, &t, 2, 0), cell_id(&d, &t, 1, 2));
        if let ElementKind::Text(tx) = &mut el_mut(&mut d, &head).kind {
            tx.content = RichText::plain("Quand ?");
            tx.style.size_pt = 18.0;
            tx.style.valign = Some(VAlign::Top);
            tx.frame = Some(TextFrame { fill: Some("#f1e6d6".into()), stroke: None, corner_radius_mm: 0.0, image: None, pattern: None, write_on_lines: None });
        }
        if let ElementKind::Text(tx) = &mut el_mut(&mut d, &list).kind {
            tx.content = RichText { paragraphs: vec![
                Paragraph { align: None, runs: vec![Run::plain("Pomme")], list: Some(ListKind::Bullet) },
                Paragraph { align: None, runs: vec![Run { bold: true, ..Run::plain("Poire") }], list: Some(ListKind::Bullet) },
            ] };
            tx.style.align = Align::Left;
        }
        if let ElementKind::Text(tx) = &mut el_mut(&mut d, &empty).kind {
            tx.style.font = "Playwrite FR Moderne".into();
            tx.style.size_pt = 20.0;
        }
        crate::table::set_cell_content(&mut d.pages[0], &picto, crate::table::CellContent::Picto("1F98A".into())).unwrap();
        el_mut(&mut d, &picto).flip_h = true;
        crate::table::set_cell_content(&mut d.pages[0], &photo, crate::table::CellContent::Photo).unwrap();
        if let ElementKind::Photo(ph) = &mut el_mut(&mut d, &photo).kind {
            ph.asset = Some(aid);
        }
        assert!(is_simple(&d, &t));
        let mut buf = Cursor::new(Vec::new());
        export_odg(&d, &store, &FontSet::builtin(), &mut buf).unwrap();
        buf.set_position(0);
        let known: Vec<String> = FontSet::builtin().families().into_iter().map(|f| f.name).collect();
        let back = crate::import::odg::read_drawing(buf, &known).unwrap();
        assert_eq!(back.report.skipped, 0, "{:?}", back.report);
        let page = &back.pages[0];
        let tables: Vec<&Element> = page.elements.iter().filter(|e| matches!(e.kind, ElementKind::Table(_))).collect();
        assert_eq!(tables.len(), 1);
        let (before, after) = (d.pages[0].elements.iter().find(|e| e.id == t).unwrap(), tables[0]);
        let (ElementKind::Table(tb), ElementKind::Table(ta)) = (&before.kind, &after.kind) else { unreachable!() };
        let near = |a: f64, b: f64| (a - b).abs() < 0.01;
        let f = (before.frame, after.frame);
        assert!(near(f.0.x_mm, f.1.x_mm) && near(f.0.y_mm, f.1.y_mm) && near(f.0.w_mm, f.1.w_mm) && near(f.0.h_mm, f.1.h_mm), "{f:?}");
        let sizes = |v: &[f64], len: f64| v.iter().map(|s| s / v.iter().sum::<f64>() * len).collect::<Vec<_>>();
        for (x, y) in sizes(&tb.cols, f.0.w_mm).into_iter().zip(sizes(&ta.cols, f.1.w_mm)).chain(sizes(&tb.rows, f.0.h_mm).into_iter().zip(sizes(&ta.rows, f.1.h_mm))) {
            assert!(near(x, y), "colonnes {:?} / {:?}, lignes {:?} / {:?}", tb.cols, ta.cols, tb.rows, ta.rows);
        }
        assert_eq!(ta.border, tb.border.clone().map(|b| Stroke { style: None, ..b }));
        // Chaque case : place, fusion, et ce qu'elle montre.
        let describe = |els: &[Element], tp: &TableProps| -> Vec<String> {
            let mut v: Vec<String> = tp
                .cells
                .iter()
                .map(|c| {
                    let e = els.iter().find(|e| e.id == c.element_id).unwrap();
                    let what = match &e.kind {
                        ElementKind::Text(tx) => format!(
                            "texte {:?} {:?} {:?} {} {} {:?} {:?}",
                            tx.content.paragraphs.iter().map(|p| (p.runs.iter().map(|r| (r.text.clone(), r.bold)).collect::<Vec<_>>(), p.list)).collect::<Vec<_>>(),
                            tx.style.align,
                            tx.style.valign.unwrap_or_default(),
                            tx.style.font,
                            tx.style.size_pt,
                            tx.style.padding(),
                            tx.frame.as_ref().and_then(|f| f.fill.clone())
                        ),
                        ElementKind::Image(i) => format!("picto {} miroir {} {}", i.code, e.flip_h, e.flip_v),
                        ElementKind::Photo(p) => format!("photo {}", p.asset.is_some()),
                        k => format!("{k:?}"),
                    };
                    format!("{} {} {} {} : {what}", c.col, c.row, c.col_span, c.row_span)
                })
                .collect();
            v.sort();
            v
        };
        assert_eq!(describe(&page.elements, ta), describe(&d.pages[0].elements, tb));
        // Chaque case à sa place, au 1/100 mm près (les bords arrondis de l'export).
        for c in &tb.cells {
            let e = d.pages[0].elements.iter().find(|e| e.id == c.element_id).unwrap().frame;
            let g = ta.cells.iter().find(|g| (g.col, g.row) == (c.col, c.row)).unwrap();
            let r = page.elements.iter().find(|e| e.id == g.element_id).unwrap().frame;
            let near = |a: f64, b: f64| (a - b).abs() < 0.011;
            assert!(near(e.x_mm, r.x_mm) && near(e.y_mm, r.y_mm) && near(e.w_mm, r.w_mm) && near(e.h_mm, r.h_mm), "case {} {} : {e:?} / {r:?}", c.col, c.row);
        }
    }

    /// Lettres creuses, police cursive et pictogramme en miroir dans une case : mesurés au même rendu qu'en cadres à
    /// part (mesure faite à la main : chaque mot à 0,01 mm près, aucun pixel d'écart dans la case), le tableau reste
    /// simple.
    #[test]
    fn outlined_cursive_and_mirrored_picto_cells_stay_simple() {
        let (mut d, t) = table_doc(3, 1);
        let (a, b, c) = (cell_id(&d, &t, 0, 0), cell_id(&d, &t, 1, 0), cell_id(&d, &t, 2, 0));
        set_text(&mut d, &a, "IIoo");
        set_text(&mut d, &b, "Bonjour");
        if let ElementKind::Text(tx) = &mut el_mut(&mut d, &a).kind {
            tx.style.outline = Some(true);
        }
        if let ElementKind::Text(tx) = &mut el_mut(&mut d, &b).kind {
            tx.style.font = "Playwrite FR Moderne".into();
        }
        crate::table::set_cell_content(&mut d.pages[0], &c, crate::table::CellContent::Picto("1F98A".into())).unwrap();
        el_mut(&mut d, &c).flip_h = true;
        assert!(is_simple(&d, &t));
        let mut out = export(&d, &AssetStore::default());
        assert!(out.xml("styles.xml").contains("Pictures/openmoji-1F98A-miroir-h.svg"));
        assert!(out.xml("content.xml").contains(r#"style:text-outline="true""#));
    }

    #[test]
    fn writing_lines_are_drawn_under_the_text() {
        let mut t = text(RichText::plain("Léa"), None);
        if let ElementKind::Text(tp) = &mut t.kind {
            tp.frame = Some(TextFrame { fill: Some("#ffffff".into()), stroke: None, corner_radius_mm: 2.0, image: None, pattern: Some(Pattern::Lines), write_on_lines: None });
        }
        let content = export(&doc_with(vec![t]), &AssetStore::default()).xml("content.xml");
        let (rect, rest) = (content.find("<draw:rect").expect("fond du cadre"), content.find("<draw:frame").expect("texte"));
        let lines: Vec<usize> = content.match_indices("<draw:line").map(|(i, _)| i).collect();
        // Cadre de 40 mm de haut, une ligne tous les 8 mm : 4 lignes, entre le fond et le texte.
        assert_eq!(lines.len(), 4);
        assert!(lines.iter().all(|&i| rect < i && i < rest));
        assert!(content.contains(r#"svg:x1="10.000mm" svg:y1="208.000mm" svg:x2="200.000mm" svg:y2="208.000mm""#), "{content}");
    }

    /// Sur les lignes : hauteur de ligne d'au moins un pas, marge du haut égale à la descendante (la première ligne de
    /// base tombe à un pas du haut du cadre), en haut quel que soit l'alignement, l'interligne sans effet. Sans le
    /// réglage, ou sur des petits carreaux, rien ne change.
    #[test]
    fn text_written_on_lines_sits_one_pitch_apart() {
        let on = |pattern, write_on_lines, size_pt| {
            let mut t = text(RichText::plain("Léa\nTom"), None);
            if let ElementKind::Text(tp) = &mut t.kind {
                tp.style.size_pt = size_pt;
                tp.style.valign = Some(VAlign::Bottom);
                tp.style.line_spacing = Some(1.5);
                tp.frame = Some(TextFrame { pattern: Some(pattern), write_on_lines, ..TextFrame::default() });
            }
            export(&doc_with(vec![t]), &AssetStore::default()).xml("content.xml")
        };
        let content = on(Pattern::Seyes, Some(true), 14.0);
        assert!(content.contains(r#"style:line-height-at-least="8.000mm""#), "{content}");
        assert!(!content.contains("fo:line-height"), "{content}");
        // Andika 14 pt : 46 points à 600 ppp sous la ligne de base.
        assert!(content.contains(&format!(r#"fo:padding-top="{}""#, mm(46.0 * 25.4 / 600.0))), "{content}");
        assert!(content.contains(r#"draw:textarea-vertical-align="top""#), "{content}");
        assert!(on(Pattern::Lines, Some(true), 20.0).contains(r#"style:line-height-at-least="16.000mm""#));
        for content in [on(Pattern::Seyes, None, 14.0), on(Pattern::SmallSquares, Some(true), 14.0)] {
            assert!(content.contains(r#"fo:line-height="150%""#) && !content.contains("line-height-at-least"), "{content}");
            assert!(content.contains(r#"draw:textarea-vertical-align="bottom""#), "{content}");
        }
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
    fn nested_groups_are_nested_draw_g() {
        let mut d = doc_with(vec![picture_el("1F98A", Some("g")), picture_el("2600", Some("g")), picture_el("1F33B", Some("g")), picture_el("1F431", None)]);
        d.pages[0].elements[0].subgroups = vec!["f".into()];
        d.pages[0].elements[1].subgroups = vec!["f".into()];
        let content = export(&d, &AssetStore::default()).xml("content.xml");
        assert_eq!(content.matches("<draw:g>").count(), 2);
        // Le petit groupe (renard, soleil) est dans le grand, avec le tournesol ; le chat est dehors.
        let outer = content.split("<draw:g>").nth(1).unwrap();
        let inner = content.split("<draw:g>").nth(2).unwrap().split("</draw:g>").next().unwrap();
        assert!(inner.contains("openmoji-1F98A") && inner.contains("openmoji-2600") && !inner.contains("openmoji-1F33B"));
        let after_inner = content.split("</draw:g>").nth(1).unwrap();
        assert!(after_inner.contains("openmoji-1F33B") && !outer.contains("openmoji-1F431"));
        assert!(content.split("</draw:g>").nth(2).unwrap().contains("openmoji-1F431"));
    }

    /// L'ODG tel que LibreOffice le réenregistre pour ses images : chaque fichier de `Pictures/` renommé d'un nom
    /// haché (`Pictures/1000…N.svg`), octets inchangés ; `edit` peut changer les octets d'un fichier.
    fn renamed_pictures(odg: &[u8], edit: impl Fn(&str, Vec<u8>) -> Vec<u8>) -> Vec<u8> {
        let mut zip = ZipArchive::new(Cursor::new(odg.to_vec())).unwrap();
        let names: Vec<String> = zip.file_names().map(|n| n.unwrap().into_owned()).collect();
        let renamed: Vec<(String, String)> = names.iter().filter(|n| n.starts_with("Pictures/")).enumerate().map(|(i, n)| (n.clone(), format!("Pictures/100000000000{i:04}.svg"))).collect();
        let mut out = zip::ZipWriter::new(Cursor::new(Vec::new()));
        for n in &names {
            let mut bytes = Vec::new();
            zip.by_name(n).unwrap().read_to_end(&mut bytes).unwrap();
            let mut name = n.clone();
            if let Some((_, new)) = renamed.iter().find(|(old, _)| old == n) {
                bytes = edit(n, bytes);
                name = new.clone();
            } else if n.ends_with(".xml") {
                let mut x = String::from_utf8(bytes).unwrap();
                for (old, new) in &renamed {
                    x = x.replace(old.as_str(), new);
                }
                bytes = x.into_bytes();
            }
            out.start_file(name, zip::write::SimpleFileOptions::default()).unwrap();
            std::io::Write::write_all(&mut out, &bytes).unwrap();
        }
        out.finish().unwrap().into_inner()
    }

    /// Un pictogramme revient pictogramme après que LibreOffice a renommé son fichier, à colorier et en miroir
    /// compris : le commentaire du SVG le nomme. Une image remplacée (octets changés) n'est pas un pictogramme ; un
    /// fichier d'avant le commentaire est retrouvé parmi les images de la banque.
    #[test]
    fn a_picture_renamed_by_libreoffice_comes_back() {
        let outlined = Element { kind: ElementKind::Image(ImageProps { code: "1F431".into(), outline: Some(true) }), ..picture_el("1F431", None) };
        let mirrored = Element { flip_h: true, ..picture_el("2600", None) };
        let doc = doc_with(vec![picture_el("1F98A", None), outlined, mirrored]);
        let mut buf = Cursor::new(Vec::new());
        export_odg(&doc, &AssetStore::default(), &FontSet::builtin(), &mut buf).unwrap();
        let odg = buf.into_inner();
        let pictos = |odg: Vec<u8>| -> Vec<(String, Option<bool>, bool)> {
            let back = crate::import::odg::read_drawing(Cursor::new(odg), &[]).unwrap();
            back.pages[0].elements.iter().filter_map(|e| if let ElementKind::Image(i) = &e.kind { Some((i.code.clone(), i.outline, e.flip_h)) } else { None }).collect()
        };
        let all = vec![("1F98A".to_string(), None, false), ("1F431".to_string(), Some(true), false), ("2600".to_string(), None, true)];
        assert_eq!(pictos(renamed_pictures(&odg, |_, b| b)), all);
        let changed = renamed_pictures(&odg, |n, mut b| {
            if n.contains("1F98A") {
                b.extend(b"<!-- retouche -->");
            }
            b
        });
        assert_eq!(pictos(changed), all[1..]);
        // Avant le commentaire : le SVG de la banque tel quel ; retourné, il n'est pas retrouvé.
        let old = renamed_pictures(&odg, |_, b| b[b.iter().position(|c| *c == b'\n').unwrap() + 1..].to_vec());
        assert_eq!(pictos(old), all[..2]);
    }

    /// Le seul élément relu de la première page, hors mention de licence.
    fn only(back: &crate::import::Imported) -> &Element {
        assert_eq!(back.pages[0].elements.len(), 1, "{:?}", back.pages[0].elements);
        &back.pages[0].elements[0]
    }

    /// Une flèche revient flèche, sa pointe au même bout, retournée comprise (le bout de la pointe : la fin du trait).
    #[test]
    fn an_arrow_keeps_its_head() {
        let tip = |e: &Element| {
            let f = e.frame;
            let (sin, cos) = f.rotation_deg.to_radians().sin_cos();
            let half = if e.flip_h { -f.w_mm / 2.0 } else { f.w_mm / 2.0 };
            let r = |v: f64| (v * 100.0).round() / 100.0 + 0.0;
            (r(f.x_mm + f.w_mm / 2.0 + half * cos), r(f.y_mm + f.h_mm / 2.0 + half * sin))
        };
        for (flip, rotation) in [(false, 0.0), (true, 0.0), (false, 30.0), (true, 200.0)] {
            let mut arrow = Element { flip_h: flip, ..shape_el(ShapeKind::Arrow, Frame { rotation_deg: rotation, ..frame(20.0, 100.0, 60.0, 8.0) }) };
            if let ElementKind::Shape(s) = &mut arrow.kind {
                s.fill = None;
            }
            let back = round_trip(&doc_with(vec![arrow.clone()]), &AssetStore::default());
            let e = only(&back);
            let ElementKind::Shape(s) = &e.kind else { panic!("{e:?}") };
            assert_eq!(s.shape, ShapeKind::Arrow, "{flip} {rotation}");
            assert_eq!(s.stroke.as_ref().map(|s| s.width_mm), Some(1.0));
            assert_eq!(tip(e), tip(&arrow), "{flip} {rotation} : {e:?}");
            assert!((e.frame.w_mm - 60.0).abs() < 0.01);
        }
    }

    /// Tirets et pointillés reviennent tels quels sur une forme, le cadre d'un texte et la bordure d'une photo.
    #[test]
    fn dotted_and_dashed_strokes_come_back() {
        let mut store = AssetStore::default();
        let asset = raw_asset(40, 30);
        let id = asset.meta.id.clone();
        store.insert(asset).unwrap();
        for style in [StrokeStyle::Dashed, StrokeStyle::Dotted] {
            let st = Stroke { width_mm: 0.8, color: "#c0392b".into(), style: Some(style) };
            let mut shape = shape_el(ShapeKind::Rect, frame(20.0, 20.0, 40.0, 30.0));
            if let ElementKind::Shape(s) = &mut shape.kind {
                s.stroke = Some(st.clone());
            }
            let mut label = text(RichText::plain("Léa"), None);
            if let ElementKind::Text(t) = &mut label.kind {
                t.frame = Some(TextFrame { stroke: Some(st.clone()), ..TextFrame::default() });
            }
            let mut picture = photo(Some(&id), frame(120.0, 20.0, 40.0, 30.0), 0.0);
            if let ElementKind::Photo(p) = &mut picture.kind {
                p.border = Some(st.clone());
            }
            let back = round_trip(&doc_with(vec![shape, label, picture]), &store);
            let strokes: Vec<Option<StrokeStyle>> = back.pages[0]
                .elements
                .iter()
                .map(|e| match &e.kind {
                    ElementKind::Shape(s) => s.stroke.as_ref().and_then(|s| s.style),
                    ElementKind::Text(t) => t.frame.as_ref().and_then(|f| f.stroke.as_ref()).and_then(|s| s.style),
                    ElementKind::Photo(p) => p.border.as_ref().and_then(|s| s.style),
                    _ => None,
                })
                .collect();
            assert_eq!(strokes, [Some(style); 3], "{:?}", back.pages[0].elements);
        }
    }

    /// Une photo garde sa bordure (épaisseur, couleur) et son ombre.
    #[test]
    fn a_photo_keeps_its_border_and_shadow() {
        let mut store = AssetStore::default();
        let asset = raw_asset(40, 30);
        let id = asset.meta.id.clone();
        store.insert(asset).unwrap();
        let mut picture = photo(Some(&id), frame(20.0, 20.0, 80.0, 60.0), 2.0);
        if let ElementKind::Photo(p) = &mut picture.kind {
            p.border = Some(Stroke { width_mm: 1.5, color: "#2e86c1".into(), style: None });
            p.shadow = true;
        }
        let back = round_trip(&doc_with(vec![picture]), &store);
        let ElementKind::Photo(p) = &only(&back).kind else { panic!() };
        assert_eq!((p.border.clone(), p.shadow, p.corner_radius_mm), (Some(Stroke { width_mm: 1.5, color: "#2e86c1".into(), style: None }), true, 2.0));
    }

    /// L'image de fond d'un cadre de texte revient image de fond, droite ou tournée (ses pixels à leur place).
    #[test]
    fn a_text_frame_keeps_its_background_picture() {
        for rotation in [0.0, 30.0] {
            let mut store = AssetStore::default();
            let asset = halves_asset();
            let id = asset.meta.id.clone();
            store.insert(asset).unwrap();
            let mut label = text(RichText::plain("Léa"), None);
            label.frame = Frame { rotation_deg: rotation, ..frame(20.0, 20.0, 80.0, 40.0) };
            if let ElementKind::Text(t) = &mut label.kind {
                t.frame = Some(TextFrame { image: Some(id.clone()), corner_radius_mm: 3.0, ..TextFrame::default() });
            }
            let back = round_trip(&doc_with(vec![label]), &store);
            let ElementKind::Text(t) = &only(&back).kind else { panic!() };
            let image = t.frame.as_ref().and_then(|f| f.image.clone()).expect("image de fond");
            let asset = back.assets.iter().find(|a| a.meta.id == image).expect("image relue");
            let img = image::load_from_memory(&asset.bytes).unwrap().to_rgba8();
            // L'image de deux moitiés (rouge à gauche, bleue à droite) garde ses moitiés, à 2 : 1 comme le cadre.
            let (w, h) = img.dimensions();
            assert!((w as f64 / h as f64 - 2.0).abs() < 0.05, "{rotation} : {w} × {h}");
            assert!(is_red(img.get_pixel(w / 4, h / 2)) && is_blue(img.get_pixel(3 * w / 4, h / 2)), "{rotation}");
        }
    }

    /// L'interligne revient (un pourcentage de la hauteur naturelle) ; sans réglage, il reste absent.
    #[test]
    fn line_spacing_comes_back() {
        for spacing in [Some(1.5), Some(0.9), None] {
            let mut t = text(RichText::plain("Léa\nTom"), None);
            if let ElementKind::Text(tp) = &mut t.kind {
                tp.style.line_spacing = spacing;
            }
            let back = round_trip(&doc_with(vec![t]), &AssetStore::default());
            let ElementKind::Text(tp) = &only(&back).kind else { panic!() };
            assert_eq!(tp.style.line_spacing, spacing);
        }
    }

    /// Une étiquette encadrée vide (une case à remplir à la main) revient, avec son cadre.
    #[test]
    fn an_empty_framed_label_comes_back() {
        let mut t = text(RichText::default(), None);
        if let ElementKind::Text(tp) = &mut t.kind {
            tp.frame = Some(TextFrame { fill: Some("#fff1c1".into()), stroke: Some(Stroke { width_mm: 0.5, color: "#9aa5b1".into(), style: None }), corner_radius_mm: 3.0, ..TextFrame::default() });
        }
        let back = round_trip(&doc_with(vec![t.clone()]), &AssetStore::default());
        let e = only(&back);
        let (ElementKind::Text(a), ElementKind::Text(b)) = (&t.kind, &e.kind) else { panic!() };
        assert!(b.content.is_blank() && e.frame == t.frame);
        assert_eq!(a.frame, b.frame);
    }

    /// Une tabulation et un retour à la ligne dans un paragraphe reviennent tels quels.
    #[test]
    fn tabs_and_line_breaks_come_back() {
        let t = text(RichText { paragraphs: vec![Paragraph { align: None, runs: vec![Run::plain("Lundi\tMardi\nMercredi")], list: None }] }, None);
        let back = round_trip(&doc_with(vec![t]), &AssetStore::default());
        let ElementKind::Text(tp) = &only(&back).kind else { panic!() };
        assert_eq!(tp.content.paragraphs.len(), 1);
        assert_eq!(tp.content.paragraphs[0].runs.iter().map(|r| r.text.as_str()).collect::<String>(), "Lundi\tMardi\nMercredi");
    }

    /// Un élément verrouillé (photo simple ou décorée, pictogramme, forme, trait, texte) l'est dans LibreOffice (son
    /// style protège sa place et sa taille) et revient verrouillé ; les autres non.
    #[test]
    fn a_locked_element_comes_back_locked() {
        let mut store = AssetStore::default();
        let asset = raw_asset(40, 30);
        let id = asset.meta.id.clone();
        store.insert(asset).unwrap();
        let els = vec![
            photo(Some(&id), frame(10.0, 10.0, 40.0, 30.0), 0.0),
            photo(Some(&id), frame(60.0, 10.0, 40.0, 30.0), 3.0),
            picture_el("1F98A", None),
            shape_el(ShapeKind::Star, frame(10.0, 60.0, 30.0, 30.0)),
            shape_el(ShapeKind::Line, frame(60.0, 60.0, 30.0, 2.0)),
            text(RichText::plain("Léa"), None),
        ];
        for locked in [true, false] {
            let doc = doc_with(els.iter().cloned().map(|e| Element { locked, ..e }).collect());
            let content = export(&doc, &store).xml("content.xml");
            // Sans verrou, seule la mention de licence est protégée.
            assert_eq!(content.matches(r#"style:protect="position size""#).count() > 1, locked, "{content}");
            let back = round_trip(&doc, &store);
            assert_eq!(back.pages[0].elements.len(), els.len());
            assert!(back.pages[0].elements.iter().all(|e| e.locked == locked), "{:?}", back.pages[0].elements);
        }
    }

    /// Le motif du fond de page (dessiné en traits et en points sur la page maîtresse) revient motif, sur une page de
    /// chaque taille, avec sa couleur.
    #[test]
    fn a_page_pattern_comes_back() {
        let mut d = doc_with(vec![]);
        d.pages.clear();
        let sizes = [A4_PORTRAIT, PageSize { width_mm: 297.0, height_mm: 210.0 }, PageSize { width_mm: 148.0, height_mm: 210.0 }];
        for (i, pattern) in [Pattern::Seyes, Pattern::SmallSquares, Pattern::Lines, Pattern::Dots].into_iter().enumerate() {
            d.pages.push(Page { background: Some(Background { color: if i % 2 == 0 { "#ffffff".into() } else { "#fff8dc".into() }, pattern: Some(pattern) }), ..Page::new(sizes[i % 3]) });
        }
        let back = round_trip(&d, &AssetStore::default());
        let got: Vec<Option<Background>> = back.pages.iter().map(|p| p.background.clone()).collect();
        assert_eq!(got, d.pages.iter().map(|p| p.background.clone()).collect::<Vec<_>>());
    }

    /// Un cadre de texte à lignes d'écriture (fond, bordure et coins dessinés à part, les lignes ou les points, puis le
    /// texte) revient cadre à lignes, droit ou tourné, avec son fond, sa bordure, ses coins et l'écriture sur les lignes ;
    /// l'image de fond aussi.
    #[test]
    fn a_lined_text_frame_comes_back() {
        let mut store = AssetStore::default();
        let asset = raw_asset(40, 30);
        let id = asset.meta.id.clone();
        store.insert(asset).unwrap();
        let mut els = Vec::new();
        for (i, pattern) in [Pattern::Seyes, Pattern::SmallSquares, Pattern::Lines, Pattern::Dots].into_iter().enumerate() {
            for (j, rotation) in [0.0, 20.0].into_iter().enumerate() {
                let mut t = text(RichText::plain("Léa"), None);
                t.frame = Frame { rotation_deg: rotation, ..frame(10.0 + 100.0 * j as f64, 10.0 + 60.0 * i as f64, 80.0, 40.0) };
                // Verrouillé : son fond, ses lignes et son texte le sont tous dans LibreOffice.
                t.locked = j == 1;
                if let ElementKind::Text(tp) = &mut t.kind {
                    tp.frame = Some(TextFrame {
                        fill: (i % 2 == 0).then(|| "#fff1c1".into()),
                        stroke: Some(Stroke { width_mm: 0.5, color: "#9aa5b1".into(), style: if j == 1 { Some(StrokeStyle::Dashed) } else { None } }),
                        corner_radius_mm: 2.0 * i as f64,
                        image: (i == 3 && j == 0).then(|| id.clone()),
                        pattern: Some(pattern),
                        write_on_lines: matches!(pattern, Pattern::Seyes | Pattern::Lines).then_some(true),
                    });
                }
                els.push(t);
            }
        }
        // Une étiquette vide où l'on écrira sur les lignes.
        let mut blank = text(RichText::default(), None);
        blank.frame = frame(10.0, 250.0, 80.0, 40.0);
        if let ElementKind::Text(tp) = &mut blank.kind {
            tp.frame = Some(TextFrame { pattern: Some(Pattern::Seyes), write_on_lines: Some(true), ..TextFrame::default() });
        }
        els.push(blank);
        let d = doc_with(els);
        let back = round_trip(&d, &store);
        let frames = |els: &[Element]| -> Vec<(Option<TextFrame>, bool)> {
            els.iter().filter_map(|e| if let ElementKind::Text(t) = &e.kind { Some((t.frame.clone().map(|f| TextFrame { image: f.image.map(|_| "image".into()), ..f }), e.locked)) } else { None }).collect()
        };
        assert_eq!(back.pages[0].elements.len(), d.pages[0].elements.len(), "{:?}", back.pages[0].elements);
        assert_eq!(frames(&back.pages[0].elements), frames(&d.pages[0].elements));
    }

    /// L'ODG exporté, une partie XML changée par `edit`.
    fn edited_odg(doc: &Document, part: &str, edit: impl Fn(String) -> String) -> Vec<u8> {
        let mut buf = Cursor::new(Vec::new());
        export_odg(doc, &AssetStore::default(), &FontSet::builtin(), &mut buf).unwrap();
        let mut zip = ZipArchive::new(Cursor::new(buf.into_inner())).unwrap();
        let names: Vec<String> = zip.file_names().map(|n| n.unwrap().into_owned()).collect();
        let mut out = zip::ZipWriter::new(Cursor::new(Vec::new()));
        for n in &names {
            let mut bytes = Vec::new();
            zip.by_name(n).unwrap().read_to_end(&mut bytes).unwrap();
            if n == part {
                bytes = edit(String::from_utf8(bytes).unwrap()).into_bytes();
            }
            out.start_file(n, zip::write::SimpleFileOptions::default()).unwrap();
            std::io::Write::write_all(&mut out, &bytes).unwrap();
        }
        out.finish().unwrap().into_inner()
    }

    /// Un motif ou un cadre à lignes dont un seul trait a changé (ou manque) n'est pas reconnu : la page garde sa
    /// couleur, le cadre est relu tel qu'il est dessiné (un rectangle, des traits, un texte).
    #[test]
    fn a_pattern_is_recognised_only_when_everything_matches() {
        let mut t = text(RichText::plain("Léa"), None);
        if let ElementKind::Text(tp) = &mut t.kind {
            tp.frame = Some(TextFrame { pattern: Some(Pattern::Lines), stroke: Some(Stroke { width_mm: 0.5, color: "#000000".into(), style: None }), ..TextFrame::default() });
        }
        let d = with_background(doc_with(vec![t]), "#fff8dc", Some(Pattern::Lines));
        let read = |odg: Vec<u8>| crate::import::odg::read_drawing(Cursor::new(odg), &[]).unwrap();
        let one_line_less = |x: String| {
            let at = x.find("<draw:line").unwrap();
            let end = at + x[at..].find("/>").unwrap() + 2;
            format!("{}{}", &x[..at], &x[end..])
        };
        let back = read(edited_odg(&d, "content.xml", one_line_less));
        assert_eq!(back.pages[0].background.as_ref().map(|b| b.pattern), Some(Some(Pattern::Lines)));
        let kinds: Vec<&str> = back.pages[0].elements.iter().map(|e| match &e.kind {
            ElementKind::Shape(s) if s.shape == ShapeKind::Rect => "rectangle",
            ElementKind::Shape(_) => "trait",
            ElementKind::Text(t) if t.frame.as_ref().is_none_or(|f| f.pattern.is_none()) => "texte",
            _ => "autre",
        }).collect();
        assert_eq!(kinds, [&["rectangle"][..], &["trait"; 3], &["texte"]].concat());
        let back = read(edited_odg(&d, "styles.xml", one_line_less));
        assert_eq!(back.pages[0].background, Some(Background { color: "#fff8dc".into(), pattern: None }));
        // Sans sa dernière ligne, le motif n'est pas non plus celui d'un autre format.
        let last_line_less = |x: String| {
            let at = x.rfind("<draw:line").unwrap();
            let end = at + x[at..].find("/>").unwrap() + 2;
            format!("{}{}", &x[..at], &x[end..])
        };
        let back = read(edited_odg(&d, "styles.xml", last_line_less));
        assert_eq!(back.pages[0].background, Some(Background { color: "#fff8dc".into(), pattern: None }));
        assert!(matches!(&back.pages[0].elements[..], [e] if matches!(&e.kind, ElementKind::Text(t) if t.frame.as_ref().and_then(|f| f.pattern) == Some(Pattern::Lines))));
    }

    /// Un rectangle et un texte de même cadre, trop bas pour une ligne d'écriture, restent un rectangle et un texte.
    /// Un cadre à lignes dont le texte a été mis seul dans un groupe (dans LibreOffice) n'est pas reconnu : ses
    /// morceaux restent.
    #[test]
    fn a_lined_frame_is_not_made_up() {
        let mut t = text(RichText::plain("Léa"), None);
        t.frame = frame(20.0, 20.0, 80.0, 6.0);
        let mut rect = shape_el(ShapeKind::Rect, frame(20.0, 20.0, 80.0, 6.0));
        rect.locked = true;
        let back = round_trip(&doc_with(vec![rect, t.clone()]), &AssetStore::default());
        assert_eq!(back.pages[0].elements.len(), 2, "{:?}", back.pages[0].elements);
        if let ElementKind::Text(tp) = &mut t.kind {
            tp.frame = Some(TextFrame { pattern: Some(Pattern::Lines), ..TextFrame::default() });
        }
        t.frame = frame(20.0, 20.0, 80.0, 40.0);
        let mut star = shape_el(ShapeKind::Star, frame(120.0, 20.0, 20.0, 20.0));
        (t.group, star.group) = (Some("g".into()), Some("g".into()));
        // Le groupe ouvert juste avant le texte : le fond et les lignes restent dehors.
        let odg = edited_odg(&doc_with(vec![t, star]), "content.xml", |x| {
            let x = x.replacen("<draw:g>", "", 1);
            let at = x.find("<draw:frame").unwrap();
            format!("{}<draw:g>{}", &x[..at], &x[at..])
        });
        let back = crate::import::odg::read_drawing(Cursor::new(odg), &[]).unwrap();
        let els = &back.pages[0].elements;
        assert_eq!(els.iter().filter(|e| matches!(&e.kind, ElementKind::Shape(s) if s.shape == ShapeKind::Line)).count(), 4, "{els:?}");
        assert!(els.iter().any(|e| matches!(&e.kind, ElementKind::Text(t) if t.frame.is_none()) && e.group.is_some()));
    }

    /// Exporté puis relu par Cahutin (`read_drawing`).
    fn round_trip(doc: &Document, store: &AssetStore) -> crate::import::Imported {
        let fonts = FontSet::builtin();
        let mut buf = Cursor::new(Vec::new());
        export_odg(doc, store, &fonts, &mut buf).unwrap();
        buf.set_position(0);
        let known: Vec<String> = fonts.families().into_iter().map(|f| f.name).collect();
        crate::import::odg::read_drawing(buf, &known).unwrap()
    }

    /// Les chemins de groupe des éléments, chaque groupe renommé d'après son premier membre (les identifiants
    /// changent à l'import).
    fn group_paths(els: &[Element]) -> Vec<Vec<usize>> {
        let mut names: Vec<Id> = Vec::new();
        els.iter()
            .map(|e| {
                crate::groups::path(e)
                    .into_iter()
                    .map(|g| match names.iter().position(|n| *n == g) {
                        Some(i) => i,
                        None => {
                            names.push(g);
                            names.len() - 1
                        }
                    })
                    .collect()
            })
            .collect()
    }

    /// Un groupe, un groupe dans un groupe et ce qui est hors des groupes reviennent tels quels de LibreOffice.
    #[test]
    fn nested_groups_come_back() {
        let mut d = doc_with(vec![picture_el("1F98A", Some("g")), picture_el("2600", Some("g")), picture_el("1F33B", Some("g")), picture_el("1F431", None), shape_el(ShapeKind::Star, frame(60.0, 120.0, 20.0, 20.0)), shape_el(ShapeKind::Heart, frame(90.0, 120.0, 20.0, 20.0))]);
        d.pages[0].elements[0].subgroups = vec!["f".into()];
        d.pages[0].elements[1].subgroups = vec!["f".into()];
        d.pages[0].elements[4].group = Some("h".into());
        d.pages[0].elements[5].group = Some("h".into());
        let back = round_trip(&d, &AssetStore::default());
        let els = &back.pages[0].elements;
        assert_eq!(els.len(), 6, "{els:?}");
        assert_eq!(group_paths(els), group_paths(&d.pages[0].elements));
    }

    /// Un tableau, simple (vrai tableau) ou non (traits et cases dans un groupe), garde son groupe et rien que lui ;
    /// un groupe qui ne garde qu'un membre est défait.
    #[test]
    fn a_table_keeps_its_own_group_and_lonely_groups_are_undone() {
        for dashed in [false, true] {
            let (mut d, t) = table_doc(2, 2);
            if dashed {
                table_mut(&mut d, &t).border = Some(Stroke { width_mm: 1.0, color: "#c0392b".into(), style: Some(StrokeStyle::Dashed) });
            }
            let mut star = shape_el(ShapeKind::Star, frame(150.0, 150.0, 20.0, 20.0));
            star.group = Some("seule".into());
            d.pages[0].elements.push(star);
            let back = round_trip(&d, &AssetStore::default());
            let els = &back.pages[0].elements;
            let star = els.iter().find(|e| matches!(&e.kind, ElementKind::Shape(s) if s.shape == ShapeKind::Star)).unwrap();
            assert!(star.group.is_none(), "{dashed} : {star:?}");
            if !dashed {
                let table = els.iter().find(|e| matches!(e.kind, ElementKind::Table(_))).unwrap();
                assert!(table.subgroups.is_empty() && els.iter().filter(|e| e.group == table.group).count() == 5, "{els:?}");
            } else {
                // Tableau en traits : ses traits (et ses cases écrites) restent ensemble, dans un groupe.
                let g = els[0].group.clone().expect("groupe");
                let rest: Vec<&Element> = els.iter().filter(|e| e.id != star.id).collect();
                assert!(rest.len() >= 6 && rest.iter().all(|e| e.group.as_ref() == Some(&g) && e.subgroups.is_empty()), "{els:?}");
            }
        }
    }

    #[test]
    fn credit_line_only_on_pages_with_pictures() {
        let mut d = doc_with(vec![picture_el("1F98A", None)]);
        d.pages.push(Page::new(A4_PORTRAIT));
        let content = export(&d, &AssetStore::default()).xml("content.xml");
        let pages: Vec<&str> = content.split("<draw:page ").skip(1).collect();
        assert_eq!(pages.len(), 2);
        assert!(pages[0].contains(OPENMOJI_CREDIT_PAGE) && !content.contains("draw:protect"));
        // La mention est verrouillée par son style, ce que LibreOffice respecte.
        let credit_style = pages[0].split(OPENMOJI_CREDIT_PAGE).next().unwrap().rsplit(r#"<draw:frame draw:style-name=""#).next().unwrap().split('"').next().unwrap();
        assert!(style_def(&content, credit_style).contains(r#"style:protect="position size""#), "{content}");
        assert!(!pages[1].contains(OPENMOJI_CREDIT_PAGE));
    }

    /// Mention de licence : cadre de 5 mm entre 10 et 5 mm du bas, marges de 8 mm sur les côtés. Le texte (6 pt,
    /// marge de 1,5 mm) y laisse son encre à plus de 5 mm du bas, hors de la bande que l'imprimante n'atteint pas, et
    /// sous les cadres de page à 8 mm (banc : `openmoji_credit_ink_stays_5mm_from_the_bottom`).
    #[test]
    fn credit_frame_stays_out_of_the_unprintable_band() {
        let f = content::credit_frame(PageSize { width_mm: 210.0, height_mm: 297.0 });
        assert_eq!((f.x_mm, f.y_mm, f.w_mm, f.h_mm), (8.0, 287.0, 194.0, 5.0));
        let content = export(&doc_with(vec![picture_el("1F98A", None)]), &AssetStore::default()).xml("content.xml");
        assert!(content.contains(r#"svg:x="8.000mm" svg:y="287.000mm" svg:width="194.000mm" svg:height="5.000mm""#), "{content}");
    }

    #[test]
    fn meta_credits_only_when_pictures() {
        assert!(export(&doc_with(vec![picture_el("1F98A", None)]), &AssetStore::default()).xml("meta.xml").contains(OPENMOJI_CREDIT));
        assert!(!export(&doc_with(vec![]), &AssetStore::default()).xml("meta.xml").contains("OpenMoji"));
    }

    /// Chaque export porte la marque de Cahutin dans les propriétés du fichier, que LibreOffice garde en réenregistrant
    /// (24.2 et 26.8, mesuré) : un simple mot, ni titre, ni chemin, ni version.
    #[test]
    fn meta_carries_the_cahutin_mark() {
        let mark = format!(r#"<meta:user-defined meta:name="{CAHUTIN_MARK_NAME}" meta:value-type="string">{CAHUTIN_MARK}</meta:user-defined>"#);
        assert_eq!((CAHUTIN_MARK_NAME, CAHUTIN_MARK), ("Logiciel", "Cahutin"));
        for doc in [doc_with(vec![]), doc_with(vec![picture_el("1F98A", None)])] {
            assert!(export(&doc, &AssetStore::default()).xml("meta.xml").contains(&mark));
        }
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
