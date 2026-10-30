//! `content.xml` : une `draw:page` par page, un objet ODG par élément.

use super::styles::{ARROW_MARKER, Props, Registry, dash_name, mm};
use super::xml::Xml;
use super::{NS, OPENMOJI_CREDIT_PAGE, OdgError, odf_transform};
use crate::crop::source_rect;
use crate::model::*;
use crate::photo::{self, AssetStore, MAX_LONG_EDGE_PX};
use crate::picto;
use crate::shapes;
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

/// Largeur de la pointe d'une flèche : quatre fois le trait, 4 mm au moins (comme à l'écran).
pub(crate) fn arrow_head_mm(s: &ShapeProps) -> f64 {
    (s.stroke.as_ref().map_or(1.0, |st| st.width_mm) * 4.0).max(4.0)
}

/// Extrémités d'un trait : milieux des bords gauche et droit du cadre, tournés autour de son centre
/// (sens horaire, comme à l'écran).
fn line_ends(f: &Frame) -> Vec<(&'static str, String)> {
    let (cx, cy) = (f.x_mm + f.w_mm / 2.0, f.y_mm + f.h_mm / 2.0);
    let (sin, cos) = f.rotation_deg.to_radians().sin_cos();
    let end = |dx: f64| (cx + dx * cos, cy + dx * sin);
    let ((x1, y1), (x2, y2)) = (end(-f.w_mm / 2.0), end(f.w_mm / 2.0));
    let r = |v: f64| mm((v * 1000.0).round() / 1000.0 + 0.0);
    vec![("svg:x1", r(x1)), ("svg:y1", r(y1)), ("svg:x2", r(x2)), ("svg:y2", r(y2))]
}

/// Trait plein, ou tirets déclarés une fois dans `styles.xml`.
fn stroke(reg: &mut Registry, s: Option<&Stroke>) -> Vec<(&'static str, String)> {
    let Some(s) = s else { return vec![("draw:stroke", "none".into())] };
    let style = s.style.unwrap_or_default();
    let mut out = match dash_name(style) {
        None => vec![("draw:stroke", "solid".into())],
        Some(name) => {
            if !reg.dashes.contains(&style) {
                reg.dashes.push(style);
            }
            vec![("draw:stroke", "dash".into()), ("draw:stroke-dash", name.into())]
        }
    };
    out.push(("svg:stroke-width", mm(s.width_mm)));
    out.push(("svg:stroke-color", s.color.clone()));
    out
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
    /// Le prochain cadre de texte est protégé (mention de licence).
    protect_next: bool,
}

/// Mention de licence : 6 pt, gris, centrée dans la marge du bas (même cadre qu'à l'écran).
pub const CREDIT_SIZE_PT: f64 = 6.0;
pub const CREDIT_COLOR: &str = "#888888";
pub const CREDIT_MARGIN_MM: f64 = 8.0;

pub fn credit_frame(size: PageSize) -> Frame {
    Frame { x_mm: CREDIT_MARGIN_MM, y_mm: size.height_mm - CREDIT_MARGIN_MM, w_mm: size.width_mm - 2.0 * CREDIT_MARGIN_MM, h_mm: 5.0, rotation_deg: 0.0 }
}

pub(crate) fn has_pictures(doc: &Document) -> bool {
    doc.pages.iter().any(page_has_pictures)
}

fn page_has_pictures(page: &Page) -> bool {
    page.elements.iter().any(|e| matches!(e.kind, ElementKind::Image(_)))
}

impl Writer<'_> {
    fn element(&mut self, e: &Element) -> Result<(), OdgError> {
        match &e.kind {
            ElementKind::Photo(p) => self.photo(e, p),
            ElementKind::Text(t) => self.text(e, t),
            ElementKind::Shape(s) => {
                self.shape(e, s);
                Ok(())
            }
            ElementKind::Image(i) => self.picture(e, &i.code),
            ElementKind::Table(_) => Ok(()),
        }
    }

    /// Image OpenMoji : le SVG d'origine, ajouté une seule fois à l'archive.
    fn picture(&mut self, e: &Element, code: &str) -> Result<(), OdgError> {
        let svg = picto::svg(code).ok_or_else(|| OdgError::MissingPicture(code.to_string()))?;
        let path = format!("Pictures/openmoji-{code}.svg");
        if !self.pictures.iter().any(|p| p.path == path) {
            self.pictures.push(Picture { path: path.clone(), media_type: "image/svg+xml", bytes: svg.to_vec() });
        }
        let style = self.reg.style("graphic", "gr", vec![("style:graphic-properties", vec![("draw:stroke", "none".into()), ("draw:fill", "none".into())])]);
        let mut a = vec![("draw:style-name", style)];
        a.extend(geometry(&e.frame));
        self.body.open("draw:frame", &attrs(&a));
        self.body.empty("draw:image", &[("xlink:href", &path), ("xlink:type", "simple"), ("xlink:show", "embed"), ("xlink:actuate", "onLoad")]);
        self.body.close();
        Ok(())
    }

    /// Mention de licence OpenMoji, en bas de la page, verrouillée dans LibreOffice.
    fn credit(&mut self, page: &Page) {
        let f = credit_frame(page.size);
        let mut style = TextStyle::default_zone();
        style.size_pt = CREDIT_SIZE_PT;
        style.color = CREDIT_COLOR.into();
        style.align = Align::Center;
        let t = TextProps { content: RichText::plain(OPENMOJI_CREDIT_PAGE), style, prompt: None, frame: None };
        let e = Element { id: String::new(), frame: f, locked: true, zone: false, group: None, kind: ElementKind::Text(t.clone()) };
        self.protect_next = true;
        // Sans image de fond, l'écriture du texte ne peut pas échouer.
        let _ = self.text(&e, &t);
    }

    /// Image de la photothèque prérecadrée pour le cadre : LibreOffice reçoit exactement les pixels visibles.
    /// Renvoie son chemin dans l'archive.
    fn cropped_picture(&mut self, asset_id: &Id, f: &Frame, crop: &Crop, name: &str) -> Result<String, OdgError> {
        let asset = self.store.get(asset_id).ok_or_else(|| OdgError::MissingAsset(asset_id.clone()))?;
        let img = image::load_from_memory(&asset.bytes).map_err(|_| OdgError::Image)?;
        let r = source_rect(img.width(), img.height(), f.w_mm, f.h_mm, crop);
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
        let path = format!("Pictures/{name}.{ext}");
        let bytes = photo::encode(&cropped, mime).map_err(|_| OdgError::Image)?;
        self.pictures.push(Picture { path: path.clone(), media_type, bytes });
        Ok(path)
    }

    fn photo(&mut self, e: &Element, p: &PhotoProps) -> Result<(), OdgError> {
        let Some(asset_id) = &p.asset else { return Ok(()) };
        let path = self.cropped_picture(asset_id, &e.frame, &p.crop, &e.id)?;
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
        props.extend(stroke(self.reg, p.border.as_ref()));
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
                ("style:font-name", font_name),
                ("fo:font-size", format!("{}pt", run.size_pt.unwrap_or(style.size_pt))),
                ("fo:font-weight", if run.bold { "bold" } else { "normal" }.into()),
                ("fo:font-style", if run.italic { "italic" } else { "normal" }.into()),
                ("style:text-underline-style", if run.underline { "solid" } else { "none" }.into()),
                ("fo:color", run.color.clone().unwrap_or_else(|| style.color.clone())),
            ],
        )]
    }

    fn text(&mut self, e: &Element, t: &TextProps) -> Result<(), OdgError> {
        // Un texte vide disparaît, sauf s'il a un cadre (une étiquette à remplir) ; l'invite n'est jamais exportée.
        let blank = t.content.is_blank();
        if blank && t.frame.is_none() {
            return Ok(());
        }
        let pad = mm(t.style.padding());
        let valign = match t.style.valign.unwrap_or_default() {
            VAlign::Top => "top",
            VAlign::Middle => "middle",
            VAlign::Bottom => "bottom",
        };
        // Cadre : fond et bordure natifs du cadre de texte (le texte reste modifiable d'un clic dans LibreOffice).
        // Une image de fond couvre le cadre (recadrée au centre) et remplace la couleur.
        let frame = t.frame.as_ref();
        let mut props: Vec<(&'static str, String)> = match (frame.and_then(|f| f.image.as_ref()), frame.and_then(|f| f.fill.clone())) {
            (Some(image), _) => {
                let path = self.cropped_picture(image, &e.frame, &Crop::COVER, &format!("{}-fond", e.id))?;
                let fill_name = format!("img_{}_fond", e.id);
                self.reg.fill_image(fill_name.clone(), path);
                vec![("draw:fill", "bitmap".into()), ("draw:fill-image-name", fill_name), ("style:repeat", "stretch".into())]
            }
            (None, Some(c)) => vec![("draw:fill", "solid".into()), ("draw:fill-color", c)],
            (None, None) => vec![("draw:fill", "none".into())],
        };
        props.extend(stroke(self.reg, frame.and_then(|f| f.stroke.as_ref())));
        let style = self.reg.style(
            "graphic",
            "gr",
            vec![(
                "style:graphic-properties",
                [props, vec![
                    ("fo:padding-top", pad.clone()),
                    ("fo:padding-bottom", pad.clone()),
                    ("fo:padding-left", pad.clone()),
                    ("fo:padding-right", pad),
                    ("draw:auto-grow-height", "false".into()),
                    ("draw:auto-grow-width", "false".into()),
                    ("draw:textarea-vertical-align", valign.into()),
                    ("draw:textarea-horizontal-align", "justify".into()),
                    ("fo:wrap-option", "wrap".into()),
                ]]
                .concat(),
            )],
        );
        let mut a = vec![("draw:style-name", style)];
        a.extend(geometry(&e.frame));
        if std::mem::take(&mut self.protect_next) {
            a.push(("draw:protect", "position size".into()));
        }
        self.body.open("draw:frame", &attrs(&a));
        let radius = frame.map(|f| f.corner_radius_mm).filter(|r| *r > 0.0).map(mm);
        let box_attrs: Vec<(&str, &str)> = radius.iter().map(|r| ("draw:corner-radius", r.as_str())).collect();
        self.body.open("draw:text-box", &box_attrs);
        for p in t.content.paragraphs.iter().filter(|_| !blank) {
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
        Ok(())
    }

    fn shape(&mut self, e: &Element, s: &ShapeProps) {
        let stroke_only = shapes::is_stroke_only(s.shape);
        let mut props = match &s.fill {
            Some(c) if !stroke_only => vec![("draw:fill", "solid".into()), ("draw:fill-color", c.clone())],
            _ => vec![("draw:fill", "none".into())],
        };
        props.extend(stroke(self.reg, s.stroke.as_ref()));
        if s.shape == ShapeKind::Arrow {
            self.reg.arrow_marker = true;
            props.push(("draw:marker-end", ARROW_MARKER.into()));
            props.push(("draw:marker-end-width", mm(arrow_head_mm(s))));
        }
        let style = self.reg.style("graphic", "gr", vec![("style:graphic-properties", props)]);
        let mut a = vec![("draw:style-name", style)];
        if stroke_only {
            a.extend(line_ends(&e.frame));
            self.body.empty("draw:line", &attrs(&a));
            return;
        }
        if let Some(path) = shapes::path(s.shape) {
            a.extend(geometry(&e.frame));
            self.body.open("draw:custom-shape", &attrs(&a));
            let viewbox = format!("0 0 {0} {0}", shapes::VIEWBOX);
            let enhanced = format!("{path} N");
            self.body.empty("draw:enhanced-geometry", &[("svg:viewBox", viewbox.as_str()), ("draw:enhanced-path", enhanced.as_str())]);
            self.body.close();
            return;
        }
        let name = match s.shape {
            ShapeKind::Rect => {
                a.push(("draw:corner-radius", mm(s.corner_radius_mm)));
                "draw:rect"
            }
            _ => "draw:ellipse",
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
    let mut w = Writer { reg, store, pictures: vec![], body: Xml::fragment(), protect_next: false };
    for (i, page) in doc.pages.iter().enumerate() {
        let master = w.reg.master_page(page.size, page.background.as_ref());
        w.body.open("draw:page", &[("draw:name", &format!("page{}", i + 1)), ("draw:master-page-name", &master)]);
        // Les membres consécutifs d'un même groupe sont écrits dans un groupe LibreOffice.
        let mut open_group: Option<&str> = None;
        for e in &page.elements {
            let g = e.group.as_deref();
            if open_group.is_some() && open_group != g {
                w.body.close();
                open_group = None;
            }
            if g.is_some() && open_group.is_none() {
                w.body.open("draw:g", &[]);
                open_group = g;
            }
            w.element(e)?;
        }
        if open_group.is_some() {
            w.body.close();
        }
        if page_has_pictures(page) {
            w.credit(page);
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
