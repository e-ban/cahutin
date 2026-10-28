//! Géométrie des formes dessinées par un tracé : le même tracé sert à l'écran (SVG) et à l'export
//! (forme personnalisée ODG, toujours modifiable dans LibreOffice).

use crate::model::ShapeKind;

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
        ShapeKind::Rect | ShapeKind::Ellipse | ShapeKind::Line | ShapeKind::Arrow => return None,
    })
}

/// Trait ou flèche : un segment, du milieu du bord gauche au milieu du bord droit du cadre.
pub fn is_stroke_only(kind: ShapeKind) -> bool {
    matches!(kind, ShapeKind::Line | ShapeKind::Arrow)
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn screen_uses_the_same_paths() {
        let ts = include_str!("../../../ui/src/lib/shapePaths.ts");
        for kind in [ShapeKind::Star, ShapeKind::Heart, ShapeKind::Bubble, ShapeKind::Cloud] {
            let line = format!("{kind:?}: \"{}\"", path(kind).unwrap());
            assert!(ts.contains(&line), "tracé de {kind:?} différent à l'écran");
        }
    }
}
