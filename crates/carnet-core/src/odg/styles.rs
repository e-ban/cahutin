//! Registre des styles automatiques, des polices, des images de remplissage et des pages ; `styles.xml`.

use super::NS;
use super::xml::Xml;
use crate::fonts::{self, FontFace, FontSet};
use crate::background::{self, Mark};
use crate::model::{Background, PageSize};

pub(crate) type Props = Vec<(&'static str, Vec<(&'static str, String)>)>;

pub(crate) struct AutoStyle {
    pub name: String,
    pub family: &'static str,
    pub props: Props,
}

pub(crate) struct Registry {
    fonts: FontSet,
    pub styles: Vec<AutoStyle>,
    /// Fichiers de police réellement utilisés, dans l'ordre d'apparition.
    pub faces: Vec<FontFace>,
    /// (nom, chemin dans l'archive)
    pub fill_images: Vec<(String, String)>,
    /// Pages maîtresses : une par couple (taille, fond).
    pub masters: Vec<(PageSize, Option<Background>)>,
    /// Une flèche est présente : la pointe (`draw:marker`) est déclarée dans `styles.xml`.
    pub arrow_marker: bool,
}

/// Nom de la pointe de flèche, celui de LibreOffice.
pub(crate) const ARROW_MARKER: &str = "Arrow";

pub(crate) fn mm(v: f64) -> String {
    format!("{v:.3}mm")
}

impl Registry {
    pub fn new(fonts: &FontSet) -> Registry {
        Registry { fonts: fonts.clone(), styles: vec![], faces: vec![], fill_images: vec![], masters: vec![], arrow_marker: false }
    }

    /// Style automatique dédupliqué ; renvoie son nom.
    pub fn style(&mut self, family: &'static str, prefix: &str, props: Props) -> String {
        if let Some(s) = self.styles.iter().find(|s| s.family == family && s.props == props) {
            return s.name.clone();
        }
        let n = self.styles.iter().filter(|s| s.family == family).count() + 1;
        let name = format!("{prefix}{n}");
        self.styles.push(AutoStyle { name: name.clone(), family, props });
        name
    }

    /// Enregistre la police à intégrer ; renvoie le nom de famille à écrire dans `style:font-name`.
    pub fn font(&mut self, family: &str, bold: bool, italic: bool) -> String {
        // Une police du poste dont le fichier a disparu depuis le démarrage est remplacée par la police par défaut.
        let available = |f: &&FontFace| f.system_path().is_none_or(|p| p.is_file());
        let face = self
            .fonts
            .face(family, bold, italic)
            .filter(available)
            .or_else(|| self.fonts.face(fonts::DEFAULT_FAMILY, bold, italic))
            .expect("la police par défaut est embarquée")
            .clone();
        let name = face.family.clone();
        if !self.faces.iter().any(|f| f.family == face.family && f.bold == face.bold && f.italic == face.italic) {
            self.faces.push(face);
        }
        name
    }

    /// Nom de chaque police dans `Fonts/`, dans l'ordre de `faces` : deux fichiers de même nom restent distincts.
    pub fn archive_names(&self) -> Vec<String> {
        fonts::unique_names(self.faces.iter().map(|f| f.file_name.as_str()))
    }

    pub fn fill_image(&mut self, name: String, href: String) {
        self.fill_images.push((name, href));
    }

    /// Nom de la page maîtresse pour cette taille de page et ce fond.
    pub fn master_page(&mut self, size: PageSize, background: Option<&Background>) -> String {
        let key = (size, background.cloned());
        let i = match self.masters.iter().position(|m| *m == key) {
            Some(i) => i,
            None => {
                self.masters.push(key);
                self.masters.len() - 1
            }
        };
        format!("M{}", i + 1)
    }

    pub fn write_font_decls(&self, x: &mut Xml) {
        x.open("office:font-face-decls", &[]);
        let mut families: Vec<&str> = self.faces.iter().map(|f| f.family.as_str()).collect();
        families.sort();
        families.dedup();
        for family in families {
            x.open("style:font-face", &[("style:name", family), ("svg:font-family", family)]);
            // Licence restreinte : la police est déclarée sans fichier, LibreOffice prendra celle du poste.
            let names = self.archive_names();
            let embedded: Vec<(&FontFace, &String)> = self.faces.iter().zip(&names).filter(|(f, _)| f.family == family && f.embeddable).collect();
            if embedded.is_empty() {
                x.close();
                continue;
            }
            x.open("svg:font-face-src", &[]);
            for (f, name) in embedded {
                let href = format!("Fonts/{name}");
                x.open(
                    "svg:font-face-uri",
                    &[
                        ("xlink:href", &href),
                        ("xlink:type", "simple"),
                        ("loext:font-style", if f.italic { "italic" } else { "normal" }),
                        ("loext:font-weight", if f.bold { "bold" } else { "normal" }),
                    ],
                );
                x.empty("svg:font-face-format", &[("svg:string", f.format)]);
                x.close();
            }
            x.close();
            x.close();
        }
        x.close();
    }

    pub fn write_automatic_styles(&self, x: &mut Xml) {
        x.open("office:automatic-styles", &[]);
        for s in &self.styles {
            x.open("style:style", &[("style:name", &s.name), ("style:family", s.family)]);
            for (element, attrs) in &s.props {
                let attrs: Vec<(&str, &str)> = attrs.iter().map(|(k, v)| (*k, v.as_str())).collect();
                x.empty(element, &attrs);
            }
            x.close();
        }
        x.close();
    }

    pub fn styles_xml(&self) -> String {
        let mut x = Xml::new();
        let mut root: Vec<(&str, &str)> = NS.to_vec();
        root.push(("office:version", "1.3"));
        x.open("office:document-styles", &root);
        self.write_font_decls(&mut x);
        x.open("office:styles", &[]);
        if self.arrow_marker {
            // Pointe triangulaire standard de LibreOffice.
            x.empty("draw:marker", &[("draw:name", ARROW_MARKER), ("svg:viewBox", "0 0 20 30"), ("svg:d", "M10 0l-10 30h20z")]);
        }
        for (name, href) in &self.fill_images {
            x.empty(
                "draw:fill-image",
                &[
                    ("draw:name", name),
                    ("xlink:href", href),
                    ("xlink:type", "simple"),
                    ("xlink:show", "embed"),
                    ("xlink:actuate", "onLoad"),
                ],
            );
        }
        x.close();
        x.open("office:automatic-styles", &[]);
        for (i, (size, _)) in self.masters.iter().enumerate() {
            let (w, h) = (mm(size.width_mm), mm(size.height_mm));
            let orientation = if size.width_mm > size.height_mm { "landscape" } else { "portrait" };
            x.open("style:page-layout", &[("style:name", &format!("PM{}", i + 1))]);
            x.empty(
                "style:page-layout-properties",
                &[
                    ("fo:margin-top", "0mm"),
                    ("fo:margin-bottom", "0mm"),
                    ("fo:margin-left", "0mm"),
                    ("fo:margin-right", "0mm"),
                    ("fo:page-width", &w),
                    ("fo:page-height", &h),
                    ("style:print-orientation", orientation),
                ],
            );
            x.close();
        }
        // Fond de chaque page maîtresse : couleur de remplissage ; styles des traits et points du motif.
        let mut mark_styles: Vec<(bool, String, String)> = Vec::new();
        for (i, (_, bg)) in self.masters.iter().enumerate() {
            x.open("style:style", &[("style:name", &format!("dp{}", i + 1)), ("style:family", "drawing-page")]);
            match bg {
                Some(b) => x.empty("style:drawing-page-properties", &[("draw:background-size", "border"), ("draw:fill", "solid"), ("draw:fill-color", &b.color)]),
                None => x.empty("style:drawing-page-properties", &[("draw:background-size", "border"), ("draw:fill", "none")]),
            }
            x.close();
            for m in bg.iter().filter_map(|b| b.pattern).flat_map(|p| background::marks(self.masters[i].0, p)) {
                let key = match m {
                    Mark::Line { width_mm, color, .. } => (true, mm(width_mm), color.to_string()),
                    Mark::Dot { color, .. } => (false, String::new(), color.to_string()),
                };
                if !mark_styles.contains(&key) {
                    mark_styles.push(key);
                }
            }
        }
        for (n, (line, width, color)) in mark_styles.iter().enumerate() {
            x.open("style:style", &[("style:name", &format!("bg{}", n + 1)), ("style:family", "graphic")]);
            if *line {
                x.empty("style:graphic-properties", &[("draw:stroke", "solid"), ("svg:stroke-width", width), ("svg:stroke-color", color), ("draw:fill", "none")]);
            } else {
                x.empty("style:graphic-properties", &[("draw:stroke", "none"), ("draw:fill", "solid"), ("draw:fill-color", color)]);
            }
            x.close();
        }
        x.close();
        x.open("office:master-styles", &[]);
        for (i, (size, bg)) in self.masters.iter().enumerate() {
            let n = i + 1;
            x.open("style:master-page", &[("style:name", &format!("M{n}")), ("style:page-layout-name", &format!("PM{n}")), ("draw:style-name", &format!("dp{n}"))]);
            // Motif d'école : dessiné sur la page maîtresse, derrière tout et non sélectionnable.
            for m in bg.iter().filter_map(|b| b.pattern).flat_map(|p| background::marks(*size, p)) {
                match m {
                    Mark::Line { x1, y1, x2, y2, width_mm, color } => {
                        let style = format!("bg{}", mark_styles.iter().position(|k| *k == (true, mm(width_mm), color.to_string())).unwrap() + 1);
                        x.empty("draw:line", &[("draw:style-name", &style), ("draw:layer", "backgroundobjects"), ("svg:x1", &mm(x1)), ("svg:y1", &mm(y1)), ("svg:x2", &mm(x2)), ("svg:y2", &mm(y2))]);
                    }
                    Mark::Dot { x: cx, y: cy, d_mm, color } => {
                        let style = format!("bg{}", mark_styles.iter().position(|k| *k == (false, String::new(), color.to_string())).unwrap() + 1);
                        let r = d_mm / 2.0;
                        x.empty("draw:ellipse", &[("draw:style-name", &style), ("draw:layer", "backgroundobjects"), ("svg:x", &mm(cx - r)), ("svg:y", &mm(cy - r)), ("svg:width", &mm(d_mm)), ("svg:height", &mm(d_mm))]);
                    }
                }
            }
            x.close();
        }
        x.close();
        x.close();
        x.finish()
    }
}
