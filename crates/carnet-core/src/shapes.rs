//! Géométrie des formes dessinées par un tracé : le même tracé sert à l'écran (SVG) et à l'export
//! (forme personnalisée ODG, toujours modifiable dans LibreOffice).

use crate::model::{HeadSize, LineEnd, LineEnds, ShapeKind, ShapeProps};

/// Le tracé s'inscrit dans un carré de `VIEWBOX` unités, étiré à la taille du cadre.
pub const VIEWBOX: f64 = 1000.0;

/// Tracé (commandes M, L, C et Z) ; `None` pour les formes natives (rectangle, ellipse, traits).
pub fn path(kind: ShapeKind) -> Option<&'static str> {
    Some(match kind {
        ShapeKind::Star => "M 500 0 L 618 382 L 1000 382 L 691 618 L 809 1000 L 500 764 L 191 1000 L 309 618 L 0 382 L 382 382 Z",
        ShapeKind::Heart => {
            "M 500 1000 C 150 760 0 560 0 310 C 0 120 150 0 300 0 C 400 0 470 50 500 130 C 530 50 600 0 700 0 C 850 0 1000 120 1000 310 C 1000 560 850 760 500 1000 Z"
        }
        // Bulle de parole : la pointe en bas à gauche.
        ShapeKind::Bubble => {
            "M 120 0 L 880 0 C 946 0 1000 54 1000 120 L 1000 660 C 1000 726 946 780 880 780 L 380 780 L 180 1000 L 240 780 L 120 780 C 54 780 0 726 0 660 L 0 120 C 0 54 54 0 120 0 Z"
        }
        ShapeKind::Cloud => {
            "M 500 127 C 598 -89 876 -8 779 209 C 917 144 1065 351 927 416 C 1054 434 1002 669 876 651 C 1015 785 787 938 648 805 C 648 1065 352 1065 352 805 C 213 938 -15 785 124 651 C -2 669 -54 434 73 416 C -65 351 83 144 221 209 C 124 -8 402 -89 500 127 Z"
        }
        ShapeKind::Rect | ShapeKind::Ellipse | ShapeKind::Line | ShapeKind::Arrow | ShapeKind::Path => return None,
    })
}

/// Le tracé d'une forme : celui de sa sorte, ou pour un tracé libre le sien, s'il est valide.
pub fn path_of(s: &crate::model::ShapeProps) -> Option<&str> {
    match s.shape {
        ShapeKind::Path => s.path.as_deref().filter(|p| valid_path(p)),
        kind => path(kind),
    }
}

/// Plus long tracé libre accepté (une ligne de graphisme en fait quelques milliers de caractères).
const MAX_PATH_LEN: usize = 20_000;

/// Un tracé libre : commandes M, L, C et Z suivies de leurs nombres (2, 2, 6 et 0), séparés par des espaces,
/// commençant par M. Tout autre contenu (commandes relatives, arcs, lettres, texte) est refusé : le même
/// tracé part tel quel dans l'ODG.
pub fn valid_path(p: &str) -> bool {
    if p.len() > MAX_PATH_LEN || !p.trim_start().starts_with('M') {
        return false;
    }
    let mut tokens = p.split_whitespace().peekable();
    while let Some(cmd) = tokens.next() {
        let n = match cmd {
            "M" | "L" => 2,
            "C" => 6,
            "Z" => 0,
            _ => return false,
        };
        for _ in 0..n {
            match tokens.next().map(str::parse::<f64>) {
                Some(Ok(v)) if v.is_finite() && v.abs() <= 100_000.0 => {}
                _ => return false,
            }
        }
    }
    true
}

/// Trait ou flèche : un segment, du milieu du bord gauche au milieu du bord droit du cadre.
pub fn is_stroke_only(kind: ShapeKind) -> bool {
    matches!(kind, ShapeKind::Line | ShapeKind::Arrow)
}

/// Terminaisons d'un trait qui n'en précise pas : aucune pour un trait, une flèche pleine au bout pour une flèche.
pub fn default_ends(kind: ShapeKind) -> LineEnds {
    let end = if kind == ShapeKind::Arrow { LineEnd::Arrow } else { LineEnd::None };
    LineEnds { start: LineEnd::None, end, size: HeadSize::Medium }
}

/// Terminaisons d'un trait : les siennes, ou celles de son genre.
pub fn ends_of(s: &ShapeProps) -> LineEnds {
    s.ends.unwrap_or_else(|| default_ends(s.shape))
}

/// Facteur de chaque taille de terminaison ; les mêmes à l'écran (`ui/src/lib/shapePaths.ts`, un test le vérifie).
pub const HEAD_FACTORS: [(HeadSize, f64); 3] = [(HeadSize::Small, 0.7), (HeadSize::Medium, 1.0), (HeadSize::Large, 1.5)];

/// Cadre d'un trait assez haut pour sa plus grande terminaison, autour du même centre (comme `withHitHeight` à
/// l'écran) : la zone cliquable la couvre.
pub fn grow_to_hit_height(f: &mut crate::model::Frame, s: &ShapeProps) {
    let ends = ends_of(s);
    if !is_stroke_only(s.shape) || (ends.start == LineEnd::None && ends.end == LineEnd::None) {
        return;
    }
    let h = head_width_mm(s);
    if f.h_mm < h {
        f.y_mm += (f.h_mm - h) / 2.0;
        f.h_mm = h;
    }
}

/// Diamètre d'un rond, côté d'un carré : cette part de la largeur d'une pointe.
pub const DOT_FACTOR: f64 = 0.6;

/// Largeur d'une pointe de flèche, en mm : quatre fois le trait, 4 mm au moins (la taille moyenne), multipliée par
/// le facteur de la taille choisie.
pub fn head_width_mm(s: &ShapeProps) -> f64 {
    let size = ends_of(s).size;
    let factor = HEAD_FACTORS.iter().find(|(k, _)| *k == size).map_or(1.0, |(_, f)| *f);
    (s.stroke.as_ref().map_or(1.0, |st| st.width_mm) * 4.0).max(4.0) * factor
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_simple_absolute_paths_are_valid() {
        assert!(valid_path("M 0 1000 C 0 0 250 0 250 1000 L 300 1000 Z"));
        assert!(valid_path("M 0.5 -2 L 10 20"));
        for bad in ["", "L 0 0", "M 0", "M 0 0 c 1 2 3 4 5 6", "M 0 0 A 1 1 0 0 1 2 2", "M 0 0 L x 1", "M 0 0 L NaN 1", "<svg>"] {
            assert!(!valid_path(bad), "{bad:?}");
        }
        assert!(!valid_path(&format!("M 0 0{}", " L 1 1".repeat(5000))));
    }

    /// Points de passage (extrémités des segments et des courbes) du tracé.
    fn anchors(p: &str) -> Vec<(f64, f64)> {
        let mut out = Vec::new();
        let mut nums: Vec<f64> = Vec::new();
        for tok in p.split_whitespace() {
            match tok {
                "M" | "L" | "C" | "Z" => {
                    if nums.len() >= 2 {
                        out.push((nums[nums.len() - 2], nums[nums.len() - 1]));
                    }
                    nums.clear();
                }
                n => nums.push(n.parse().expect("nombre")),
            }
        }
        out
    }

    #[test]
    fn drawn_shapes_have_a_closed_path_filling_the_frame() {
        for kind in [ShapeKind::Star, ShapeKind::Heart, ShapeKind::Bubble, ShapeKind::Cloud] {
            let p = path(kind).unwrap();
            assert!(p.starts_with("M ") && p.ends_with(" Z"), "{kind:?}");
            let pts = anchors(p);
            let (min_x, max_x) = pts.iter().fold((f64::MAX, f64::MIN), |(a, b), p| (a.min(p.0), b.max(p.0)));
            let (min_y, max_y) = pts.iter().fold((f64::MAX, f64::MIN), |(a, b), p| (a.min(p.1), b.max(p.1)));
            assert!(min_x >= -1.0 && max_x <= VIEWBOX + 1.0 && min_y >= -1.0 && max_y <= VIEWBOX + 1.0, "{kind:?}");
            assert!(max_x - min_x > 600.0 && max_y - min_y > 600.0, "{kind:?} ne remplit pas son cadre");
        }
        for kind in [ShapeKind::Rect, ShapeKind::Ellipse, ShapeKind::Line, ShapeKind::Arrow] {
            assert!(path(kind).is_none());
        }
        assert!(is_stroke_only(ShapeKind::Arrow) && !is_stroke_only(ShapeKind::Star));
    }

    fn line(kind: ShapeKind, width_mm: f64, ends: Option<LineEnds>) -> ShapeProps {
        ShapeProps {
            shape: kind,
            fill: None,
            stroke: Some(crate::model::Stroke { width_mm, color: "#333333".into(), style: None }),
            corner_radius_mm: 0.0,
            path: None,
            graphisme: None,
            ends,
        }
    }

    #[test]
    fn ends_default_by_kind() {
        let medium = HeadSize::Medium;
        assert_eq!(ends_of(&line(ShapeKind::Line, 1.0, None)), LineEnds { start: LineEnd::None, end: LineEnd::None, size: medium });
        assert_eq!(ends_of(&line(ShapeKind::Arrow, 1.0, None)), LineEnds { start: LineEnd::None, end: LineEnd::Arrow, size: medium });
        let set = LineEnds { start: LineEnd::Circle, end: LineEnd::OpenArrow, size: HeadSize::Large };
        assert_eq!(ends_of(&line(ShapeKind::Line, 1.0, Some(set))), set);
    }

    #[test]
    fn hit_height_covers_the_largest_end() {
        let large = Some(LineEnds { start: LineEnd::None, end: LineEnd::Arrow, size: HeadSize::Large });
        let mut f = crate::model::Frame { x_mm: 10.0, y_mm: 20.0, w_mm: 60.0, h_mm: 8.0, rotation_deg: 30.0 };
        grow_to_hit_height(&mut f, &line(ShapeKind::Arrow, 3.0, large));
        assert_eq!((f.h_mm, f.y_mm + f.h_mm / 2.0), (18.0, 24.0));
        let mut g = crate::model::Frame { x_mm: 10.0, y_mm: 20.0, w_mm: 60.0, h_mm: 8.0, rotation_deg: 0.0 };
        grow_to_hit_height(&mut g, &line(ShapeKind::Line, 3.0, None));
        assert_eq!(g.h_mm, 8.0);
    }

    #[test]
    fn head_width_matches_todays_arrow() {
        assert_eq!(head_width_mm(&line(ShapeKind::Arrow, 1.0, None)), 4.0);
        assert_eq!(head_width_mm(&line(ShapeKind::Arrow, 2.0, None)), 8.0);
        let small = Some(LineEnds { start: LineEnd::None, end: LineEnd::Arrow, size: HeadSize::Small });
        assert!((head_width_mm(&line(ShapeKind::Arrow, 1.0, small)) - 2.8).abs() < 1e-9);
        let large = Some(LineEnds { start: LineEnd::None, end: LineEnd::Arrow, size: HeadSize::Large });
        assert!((head_width_mm(&line(ShapeKind::Arrow, 2.0, large)) - 12.0).abs() < 1e-9);
    }

    #[test]
    fn old_shape_serializes_without_ends() {
        let json = serde_json::to_string(&line(ShapeKind::Arrow, 1.0, None)).unwrap();
        assert!(!json.contains("ends"), "{json}");
        let back: ShapeProps = serde_json::from_str(r#"{"shape":"Arrow","fill":null,"stroke":null,"corner_radius_mm":0}"#).unwrap();
        assert_eq!(back.ends, None);
    }

    #[test]
    fn screen_uses_the_same_head_factors() {
        let ts = include_str!("../../../ui/src/lib/shapePaths.ts");
        for (size, factor) in HEAD_FACTORS {
            let line = format!("  {size:?}: {factor},");
            assert!(ts.contains(&line), "taille {size:?} différente à l'écran : {line}");
        }
        assert!(ts.contains(&format!("export const DOT_FACTOR = {DOT_FACTOR};")), "rond et carré différents à l'écran");
    }

    #[test]
    fn screen_uses_the_same_paths() {
        let ts = include_str!("../../../ui/src/lib/shapePaths.ts");
        for kind in [ShapeKind::Star, ShapeKind::Heart, ShapeKind::Bubble, ShapeKind::Cloud] {
            let line = format!("{kind:?}: \"{}\"", path(kind).unwrap());
            assert!(ts.contains(&line), "tracé de {kind:?} différent à l'écran");
        }
    }
}
