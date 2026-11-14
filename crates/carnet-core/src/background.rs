//! Fonds de page : couleur et motifs d'école (Seyès, petits carreaux, lignes, pointillés).
//! La géométrie, en mm depuis le coin haut gauche, est la même à l'écran et dans l'ODG
//! (l'interface la recalcule et la vérifie contre `fixtures/background_marks.json`).

use crate::fonts::FontMetrics;
use crate::model::{PageSize, Pattern, TextFrame};
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

/// Motif dans un cadre de texte : comme sur la page, sans la marge rouge du Seyès.
pub fn frame_marks(size: PageSize, pattern: Pattern) -> Vec<Mark> {
    marks(size, pattern).into_iter().filter(|m| !matches!(m, Mark::Line { color, .. } if *color == SEYES_MARGIN)).collect()
}

/// Écart des lignes où l'on écrit dans un cadre : les lignes fortes du Seyès, les lignes simples ; la première à
/// un écart du haut du cadre.
pub const WRITING_LINE_MM: f64 = 8.0;

/// Le texte du cadre s'écrit sur ses lignes : le réglage est mis et le motif a des lignes où écrire.
pub fn writes_on_lines(frame: Option<&TextFrame>) -> bool {
    frame.is_some_and(|f| f.write_on_lines == Some(true) && matches!(f.pattern, Some(Pattern::Seyes | Pattern::Lines)))
}

/// Hauteurs d'Andika, la police de secours de l'écran, pour une police inconnue (comme ui/src/lib/lineSpacing.ts).
pub const FALLBACK_METRICS: FontMetrics = FontMetrics { units_per_em: 2048, ascent: 2500.0, descent: 800.0 };

/// Un texte écrit sur les lignes : `pitch_mm` d'une ligne de base à la suivante, la première à `pitch_mm` du haut
/// du cadre ; `descent_mm`, la descendante de la police telle que LibreOffice la compte (marge du haut de l'ODG).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WritingLines {
    pub pitch_mm: f64,
    pub descent_mm: f64,
}

/// Comme LibreOffice : ascendante et descendante arrondies au point à 600 ppp. Le pas est le plus petit multiple
/// de l'écart des lignes où tient la hauteur d'une ligne de la police (à un point près) : des lettres trop hautes
/// pour une ligne écrivent une ligne sur deux (ou sur trois) plutôt que de chevaucher la ligne d'au-dessus. Même
/// calcul à l'écran (`writingLineBox`, ui/src/lib/lineSpacing.ts).
pub fn writing_lines(metrics: Option<FontMetrics>, size_pt: f64) -> WritingLines {
    let m = metrics.filter(|m| m.units_per_em > 0 && m.ascent + m.descent > 0.0).unwrap_or(FALLBACK_METRICS);
    let dots = (size_pt * 600.0 / 72.0).round();
    let at = |units: f64| (dots * units / m.units_per_em as f64).round();
    let (ascent, descent) = (at(m.ascent), at(m.descent));
    let lines = ((ascent + descent - 1.0) * 25.4 / 600.0 / WRITING_LINE_MM).ceil().max(1.0);
    WritingLines { pitch_mm: lines * WRITING_LINE_MM, descent_mm: descent * 25.4 / 600.0 }
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
    fn writing_pitch_is_the_smallest_multiple_of_8_mm_the_font_fits_in() {
        let andika = FontMetrics { units_per_em: 2048, ascent: 2500.0, descent: 800.0 };
        let playwrite = FontMetrics { units_per_em: 1000, ascent: 1550.0, descent: 607.0 };
        // (police, taille, pas, descendante en points à 600 ppp) ; Andika 14 pt mesure 8,001 mm : sur une ligne.
        for (m, pt, pitch, dots) in [(andika, 12.0, 8.0, 39.0), (andika, 14.0, 8.0, 46.0), (andika, 20.0, 16.0, 65.0), (andika, 28.0, 16.0, 91.0), (playwrite, 10.0, 8.0, 50.0), (playwrite, 14.0, 16.0, 71.0), (playwrite, 28.0, 24.0, 141.0)] {
            let w = writing_lines(Some(m), pt);
            assert_eq!(w.pitch_mm, pitch, "{m:?} {pt} pt");
            assert!((w.descent_mm - dots * 25.4 / 600.0).abs() < 1e-9, "{m:?} {pt} pt : {w:?}");
        }
        assert_eq!(writing_lines(None, 14.0), writing_lines(Some(andika), 14.0));
        assert_eq!(writing_lines(Some(FontMetrics { units_per_em: 0, ascent: 0.0, descent: 0.0 }), 14.0), writing_lines(Some(andika), 14.0));
    }

    #[test]
    fn only_lined_patterns_with_the_setting_write_on_lines() {
        let f = |pattern, on| TextFrame { pattern, write_on_lines: on, ..TextFrame::default() };
        assert!(writes_on_lines(Some(&f(Some(Pattern::Seyes), Some(true)))));
        assert!(writes_on_lines(Some(&f(Some(Pattern::Lines), Some(true)))));
        assert!(!writes_on_lines(Some(&f(Some(Pattern::SmallSquares), Some(true)))));
        assert!(!writes_on_lines(Some(&f(Some(Pattern::Dots), Some(true)))));
        assert!(!writes_on_lines(Some(&f(Some(Pattern::Seyes), None))));
        assert!(!writes_on_lines(Some(&f(None, Some(true)))));
        assert!(!writes_on_lines(None));
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
        // Comparaison des valeurs, pas du texte : la copie Windows peut avoir des fins de ligne CRLF.
        let stored: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap_or_default()).unwrap_or_default();
        assert_eq!(stored, serde_json::to_value(&all).unwrap(), "lancer avec CARNET_WRITE_FIXTURES=1 pour régénérer");
    }
}
