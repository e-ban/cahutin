//! Arranger la page : devant ou derrière, dupliquer, miroir, grouper et dissocier, format de la page.
//! Mêmes règles que l'interface (`ui/src/lib/groups.ts`, `clipboard.ts`, `pageFormat.ts`).

use carnet_core::{Document, Element, ElementKind, Id, Page, PageSize, groups, new_id};
use serde_json::{Value, json};
use std::collections::HashMap;

/// Décalage d'une copie, comme un collage dans l'interface (`PASTE_OFFSET_MM`).
const PASTE_OFFSET_MM: f64 = 5.0;

fn text<'a>(args: &'a Value, key: &str) -> Result<&'a str, String> {
    args.get(key).and_then(Value::as_str).ok_or(format!("« {key} » manque"))
}

/// Page et rang de l'élément.
fn locate(doc: &Document, id: &str) -> Result<(usize, usize), String> {
    doc.pages
        .iter()
        .enumerate()
        .find_map(|(p, page)| page.elements.iter().position(|e| e.id == id).map(|i| (p, i)))
        .ok_or(format!("aucun élément « {id} »"))
}

/// L'élément et tout son groupe le plus large, dans l'ordre de la page.
fn block(page: &Page, id: &str) -> Vec<Id> {
    let group = page.elements.iter().find(|e| e.id == id).and_then(|e| e.group.clone());
    page.elements.iter().filter(|e| e.id == id || (group.is_some() && e.group == group)).map(|e| e.id.clone()).collect()
}

/// Refuse un élément verrouillé, une case de grille et, sauf `tables`, un tableau ou ses cases.
fn check(page: &Page, ids: &[Id], tables: bool) -> Result<(), String> {
    let table_groups: Vec<&Id> = page.elements.iter().filter(|e| matches!(e.kind, ElementKind::Table(_))).filter_map(|e| e.group.as_ref()).collect();
    for e in page.elements.iter().filter(|e| ids.contains(&e.id)) {
        if e.locked {
            return Err(format!("l'élément « {} » est verrouillé par l'utilisateur", e.id));
        }
        if page.grid.as_ref().is_some_and(|g| g.cells.iter().any(|c| c.element_id == e.id)) {
            return Err(format!("« {} » est une case de la grille de la page : elle reste à sa place", e.id));
        }
        let in_table = matches!(e.kind, ElementKind::Table(_)) || e.group.as_ref().is_some_and(|g| table_groups.contains(&g));
        if in_table && !tables {
            return Err(format!("« {} » fait partie d'un tableau : cette action ne s'applique pas à un tableau", e.id));
        }
    }
    Ok(())
}

fn round(v: f64) -> f64 {
    (v * 10.0).round() / 10.0
}

/// Retourné dans son cadre, comme `flipped` : un texte ou un tableau ne se retourne pas, une photo garde
/// son propre miroir de gauche à droite.
fn flip(e: &mut Element, horizontal: bool) {
    match &mut e.kind {
        ElementKind::Text(_) | ElementKind::Table(_) => {}
        ElementKind::Photo(p) if horizontal => p.mirrored = !p.mirrored,
        _ if horizontal => e.flip_h = !e.flip_h,
        _ => e.flip_v = !e.flip_v,
    }
}

/// Rectangle qui contient les cadres, coins tournés compris.
fn bounds(els: &[&Element]) -> (f64, f64, f64, f64) {
    let (mut x0, mut y0, mut x1, mut y1) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
    for e in els {
        let f = &e.frame;
        let (cx, cy) = (f.x_mm + f.w_mm / 2.0, f.y_mm + f.h_mm / 2.0);
        let (sin, cos) = f.rotation_deg.to_radians().sin_cos();
        for (dx, dy) in [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)] {
            let (px, py) = (dx * f.w_mm / 2.0, dy * f.h_mm / 2.0);
            let (x, y) = (cx + px * cos - py * sin, cy + px * sin + py * cos);
            (x0, y0, x1, y1) = (x0.min(x), y0.min(y), x1.max(x), y1.max(y));
        }
    }
    (x0, y0, x1, y1)
}

/// Devant, derrière, dupliquer ou miroir d'un élément, avec tout son groupe.
pub fn arrange(doc: &mut Document, args: &Value) -> Result<Value, String> {
    let id = text(args, "element_id")?;
    let action = text(args, "action")?;
    let (p, _) = locate(doc, id)?;
    let page = &mut doc.pages[p];
    let ids = block(page, id);
    match action {
        "front" | "back" => {
            check(page, &ids, true)?;
            let (moved, rest): (Vec<Element>, Vec<Element>) = page.elements.drain(..).partition(|e| ids.contains(&e.id));
            page.elements = if action == "front" { rest.into_iter().chain(moved).collect() } else { moved.into_iter().chain(rest).collect() };
            Ok(json!({ "ok": true }))
        }
        "duplicate" => {
            check(page, &ids, false)?;
            // Chaque copie a ses propres groupes, comme un collage.
            let mut regroup: HashMap<Id, Id> = HashMap::new();
            let copies: Vec<Element> = page
                .elements
                .iter()
                .filter(|e| ids.contains(&e.id))
                .map(|e| {
                    let mut c = e.clone();
                    c.id = new_id();
                    (c.zone, c.locked) = (false, false);
                    c.frame.x_mm += PASTE_OFFSET_MM;
                    c.frame.y_mm += PASTE_OFFSET_MM;
                    let path = groups::path(e).into_iter().map(|g| regroup.entry(g).or_insert_with(new_id).clone()).collect();
                    groups::set_path(&mut c, path);
                    c
                })
                .collect();
            let new_ids: Vec<Id> = copies.iter().map(|c| c.id.clone()).collect();
            page.elements.extend(copies);
            Ok(json!({ "element_ids": new_ids }))
        }
        "mirror_h" | "mirror_v" => {
            check(page, &ids, false)?;
            let horizontal = action == "mirror_h";
            if ids.len() == 1 {
                let e = page.elements.iter_mut().find(|e| e.id == ids[0]).unwrap();
                if matches!(e.kind, ElementKind::Text(_)) {
                    return Err("un texte ne se retourne pas : il resterait illisible".into());
                }
                flip(e, horizontal);
                return Ok(json!({ "ok": true }));
            }
            // Un groupe change de côté autour de son centre, comme `mirrorElements`.
            let (x0, y0, x1, y1) = bounds(&page.elements.iter().filter(|e| ids.contains(&e.id)).collect::<Vec<_>>());
            for e in page.elements.iter_mut().filter(|e| ids.contains(&e.id)) {
                let f = &mut e.frame;
                if horizontal {
                    f.x_mm = round(x0 + x1 - f.x_mm - f.w_mm);
                } else {
                    f.y_mm = round(y0 + y1 - f.y_mm - f.h_mm);
                }
                if f.rotation_deg != 0.0 {
                    f.rotation_deg = (-f.rotation_deg).rem_euclid(360.0);
                }
                flip(e, horizontal);
            }
            Ok(json!({ "ok": true }))
        }
        a => Err(format!("action inconnue : {a} (front, back, duplicate, mirror_h, mirror_v)")),
    }
}

/// Groupe les éléments donnés (chacun avec son groupe entier, qui reste un groupe dans le nouveau).
pub fn group(doc: &mut Document, args: &Value) -> Result<Value, String> {
    let given: Vec<&str> = args.get("element_ids").and_then(Value::as_array).ok_or("« element_ids » manque")?.iter().filter_map(Value::as_str).collect();
    let pages: Vec<usize> = given.iter().map(|id| locate(doc, id).map(|l| l.0)).collect::<Result<_, _>>()?;
    let p = *pages.first().ok_or("« element_ids » est vide")?;
    if pages.iter().any(|&q| q != p) {
        return Err("les éléments à grouper doivent être sur la même page".into());
    }
    let page = &mut doc.pages[p];
    let mut members: Vec<Id> = Vec::new();
    let mut blocks = 0;
    for id in &given {
        if members.iter().any(|m| m == id) {
            continue;
        }
        members.extend(block(page, id));
        blocks += 1;
    }
    check(page, &members, false)?;
    if blocks < 2 {
        return Err("il faut au moins deux éléments ou groupes différents pour faire un groupe".into());
    }
    let g = new_id();
    // Tous rassemblés à la place du plus haut, comme `groupElements`.
    let top = page.elements.iter().rposition(|e| members.contains(&e.id)).unwrap();
    let before = page.elements[..top].iter().filter(|e| !members.contains(&e.id)).count();
    let (mut grouped, mut rest): (Vec<Element>, Vec<Element>) = page.elements.drain(..).partition(|e| members.contains(&e.id));
    for e in &mut grouped {
        let path = std::iter::once(g.clone()).chain(groups::path(e)).collect();
        groups::set_path(e, path);
    }
    let after = rest.split_off(before);
    page.elements = rest.into_iter().chain(grouped).chain(after).collect();
    groups::dissolve_small_groups(page);
    Ok(json!({ "group": g }))
}

/// Dissocie le groupe le plus large de l'élément : ses sous-groupes restent.
pub fn ungroup(doc: &mut Document, args: &Value) -> Result<Value, String> {
    let id = text(args, "element_id")?;
    let (p, i) = locate(doc, id)?;
    let page = &mut doc.pages[p];
    if page.elements[i].group.is_none() {
        return Err(format!("« {id} » n'est pas dans un groupe"));
    }
    let ids = block(page, id);
    check(page, &ids, false)?;
    for e in page.elements.iter_mut().filter(|e| ids.contains(&e.id)) {
        let path = groups::path(e).into_iter().skip(1).collect();
        groups::set_path(e, path);
    }
    groups::dissolve_small_groups(page);
    Ok(json!({ "ok": true }))
}

/// Agrandit ou réduit un élément depuis le coin de la page, comme `scaleElements` (tailles de texte au
/// demi-point, le reste au dixième de mm).
fn scale(e: &mut Element, k: f64) {
    let half = |v: f64| (v * 2.0).round() / 2.0;
    let f = &mut e.frame;
    (f.x_mm, f.y_mm, f.w_mm, f.h_mm) = (round(f.x_mm * k), round(f.y_mm * k), round(f.w_mm * k), round(f.h_mm * k));
    match &mut e.kind {
        ElementKind::Text(t) => {
            t.style.size_pt = half(t.style.size_pt * k);
            t.style.padding_mm = t.style.padding_mm.map(|v| round(v * k));
            if let Some(fr) = &mut t.frame {
                fr.corner_radius_mm = round(fr.corner_radius_mm * k);
                if let Some(s) = &mut fr.stroke {
                    s.width_mm = round(s.width_mm * k);
                }
            }
            for r in t.content.paragraphs.iter_mut().flat_map(|p| &mut p.runs) {
                r.size_pt = r.size_pt.map(|v| half(v * k));
            }
        }
        ElementKind::Shape(s) => {
            s.corner_radius_mm = round(s.corner_radius_mm * k);
            if let Some(st) = &mut s.stroke {
                st.width_mm = round(st.width_mm * k);
            }
        }
        ElementKind::Photo(p) => {
            p.corner_radius_mm = round(p.corner_radius_mm * k);
            if let Some(b) = &mut p.border {
                b.width_mm = round(b.width_mm * k);
            }
        }
        ElementKind::Table(t) => {
            if let Some(b) = &mut t.border {
                b.width_mm = round(b.width_mm * k);
            }
        }
        ElementKind::Image(_) => {}
    }
}

/// Format (A5, A4, A3) et orientation d'une page. Changer de format agrandit ou réduit le contenu d'autant ;
/// tourner la page ne bouge rien, sauf la grille qui se recalcule.
pub fn set_page_format(doc: &mut Document, args: &Value) -> Result<Value, String> {
    let index = args.get("page").and_then(Value::as_u64).ok_or("« page » manque")? as usize;
    let format = args.get("format").and_then(Value::as_str);
    let orientation = args.get("orientation").and_then(Value::as_str);
    let landscape = match orientation {
        None => None,
        Some("portrait") => Some(false),
        Some("landscape") => Some(true),
        Some(o) => return Err(format!("orientation inconnue : {o} (portrait, landscape)")),
    };
    let dims = match format {
        None => None,
        Some(f) => Some(crate::styles::FORMATS.iter().find(|(n, ..)| n.eq_ignore_ascii_case(f)).ok_or(format!("format inconnu : {f} (A5, A4, A3)"))?),
    };
    if dims.is_none() && landscape.is_none() {
        return Err("donnez « format » ou « orientation »".into());
    }
    let page = doc.pages.get_mut(index).ok_or(format!("aucune page {index}"))?;
    if let Some((_, short, long)) = dims {
        let size = if page.size.width_mm > page.size.height_mm { PageSize { width_mm: *long, height_mm: *short } } else { PageSize { width_mm: *short, height_mm: *long } };
        let k = size.width_mm / page.size.width_mm;
        let cells: Vec<Id> = page.grid.as_ref().map(|g| g.cells.iter().map(|c| c.element_id.clone()).collect()).unwrap_or_default();
        for e in page.elements.iter_mut().filter(|e| !cells.contains(&e.id)) {
            scale(e, k);
        }
        carnet_core::grid::set_page_size(page, size);
    }
    if let Some(l) = landscape
        && l != (page.size.width_mm > page.size.height_mm)
    {
        let size = PageSize { width_mm: page.size.height_mm, height_mm: page.size.width_mm };
        carnet_core::grid::set_page_size(page, size);
    }
    Ok(json!({ "ok": true }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::edit;
    use carnet_core::*;
    use serde_json::json;

    fn doc() -> Document {
        let mut d = Document::new("essai");
        for (x, shape) in [(10, "rect"), (50, "star"), (90, "heart")] {
            edit::add_shape(&mut d, &json!({ "page": 0, "x_mm": x, "y_mm": 10, "w_mm": 30, "h_mm": 20, "shape": shape })).unwrap();
        }
        d
    }

    fn ids(d: &Document) -> Vec<String> {
        d.pages[0].elements.iter().map(|e| e.id.clone()).collect()
    }

    fn el<'a>(d: &'a Document, id: &str) -> &'a Element {
        d.pages.iter().flat_map(|p| &p.elements).find(|e| e.id == id).unwrap()
    }

    #[test]
    fn front_and_back_move_a_whole_group() {
        let mut d = doc();
        let [a, b, c] = <[String; 3]>::try_from(ids(&d)).unwrap();
        group(&mut d, &json!({ "element_ids": [a, b] })).unwrap();
        arrange(&mut d, &json!({ "element_id": a, "action": "front" })).unwrap();
        assert_eq!(ids(&d), [c.clone(), a.clone(), b.clone()]);
        arrange(&mut d, &json!({ "element_id": c, "action": "front" })).unwrap();
        arrange(&mut d, &json!({ "element_id": c, "action": "back" })).unwrap();
        assert_eq!(ids(&d), [c, a, b]);
    }

    #[test]
    fn duplicate_copies_a_group_beside_itself() {
        let mut d = doc();
        let [a, b, _] = <[String; 3]>::try_from(ids(&d)).unwrap();
        group(&mut d, &json!({ "element_ids": [a, b] })).unwrap();
        let out = arrange(&mut d, &json!({ "element_id": a, "action": "duplicate" })).unwrap();
        let copies: Vec<String> = out["element_ids"].as_array().unwrap().iter().map(|v| v.as_str().unwrap().to_string()).collect();
        assert_eq!(copies.len(), 2);
        let (orig, copy) = (el(&d, &a), el(&d, &copies[0]));
        assert_eq!((copy.frame.x_mm, copy.frame.y_mm), (orig.frame.x_mm + 5.0, orig.frame.y_mm + 5.0));
        assert!(copy.group.is_some() && copy.group != orig.group);
        assert_eq!(el(&d, &copies[1]).group, copy.group);
    }

    #[test]
    fn mirror_turns_an_element_and_a_photo_uses_its_own_flag() {
        let mut d = doc();
        let a = ids(&d)[0].clone();
        arrange(&mut d, &json!({ "element_id": a, "action": "mirror_h" })).unwrap();
        assert!(el(&d, &a).flip_h);
        arrange(&mut d, &json!({ "element_id": a, "action": "mirror_v" })).unwrap();
        assert!(el(&d, &a).flip_v);
        let p = edit::add_photo_zone(&mut d, &json!({ "page": 0, "x_mm": 1, "y_mm": 1, "w_mm": 9, "h_mm": 9 })).unwrap()["element_id"].as_str().unwrap().to_string();
        arrange(&mut d, &json!({ "element_id": p, "action": "mirror_h" })).unwrap();
        let ElementKind::Photo(ph) = &el(&d, &p).kind else { unreachable!() };
        assert!(ph.mirrored && !el(&d, &p).flip_h);
        let t = edit::add_text(&mut d, &json!({ "page": 0, "x_mm": 1, "y_mm": 1, "w_mm": 9, "h_mm": 9, "content": "x" })).unwrap()["element_id"].as_str().unwrap().to_string();
        assert!(arrange(&mut d, &json!({ "element_id": t, "action": "mirror_h" })).is_err());
    }

    #[test]
    fn a_mirrored_group_swaps_sides_around_its_center() {
        let mut d = doc();
        let [a, b, _] = <[String; 3]>::try_from(ids(&d)).unwrap();
        group(&mut d, &json!({ "element_ids": [a, b] })).unwrap();
        arrange(&mut d, &json!({ "element_id": a, "action": "mirror_h" })).unwrap();
        // Le groupe va de 10 à 80 : le rectangle (10-40) passe à 50-80, l'étoile (50-80) à 10-40.
        assert_eq!((el(&d, &a).frame.x_mm, el(&d, &b).frame.x_mm), (50.0, 10.0));
        assert!(el(&d, &a).flip_h && el(&d, &b).flip_h);
    }

    #[test]
    fn locked_grid_and_table_elements_are_refused() {
        let mut d = doc();
        let a = ids(&d)[0].clone();
        d.pages[0].elements[0].locked = true;
        assert!(arrange(&mut d, &json!({ "element_id": a, "action": "duplicate" })).unwrap_err().contains("verrouillé"));
        let table = table::add_table(&mut d.pages[0], Frame { x_mm: 10.0, y_mm: 100.0, w_mm: 100.0, h_mm: 40.0, rotation_deg: 0.0 }, 2, 2);
        assert!(arrange(&mut d, &json!({ "element_id": table, "action": "mirror_h" })).unwrap_err().contains("tableau"));
        let mut g = Page::new(A4_PORTRAIT);
        grid::set_grid(&mut g, 2, 1);
        d.pages.push(g);
        let cell = d.pages[1].elements[0].id.clone();
        assert!(arrange(&mut d, &json!({ "element_id": cell, "action": "duplicate" })).unwrap_err().contains("grille"));
    }

    #[test]
    fn group_and_ungroup_one_level_at_a_time() {
        let mut d = doc();
        let [a, b, c] = <[String; 3]>::try_from(ids(&d)).unwrap();
        group(&mut d, &json!({ "element_ids": [a, b] })).unwrap();
        let inner = el(&d, &a).group.clone().unwrap();
        group(&mut d, &json!({ "element_ids": [a, c] })).unwrap();
        // Le groupe a-b reste entier, dans le nouveau.
        let outer = el(&d, &c).group.clone().unwrap();
        assert_eq!((el(&d, &a).group.clone(), el(&d, &a).subgroups.clone()), (Some(outer.clone()), vec![inner.clone()]));
        assert_eq!(el(&d, &b).subgroups, vec![inner.clone()]);
        ungroup(&mut d, &json!({ "element_id": c })).unwrap();
        assert_eq!((el(&d, &a).group.clone(), el(&d, &c).group.clone()), (Some(inner), None));
        assert!(group(&mut d, &json!({ "element_ids": [a, b] })).unwrap_err().contains("deux"));
        assert!(ungroup(&mut d, &json!({ "element_id": c })).is_err());
    }

    #[test]
    fn page_format_scales_the_content_and_orientation_turns_the_page() {
        let mut d = doc();
        edit::add_text(&mut d, &json!({ "page": 0, "x_mm": 21, "y_mm": 42, "w_mm": 100, "h_mm": 20, "content": "x", "size_pt": 20 })).unwrap();
        set_page_format(&mut d, &json!({ "page": 0, "format": "A5" })).unwrap();
        assert_eq!((d.pages[0].size.width_mm, d.pages[0].size.height_mm), (148.0, 210.0));
        let t = &d.pages[0].elements[3];
        // 148 / 210 : 21 → 14,8 ; 100 → 70,5 ; 20 pt → 14 pt (au demi-point).
        assert_eq!((t.frame.x_mm, t.frame.w_mm), (14.8, 70.5));
        let ElementKind::Text(tp) = &t.kind else { unreachable!() };
        assert_eq!(tp.style.size_pt, 14.0);
        set_page_format(&mut d, &json!({ "page": 0, "orientation": "landscape" })).unwrap();
        assert_eq!((d.pages[0].size.width_mm, d.pages[0].size.height_mm), (210.0, 148.0));
        assert_eq!(d.pages[0].elements[3].frame.x_mm, 14.8);
        assert!(set_page_format(&mut d, &json!({ "page": 0, "format": "A6" })).is_err());
    }

    #[test]
    fn grid_cells_follow_a_new_page_format() {
        let mut d = Document::new("x");
        grid::set_grid(&mut d.pages[0], 1, 2);
        set_page_format(&mut d, &json!({ "page": 0, "orientation": "landscape" })).unwrap();
        assert!(d.pages[0].elements.iter().all(|e| e.frame.y_mm + e.frame.h_mm <= 210.0 + 1e-6));
    }
}
