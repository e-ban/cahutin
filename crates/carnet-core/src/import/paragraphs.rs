//! Recollage des lignes : l'import PDF de LibreOffice donne un cadre par ligne ; les lignes d'un même
//! paragraphe redeviennent un seul texte, plus facile à reprendre.

use crate::model::*;

/// Une ligne de texte posée par l'import, passages avec police, taille et couleur explicites.
#[derive(Clone, Debug)]
pub struct Line {
    pub x_mm: f64,
    pub y_mm: f64,
    pub w_mm: f64,
    pub h_mm: f64,
    pub font: String,
    pub size_pt: f64,
    pub runs: Vec<Run>,
}

#[derive(Clone, Debug)]
pub struct Block {
    pub align: Align,
    pub lines: Vec<Line>,
    /// Indice, dans l'entrée de `merge_lines`, de la première ligne du bloc.
    pub first: usize,
}

const MM_PER_PT: f64 = 0.3528;
/// Bords ou centres alignés à cette distance près.
const EDGE_MM: f64 = 1.0;
/// Écart maximal entre deux débuts de ligne, en tailles de police.
const MAX_PITCH: f64 = 1.5;

/// Alignement commun de deux lignes, dans l'ordre : bord gauche, centre, bord droit.
fn shared_align(a: &Line, b: &Line) -> Vec<Align> {
    let mut out = Vec::new();
    if (a.x_mm - b.x_mm).abs() <= EDGE_MM {
        out.push(Align::Left);
    }
    if ((a.x_mm + a.w_mm / 2.0) - (b.x_mm + b.w_mm / 2.0)).abs() <= EDGE_MM {
        out.push(Align::Center);
    }
    if ((a.x_mm + a.w_mm) - (b.x_mm + b.w_mm)).abs() <= EDGE_MM {
        out.push(Align::Right);
    }
    out
}

fn follows(last: &Line, l: &Line) -> bool {
    let dy = l.y_mm - last.y_mm;
    last.font == l.font && (last.size_pt - l.size_pt).abs() < 0.01 && dy > 0.0 && dy <= MAX_PITCH * l.size_pt * MM_PER_PT
}

/// Lignes rattachées en blocs ; les blocs suivent l'ordre de leur première ligne dans l'entrée.
pub fn merge_lines(lines: Vec<Line>) -> Vec<Block> {
    let mut order: Vec<usize> = (0..lines.len()).collect();
    order.sort_by(|&a, &b| lines[a].y_mm.total_cmp(&lines[b].y_mm).then(lines[a].x_mm.total_cmp(&lines[b].x_mm)));
    let mut blocks: Vec<Block> = Vec::new();
    for i in order {
        let l = &lines[i];
        let target = blocks.iter_mut().rev().find_map(|b| {
            let last = b.lines.last().unwrap();
            if !follows(last, l) {
                return None;
            }
            let shared = shared_align(last, l);
            let align = if b.lines.len() == 1 { shared.first().copied() } else { shared.contains(&b.align).then_some(b.align) };
            align.map(|a| (b, a))
        });
        match target {
            Some((b, align)) => {
                b.align = align;
                b.lines.push(l.clone());
            }
            None => blocks.push(Block { align: Align::Left, lines: vec![l.clone()], first: i }),
        }
    }
    blocks.sort_by_key(|b| b.first);
    blocks
}

/// Texte libre : un paragraphe (lignes jointes par une espace), cadre de toutes les lignes avec sa marge.
pub fn block_element(b: &Block) -> Element {
    let first = &b.lines[0];
    let color = first.runs.first().and_then(|r| r.color.clone()).unwrap_or_else(|| "#000000".into());
    let style = TextStyle { font: first.font.clone(), size_pt: first.size_pt, color, align: b.align, valign: None, padding_mm: None };
    let mut runs: Vec<Run> = Vec::new();
    for (i, line) in b.lines.iter().enumerate() {
        let mut line_runs = line.runs.clone();
        if i > 0
            && let Some(r) = line_runs.first_mut()
        {
            r.text = r.text.trim_start().to_string();
        }
        if i + 1 < b.lines.len() {
            if let Some(r) = line_runs.last_mut() {
                r.text = format!("{} ", r.text.trim_end());
            }
        } else if let Some(r) = line_runs.last_mut() {
            r.text = r.text.trim_end().to_string();
        }
        runs.extend(line_runs);
    }
    for r in &mut runs {
        if r.font.as_deref() == Some(style.font.as_str()) {
            r.font = None;
        }
        if r.size_pt.is_some_and(|s| (s - style.size_pt).abs() < 0.01) {
            r.size_pt = None;
        }
        if r.color.as_deref() == Some(style.color.as_str()) {
            r.color = None;
        }
    }
    let pad = TEXT_PADDING_MM;
    let left = b.lines.iter().map(|l| l.x_mm).fold(f64::INFINITY, f64::min);
    let right = b.lines.iter().map(|l| l.x_mm + l.w_mm).fold(f64::NEG_INFINITY, f64::max);
    let top = first.y_mm;
    let bottom = b.lines.iter().map(|l| l.y_mm + l.h_mm).fold(f64::NEG_INFINITY, f64::max);
    Element {
        id: new_id(),
        frame: Frame { x_mm: left - pad, y_mm: top - pad, w_mm: right - left + 2.0 * pad + 1.0, h_mm: bottom - top + 2.0 * pad, rotation_deg: 0.0 },
        locked: false,
        zone: false,
        group: None,
        kind: ElementKind::Text(TextProps { content: RichText { paragraphs: vec![Paragraph { align: None, runs }] }, style, prompt: None, frame: None }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(text: &str, bold: bool) -> Run {
        Run { text: text.into(), bold, italic: false, underline: false, color: Some("#000000".into()), font: Some("Liberation Serif".into()), size_pt: Some(12.0) }
    }

    fn line(x: f64, y: f64, w: f64, text: &str) -> Line {
        Line { x_mm: x, y_mm: y, w_mm: w, h_mm: 4.71, font: "Liberation Serif".into(), size_pt: 12.0, runs: vec![run(text, false)] }
    }

    fn text_of(e: &Element) -> String {
        let ElementKind::Text(t) = &e.kind else { panic!("pas un texte") };
        t.content.paragraphs.iter().map(|p| p.runs.iter().map(|r| r.text.as_str()).collect::<String>()).collect::<Vec<_>>().join("\n")
    }

    #[test]
    fn consecutive_lines_with_same_left_edge_merge() {
        let blocks = merge_lines(vec![
            line(20.04, 39.66, 43.74, "Complète le tableau avec "),
            line(20.04, 44.52, 40.82, "les mots : rouge, ronde, "),
            line(20.04, 49.39, 8.37, "fruit."),
        ]);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].align, Align::Left);
        assert_eq!(text_of(&block_element(&blocks[0])), "Complète le tableau avec les mots : rouge, ronde, fruit.");
    }

    #[test]
    fn too_large_a_gap_splits() {
        let blocks = merge_lines(vec![line(20.0, 40.0, 40.0, "Un"), line(20.0, 50.0, 40.0, "Deux")]);
        assert_eq!(blocks.len(), 2);
    }

    #[test]
    fn different_font_or_size_splits() {
        let mut other_font = line(20.0, 44.86, 40.0, "Deux");
        other_font.font = "Andika".into();
        let mut other_size = line(20.0, 49.72, 40.0, "Trois");
        other_size.size_pt = 14.0;
        let blocks = merge_lines(vec![line(20.0, 40.0, 40.0, "Un"), other_font, other_size]);
        assert_eq!(blocks.len(), 3);
    }

    #[test]
    fn centred_lines_merge_as_centre() {
        let blocks = merge_lines(vec![line(80.0, 40.0, 50.0, "Une longue ligne"), line(95.0, 44.86, 20.0, "courte")]);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].align, Align::Center);
        let ElementKind::Text(t) = block_element(&blocks[0]).kind else { unreachable!() };
        assert_eq!(t.style.align, Align::Center);
    }

    #[test]
    fn bold_run_inside_a_line_is_kept() {
        let mut l = line(20.0, 29.92, 86.61, "");
        l.runs = vec![run("Consigne : ", true), run("observe l'image puis colorie la pomme.", false)];
        let e = block_element(&merge_lines(vec![l])[0]);
        let ElementKind::Text(t) = &e.kind else { unreachable!() };
        let runs = &t.content.paragraphs[0].runs;
        assert_eq!(runs.len(), 2);
        assert!(runs[0].bold && !runs[1].bold);
        assert_eq!(runs[0].text, "Consigne : ");
        // Police, taille et couleur sont celles du style : les passages ne les répètent pas.
        assert_eq!((t.style.font.as_str(), t.style.size_pt, t.style.color.as_str()), ("Liberation Serif", 12.0, "#000000"));
        assert!(runs.iter().all(|r| r.font.is_none() && r.size_pt.is_none() && r.color.is_none()));
    }

    #[test]
    fn frame_includes_padding() {
        let e = block_element(&merge_lines(vec![line(20.0, 40.0, 50.0, "Seule")])[0]);
        assert!((e.frame.x_mm - 18.5).abs() < 1e-9 && (e.frame.w_mm - 54.0).abs() < 1e-9, "{:?}", e.frame);
        assert!((e.frame.y_mm - 38.5).abs() < 1e-9 && (e.frame.h_mm - (4.71 + 3.0)).abs() < 1e-9, "{:?}", e.frame);
        assert!(!e.locked && !e.zone);
    }
}
