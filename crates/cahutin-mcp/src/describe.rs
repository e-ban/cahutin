//! Ce qu'une étape a changé, en mots simples pour l'utilisatrice. Jamais le contenu d'un texte.

use carnet_core::{Align, Background, Document, Element, ElementKind, Id, Pattern, Stroke, VAlign};

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

fn plain(t: &carnet_core::TextProps) -> Vec<&str> {
    t.content.paragraphs.iter().flat_map(|p| p.runs.iter().map(|r| r.text.as_str())).collect()
}

fn lines_fr(p: Option<Pattern>) -> String {
    match p {
        None => "lignes d'écriture retirées".into(),
        p => format!("lignes d'écriture ({})", pattern_fr(p)),
    }
}

/// Bord, remplissage et coins, communs aux formes, aux photos et aux cadres de texte.
fn look(out: &mut Vec<String>, fill: (Option<&String>, Option<&String>), stroke: (Option<&Stroke>, Option<&Stroke>), corner: (f64, f64), fill_word: &str) {
    if fill.0 != fill.1 {
        out.push(fill_word.into());
    }
    if stroke.0 != stroke.1 {
        out.push("bord changé".into());
    }
    if corner.0 != corner.1 {
        out.push(if corner.1 > corner.0 { "coins arrondis" } else { "coins moins arrondis" }.into());
    }
}

/// Sans ses groupes : grouper et dissocier se disent pour toute la page.
fn ungrouped(e: &Element) -> Element {
    Element { group: None, subgroups: Vec::new(), ..e.clone() }
}

fn changes(doc: &Document, b: &Element, a: &Element) -> Vec<String> {
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
    let mirrored = |e: &Element| matches!(&e.kind, ElementKind::Photo(p) if p.mirrored);
    if (b.flip_h, b.flip_v, mirrored(b)) != (a.flip_h, a.flip_v, mirrored(a)) {
        out.push(agree("retourné", fem));
    }
    match (&b.kind, &a.kind) {
        (ElementKind::Text(tb), ElementKind::Text(ta)) => {
            let (sb, sa) = (&tb.style, &ta.style);
            if sb.named != sa.named && let Some(id) = &sa.named {
                // Un style donne toute l'allure d'un coup : une ligne, pas une par réglage.
                let style = crate::styles::of(doc).into_iter().find(|(s, _)| &s.id == id);
                out.push(match style {
                    Some((s, true)) => format!("style {}", s.name),
                    _ => "style changé".into(),
                });
            } else {
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
                if sb.outline.unwrap_or(false) != sa.outline.unwrap_or(false) {
                    out.push(if sa.outline == Some(true) { "lettres creuses" } else { "lettres pleines" }.into());
                }
            }
            if sb.valign.unwrap_or_default() != sa.valign.unwrap_or_default() {
                out.push(
                    match sa.valign.unwrap_or_default() {
                        VAlign::Top => "placé en haut",
                        VAlign::Middle => "centré en hauteur",
                        VAlign::Bottom => "placé en bas",
                    }
                    .into(),
                );
            }
            let (frb, fra) = (tb.frame.clone().unwrap_or_default(), ta.frame.clone().unwrap_or_default());
            if frb.pattern != fra.pattern {
                out.push(lines_fr(fra.pattern));
            }
            look(&mut out, (frb.fill.as_ref(), fra.fill.as_ref()), (frb.stroke.as_ref(), fra.stroke.as_ref()), (frb.corner_radius_mm, fra.corner_radius_mm), "fond changé");
            // Seuls les mots comptent : un style retire les tailles posées sur les mots sans rien écrire.
            if plain(tb) != plain(ta) {
                out.push("nouveau texte écrit".into());
            }
        }
        (ElementKind::Shape(kb), ElementKind::Shape(ka)) => {
            look(&mut out, (kb.fill.as_ref(), ka.fill.as_ref()), (kb.stroke.as_ref(), ka.stroke.as_ref()), (kb.corner_radius_mm, ka.corner_radius_mm), "couleur changée");
        }
        (ElementKind::Photo(kb), ElementKind::Photo(ka)) => {
            look(&mut out, (None, None), (kb.border.as_ref(), ka.border.as_ref()), (kb.corner_radius_mm, ka.corner_radius_mm), "");
            if kb.shadow != ka.shadow {
                out.push(if ka.shadow { "ombre ajoutée" } else { "ombre retirée" }.into());
            }
        }
        _ => {}
    }
    if out.is_empty() && ungrouped(b) != ungrouped(a) {
        out.push(agree("modifié", fem));
    }
    out.into_iter().map(|c| format!("{name} : {c}")).collect()
}

/// Les éléments passés devant (à la fin) ou derrière (au début), s'il n'y a que cela.
fn reordered(before: &[Id], after: &[Id]) -> Option<(Vec<Id>, bool)> {
    let after: Vec<&Id> = after.iter().filter(|id| before.contains(id)).collect();
    let before: Vec<&Id> = before.iter().filter(|id| after.contains(id)).collect();
    if before == after {
        return None;
    }
    let n = after.len();
    for k in 1..n {
        let (front, back) = (&after[n - k..], &after[..k]);
        if before.iter().filter(|id| !front.contains(id)).eq(after[..n - k].iter()) {
            return Some((front.iter().map(|id| (*id).clone()).collect(), true));
        }
        if before.iter().filter(|id| !back.contains(id)).eq(after[k..].iter()) {
            return Some((back.iter().map(|id| (*id).clone()).collect(), false));
        }
    }
    None
}

fn ids(p: &carnet_core::Page) -> Vec<Id> {
    p.elements.iter().map(|e| e.id.clone()).collect()
}

fn format_fr(size: carnet_core::PageSize) -> String {
    let side = if size.width_mm > size.height_mm { "paysage" } else { "portrait" };
    match crate::styles::format_of(size) {
        Some(f) => format!("format {f} {side}"),
        None => format!("{} × {} mm", num(size.width_mm), num(size.height_mm)),
    }
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
    // Une page au nouveau format : tout son contenu change de taille, une seule ligne le dit.
    let resized: Vec<usize> = before.pages.iter().zip(&after.pages).enumerate().filter(|(_, (b, a))| b.size != a.size).map(|(i, _)| i).collect();
    for (i, pa) in after.pages.iter().enumerate() {
        if resized.contains(&i) {
            out.push(format!("Page {} : {}", i + 1, format_fr(pa.size)));
            continue;
        }
        for a in &pa.elements {
            match elements(before).find(|b| b.id == a.id) {
                Some(b) => out.extend(changes(after, b, a)),
                None => out.push(new_line(a)),
            }
        }
    }
    let depth = |d: &Document, id: &Id| elements(d).find(|e| &e.id == id).map(|e| carnet_core::groups::path(e).len());
    let (mut grouped, mut ungrouped) = (0, 0);
    for a in elements(after) {
        match depth(before, &a.id) {
            Some(n) if carnet_core::groups::path(a).len() > n => grouped += 1,
            Some(n) if carnet_core::groups::path(a).len() < n => ungrouped += 1,
            _ => {}
        }
    }
    if grouped > 0 {
        out.push(format!("{grouped} éléments groupés"));
    }
    if ungrouped > 0 {
        out.push(format!("{ungrouped} éléments dissociés"));
    }
    for (pb, pa) in before.pages.iter().zip(&after.pages) {
        if let Some((moved, front)) = reordered(&ids(pb), &ids(pa)) {
            let word = if front { "mis devant" } else { "mis derrière" };
            out.push(match &moved[..] {
                [one] => {
                    let (name, fem) = label(pa.elements.iter().find(|e| &e.id == one).unwrap());
                    format!("{name} {}", if fem { word.replacen("mis", "mise", 1) } else { word.to_string() })
                }
                many => format!("{} éléments {word}", many.len()),
            });
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
    let mut out: Vec<Id> = elements(after).filter(|a| elements(before).find(|b| b.id == a.id) != Some(a)).map(|a| a.id.clone()).collect();
    for (pb, pa) in before.pages.iter().zip(&after.pages) {
        if let Some((moved, _)) = reordered(&ids(pb), &ids(pa)) {
            out.extend(moved.into_iter().filter(|id| !out.contains(id)).collect::<Vec<_>>());
        }
    }
    out
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

    #[test]
    fn front_and_back() {
        let (b, a) = after(|d| {
            let id = id(d, 0);
            crate::arrange::arrange(d, &json!({ "element_id": id, "action": "front" })).unwrap();
        });
        assert_eq!(describe(&b, &a), ["Texte mis devant"]);
        assert_eq!(touched(&b, &a), [id(&b, 0)]);
        let (b, a) = after(|d| {
            let id = id(d, 2);
            crate::arrange::arrange(d, &json!({ "element_id": id, "action": "back" })).unwrap();
        });
        assert_eq!(describe(&b, &a), ["Forme mise derrière"]);
    }

    #[test]
    fn a_named_style_is_one_line_and_writes_no_text() {
        let (b, a) = after(|d| {
            let id = id(d, 0);
            edit::set_text_style(d, &json!({ "element_id": id, "style": "titre" })).unwrap();
        });
        assert_eq!(describe(&b, &a), ["Texte : style Titre"]);
    }

    #[test]
    fn text_frame_and_writing_lines() {
        let (b, a) = after(|d| {
            let id = id(d, 0);
            edit::set_text_style(d, &json!({ "element_id": id, "valign": "middle", "outline": true, "writing_lines": "seyes" })).unwrap();
            edit::set_look(d, &json!({ "element_id": id, "fill": "#ffeeaa", "border_style": "dotted" })).unwrap();
        });
        assert_eq!(
            describe(&b, &a),
            ["Texte : lettres creuses", "Texte : centré en hauteur", "Texte : lignes d'écriture (quadrillage Seyès)", "Texte : fond changé", "Texte : bord changé"]
        );
    }

    #[test]
    fn look_and_mirror_of_a_shape() {
        let (b, a) = after(|d| {
            let id = id(d, 2);
            edit::set_look(d, &json!({ "element_id": id, "fill": "#ff0000", "border_color": "#0000ff", "corner_radius_mm": 2 })).unwrap();
            crate::arrange::arrange(d, &json!({ "element_id": id, "action": "mirror_h" })).unwrap();
        });
        assert_eq!(describe(&b, &a), ["Forme : retournée", "Forme : couleur changée", "Forme : bord changé", "Forme : coins arrondis"]);
    }

    #[test]
    fn grouped_and_ungrouped_in_one_line() {
        let (b, a) = after(|d| {
            let ids = [id(d, 1), id(d, 2)];
            crate::arrange::group(d, &json!({ "element_ids": ids })).unwrap();
        });
        assert_eq!(describe(&b, &a), ["2 éléments groupés"]);
        assert_eq!(touched(&b, &a).len(), 2);
        let (b2, a2) = (a.clone(), {
            let mut d = a.clone();
            let id = id(&d, 1);
            crate::arrange::ungroup(&mut d, &json!({ "element_id": id })).unwrap();
            d
        });
        assert_eq!(describe(&b2, &a2), ["2 éléments dissociés"]);
    }

    #[test]
    fn a_new_page_format_is_one_line() {
        let (b, a) = after(|d| {
            crate::arrange::set_page_format(d, &json!({ "page": 0, "format": "A5" })).unwrap();
        });
        assert_eq!(describe(&b, &a), ["Page 1 : format A5 portrait"]);
        let (b, a) = after(|d| {
            crate::arrange::set_page_format(d, &json!({ "page": 0, "orientation": "landscape" })).unwrap();
        });
        assert_eq!(describe(&b, &a), ["Page 1 : format A4 paysage"]);
    }
}
