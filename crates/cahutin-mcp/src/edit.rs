//! Ce que l'assistant peut changer : position et taille, style d'un texte, fond d'une page ; il peut aussi
//! ajouter des éléments, écrire un texte sans le lire, supprimer un élément et ajouter une page. Il ne lit
//! jamais le contenu d'un texte ni une photo.

use carnet_core::{
    Align, Background, Crop, Document, Element, ElementKind, Frame, ImageProps, Page, Pattern, PhotoProps, RichText, ShapeKind, ShapeProps, Stroke,
    StrokeStyle, TextFrame, TextProps, TextStyle, VAlign, new_id,
};
use serde_json::{Value, json};

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

pub fn set_frame(doc: &mut Document, args: &Value) -> Result<Value, String> {
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
    Ok(json!({ "ok": true }))
}

/// Booléen facultatif.
fn flag(args: &Value, key: &str) -> Result<Option<bool>, String> {
    match args.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(v) => v.as_bool().map(Some).ok_or(format!("« {key} » doit être true ou false")),
    }
}

fn pattern_arg(args: &Value, key: &str) -> Result<Option<Option<Pattern>>, String> {
    Ok(match text(args, key)? {
        None => None,
        Some("none") => Some(None),
        Some("seyes") => Some(Some(Pattern::Seyes)),
        Some("small_squares") => Some(Some(Pattern::SmallSquares)),
        Some("lines") => Some(Some(Pattern::Lines)),
        Some("dots") => Some(Some(Pattern::Dots)),
        Some(p) => return Err(format!("motif inconnu : {p} (seyes, small_squares, lines, dots, none)")),
    })
}

/// Cadre d'un texte, retiré s'il ne porte plus rien.
fn tidy_frame(t: &mut TextProps) {
    if t.frame.as_ref().is_some_and(|f| f.is_empty() && f.corner_radius_mm == 0.0) {
        t.frame = None;
    }
}

pub fn set_text_style(doc: &mut Document, args: &Value) -> Result<Value, String> {
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
    let valign = match text(args, "valign")? {
        None => None,
        Some("top") => Some(VAlign::Top),
        Some("middle") => Some(VAlign::Middle),
        Some("bottom") => Some(VAlign::Bottom),
        Some(a) => return Err(format!("alignement vertical inconnu : {a} (top, middle, bottom)")),
    };
    let outline = flag(args, "outline")?;
    let lines = pattern_arg(args, "writing_lines")?;
    let named = match text(args, "style")? {
        None => None,
        Some(id) => {
            let styles = crate::styles::of(doc);
            let ids = styles.iter().map(|(s, _)| s.id.as_str()).collect::<Vec<_>>().join(", ");
            Some(styles.into_iter().find(|(s, _)| s.id == id).ok_or(format!("style inconnu : {id} ({ids})"))?.0)
        }
    };
    let ElementKind::Text(t) = &mut element(doc, args)?.kind else { return Err("cet élément n'est pas un texte".into()) };
    // Comme `applyStyle` dans l'interface : l'allure du style, et plus de taille, police ni couleur sur les mots.
    if let Some(s) = named {
        t.style = TextStyle { font: s.font, size_pt: s.size_pt, color: s.color, align: s.align, outline: s.outline, named: Some(s.id), ..t.style.clone() };
        for p in &mut t.content.paragraphs {
            p.align = None;
            for r in &mut p.runs {
                (r.font, r.size_pt, r.color) = (None, None, None);
            }
        }
    }
    if let Some(f) = font {
        t.style.font = f;
    }
    t.style.size_pt = size.unwrap_or(t.style.size_pt);
    if let Some(c) = col {
        t.style.color = c;
    }
    t.style.align = align.unwrap_or(t.style.align);
    if valign.is_some() {
        t.style.valign = valign;
    }
    if let Some(o) = outline {
        t.style.outline = o.then_some(true);
    }
    if let Some(p) = lines {
        t.frame.get_or_insert_with(TextFrame::default).pattern = p;
        tidy_frame(t);
    }
    Ok(json!({ "ok": true }))
}

/// Bord après `set_look` : `border_style` « none » l'enlève ; une couleur, une épaisseur ou un style le
/// créent (trait de 0,5 mm gris foncé) ou le changent.
fn look_stroke(current: &Option<Stroke>, args: &Value) -> Result<Option<Stroke>, String> {
    let style = match text(args, "border_style")? {
        None => None,
        Some("none") => return Ok(None),
        Some("solid") => Some(StrokeStyle::Solid),
        Some("dashed") => Some(StrokeStyle::Dashed),
        Some("dotted") => Some(StrokeStyle::Dotted),
        Some(s) => return Err(format!("bord inconnu : {s} (solid, dashed, dotted, none)")),
    };
    let (col, width) = (color(args, "border_color")?, number(args, "border_width_mm")?);
    if style.is_none() && col.is_none() && width.is_none() {
        return Ok(current.clone());
    }
    let mut s = current.clone().unwrap_or(Stroke { width_mm: 0.5, color: "#222222".into(), style: None });
    if let Some(w) = width {
        s.width_mm = w.clamp(0.1, 20.0);
    }
    if let Some(c) = col {
        s.color = c;
    }
    if style.is_some() {
        s.style = style;
    }
    Ok(Some(s))
}

/// Remplissage après `set_look` : une couleur, « none » pour aucun, absent pour garder.
fn look_fill(current: &Option<String>, args: &Value) -> Result<Option<String>, String> {
    if text(args, "fill")? == Some("none") {
        return Ok(None);
    }
    Ok(color(args, "fill")?.or(current.clone()))
}

/// Allure d'une forme, d'une case photo ou du cadre d'un texte : remplissage, bord, coins arrondis, ombre
/// (photo seulement).
pub fn set_look(doc: &mut Document, args: &Value) -> Result<Value, String> {
    let corner = number(args, "corner_radius_mm")?.map(|r| r.clamp(0.0, 100.0));
    let shadow = flag(args, "shadow")?;
    let has_fill = text(args, "fill")?.is_some();
    match &mut element(doc, args)?.kind {
        ElementKind::Shape(s) => {
            if shadow.is_some() {
                return Err("seule une case photo a une ombre".into());
            }
            s.fill = look_fill(&s.fill, args)?;
            s.stroke = look_stroke(&s.stroke, args)?;
            s.corner_radius_mm = corner.unwrap_or(s.corner_radius_mm);
        }
        ElementKind::Photo(p) => {
            if has_fill {
                return Err("une case photo n'a pas de remplissage".into());
            }
            p.border = look_stroke(&p.border, args)?;
            p.corner_radius_mm = corner.unwrap_or(p.corner_radius_mm);
            p.shadow = shadow.unwrap_or(p.shadow);
        }
        ElementKind::Text(t) => {
            if shadow.is_some() {
                return Err("seule une case photo a une ombre".into());
            }
            let f = t.frame.get_or_insert_with(TextFrame::default);
            f.fill = look_fill(&f.fill, args)?;
            f.stroke = look_stroke(&f.stroke, args)?;
            f.corner_radius_mm = corner.unwrap_or(f.corner_radius_mm);
            tidy_frame(t);
        }
        ElementKind::Image(_) => return Err("une image de la banque n'a ni remplissage ni bord".into()),
        ElementKind::Table(_) => return Err("le bord d'un tableau ne se change pas ici".into()),
    }
    Ok(json!({ "ok": true }))
}

pub fn set_background(doc: &mut Document, args: &Value) -> Result<Value, String> {
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
    Ok(json!({ "ok": true }))
}


/// Cadre d'un nouvel élément : `x_mm`, `y_mm`, `w_mm`, `h_mm` obligatoires, mêmes limites que `set_frame`.
fn new_frame(args: &Value) -> Result<Frame, String> {
    let get = |k: &str| number(args, k)?.ok_or(format!("« {k} » manque"));
    let (x, y, w, h) = (get("x_mm")?, get("y_mm")?, get("w_mm")?, get("h_mm")?);
    for v in [x, y] {
        if !(-MAX_MM..=MAX_MM).contains(&v) {
            return Err(format!("position hors limites : {v} mm"));
        }
    }
    for v in [w, h] {
        if !(v > 0.0 && v <= MAX_MM) {
            return Err(format!("taille hors limites : {v} mm"));
        }
    }
    Ok(Frame { x_mm: x, y_mm: y, w_mm: w, h_mm: h, rotation_deg: 0.0 })
}

/// Ajoute l'élément à la page `page` et rend son identifiant.
fn push(doc: &mut Document, args: &Value, zone: bool, kind: ElementKind) -> Result<Value, String> {
    let index = number(args, "page")?.ok_or("« page » manque")? as usize;
    let frame = new_frame(args)?;
    let page = doc.pages.get_mut(index).ok_or(format!("aucune page {index}"))?;
    let id = new_id();
    page.elements.push(Element { id: id.clone(), frame, locked: false, zone, group: None, subgroups: Vec::new(), flip_h: false, flip_v: false, kind });
    Ok(json!({ "element_id": id }))
}

pub fn add_text(doc: &mut Document, args: &Value) -> Result<Value, String> {
    // Un champ de la classe ({prenom}…) : un texte qui le contient, que la classe remplira.
    let field = text(args, "field")?.map(crate::template::check_field).transpose()?;
    let content = match (text(args, "content")?, &field) {
        (Some(c), _) => c.to_string(),
        (None, Some(f)) => format!("{{{f}}}"),
        (None, None) => return Err("« content » manque".into()),
    };
    let props = TextProps { content: RichText::plain(&content), style: TextStyle::default_zone(), prompt: None, frame: None };
    let out = push(doc, args, false, ElementKind::Text(props))?;
    let mut style = args.clone();
    style["element_id"] = out["element_id"].clone();
    set_text_style(doc, &style)?;
    Ok(out)
}

/// Remplace le texte d'un élément. Rien du texte d'avant n'est rendu.
pub fn set_text(doc: &mut Document, args: &Value) -> Result<Value, String> {
    let content = text(args, "content")?.ok_or("« content » manque")?;
    let ElementKind::Text(t) = &mut element(doc, args)?.kind else { return Err("cet élément n'est pas un texte".into()) };
    t.content = RichText::plain(content);
    Ok(json!({ "ok": true }))
}

pub fn add_shape(doc: &mut Document, args: &Value) -> Result<Value, String> {
    let shape = match text(args, "shape")?.ok_or("« shape » manque")? {
        "rect" => ShapeKind::Rect,
        "ellipse" => ShapeKind::Ellipse,
        "line" => ShapeKind::Line,
        "arrow" => ShapeKind::Arrow,
        "star" => ShapeKind::Star,
        "heart" => ShapeKind::Heart,
        "bubble" => ShapeKind::Bubble,
        "cloud" => ShapeKind::Cloud,
        s => return Err(format!("forme inconnue : {s} (rect, ellipse, line, arrow, star, heart, bubble, cloud)")),
    };
    let fill = color(args, "fill")?;
    let stroke = match (color(args, "border_color")?, number(args, "border_width_mm")?) {
        (None, None) => None,
        (c, w) => Some(Stroke { width_mm: w.unwrap_or(0.5).clamp(0.1, 20.0), color: c.unwrap_or("#222222".into()), style: None }),
    };
    push(doc, args, false, ElementKind::Shape(ShapeProps { shape, fill, stroke, corner_radius_mm: 0.0, path: None }))
}

/// Image de la banque trouvée par un mot-clé français ; rend aussi son nom.
pub fn add_image(doc: &mut Document, args: &Value) -> Result<Value, String> {
    let query = text(args, "query")?.ok_or("« query » manque")?;
    let (code, name) = crate::pictos::search(query).ok_or(format!("aucune image pour « {query} »"))?;
    let outline = flag(args, "to_color")?.and_then(|c| c.then_some(true));
    let mut out = push(doc, args, false, ElementKind::Image(ImageProps { code: code.to_string(), outline }))?;
    out["name"] = json!(name);
    Ok(out)
}

pub fn add_photo_zone(doc: &mut Document, args: &Value) -> Result<Value, String> {
    let props = PhotoProps { asset: None, crop: Crop::COVER, corner_radius_mm: 0.0, border: None, shadow: false, mirrored: false };
    push(doc, args, true, ElementKind::Photo(props))
}

pub fn delete_element(doc: &mut Document, args: &Value) -> Result<Value, String> {
    let id = element(doc, args)?.id.clone();
    let in_table = doc.pages.iter().flat_map(|p| &p.elements).any(|e| matches!(&e.kind, ElementKind::Table(t) if t.cells.iter().any(|c| c.element_id == id)));
    if in_table {
        return Err(format!("« {id} » est une case de tableau : supprimez le tableau"));
    }
    // Un tableau s'en va avec ses cases.
    let cells: Vec<String> = doc.pages.iter().flat_map(|p| &p.elements).filter(|e| e.id == id).flat_map(|e| match &e.kind {
        ElementKind::Table(t) => t.cells.iter().map(|c| c.element_id.clone()).collect(),
        _ => Vec::new(),
    }).collect();
    for p in &mut doc.pages {
        p.elements.retain(|e| e.id != id && !cells.contains(&e.id));
    }
    Ok(json!({ "ok": true }))
}

/// Page vide à la fin, de la taille de la dernière page.
pub fn add_page(doc: &mut Document, _args: &Value) -> Result<Value, String> {
    let size = doc.pages.last().map(|p| p.size).ok_or("le document n'a pas de page")?;
    doc.pages.push(Page::new(size));
    Ok(json!({ "page": doc.pages.len() - 1 }))
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

    fn created<'a>(d: &'a Document, out: &Value) -> &'a Element {
        el(d, out["element_id"].as_str().unwrap())
    }

    fn at() -> Value {
        json!({ "page": 0, "x_mm": 20, "y_mm": 30, "w_mm": 60, "h_mm": 25 })
    }

    fn with(extra: Value) -> Value {
        let mut a = at();
        a.as_object_mut().unwrap().extend(extra.as_object().unwrap().clone());
        a
    }

    #[test]
    fn add_text_creates_a_styled_text() {
        let mut d = doc();
        let out = add_text(&mut d, &with(json!({ "content": "Printemps", "size_pt": 30, "align": "center" }))).unwrap();
        let e = created(&d, &out);
        assert_eq!((e.frame.x_mm, e.frame.w_mm, e.locked, e.zone), (20.0, 60.0, false, false));
        let ElementKind::Text(t) = &e.kind else { panic!() };
        assert_eq!(t.content, RichText::plain("Printemps"));
        assert_eq!((t.style.size_pt, t.style.align), (30.0, Align::Center));
    }

    #[test]
    fn add_text_checks_the_page_and_the_frame() {
        let mut d = doc();
        assert!(add_text(&mut d, &json!({ "page": 3, "x_mm": 0, "y_mm": 0, "w_mm": 10, "h_mm": 10, "content": "x" })).is_err());
        assert!(add_text(&mut d, &json!({ "page": 0, "x_mm": 0, "y_mm": 0, "w_mm": 0, "h_mm": 10, "content": "x" })).is_err());
    }

    #[test]
    fn set_text_replaces_the_content_without_giving_it_back() {
        let mut d = doc();
        let out = set_text(&mut d, &json!({ "element_id": "t1", "content": "Léa" })).unwrap();
        assert!(!out.to_string().contains('x'), "{out}");
        let ElementKind::Text(t) = &el(&d, "t1").kind else { panic!() };
        assert_eq!(t.content, RichText::plain("Léa"));
        assert!(set_text(&mut d, &json!({ "element_id": "verrou", "content": "y" })).is_err());
    }

    #[test]
    fn written_text_is_never_read_back() {
        let mut d = doc();
        set_text(&mut d, &json!({ "element_id": "t1", "content": "Léa" })).unwrap();
        let view = crate::view::layout(&d, None, &[]);
        assert!(!view.to_string().contains("Léa"));
        let t1 = view["pages"][0]["elements"].as_array().unwrap().iter().find(|e| e["id"] == "t1").unwrap().clone();
        assert!(t1.get("empty").is_none(), "un texte écrit n'est pas vide : {t1}");
    }

    #[test]
    fn add_shape_creates_a_shape() {
        let mut d = doc();
        let out = add_shape(&mut d, &with(json!({ "shape": "star", "fill": "#ffcc00", "border_color": "#000000", "border_width_mm": 1 }))).unwrap();
        let ElementKind::Shape(s) = &created(&d, &out).kind else { panic!() };
        assert_eq!((s.shape, s.fill.as_deref()), (ShapeKind::Star, Some("#ffcc00")));
        assert_eq!(s.stroke.as_ref().map(|st| st.width_mm), Some(1.0));
        assert!(add_shape(&mut d, &with(json!({ "shape": "path" }))).is_err());
    }

    #[test]
    fn add_image_finds_a_picture_by_its_french_name() {
        let mut d = doc();
        let out = add_image(&mut d, &with(json!({ "query": "tulipe" }))).unwrap();
        assert_eq!(out["name"], "tulipe");
        let ElementKind::Image(i) = &created(&d, &out).kind else { panic!() };
        assert_eq!(i.code, "1F337");
        assert!(add_image(&mut d, &with(json!({ "query": "xqzwv" }))).is_err());
    }

    #[test]
    fn add_photo_zone_creates_an_empty_photo_zone() {
        let mut d = doc();
        let out = add_photo_zone(&mut d, &at()).unwrap();
        let e = created(&d, &out);
        assert!(e.zone);
        let ElementKind::Photo(p) = &e.kind else { panic!() };
        assert!(p.asset.is_none());
    }

    #[test]
    fn delete_element_spares_locked_elements() {
        let mut d = doc();
        delete_element(&mut d, &json!({ "element_id": "t1" })).unwrap();
        assert!(d.pages[0].elements.iter().all(|e| e.id != "t1"));
        assert!(delete_element(&mut d, &json!({ "element_id": "verrou" })).is_err());
        assert!(delete_element(&mut d, &json!({ "element_id": "inconnu" })).is_err());
    }

    #[test]
    fn delete_element_refuses_a_table_cell() {
        let mut d = doc();
        let fr = Frame { x_mm: 10.0, y_mm: 50.0, w_mm: 100.0, h_mm: 60.0, rotation_deg: 0.0 };
        let table = table::add_table(&mut d.pages[0], fr, 3, 2);
        let ElementKind::Table(t) = &el(&d, &table).kind else { panic!() };
        let cell = t.cells[0].element_id.clone();
        assert!(delete_element(&mut d, &json!({ "element_id": cell })).is_err());
    }

    #[test]
    fn add_page_adds_a_page_of_the_same_size() {
        let mut d = doc();
        d.pages[0].size = PageSize { width_mm: 297.0, height_mm: 420.0 };
        let out = add_page(&mut d, &json!({})).unwrap();
        assert_eq!(out["page"], 1);
        assert_eq!(d.pages[1].size.width_mm, 297.0);
        assert!(d.pages[1].elements.is_empty());
    }

    fn text_props<'a>(d: &'a Document, id: &str) -> &'a TextProps {
        match &el(d, id).kind {
            ElementKind::Text(t) => t,
            _ => unreachable!(),
        }
    }

    #[test]
    fn a_named_style_gives_its_look_and_clears_the_words_own_sizes() {
        let mut d = doc();
        if let ElementKind::Text(t) = &mut d.pages[0].elements[0].kind {
            t.content.paragraphs[0].runs[0].size_pt = Some(40.0);
            t.content.paragraphs[0].align = Some(Align::Right);
        }
        set_text_style(&mut d, &json!({ "element_id": "t1", "style": "titre" })).unwrap();
        let t = text_props(&d, "t1");
        assert_eq!((t.style.font.as_str(), t.style.size_pt, t.style.align, t.style.named.as_deref()), ("Playwrite FR Moderne", 28.0, Align::Center, Some("titre")));
        assert_eq!((t.content.paragraphs[0].runs[0].size_pt, t.content.paragraphs[0].align), (None, None));
        assert_eq!(t.content.paragraphs[0].runs[0].text, "x");
        // Une valeur donnée en plus du style l'emporte.
        set_text_style(&mut d, &json!({ "element_id": "t1", "style": "consigne", "size_pt": 20 })).unwrap();
        assert_eq!((text_props(&d, "t1").style.size_pt, text_props(&d, "t1").style.font.as_str()), (20.0, "Andika"));
        assert!(set_text_style(&mut d, &json!({ "element_id": "t1", "style": "inconnu" })).unwrap_err().contains("titre"));
    }

    #[test]
    fn vertical_alignment_outline_and_writing_lines() {
        let mut d = doc();
        set_text_style(&mut d, &json!({ "element_id": "t1", "valign": "middle", "outline": true, "writing_lines": "seyes" })).unwrap();
        let t = text_props(&d, "t1");
        assert_eq!((t.style.valign, t.style.outline), (Some(VAlign::Middle), Some(true)));
        assert_eq!(t.frame.as_ref().unwrap().pattern, Some(Pattern::Seyes));
        set_text_style(&mut d, &json!({ "element_id": "t1", "outline": false, "writing_lines": "none" })).unwrap();
        let t = text_props(&d, "t1");
        assert_eq!((t.style.outline, t.frame.clone()), (None, None));
        assert!(set_text_style(&mut d, &json!({ "element_id": "t1", "valign": "haut" })).is_err());
    }

    #[test]
    fn the_look_of_a_shape() {
        let mut d = doc();
        let id = add_shape(&mut d, &json!({ "page": 0, "x_mm": 1, "y_mm": 1, "w_mm": 9, "h_mm": 9, "shape": "rect" })).unwrap()["element_id"].as_str().unwrap().to_string();
        set_look(&mut d, &json!({ "element_id": id, "fill": "#FF0000", "border_style": "dashed", "border_width_mm": 1, "corner_radius_mm": 3 })).unwrap();
        let ElementKind::Shape(s) = &el(&d, &id).kind else { unreachable!() };
        assert_eq!((s.fill.as_deref(), s.corner_radius_mm), (Some("#ff0000"), 3.0));
        assert_eq!(s.stroke, Some(Stroke { width_mm: 1.0, color: "#222222".into(), style: Some(StrokeStyle::Dashed) }));
        set_look(&mut d, &json!({ "element_id": id, "fill": "none", "border_style": "none" })).unwrap();
        let ElementKind::Shape(s) = &el(&d, &id).kind else { unreachable!() };
        assert_eq!((s.fill.clone(), s.stroke.clone()), (None, None));
    }

    #[test]
    fn the_look_of_a_photo_and_of_a_text_frame() {
        let mut d = doc();
        let id = add_photo_zone(&mut d, &json!({ "page": 0, "x_mm": 1, "y_mm": 1, "w_mm": 9, "h_mm": 9 })).unwrap()["element_id"].as_str().unwrap().to_string();
        set_look(&mut d, &json!({ "element_id": id, "shadow": true, "border_color": "#0000ff" })).unwrap();
        let ElementKind::Photo(p) = &el(&d, &id).kind else { unreachable!() };
        assert!(p.shadow);
        assert_eq!(p.border.as_ref().map(|b| b.color.as_str()), Some("#0000ff"));
        assert!(set_look(&mut d, &json!({ "element_id": id, "fill": "#ffffff" })).is_err());
        set_look(&mut d, &json!({ "element_id": "t1", "fill": "#ffeeaa", "corner_radius_mm": 4 })).unwrap();
        let f = text_props(&d, "t1").frame.clone().unwrap();
        assert_eq!((f.fill.as_deref(), f.corner_radius_mm), (Some("#ffeeaa"), 4.0));
        assert!(set_look(&mut d, &json!({ "element_id": "t1", "shadow": true })).is_err());
        assert!(set_look(&mut d, &json!({ "element_id": "verrou", "fill": "#ffffff" })).is_err());
    }

    #[test]
    fn an_image_to_color() {
        let mut d = doc();
        let id = add_image(&mut d, &json!({ "page": 0, "x_mm": 1, "y_mm": 1, "w_mm": 9, "h_mm": 9, "query": "soleil", "to_color": true })).unwrap()["element_id"].as_str().unwrap().to_string();
        let ElementKind::Image(i) = &el(&d, &id).kind else { unreachable!() };
        assert_eq!(i.outline, Some(true));
        assert!(set_look(&mut d, &json!({ "element_id": id, "fill": "#ffffff" })).is_err());
    }
}
