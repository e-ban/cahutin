//! Repères de découpe, légendes de photo, cadres de page et frises : les mêmes formes que l'interface
//! (`ui/src/lib/cutMarks.ts`, `editActions.ts`, `decor.ts`), pour qu'elle les reconnaisse et sache les
//! modifier ou les retirer. Toute constante ici a sa jumelle là-bas.

use crate::arrange::{bounds, locate, round};
use carnet_core::{Document, Element, ElementKind, Frame, Id, ImageProps, Page, RichText, ShapeKind, ShapeProps, Stroke, StrokeStyle, TextProps, TextStyle, Align, groups, new_id};
use serde_json::{Value, json};

/// Repères de découpe (`cutMarks.ts`).
const CUT_GAP_MM: f64 = 2.0;
const CUT_LEN_MM: f64 = 5.0;
const CUT_COLOR: &str = "#8a8a8a";
const CUT_WIDTH_MM: f64 = 0.3;
const SCISSORS: &str = "2702";
const SCISSORS_MM: f64 = 5.0;
const LINE_FRAME_MM: f64 = 2.0;
/// Légende d'une photo (`editActions.ts`).
const CAPTION_GAP_MM: f64 = 2.0;
const CAPTION_HEIGHT_MM: f64 = 10.0;
const CAPTION_PROMPT: &str = "Légende…";

fn str_arg<'a>(args: &'a Value, key: &str) -> Result<&'a str, String> {
    args.get(key).and_then(Value::as_str).ok_or(format!("« {key} » manque"))
}

fn ids_arg(args: &Value) -> Result<Vec<&str>, String> {
    let ids: Vec<&str> = args.get("element_ids").and_then(Value::as_array).ok_or("« element_ids » manque")?.iter().filter_map(Value::as_str).collect();
    if ids.is_empty() { Err("« element_ids » est vide".into()) } else { Ok(ids) }
}

pub(crate) fn element(id: Id, frame: Frame, group: Option<Id>, kind: ElementKind) -> Element {
    Element { id, frame, locked: false, zone: false, group, subgroups: Vec::new(), flip_h: false, flip_v: false, kind }
}

pub(crate) fn shape(group: &Id, frame: Frame, shape: ShapeKind, stroke: Option<Stroke>) -> Element {
    element(new_id(), frame, Some(group.clone()), ElementKind::Shape(ShapeProps { shape, fill: None, stroke, corner_radius_mm: 0.0, path: None, graphisme: None }))
}

fn frame(x_mm: f64, y_mm: f64, w_mm: f64, h_mm: f64) -> Frame {
    Frame { x_mm, y_mm, w_mm, h_mm, rotation_deg: 0.0 }
}

pub(crate) fn is_cut_mark(e: &Element) -> bool {
    match &e.kind {
        ElementKind::Image(i) => i.code == SCISSORS && e.frame.w_mm == SCISSORS_MM && e.frame.h_mm == SCISSORS_MM,
        ElementKind::Shape(s) => {
            matches!(s.shape, ShapeKind::Line | ShapeKind::Rect) && s.fill.is_none() && s.stroke.as_ref().is_some_and(|st| st.color == CUT_COLOR && st.width_mm == CUT_WIDTH_MM)
        }
        _ => false,
    }
}

fn cut_stroke(dashed: bool) -> Option<Stroke> {
    Some(Stroke { width_mm: CUT_WIDTH_MM, color: CUT_COLOR.into(), style: dashed.then_some(StrokeStyle::Dashed) })
}

/// Trait horizontal ou vertical de (x1, y1) à (x2, y2).
fn line(group: &Id, x1: f64, y1: f64, x2: f64, y2: f64) -> Element {
    let len = (x2 - x1).hypot(y2 - y1);
    let (cx, cy) = ((x1 + x2) / 2.0, (y1 + y2) / 2.0);
    let f = Frame { x_mm: cx - len / 2.0, y_mm: cy - LINE_FRAME_MM / 2.0, w_mm: len, h_mm: LINE_FRAME_MM, rotation_deg: if x1 == x2 { 90.0 } else { 0.0 } };
    shape(group, f, ShapeKind::Line, cut_stroke(false))
}

fn marks(group: &Id, (x0, y0, x1, y1): (f64, f64, f64, f64), dashed: bool) -> Vec<Element> {
    let (g, l) = (CUT_GAP_MM, CUT_LEN_MM);
    if dashed {
        let outline = shape(group, frame(x0 - g, y0 - g, x1 - x0 + 2.0 * g, y1 - y0 + 2.0 * g), ShapeKind::Rect, cut_stroke(true));
        let scissors = element(new_id(), frame(x0 + 2.0 * g, y0 - g - SCISSORS_MM / 2.0, SCISSORS_MM, SCISSORS_MM), Some(group.clone()), ElementKind::Image(ImageProps { code: SCISSORS.into(), outline: None }));
        return vec![outline, scissors];
    }
    // Pour chaque coin, deux traits dans le prolongement des bords, à l'écart du bloc.
    vec![
        line(group, x0 - g - l, y0, x0 - g, y0),
        line(group, x0, y0 - g - l, x0, y0 - g),
        line(group, x1 + g, y0, x1 + g + l, y0),
        line(group, x1, y0 - g - l, x1, y0 - g),
        line(group, x0 - g - l, y1, x0 - g, y1),
        line(group, x0, y1 + g, x0, y1 + g + l),
        line(group, x1 + g, y1, x1 + g + l, y1),
        line(group, x1, y1 + g, x1, y1 + g + l),
    ]
}

/// Pose, remplace ou retire (`none`) les repères de découpe de chaque bloc donné, comme `cutMarks` : un
/// groupe compte pour un bloc, un élément seul est groupé avec ses repères.
pub fn cut_marks(doc: &mut Document, args: &Value) -> Result<Value, String> {
    let given = ids_arg(args)?;
    let dashed = match str_arg(args, "style")? {
        "corners" => Some(false),
        "dashed" => Some(true),
        "none" => None,
        s => return Err(format!("style inconnu : {s} (corners, dashed, none)")),
    };
    let (p, _) = locate(doc, given[0])?;
    for id in &given {
        if locate(doc, id)?.0 != p {
            return Err("les blocs doivent être sur la même page".into());
        }
    }
    let page: &mut Page = &mut doc.pages[p];
    let in_grid = |page: &Page, id: &Id| page.grid.as_ref().is_some_and(|g| g.cells.iter().any(|c| &c.element_id == id));
    let mut selected: Vec<Id> = Vec::new();
    for id in &given {
        for m in crate::arrange::block(page, id) {
            if !selected.contains(&m) && !in_grid(page, &m) {
                selected.push(m);
            }
        }
    }
    // Les blocs : chaque groupe le plus large, ou un élément seul.
    let mut units: Vec<(Option<Id>, Vec<Id>)> = Vec::new();
    for e in page.elements.iter().filter(|e| selected.contains(&e.id) && !is_cut_mark(e)) {
        match units.iter_mut().find(|(g, members)| if e.group.is_some() { *g == e.group } else { members == &vec![e.id.clone()] }) {
            Some((_, members)) => members.push(e.id.clone()),
            None => units.push((e.group.clone(), vec![e.id.clone()])),
        }
    }
    if units.is_empty() {
        return Err("aucun bloc à découper (une case de grille n'a pas de repères)".into());
    }
    for (old, members) in units {
        let group = old.clone().unwrap_or_else(new_id);
        page.elements.retain(|e| !(old.is_some() && e.group == old && is_cut_mark(e)));
        for e in page.elements.iter_mut().filter(|e| members.contains(&e.id)) {
            e.group = Some(group.clone());
        }
        let Some(dashed) = dashed else { continue };
        let b = bounds(&page.elements.iter().filter(|e| members.contains(&e.id)).collect::<Vec<_>>());
        let last = page.elements.iter().rposition(|e| members.contains(&e.id)).unwrap();
        let added = marks(&group, b, dashed);
        page.elements.splice(last + 1..last + 1, added);
    }
    groups::dissolve_small_groups(page);
    Ok(json!({ "ok": true }))
}

/// La légende d'une photo, s'il y en a une, comme `captionOf` : un texte juste sous elle, aussi large, de
/// son groupe (ou sans groupe avec l'invite de légende).
fn caption_of(page: &Page, photo: &Element) -> Option<Id> {
    let (x0, _, x1, y1) = bounds(&[photo]);
    page.elements
        .iter()
        .find(|e| {
            let ElementKind::Text(t) = &e.kind else { return false };
            if e.id == photo.id {
                return false;
            }
            let same = if photo.group.is_some() { groups::path(e) == groups::path(photo) } else { e.group.is_none() && t.prompt.as_deref() == Some(CAPTION_PROMPT) };
            let below = e.frame.y_mm >= y1 - CAPTION_HEIGHT_MM - CAPTION_GAP_MM && e.frame.y_mm <= y1 + 2.0 * CAPTION_HEIGHT_MM;
            let overlap = (e.frame.x_mm + e.frame.w_mm).min(x1) - e.frame.x_mm.max(x0);
            same && below && overlap > e.frame.w_mm / 2.0
        })
        .map(|e| e.id.clone())
}

/// Légende sous une photo, groupée avec elle, comme `addCaption` ; `content` l'écrit.
pub fn add_caption(doc: &mut Document, args: &Value) -> Result<Value, String> {
    let id = str_arg(args, "photo_id")?;
    let content = args.get("content").and_then(Value::as_str).unwrap_or("");
    let (p, i) = locate(doc, id)?;
    let page = &mut doc.pages[p];
    let photo = page.elements[i].clone();
    if !matches!(photo.kind, ElementKind::Photo(_)) {
        return Err("seule une photo a une légende".into());
    }
    if let Some(c) = caption_of(page, &photo) {
        return Err(format!("cette photo a déjà une légende : « {c} » (écrivez-y avec set_text)"));
    }
    let in_grid = page.grid.as_ref().is_some_and(|g| g.cells.iter().any(|c| c.element_id == photo.id));
    // La légende rejoint tout le chemin de groupes de la photo, ou un groupe nouveau avec elle.
    let path: Vec<Id> = if in_grid { Vec::new() } else if photo.group.is_some() { groups::path(&photo) } else { vec![new_id()] };
    let (x0, _, x1, y1) = bounds(&[&photo]);
    let max_y = page.size.height_mm - CAPTION_HEIGHT_MM - CAPTION_GAP_MM;
    let style = TextStyle { font: "Andika".into(), size_pt: 12.0, color: "#333333".into(), align: Align::Center, valign: None, padding_mm: None, outline: None, named: None };
    let kind = ElementKind::Text(TextProps { content: RichText::plain(content), style, prompt: Some(CAPTION_PROMPT.into()), frame: None });
    let mut caption = element(new_id(), frame(round(x0), round((y1 + CAPTION_GAP_MM).min(max_y)), round(x1 - x0), CAPTION_HEIGHT_MM), None, kind);
    groups::set_path(&mut caption, path.clone());
    let cid = caption.id.clone();
    if path.is_empty() {
        page.elements.push(caption);
    } else {
        if photo.group.is_none() {
            groups::set_path(&mut page.elements[i], path);
        }
        page.elements.insert(i + 1, caption);
    }
    Ok(json!({ "element_id": cid }))
}

/// Cadres de page et frises (`decor.ts`).
const MARGIN_MM: f64 = 8.0;
const FRAME_COLOR: &str = "#555555";
const FRIEZE_SIZE_MM: f64 = 15.0;
const FRIEZE_GAP_MM: f64 = 3.0;

fn border(size: carnet_core::PageSize, inset: f64, width_mm: f64, corner_radius_mm: f64, group: Option<Id>) -> Element {
    let f = frame(inset, inset, size.width_mm - 2.0 * inset, size.height_mm - 2.0 * inset);
    let stroke = Some(Stroke { width_mm, color: FRAME_COLOR.into(), style: None });
    element(new_id(), f, group, ElementKind::Shape(ShapeProps { shape: ShapeKind::Rect, fill: None, stroke, corner_radius_mm, path: None, graphisme: None }))
}

/// Positions régulières de `size` sur une longueur, la première au début, la dernière à la fin (`spread`).
fn spread(start: f64, length: f64, size: f64, gap: f64) -> Vec<f64> {
    let n = (((length + gap) / (size + gap)).floor() as usize).max(1);
    if n == 1 {
        return vec![round(start + (length - size) / 2.0)];
    }
    let step = (length - size) / (n - 1) as f64;
    (0..n).map(|i| round(start + i as f64 * step)).collect()
}

/// Positions des motifs le long d'un rectangle ; tout autour, une seule boucle dans le sens horaire
/// (`friezePositions`).
fn frieze_positions((x, y, w, h): (f64, f64, f64, f64), side: &str, size: f64, gap: f64) -> Vec<(f64, f64)> {
    let xs = spread(x, w, size, gap);
    let (top, bottom) = (y, round(y + h - size));
    match side {
        "top" => xs.iter().map(|&x| (x, top)).collect(),
        "bottom" => xs.iter().map(|&x| (x, bottom)).collect(),
        _ => {
            let ys = spread(y, h, size, gap);
            let ys = if ys.len() > 2 { ys[1..ys.len() - 1].to_vec() } else { Vec::new() };
            let right = round(x + w - size);
            let mut out: Vec<(f64, f64)> = xs.iter().map(|&x| (x, top)).collect();
            out.extend(ys.iter().map(|&y| (right, y)));
            out.extend(xs.iter().rev().map(|&x| (x, bottom)));
            out.extend(ys.iter().rev().map(|&y| (x, y)));
            out
        }
    }
}

/// Codes des pictogrammes trouvés par mots-clés.
fn motif_codes(args: &Value) -> Result<Vec<String>, String> {
    let words: Vec<&str> = args.get("motifs").and_then(Value::as_array).ok_or("« motifs » manque")?.iter().filter_map(Value::as_str).collect();
    if words.is_empty() {
        return Err("« motifs » est vide : donnez un ou plusieurs mots-clés".into());
    }
    words.iter().map(|w| crate::pictos::search(w).map(|(c, _)| c.to_string()).ok_or(format!("aucune image pour « {w} »"))).collect()
}

fn frieze_in(rect: (f64, f64, f64, f64), codes: &[String], side: &str, size: f64, gap: f64) -> Vec<Element> {
    let group = new_id();
    frieze_positions(rect, side, size, gap)
        .into_iter()
        .enumerate()
        .map(|(i, (x, y))| element(new_id(), frame(x, y, size, size), Some(group.clone()), ElementKind::Image(ImageProps { code: codes[i % codes.len()].clone(), outline: None })))
        .collect()
}

fn is_picture(e: &Element) -> bool {
    (matches!(e.kind, ElementKind::Image(_)) || matches!(&e.kind, ElementKind::Photo(p) if p.asset.is_some())) && !e.zone && !e.locked && e.frame.rotation_deg == 0.0
}

fn page_of(doc: &mut Document, args: &Value) -> Result<usize, String> {
    let index = args.get("page").and_then(Value::as_u64).ok_or("« page » manque")? as usize;
    if index < doc.pages.len() { Ok(index) } else { Err(format!("aucune page {index}")) }
}

/// Cadre de page, frise le long des marges ou autour d'un bloc, et retrait d'une frise.
pub fn decor(doc: &mut Document, args: &Value) -> Result<Value, String> {
    let size = args.get("size_mm").and_then(Value::as_f64).unwrap_or(FRIEZE_SIZE_MM).clamp(5.0, 60.0);
    let gap = args.get("gap_mm").and_then(Value::as_f64).unwrap_or(FRIEZE_GAP_MM).clamp(0.0, 50.0);
    match str_arg(args, "kind")? {
        "page_frame" => {
            let p = page_of(doc, args)?;
            let page = &mut doc.pages[p];
            let els = match str_arg(args, "style")? {
                "thin" => vec![border(page.size, MARGIN_MM, 0.5, 0.0, None)],
                "thick" => vec![border(page.size, MARGIN_MM, 2.0, 0.0, None)],
                "rounded" => vec![border(page.size, MARGIN_MM, 1.0, 8.0, None)],
                "double" => {
                    let g = new_id();
                    vec![border(page.size, MARGIN_MM, 0.5, 0.0, Some(g.clone())), border(page.size, MARGIN_MM + 3.0, 1.5, 0.0, Some(g))]
                }
                st => return Err(format!("cadre inconnu : {st} (thin, thick, rounded, double)")),
            };
            let ids: Vec<Id> = els.iter().map(|e| e.id.clone()).collect();
            // Derrière tout, comme dans le tiroir Décors.
            page.elements.splice(0..0, els);
            Ok(json!({ "element_ids": ids }))
        }
        "frieze" => {
            let codes = motif_codes(args)?;
            let side = match str_arg(args, "side")? {
                s @ ("top" | "bottom" | "around") => s,
                s => return Err(format!("côté inconnu : {s} (top, bottom, around)")),
            };
            let p = page_of(doc, args)?;
            let page = &mut doc.pages[p];
            let (w, h) = (page.size.width_mm, page.size.height_mm);
            let els = frieze_in((MARGIN_MM, MARGIN_MM, w - 2.0 * MARGIN_MM, h - 2.0 * MARGIN_MM), &codes, side, size, gap);
            let ids: Vec<Id> = els.iter().map(|e| e.id.clone()).collect();
            page.elements.extend(els);
            Ok(json!({ "element_ids": ids }))
        }
        "block_frieze" => {
            let codes = motif_codes(args)?;
            let id = str_arg(args, "element_id")?;
            let (p, i) = locate(doc, id)?;
            let page = &mut doc.pages[p];
            if page.grid.as_ref().is_some_and(|g| g.cells.iter().any(|c| c.element_id == id)) {
                return Err("une case de grille n'a pas de frise".into());
            }
            let target = page.elements[i].clone();
            let f = target.frame;
            // Comme `blockFrieze` : motifs du quart du plus petit côté (6 à 15 mm), centrés sur les bords.
            let size = round((f.w_mm.min(f.h_mm) / 4.0).clamp(6.0, 15.0));
            let rect = (f.x_mm - size / 2.0, f.y_mm - size / 2.0, f.w_mm + size, f.h_mm + size);
            let group = new_id();
            let mut pics = frieze_in(rect, &codes, "around", size, FRIEZE_GAP_MM);
            for e in &mut pics {
                let path = std::iter::once(group.clone()).chain(groups::path(e)).collect();
                groups::set_path(e, path);
            }
            if f.rotation_deg != 0.0 {
                let (cx, cy) = (f.x_mm + f.w_mm / 2.0, f.y_mm + f.h_mm / 2.0);
                let (sin, cos) = f.rotation_deg.to_radians().sin_cos();
                for e in &mut pics {
                    let fr = &mut e.frame;
                    let (dx, dy) = (fr.x_mm + fr.w_mm / 2.0 - cx, fr.y_mm + fr.h_mm / 2.0 - cy);
                    fr.x_mm = round(cx + dx * cos - dy * sin - fr.w_mm / 2.0);
                    fr.y_mm = round(cy + dx * sin + dy * cos - fr.h_mm / 2.0);
                    fr.rotation_deg = f.rotation_deg;
                }
            }
            let block = crate::arrange::block(page, id);
            for e in page.elements.iter_mut().filter(|e| block.contains(&e.id)) {
                let path = std::iter::once(group.clone()).chain(groups::path(e)).collect();
                groups::set_path(e, path);
            }
            let ids: Vec<Id> = pics.iter().map(|e| e.id.clone()).collect();
            page.elements.splice(i + 1..i + 1, pics);
            groups::dissolve_small_groups(page);
            Ok(json!({ "element_ids": ids }))
        }
        "remove_frieze" => {
            let id = str_arg(args, "element_id")?;
            let (p, i) = locate(doc, id)?;
            let page = &mut doc.pages[p];
            let e = &page.elements[i];
            let path = groups::path(e);
            let members: Vec<Id> = page.elements.iter().filter(|o| !path.is_empty() && groups::path(o) == path).map(|o| o.id.clone()).collect();
            let all_pictures = page.elements.iter().filter(|o| members.contains(&o.id)).all(is_picture);
            if members.len() < 3 || !all_pictures {
                return Err(format!("« {id} » ne fait pas partie d'une frise (au moins trois images d'un même groupe)"));
            }
            page.elements.retain(|o| !members.contains(&o.id));
            groups::dissolve_small_groups(page);
            Ok(json!({ "ok": true }))
        }
        k => Err(format!("décor inconnu : {k} (page_frame, frieze, block_frieze, remove_frieze)")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::edit;
    use serde_json::json;

    fn new_id_of(v: Value) -> String {
        v["element_id"].as_str().unwrap().to_string()
    }

    fn el<'a>(d: &'a Document, id: &str) -> &'a Element {
        d.pages.iter().flat_map(|p| &p.elements).find(|e| e.id == id).unwrap()
    }

    /// Ce que regarde `isCutMark` dans l'interface.
    fn looks_like_a_cut_mark(e: &Element) -> bool {
        match &e.kind {
            ElementKind::Image(i) => i.code == "2702" && e.frame.w_mm == 5.0 && e.frame.h_mm == 5.0,
            ElementKind::Shape(s) => {
                matches!(s.shape, ShapeKind::Line | ShapeKind::Rect) && s.fill.is_none() && s.stroke.as_ref().is_some_and(|st| st.color == "#8a8a8a" && st.width_mm == 0.3)
            }
            _ => false,
        }
    }

    fn label() -> (Document, String) {
        let mut d = Document::new("x");
        let r = new_id_of(edit::add_shape(&mut d, &json!({ "page": 0, "x_mm": 20, "y_mm": 20, "w_mm": 80, "h_mm": 40, "shape": "rect" })).unwrap());
        (d, r)
    }

    #[test]
    fn corner_marks_are_eight_grey_lines_grouped_with_the_block() {
        let (mut d, r) = label();
        cut_marks(&mut d, &json!({ "element_ids": [r], "style": "corners" })).unwrap();
        let page = &d.pages[0];
        let marks: Vec<&Element> = page.elements.iter().filter(|e| e.id != r).collect();
        assert_eq!(marks.len(), 8);
        assert!(marks.iter().all(|m| looks_like_a_cut_mark(m) && m.group == el(&d, &r).group && m.group.is_some()));
        // Le trait du coin haut gauche, horizontal : de 13 à 18 mm, à la hauteur du bord.
        assert!(marks.iter().any(|m| m.frame.x_mm == 13.0 && m.frame.w_mm == 5.0 && m.frame.y_mm == 19.0 && m.frame.rotation_deg == 0.0));
    }

    #[test]
    fn dashed_marks_replace_corners_and_none_removes_them() {
        let (mut d, r) = label();
        cut_marks(&mut d, &json!({ "element_ids": [r], "style": "corners" })).unwrap();
        cut_marks(&mut d, &json!({ "element_ids": [r], "style": "dashed" })).unwrap();
        let marks: Vec<&Element> = d.pages[0].elements.iter().filter(|e| e.id != r).collect();
        assert_eq!(marks.len(), 2);
        assert!(marks.iter().all(|m| looks_like_a_cut_mark(m)));
        let outline = marks.iter().find(|m| matches!(m.kind, ElementKind::Shape(_))).unwrap();
        assert_eq!((outline.frame.x_mm, outline.frame.w_mm), (18.0, 84.0));
        cut_marks(&mut d, &json!({ "element_ids": [r], "style": "none" })).unwrap();
        assert_eq!(d.pages[0].elements.len(), 1);
        assert_eq!(el(&d, &r).group, None);
    }

    #[test]
    fn a_caption_goes_under_its_photo_in_its_group() {
        let mut d = Document::new("x");
        let p = new_id_of(edit::add_photo_zone(&mut d, &json!({ "page": 0, "x_mm": 20, "y_mm": 20, "w_mm": 80, "h_mm": 60 })).unwrap());
        let c = new_id_of(add_caption(&mut d, &json!({ "photo_id": p, "content": "La sortie" })).unwrap());
        let cap = el(&d, &c);
        assert_eq!((cap.frame.x_mm, cap.frame.y_mm, cap.frame.w_mm, cap.frame.h_mm), (20.0, 82.0, 80.0, 10.0));
        assert!(cap.group.is_some() && cap.group == el(&d, &p).group);
        let ElementKind::Text(t) = &cap.kind else { unreachable!() };
        assert_eq!((t.prompt.as_deref(), t.content.is_blank()), (Some("Légende…"), false));
        assert!(add_caption(&mut d, &json!({ "photo_id": p })).unwrap_err().contains(&c));
        let s = new_id_of(edit::add_shape(&mut d, &json!({ "page": 0, "x_mm": 1, "y_mm": 1, "w_mm": 9, "h_mm": 9, "shape": "rect" })).unwrap());
        assert!(add_caption(&mut d, &json!({ "photo_id": s })).is_err());
    }

    #[test]
    fn a_caption_stays_on_the_page() {
        let mut d = Document::new("x");
        let p = new_id_of(edit::add_photo_zone(&mut d, &json!({ "page": 0, "x_mm": 20, "y_mm": 230, "w_mm": 80, "h_mm": 60 })).unwrap());
        let c = new_id_of(add_caption(&mut d, &json!({ "photo_id": p })).unwrap());
        assert_eq!(el(&d, &c).frame.y_mm, 297.0 - 10.0 - 2.0);
    }

    fn images(d: &Document) -> Vec<&Element> {
        d.pages[0].elements.iter().filter(|e| matches!(e.kind, ElementKind::Image(_))).collect()
    }

    #[test]
    fn page_frames_go_behind_everything() {
        let (mut d, r) = label();
        decor(&mut d, &json!({ "kind": "page_frame", "page": 0, "style": "thin" })).unwrap();
        let f = &d.pages[0].elements[0];
        assert_eq!((f.frame.x_mm, f.frame.y_mm, f.frame.w_mm, f.frame.h_mm), (8.0, 8.0, 194.0, 281.0));
        let ElementKind::Shape(s) = &f.kind else { unreachable!() };
        assert_eq!((s.shape, s.fill.clone(), s.stroke.clone().map(|st| (st.width_mm, st.color))), (ShapeKind::Rect, None, Some((0.5, "#555555".to_string()))));
        assert_eq!(d.pages[0].elements[1].id, r);
        decor(&mut d, &json!({ "kind": "page_frame", "page": 0, "style": "double" })).unwrap();
        let (a, b) = (&d.pages[0].elements[0], &d.pages[0].elements[1]);
        assert!(a.group.is_some() && a.group == b.group && b.frame.x_mm == 11.0);
        assert!(decor(&mut d, &json!({ "kind": "page_frame", "page": 0, "style": "fleuri" })).is_err());
    }

    #[test]
    fn a_frieze_along_the_page_edges() {
        let mut d = Document::new("x");
        decor(&mut d, &json!({ "kind": "frieze", "page": 0, "side": "top", "motifs": ["soleil", "nuage"] })).unwrap();
        let pics = images(&d);
        // 194 mm utiles, cases de 15 mm, au moins 3 mm entre deux : 10 images, la dernière au bord.
        assert_eq!(pics.len(), 10);
        assert_eq!((pics[0].frame.x_mm, pics[0].frame.y_mm, pics[9].frame.x_mm, pics[0].frame.w_mm), (8.0, 8.0, 187.0, 15.0));
        assert!(pics.iter().all(|e| e.group.is_some() && e.group == pics[0].group));
        let codes: Vec<&str> = pics.iter().map(|e| match &e.kind { ElementKind::Image(i) => i.code.as_str(), _ => "" }).collect();
        assert_ne!(codes[0], codes[1]);
        assert_eq!(codes[0], codes[2]);
        let mut d = Document::new("x");
        decor(&mut d, &json!({ "kind": "frieze", "page": 0, "side": "around", "motifs": ["fleur"] })).unwrap();
        assert_eq!(images(&d).len(), 46);
        assert!(decor(&mut d, &json!({ "kind": "frieze", "page": 0, "side": "top", "motifs": [] })).is_err());
    }

    #[test]
    fn a_frieze_around_a_block_and_its_removal() {
        let (mut d, r) = label();
        decor(&mut d, &json!({ "kind": "block_frieze", "element_id": r, "motifs": ["étoile"] })).unwrap();
        let pics = images(&d);
        assert!(pics.len() >= 3);
        // Un groupe réunit le bloc et la frise, qui reste un groupe à elle.
        let outer = el(&d, &r).group.clone().unwrap();
        assert!(pics.iter().all(|p| p.group.as_ref() == Some(&outer) && p.subgroups.len() == 1));
        // Cases du quart du plus petit côté (40 mm), centrées sur les bords.
        assert_eq!((pics[0].frame.w_mm, pics[0].frame.x_mm, pics[0].frame.y_mm), (10.0, 15.0, 15.0));
        let one = pics[1].id.clone();
        decor(&mut d, &json!({ "kind": "remove_frieze", "element_id": one })).unwrap();
        assert!(images(&d).is_empty());
        assert_eq!(el(&d, &r).group, None);
        assert!(decor(&mut d, &json!({ "kind": "remove_frieze", "element_id": r })).unwrap_err().contains("frise"));
    }
}
