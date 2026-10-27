//! `content.xml` : une `draw:page` par page, un objet ODG par élément.

use super::styles::{Props, Registry, mm};
use super::xml::Xml;
use super::{NS, OdgError, TEXT_PADDING_MM, odf_transform};
use crate::crop::source_rect;
use crate::model::*;
use crate::photo::{self, AssetStore, MAX_LONG_EDGE_PX};
use image::imageops::FilterType;

pub(crate) struct Picture {
    pub path: String,
    pub media_type: &'static str,
    pub bytes: Vec<u8>,
}

fn geometry(f: &Frame) -> Vec<(&'static str, String)> {
    match odf_transform(f) {
        None => vec![("svg:x", mm(f.x_mm)), ("svg:y", mm(f.y_mm)), ("svg:width", mm(f.w_mm)), ("svg:height", mm(f.h_mm))],
        Some(t) => vec![("svg:width", mm(f.w_mm)), ("svg:height", mm(f.h_mm)), ("draw:transform", t)],
    }
}

fn stroke(s: Option<&Stroke>) -> Vec<(&'static str, String)> {
    match s {
        None => vec![("draw:stroke", "none".into())],
        Some(s) => vec![
            ("draw:stroke", "solid".into()),
            ("svg:stroke-width", mm(s.width_mm)),
            ("svg:stroke-color", s.color.clone()),
        ],
    }
}

fn shadow(on: bool) -> Vec<(&'static str, String)> {
    if !on {
        return vec![("draw:shadow", "hidden".into())];
    }
    vec![
        ("draw:shadow", "visible".into()),
        ("draw:shadow-offset-x", "1mm".into()),
        ("draw:shadow-offset-y", "1mm".into()),
        ("draw:shadow-color", "#808080".into()),
        ("draw:shadow-opacity", "50%".into()),
    ]
}

fn align(a: Align) -> &'static str {
    match a {
        Align::Left => "start",
        Align::Center => "center",
        Align::Right => "end",
        Align::Justify => "justify",
    }
}

fn attrs<'a>(v: &'a [(&'static str, String)]) -> Vec<(&'static str, &'a str)> {
    v.iter().map(|(k, v)| (*k, v.as_str())).collect()
}

struct Writer<'a> {
    reg: &'a mut Registry,
    store: &'a AssetStore,
    pictures: Vec<Picture>,
    body: Xml,
}

impl Writer<'_> {
    fn element(&mut self, e: &Element) -> Result<(), OdgError> {
        match &e.kind {
            ElementKind::Photo(p) => self.photo(e, p),
            ElementKind::Text(t) => {
                self.text(e, t);
                Ok(())
            }
            ElementKind::Shape(s) => {
                self.shape(e, s);
                Ok(())
            }
        }
    }

    /// L'image est prérecadrée : LibreOffice reçoit exactement les pixels visibles.
    fn photo(&mut self, e: &Element, p: &PhotoProps) -> Result<(), OdgError> {
        let Some(asset_id) = &p.asset else { return Ok(()) };
        let asset = self.store.get(asset_id).ok_or_else(|| OdgError::MissingAsset(asset_id.clone()))?;
        let img = image::load_from_memory(&asset.bytes).map_err(|_| OdgError::Image)?;
        let r = source_rect(img.width(), img.height(), e.frame.w_mm, e.frame.h_mm, &p.crop);
        let (x, y) = (r.x.round() as u32, r.y.round() as u32);
        let w = (r.w.round() as u32).clamp(1, img.width() - x);
        let h = (r.h.round() as u32).clamp(1, img.height() - y);
        let mut cropped = img.crop_imm(x, y, w, h);
        if w.max(h) > MAX_LONG_EDGE_PX {
            cropped = cropped.resize(MAX_LONG_EDGE_PX, MAX_LONG_EDGE_PX, FilterType::Lanczos3);
        }
        let mime = asset.meta.mime;
        let (ext, media_type) = match mime {
            Mime::Jpeg => ("jpg", "image/jpeg"),
            Mime::Png => ("png", "image/png"),
        };
        let path = format!("Pictures/{}.{ext}", e.id);
        let bytes = photo::encode(&cropped, mime).map_err(|_| OdgError::Image)?;
        self.pictures.push(Picture { path: path.clone(), media_type, bytes });

        let geo = geometry(&e.frame);
        let decorated = p.corner_radius_mm > 0.0 || p.border.is_some() || p.shadow;
        if !decorated {
            let style = self.reg.style(
                "graphic",
                "gr",
                vec![("style:graphic-properties", vec![("draw:stroke", "none".into()), ("draw:fill", "none".into())])],
            );
            let mut a = vec![("draw:style-name", style)];
            a.extend(geo);
            self.body.open("draw:frame", &attrs(&a));
            self.body.empty(
                "draw:image",
                &[("xlink:href", &path), ("xlink:type", "simple"), ("xlink:show", "embed"), ("xlink:actuate", "onLoad")],
            );
            self.body.close();
            return Ok(());
        }
        // Coins arrondis, bordure ou ombre : un rectangle rempli par l'image.
        let fill_name = format!("img_{}", e.id);
        self.reg.fill_image(fill_name.clone(), path);
        let mut props = vec![
            ("draw:fill", "bitmap".into()),
            ("draw:fill-image-name", fill_name),
            ("style:repeat", "stretch".into()),
        ];
        props.extend(stroke(p.border.as_ref()));
        props.extend(shadow(p.shadow));
        let style = self.reg.style("graphic", "gr", vec![("style:graphic-properties", props)]);
        let mut a = vec![("draw:style-name", style), ("draw:corner-radius", mm(p.corner_radius_mm))];
        a.extend(geo);
        self.body.empty("draw:rect", &attrs(&a));
        Ok(())
    }

    fn text_props(&mut self, run: &Run, style: &TextStyle) -> Props {
        let family = run.font.as_deref().unwrap_or(&style.font);
        let font_name = self.reg.font(family, run.bold, run.italic);
        vec![(
            "style:text-properties",
            vec![
                ("style:font-name", font_name.to_string()),
                ("fo:font-size", format!("{}pt", run.size_pt.unwrap_or(style.size_pt))),
                ("fo:font-weight", if run.bold { "bold" } else { "normal" }.into()),
                ("fo:font-style", if run.italic { "italic" } else { "normal" }.into()),
                ("style:text-underline-style", if run.underline { "solid" } else { "none" }.into()),
                ("fo:color", run.color.clone().unwrap_or_else(|| style.color.clone())),
            ],
        )]
    }

    fn text(&mut self, e: &Element, t: &TextProps) {
        if t.content.is_blank() {
            return;
        }
        let pad = mm(TEXT_PADDING_MM);
        let style = self.reg.style(
            "graphic",
            "gr",
            vec![(
                "style:graphic-properties",
                vec![
                    ("draw:stroke", "none".into()),
                    ("draw:fill", "none".into()),
                    ("fo:padding-top", pad.clone()),
                    ("fo:padding-bottom", pad.clone()),
                    ("fo:padding-left", pad.clone()),
                    ("fo:padding-right", pad),
                    ("draw:auto-grow-height", "false".into()),
                    ("draw:auto-grow-width", "false".into()),
                    ("draw:textarea-vertical-align", "top".into()),
                    ("draw:textarea-horizontal-align", "justify".into()),
                    ("fo:wrap-option", "wrap".into()),
                ],
            )],
        );
        let mut a = vec![("draw:style-name", style)];
        a.extend(geometry(&e.frame));
        self.body.open("draw:frame", &attrs(&a));
        self.body.open("draw:text-box", &[]);
        for p in &t.content.paragraphs {
            let mut props: Props = vec![(
                "style:paragraph-properties",
                vec![
                    ("fo:text-align", align(p.align.unwrap_or(t.style.align)).into()),
                    ("fo:line-height", "100%".into()),
                    ("fo:margin-top", "0mm".into()),
                    ("fo:margin-bottom", "0mm".into()),
                ],
            )];
            // Le style par défaut du paragraphe donne sa hauteur à une ligne vide.
            props.extend(self.text_props(&Run::plain(""), &t.style));
            let p_style = self.reg.style("paragraph", "P", props);
            self.body.open("text:p", &[("text:style-name", &p_style)]);
            let mut at_start = true;
            for run in &p.runs {
                if run.text.is_empty() {
                    continue;
                }
                let props = self.text_props(run, &t.style);
                let t_style = self.reg.style("text", "T", props);
                self.body.open("text:span", &[("text:style-name", &t_style)]);
                write_run_text(&mut self.body, &run.text, &mut at_start);
                self.body.close();
            }
            self.body.close();
        }
        self.body.close();
        self.body.close();
    }

    fn shape(&mut self, e: &Element, s: &ShapeProps) {
        let mut props = match &s.fill {
            Some(c) => vec![("draw:fill", "solid".into()), ("draw:fill-color", c.clone())],
            None => vec![("draw:fill", "none".into())],
        };
        props.extend(stroke(s.stroke.as_ref()));
        let style = self.reg.style("graphic", "gr", vec![("style:graphic-properties", props)]);
        let mut a = vec![("draw:style-name", style)];
        let name = match s.shape {
            ShapeKind::Rect => {
                a.push(("draw:corner-radius", mm(s.corner_radius_mm)));
                "draw:rect"
            }
            ShapeKind::Ellipse => "draw:ellipse",
        };
        a.extend(geometry(&e.frame));
        self.body.empty(name, &attrs(&a));
    }
}

/// Espaces multiples, tabulations et retours à la ligne, que l'ODF ne garde pas tels quels.
fn write_run_text(x: &mut Xml, text: &str, at_start: &mut bool) {
    let mut chars = text.chars().peekable();
    let mut word = String::new();
    while let Some(c) = chars.next() {
        match c {
            ' ' => {
                let mut n = 1;
                while chars.peek() == Some(&' ') {
                    chars.next();
                    n += 1;
                }
                x.text(&word);
                word.clear();
                if *at_start {
                    x.empty("text:s", &[("text:c", &n.to_string())]);
                } else {
                    x.text(" ");
                    if n > 1 {
                        x.empty("text:s", &[("text:c", &(n - 1).to_string())]);
                    }
                }
            }
            '\t' => {
                x.text(&word);
                word.clear();
                x.empty("text:tab", &[]);
            }
            // U+000B et U+000C : retours à la ligne manuels du texte brut copié depuis Word.
            '\n' | '\u{b}' | '\u{c}' => {
                x.text(&word);
                word.clear();
                x.empty("text:line-break", &[]);
            }
            // Les autres caractères de contrôle sont interdits en XML et rendraient le fichier illisible.
            c if c.is_control() => continue,
            c => word.push(c),
        }
        *at_start = false;
    }
    x.text(&word);
}

pub(crate) fn content_xml(doc: &Document, store: &AssetStore, reg: &mut Registry) -> Result<(String, Vec<Picture>), OdgError> {
    let mut w = Writer { reg, store, pictures: vec![], body: Xml::fragment() };
    for (i, page) in doc.pages.iter().enumerate() {
        let master = w.reg.master_page(page.size);
        w.body.open("draw:page", &[("draw:name", &format!("page{}", i + 1)), ("draw:master-page-name", &master)]);
        for e in &page.elements {
            w.element(e)?;
        }
        w.body.close();
    }
    let Writer { reg, pictures, body, .. } = w;

    let mut x = Xml::new();
    let mut root: Vec<(&str, &str)> = NS.to_vec();
    root.push(("office:version", "1.3"));
    x.open("office:document-content", &root);
    reg.write_font_decls(&mut x);
    reg.write_automatic_styles(&mut x);
    x.open("office:body", &[]);
    x.open("office:drawing", &[]);
    x.raw(&body.finish());
    x.close();
    x.close();
    x.close();
    Ok((x.finish(), pictures))
}
