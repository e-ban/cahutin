//! Défaire une étape d'une proposition en gardant les autres. Chaque étape est un document entier ; défaire
//! l'étape `k` remet, dans chacun des documents suivants, ce qu'elle a changé (éléments, réglages de page,
//! pages ajoutées) tel qu'il était avant elle. Une étape suivante qui a touché la même chose en dépend :
//! l'étape ne se défait alors pas seule.

use carnet_core::{Document, Element, Id, Page};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use ts_rs::TS;

/// Pourquoi une étape ne se défait pas seule.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, TS)]
#[ts(export, rename = "AssistantUndoBlocked")]
pub struct Blocked {
    /// Indice, parmi les étapes suivantes, de la première qui dépend de celle-ci ; aucun : l'étape change
    /// le document entier (pages retirées, styles nommés), et seule la dernière étape se défait.
    pub step: Option<usize>,
}

/// Ce qu'une étape a changé, ou `None` si elle a changé le document entier.
fn changes(a: &Document, b: &Document) -> Option<HashSet<String>> {
    if b.pages.len() < a.pages.len() || a.title != b.title || a.pending_photos != b.pending_photos || a.text_styles != b.text_styles {
        return None;
    }
    let mut out = HashSet::new();
    for (i, pb) in b.pages.iter().enumerate() {
        let Some(pa) = a.pages.get(i) else {
            // Une page ajoutée : tout ce qui s'y trouve en dépend.
            out.insert(format!("page:{i}"));
            out.insert(format!("content:{i}"));
            continue;
        };
        if pa.id != pb.id || pa.size != pb.size || pa.grid != pb.grid || pa.background != pb.background {
            out.insert(format!("page:{i}"));
        }
        let ids_a: Vec<&Id> = pa.elements.iter().map(|e| &e.id).collect();
        let ids_b: Vec<&Id> = pb.elements.iter().map(|e| &e.id).collect();
        let reordered = {
            let common_a: Vec<&&Id> = ids_a.iter().filter(|id| ids_b.contains(id)).collect();
            let common_b: Vec<&&Id> = ids_b.iter().filter(|id| ids_a.contains(id)).collect();
            common_a != common_b
        };
        let mut element_changed = false;
        for e in &pb.elements {
            let same = pa.elements.iter().find(|x| x.id == e.id) == Some(e);
            let moved = reordered && {
                // Une seule place qui change suffit à marquer l'élément : comparer les voisins communs.
                let before_pos = pa.elements.iter().position(|x| x.id == e.id);
                let after_pos = pb.elements.iter().position(|x| x.id == e.id);
                let neighbours = |els: &Vec<Element>, pos: Option<usize>, other: &[&Id]| {
                    pos.map(|p| els[..p].iter().filter(|x| other.contains(&&x.id)).map(|x| x.id.clone()).collect::<Vec<_>>())
                };
                neighbours(&pa.elements, before_pos, &ids_b) != neighbours(&pb.elements, after_pos, &ids_a)
            };
            if !same || moved {
                out.insert(e.id.clone());
                element_changed = true;
            }
        }
        for e in pa.elements.iter().filter(|e| !ids_b.contains(&&e.id)) {
            out.insert(e.id.clone());
            element_changed = true;
        }
        if element_changed {
            out.insert(format!("content:{i}"));
        }
    }
    Some(out)
}

/// Les éléments de `changed` remis dans `page` tels qu'ils sont dans `before` (à leur place parmi les autres),
/// ou retirés s'ils n'y étaient pas.
fn restore_elements(page: &mut Page, before: Option<&Page>, changed: &HashSet<String>) {
    page.elements.retain(|e| !changed.contains(&e.id));
    let Some(before) = before else { return };
    for (i, e) in before.elements.iter().enumerate() {
        if !changed.contains(&e.id) {
            continue;
        }
        // Juste après le plus proche élément qui le précédait et qui est encore là.
        let at = before.elements[..i].iter().rev().find_map(|p| page.elements.iter().position(|x| x.id == p.id)).map_or(0, |p| p + 1);
        page.elements.insert(at, e.clone());
    }
}

/// `doc` sans ce que l'étape (`before` → `after`) a changé.
fn revert(doc: &Document, before: &Document, after: &Document, changed: &HashSet<String>) -> Document {
    let mut out = doc.clone();
    // Les pages que l'étape a ajoutées, à la fin du document : personne d'autre ne les a touchées.
    for i in (before.pages.len()..after.pages.len()).rev() {
        if i < out.pages.len() {
            out.pages.remove(i);
        }
    }
    for (i, page) in out.pages.iter_mut().enumerate() {
        let old = before.pages.get(i);
        if changed.contains(&format!("page:{i}"))
            && let Some(old) = old
        {
            page.size = old.size;
            page.grid = old.grid.clone();
            page.background = old.background.clone();
        }
        // Chaque page retire les éléments changés : un élément que l'étape avait mis sur une autre page n'y reste pas.
        restore_elements(page, old, changed);
    }
    out
}

/// Les documents des étapes suivantes (`rest`), refaits sans l'étape `before` → `after`.
pub fn undo_step(before: &Document, after: &Document, rest: &[Document]) -> Result<Vec<Document>, Blocked> {
    let Some(mut changed) = changes(before, after) else {
        return if rest.is_empty() { Ok(vec![]) } else { Err(Blocked { step: None }) };
    };
    // « content:i » ne compte que pour une page que l'étape a ajoutée : ailleurs, deux étapes qui touchent
    // des éléments différents d'une même page ne dépendent pas l'une de l'autre.
    changed.retain(|c| c.strip_prefix("content:").and_then(|i| i.parse::<usize>().ok()).is_none_or(|i| i >= before.pages.len()));
    let mut previous = after;
    for (j, doc) in rest.iter().enumerate() {
        let later = changes(previous, doc);
        if later.is_none_or(|c| !c.is_disjoint(&changed)) {
            return Err(Blocked { step: Some(j) });
        }
        previous = doc;
    }
    Ok(rest.iter().map(|d| revert(d, before, after, &changed)).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools;
    use serde_json::{Value, json};

    fn doc() -> Document {
        let mut d = Document::new("essai");
        crate::edit::add_text(&mut d, &json!({ "page": 0, "x_mm": 10, "y_mm": 10, "w_mm": 80, "h_mm": 20, "content": "Titre" })).unwrap();
        crate::edit::add_shape(&mut d, &json!({ "page": 0, "x_mm": 10, "y_mm": 50, "w_mm": 30, "h_mm": 30, "shape": "star" })).unwrap();
        d
    }

    fn id(d: &Document, i: usize) -> String {
        d.pages[0].elements[i].id.clone()
    }

    /// Joue les outils l'un après l'autre ; rend le document après chacun.
    fn steps(start: &Document, calls: &[(&str, Value)]) -> Vec<Document> {
        let mut d = start.clone();
        calls
            .iter()
            .map(|(name, args)| {
                tools::apply(&mut d, name, args, &[]).unwrap();
                d.clone()
            })
            .collect()
    }

    #[test]
    fn an_independent_step_is_undone_and_the_others_stay() {
        let base = doc();
        let (title, star) = (id(&base, 0), id(&base, 1));
        let s = steps(&base, &[
            ("set_text_style", json!({ "element_id": title, "color": "#1b2a6b" })),
            ("set_frame", json!({ "element_id": star, "x_mm": 120 })),
            ("set_background", json!({ "page": 0, "pattern": "seyes" })),
        ]);
        // Défaire la première : le titre reprend sa couleur, l'étoile reste déplacée, le fond reste.
        let rest = undo_step(&base, &s[0], &s[1..]).unwrap();
        assert_eq!(rest.len(), 2);
        let last = &rest[1];
        assert_eq!(last.pages[0].elements[0], base.pages[0].elements[0]);
        assert_eq!(last.pages[0].elements[1].frame.x_mm, 120.0);
        assert!(last.pages[0].background.is_some());
        // Défaire celle du milieu.
        let rest = undo_step(&s[0], &s[1], &s[2..]).unwrap();
        assert_eq!(rest[0].pages[0].elements[1], base.pages[0].elements[1]);
        assert_eq!(rest[0].pages[0].elements[0], s[0].pages[0].elements[0]);
        // Défaire la dernière : rien après elle.
        assert_eq!(undo_step(&s[1], &s[2], &[]), Ok(vec![]));
    }

    #[test]
    fn a_step_a_later_one_builds_on_is_not_undone_alone() {
        let base = doc();
        let title = id(&base, 0);
        let s = steps(&base, &[
            ("set_text_style", json!({ "element_id": title, "color": "#1b2a6b" })),
            ("set_frame", json!({ "element_id": title, "y_mm": 40 })),
        ]);
        assert_eq!(undo_step(&base, &s[0], &s[1..]), Err(Blocked { step: Some(0) }));
    }

    #[test]
    fn an_added_element_goes_away_unless_a_later_step_uses_it() {
        let base = doc();
        let mut d = base.clone();
        let added = tools::apply(&mut d, "add_text", &json!({ "page": 0, "x_mm": 10, "y_mm": 100, "w_mm": 80, "h_mm": 20, "content": "Consigne" }), &[]).unwrap();
        let new_id = added["element_id"].as_str().unwrap().to_string();
        let mut later_on_it = d.clone();
        tools::apply(&mut later_on_it, "set_text_style", &json!({ "element_id": new_id, "color": "#ff0000" }), &[]).unwrap();
        assert_eq!(undo_step(&base, &d, std::slice::from_ref(&later_on_it)), Err(Blocked { step: Some(0) }));
        let mut later_elsewhere = d.clone();
        tools::apply(&mut later_elsewhere, "set_background", &json!({ "page": 0, "color": "#fff8e0" }), &[]).unwrap();
        let rest = undo_step(&base, &d, &[later_elsewhere]).unwrap();
        assert_eq!(rest[0].pages[0].elements, base.pages[0].elements);
        assert_eq!(rest[0].pages[0].background.as_ref().unwrap().color, "#fff8e0");
    }

    #[test]
    fn a_deleted_element_comes_back_in_its_place() {
        let base = doc();
        let title = id(&base, 0);
        let s = steps(&base, &[("delete_element", json!({ "element_id": title })), ("set_background", json!({ "page": 0, "pattern": "lines" }))]);
        let rest = undo_step(&base, &s[0], &s[1..]).unwrap();
        assert_eq!(rest[0].pages[0].elements, base.pages[0].elements, "le titre revient en premier, sous l'étoile");
    }

    #[test]
    fn a_change_of_order_is_undone() {
        let base = doc();
        let title = id(&base, 0);
        let s = steps(&base, &[("arrange", json!({ "element_id": title, "action": "front" })), ("set_background", json!({ "page": 0, "pattern": "dots" }))]);
        assert_ne!(s[0].pages[0].elements, base.pages[0].elements);
        let rest = undo_step(&base, &s[0], &s[1..]).unwrap();
        assert_eq!(rest[0].pages[0].elements, base.pages[0].elements);
    }

    #[test]
    fn an_added_page_goes_away_unless_something_was_put_on_it() {
        let base = doc();
        let star = id(&base, 1);
        let s = steps(&base, &[("add_page", json!({})), ("set_frame", json!({ "element_id": star, "x_mm": 100 }))]);
        let rest = undo_step(&base, &s[0], &s[1..]).unwrap();
        assert_eq!(rest[0].pages.len(), 1);
        assert_eq!(rest[0].pages[0].elements[1].frame.x_mm, 100.0);
        let s = steps(&base, &[("add_page", json!({})), ("add_shape", json!({ "page": 1, "x_mm": 10, "y_mm": 10, "w_mm": 20, "h_mm": 20, "shape": "heart" }))]);
        assert_eq!(undo_step(&base, &s[0], &s[1..]), Err(Blocked { step: Some(0) }));
    }

    #[test]
    fn a_step_that_changes_the_whole_document_is_only_undone_last() {
        let base = doc();
        let mut renamed = base.clone();
        renamed.title = "autre".into();
        assert_eq!(undo_step(&base, &renamed, std::slice::from_ref(&renamed)), Err(Blocked { step: None }));
        assert_eq!(undo_step(&base, &renamed, &[]), Ok(vec![]));
    }

    #[test]
    fn a_step_that_changed_nothing_is_dropped_without_touching_the_rest() {
        // L'enregistrement d'un modèle ne change pas le document.
        let base = doc();
        let s = steps(&base, &[("set_background", json!({ "page": 0, "pattern": "seyes" }))]);
        assert_eq!(undo_step(&base, &base, &s), Ok(s.clone()));
    }
}
