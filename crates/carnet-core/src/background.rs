//! Fonds de page : couleur et motifs d'école (Seyès, petits carreaux, lignes, pointillés).
//! La géométrie, en mm depuis le coin haut gauche, est la même à l'écran et dans l'ODG
//! (l'interface la recalcule et la vérifie contre `fixtures/background_marks.json`).

use crate::model::{PageSize, Pattern};
use serde::Serialize;

/// Couleur du papier kraft proposé.
pub const KRAFT: &str = "#d8bc8a";

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(tag = "type")]
pub enum Mark {
    Line { x1: f64, y1: f64, x2: f64, y2: f64, width_mm: f64, color: &'static str },
    Dot { x: f64, y: f64, d_mm: f64, color: &'static str },
}

const SEYES_THIN: &str = "#c9c3e6";
const SEYES_STRONG: &str = "#9d93d4";
const SEYES_MARGIN: &str = "#e8a2a2";
const SEYES_MARGIN_MM: f64 = 25.0;
const SQUARES: &str = "#b9d3ee";
const LINES: &str = "#c8c8c8";
const DOTS: &str = "#bdbdbd";

/// Multiples de `step` strictement à l'intérieur de `limit`, avec leur rang (1, 2, …).
fn steps(step: f64, limit: f64) -> impl Iterator<Item = (u32, f64)> {
    (1..).map(move |k| (k, (k as f64 * step * 1000.0).round() / 1000.0)).take_while(move |&(_, v)| v < limit)
}

fn h(y: f64, w: f64, width_mm: f64, color: &'static str) -> Mark {
    Mark::Line { x1: 0.0, y1: y, x2: w, y2: y, width_mm, color }
}

fn v(x: f64, h: f64, width_mm: f64, color: &'static str) -> Mark {
    Mark::Line { x1: x, y1: 0.0, x2: x, y2: h, width_mm, color }
}

/// Traits et points du motif, sur toute la page.
pub fn marks(size: PageSize, pattern: Pattern) -> Vec<Mark> {
    let (w, ht) = (size.width_mm, size.height_mm);
    let mut out = Vec::new();
    match pattern {
        Pattern::Seyes => {
            // Grands carreaux : une ligne fine tous les 2 mm, marquée tous les 8 mm.
            out.extend(steps(2.0, ht).map(|(k, y)| if k % 4 == 0 { h(y, w, 0.25, SEYES_STRONG) } else { h(y, w, 0.15, SEYES_THIN) }));
            out.extend(steps(8.0, w).map(|(_, x)| v(x, ht, 0.25, SEYES_STRONG)));
            out.push(v(SEYES_MARGIN_MM, ht, 0.3, SEYES_MARGIN));
        }
        Pattern::SmallSquares => {
            out.extend(steps(5.0, ht).map(|(_, y)| h(y, w, 0.15, SQUARES)));
            out.extend(steps(5.0, w).map(|(_, x)| v(x, ht, 0.15, SQUARES)));
        }
        Pattern::Lines => out.extend(steps(8.0, ht).map(|(_, y)| h(y, w, 0.2, LINES))),
        Pattern::Dots => {
            for (_, y) in steps(10.0, ht) {
                out.extend(steps(10.0, w).map(|(_, x)| Mark::Dot { x, y, d_mm: 0.6, color: DOTS }));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{A4_PORTRAIT, Page};

    fn lines(m: &[Mark], horizontal: bool) -> Vec<&Mark> {
        m.iter().filter(|k| matches!(k, Mark::Line { y1, y2, x1, x2, .. } if (y1 == y2) == horizontal && (x1 == x2) != horizontal)).collect()
    }

    #[test]
    fn seyes_has_thin_and_strong_lines_and_a_margin() {
        let m = marks(A4_PORTRAIT, Pattern::Seyes);
        let h = lines(&m, true);
        assert_eq!(h.len(), 148);
        assert_eq!(h.iter().filter(|k| matches!(k, Mark::Line { width_mm, .. } if *width_mm == 0.25)).count(), 37);
        let v = lines(&m, false);
        assert_eq!(v.iter().filter(|k| matches!(k, Mark::Line { color, .. } if *color == "#9d93d4")).count(), 26);
        assert!(v.iter().any(|k| matches!(k, Mark::Line { x1, color, .. } if *x1 == 25.0 && *color == "#e8a2a2")));
    }

    #[test]
    fn marks_cover_landscape_pages() {
        let size = PageSize { width_mm: 297.0, height_mm: 210.0 };
        for (pattern, step) in [(Pattern::Seyes, 2.0), (Pattern::SmallSquares, 5.0), (Pattern::Lines, 8.0)] {
            let m = marks(size, pattern);
            let max_y = lines(&m, true).iter().map(|k| if let Mark::Line { y1, x2, .. } = k { assert_eq!(*x2, 297.0); *y1 } else { 0.0 }).fold(0.0, f64::max);
            assert!(210.0 - max_y <= step, "{pattern:?} : bande vide en bas");
        }
        let dots = marks(size, Pattern::Dots);
        let max_x = dots.iter().map(|k| if let Mark::Dot { x, .. } = k { *x } else { 0.0 }).fold(0.0, f64::max);
        assert!(297.0 - max_x <= 10.0);
    }

    #[test]
    fn old_pages_load_without_background() {
        let json = r#"{"id":"p","size":{"width_mm":210,"height_mm":297},"elements":[],"grid":null}"#;
        let p: Page = serde_json::from_str(json).unwrap();
        assert_eq!(p.background, None);
    }

    #[test]
    fn reference_file_matches() {
        let all: Vec<(Pattern, Vec<Mark>)> = [Pattern::Seyes, Pattern::SmallSquares, Pattern::Lines, Pattern::Dots].into_iter().map(|p| (p, marks(A4_PORTRAIT, p))).collect();
        let json = serde_json::to_string_pretty(&all).unwrap();
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/background_marks.json");
        if std::env::var_os("CARNET_WRITE_FIXTURES").is_some() {
            std::fs::write(path, &json).unwrap();
        }
        assert_eq!(std::fs::read_to_string(path).unwrap_or_default(), json, "lancer avec CARNET_WRITE_FIXTURES=1 pour régénérer");
    }
}
