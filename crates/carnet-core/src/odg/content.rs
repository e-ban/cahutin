//! `content.xml` : une `draw:page` par page, un objet ODG par élément.

use super::styles::{Props, Registry, dash_name, marker_name, mm};
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
        let path = self.picto_path(e, code, outline)?;
        let style = self.reg.style("graphic", "gr", vec![("style:graphic-properties", vec![("draw:stroke", "none".into()), ("draw:fill", "none".into())])]);
        let mut a = vec![("draw:style-name", style)];
        a.extend(geometry(&e.frame));
        self.body.open("draw:frame", &attrs(&a));
        self.body.empty("draw:image", &[("xlink:href", &path), ("xlink:type", "simple"), ("xlink:show", "embed"), ("xlink:actuate", "onLoad")]);
        self.body.close();
        Ok(())
    }

    /// Chemin dans l'archive du SVG OpenMoji de l'élément (en miroir s'il l'est), ajouté une seule fois.
    fn picto_path(&mut self, e: &Element, code: &str, outline: bool) -> Result<String, OdgError> {
        let path = format!("Pictures/{}.svg", picto_stem(code, outline, e.flip_h, e.flip_v));
        // Décompressée seulement si elle n'est pas déjà dans l'archive (un code inconnu n'y est jamais).
        if !self.pictures.iter().any(|p| p.path == path) {
            let bytes = picto_file(code, outline, e.flip_h, e.flip_v).ok_or_else(|| OdgError::MissingPicture(code.to_string()))?;
            self.pictures.push(Picture { path: path.clone(), media_type: "image/svg+xml", bytes });
        }
        Ok(path)
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
        self.reg.protect = true;
        // Sans image de fond, l'écriture du texte ne peut pas échouer.
        let _ = self.text(&e, &t);
        self.reg.protect = false;
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
                let fill_name = format!("img_{}_fond{}", e.id, if e.frame.rotation_deg != 0.0 { TURNED_FILL } else { "" });
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
        let blank_text = if blank { self.text_props(&Run::plain(""), &t.style) } else { Vec::new() };
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
            )]
            .into_iter()
            // Une étiquette vide : sa police, sa taille et sa couleur aussi dans le style du cadre (LibreOffice ne garde
            // pas le style d'un paragraphe vide), celles du texte qu'on y tapera.
            .chain(blank_text)
            .collect(),
        );
        let mut a = vec![("draw:style-name", style)];
        a.extend(geometry(&e.frame));
        self.body.open("draw:frame", &attrs(&a));
        let radius = frame.map(|f| f.corner_radius_mm).filter(|r| *r > 0.0).map(mm);
        let box_attrs: Vec<(&str, &str)> = radius.iter().map(|r| ("draw:corner-radius", r.as_str())).collect();
        self.body.open("draw:text-box", &box_attrs);
        self.paragraphs(t, on_lines);
        self.body.close();
        // Les champs qui s'affichent comme un autre (une lettre ne montre que son numéro) : leurs noms dans la
        // description du cadre, dans l'ordre du texte, pour que l'import les retrouve (`import::odg`, `field_marks`).
        let shared: Vec<String> = t
            .content
            .paragraphs
            .iter()
            .flat_map(|p| &p.runs)
            .flat_map(|r| crate::fields::split(&r.text))
            .filter_map(|p| match p {
                crate::fields::Piece::Field(f) if crate::fields::shown_is_shared(f) => Some(format!("{{{f}}}")),
                _ => None,
            })
            .collect();
        if !shared.is_empty() && !blank {
            self.body.open("svg:desc", &[]);
            self.body.text(&shared.join(" "));
            self.body.close();
        }
        self.body.close();
        Ok(())
    }

    /// Paragraphes d'un texte (cadre de texte ou case de tableau) : styles, puces, interligne, champs de la classe.
    /// `on_lines` : le texte s'écrit sur les lignes de son cadre.
    fn paragraphs(&mut self, t: &TextProps, on_lines: Option<crate::background::WritingLines>) {
        let blank = t.content.is_blank();
        // Éléments de liste : une `text:list` par suite de paragraphes à puce. L'étiquette (la puce, au bord du texte)
        // a la largeur du retrait, où commencent le texte et ses lignes suivantes. La puce est dans la police et à la
        // taille du bloc, pour que la ligne garde sa hauteur ; LibreOffice ne connaît qu'une taille relative au premier
        // caractère du paragraphe : un premier mot d'une autre taille demande sa propre liste.
        let mut list: Option<String> = None;
        // Un texte vide (une étiquette encadrée à remplir) : un paragraphe vide, qui ne montre rien et garde au texte sa
        // police, sa taille et sa couleur.
        let empty = [Paragraph { align: None, runs: Vec::new(), list: None }];
        let paragraphs: &[Paragraph] = if blank { &empty } else { &t.content.paragraphs };
        for p in paragraphs {
            let style = p.list.is_some().then(|| self.list_style(p, &t.style));
            if style != list {
                if list.is_some() {
                    self.body.close();
                }
                if let Some(l) = &style {
                    self.body.open("text:list", &[("text:style-name", l)]);
                }
                list = style;
            }
            let bullet = list.is_some();
            if bullet {
                self.body.open("text:list-item", &[]);
            }
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
            if bullet {
                self.body.close();
            }
        }
        if list.is_some() {
            self.body.close();
        }
    }

    /// Style de la liste d'un paragraphe à puce : l'étiquette large du retrait, la puce dans la police, la couleur et
    /// à la taille du bloc (en pour cent du premier caractère, seule taille que LibreOffice lit).
    fn list_style(&mut self, p: &Paragraph, style: &TextStyle) -> String {
        let first = p.runs.iter().find(|r| !r.text.is_empty()).and_then(|r| r.size_pt).filter(|s| *s > 0.0).unwrap_or(style.size_pt);
        // Les lettres creuses ne s'appliquent pas à la puce : LibreOffice la dessine toujours pleine.
        let bullet = vec![
            ("style:font-name", self.reg.font(&style.font, false, false)),
            ("fo:font-size", format!("{}%", (style.size_pt / first * 100.0).round())),
            ("fo:color", style.color.clone()),
        ];
        let indent = LIST_INDENT_EM * style.size_pt * 25.4 / 72.0;
        self.reg.list_style(vec![("text:min-label-width", mm(indent))], bullet)
    }

    /// Vrai tableau de LibreOffice (`table:table` dans un `draw:frame`) pour un tableau simple (voir `is_simple`) : ses
    /// cases y sont écrites, à la place des éléments à part. Colonnes et lignes aux bords arrondis au 1/100 mm (LibreOffice
    /// garde ses tailles au 1/100 mm : chaque taille est la différence de deux bords arrondis, les erreurs ne s'additionnent
    /// pas), le cadre à leur somme (sinon LibreOffice redistribue les lignes). Le trait est écrit divisé par
    /// `CELL_BORDER_DRAWN`, centré sur la grille comme à l'écran. Un pictogramme ou une photo est le fond de sa case, non
    /// étiré, à sa taille dans Cahutin, au centre. Aucune image d'aperçu (`.svm`) n'est écrite.
    fn real_table(&mut self, e: &Element, elements: &[Element]) -> Result<(), OdgError> {
        let ElementKind::Table(t) = &e.kind else { return Ok(()) };
        let f = e.frame;
        let hundredth = |v: f64| (v * 100.0).round() / 100.0;
        let xs: Vec<f64> = crate::table::edges(&t.cols, f.x_mm, f.w_mm).into_iter().map(hundredth).collect();
        let ys: Vec<f64> = crate::table::edges(&t.rows, f.y_mm, f.h_mm).into_iter().map(hundredth).collect();
        let border = match &t.border {
            Some(b) => format!("{} solid {}", mm(b.width_mm / super::CELL_BORDER_DRAWN), b.color),
            None => "none".into(),
        };
        let style = self.reg.style("graphic", "gr", vec![("style:graphic-properties", vec![])]);
        let (x0, y0, x1, y1) = (xs[0], ys[0], xs[xs.len() - 1], ys[ys.len() - 1]);
        self.body.open("draw:frame", &[("draw:style-name", &style), ("svg:x", &mm(x0)), ("svg:y", &mm(y0)), ("svg:width", &mm(x1 - x0)), ("svg:height", &mm(y1 - y0))]);
        self.body.open("table:table", &[]);
        for w in xs.windows(2) {
            let s = self.reg.style("table-column", "co", vec![("style:table-column-properties", vec![("style:column-width", mm(w[1] - w[0])), ("style:use-optimal-column-width", "false".into())])]);
            self.body.empty("table:table-column", &[("table:style-name", &s)]);
        }
        let mut covered = std::collections::HashSet::new();
        for c in &t.cells {
            for dc in 0..c.col_span {
                for dr in 0..c.row_span {
                    if (dc, dr) != (0, 0) {
                        covered.insert((c.col + dc, c.row + dr));
                    }
                }
            }
        }
        for (r, h) in ys.windows(2).enumerate() {
            // `style:row-height` : la hauteur de la ligne (`style:min-row-height` est ignoré par LibreOffice).
            let s = self.reg.style("table-row", "ro", vec![("style:table-row-properties", vec![("style:row-height", mm(h[1] - h[0])), ("style:use-optimal-row-height", "false".into())])]);
            self.body.open("table:table-row", &[("table:style-name", &s)]);
            for c in 0..xs.len() - 1 {
                let (c, r) = (c as u8, r as u8);
                if covered.contains(&(c, r)) {
                    self.body.empty("table:covered-table-cell", &[]);
                    continue;
                }
                let Some(cell) = t.cells.iter().find(|g| g.col == c && g.row == r) else { continue };
                let Some(el) = elements.iter().find(|x| x.id == cell.element_id) else { continue };
                let height = ys[(r + cell.row_span) as usize] - ys[r as usize];
                self.table_cell(el, cell, height, &border)?;
            }
            self.body.close();
        }
        self.body.close();
        self.body.close();
        Ok(())
    }

    /// Une case du vrai tableau : son style (fond, marges, alignement vertical, trait), puis son contenu. Une case sans
    /// texte reçoit une espace sans largeur (U+200B) : sans contenu, LibreOffice lui donnerait 7,1 mm de haut. Elle est
    /// dans la police et à la taille de la case (le texte tapé ensuite dans LibreOffice en hérite), sauf si la ligne est
    /// trop basse pour elles : alors en 1 pt, sans marge en haut ni en bas (voir `empty_cell_fits`).
    fn table_cell(&mut self, el: &Element, cell: &GridCell, height: f64, border: &str) -> Result<(), OdgError> {
        let tiny = || TextProps { content: RichText::plain(ZERO_WIDTH), style: TextStyle { size_pt: TINY_CELL_PT, ..TextStyle::default_zone() }, prompt: None, frame: None };
        let no_fill = || vec![("draw:fill", "none".into())];
        // L'image remplit la case sans être étirée : à sa taille dans Cahutin, au centre.
        let picture_fill = |reg: &mut Registry, name: String, path: String, f: &Frame| {
            if !reg.fill_images.iter().any(|(n, _)| *n == name) {
                reg.fill_image(name.clone(), path);
            }
            vec![
                ("draw:fill", "bitmap".into()),
                ("draw:fill-image-name", name),
                ("style:repeat", "no-repeat".into()),
                ("draw:fill-image-width", mm(f.w_mm)),
                ("draw:fill-image-height", mm(f.h_mm)),
                ("draw:fill-image-ref-point", "center".into()),
            ]
        };
        let (fill, pad, pad_v, valign, content) = match &el.kind {
            ElementKind::Text(tp) => {
                let fill = match tp.frame.as_ref().and_then(|f| f.fill.clone()) {
                    Some(c) => vec![("draw:fill", "solid".into()), ("draw:fill-color", c)],
                    None => no_fill(),
                };
                let pad = tp.style.padding();
                let valign = match tp.style.valign.unwrap_or_default() {
                    VAlign::Top => "top",
                    VAlign::Middle => "middle",
                    VAlign::Bottom => "bottom",
                };
                let (pad_v, content) = if !tp.content.is_blank() {
                    (pad, tp.clone())
                } else if empty_cell_fits(self.line_height_mm(&tp.style.font, tp.style.size_pt), pad, height) {
                    (pad, TextProps { content: RichText::plain(ZERO_WIDTH), style: tp.style.clone(), prompt: None, frame: None })
                } else {
                    (0.0, TextProps { content: RichText::plain(ZERO_WIDTH), style: TextStyle { size_pt: TINY_CELL_PT, ..tp.style.clone() }, prompt: None, frame: None })
                };
                (fill, pad, pad_v, valign, content)
            }
            ElementKind::Image(i) => {
                let path = self.picto_path(el, &i.code, i.outline == Some(true))?;
                let name = format!("{PICTO_CELL_FILL}{}", path.trim_start_matches("Pictures/").trim_end_matches(".svg"));
                (picture_fill(self.reg, name, path, &el.frame), 0.0, 0.0, "middle", tiny())
            }
            ElementKind::Photo(p) => {
                let fill = match &p.asset {
                    Some(asset) => {
                        let path = self.cropped_picture(asset, &el.frame, &p.crop, (false, false), None, &el.id)?;
                        picture_fill(self.reg, format!("img_{}", el.id), path, &el.frame)
                    }
                    None => no_fill(),
                };
                (fill, 0.0, 0.0, "middle", tiny())
            }
            // Écartés par `is_simple`.
            ElementKind::Shape(_) | ElementKind::Table(_) => (no_fill(), 0.0, 0.0, "middle", tiny()),
        };
        let graphic = [fill, vec![
            ("fo:padding-top", mm(pad_v)),
            ("fo:padding-bottom", mm(pad_v)),
            ("fo:padding-left", mm(pad)),
            ("fo:padding-right", mm(pad)),
            ("draw:textarea-vertical-align", valign.into()),
        ]]
        .concat();
        let style = self.reg.style("table-cell", "ce", vec![("loext:graphic-properties", graphic), ("style:paragraph-properties", vec![("fo:border", border.to_string())])]);
        let (cs, rs) = (cell.col_span.to_string(), cell.row_span.to_string());
        let mut a = vec![("table:style-name", style.as_str())];
        if cell.col_span > 1 {
            a.push(("table:number-columns-spanned", &cs));
        }
        if cell.row_span > 1 {
            a.push(("table:number-rows-spanned", &rs));
        }
        self.body.open("table:table-cell", &a);
        self.paragraphs(&content, None);
        self.body.close();
        Ok(())
    }

    /// Hauteur d'une ligne de texte dans cette police à cette taille, en mm (celle du moteur, qui surestime celle de
    /// LibreOffice : une case jugée assez haute l'est).
    fn line_height_mm(&self, family: &str, size_pt: f64) -> f64 {
        crate::fonts::builtin_line_height_mm(family, size_pt)
            .or_else(|| self.reg.fonts.face(family, false, false).and_then(|f| f.metrics).map(|m| (m.ascent + m.descent) / m.units_per_em as f64 * size_pt * 25.4 / 72.0))
            .unwrap_or(size_pt * 25.4 / 72.0 * 1.6)
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
        if stroke_only {
            // Terminaisons : la première au bout x1, la seconde au bout x2 ; le miroir échange les deux bouts et leurs
            // terminaisons ensemble. Rond et carré, centrés sur le bout, sont plus petits qu'une pointe.
            let ends = shapes::ends_of(s);
            let head = shapes::head_width_mm(s);
            for (end, (key, width, center)) in [
                (ends.start, ("draw:marker-start", "draw:marker-start-width", "draw:marker-start-center")),
                (ends.end, ("draw:marker-end", "draw:marker-end-width", "draw:marker-end-center")),
            ] {
                let Some(name) = marker_name(end) else { continue };
                if !self.reg.markers.contains(&end) {
                    self.reg.markers.push(end);
                }
                let round = matches!(end, LineEnd::Circle | LineEnd::Square);
                props.push((key, name.into()));
                props.push((width, mm(if round { head * shapes::DOT_FACTOR } else { head })));
                if round {
                    props.push((center, "true".into()));
                }
            }
        }
        let style = self.reg.style("graphic", "gr", vec![("style:graphic-properties", props)]);
        let mut a = vec![("draw:style-name", style)];
        if stroke_only && let Some(pts) = shapes::path_page_points(&e.frame, s, e.flip_h, e.flip_v) {
            // Chemin : ses points visibles (rotation et miroir déjà appliqués), dans une boîte en micromètres.
            // Coudes : `draw:polyline` ; courbe : `draw:path` fait de cubiques (`shapes::curve_beziers`). LibreOffice
            // oriente les terminaisons selon le premier et le dernier segment, comme l'écran.
            let x0 = pts.iter().map(|p| p.0).fold(f64::INFINITY, f64::min);
            let y0 = pts.iter().map(|p| p.1).fold(f64::INFINITY, f64::min);
            let w = (pts.iter().map(|p| p.0).fold(f64::NEG_INFINITY, f64::max) - x0).max(0.01);
            let h = (pts.iter().map(|p| p.1).fold(f64::NEG_INFINITY, f64::max) - y0).max(0.01);
            let um = |v: f64| format!("{}", (v * 1000.0).round() as i64);
            let at = |p: (f64, f64)| (um(p.0 - x0), um(p.1 - y0));
            a.extend([("svg:x", mm(x0)), ("svg:y", mm(y0)), ("svg:width", mm(w)), ("svg:height", mm(h))]);
            a.push(("svg:viewBox", format!("0 0 {} {}", um(w), um(h))));
            if s.curve {
                let mut d = { let (x, y) = at(pts[0]); format!("M {x} {y}") };
                for [c1, c2, p] in shapes::curve_beziers(&pts) {
                    let ((ax, ay), (bx, by), (px, py)) = (at(c1), at(c2), at(p));
                    d.push_str(&format!(" C {ax} {ay} {bx} {by} {px} {py}"));
                }
                a.push(("svg:d", d));
                self.body.empty("draw:path", &attrs(&a));
            } else {
                let points = pts.iter().map(|&p| { let (x, y) = at(p); format!("{x},{y}") }).collect::<Vec<_>>().join(" ");
                a.push(("draw:points", points));
                self.body.empty("draw:polyline", &attrs(&a));
            }
            return;
        }
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

/// Nom du fichier d'un pictogramme dans l'archive, sans `Pictures/` ni `.svg` : `openmoji-1F98A`, à colorier
/// `openmoji-contour-1F98A` ; en miroir, une copie retournée nommée `…-miroir-h` (`-v`, `-hv`).
pub(crate) fn picto_stem(code: &str, outline: bool, flip_h: bool, flip_v: bool) -> String {
    let mirror = match (flip_h, flip_v) {
        (false, false) => "",
        (true, false) => "-miroir-h",
        (false, true) => "-miroir-v",
        (true, true) => "-miroir-hv",
    };
    if outline { format!("openmoji-contour-{}{mirror}", picto::base_code(code)) } else { format!("openmoji-{code}{mirror}") }
}

/// Début du commentaire qui ouvre le SVG d'un pictogramme : `<!-- cahutin:openmoji-1F98A -->`, le nom de son fichier
/// (voir `picto_stem`). LibreOffice renomme le fichier en réenregistrant, mais garde ses octets : l'import y retrouve le
/// pictogramme. Un commentaire ne change rien au dessin.
pub(crate) const PICTO_MARK: &str = "<!-- cahutin:";

/// Les octets du SVG d'un pictogramme dans l'archive : le commentaire qui le nomme (`PICTO_MARK`), puis le SVG de la
/// banque, à colorier ou retourné ; `None` pour un code inconnu.
pub(crate) fn picto_file(code: &str, outline: bool, flip_h: bool, flip_v: bool) -> Option<Vec<u8>> {
    let svg = if outline { picto::outline_svg(code) } else { picto::svg(code) }?;
    let svg = if flip_h || flip_v { picto::mirrored_svg(&svg, flip_h, flip_v)? } else { svg };
    let mut out = format!("{PICTO_MARK}{} -->\n", picto_stem(code, outline, flip_h, flip_v)).into_bytes();
    out.extend(svg);
    Some(out)
}

/// Début du nom de l'image de remplissage d'une case pictogramme, suivi du nom de son SVG (`case_openmoji-1F98A`) :
/// LibreOffice renomme le fichier en réenregistrant, pas l'image de remplissage ; l'import y retrouve le pictogramme.
pub(crate) const PICTO_CELL_FILL: &str = "case_";
/// Contenu d'une case vide du vrai tableau : une espace sans largeur, qui donne sa hauteur à la ligne sans rien montrer.
pub(crate) const ZERO_WIDTH: &str = "\u{200B}";
/// Taille de l'espace sans largeur d'une case trop basse pour sa police (ou d'une case pictogramme ou photo).
pub(crate) const TINY_CELL_PT: f64 = 1.0;

/// Une case vide garde sa police et sa taille si une ligne de ce texte et ses marges du haut et du bas tiennent dans sa
/// hauteur (`line_mm` : la hauteur de ligne du moteur, qui surestime celle de LibreOffice) ; sinon LibreOffice
/// agrandirait la ligne, et les autres lignes rétréciraient d'autant.
pub(crate) fn empty_cell_fits(line_mm: f64, padding_mm: f64, height_mm: f64) -> bool {
    line_mm + 2.0 * padding_mm <= height_mm + 1e-9
}

/// Vrai si le tableau s'écrit en vrai tableau de LibreOffice au même rendu qu'à l'écran ; sinon il garde l'export en
/// traits et cadres. Dans le doute, faux. Il faut : un tableau droit (ni tourné ni retourné), au trait plein ou sans
/// trait ; une grille de colonnes et de lignes de tailles positives dont chaque place est couverte par une case et une
/// seule ; un groupe fait du tableau et de ses cases seulement, les cases juste avant le tableau (rien entre elles), à
/// la place que leur donne `table::layout` ; et chaque case simple (voir `cell_is_simple`).
pub(crate) fn is_simple(table: &Element, elements: &[Element]) -> bool {
    let ElementKind::Table(t) = &table.kind else { return false };
    if table.frame.rotation_deg != 0.0 || table.flip_h || table.flip_v || t.border.as_ref().is_some_and(|b| !b.is_solid()) {
        return false;
    }
    let (ncols, nrows) = (t.cols.len(), t.rows.len());
    if ncols == 0 || nrows == 0 || t.cols.iter().chain(&t.rows).any(|v| !v.is_finite() || *v <= 0.0) {
        return false;
    }
    let mut taken = vec![false; ncols * nrows];
    for c in &t.cells {
        let (c0, r0, cs, rs) = (c.col as usize, c.row as usize, c.col_span as usize, c.row_span as usize);
        if cs == 0 || rs == 0 || c0 + cs > ncols || r0 + rs > nrows {
            return false;
        }
        for r in r0..r0 + rs {
            for k in c0..c0 + cs {
                if std::mem::replace(&mut taken[r * ncols + k], true) {
                    return false;
                }
            }
        }
    }
    if taken.contains(&false) {
        return false;
    }
    let path = crate::groups::path(table);
    let (Some(own), Some(at)) = (path.last(), elements.iter().position(|e| e.id == table.id)) else { return false };
    let members: Vec<usize> = elements.iter().enumerate().filter(|(_, e)| crate::groups::path(e).contains(own)).map(|(i, _)| i).collect();
    if members.len() != t.cells.len() + 1 || members.last() != Some(&at) || members.windows(2).any(|w| w[1] != w[0] + 1) {
        return false;
    }
    let cells: Vec<&Element> = members[..members.len() - 1].iter().map(|&i| &elements[i]).collect();
    if !cells.iter().all(|e| crate::groups::path(e) == path && t.cells.iter().any(|c| c.element_id == e.id)) {
        return false;
    }
    // Chaque case à la place que lui donne la grille.
    let mut page = Page::new(A4_PORTRAIT);
    page.elements = members.iter().map(|&i| elements[i].clone()).collect();
    crate::table::layout(&mut page, &table.id);
    let same = |a: &Frame, b: &Frame| [a.x_mm - b.x_mm, a.y_mm - b.y_mm, a.w_mm - b.w_mm, a.h_mm - b.h_mm].iter().all(|d| d.abs() < 0.01) && a.rotation_deg == 0.0 && b.rotation_deg == 0.0;
    cells.iter().zip(&page.elements).all(|(e, placed)| same(&e.frame, &placed.frame) && cell_is_simple(e))
}

/// Une case que le vrai tableau rend comme l'écran : un texte sans bordure propre, sans coins arrondis, sans lignes
/// d'écriture ni image de fond, à l'endroit ; un pictogramme (en miroir compris : son SVG est retourné) ; une photo sans
/// coins arrondis, bordure, ombre ni miroir (vide comprise). Un autre élément (forme, tableau) : non.
pub(crate) fn cell_is_simple(e: &Element) -> bool {
    match &e.kind {
        ElementKind::Text(t) => !e.flip_h && !e.flip_v && t.frame.as_ref().is_none_or(|f| f.stroke.is_none() && f.corner_radius_mm == 0.0 && f.pattern.is_none() && f.image.is_none()),
        ElementKind::Image(_) => true,
        ElementKind::Photo(p) => p.corner_radius_mm == 0.0 && p.border.is_none() && !p.shadow && !p.mirrored && !e.flip_h && !e.flip_v,
        ElementKind::Shape(_) | ElementKind::Table(_) => false,
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
        let master = w.reg.master_page(page.size, page.background.as_ref());
        w.body.open("draw:page", &[("draw:name", &format!("page{}", i + 1)), ("draw:master-page-name", &master)]);
        // Les membres consécutifs d'un même groupe sont écrits dans un groupe LibreOffice, un groupe dans un
        // groupe dans un groupe LibreOffice imbriqué.
        let mut open: Vec<Id> = Vec::new();
        // Tableaux simples : écrits en vrais tableaux, leurs cases dedans. Le groupe du tableau, fait de lui et de ses
        // cases seulement, n'est pas écrit (un tableau seul dans un groupe ne se modifie pas d'un clic).
        let simple: Vec<&Id> = page.elements.iter().filter(|e| is_simple(e, &page.elements)).map(|e| &e.id).collect();
        let inside: std::collections::HashSet<&Id> = page
            .elements
            .iter()
            .filter(|e| simple.contains(&&e.id))
            .flat_map(|e| match &e.kind {
                ElementKind::Table(t) => t.cells.iter().map(|c| &c.element_id).collect::<Vec<_>>(),
                _ => Vec::new(),
            })
            .collect();
        for e in &page.elements {
            if inside.contains(&e.id) {
                continue;
            }
            let real = simple.contains(&&e.id);
            let mut path = crate::groups::path(e);
            if real {
                path.pop();
            }
            let common = open.iter().zip(&path).take_while(|(a, b)| a == b).count();
            for _ in common..open.len() {
                w.body.close();
            }
            open.truncate(common);
            for g in &path[common..] {
                w.body.open("draw:g", &[]);
                open.push(g.clone());
            }
            // Un élément verrouillé l'est aussi dans LibreOffice : ni déplacé ni agrandi d'un geste.
            w.reg.protect = e.locked;
            if real {
                w.real_table(e, &page.elements)?;
            } else {
                w.element(e)?;
            }
            w.reg.protect = false;
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
