//! Ce que voit l'assistant : la géométrie et les styles, construits champ par champ (liste blanche).
//! Les structures du modèle ne sont jamais sérialisées telles quelles : un champ ajouté plus tard au
//! modèle n'est pas envoyé tant qu'il n'est pas écrit ici.

use carnet_core::{Document, Element, ElementKind, Page, Pattern, ShapeKind, Stroke};
use serde_json::{Value, json};

/// Mise en page de tout le document, ou d'une seule page (indice à partir de 0).
pub fn layout(doc: &Document, page: Option<usize>) -> Value {
    let pages: Vec<Value> = doc
        .pages
        .iter()
        .enumerate()
        .filter(|(i, _)| page.is_none_or(|p| p == *i))
        .map(|(i, p)| page_view(doc, i, p))
        .collect();
    json!({ "page_count": doc.pages.len(), "pages": pages })
}

fn page_view(doc: &Document, index: usize, page: &Page) -> Value {
    let background = page.background.as_ref().map(|b| {
        json!({ "color": b.color, "pattern": b.pattern.map(pattern_name) })
    });
    json!({
        "index": index,
        "width_mm": page.size.width_mm,
        "height_mm": page.size.height_mm,
        "background": background,
        "elements": page.elements.iter().map(|e| element_view(doc, e)).collect::<Vec<_>>(),
    })
}

fn element_view(doc: &Document, e: &Element) -> Value {
    let f = &e.frame;
    let mut v = json!({
        "id": e.id,
        "x_mm": f.x_mm,
        "y_mm": f.y_mm,
        "w_mm": f.w_mm,
        "h_mm": f.h_mm,
        "rotation_deg": f.rotation_deg,
        "locked": e.locked,
        "zone": e.zone,
        "group": e.group,
    });
    let extra = match &e.kind {
        ElementKind::Text(t) => json!({
            "type": "text",
            "empty": t.content.is_blank(),
            "font": t.style.font,
            "size_pt": t.style.size_pt,
            "color": t.style.color,
            "align": format!("{:?}", t.style.align).to_lowercase(),
            "valign": t.style.valign.map(|a| format!("{a:?}").to_lowercase()),
            "fill": t.frame.as_ref().and_then(|fr| fr.fill.clone()),
            "border": t.frame.as_ref().and_then(|fr| fr.stroke.as_ref()).map(stroke_view),
            "has_background_image": t.frame.as_ref().is_some_and(|fr| fr.image.is_some()),
            "writing_lines": t.frame.as_ref().and_then(|fr| fr.pattern).map(pattern_name),
        }),
        ElementKind::Photo(p) => {
            let aspect = p.asset.as_ref().and_then(|id| doc.assets.get(id)).map(|m| (m.width_px as f64 / m.height_px.max(1) as f64 * 1000.0).round() / 1000.0);
            json!({
                "type": "photo",
                "has_photo": p.asset.is_some(),
                "photo_aspect": aspect,
                "corner_radius_mm": p.corner_radius_mm,
                "border": p.border.as_ref().map(stroke_view),
                "shadow": p.shadow,
                "mirrored": p.mirrored,
            })
        }
        ElementKind::Shape(s) => json!({
            "type": "shape",
            "shape": shape_name(s.shape),
            "fill": s.fill,
            "border": s.stroke.as_ref().map(stroke_view),
            "corner_radius_mm": s.corner_radius_mm,
        }),
        // Code OpenMoji d'une image décorative de la banque : public, pas une donnée de l'utilisateur.
        ElementKind::Image(i) => json!({ "type": "image", "openmoji": i.code }),
        // Un tableau : sa structure ; ses cases sont des éléments de la page, vus comme les autres.
        ElementKind::Table(t) => json!({
            "type": "table",
            "cols": t.cols.len(),
            "rows": t.rows.len(),
            "border": t.border.as_ref().map(stroke_view),
            "cells": t.cells.iter().map(|c| json!({ "element_id": c.element_id, "col": c.col, "row": c.row, "col_span": c.col_span, "row_span": c.row_span })).collect::<Vec<_>>(),
        }),
    };
    if let (Value::Object(base), Value::Object(more)) = (&mut v, extra) {
        base.extend(more);
    }
    v
}

fn pattern_name(p: Pattern) -> &'static str {
    match p {
        Pattern::Seyes => "seyes",
        Pattern::SmallSquares => "small_squares",
        Pattern::Lines => "lines",
        Pattern::Dots => "dots",
    }
}

fn stroke_view(s: &Stroke) -> Value {
    json!({ "width_mm": s.width_mm, "color": s.color, "style": s.style.map(|st| format!("{st:?}").to_lowercase()) })
}

fn shape_name(k: ShapeKind) -> &'static str {
    match k {
        ShapeKind::Rect => "rect",
        ShapeKind::Ellipse => "ellipse",
        ShapeKind::Line => "line",
        ShapeKind::Arrow => "arrow",
        ShapeKind::Star => "star",
        ShapeKind::Heart => "heart",
        ShapeKind::Bubble => "bubble",
        ShapeKind::Cloud => "cloud",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use carnet_core::*;

    const SECRETS: [&str; 4] = ["Léa", "Mathis", "piege-asset-id", "Prénom secret"];

    fn frame() -> Frame {
        Frame { x_mm: 10.0, y_mm: 20.0, w_mm: 80.0, h_mm: 30.0, rotation_deg: 0.0 }
    }

    /// Un document plein de données personnelles : textes, invite, photo, image de fond d'un texte.
    fn sensitive_doc() -> Document {
        let mut doc = Document::new("Cahier de Léa");
        doc.assets.insert("piege-asset-id".into(), AssetMeta { id: "piege-asset-id".into(), width_px: 400, height_px: 300, mime: Mime::Jpeg });
        let page = &mut doc.pages[0];
        page.elements.push(Element {
            id: "t1".into(),
            frame: frame(),
            locked: false,
            zone: false,
            group: None,
            kind: ElementKind::Text(TextProps {
                content: RichText::plain("Léa est venue avec Mathis"),
                style: TextStyle::default_zone(),
                prompt: Some("Prénom secret".into()),
                frame: Some(TextFrame { fill: None, stroke: None, corner_radius_mm: 0.0, image: Some("piege-asset-id".into()), pattern: None }),
            }),
        });
        page.elements.push(Element {
            id: "p1".into(),
            frame: frame(),
            locked: false,
            zone: true,
            group: None,
            kind: ElementKind::Photo(PhotoProps { asset: Some("piege-asset-id".into()), crop: Crop::COVER, corner_radius_mm: 3.0, border: None, shadow: false, mirrored: false }),
        });
        doc.pending_photos.push("/home/maitresse/Photos/Léa.jpg".into());
        doc
    }

    #[test]
    fn no_text_photo_or_file_name_reaches_the_assistant() {
        let out = layout(&sensitive_doc(), None).to_string();
        for secret in SECRETS {
            assert!(!out.contains(secret), "{secret:?} dans {out}");
        }
        assert!(!out.contains("Photos"), "{out}");
    }

    #[test]
    fn the_layout_keeps_what_is_needed_to_arrange_the_page() {
        let v = layout(&sensitive_doc(), None);
        let page = &v["pages"][0];
        assert_eq!(page["width_mm"], 210.0);
        let text = &page["elements"][0];
        assert_eq!(text["id"], "t1");
        assert_eq!(text["type"], "text");
        assert_eq!(text["x_mm"], 10.0);
        assert_eq!(text["w_mm"], 80.0);
        assert_eq!(text["font"], "Andika");
        assert_eq!(text["empty"], false);
        let photo = &page["elements"][1];
        assert_eq!(photo["type"], "photo");
        assert_eq!(photo["has_photo"], true);
        assert_eq!(photo["photo_aspect"], 1.333);
        assert_eq!(photo["zone"], true);
    }

    /// Les cinq exemples fournis : aucun de leurs textes ni de leurs invites ne sort.
    #[test]
    fn no_example_text_reaches_the_assistant() {
        for info in examples::list() {
            let doc = examples::document(&info.id).unwrap();
            let out = layout(&doc, None).to_string();
            let mut words = vec![doc.title.clone()];
            for e in doc.pages.iter().flat_map(|p| &p.elements) {
                if let ElementKind::Text(t) = &e.kind {
                    words.extend(t.content.paragraphs.iter().flat_map(|p| p.runs.iter().map(|r| r.text.clone())));
                    words.extend(t.prompt.clone());
                }
            }
            let words: Vec<_> = words.into_iter().filter(|w| w.trim().chars().count() > 2).collect();
            assert!(!words.is_empty(), "{} n'a pas de texte", info.id);
            for w in words {
                assert!(!out.contains(w.trim()), "{}: {w:?} dans la vue", info.id);
            }
        }
    }

    #[test]
    fn a_table_shows_its_structure_not_the_text_of_its_cells() {
        let mut doc = sensitive_doc();
        let page = &mut doc.pages[0];
        let id = table::add_table(page, frame(), 3, 2);
        let cell = match &page.elements.iter().find(|e| e.id == id).unwrap().kind {
            ElementKind::Table(t) => t.cells[0].element_id.clone(),
            _ => unreachable!(),
        };
        if let ElementKind::Text(t) = &mut page.elements.iter_mut().find(|e| e.id == cell).unwrap().kind {
            t.content = RichText::plain("Mathis");
        }
        let v = layout(&doc, None);
        let t = v["pages"][0]["elements"].as_array().unwrap().iter().find(|e| e["type"] == "table").unwrap().clone();
        assert_eq!((t["cols"].as_u64(), t["rows"].as_u64(), t["cells"].as_array().unwrap().len()), (Some(3), Some(2), 6));
        assert!(!v.to_string().contains("Mathis"));
    }

    #[test]
    fn one_page_can_be_asked_for() {
        let mut doc = sensitive_doc();
        doc.pages.push(Page::new(PageSize { width_mm: 297.0, height_mm: 210.0 }));
        let v = layout(&doc, Some(1));
        assert_eq!(v["pages"].as_array().unwrap().len(), 1);
        assert_eq!(v["pages"][0]["index"], 1);
        assert_eq!(v["page_count"], 2);
    }
}
