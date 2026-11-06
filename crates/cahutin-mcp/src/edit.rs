//! Ce que l'assistant peut changer : position et taille, style d'un texte, fond d'une page. Jamais le
//! contenu d'un texte ni une photo.

use carnet_core::{Align, Background, Document, Element, ElementKind, Pattern};
use serde_json::Value;

/// Plus grande position ou taille acceptée : bien au-delà d'une affiche A3.
const MAX_MM: f64 = 1000.0;

/// Nombre facultatif : absent ou `null` donne rien, autre chose qu'un nombre est une erreur.
fn number(args: &Value, key: &str) -> Result<Option<f64>, String> {
    match args.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(v) => v.as_f64().map(Some).ok_or(format!("« {key} » doit être un nombre")),
    }
}

fn text<'a>(args: &'a Value, key: &str) -> Result<Option<&'a str>, String> {
    match args.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(v) => v.as_str().map(Some).ok_or(format!("« {key} » doit être un texte")),
    }
}

fn color(args: &Value, key: &str) -> Result<Option<String>, String> {
    let Some(c) = text(args, key)? else { return Ok(None) };
    let ok = c.len() == 7 && c.starts_with('#') && c[1..].bytes().all(|b| b.is_ascii_hexdigit());
    if ok { Ok(Some(c.to_lowercase())) } else { Err(format!("« {key} » doit être une couleur #rrggbb")) }
}

/// Élément modifiable : il existe et n'est pas verrouillé.
fn element<'a>(doc: &'a mut Document, args: &Value) -> Result<&'a mut Element, String> {
    let id = text(args, "element_id")?.ok_or("« element_id » manque")?;
    let e = doc.pages.iter_mut().flat_map(|p| &mut p.elements).find(|e| e.id == id).ok_or(format!("aucun élément « {id} »"))?;
    if e.locked {
        return Err(format!("l'élément « {id} » est verrouillé par l'utilisateur"));
    }
    Ok(e)
}

pub fn set_frame(doc: &mut Document, args: &Value) -> Result<(), String> {
    let (x, y, w, h, r) = (number(args, "x_mm")?, number(args, "y_mm")?, number(args, "w_mm")?, number(args, "h_mm")?, number(args, "rotation_deg")?);
    for v in [x, y].into_iter().flatten() {
        if !(-MAX_MM..=MAX_MM).contains(&v) {
            return Err(format!("position hors limites : {v} mm"));
        }
    }
    for v in [w, h].into_iter().flatten() {
        if !(v > 0.0 && v <= MAX_MM) {
            return Err(format!("taille hors limites : {v} mm"));
        }
    }
    // Une case de tableau suit son tableau : c'est le tableau qui se déplace.
    let id = text(args, "element_id")?.unwrap_or_default().to_string();
    let in_table = doc.pages.iter().flat_map(|p| &p.elements).any(|e| matches!(&e.kind, ElementKind::Table(t) if t.cells.iter().any(|c| c.element_id == id)));
    if in_table {
        return Err(format!("« {id} » est une case de tableau : déplacez ou redimensionnez le tableau"));
    }
    let e = element(doc, args)?;
    let is_table = matches!(e.kind, ElementKind::Table(_));
    let f = &mut e.frame;
    f.x_mm = x.unwrap_or(f.x_mm);
    f.y_mm = y.unwrap_or(f.y_mm);
    f.w_mm = w.unwrap_or(f.w_mm);
    f.h_mm = h.unwrap_or(f.h_mm);
    f.rotation_deg = r.map(|r| r.rem_euclid(360.0)).unwrap_or(f.rotation_deg);
    if is_table && let Some(page) = doc.pages.iter_mut().find(|p| p.elements.iter().any(|e| e.id == id)) {
        carnet_core::table::layout(page, &id);
    }
    Ok(())
}

pub fn set_text_style(doc: &mut Document, args: &Value) -> Result<(), String> {
    let font = text(args, "font")?.map(str::to_string);
    let size = number(args, "size_pt")?;
    if let Some(s) = size
        && !(4.0..=400.0).contains(&s)
    {
        return Err(format!("taille de police hors limites : {s} pt"));
    }
    let col = color(args, "color")?;
    let align = match text(args, "align")? {
        None => None,
        Some("left") => Some(Align::Left),
        Some("center") => Some(Align::Center),
        Some("right") => Some(Align::Right),
        Some("justify") => Some(Align::Justify),
        Some(a) => return Err(format!("alignement inconnu : {a} (left, center, right, justify)")),
    };
    let ElementKind::Text(t) = &mut element(doc, args)?.kind else { return Err("cet élément n'est pas un texte".into()) };
    if let Some(f) = font {
        t.style.font = f;
    }
    t.style.size_pt = size.unwrap_or(t.style.size_pt);
    if let Some(c) = col {
        t.style.color = c;
    }
    t.style.align = align.unwrap_or(t.style.align);
    Ok(())
}

pub fn set_background(doc: &mut Document, args: &Value) -> Result<(), String> {
    let index = number(args, "page")?.ok_or("« page » manque")? as usize;
    let col = color(args, "color")?;
    let pattern = match text(args, "pattern")? {
        None => None,
        Some("seyes") => Some(Pattern::Seyes),
        Some("small_squares") => Some(Pattern::SmallSquares),
        Some("lines") => Some(Pattern::Lines),
        Some("dots") => Some(Pattern::Dots),
        Some(p) => return Err(format!("motif inconnu : {p} (seyes, small_squares, lines, dots)")),
    };
    let page = doc.pages.get_mut(index).ok_or(format!("aucune page {index}"))?;
    let current = page.background.clone().unwrap_or(Background { color: "#ffffff".into(), pattern: None });
    let pattern = if args.get("pattern").is_some() { pattern } else { current.pattern };
    page.background = Some(Background { color: col.unwrap_or(current.color), pattern });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use carnet_core::*;
    use serde_json::json;

    fn doc() -> Document {
        let mut doc = Document::new("essai");
        for (id, locked) in [("t1", false), ("verrou", true)] {
            doc.pages[0].elements.push(Element {
                id: id.into(),
                frame: Frame { x_mm: 10.0, y_mm: 10.0, w_mm: 50.0, h_mm: 20.0, rotation_deg: 0.0 },
                locked,
                zone: false,
                group: None,
                subgroups: Vec::new(),
                flip_h: false,
                flip_v: false,
                kind: ElementKind::Text(TextProps { content: RichText::plain("x"), style: TextStyle::default_zone(), prompt: None, frame: None }),
            });
        }
        doc
    }

    fn el<'a>(d: &'a Document, id: &str) -> &'a Element {
        d.pages[0].elements.iter().find(|e| e.id == id).unwrap()
    }

    #[test]
    fn a_table_moves_with_its_cells_and_a_cell_alone_does_not() {
        let mut d = doc();
        let id = table::add_table(&mut d.pages[0], Frame { x_mm: 10.0, y_mm: 100.0, w_mm: 100.0, h_mm: 40.0, rotation_deg: 0.0 }, 2, 2);
        let cell = match &el(&d, &id).kind {
            ElementKind::Table(t) => t.cells[0].element_id.clone(),
            _ => unreachable!(),
        };
        let before = el(&d, &cell).frame.x_mm;
        set_frame(&mut d, &json!({ "element_id": id, "x_mm": 50 })).unwrap();
        assert_eq!(el(&d, &cell).frame.x_mm - before, 40.0);
        assert!(set_frame(&mut d, &json!({ "element_id": cell, "x_mm": 0 })).is_err());
    }

    #[test]
    fn an_element_moves_and_resizes_with_only_the_given_values() {
        let mut d = doc();
        set_frame(&mut d, &json!({ "element_id": "t1", "x_mm": 30, "w_mm": 120.5 })).unwrap();
        let f = &el(&d, "t1").frame;
        assert_eq!((f.x_mm, f.y_mm, f.w_mm, f.h_mm), (30.0, 10.0, 120.5, 20.0));
    }

    #[test]
    fn bad_frames_and_locked_or_unknown_elements_are_refused() {
        let mut d = doc();
        assert!(set_frame(&mut d, &json!({ "element_id": "t1", "w_mm": 0 })).is_err());
        assert!(set_frame(&mut d, &json!({ "element_id": "t1", "x_mm": "loin" })).is_err());
        assert!(set_frame(&mut d, &json!({ "element_id": "t1", "x_mm": 5000 })).is_err());
        assert!(set_frame(&mut d, &json!({ "element_id": "verrou", "x_mm": 1 })).unwrap_err().contains("verrouillé"));
        assert!(set_frame(&mut d, &json!({ "element_id": "inconnu", "x_mm": 1 })).is_err());
        assert_eq!(el(&d, "t1").frame.w_mm, 50.0);
    }

    #[test]
    fn a_text_style_changes_but_never_its_content() {
        let mut d = doc();
        set_text_style(&mut d, &json!({ "element_id": "t1", "font": "Playwrite FR Moderne", "size_pt": 28, "color": "#c0392b", "align": "center" })).unwrap();
        let ElementKind::Text(t) = &el(&d, "t1").kind else { panic!() };
        assert_eq!(t.style.font, "Playwrite FR Moderne");
        assert_eq!(t.style.size_pt, 28.0);
        assert_eq!(t.style.color, "#c0392b");
        assert_eq!(t.style.align, Align::Center);
        assert_eq!(t.content, RichText::plain("x"));
        assert!(set_text_style(&mut d, &json!({ "element_id": "t1", "color": "rouge" })).is_err());
        assert!(set_text_style(&mut d, &json!({ "element_id": "t1", "align": "diagonal" })).is_err());
    }

    #[test]
    fn a_page_background_changes() {
        let mut d = doc();
        set_background(&mut d, &json!({ "page": 0, "color": "#fff8e7", "pattern": "seyes" })).unwrap();
        let b = d.pages[0].background.as_ref().unwrap();
        assert_eq!((b.color.as_str(), b.pattern), ("#fff8e7", Some(Pattern::Seyes)));
        set_background(&mut d, &json!({ "page": 0, "color": "#ffffff", "pattern": null })).unwrap();
        assert_eq!(d.pages[0].background.as_ref().unwrap().pattern, None);
        assert!(set_background(&mut d, &json!({ "page": 3, "color": "#ffffff" })).is_err());
    }
}
