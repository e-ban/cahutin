//! Nettoyage des pages reprises d'un PDF : le fond blanc dessiné comme un rectangle devient le fond de la
//! page, et les bordures décoratives éclatées en centaines de petits morceaux deviennent un groupe.

use crate::model::{Background, Element, ElementKind, Frame, Page, ShapeKind, new_id};

/// Écart toléré entre le rectangle et les bords de la page.
const PAGE_EDGE_MM: f64 = 1.0;
/// Plus grand côté d'un morceau de bordure (carreau du motif, trait d'ornement).
const FRAGMENT_MAX_MM: f64 = 12.0;
/// Deux morceaux à moins de cette distance appartiennent au même amas.
const FRAGMENT_GAP_MM: f64 = 1.0;
/// Nombre de morceaux à partir duquel un amas est une bordure éclatée, pas quelques formes voisines.
const FRAGMENT_MIN_COUNT: usize = 12;

/// Rectangle plein de la taille de la page, tout au fond : retiré, sa couleur devient celle du fond.
pub fn full_page_rect_to_background(page: &mut Page) {
    let Some(first) = page.elements.first() else { return };
    let ElementKind::Shape(s) = &first.kind else { return };
    let Some(fill) = s.fill.clone() else { return };
    let f = &first.frame;
    let (w, h) = (page.size.width_mm, page.size.height_mm);
    let covers = f.rotation_deg == 0.0 && f.x_mm <= PAGE_EDGE_MM && f.y_mm <= PAGE_EDGE_MM && f.x_mm + f.w_mm >= w - PAGE_EDGE_MM && f.y_mm + f.h_mm >= h - PAGE_EDGE_MM;
    let plain_edge = s.stroke.as_ref().is_none_or(|st| st.color == fill);
    if s.shape != ShapeKind::Rect || !covers || !plain_edge {
        return;
    }
    page.elements.remove(0);
    if !fill.eq_ignore_ascii_case("#ffffff") {
        page.background = Some(Background { color: fill, pattern: None });
    }
}

fn is_fragment(e: &Element) -> bool {
    matches!(e.kind, ElementKind::Shape(_)) && e.group.is_none() && !e.locked && e.frame.w_mm.max(e.frame.h_mm) <= FRAGMENT_MAX_MM
}

fn near(a: &Frame, b: &Frame) -> bool {
    let gap_x = (a.x_mm - (b.x_mm + b.w_mm)).max(b.x_mm - (a.x_mm + a.w_mm));
    let gap_y = (a.y_mm - (b.y_mm + b.h_mm)).max(b.y_mm - (a.y_mm + a.h_mm));
    gap_x <= FRAGMENT_GAP_MM && gap_y <= FRAGMENT_GAP_MM
}

/// Morceaux d'une bordure décorative (petites formes collées les unes aux autres) : un groupe par amas.
/// Renvoie le nombre de groupes formés.
pub fn group_fragments(page: &mut Page) -> usize {
    let idx: Vec<usize> = (0..page.elements.len()).filter(|&i| is_fragment(&page.elements[i])).collect();
    // Amas : union-find sur les morceaux voisins.
    let mut parent: Vec<usize> = (0..idx.len()).collect();
    fn root(parent: &mut [usize], mut i: usize) -> usize {
        while parent[i] != i {
            parent[i] = parent[parent[i]];
            i = parent[i];
        }
        i
    }
    for a in 0..idx.len() {
        for b in a + 1..idx.len() {
            if near(&page.elements[idx[a]].frame, &page.elements[idx[b]].frame) {
                let (ra, rb) = (root(&mut parent, a), root(&mut parent, b));
                parent[ra] = rb;
            }
        }
    }
    let mut clusters: std::collections::BTreeMap<usize, Vec<usize>> = std::collections::BTreeMap::new();
    for k in 0..idx.len() {
        let r = root(&mut parent, k);
        clusters.entry(r).or_default().push(idx[k]);
    }
    let mut groups = 0;
    for mut members in clusters.into_values().filter(|m| m.len() >= FRAGMENT_MIN_COUNT) {
        // La bande sous les carreaux : formes plus grandes, dans le contour de l'amas et collées à lui.
        let frames: Vec<Frame> = members.iter().map(|&i| page.elements[i].frame).collect();
        let (x0, y0) = (frames.iter().map(|f| f.x_mm).fold(f64::MAX, f64::min), frames.iter().map(|f| f.y_mm).fold(f64::MAX, f64::min));
        let (x1, y1) = (frames.iter().map(|f| f.x_mm + f.w_mm).fold(f64::MIN, f64::max), frames.iter().map(|f| f.y_mm + f.h_mm).fold(f64::MIN, f64::max));
        for (i, e) in page.elements.iter().enumerate() {
            let f = &e.frame;
            let inside = f.x_mm >= x0 - FRAGMENT_GAP_MM && f.y_mm >= y0 - FRAGMENT_GAP_MM && f.x_mm + f.w_mm <= x1 + FRAGMENT_GAP_MM && f.y_mm + f.h_mm <= y1 + FRAGMENT_GAP_MM;
            let free = matches!(e.kind, ElementKind::Shape(_)) && e.group.is_none() && !e.locked && !members.contains(&i);
            if free && inside && frames.iter().any(|m| near(m, f)) {
                members.push(i);
            }
        }
        let id = new_id();
        for i in members {
            page.elements[i].group = Some(id.clone());
        }
        groups += 1;
    }
    groups
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::*;

    fn shape(x: f64, y: f64, w: f64, h: f64, kind: ShapeKind, fill: Option<&str>) -> Element {
        Element {
            id: new_id(),
            frame: Frame { x_mm: x, y_mm: y, w_mm: w, h_mm: h, rotation_deg: 0.0 },
            locked: false,
            zone: false,
            group: None,
            kind: ElementKind::Shape(ShapeProps { shape: kind, fill: fill.map(String::from), stroke: None, corner_radius_mm: 0.0 }),
        }
    }

    fn a4() -> Page {
        Page::new(PageSize { width_mm: 210.0, height_mm: 297.0 })
    }

    #[test]
    fn a_white_full_page_rectangle_is_removed() {
        let mut p = a4();
        p.elements.push(shape(0.0, 0.0, 210.0, 297.0, ShapeKind::Rect, Some("#ffffff")));
        p.elements.push(shape(20.0, 20.0, 50.0, 30.0, ShapeKind::Rect, Some("#ffffff")));
        full_page_rect_to_background(&mut p);
        assert_eq!(p.elements.len(), 1);
        assert_eq!(p.elements[0].frame.w_mm, 50.0);
        assert!(p.background.is_none());
    }

    #[test]
    fn a_coloured_full_page_rectangle_becomes_the_page_background() {
        let mut p = a4();
        p.elements.push(shape(-0.3, -0.2, 210.5, 297.4, ShapeKind::Rect, Some("#fff3dd")));
        full_page_rect_to_background(&mut p);
        assert!(p.elements.is_empty());
        assert_eq!(p.background, Some(Background { color: "#fff3dd".into(), pattern: None }));
    }

    #[test]
    fn other_big_shapes_stay() {
        for (e, why) in [
            (shape(0.0, 0.0, 210.0, 297.0, ShapeKind::Ellipse, Some("#ffffff")), "ellipse"),
            (shape(0.0, 0.0, 210.0, 297.0, ShapeKind::Rect, None), "sans remplissage"),
            (shape(10.0, 10.0, 190.0, 277.0, ShapeKind::Rect, Some("#ffffff")), "cadre plus petit"),
        ] {
            let mut p = a4();
            p.elements.push(e);
            full_page_rect_to_background(&mut p);
            assert_eq!(p.elements.len(), 1, "{why}");
        }
        // Rectangle pleine page par-dessus une photo : ce n'est pas un fond.
        let mut p = a4();
        p.elements.push(shape(30.0, 30.0, 40.0, 40.0, ShapeKind::Ellipse, Some("#ff0000")));
        p.elements.push(shape(0.0, 0.0, 210.0, 297.0, ShapeKind::Rect, Some("#ffffff")));
        full_page_rect_to_background(&mut p);
        assert_eq!(p.elements.len(), 2);
    }

    /// Bordure d'un titre : 4 côtés de carreaux de 8 mm, chacun orné de 3 traits.
    fn border(x0: f64, y0: f64) -> Vec<Element> {
        let mut out = Vec::new();
        for i in 0..10 {
            for (x, y) in [(x0 + i as f64 * 8.0, y0), (x0 + i as f64 * 8.0, y0 + 40.0)] {
                out.push(shape(x, y, 8.0, 8.0, ShapeKind::Rect, Some("#73264d")));
                for k in 0..3 {
                    out.push(shape(x + 1.0 + k as f64 * 2.0, y + 2.0, 2.5, 2.0, ShapeKind::Line, None));
                }
            }
        }
        for j in 1..5 {
            for x in [x0, x0 + 72.0] {
                out.push(shape(x, y0 + j as f64 * 8.0, 8.0, 8.0, ShapeKind::Rect, Some("#73264d")));
            }
        }
        out
    }

    #[test]
    fn the_pieces_of_a_decorative_border_become_one_group() {
        let mut p = a4();
        p.elements.push(shape(0.0, 0.0, 210.0, 297.0, ShapeKind::Rect, None));
        let pieces = border(50.0, 10.0);
        let n = pieces.len();
        p.elements.extend(pieces);
        p.elements.push(shape(20.0, 200.0, 40.0, 20.0, ShapeKind::Rect, Some("#2e86de")));
        assert_eq!(group_fragments(&mut p), 1);
        let grouped: Vec<_> = p.elements.iter().filter(|e| e.group.is_some()).collect();
        assert_eq!(grouped.len(), n);
        assert!(grouped.iter().all(|e| e.group == grouped[0].group));
        assert!(p.elements[0].group.is_none() && p.elements.last().unwrap().group.is_none());
    }

    /// Sous les carreaux, la bande de la bordure (quatre longs rectangles) rejoint le groupe ; un texte
    /// ou une forme hors du contour de la bordure, non.
    #[test]
    fn the_band_under_a_border_joins_its_group() {
        let mut p = a4();
        let band = [(50.0, 10.0, 80.0, 8.0), (50.0, 50.0, 80.0, 8.0), (50.0, 10.0, 8.0, 48.0), (122.0, 10.0, 8.0, 48.0)];
        for (x, y, w, h) in band {
            p.elements.push(shape(x, y, w, h, ShapeKind::Rect, Some("#ffffff")));
        }
        let pieces = border(50.0, 10.0);
        let n = pieces.len();
        p.elements.extend(pieces);
        p.elements.push(shape(60.0, 20.0, 60.0, 25.0, ShapeKind::Rect, Some("#ffeeaa")));
        p.elements.push(shape(40.0, 70.0, 100.0, 4.0, ShapeKind::Rect, Some("#000000")));
        assert_eq!(group_fragments(&mut p), 1);
        assert_eq!(p.elements.iter().filter(|e| e.group.is_some()).count(), n + 4);
        assert!(p.elements[p.elements.len() - 1].group.is_none());
    }

    #[test]
    fn two_separate_borders_make_two_groups() {
        let mut p = a4();
        p.elements.extend(border(20.0, 10.0));
        p.elements.extend(border(20.0, 150.0));
        assert_eq!(group_fragments(&mut p), 2);
    }

    #[test]
    fn a_few_small_shapes_are_left_alone() {
        let mut p = a4();
        for i in 0..5 {
            p.elements.push(shape(20.0 + i as f64 * 8.0, 20.0, 8.0, 8.0, ShapeKind::Star, Some("#f1c40f")));
        }
        assert_eq!(group_fragments(&mut p), 0);
        assert!(p.elements.iter().all(|e| e.group.is_none()));
    }
}
