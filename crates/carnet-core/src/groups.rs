//! Groupes d'éléments : un identifiant partagé par les membres, sur une même page.

use crate::model::{ElementKind, Page};
use std::collections::HashMap;

/// Un groupe a au moins deux membres : les autres perdent leur identifiant de groupe. Un tableau garde
/// toujours le sien.
pub fn dissolve_small_groups(page: &mut Page) {
    let mut counts: HashMap<String, usize> = HashMap::new();
    for g in page.elements.iter().filter_map(|e| e.group.clone()) {
        *counts.entry(g).or_default() += 1;
    }
    for e in &page.elements {
        if let (Some(g), ElementKind::Table(_)) = (&e.group, &e.kind) {
            counts.insert(g.clone(), usize::MAX);
        }
    }
    for e in &mut page.elements {
        if e.group.as_ref().is_some_and(|g| counts[g] < 2) {
            e.group = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::*;

    fn el(group: Option<&str>) -> Element {
        Element {
            id: new_id(),
            frame: Frame { x_mm: 0.0, y_mm: 0.0, w_mm: 10.0, h_mm: 10.0, rotation_deg: 0.0 },
            locked: false,
            zone: false,
            group: group.map(String::from),
            kind: ElementKind::Shape(ShapeProps { shape: ShapeKind::Rect, fill: None, stroke: None, corner_radius_mm: 0.0 }),
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
