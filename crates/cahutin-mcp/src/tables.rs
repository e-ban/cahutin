//! Tableaux et « Répéter » : mêmes règles que l'interface (`carnet_core::table`, `ui/src/lib/repeat.ts`).

use carnet_core::{Document, ElementKind, Frame, Id, table};
use serde_json::{Value, json};

use crate::arrange::{block, bounds, check, locate, remap_tables, round};
use carnet_core::{Element, groups, new_id};
use std::collections::HashMap;

/// Espace entre les exemplaires et marge au bord de la page, comme `REPEAT_GAP_MM` et `REPEAT_MARGIN_MM`.
const REPEAT_GAP_MM: f64 = 5.0;
const REPEAT_MARGIN_MM: f64 = 10.0;
/// Au plus, en colonnes comme en lignes, pour « Répéter ».
const MAX_REPEAT: u64 = 20;

fn count(args: &Value, key: &str, max: u64) -> Result<Option<u64>, String> {
    match args.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(v) => match v.as_u64() {
            Some(n) if (1..=max).contains(&n) => Ok(Some(n)),
            _ => Err(format!("« {key} » doit être un entier de 1 à {max}")),
        },
    }
}

fn str_arg<'a>(args: &'a Value, key: &str) -> Result<&'a str, String> {
    args.get(key).and_then(Value::as_str).ok_or(format!("« {key} » manque"))
}

/// Pose un tableau de cases texte vides ; rend son id et ses cases.
pub fn add_table(doc: &mut Document, args: &Value) -> Result<Value, String> {
    let max = table::MAX_LINES as u64;
    let cols = count(args, "cols", max)?.ok_or("« cols » manque")?;
    let rows = count(args, "rows", max)?.ok_or("« rows » manque")?;
    let n = |k: &str| args.get(k).and_then(Value::as_f64).ok_or(format!("« {k} » manque"));
    let frame = Frame { x_mm: n("x_mm")?, y_mm: n("y_mm")?, w_mm: n("w_mm")?, h_mm: n("h_mm")?, rotation_deg: 0.0 };
    if !(frame.w_mm > 0.0 && frame.h_mm > 0.0 && frame.w_mm <= 1000.0 && frame.h_mm <= 1000.0) {
        return Err("taille hors limites".into());
    }
    let index = args.get("page").and_then(Value::as_u64).ok_or("« page » manque")? as usize;
    let page = doc.pages.get_mut(index).ok_or(format!("aucune page {index}"))?;
    let id = table::add_table(page, frame, cols as u8, rows as u8);
    let cells = match &page.elements.iter().find(|e| e.id == id).unwrap().kind {
        ElementKind::Table(t) => t.cells.iter().map(|c| json!({ "element_id": c.element_id, "col": c.col, "row": c.row })).collect::<Vec<_>>(),
        _ => unreachable!(),
    };
    Ok(json!({ "element_id": id, "cells": cells }))
}

/// Lignes, colonnes, fusion, séparation et contenu d'une case.
pub fn table_edit(doc: &mut Document, args: &Value) -> Result<Value, String> {
    let cell = str_arg(args, "cell_id")?.to_string();
    let action = str_arg(args, "action")?;
    let content = if action == "set_content" {
        Some(match str_arg(args, "content")? {
            "text" => table::CellContent::Text,
            "photo" => table::CellContent::Photo,
            "image" => {
                let q = str_arg(args, "query")?;
                table::CellContent::Picto(crate::pictos::search(q).ok_or(format!("aucune image pour « {q} »"))?.0.to_string())
            }
            c => return Err(format!("contenu inconnu : {c} (text, image, photo)")),
        })
    } else {
        None
    };
    let other = if action == "merge" { Some(str_arg(args, "other_cell_id")?.to_string()) } else { None };
    let (p, _) = locate(doc, &cell)?;
    let page = &mut doc.pages[p];
    let r = match action {
        "insert_row_above" => table::insert_row(page, &cell, false),
        "insert_row_below" => table::insert_row(page, &cell, true),
        "insert_col_left" => table::insert_col(page, &cell, false),
        "insert_col_right" => table::insert_col(page, &cell, true),
        "remove_row" => table::remove_row(page, &cell),
        "remove_col" => table::remove_col(page, &cell),
        "merge" => table::merge(page, &cell, &other.unwrap()),
        "split" => table::split(page, &cell),
        "set_content" => table::set_cell_content(page, &cell, content.unwrap()),
        a => return Err(format!("action inconnue : {a} (insert_row_above, insert_row_below, insert_col_left, insert_col_right, remove_row, remove_col, merge, split, set_content)")),
    };
    r.map_err(|e| e.message_fr().to_string())?;
    Ok(json!({ "ok": true }))
}

/// Répète un bloc en grille sur sa page, lui compris en haut à gauche, comme `repeatBlock` : chaque
/// exemplaire forme un groupe et garde ses zones à remplir. `fill_page` en met autant que la page en tient.
pub fn repeat(doc: &mut Document, args: &Value) -> Result<Value, String> {
    let given: Vec<&str> = args.get("element_ids").and_then(Value::as_array).ok_or("« element_ids » manque")?.iter().filter_map(Value::as_str).collect();
    let first = *given.first().ok_or("« element_ids » est vide")?;
    let (p, _) = locate(doc, first)?;
    for id in &given {
        if locate(doc, id)?.0 != p {
            return Err("les éléments à répéter doivent être sur la même page".into());
        }
    }
    let gap = args.get("gap_mm").and_then(Value::as_f64).unwrap_or(REPEAT_GAP_MM).clamp(0.0, 50.0);
    let page = &mut doc.pages[p];
    let mut members: Vec<Id> = Vec::new();
    for id in &given {
        if !members.iter().any(|m| m == id) {
            members.extend(block(page, id));
        }
    }
    check(page, &members, true)?;
    let originals: Vec<&Element> = page.elements.iter().filter(|e| members.contains(&e.id)).collect();
    let (x0, y0, x1, y1) = bounds(&originals);
    let (w, h) = (x1 - x0, y1 - y0);
    let (cols, rows) = if args.get("fill_page").and_then(Value::as_bool) == Some(true) {
        let fit = |start: f64, size: f64, limit: f64| ((limit - REPEAT_MARGIN_MM - start + gap) / (size + gap)).floor().clamp(1.0, MAX_REPEAT as f64) as u64;
        (fit(x0, w, page.size.width_mm), fit(y0, h, page.size.height_mm))
    } else {
        (count(args, "cols", MAX_REPEAT)?.ok_or("donnez « cols » et « rows », ou « fill_page »")?, count(args, "rows", MAX_REPEAT)?.ok_or("donnez « cols » et « rows », ou « fill_page »")?)
    };
    // Plusieurs éléments sans groupe commun : l'original devient un groupe, comme ses copies.
    let shared = originals.iter().all(|e| e.group.is_some() && e.group == originals[0].group);
    let own = (originals.len() > 1 && !shared).then(new_id);
    if let Some(g) = &own {
        for e in page.elements.iter_mut().filter(|e| members.contains(&e.id)) {
            let path = std::iter::once(g.clone()).chain(groups::path(e)).collect();
            groups::set_path(e, path);
        }
    }
    let originals: Vec<Element> = page.elements.iter().filter(|e| members.contains(&e.id)).cloned().collect();
    let mut all: Vec<Id> = members.clone();
    for r in 0..rows {
        for c in 0..cols {
            if r == 0 && c == 0 {
                continue;
            }
            let mut regroup: HashMap<Id, Id> = HashMap::new();
            let map: HashMap<Id, Id> = originals.iter().map(|e| (e.id.clone(), new_id())).collect();
            let copies: Vec<Element> = originals
                .iter()
                .map(|e| {
                    let mut k = e.clone();
                    k.id = map[&e.id].clone();
                    k.locked = false;
                    k.frame.x_mm = round(e.frame.x_mm + c as f64 * (w + gap));
                    k.frame.y_mm = round(e.frame.y_mm + r as f64 * (h + gap));
                    let path = groups::path(e).into_iter().map(|g| regroup.entry(g).or_insert_with(new_id).clone()).collect();
                    groups::set_path(&mut k, path);
                    k
                })
                .collect();
            let copies = remap_tables(copies, &map);
            all.extend(copies.iter().map(|k| k.id.clone()));
            page.elements.extend(copies);
        }
    }
    Ok(json!({ "element_ids": all, "cols": cols, "rows": rows }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::edit;
    use carnet_core::*;
    use serde_json::json;

    fn el<'a>(d: &'a Document, id: &str) -> &'a Element {
        d.pages.iter().flat_map(|p| &p.elements).find(|e| e.id == id).unwrap()
    }

    fn props<'a>(d: &'a Document, id: &str) -> &'a TableProps {
        match &el(d, id).kind {
            ElementKind::Table(t) => t,
            _ => unreachable!(),
        }
    }

    /// Un tableau 3 × 2 ; rend son id et ses cases par (colonne, ligne).
    fn with_table() -> (Document, String, Vec<(u64, u64, String)>) {
        let mut d = Document::new("x");
        let out = add_table(&mut d, &json!({ "page": 0, "x_mm": 10, "y_mm": 10, "w_mm": 150, "h_mm": 40, "cols": 3, "rows": 2 })).unwrap();
        let cells = out["cells"].as_array().unwrap().iter().map(|c| (c["col"].as_u64().unwrap(), c["row"].as_u64().unwrap(), c["element_id"].as_str().unwrap().to_string())).collect();
        (d, out["element_id"].as_str().unwrap().to_string(), cells)
    }

    #[test]
    fn a_table_is_added_with_its_cells() {
        let (d, id, cells) = with_table();
        assert_eq!((props(&d, &id).cols.len(), props(&d, &id).rows.len(), cells.len()), (3, 2, 6));
        assert!(matches!(el(&d, &cells[0].2).kind, ElementKind::Text(_)));
        let mut d = d;
        assert!(add_table(&mut d, &json!({ "page": 0, "x_mm": 1, "y_mm": 1, "w_mm": 9, "h_mm": 9, "cols": 0, "rows": 2 })).is_err());
    }

    #[test]
    fn rows_and_columns_come_and_go() {
        let (mut d, id, cells) = with_table();
        table_edit(&mut d, &json!({ "cell_id": cells[0].2, "action": "insert_row_below" })).unwrap();
        table_edit(&mut d, &json!({ "cell_id": cells[0].2, "action": "insert_col_right" })).unwrap();
        assert_eq!((props(&d, &id).cols.len(), props(&d, &id).rows.len()), (4, 3));
        table_edit(&mut d, &json!({ "cell_id": cells[0].2, "action": "remove_row" })).unwrap();
        table_edit(&mut d, &json!({ "cell_id": cells[0].2, "action": "remove_col" })).unwrap_err();
        assert_eq!(props(&d, &id).rows.len(), 2);
    }

    #[test]
    fn merge_split_and_cell_content() {
        let (mut d, id, cells) = with_table();
        table_edit(&mut d, &json!({ "cell_id": cells[0].2, "action": "merge", "other_cell_id": cells[1].2 })).unwrap();
        assert_eq!(props(&d, &id).cells.len(), 5);
        table_edit(&mut d, &json!({ "cell_id": cells[0].2, "action": "split" })).unwrap();
        assert_eq!(props(&d, &id).cells.len(), 6);
        table_edit(&mut d, &json!({ "cell_id": cells[2].2, "action": "set_content", "content": "image", "query": "soleil" })).unwrap();
        assert!(matches!(el(&d, &cells[2].2).kind, ElementKind::Image(_)));
        table_edit(&mut d, &json!({ "cell_id": cells[3].2, "action": "set_content", "content": "photo" })).unwrap();
        assert!(matches!(el(&d, &cells[3].2).kind, ElementKind::Photo(_)));
        // Un message en français, celui de l'application.
        let err = table_edit(&mut d, &json!({ "cell_id": cells[0].2, "action": "split" })).unwrap_err();
        assert_eq!(err, table::TableError::NotSpanned.message_fr());
        assert!(table_edit(&mut d, &json!({ "cell_id": cells[0].2, "action": "merge" })).unwrap_err().contains("other_cell_id"));
    }

    #[test]
    fn repeat_fills_a_grid_of_grouped_copies() {
        let mut d = Document::new("x");
        let a = edit::add_shape(&mut d, &json!({ "page": 0, "x_mm": 10, "y_mm": 10, "w_mm": 60, "h_mm": 30, "shape": "rect" })).unwrap()["element_id"].as_str().unwrap().to_string();
        let b = edit::add_text(&mut d, &json!({ "page": 0, "x_mm": 15, "y_mm": 15, "w_mm": 50, "h_mm": 20, "content": "x" })).unwrap()["element_id"].as_str().unwrap().to_string();
        let out = repeat(&mut d, &json!({ "element_ids": [a, b], "cols": 2, "rows": 3 })).unwrap();
        assert_eq!(out["element_ids"].as_array().unwrap().len(), 12);
        assert_eq!(d.pages[0].elements.len(), 12);
        // L'original devient un groupe, chaque copie le sien ; la copie de droite est à 60 + 5 mm.
        let g = el(&d, &a).group.clone().unwrap();
        assert_eq!(el(&d, &b).group, Some(g.clone()));
        let groups: std::collections::HashSet<_> = d.pages[0].elements.iter().map(|e| e.group.clone().unwrap()).collect();
        assert_eq!(groups.len(), 6);
        assert!(d.pages[0].elements.iter().any(|e| e.frame.x_mm == 75.0 && e.frame.y_mm == 10.0));
        assert!(d.pages[0].elements.iter().any(|e| e.frame.x_mm == 10.0 && e.frame.y_mm == 80.0));
    }

    #[test]
    fn repeat_can_fill_the_page_and_copies_tables_with_their_cells() {
        let (mut d, id, _) = with_table();
        repeat(&mut d, &json!({ "element_ids": [id], "fill_page": true })).unwrap();
        // 10 + n × (40 + 5) − 5 ≤ 287 : 6 exemplaires en hauteur, 1 en largeur.
        let tables: Vec<&Element> = d.pages[0].elements.iter().filter(|e| matches!(e.kind, ElementKind::Table(_))).collect();
        assert_eq!(tables.len(), 6);
        for t in tables {
            let ElementKind::Table(p) = &t.kind else { unreachable!() };
            assert!(p.cells.iter().all(|c| el(&d, &c.element_id).group == t.group), "les cases suivent leur tableau");
        }
        assert!(repeat(&mut d, &json!({ "element_ids": [id] })).unwrap_err().contains("cols"));
    }

    #[test]
    fn a_table_can_be_duplicated() {
        let (mut d, id, _) = with_table();
        let out = crate::arrange::arrange(&mut d, &json!({ "element_id": id, "action": "duplicate" })).unwrap();
        let copy = out["element_ids"].as_array().unwrap().iter().map(|v| v.as_str().unwrap()).find(|i| matches!(el(&d, i).kind, ElementKind::Table(_))).unwrap().to_string();
        let ElementKind::Table(p) = &el(&d, &copy).kind else { unreachable!() };
        assert!(p.cells.iter().all(|c| el(&d, &c.element_id).group == el(&d, &copy).group));
    }

    #[test]
    fn deleting_a_table_takes_its_cells() {
        let (mut d, id, _) = with_table();
        edit::delete_element(&mut d, &json!({ "element_id": id })).unwrap();
        assert!(d.pages[0].elements.is_empty());
    }
}
