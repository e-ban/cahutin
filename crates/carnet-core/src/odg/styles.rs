//! Registre des styles automatiques, des polices, des images de remplissage et des pages ; `styles.xml`.

use super::NS;
use super::xml::Xml;
use crate::fonts::{self, FontFace, FontSet};
use crate::model::PageSize;

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
    pub page_sizes: Vec<PageSize>,
}

pub(crate) fn mm(v: f64) -> String {
    format!("{v:.3}mm")
}

impl Registry {
    pub fn new(fonts: &FontSet) -> Registry {
        Registry { fonts: fonts.clone(), styles: vec![], faces: vec![], fill_images: vec![], page_sizes: vec![] }
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
        let face = self
            .fonts
            .face(family, bold, italic)
            .or_else(|| self.fonts.face(fonts::DEFAULT_FAMILY, bold, italic))
            .expect("la police par défaut est embarquée")
            .clone();
        let name = face.family.clone();
        if !self.faces.iter().any(|f| f.file_name == face.file_name) {
            self.faces.push(face);
        }
        name
    }

    pub fn fill_image(&mut self, name: String, href: String) {
        self.fill_images.push((name, href));
    }

    /// Nom de la page maîtresse pour cette taille de page.
    pub fn master_page(&mut self, size: PageSize) -> String {
        let i = match self.page_sizes.iter().position(|s| *s == size) {
            Some(i) => i,
            None => {
                self.page_sizes.push(size);
                self.page_sizes.len() - 1
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
            let embedded: Vec<&FontFace> = self.faces.iter().filter(|f| f.family == family && f.embeddable).collect();
            if embedded.is_empty() {
                x.close();
                continue;
            }
            x.open("svg:font-face-src", &[]);
            for f in embedded {
                let href = format!("Fonts/{}", f.file_name);
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
        for (i, size) in self.page_sizes.iter().enumerate() {
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
        x.open("style:style", &[("style:name", "dp1"), ("style:family", "drawing-page")]);
        x.empty("style:drawing-page-properties", &[("draw:background-size", "border"), ("draw:fill", "none")]);
        x.close();
        x.close();
        x.open("office:master-styles", &[]);
        for i in 1..=self.page_sizes.len() {
            x.empty(
                "style:master-page",
                &[("style:name", &format!("M{i}")), ("style:page-layout-name", &format!("PM{i}")), ("draw:style-name", "dp1")],
            );
        }
        x.close();
        x.close();
        x.finish()
    }
}
