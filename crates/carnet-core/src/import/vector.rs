//! Tracés qu'aucune forme de Cahutin ne reprend (courbes d'un PDF, polygones quelconques) : redessinés
//! ensemble, page par page, dans une image fixe posée au fond de la page. Une fiche de graphisme ou un
//! cadre arrondi s'impriment ainsi comme dans l'original ; ils ne se modifient plus trait par trait.

use crate::model::{Frame, Mime, ShapeKind, ShapeProps, Stroke, StrokeStyle};
use crate::photo::{Asset, asset_from_image};
use image::{DynamicImage, RgbaImage};
use svgtypes::{SimplePathSegment, SimplifyingPathParser};
use tiny_skia::{
    Color, FillRule, LineCap, Paint, Path, PathBuilder, PathSegment, Pixmap, StrokeDash, Transform,
};

/// Résolution de l'image : celle d'une impression soignée.
const DPI: f64 = 200.0;
/// Plus grand côté de l'image, en pixels (comme les photos).
const MAX_PX: f64 = 3000.0;

pub(super) struct Drawing {
    /// Tracé en mm, dans le repère de la page.
    path: Path,
    fill: Option<Color>,
    /// Couleur de remplissage telle que le document la donne (« #rrggbb »).
    fill_hex: Option<String>,
    stroke: Option<Stroke>,
    /// Motif du pointillé tel que le document le donne (trait, espace, en mm) ; sinon un motif d'après le style.
    dash: Option<(f32, f32)>,
}

/// Passage du repère `viewBox` de l'objet (largeur, hauteur, origine) à des mm sur la page, rotation comprise.
fn placement(frame: &Frame, view_box: Option<&str>) -> Option<Transform> {
    let vb: Vec<f32> = match view_box {
        Some(v) => v
            .split([' ', ','])
            .filter(|s| !s.is_empty())
            .map(|s| s.parse().ok())
            .collect::<Option<_>>()?,
        None => vec![0.0, 0.0, frame.w_mm as f32, frame.h_mm as f32],
    };
    let [vx, vy, vw, vh] = vb[..] else {
        return None;
    };
    if vw <= 0.0 || vh <= 0.0 {
        return None;
    }
    let (sx, sy) = (frame.w_mm as f32 / vw, frame.h_mm as f32 / vh);
    let fit = Transform::from_row(
        sx,
        0.0,
        0.0,
        sy,
        frame.x_mm as f32 - vx * sx,
        frame.y_mm as f32 - vy * sy,
    );
    let (cx, cy) = (
        (frame.x_mm + frame.w_mm / 2.0) as f32,
        (frame.y_mm + frame.h_mm / 2.0) as f32,
    );
    Some(fit.post_concat(Transform::from_rotate_at(frame.rotation_deg as f32, cx, cy)))
}

fn color(hex: &str) -> Option<Color> {
    let h = hex.strip_prefix('#')?;
    let v = |i: usize| u8::from_str_radix(h.get(i..i + 2)?, 16).ok();
    Some(Color::from_rgba8(v(0)?, v(2)?, v(4)?, 255))
}

impl Drawing {
    fn new(
        path: Path,
        frame: &Frame,
        view_box: Option<&str>,
        fill: Option<&str>,
        stroke: Option<&Stroke>,
    ) -> Option<Drawing> {
        let path = path.transform(placement(frame, view_box)?)?;
        let fill_hex = fill.filter(|f| color(f).is_some()).map(str::to_string);
        let fill = fill.and_then(color);
        (fill.is_some() || stroke.is_some()).then(|| Drawing {
            path,
            fill,
            fill_hex,
            stroke: stroke.cloned(),
            dash: None,
        })
    }

    /// Le tracé en forme modifiable de Cahutin (tracé libre) : son cadre en mm et ses propriétés. Un pointillé
    /// devient « points » s'il est court (au plus une fois et demie l'épaisseur), sinon « tirets ».
    pub(super) fn to_shape(&self) -> Option<(Frame, ShapeProps)> {
        let b = self.path.bounds();
        let (w, h) = ((b.width() as f64).max(0.5), (b.height() as f64).max(0.5));
        let (x0, y0) = (b.left() as f64, b.top() as f64);
        let px = |x: f32| ((x as f64 - x0) / w * 1000.0).round() as i64;
        let py = |y: f32| ((y as f64 - y0) / h * 1000.0).round() as i64;
        let mut d: Vec<String> = Vec::new();
        let mut last = (0.0f32, 0.0f32);
        for seg in self.path.segments() {
            match seg {
                PathSegment::MoveTo(p) => {
                    d.push(format!("M {} {}", px(p.x), py(p.y)));
                    last = (p.x, p.y);
                }
                PathSegment::LineTo(p) => {
                    d.push(format!("L {} {}", px(p.x), py(p.y)));
                    last = (p.x, p.y);
                }
                // Quadratique : la cubique équivalente (points de contrôle aux deux tiers).
                PathSegment::QuadTo(c, p) => {
                    let (c1x, c1y) = (last.0 + 2.0 / 3.0 * (c.x - last.0), last.1 + 2.0 / 3.0 * (c.y - last.1));
                    let (c2x, c2y) = (p.x + 2.0 / 3.0 * (c.x - p.x), p.y + 2.0 / 3.0 * (c.y - p.y));
                    d.push(format!("C {} {} {} {} {} {}", px(c1x), py(c1y), px(c2x), py(c2y), px(p.x), py(p.y)));
                    last = (p.x, p.y);
                }
                PathSegment::CubicTo(c1, c2, p) => {
                    d.push(format!("C {} {} {} {} {} {}", px(c1.x), py(c1.y), px(c2.x), py(c2.y), px(p.x), py(p.y)));
                    last = (p.x, p.y);
                }
                PathSegment::Close => d.push("Z".into()),
            }
        }
        let path = d.join(" ");
        if !crate::shapes::valid_path(&path) {
            return None;
        }
        let stroke = self.stroke.clone().map(|mut s| {
            if let Some((on, _)) = self.dash {
                s.style = Some(if (on as f64) <= 1.5 * s.width_mm { StrokeStyle::Dotted } else { StrokeStyle::Dashed });
            }
            s
        });
        let frame = Frame { x_mm: x0, y_mm: y0, w_mm: w, h_mm: h, rotation_deg: 0.0 };
        Some((frame, ShapeProps { shape: ShapeKind::Path, fill: self.fill_hex.clone(), stroke, corner_radius_mm: 0.0, path: Some(path), graphisme: None }))
    }

    /// Le motif de pointillé du document (trait, espace, en mm).
    pub(super) fn with_dash(mut self, dash: Option<(f64, f64)>) -> Drawing {
        self.dash = dash.map(|(on, off)| (on as f32, off as f32));
        self
    }

    /// `draw:path` : ses données `svg:d`.
    pub(super) fn from_svg_path(
        d: &str,
        frame: &Frame,
        view_box: Option<&str>,
        fill: Option<&str>,
        stroke: Option<&Stroke>,
    ) -> Option<Drawing> {
        let mut b = PathBuilder::new();
        for seg in SimplifyingPathParser::from(d) {
            match seg.ok()? {
                SimplePathSegment::MoveTo { x, y } => b.move_to(x as f32, y as f32),
                SimplePathSegment::LineTo { x, y } => b.line_to(x as f32, y as f32),
                SimplePathSegment::CurveTo {
                    x1,
                    y1,
                    x2,
                    y2,
                    x,
                    y,
                } => b.cubic_to(
                    x1 as f32, y1 as f32, x2 as f32, y2 as f32, x as f32, y as f32,
                ),
                SimplePathSegment::Quadratic { x1, y1, x, y } => {
                    b.quad_to(x1 as f32, y1 as f32, x as f32, y as f32)
                }
                SimplePathSegment::ClosePath => b.close(),
            }
        }
        Drawing::new(b.finish()?, frame, view_box, fill, stroke)
    }

    /// `draw:polygon` (fermé) ou `draw:polyline` : ses points « x,y x,y ».
    pub(super) fn from_points(
        points: &str,
        closed: bool,
        frame: &Frame,
        view_box: Option<&str>,
        fill: Option<&str>,
        stroke: Option<&Stroke>,
    ) -> Option<Drawing> {
        let mut b = PathBuilder::new();
        for (i, pt) in points.split_whitespace().enumerate() {
            let (x, y) = pt.split_once(',')?;
            let (x, y) = (x.parse().ok()?, y.parse().ok()?);
            if i == 0 {
                b.move_to(x, y)
            } else {
                b.line_to(x, y)
            }
        }
        if closed {
            b.close();
        }
        Drawing::new(
            b.finish()?,
            frame,
            view_box,
            if closed { fill } else { None },
            stroke,
        )
    }
}

/// Les tracés d'une page dans une seule image transparente, avec le cadre où la poser.
pub(super) fn render(drawings: &[Drawing]) -> Option<(Frame, Asset)> {
    let margin = drawings
        .iter()
        .filter_map(|d| d.stroke.as_ref())
        .map(|s| s.width_mm)
        .fold(0.0, f64::max)
        / 2.0
        + 0.5;
    let (mut x0, mut y0, mut x1, mut y1) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
    for d in drawings {
        let b = d.path.bounds();
        x0 = x0.min(b.left() as f64);
        y0 = y0.min(b.top() as f64);
        x1 = x1.max(b.right() as f64);
        y1 = y1.max(b.bottom() as f64);
    }
    let frame = Frame {
        x_mm: x0 - margin,
        y_mm: y0 - margin,
        w_mm: x1 - x0 + 2.0 * margin,
        h_mm: y1 - y0 + 2.0 * margin,
        rotation_deg: 0.0,
    };
    let scale = (DPI / 25.4).min(MAX_PX / frame.w_mm.max(frame.h_mm));
    let mut pixmap = Pixmap::new(
        (frame.w_mm * scale).ceil() as u32,
        (frame.h_mm * scale).ceil() as u32,
    )?;
    let to_px = Transform::from_row(
        scale as f32,
        0.0,
        0.0,
        scale as f32,
        (-frame.x_mm * scale) as f32,
        (-frame.y_mm * scale) as f32,
    );
    for d in drawings {
        let mut paint = Paint {
            anti_alias: true,
            ..Paint::default()
        };
        if let Some(c) = d.fill {
            paint.set_color(c);
            pixmap.fill_path(&d.path, &paint, FillRule::Winding, to_px, None);
        }
        if let Some(s) = &d.stroke {
            paint.set_color(color(&s.color).unwrap_or(Color::BLACK));
            let w = s.width_mm as f32;
            let dash = match (d.dash, s.style) {
                (Some((on, off)), _) => StrokeDash::new(vec![on.max(0.05), off.max(0.05)], 0.0),
                (None, style) => match style {
                Some(StrokeStyle::Dashed) => {
                    StrokeDash::new(vec![(3.0 * w).max(1.0), (3.0 * w).max(1.0)], 0.0)
                }
                Some(StrokeStyle::Dotted) => {
                    StrokeDash::new(vec![w.max(0.3), (2.0 * w).max(0.6)], 0.0)
                }
                _ => None,
                },
            };
            let line_cap = if s.style == Some(StrokeStyle::Dotted) {
                LineCap::Round
            } else {
                LineCap::Butt
            };
            pixmap.stroke_path(
                &d.path,
                &paint,
                &tiny_skia::Stroke {
                    width: w,
                    dash,
                    line_cap,
                    ..tiny_skia::Stroke::default()
                },
                to_px,
                None,
            );
        }
    }
    let rgba: Vec<u8> = pixmap
        .pixels()
        .iter()
        .flat_map(|p| {
            let c = p.demultiply();
            [c.red(), c.green(), c.blue(), c.alpha()]
        })
        .collect();
    let img = DynamicImage::ImageRgba8(RgbaImage::from_raw(pixmap.width(), pixmap.height(), rgba)?);
    let asset = asset_from_image(&img, Mime::Png).ok()?;
    Some((frame, asset))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(x: f64, y: f64, w: f64, h: f64) -> Frame {
        Frame {
            x_mm: x,
            y_mm: y,
            w_mm: w,
            h_mm: h,
            rotation_deg: 0.0,
        }
    }

    #[test]
    fn a_curve_is_placed_from_its_view_box_and_drawn() {
        let stroke = Stroke {
            width_mm: 1.0,
            color: "#2e86c1".into(),
            style: Some(StrokeStyle::Dashed),
        };
        // Un pont : 24 mm de large, 21 mm de haut, en coordonnées 1/100 mm (comme LibreOffice).
        let d = Drawing::from_svg_path(
            "M0 2101c0-2801 2401-2801 2401 0",
            &frame(25.0, 49.0, 24.0, 21.0),
            Some("0 0 2401 2101"),
            None,
            Some(&stroke),
        )
        .unwrap();
        let b = d.path.bounds();
        assert!(
            (b.left() - 25.0).abs() < 0.1
                && (b.right() - 49.0).abs() < 0.1
                && (b.bottom() - 70.0).abs() < 0.1,
            "{b:?}"
        );
        let (f, asset) = render(&[d]).unwrap();
        assert!((f.x_mm - 24.0).abs() < 0.1 && f.w_mm > 24.0, "{f:?}");
        assert_eq!(asset.meta.mime, Mime::Png);
        // Environ 200 points par pouce.
        assert!(
            (asset.meta.width_px as f64 - f.w_mm * DPI / 25.4).abs() < 2.0,
            "{:?}",
            asset.meta
        );
        // Le trait est dessiné : des pixels opaques, de la couleur du trait.
        let img = image::load_from_memory(&asset.bytes).unwrap().to_rgba8();
        assert!(img.pixels().any(|p| p[3] > 200 && p[2] > 150 && p[0] < 100));
    }

    #[test]
    fn a_polygon_is_filled_and_a_shape_without_paint_is_left_out() {
        let tri = Drawing::from_points(
            "0,2000 1000,0 2000,2000",
            true,
            &frame(0.0, 0.0, 20.0, 20.0),
            Some("0 0 2000 2000"),
            Some("#ff0000"),
            None,
        )
        .unwrap();
        assert!(tri.fill.is_some());
        assert!(
            Drawing::from_points(
                "0,0 10,10",
                false,
                &frame(0.0, 0.0, 1.0, 1.0),
                None,
                None,
                None
            )
            .is_none()
        );
        assert!(
            Drawing::from_svg_path(
                "M 0 0 Q",
                &frame(0.0, 0.0, 1.0, 1.0),
                None,
                Some("#000000"),
                None
            )
            .is_none()
        );
    }
}
