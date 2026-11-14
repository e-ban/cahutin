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
use image::{DynamicImage, RgbaImage};

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

/// Mention de licence : 6 pt, gris, centrée en bas de la page (même cadre qu'à l'écran).
pub const CREDIT_SIZE_PT: f64 = 6.0;
pub const CREDIT_COLOR: &str = "#888888";
pub const CREDIT_MARGIN_MM: f64 = 8.0;
/// Haut du cadre de la mention, depuis le bas de la page : son encre tombe entre 7,5 et 5,5 mm du bas, au-dessus
/// de la bande de 5 mm que beaucoup d'imprimantes n'atteignent pas, et sous les cadres de page posés à 8 mm.
pub const CREDIT_BOTTOM_MM: f64 = 10.0;

pub fn credit_frame(size: PageSize) -> Frame {
    Frame { x_mm: CREDIT_MARGIN_MM, y_mm: size.height_mm - CREDIT_BOTTOM_MM, w_mm: size.width_mm - 2.0 * CREDIT_MARGIN_MM, h_mm: 5.0, rotation_deg: 0.0 }
}

/// Marque du nom d'une image de remplissage posée déjà tournée (voir `turned_box_mm`) : l'import la redresse.
pub(crate) const TURNED_FILL: &str = "-tourne";

/// Boîte droite (largeur, hauteur en mm) qui entoure le cadre tourné, aux coins arrondis de `radius_mm` : LibreOffice
/// y étire l'image qui remplit une forme tournée, sans la tourner. L'export l'y pose donc déjà tournée.
pub(crate) fn turned_box_mm(f: &Frame, radius_mm: f64) -> (f64, f64) {
    let r = radius_mm.clamp(0.0, f.w_mm.min(f.h_mm) / 2.0);
    let (sin, cos) = f.rotation_deg.to_radians().sin_cos();
    let (sin, cos) = (sin.abs(), cos.abs());
    let (w, h) = (f.w_mm - 2.0 * r, f.h_mm - 2.0 * r);
    (w * cos + h * sin + 2.0 * r, w * sin + h * cos + 2.0 * r)
}

/// L'image `src`, qui couvre `src_mm` (largeur, hauteur), reprise en une image de `out_mm` à `px_per_mm` pixels par
/// mm, tournée de `deg` (sens horaire) autour de leur centre commun. Bilinéaire ; les bords sont prolongés (pas de
/// liseré clair sous une bordure).
pub(crate) fn turned_image(src: &RgbaImage, src_mm: (f64, f64), out_mm: (f64, f64), px_per_mm: f64, deg: f64) -> RgbaImage {
    let (ow, oh) = (((out_mm.0 * px_per_mm).round() as u32).max(1), ((out_mm.1 * px_per_mm).round() as u32).max(1));
    let (sx, sy) = (src.width() as f64 / src_mm.0, src.height() as f64 / src_mm.1);
    let (sin, cos) = deg.to_radians().sin_cos();
    let (mx, my) = (src.width() as f64 - 1.0, src.height() as f64 - 1.0);
    RgbaImage::from_fn(ow, oh, |x, y| {
        // Point de la sortie, depuis le centre, ramené dans la source par la rotation inverse.
        let (dx, dy) = ((x as f64 + 0.5) / px_per_mm - out_mm.0 / 2.0, (y as f64 + 0.5) / px_per_mm - out_mm.1 / 2.0);
        let (lx, ly) = (dx * cos + dy * sin, -dx * sin + dy * cos);
        let u = ((lx + src_mm.0 / 2.0) * sx - 0.5).clamp(0.0, mx);
        let v = ((ly + src_mm.1 / 2.0) * sy - 0.5).clamp(0.0, my);
        let (x0, y0) = (u.floor() as u32, v.floor() as u32);
        let (x1, y1) = ((x0 + 1).min(src.width() - 1), (y0 + 1).min(src.height() - 1));
        let (fx, fy) = (u - x0 as f64, v - y0 as f64);
        let (a, b, c, d) = (src.get_pixel(x0, y0).0, src.get_pixel(x1, y0).0, src.get_pixel(x0, y1).0, src.get_pixel(x1, y1).0);
        image::Rgba(std::array::from_fn(|i| {
            let top = a[i] as f64 * (1.0 - fx) + b[i] as f64 * fx;
            let bottom = c[i] as f64 * (1.0 - fx) + d[i] as f64 * fx;
            (top * (1.0 - fy) + bottom * fy).round() as u8
        }))
    })
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
            ElementKind::Image(i) => self.picture(e, &i.code, i.outline == Some(true)),
            ElementKind::Table(t) => {
                self.table(e, t);
                Ok(())
            }
        }
    }

    /// Image OpenMoji : le SVG d'origine, ajouté une seule fois à l'archive.
    fn picture(&mut self, e: &Element, code: &str, outline: bool) -> Result<(), OdgError> {
        // En miroir : une copie retournée, nommée `…-miroir-h.svg` (`-v`, `-hv`) pour être relue en miroir.
        let mirror = match (e.flip_h, e.flip_v) {
            (false, false) => "",
            (true, false) => "-miroir-h",
            (false, true) => "-miroir-v",
            (true, true) => "-miroir-hv",
        };
        let base = if outline { format!("Pictures/openmoji-contour-{}", picto::base_code(code)) } else { format!("Pictures/openmoji-{code}") };
        let path = format!("{base}{mirror}.svg");
        // Décompressée seulement si elle n'est pas déjà dans l'archive (un code inconnu n'y est jamais).
        if !self.pictures.iter().any(|p| p.path == path) {
            let svg = if outline { picto::outline_svg(code) } else { picto::svg(code) }.ok_or_else(|| OdgError::MissingPicture(code.to_string()))?;
            let bytes = if mirror.is_empty() { Some(svg) } else { picto::mirrored_svg(&svg, e.flip_h, e.flip_v) };
            self.pictures.push(Picture { path: path.clone(), media_type: "image/svg+xml", bytes: bytes.ok_or_else(|| OdgError::MissingPicture(code.to_string()))? });
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
        let e = Element { id: String::new(), frame: f, locked: true, zone: false, group: None, kind: ElementKind::Text(t.clone()), subgroups: Vec::new(), flip_h: false, flip_v: false };
        self.protect_next = true;
        // Sans image de fond, l'écriture du texte ne peut pas échouer.
        let _ = self.text(&e, &t);
    }

    /// Image de la photothèque prérecadrée pour le cadre : LibreOffice reçoit exactement les pixels visibles.
    /// `fill_radius` : l'image remplit une forme aux coins arrondis de ce rayon ; tournée, elle est posée tournée
    /// dans la boîte droite qui entoure la forme (voir `turned_box_mm`). Renvoie son chemin dans l'archive.
    fn cropped_picture(&mut self, asset_id: &Id, f: &Frame, crop: &Crop, (flip_h, flip_v): (bool, bool), fill_radius: Option<f64>, name: &str) -> Result<String, OdgError> {
        let asset = self.store.get(asset_id).ok_or_else(|| OdgError::MissingAsset(asset_id.clone()))?;
        let img = image::load_from_memory(&asset.bytes).map_err(|_| OdgError::Image)?;
        let r = source_rect(img.width(), img.height(), f.w_mm, f.h_mm, crop);
        let (x, y) = (r.x.round() as u32, r.y.round() as u32);
        let w = (r.w.round() as u32).clamp(1, img.width() - x);
        let h = (r.h.round() as u32).clamp(1, img.height() - y);
        let mut cropped = img.crop_imm(x, y, w, h);
        if flip_h {
            cropped = cropped.fliph();
        }
        if flip_v {
            cropped = cropped.flipv();
        }
        if let Some(radius) = fill_radius.filter(|_| f.rotation_deg != 0.0) {
            let px_per_mm = (w as f64 / f.w_mm).max(h as f64 / f.h_mm);
            cropped = DynamicImage::ImageRgba8(turned_image(&cropped.to_rgba8(), (f.w_mm, f.h_mm), turned_box_mm(f, radius), px_per_mm, f.rotation_deg));
        }
        if cropped.width().max(cropped.height()) > MAX_LONG_EDGE_PX {
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
        let decorated = p.corner_radius_mm > 0.0 || p.border.is_some() || p.shadow;
        let path = self.cropped_picture(asset_id, &e.frame, &p.crop, (p.mirrored || e.flip_h, e.flip_v), decorated.then_some(p.corner_radius_mm), &e.id)?;
        let geo = geometry(&e.frame);
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
        // Coins arrondis, bordure ou ombre : un rectangle rempli par l'image (tournée, marquée pour l'import).
        let fill_name = format!("img_{}{}", e.id, if e.frame.rotation_deg != 0.0 { TURNED_FILL } else { "" });
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
        let mut props = vec![
            ("style:font-name", font_name),
            ("fo:font-size", format!("{}pt", run.size_pt.unwrap_or(style.size_pt))),
            ("fo:font-weight", if run.bold { "bold" } else { "normal" }.into()),
            ("fo:font-style", if run.italic { "italic" } else { "normal" }.into()),
            ("style:text-underline-style", if run.underline { "solid" } else { "none" }.into()),
            ("fo:color", run.color.clone().unwrap_or_else(|| style.color.clone())),
        ];
        // Lettres creuses : l'effet « contour » de LibreOffice, dessiné dans la couleur du texte.
        if style.outline == Some(true) {
            props.push(("style:text-outline", "true".into()));
        }
        vec![("style:text-properties", props)]
    }

    /// Fond et bordure d'un cadre à lignes d'écriture, puis ses lignes (ou points), au même endroit que le
    /// cadre ; tournés avec lui.
    fn frame_lines(&mut self, e: &Element, fill_and_stroke: Vec<(&'static str, String)>, radius: f64, pattern: Pattern) {
        let f = e.frame;
        let style = self.reg.style("graphic", "gr", vec![("style:graphic-properties", fill_and_stroke)]);
        let mut a = vec![("draw:style-name", style), ("draw:corner-radius", mm(radius))];
        a.extend(geometry(&f));
        self.body.empty("draw:rect", &attrs(&a));
        let size = PageSize { width_mm: f.w_mm, height_mm: f.h_mm };
        // Repère du cadre : ses lignes tournent avec lui autour de son centre.
        let (cx, cy) = (f.x_mm + f.w_mm / 2.0, f.y_mm + f.h_mm / 2.0);
        let (sin, cos) = f.rotation_deg.to_radians().sin_cos();
        let at = |x: f64, y: f64| {
            let (dx, dy) = (x - f.w_mm / 2.0, y - f.h_mm / 2.0);
            (cx + dx * cos - dy * sin, cy + dx * sin + dy * cos)
        };
        for m in crate::background::frame_marks(size, pattern) {
            match m {
                crate::background::Mark::Line { x1, y1, x2, y2, width_mm, color } => {
                    let st = self.reg.style("graphic", "gr", vec![("style:graphic-properties", vec![("draw:stroke", "solid".into()), ("svg:stroke-width", mm(width_mm)), ("svg:stroke-color", color.into()), ("draw:fill", "none".into())])]);
                    let ((ax, ay), (bx, by)) = (at(x1, y1), at(x2, y2));
                    self.body.empty("draw:line", &[("draw:style-name", st.as_str()), ("svg:x1", &mm(ax)), ("svg:y1", &mm(ay)), ("svg:x2", &mm(bx)), ("svg:y2", &mm(by))]);
                }
                crate::background::Mark::Dot { x, y, d_mm, color } => {
                    let st = self.reg.style("graphic", "gr", vec![("style:graphic-properties", vec![("draw:stroke", "none".into()), ("draw:fill", "solid".into()), ("draw:fill-color", color.into())])]);
                    let (px, py) = at(x, y);
                    let r = d_mm / 2.0;
                    self.body.empty("draw:ellipse", &[("draw:style-name", st.as_str()), ("svg:x", &mm(px - r)), ("svg:y", &mm(py - r)), ("svg:width", &mm(d_mm)), ("svg:height", &mm(d_mm))]);
                }
            }
        }
    }

    fn text(&mut self, e: &Element, t: &TextProps) -> Result<(), OdgError> {
        // Un texte vide disparaît, sauf s'il a un cadre (une étiquette à remplir) ; l'invite n'est jamais exportée.
        let blank = t.content.is_blank();
        if blank && t.frame.is_none() {
            return Ok(());
        }
        let pad = mm(t.style.padding());
        // Écrit sur les lignes du cadre : la première ligne de base à un pas du haut du cadre (LibreOffice la pose à
        // marge du haut + pas - descendante), les suivantes de pas en pas, toujours depuis le haut. L'interligne et
        // l'alignement vertical du texte sont gardés mais sans effet (ui/src/lib/lineSpacing.ts, `writingLineBox`).
        let on_lines = crate::background::writes_on_lines(t.frame.as_ref()).then(|| crate::background::writing_lines(self.reg.fonts.face(&t.style.font, false, false).and_then(|f| f.metrics), t.style.size_pt));
        let pad_top = on_lines.map_or(pad.clone(), |w| mm(w.descent_mm));
        let valign = match t.style.valign.unwrap_or_default() {
            _ if on_lines.is_some() => "top",
            VAlign::Top => "top",
            VAlign::Middle => "middle",
            VAlign::Bottom => "bottom",
        };
        // Cadre : fond et bordure natifs du cadre de texte (le texte reste modifiable d'un clic dans LibreOffice).
        // Une image de fond couvre le cadre (recadrée au centre) et remplace la couleur.
        let frame = t.frame.as_ref();
        let mut props: Vec<(&'static str, String)> = match (frame.and_then(|f| f.image.as_ref()), frame.and_then(|f| f.fill.clone())) {
            (Some(image), _) => {
                let radius = frame.map_or(0.0, |f| f.corner_radius_mm);
                let path = self.cropped_picture(image, &e.frame, &Crop::COVER, (false, false), Some(radius), &format!("{}-fond", e.id))?;
                let fill_name = format!("img_{}_fond", e.id);
                self.reg.fill_image(fill_name.clone(), path);
                vec![("draw:fill", "bitmap".into()), ("draw:fill-image-name", fill_name), ("style:repeat", "stretch".into())]
            }
            (None, Some(c)) => vec![("draw:fill", "solid".into()), ("draw:fill-color", c)],
            (None, None) => vec![("draw:fill", "none".into())],
        };
        props.extend(stroke(self.reg, frame.and_then(|f| f.stroke.as_ref())));
        // Lignes d'écriture : le fond et la bordure dessinés à part, les lignes par-dessus, puis le texte.
        if let Some(pattern) = frame.and_then(|f| f.pattern) {
            self.frame_lines(e, std::mem::take(&mut props), frame.map_or(0.0, |f| f.corner_radius_mm), pattern);
            props = vec![("draw:fill", "none".into())];
            props.extend(stroke(self.reg, None));
        }
        let style = self.reg.style(
            "graphic",
            "gr",
            vec![(
                "style:graphic-properties",
                [props, vec![
                    ("fo:padding-top", pad_top),
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
                    // Interligne : un pourcentage de la hauteur naturelle, tout l'espace ajouté au-dessus de chaque
                    // ligne (l'écran le reproduit, voir ui/src/lib/lineSpacing.ts). Absent : 100 %, comme toujours.
                    // Sur les lignes : une hauteur de ligne d'au moins un pas (un passage plus grand la ferait grandir au
                    // lieu de disparaître, comme avec une hauteur fixe) ; le pas tient la police, elle vaut donc le pas.
                    match on_lines {
                        Some(w) => ("style:line-height-at-least", mm(w.pitch_mm)),
                        None => ("fo:line-height", t.style.line_spacing.map_or("100%".into(), |n| format!("{}%", (n * 100.0).round()))),
                    },
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
                // Un champ de la classe non rempli sort blanc, comme à l'impression : son nom (« Prénom ») écrit à
                // l'encre transparente garde la place qu'il prend à l'impression, sans rien laisser sur la feuille.
                for piece in crate::fields::split(&run.text) {
                    let mut props = self.text_props(run, &t.style);
                    let shown = match piece {
                        crate::fields::Piece::Text(t) => t.to_string(),
                        crate::fields::Piece::Field(f) => {
                            props[0].1.push(("loext:opacity", "0%".into()));
                            crate::fields::shown(f).unwrap_or_else(|| format!("{{{f}}}"))
                        }
                    };
                    let t_style = self.reg.style("text", "T", props);
                    self.body.open("text:span", &[("text:style-name", &t_style)]);
                    write_run_text(&mut self.body, &shown, &mut at_start);
                    self.body.close();
                }
            }
            self.body.close();
        }
        self.body.close();
        self.body.close();
        Ok(())
    }

    /// Traits d'un tableau : une ligne par segment ; les cases sont des éléments à part.
    fn table(&mut self, e: &Element, t: &TableProps) {
        let Some(border) = &t.border else { return };
        let props = stroke(self.reg, Some(border));
        let style = self.reg.style("graphic", "gr", vec![("style:graphic-properties", props)]);
        for [x1, y1, x2, y2] in crate::table::border_segments(t, &e.frame) {
            let r = |v: f64| mm((v * 1000.0).round() / 1000.0 + 0.0);
            let a = [("draw:style-name", style.clone()), ("svg:x1", r(x1)), ("svg:y1", r(y1)), ("svg:x2", r(x2)), ("svg:y2", r(y2))];
            self.body.empty("draw:line", &attrs(&a));
        }
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
            let mut ends = line_ends(&e.frame);
            if e.flip_h {
                let [x1, y1, x2, y2] = [0, 1, 2, 3].map(|i| ends[i].1.clone());
                ends = vec![("svg:x1", x2), ("svg:y1", y2), ("svg:x2", x1), ("svg:y2", y1)];
            }
            a.extend(ends);
            self.body.empty("draw:line", &attrs(&a));
            return;
        }
        if let Some(path) = shapes::path_of(s) {
            // L'écran retourne la forme dans son cadre puis la tourne ; LibreOffice la retourne après l'avoir tournée.
            // Retournée d'un seul sens, elle reçoit donc l'angle opposé (dans les deux sens, c'est un demi-tour qui
            // ne change rien à l'angle). L'import fait l'inverse (`import::odg`, `mirror_last_shape`).
            let f = if e.flip_h != e.flip_v { Frame { rotation_deg: -e.frame.rotation_deg, ..e.frame } } else { e.frame };
            a.extend(geometry(&f));
            self.body.open("draw:custom-shape", &attrs(&a));
            let viewbox = format!("0 0 {0} {0}", shapes::VIEWBOX);
            let enhanced = format!("{path} N");
            let mut g = vec![("svg:viewBox", viewbox.as_str()), ("draw:enhanced-path", enhanced.as_str())];
            if e.flip_h {
                g.push(("draw:mirror-horizontal", "true"));
            }
            if e.flip_v {
                g.push(("draw:mirror-vertical", "true"));
            }
            self.body.empty("draw:enhanced-geometry", &g);
            self.body.close();
            return;
        }
        // Tracé libre invalide (document abîmé) : rien n'est écrit.
        if s.shape == ShapeKind::Path {
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
        // Les membres consécutifs d'un même groupe sont écrits dans un groupe LibreOffice, un groupe dans un
        // groupe dans un groupe LibreOffice imbriqué.
        let mut open: Vec<Id> = Vec::new();
        for e in &page.elements {
            let path = crate::groups::path(e);
            let common = open.iter().zip(&path).take_while(|(a, b)| a == b).count();
            for _ in common..open.len() {
                w.body.close();
            }
            open.truncate(common);
            for g in &path[common..] {
                w.body.open("draw:g", &[]);
                open.push(g.clone());
            }
            w.element(e)?;
        }
        for _ in 0..open.len() {
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
