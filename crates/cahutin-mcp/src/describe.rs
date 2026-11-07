//! Ce qu'une étape a changé, en mots simples pour l'utilisatrice. Jamais le contenu d'un texte.

use carnet_core::{Align, Background, Document, Element, ElementKind, Id, Pattern};

/// Nom de l'élément et s'il est féminin (pour accorder « déplacé·e »).
fn label(e: &Element) -> (String, bool) {
    match &e.kind {
        ElementKind::Text(_) => ("Texte".into(), false),
        ElementKind::Image(i) => (crate::pictos::name(&i.code).map_or("Image".into(), |n| format!("Image « {n} »")), true),
        ElementKind::Shape(_) => ("Forme".into(), true),
        ElementKind::Photo(_) => ("Case photo".into(), true),
        ElementKind::Table(_) => ("Tableau".into(), false),
    }
}

fn agree(word: &str, fem: bool) -> String {
    if fem { format!("{word}e") } else { word.to_string() }
}

fn num(v: f64) -> String {
    let r = (v * 10.0).round() / 10.0;
    if r.fract() == 0.0 { format!("{r:.0}") } else { format!("{r}") }
}

fn changes(b: &Element, a: &Element) -> Vec<String> {
    let (name, fem) = label(a);
    let mut out = Vec::new();
    let (fb, fa) = (&b.frame, &a.frame);
    if (fb.x_mm, fb.y_mm) != (fa.x_mm, fa.y_mm) {
        out.push(agree("déplacé", fem));
    }
    if (fb.w_mm, fb.h_mm) != (fa.w_mm, fa.h_mm) {
        out.push(agree(if fa.w_mm * fa.h_mm >= fb.w_mm * fb.h_mm { "agrandi" } else { "rétréci" }, fem));
    }
    if fb.rotation_deg != fa.rotation_deg {
        out.push(format!("rotation {}° → {}°", num(fb.rotation_deg), num(fa.rotation_deg)));
    }
    match (&b.kind, &a.kind) {
        (ElementKind::Text(tb), ElementKind::Text(ta)) => {
            let (sb, sa) = (&tb.style, &ta.style);
            if sb.size_pt != sa.size_pt {
                out.push(format!("taille {} → {} pt", num(sb.size_pt), num(sa.size_pt)));
            }
            if sb.font != sa.font {
                out.push(format!("police {} → {}", sb.font, sa.font));
            }
            if sb.color != sa.color {
                out.push("couleur changée".into());
            }
            if sb.align != sa.align {
                out.push(
                    match sa.align {
                        Align::Left => "aligné à gauche",
                        Align::Center => "aligné au centre",
                        Align::Right => "aligné à droite",
                        Align::Justify => "justifié",
                    }
                    .into(),
                );
            }
            if tb.content != ta.content {
                out.push("nouveau texte écrit".into());
            }
        }
        (kb, ka) if kb != ka => out.push(agree("modifié", fem)),
        _ => {}
    }
    if out.is_empty() && b != a {
        out.push(agree("modifié", fem));
    }
    out.into_iter().map(|c| format!("{name} : {c}")).collect()
}

fn new_line(e: &Element) -> String {
    let (name, fem) = label(e);
    let lower = name[..1].to_lowercase() + &name[1..];
    format!("{} {lower}", if fem { "Nouvelle" } else { "Nouveau" })
}

fn pattern_fr(p: Option<Pattern>) -> &'static str {
    match p {
        Some(Pattern::Seyes) => "quadrillage Seyès",
        Some(Pattern::SmallSquares) => "petits carreaux",
        Some(Pattern::Lines) => "lignes",
        Some(Pattern::Dots) => "points",
        None => "sans motif",
    }
}

fn elements(d: &Document) -> impl Iterator<Item = &Element> {
    d.pages.iter().flat_map(|p| &p.elements)
}

/// Une ligne par changement, dans l'ordre des pages et des éléments.
pub fn describe(before: &Document, after: &Document) -> Vec<String> {
    let mut out = Vec::new();
    for a in elements(after) {
        match elements(before).find(|b| b.id == a.id) {
            Some(b) => out.extend(changes(b, a)),
            None => out.push(new_line(a)),
        }
    }
    for b in elements(before).filter(|b| !elements(after).any(|a| a.id == b.id)) {
        let (name, fem) = label(b);
        out.push(format!("{name} {}", agree("supprimé", fem)));
    }
    for i in before.pages.len()..after.pages.len() {
        out.push(format!("Page {} ajoutée", i + 1));
    }
    let white = Background { color: "#ffffff".into(), pattern: None };
    for (i, (pb, pa)) in before.pages.iter().zip(&after.pages).enumerate() {
        let (bb, ba) = (pb.background.as_ref().unwrap_or(&white), pa.background.as_ref().unwrap_or(&white));
        if bb.pattern != ba.pattern {
            out.push(format!("Fond de la page {} : {}", i + 1, pattern_fr(ba.pattern)));
        }
        if bb.color != ba.color {
            out.push(format!("Fond de la page {} : couleur changée", i + 1));
        }
    }
    out
}

/// Éléments créés ou modifiés (pas les supprimés), à surligner.
pub fn touched(before: &Document, after: &Document) -> Vec<Id> {
    elements(after).filter(|a| elements(before).find(|b| b.id == a.id) != Some(a)).map(|a| a.id.clone()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::edit;
    use serde_json::json;

    fn doc() -> Document {
        let mut d = Document::new("essai");
        edit::add_text(&mut d, &json!({ "page": 0, "x_mm": 10, "y_mm": 10, "w_mm": 50, "h_mm": 20, "content": "Bonjour", "size_pt": 28, "font": "Andika" })).unwrap();
        edit::add_image(&mut d, &json!({ "page": 0, "x_mm": 10, "y_mm": 50, "w_mm": 30, "h_mm": 30, "query": "ballon gonflable" })).unwrap();
        edit::add_shape(&mut d, &json!({ "page": 0, "x_mm": 10, "y_mm": 90, "w_mm": 30, "h_mm": 30, "shape": "star" })).unwrap();
        d
    }

    fn id(d: &Document, i: usize) -> String {
        d.pages[0].elements[i].id.clone()
    }

    fn after(f: impl FnOnce(&mut Document)) -> (Document, Document) {
        let before = doc();
        let mut a = before.clone();
        f(&mut a);
        (before, a)
    }

    #[test]
    fn text_size_and_font() {
        let (b, a) = after(|d| {
            let id = id(d, 0);
            edit::set_text_style(d, &json!({ "element_id": id, "size_pt": 36, "font": "OpenDyslexic" })).unwrap();
        });
        assert_eq!(describe(&b, &a), ["Texte : taille 28 → 36 pt", "Texte : police Andika → OpenDyslexic"]);
        assert_eq!(touched(&b, &a), [id(&a, 0)]);
    }

    #[test]
    fn color_and_alignment() {
        let (b, a) = after(|d| {
            let id = id(d, 0);
            edit::set_text_style(d, &json!({ "element_id": id, "color": "#ff0000", "align": "center" })).unwrap();
        });
        assert_eq!(describe(&b, &a), ["Texte : couleur changée", "Texte : aligné au centre"]);
    }

    #[test]
    fn image_rotation_names_the_image() {
        let (b, a) = after(|d| {
            let id = id(d, 1);
            edit::set_frame(d, &json!({ "element_id": id, "rotation_deg": 5 })).unwrap();
        });
        assert_eq!(describe(&b, &a), ["Image « ballon gonflable » : rotation 0° → 5°"]);
    }

    #[test]
    fn moved_grown_and_shrunk() {
        let (b, a) = after(|d| {
            let id = id(d, 2);
            edit::set_frame(d, &json!({ "element_id": id, "x_mm": 50, "w_mm": 40, "h_mm": 40 })).unwrap();
        });
        assert_eq!(describe(&b, &a), ["Forme : déplacée", "Forme : agrandie"]);
        let (b, a) = after(|d| {
            let id = id(d, 2);
            edit::set_frame(d, &json!({ "element_id": id, "w_mm": 10 })).unwrap();
        });
        assert_eq!(describe(&b, &a), ["Forme : rétrécie"]);
    }

    #[test]
    fn written_text_never_shows_its_content() {
        let (b, a) = after(|d| {
            let id = id(d, 0);
            edit::set_text(d, &json!({ "element_id": id, "content": "Léa" })).unwrap();
        });
        assert_eq!(describe(&b, &a), ["Texte : nouveau texte écrit"]);
        let (b, a) = after(|d| {
            edit::add_text(d, &json!({ "page": 0, "x_mm": 1, "y_mm": 1, "w_mm": 9, "h_mm": 9, "content": "Léa" })).unwrap();
        });
        let lines = describe(&b, &a);
        assert_eq!(lines, ["Nouveau texte"]);
        assert!(lines.iter().all(|l| !l.contains("Léa")));
    }

    #[test]
    fn new_and_deleted_elements() {
        let (b, a) = after(|d| {
            edit::add_image(d, &json!({ "page": 0, "x_mm": 1, "y_mm": 1, "w_mm": 9, "h_mm": 9, "query": "gâteau d'anniversaire" })).unwrap();
        });
        assert_eq!(describe(&b, &a), ["Nouvelle image « gâteau d’anniversaire »"]);
        assert_eq!(touched(&b, &a), [a.pages[0].elements[3].id.clone()]);
        let (b, a) = after(|d| {
            let id = id(d, 0);
            edit::delete_element(d, &json!({ "element_id": id })).unwrap();
        });
        assert_eq!(describe(&b, &a), ["Texte supprimé"]);
        assert!(touched(&b, &a).is_empty());
    }

    #[test]
    fn pages_and_backgrounds() {
        let (b, a) = after(|d| {
            edit::add_page(d, &json!({})).unwrap();
        });
        assert_eq!(describe(&b, &a), ["Page 2 ajoutée"]);
        let (b, a) = after(|d| {
            edit::set_background(d, &json!({ "page": 0, "pattern": "seyes" })).unwrap();
        });
        assert_eq!(describe(&b, &a), ["Fond de la page 1 : quadrillage Seyès"]);
        let (b, a) = after(|d| {
            edit::set_background(d, &json!({ "page": 0, "color": "#ffffe0" })).unwrap();
        });
        assert_eq!(describe(&b, &a), ["Fond de la page 1 : couleur changée"]);
    }
}
