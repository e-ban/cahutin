//! Ce que voit l'assistant : la géométrie et les styles, construits champ par champ (liste blanche).
//! Les structures du modèle ne sont jamais sérialisées telles quelles : un champ ajouté plus tard au
//! modèle n'est pas envoyé tant qu'il n'est pas écrit ici.

use carnet_core::{Document, Element, ElementKind, Id, Page, Pattern, ShapeKind, Stroke};
use serde_json::{Value, json};

/// Mise en page de tout le document, ou d'une seule page (indice à partir de 0).
/// Les éléments de `selection` portent `"selected": true`.
pub fn layout(doc: &Document, page: Option<usize>, selection: &[Id]) -> Value {
    let pages: Vec<Value> = doc
        .pages
        .iter()
        .enumerate()
        .filter(|(i, _)| page.is_none_or(|p| p == *i))
        .map(|(i, p)| page_view(doc, i, p, selection))
        .collect();
    // Les noms des styles de Cahutin sont les siens ; ceux qu'a créés l'enseignant restent chez lui.
    let styles: Vec<Value> = crate::styles::of(doc).into_iter().map(|(s, builtin)| if builtin { json!({ "id": s.id, "name": s.name }) } else { json!({ "id": s.id }) }).collect();
    json!({ "page_count": doc.pages.len(), "pages": pages, "text_styles": styles })
}

fn page_view(doc: &Document, index: usize, page: &Page, selection: &[Id]) -> Value {
    let background = page.background.as_ref().map(|b| {
        json!({ "color": b.color, "pattern": b.pattern.map(pattern_name) })
    });
    json!({
        "index": index,
        "width_mm": page.size.width_mm,
        "height_mm": page.size.height_mm,
        "format": crate::styles::format_of(page.size),
        // Comment Cahutin remplira la page avec la classe : une page par élève, une planche d'étiquettes
        // (un élève par bloc), ou une seule page pour toute la classe ; rien si elle n'a aucun champ.
        "class_fill": carnet_core::fill::has_fields(page).then(|| json!({
            "mode": match carnet_core::fill::suggested_mode(page) {
                carnet_core::fill::FillMode::PagePerStudent => "page_per_student",
                carnet_core::fill::FillMode::Sheet => "sheet",
                carnet_core::fill::FillMode::WholeClass => "whole_class",
            },
            "blocks": carnet_core::fill::block_count(page),
        })),
        "landscape": page.size.width_mm > page.size.height_mm,
        "background": background,
        "elements": page.elements.iter().map(|e| element_view(doc, page, e, selection.contains(&e.id))).collect::<Vec<_>>(),
    })
}

fn element_view(doc: &Document, page: &Page, e: &Element, selected: bool) -> Value {
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
        "subgroups": e.subgroups,
        "flip_h": e.flip_h,
        "flip_v": e.flip_v,
        // Une case de grille suit la grille de la page : elle ne se déplace pas seule.
        "in_grid": page.grid.as_ref().is_some_and(|g| g.cells.iter().any(|c| c.element_id == e.id)),
        "selected": selected,
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
            "outline": t.style.outline.unwrap_or(false),
            "padding_mm": t.style.padding(),
            "style": t.style.named,
            // Les champs de la classe du texte ({prenom}…), noms de Cahutin ; jamais les mots autour.
            "fields": t.content.paragraphs.iter().flat_map(|p| &p.runs).flat_map(|r| carnet_core::fields::split(&r.text)).filter_map(|p| match p {
                carnet_core::fields::Piece::Field(f) => Some(f.to_string()),
                carnet_core::fields::Piece::Text(_) => None,
            }).collect::<Vec<_>>(),
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
        ElementKind::Image(i) => json!({ "type": "image", "openmoji": i.code, "name": crate::pictos::name(&i.code), "to_color": i.outline.unwrap_or(false) }),
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
        ShapeKind::Path => "path",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use carnet_core::*;
    use serde_json::json;

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
            subgroups: Vec::new(),
            flip_h: false,
            flip_v: false,
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
            subgroups: Vec::new(),
            flip_h: false,
            flip_v: false,
            kind: ElementKind::Photo(PhotoProps { asset: Some("piege-asset-id".into()), crop: Crop::COVER, corner_radius_mm: 3.0, border: None, shadow: false, mirrored: false }),
        });
        doc.pending_photos.push("/home/maitresse/Photos/Léa.jpg".into());
        doc
    }

    #[test]
    fn no_text_photo_or_file_name_reaches_the_assistant() {
        let out = layout(&sensitive_doc(), None, &["t1".to_string(), "p1".to_string()]).to_string();
        for secret in SECRETS {
            assert!(!out.contains(secret), "{secret:?} dans {out}");
        }
        assert!(!out.contains("Photos"), "{out}");
    }

    #[test]
    fn the_layout_keeps_what_is_needed_to_arrange_the_page() {
        let v = layout(&sensitive_doc(), None, &[]);
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
            // Les noms des images viennent de la banque OpenMoji, publique : ils peuvent coïncider avec un
            // texte de l'exemple (« en colère ») sans rien en révéler.
            // De même pour les styles de Cahutin (« Prénom ») ; ceux de l'enseignant n'ont pas de nom dans la vue.
            let mut view = layout(&doc, None, &[]);
            view.as_object_mut().unwrap().remove("text_styles");
            for p in view["pages"].as_array_mut().unwrap() {
                for e in p["elements"].as_array_mut().unwrap() {
                    e.as_object_mut().unwrap().remove("name");
                    // Le nom d'un champ ({prenom}) est celui de Cahutin, jamais une donnée de l'enfant.
                    e.as_object_mut().unwrap().remove("fields");
                }
            }
            let out = view.to_string();
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
        let v = layout(&doc, None, &[]);
        let t = v["pages"][0]["elements"].as_array().unwrap().iter().find(|e| e["type"] == "table").unwrap().clone();
        assert_eq!((t["cols"].as_u64(), t["rows"].as_u64(), t["cells"].as_array().unwrap().len()), (Some(3), Some(2), 6));
        assert!(!v.to_string().contains("Mathis"));
    }

    #[test]
    fn the_selection_is_marked() {
        let doc = sensitive_doc();
        let v = layout(&doc, None, &["p1".to_string()]);
        let els = v["pages"][0]["elements"].as_array().unwrap();
        assert_eq!(els.iter().filter(|e| e["selected"] == true).count(), 1);
        assert_eq!(els.iter().find(|e| e["id"] == "p1").unwrap()["selected"], true);
    }

    #[test]
    fn an_image_carries_its_name() {
        let mut doc = Document::new("x");
        doc.pages[0].elements.push(Element {
            id: "i1".into(),
            frame: frame(),
            locked: false,
            zone: false,
            group: None,
            subgroups: Vec::new(),
            flip_h: false,
            flip_v: false,
            kind: ElementKind::Image(ImageProps { code: "1F382".into(), outline: None }),
        });
        let v = layout(&doc, None, &[]);
        assert_eq!(v["pages"][0]["elements"][0]["name"], "gâteau d’anniversaire");
    }

    #[test]
    fn the_layout_shows_flips_grid_cells_text_looks_and_page_format() {
        let mut doc = sensitive_doc();
        doc.text_styles.push(NamedTextStyle { id: "s-perso".into(), name: "Style de Léa".into(), font: "Andika".into(), size_pt: 12.0, color: "#000000".into(), align: Align::Left, outline: None });
        let page = &mut doc.pages[0];
        page.elements[1].flip_v = true;
        if let ElementKind::Text(t) = &mut page.elements[0].kind {
            t.style.outline = Some(true);
            t.style.padding_mm = Some(3.0);
            t.style.named = Some("titre".into());
        }
        page.elements.push(Element {
            id: "i1".into(),
            frame: frame(),
            locked: false,
            zone: false,
            group: Some("g1".into()),
            subgroups: vec!["g2".into()],
            flip_h: true,
            flip_v: false,
            kind: ElementKind::Image(ImageProps { code: "1F382".into(), outline: Some(true) }),
        });
        let mut grid = Page::new(PageSize { width_mm: 210.0, height_mm: 148.0 });
        grid::set_grid(&mut grid, 2, 1);
        doc.pages.push(grid);
        let v = layout(&doc, None, &[]);
        let els = &v["pages"][0]["elements"];
        assert_eq!((els[0]["outline"].clone(), els[0]["padding_mm"].clone(), els[0]["style"].clone()), (json!(true), json!(3.0), json!("titre")));
        assert_eq!((els[1]["flip_h"].clone(), els[1]["flip_v"].clone()), (json!(false), json!(true)));
        assert_eq!((els[2]["flip_h"].clone(), els[2]["to_color"].clone(), els[2]["subgroups"].clone()), (json!(true), json!(true), json!(["g2"])));
        assert_eq!(els[0]["in_grid"], false);
        assert_eq!(v["pages"][1]["elements"][0]["in_grid"], true);
        assert_eq!((v["pages"][0]["format"].clone(), v["pages"][0]["landscape"].clone()), (json!("A4"), json!(false)));
        assert_eq!((v["pages"][1]["format"].clone(), v["pages"][1]["landscape"].clone()), (json!("A5"), json!(true)));
        // Les styles de Cahutin avec leur nom ; ceux de l'enseignant par leur seul identifiant.
        let styles = v["text_styles"].as_array().unwrap();
        assert!(styles.contains(&json!({ "id": "titre", "name": "Titre" })), "{styles:?}");
        assert!(styles.contains(&json!({ "id": "s-perso" })), "{styles:?}");
        assert!(!v.to_string().contains("Style de"));
    }

    #[test]
    fn one_page_can_be_asked_for() {
        let mut doc = sensitive_doc();
        doc.pages.push(Page::new(PageSize { width_mm: 297.0, height_mm: 210.0 }));
        let v = layout(&doc, Some(1), &[]);
        assert_eq!(v["pages"].as_array().unwrap().len(), 1);
        assert_eq!(v["pages"][0]["index"], 1);
        assert_eq!(v["page_count"], 2);
    }
}
