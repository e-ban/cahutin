//! Groupes d'éléments : un identifiant partagé par les membres, sur une même page. Un groupe peut en contenir
//! d'autres : le chemin d'un élément est son groupe puis ses sous-groupes, du plus large au plus petit.

use crate::model::{Element, ElementKind, Id, Page};
use std::collections::{HashMap, HashSet};

/// Groupe puis sous-groupes de l'élément ; vide s'il n'est pas groupé.
pub fn path(e: &Element) -> Vec<Id> {
    e.group.iter().chain(if e.group.is_some() { e.subgroups.iter() } else { [].iter() }).cloned().collect()
}

pub fn set_path(e: &mut Element, mut path: Vec<Id>) {
    e.subgroups = if path.len() > 1 { path.split_off(1) } else { Vec::new() };
    e.group = path.pop();
}

/// Remet les groupes d'aplomb : un groupe garde la même place dans chaque chemin (sinon le chemin s'arrête
/// avant), et un groupe d'un seul membre (élément ou sous-groupe) disparaît, à chaque niveau. Le groupe d'un
/// tableau reste toujours.
pub fn dissolve_small_groups(page: &mut Page) {
    let mut paths: Vec<Vec<Id>> = page.elements.iter().map(path).collect();
    // Un même groupe sous deux parents différents, ou deux fois dans un chemin : le premier vu l'emporte.
    let mut parent: HashMap<Id, Vec<Id>> = HashMap::new();
    for p in &mut paths {
        let keep = (0..p.len()).find(|&d| p[..d].contains(&p[d]) || parent.get(&p[d]).is_some_and(|pre| pre[..] != p[..d])).unwrap_or(p.len());
        p.truncate(keep);
        for d in 0..p.len() {
            parent.entry(p[d].clone()).or_insert_with(|| p[..d].to_vec());
        }
    }
    let tables: HashSet<Id> = page.elements.iter().filter(|e| matches!(e.kind, ElementKind::Table(_))).filter_map(|e| e.group.clone()).collect();
    loop {
        // Membres directs de chaque groupe : le sous-groupe suivant, ou l'élément lui-même.
        let mut members: HashMap<&Id, HashSet<&Id>> = HashMap::new();
        for (p, e) in paths.iter().zip(&page.elements) {
            for d in 0..p.len() {
                members.entry(&p[d]).or_default().insert(p.get(d + 1).unwrap_or(&e.id));
            }
        }
        let lone: HashSet<Id> = members.into_iter().filter(|(g, m)| m.len() < 2 && !tables.contains(*g)).map(|(g, _)| g.clone()).collect();
        if lone.is_empty() {
            break;
        }
        for p in &mut paths {
            p.retain(|g| !lone.contains(g));
        }
    }
    for (e, p) in page.elements.iter_mut().zip(paths) {
        set_path(e, p);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::*;

    fn nested(path: &[&str]) -> Element {
        let mut e = el(None);
        set_path(&mut e, path.iter().map(|g| g.to_string()).collect());
        e
    }

    fn paths(page: &Page) -> Vec<Vec<Id>> {
        page.elements.iter().map(path).collect()
    }

    #[test]
    fn nested_groups_keep_their_levels_and_lose_the_lonely_ones() {
        let mut page = Page::new(A4_PORTRAIT);
        // Une frise (f) et une photo avec sa légende (p) dans un même groupe (g) ; un groupe (s) d'un seul
        // élément dans la frise ; un groupe (h) qui n'a plus qu'un sous-groupe.
        page.elements = vec![nested(&["g", "f"]), nested(&["g", "f", "s"]), nested(&["g", "p"]), nested(&["g", "p"]), nested(&["h", "k"]), nested(&["h", "k"])];
        dissolve_small_groups(&mut page);
        let s = |p: &[&str]| p.iter().map(|g| g.to_string()).collect::<Vec<_>>();
        assert_eq!(paths(&page), [s(&["g", "f"]), s(&["g", "f"]), s(&["g", "p"]), s(&["g", "p"]), s(&["k"]), s(&["k"])]);
        assert!(page.elements[4].subgroups.is_empty() && page.elements[4].group.as_deref() == Some("k"));
    }

    #[test]
    fn a_group_found_under_two_parents_is_cut() {
        let mut page = Page::new(A4_PORTRAIT);
        page.elements = vec![nested(&["a", "x"]), nested(&["a", "x"]), nested(&["a"]), nested(&["b", "x"]), nested(&["b"]), nested(&["b"])];
        dissolve_small_groups(&mut page);
        let s = |p: &[&str]| p.iter().map(|g| g.to_string()).collect::<Vec<_>>();
        assert_eq!(paths(&page), [s(&["a", "x"]), s(&["a", "x"]), s(&["a"]), s(&["b"]), s(&["b"]), s(&["b"])]);
    }

    fn el(group: Option<&str>) -> Element {
        Element {
            id: new_id(),
            frame: Frame { x_mm: 0.0, y_mm: 0.0, w_mm: 10.0, h_mm: 10.0, rotation_deg: 0.0 },
            locked: false,
            zone: false,
            group: group.map(String::from),
            kind: ElementKind::Shape(ShapeProps { shape: ShapeKind::Rect, fill: None, stroke: None, corner_radius_mm: 0.0, path: None, graphisme: None }),
            subgroups: Vec::new(),
            flip_h: false, flip_v: false,
        }
    }

    #[test]
    fn dissolve_small_groups_keeps_pairs() {
        let mut page = Page::new(A4_PORTRAIT);
        page.elements = vec![el(Some("a")), el(Some("b")), el(Some("b")), el(Some("c")), el(Some("c")), el(Some("c")), el(None)];
        dissolve_small_groups(&mut page);
        let groups: Vec<Option<&str>> = page.elements.iter().map(|e| e.group.as_deref()).collect();
        assert_eq!(groups, [None, Some("b"), Some("b"), Some("c"), Some("c"), Some("c"), None]);
    }

    #[test]
    fn dissolve_small_groups_keeps_tables() {
        let mut page = Page::new(A4_PORTRAIT);
        let mut table = el(Some("t"));
        table.kind = ElementKind::Table(TableProps { cols: vec![1.0], rows: vec![1.0], cells: vec![], border: None });
        page.elements = vec![table];
        dissolve_small_groups(&mut page);
        assert_eq!(page.elements[0].group.as_deref(), Some("t"));
    }
}
