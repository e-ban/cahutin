//! Groupes d'éléments : un identifiant partagé par les membres, sur une même page.

use crate::model::Page;
use std::collections::HashMap;

/// Un groupe a au moins deux membres : les autres perdent leur identifiant de groupe.
pub fn dissolve_small_groups(page: &mut Page) {
    let mut counts: HashMap<String, usize> = HashMap::new();
    for g in page.elements.iter().filter_map(|e| e.group.clone()) {
        *counts.entry(g).or_default() += 1;
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
}
