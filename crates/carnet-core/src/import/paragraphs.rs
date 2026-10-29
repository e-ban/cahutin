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
    /// Pour chaque ligne sauf la dernière : vrai si la ligne suivante la continue (jointe par une
    /// espace), faux si elle commence un nouveau paragraphe.
    pub joins: Vec<bool>,
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

fn starts_bold(l: &Line) -> bool {
    l.runs.first().is_some_and(|r| r.bold)
}

/// Le bord haut ou bas d'une forme passe entre deux lignes (encadré, case de tableau) : elles sont séparées.
fn edge_between(last: &Line, l: &Line, obstacles: &[Frame]) -> bool {
    let (top, bottom) = (last.y_mm + last.h_mm, l.y_mm);
    let (left, right) = (last.x_mm.min(l.x_mm), (last.x_mm + last.w_mm).max(l.x_mm + l.w_mm));
    obstacles.iter().any(|o| {
        let overlaps = o.x_mm < right && o.x_mm + o.w_mm > left;
        overlaps && [o.y_mm, o.y_mm + o.h_mm].iter().any(|e| *e > top - 0.1 && *e < bottom + 0.1)
    })
}

fn follows(last: &Line, l: &Line, obstacles: &[Frame]) -> bool {
    let dy = l.y_mm - last.y_mm;
    !edge_between(last, l, obstacles)
        && last.font == l.font
        && (last.size_pt - l.size_pt).abs() < 0.01
        && starts_bold(last) == starts_bold(l)
        && dy > 0.0
        && dy <= MAX_PITCH * l.size_pt * MM_PER_PT
}

/// Largeur estimée du premier mot d'une ligne, espace comprise (un demi-cadratin par lettre).
fn first_word_mm(l: &Line) -> f64 {
    let text: String = l.runs.iter().map(|r| r.text.as_str()).collect();
    let letters = text.split_whitespace().next().map_or(0, |w| w.chars().count());
    (letters as f64 * 0.5 + 0.25) * l.size_pt * MM_PER_PT
}

/// Un autre texte sur la même hauteur juste à côté (puce et élément de liste, cases voisines) : la ligne
/// est un élément de liste ou de tableau, son saut de ligne reste.
fn has_side_neighbour(l: &Line, all: &[Line]) -> bool {
    all.iter().any(|m| {
        let overlap = (l.y_mm + l.h_mm).min(m.y_mm + m.h_mm) - l.y_mm.max(m.y_mm);
        let gap_left = l.x_mm - (m.x_mm + m.w_mm);
        let gap_right = m.x_mm - (l.x_mm + l.w_mm);
        overlap >= 0.5 * l.h_mm.min(m.h_mm) && ((-0.5..=15.0).contains(&gap_left) || (-0.5..=15.0).contains(&gap_right))
    })
}

/// Bord droit de la colonne d'une ligne : la plus longue ligne de la page partant du même bord gauche,
/// ou plus tôt, le bord gauche d'une image ou d'une forme placée à sa droite sur la même hauteur (habillage).
fn column_right(l: &Line, all: &[Line], obstacles: &[Frame]) -> f64 {
    let column = all.iter().filter(|m| (m.x_mm - l.x_mm).abs() <= EDGE_MM).map(|m| m.x_mm + m.w_mm).fold(l.x_mm + l.w_mm, f64::max);
    obstacles
        .iter()
        .filter(|o| o.x_mm > l.x_mm + l.w_mm - 0.5 && o.y_mm < l.y_mm + l.h_mm && o.y_mm + o.h_mm > l.y_mm)
        .map(|o| o.x_mm)
        .fold(column, f64::min)
}

/// La ligne `b` continue `a` (retour à la ligne automatique) : le premier mot de `b` n'aurait pas tenu au
/// bout de `a` dans sa colonne. Sinon le saut de ligne était voulu (liste, lignes courtes) et reste.
fn continues(a: &Line, b: &Line, align: Align, all: &[Line], obstacles: &[Frame]) -> bool {
    align == Align::Left && !has_side_neighbour(a, all) && !has_side_neighbour(b, all) && a.x_mm + a.w_mm + first_word_mm(b) > column_right(a, all, obstacles)
}

/// Lignes rattachées en blocs ; les blocs suivent l'ordre de leur première ligne dans l'entrée.
/// `obstacles` : cadres des images et des formes de la page (habillage, encadrés).
pub fn merge_lines(obstacles: &[Frame], lines: Vec<Line>) -> Vec<Block> {
    let mut order: Vec<usize> = (0..lines.len()).collect();
    order.sort_by(|&a, &b| lines[a].y_mm.total_cmp(&lines[b].y_mm).then(lines[a].x_mm.total_cmp(&lines[b].x_mm)));
    let mut blocks: Vec<Block> = Vec::new();
    for i in order {
        let l = &lines[i];
        let target = blocks.iter_mut().rev().find_map(|b| {
            let last = b.lines.last().unwrap();
            if !follows(last, l, obstacles) {
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
            None => blocks.push(Block { align: Align::Left, lines: vec![l.clone()], first: i, joins: Vec::new() }),
        }
    }
    for b in &mut blocks {
        b.joins = b.lines.windows(2).map(|w| continues(&w[0], &w[1], b.align, &lines, obstacles)).collect();
    }
    blocks.sort_by_key(|b| b.first);
    blocks
}

/// Texte libre : un paragraphe (lignes jointes par une espace), cadre de toutes les lignes avec sa marge.
pub fn block_element(b: &Block) -> Element {
    let first = &b.lines[0];
    let color = first.runs.first().and_then(|r| r.color.clone()).unwrap_or_else(|| "#000000".into());
    let style = TextStyle { font: first.font.clone(), size_pt: first.size_pt, color, align: b.align, valign: None, padding_mm: None };
    // Lignes qui se continuent : jointes par une espace ; sinon un nouveau paragraphe.
    let mut paragraphs: Vec<Vec<Run>> = vec![Vec::new()];
    for (i, line) in b.lines.iter().enumerate() {
        let mut line_runs = line.runs.clone();
        if i > 0
            && b.joins[i - 1]
            && let Some(r) = line_runs.first_mut()
        {
            r.text = r.text.trim_start().to_string();
        }
        let joined_next = b.joins.get(i).copied().unwrap_or(false);
        if let Some(r) = line_runs.last_mut() {
            r.text = if joined_next { format!("{} ", r.text.trim_end()) } else { r.text.trim_end().to_string() };
        }
        paragraphs.last_mut().unwrap().extend(line_runs);
        if i + 1 < b.lines.len() && !joined_next {
            paragraphs.push(Vec::new());
        }
    }
    for r in paragraphs.iter_mut().flatten() {
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
        kind: ElementKind::Text(TextProps {
            content: RichText { paragraphs: paragraphs.into_iter().map(|runs| Paragraph { align: None, runs }).collect() },
            style,
            prompt: None,
            frame: None,
        }),
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
        let blocks = merge_lines(&[], vec![
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
        let blocks = merge_lines(&[], vec![line(20.0, 40.0, 40.0, "Un"), line(20.0, 50.0, 40.0, "Deux")]);
        assert_eq!(blocks.len(), 2);
    }

    #[test]
    fn different_font_or_size_splits() {
        let mut other_font = line(20.0, 44.86, 40.0, "Deux");
        other_font.font = "Andika".into();
        let mut other_size = line(20.0, 49.72, 40.0, "Trois");
        other_size.size_pt = 14.0;
        let blocks = merge_lines(&[], vec![line(20.0, 40.0, 40.0, "Un"), other_font, other_size]);
        assert_eq!(blocks.len(), 3);
    }

    #[test]
    fn centred_lines_merge_as_centre() {
        let blocks = merge_lines(&[], vec![line(80.0, 40.0, 50.0, "Une longue ligne"), line(95.0, 44.86, 20.0, "courte")]);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].align, Align::Center);
        let ElementKind::Text(t) = block_element(&blocks[0]).kind else { unreachable!() };
        assert_eq!(t.style.align, Align::Center);
    }

    #[test]
    fn bold_run_inside_a_line_is_kept() {
        let mut l = line(20.0, 29.92, 86.61, "");
        l.runs = vec![run("Consigne : ", true), run("observe l'image puis colorie la pomme.", false)];
        let e = block_element(&merge_lines(&[], vec![l])[0]);
        let ElementKind::Text(t) = &e.kind else { unreachable!() };
        let runs = &t.content.paragraphs[0].runs;
        assert_eq!(runs.len(), 2);
        assert!(runs[0].bold && !runs[1].bold);
        assert_eq!(runs[0].text, "Consigne : ");
        // Police, taille et couleur sont celles du style : les passages ne les répètent pas.
        assert_eq!((t.style.font.as_str(), t.style.size_pt, t.style.color.as_str()), ("Liberation Serif", 12.0, "#000000"));
        assert!(runs.iter().all(|r| r.font.is_none() && r.size_pt.is_none() && r.color.is_none()));
    }

    fn paragraphs_of(e: &Element) -> Vec<String> {
        let ElementKind::Text(t) = &e.kind else { panic!("pas un texte") };
        t.content.paragraphs.iter().map(|p| p.runs.iter().map(|r| r.text.as_str()).collect()).collect()
    }

    /// Liste LibreOffice relevée par la relecture : puces à 26,39 mm, éléments à 32,74 mm, pas de 4,87 mm.
    #[test]
    fn list_items_keep_their_line_breaks() {
        let mut lines = Vec::new();
        for (i, item) in ["pomme", "poire", "banane"].iter().enumerate() {
            let y = 50.0 + i as f64 * 4.87;
            lines.push(line(26.39, y, 2.0, "•"));
            lines.push(line(32.74, y, 12.0, item));
        }
        let elements: Vec<Element> = merge_lines(&[], lines).iter().map(block_element).collect();
        let items = elements.iter().find(|e| paragraphs_of(e).concat().contains("pomme")).unwrap();
        assert_eq!(paragraphs_of(items), ["pomme", "poire", "banane"]);
        let bullets = elements.iter().find(|e| paragraphs_of(e).concat().contains('•')).unwrap();
        assert_eq!(paragraphs_of(bullets), ["•", "•", "•"]);
    }

    #[test]
    fn short_lines_stay_separate_paragraphs() {
        let blocks = merge_lines(&[], vec![
            line(20.0, 40.0, 5.0, "un"),
            line(20.0, 44.86, 9.0, "deux"),
            line(20.0, 49.72, 10.0, "trois"),
            line(20.0, 100.0, 170.0, "Une longue ligne qui occupe toute la largeur de la colonne de texte de la page."),
        ]);
        assert_eq!(paragraphs_of(&block_element(&blocks[0])), ["un", "deux", "trois"]);
    }

    #[test]
    fn bold_heading_is_not_merged_with_its_body() {
        let mut heading = line(20.0, 40.0, 30.0, "Exercice 1 :");
        heading.runs[0].bold = true;
        let blocks = merge_lines(&[], vec![heading, line(20.0, 44.86, 120.0, "Recopie les mots dans l'ordre.")]);
        assert_eq!(blocks.len(), 2);
    }

    /// Consigne enroulée autour d'une image (fiche du banc) : la colonne est large, mais l'image coupe les lignes.
    #[test]
    fn lines_wrapped_around_an_image_continue() {
        let image = Frame { x_mm: 65.02, y_mm: 34.59, w_mm: 79.98, h_mm: 60.0, rotation_deg: 0.0 };
        let blocks = merge_lines(&[image], vec![
            line(20.04, 29.92, 86.61, "Consigne : observe l'image puis colorie la pomme."),
            line(20.04, 39.66, 43.74, "Complète le tableau avec "),
            line(20.04, 44.52, 40.82, "les mots : rouge, ronde, "),
            line(20.04, 49.39, 8.37, "fruit."),
        ]);
        let all: Vec<Vec<String>> = blocks.iter().map(|b| paragraphs_of(&block_element(b))).collect();
        assert!(all.iter().any(|p| p.iter().any(|x| x == "Complète le tableau avec les mots : rouge, ronde, fruit.")), "{all:?}");
    }

    /// Case de tableau puis texte d'un encadré : le bord haut de l'encadré passe entre les deux lignes.
    #[test]
    fn a_shape_edge_between_lines_splits() {
        let boxed = Frame { x_mm: 18.0, y_mm: 44.78, w_mm: 120.0, h_mm: 25.0, rotation_deg: 0.0 };
        let blocks = merge_lines(&[boxed], vec![line(20.0, 40.0, 15.0, "Couleur"), line(20.0, 44.86, 50.0, "Écris une phrase.")]);
        assert_eq!(blocks.len(), 2);
    }

    #[test]
    fn frame_includes_padding() {
        let e = block_element(&merge_lines(&[], vec![line(20.0, 40.0, 50.0, "Seule")])[0]);
        assert!((e.frame.x_mm - 18.5).abs() < 1e-9 && (e.frame.w_mm - 54.0).abs() < 1e-9, "{:?}", e.frame);
        assert!((e.frame.y_mm - 38.5).abs() < 1e-9 && (e.frame.h_mm - (4.71 + 3.0)).abs() < 1e-9, "{:?}", e.frame);
        assert!(!e.locked && !e.zone);
    }
}
