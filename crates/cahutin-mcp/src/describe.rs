//! Ce qu'une étape a changé, en mots simples pour l'utilisatrice. Jamais le contenu d'un texte.

use carnet_core::{Align, Background, Document, Element, ElementKind, Id, Page, Pattern, Stroke, VAlign};

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

/// Le seul champ de la classe d'un texte qui ne contient que lui.
fn only_field(t: &carnet_core::TextProps) -> Option<&str> {
    let pieces: Vec<carnet_core::fields::Piece> = t.content.paragraphs.iter().flat_map(|p| &p.runs).flat_map(|r| carnet_core::fields::split(&r.text)).filter(|p| !matches!(p, carnet_core::fields::Piece::Text(x) if x.trim().is_empty())).collect();
    match pieces[..] {
        [carnet_core::fields::Piece::Field(f)] => Some(f),
        _ => None,
    }
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
    if std::mem::discriminant(&b.kind) != std::mem::discriminant(&a.kind) {
        let (before, fem) = label(b);
        let after = name[..1].to_lowercase() + &name[1..];
        return vec![format!("{before} {} par : {after}", agree("remplacé", fem))];
    }
    match (&b.kind, &a.kind) {
        (ElementKind::Table(kb), ElementKind::Table(ka)) => {
            if kb.rows.len() != ka.rows.len() {
                out.push(format!("{} → {} lignes", kb.rows.len(), ka.rows.len()));
            }
            if kb.cols.len() != ka.cols.len() {
                out.push(format!("{} → {} colonnes", kb.cols.len(), ka.cols.len()));
            }
            if (kb.rows.len(), kb.cols.len()) == (ka.rows.len(), ka.cols.len()) && kb.cells.len() != ka.cells.len() {
                out.push(if ka.cells.len() < kb.cells.len() { "cases fusionnées" } else { "case séparée" }.into());
            }
            if kb.border != ka.border {
                out.push("traits changés".into());
            }
        }
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
            if sb.line_spacing != sa.line_spacing {
                out.push("interligne changé".into());
            }
            let bullets = |t: &carnet_core::TextProps| t.content.paragraphs.iter().map(|p| p.list).collect::<Vec<_>>();
            if bullets(tb) != bullets(ta) && plain(tb) == plain(ta) {
                out.push("puces changées".into());
            }
            let (frb, fra) = (tb.frame.clone().unwrap_or_default(), ta.frame.clone().unwrap_or_default());
            if frb.pattern != fra.pattern {
                out.push(lines_fr(fra.pattern));
            }
            look(&mut out, (frb.fill.as_ref(), fra.fill.as_ref()), (frb.stroke.as_ref(), fra.stroke.as_ref()), (frb.corner_radius_mm, fra.corner_radius_mm), "fond changé");
            // Seuls les mots comptent : un style retire les tailles posées sur les mots sans rien écrire.
            if plain(tb) != plain(ta) {
                out.push(match only_field(ta) {
                    Some(f) => format!("champ « {} »", carnet_core::fields::label(f).unwrap_or_default()),
                    None => "nouveau texte écrit".into(),
                });
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
    let prompt = |e: &Element| match &e.kind {
        ElementKind::Text(t) => t.prompt.clone(),
        _ => None,
    };
    if a.zone && (!b.zone || prompt(a) != prompt(b)) {
        out.push("à remplir".into());
    } else if b.zone && !a.zone && !a.locked {
        out.push("plus à remplir".into());
    }
    if a.locked && !b.locked {
        out.push("fixe dans le modèle".into());
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

/// Rectangle vide qui entoure presque toute la page (`isPageFrame`).
fn is_page_frame(e: &Element, size: carnet_core::PageSize) -> bool {
    matches!(&e.kind, ElementKind::Shape(s) if s.shape == carnet_core::ShapeKind::Rect && s.fill.is_none()) && e.frame.w_mm >= 0.8 * size.width_mm && e.frame.h_mm >= 0.8 * size.height_mm
}

/// Un ensemble posé d'un coup par un outil (frise, cadre, fiche), reconnu à sa forme.
fn generated(set: &[&Element], size: carnet_core::PageSize) -> Option<String> {
    let image = |e: &Element, code: &str| matches!(&e.kind, ElementKind::Image(i) if i.code == code);
    let path_colored = |e: &Element, color: &str| matches!(&e.kind, ElementKind::Shape(s) if s.path.is_some() && s.stroke.as_ref().is_some_and(|st| st.color == color));
    if set.iter().all(|e| is_page_frame(e, size)) {
        return Some("Nouveau cadre de page".into());
    }
    if set.len() >= 3 && set.iter().all(|e| matches!(e.kind, ElementKind::Image(_) | ElementKind::Photo(_))) {
        return Some(format!("Nouvelle frise ({} images)", set.len()));
    }
    if set.iter().any(|e| path_colored(e, "#d32f2f")) {
        return Some("Nouvelle fiche de symétrie".into());
    }
    if set.iter().any(|e| image(e, "1F424")) && set.iter().any(|e| image(e, "1F414")) {
        return Some("Nouveau labyrinthe".into());
    }
    // Une ligne de graphisme porte sa recette sur sa ligne de base.
    if set.iter().any(|e| matches!(&e.kind, ElementKind::Shape(s) if s.graphisme.is_some())) {
        return Some("Nouvelle ligne de graphisme".into());
    }
    None
}

/// Les éléments nouveaux d'une page : un ensemble généré en une ligne, des repères de découpe en une ligne,
/// le reste un par un.
fn new_lines(before: &Document, page: &carnet_core::Page, fresh: &[&Element]) -> Vec<String> {
    let mut out = Vec::new();
    let mut done: Vec<&Id> = Vec::new();
    for e in fresh.iter().filter(|e| !crate::decor::is_cut_mark(e)) {
        if done.contains(&&e.id) {
            continue;
        }
        let path = carnet_core::groups::path(e);
        if !path.is_empty() && !elements(before).any(|b| carnet_core::groups::path(b) == path) {
            let set: Vec<&Element> = fresh.iter().copied().filter(|o| carnet_core::groups::path(o) == path && !crate::decor::is_cut_mark(o)).collect();
            let all_new = page.elements.iter().filter(|o| carnet_core::groups::path(o) == path).all(|o| fresh.iter().any(|f| f.id == o.id));
            if let Some(line) = generated(&set, page.size).filter(|_| all_new) {
                out.push(line);
                done.extend(set.iter().map(|o| &o.id));
                continue;
            }
        }
        out.push(new_line(e, page.size));
        done.push(&e.id);
    }
    if fresh.iter().any(|e| crate::decor::is_cut_mark(e)) {
        out.push("Repères de découpe ajoutés".into());
    }
    out
}

fn new_line(e: &Element, page: carnet_core::PageSize) -> String {
    if is_page_frame(e, page) {
        return "Nouveau cadre de page".into();
    }
    if matches!(&e.kind, ElementKind::Text(t) if t.prompt.as_deref() == Some("Légende…")) {
        return "Nouvelle légende".into();
    }
    let (name, fem) = label(e);
    let lower = name[..1].to_lowercase() + &name[1..];
    let size = match &e.kind {
        ElementKind::Table(t) => format!(" ({} × {})", t.cols.len(), t.rows.len()),
        // Un texte qui n'est qu'un champ de la classe : « Nouveau champ « Âge » ».
        ElementKind::Text(t) => match only_field(t) {
            Some(f) => return format!("Nouveau champ « {} »", carnet_core::fields::label(f).unwrap_or_default()),
            None => String::new(),
        },
        _ => String::new(),
    };
    format!("{} {lower}{size}", if fem { "Nouvelle" } else { "Nouveau" })
}

fn cells(e: &Element) -> Vec<Id> {
    match &e.kind {
        ElementKind::Table(t) => t.cells.iter().map(|c| c.element_id.clone()).collect(),
        _ => Vec::new(),
    }
}

/// Cases des tableaux ajoutés, changés ou supprimés : le tableau dit seul ce qui a changé.
fn quiet_cells(before: &Document, after: &Document) -> Vec<Id> {
    let mut out = Vec::new();
    for a in elements(after).filter(|e| matches!(e.kind, ElementKind::Table(_))) {
        let b = elements(before).find(|b| b.id == a.id);
        if b != Some(a) {
            out.extend(cells(a));
            out.extend(b.map(cells).unwrap_or_default());
        }
    }
    for b in elements(before).filter(|b| matches!(b.kind, ElementKind::Table(_)) && !elements(after).any(|a| a.id == b.id)) {
        out.extend(cells(b));
    }
    // De même pour une grille de page posée, changée ou retirée.
    for (_, pb, pa) in same_pages(before, after) {
        if pb.grid != pa.grid {
            for g in [&pb.grid, &pa.grid].into_iter().flatten() {
                out.extend(g.cells.iter().map(|c| c.element_id.clone()));
            }
        }
    }
    out
}

fn grid_line(b: &Option<carnet_core::GridLayout>, a: &Option<carnet_core::GridLayout>) -> Option<String> {
    match (b, a) {
        (_, Some(ga)) if b.as_ref().is_none_or(|gb| (gb.cols, gb.rows) != (ga.cols, ga.rows)) => Some(format!("grille de {} × {} cases", ga.cols, ga.rows)),
        (Some(gb), Some(ga)) if ga.cells.len() < gb.cells.len() => Some("cases de la grille fusionnées".into()),
        (Some(gb), Some(ga)) if ga.cells.len() > gb.cells.len() => Some("case de la grille séparée".into()),
        (Some(_), None) => Some("grille retirée, ses cases restent".into()),
        _ => None,
    }
}

/// Lignes identiques (les exemplaires d'un « Répéter ») comptées une fois : « Nouvelle forme (× 3) ».
fn counted(lines: Vec<String>) -> Vec<String> {
    let mut out: Vec<(String, usize)> = Vec::new();
    for l in lines {
        match out.iter_mut().find(|(x, _)| *x == l) {
            Some((_, n)) => *n += 1,
            None => out.push((l, 1)),
        }
    }
    out.into_iter().map(|(l, n)| if n > 1 { format!("{l} (× {n})") } else { l }).collect()
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

/// Chaque page de `after` (son rang) avec la même page dans `before` : celle de même id, où qu'elle soit (pages
/// remises dans un autre ordre) ; à défaut, celle de même rang si elle n'est plus dans `after` (page remplacée).
fn page_pairs<'a>(before: &'a Document, after: &'a Document) -> Vec<(usize, Option<&'a Page>, &'a Page)> {
    let kept = |p: &Page| after.pages.iter().any(|a| a.id == p.id);
    after
        .pages
        .iter()
        .enumerate()
        .map(|(i, pa)| (i, before.pages.iter().find(|pb| pb.id == pa.id).or_else(|| before.pages.get(i).filter(|pb| !kept(pb))), pa))
        .collect()
}

/// Les pages présentes avant et après, appariées comme `page_pairs`.
fn same_pages<'a>(before: &'a Document, after: &'a Document) -> impl Iterator<Item = (usize, &'a Page, &'a Page)> {
    page_pairs(before, after).into_iter().filter_map(|(i, pb, pa)| pb.map(|pb| (i, pb, pa)))
}

/// « Page 1 déplacée en page 3 » quand une seule page a changé de place ; « Ordre des pages changé » sinon.
fn page_order(before: &Document, after: &Document) -> Option<String> {
    let old: Vec<&Id> = before.pages.iter().map(|p| &p.id).filter(|id| after.pages.iter().any(|p| &p.id == *id)).collect();
    let new: Vec<&Id> = after.pages.iter().map(|p| &p.id).filter(|id| old.contains(id)).collect();
    if old == new {
        return None;
    }
    fn without<'a>(list: &[&'a Id], id: &Id) -> Vec<&'a Id> {
        list.iter().filter(|x| **x != id).copied().collect()
    }
    let rank = |d: &Document, id: &Id| d.pages.iter().position(|p| &p.id == id).map_or(0, |i| i + 1);
    Some(match old.iter().find(|id| without(&old, id) == without(&new, id)) {
        Some(id) => format!("Page {} déplacée en page {}", rank(before, id), rank(after, id)),
        None => "Ordre des pages changé".into(),
    })
}

/// Une ligne par changement, dans l'ordre des pages et des éléments.
pub fn describe(before: &Document, after: &Document) -> Vec<String> {
    let mut out: Vec<String> = page_order(before, after).into_iter().collect();
    // Une page au nouveau format : tout son contenu change de taille, une seule ligne le dit.
    let quiet = quiet_cells(before, after);
    let resized: Vec<usize> = same_pages(before, after).filter(|(_, b, a)| b.size != a.size).map(|(i, _, _)| i).collect();
    for (i, pa) in after.pages.iter().enumerate() {
        if resized.contains(&i) {
            out.push(format!("Page {} : {}", i + 1, format_fr(pa.size)));
            continue;
        }
        let mut fresh = Vec::new();
        for a in pa.elements.iter().filter(|a| !quiet.contains(&a.id)) {
            match elements(before).find(|b| b.id == a.id) {
                Some(b) => out.extend(changes(after, b, a)),
                None => fresh.push(a),
            }
        }
        out.extend(new_lines(before, pa, &fresh));
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
    // Un seul élément « groupé » l'est avec ce qui vient d'être posé (légende, repères, frise) : rien à dire.
    if grouped > 1 {
        out.push(format!("{grouped} éléments groupés"));
    }
    if ungrouped > 1 {
        out.push(format!("{ungrouped} éléments dissociés"));
    }
    for (_, pb, pa) in same_pages(before, after) {
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
    let gone_marks = elements(before).filter(|b| crate::decor::is_cut_mark(b) && !elements(after).any(|a| a.id == b.id)).count();
    if gone_marks > 0 {
        out.push("Repères de découpe retirés".into());
    }
    for b in elements(before).filter(|b| !quiet.contains(&b.id) && !crate::decor::is_cut_mark(b) && !elements(after).any(|a| a.id == b.id)) {
        let (name, fem) = label(b);
        out.push(format!("{name} {}", agree("supprimé", fem)));
    }
    for (i, _, _) in page_pairs(before, after).into_iter().filter(|(_, pb, _)| pb.is_none()) {
        out.push(format!("Page {} ajoutée", i + 1));
    }
    for (i, pb, pa) in same_pages(before, after) {
        if let Some(l) = grid_line(&pb.grid, &pa.grid) {
            out.push(format!("Page {} : {l}", i + 1));
        }
    }
    let white = Background { color: "#ffffff".into(), pattern: None };
    for (i, pb, pa) in same_pages(before, after) {
        let (bb, ba) = (pb.background.as_ref().unwrap_or(&white), pa.background.as_ref().unwrap_or(&white));
        if bb.pattern != ba.pattern {
            out.push(format!("Fond de la page {} : {}", i + 1, pattern_fr(ba.pattern)));
        }
        if bb.color != ba.color {
            out.push(format!("Fond de la page {} : couleur changée", i + 1));
        }
    }
    counted(out)
}

/// Éléments créés ou modifiés (pas les supprimés), à surligner.
pub fn touched(before: &Document, after: &Document) -> Vec<Id> {
    let mut out: Vec<Id> = elements(after).filter(|a| elements(before).find(|b| b.id == a.id) != Some(a)).map(|a| a.id.clone()).collect();
    for (_, pb, pa) in same_pages(before, after) {
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
    fn pages_put_in_another_order() {
        let mut b = doc();
        edit::add_page(&mut b, &json!({})).unwrap();
        edit::add_page(&mut b, &json!({})).unwrap();
        b.pages[1].size = carnet_core::PageSize { width_mm: 297.0, height_mm: 420.0 };
        edit::set_background(&mut b, &json!({ "page": 2, "pattern": "seyes", "color": "#fff8e7" })).unwrap();
        let moved = |from: usize, to: usize| {
            let mut a = b.clone();
            let p = a.pages.remove(from);
            a.pages.insert(to, p);
            a
        };
        let a = moved(0, 2);
        assert_eq!(describe(&b, &a), ["Page 1 déplacée en page 3"]);
        assert!(touched(&b, &a).is_empty());
        assert_eq!(describe(&b, &moved(2, 0)), ["Page 3 déplacée en page 1"]);
        // Une page déplacée puis modifiée : le changement est dit, à sa nouvelle place.
        let mut a = moved(2, 0);
        edit::set_background(&mut a, &json!({ "page": 0, "pattern": "lines" })).unwrap();
        assert_eq!(describe(&b, &a), ["Page 3 déplacée en page 1", "Fond de la page 1 : lignes"]);
        // Plusieurs pages remises dans un autre ordre.
        let mut a = b.clone();
        a.pages.reverse();
        assert_eq!(describe(&b, &a), ["Ordre des pages changé"]);
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
    fn bullets_changed_in_the_panel() {
        let (b, a) = after(|d| {
            if let ElementKind::Text(t) = &mut d.pages[0].elements[0].kind {
                t.content.paragraphs[0].list = Some(carnet_core::ListKind::Bullet);
            }
        });
        assert_eq!(describe(&b, &a), ["Texte : puces changées"]);
    }

    #[test]
    fn line_spacing_changed_in_the_panel() {
        let (b, a) = after(|d| {
            if let ElementKind::Text(t) = &mut d.pages[0].elements[0].kind {
                t.style.line_spacing = Some(1.5);
            }
        });
        assert_eq!(describe(&b, &a), ["Texte : interligne changé"]);
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

    #[test]
    fn a_table_is_one_line_however_many_cells() {
        let (b, a) = after(|d| {
            crate::tables::add_table(d, &json!({ "page": 0, "x_mm": 10, "y_mm": 150, "w_mm": 150, "h_mm": 40, "cols": 3, "rows": 2 })).unwrap();
        });
        assert_eq!(describe(&b, &a), ["Nouveau tableau (3 × 2)"]);
        let cell = |d: &Document| match &d.pages[0].elements.iter().find(|e| matches!(e.kind, ElementKind::Table(_))).unwrap().kind {
            ElementKind::Table(t) => t.cells[0].element_id.clone(),
            _ => unreachable!(),
        };
        let mut c = a.clone();
        crate::tables::table_edit(&mut c, &json!({ "cell_id": cell(&a), "action": "insert_row_below" })).unwrap();
        assert_eq!(describe(&a, &c), ["Tableau : agrandi", "Tableau : 2 → 3 lignes"]);
        let mut m = a.clone();
        let other = match &a.pages[0].elements.iter().find(|e| matches!(e.kind, ElementKind::Table(_))).unwrap().kind {
            ElementKind::Table(t) => t.cells[1].element_id.clone(),
            _ => unreachable!(),
        };
        crate::tables::table_edit(&mut m, &json!({ "cell_id": cell(&a), "action": "merge", "other_cell_id": other })).unwrap();
        assert_eq!(describe(&a, &m), ["Tableau : cases fusionnées"]);
        let mut s = a.clone();
        crate::tables::table_edit(&mut s, &json!({ "cell_id": cell(&a), "action": "set_content", "content": "photo" })).unwrap();
        assert_eq!(describe(&a, &s), ["Texte remplacé par : case photo"]);
        let mut del = a.clone();
        let t = del.pages[0].elements.iter().find(|e| matches!(e.kind, ElementKind::Table(_))).unwrap().id.clone();
        edit::delete_element(&mut del, &json!({ "element_id": t })).unwrap();
        assert_eq!(describe(&a, &del), ["Tableau supprimé"]);
    }

    #[test]
    fn repeated_copies_are_counted_not_listed() {
        let (b, a) = after(|d| {
            let id = id(d, 2);
            crate::tables::repeat(d, &json!({ "element_ids": [id], "cols": 2, "rows": 2 })).unwrap();
        });
        assert_eq!(describe(&b, &a), ["Nouvelle forme (× 3)"]);
    }

    #[test]
    fn template_roles_and_fields() {
        let (b, a) = after(|d| {
            let id = id(d, 0);
            crate::template::set_template_role(d, &json!({ "element_id": id, "role": "fill", "field": "prenom" })).unwrap();
        });
        assert_eq!(describe(&b, &a), ["Texte : champ « Prénom »"]);
        let (b, a) = after(|d| {
            let id = id(d, 0);
            crate::template::set_template_role(d, &json!({ "element_id": id, "role": "fill" })).unwrap();
        });
        assert_eq!(describe(&b, &a), ["Texte : à remplir"]);
        let (b, a) = after(|d| {
            let id = id(d, 0);
            crate::template::set_template_role(d, &json!({ "element_id": id, "role": "fixed" })).unwrap();
        });
        assert_eq!(describe(&b, &a), ["Texte : fixe dans le modèle"]);
        let (b, a) = after(|d| {
            edit::add_text(d, &json!({ "page": 0, "x_mm": 1, "y_mm": 1, "w_mm": 9, "h_mm": 9, "field": "age" })).unwrap();
        });
        assert_eq!(describe(&b, &a), ["Nouveau champ « Âge »"]);
    }

    #[test]
    fn a_page_grid_is_one_line() {
        let (b, a) = after(|d| {
            crate::template::page_grid(d, &json!({ "page": 0, "action": "set", "cols": 2, "rows": 3 })).unwrap();
        });
        assert_eq!(describe(&b, &a), ["Page 1 : grille de 2 × 3 cases"]);
        let cells: Vec<String> = a.pages[0].grid.as_ref().unwrap().cells.iter().map(|c| c.element_id.clone()).collect();
        let mut m = a.clone();
        crate::template::page_grid(&mut m, &json!({ "page": 0, "action": "merge", "cell_id": cells[0], "other_cell_id": cells[1] })).unwrap();
        assert_eq!(describe(&a, &m), ["Page 1 : cases de la grille fusionnées"]);
        let mut r = a.clone();
        crate::template::page_grid(&mut r, &json!({ "page": 0, "action": "remove" })).unwrap();
        assert_eq!(describe(&a, &r), ["Page 1 : grille retirée, ses cases restent"]);
    }

    #[test]
    fn generated_sheets_and_decors_are_one_line_each() {
        let one = |f: &dyn Fn(&mut Document)| {
            let mut d = Document::new("x");
            let b = d.clone();
            f(&mut d);
            describe(&b, &d)
        };
        assert_eq!(one(&|d| { crate::decor::decor(d, &json!({ "kind": "frieze", "page": 0, "side": "top", "motifs": ["soleil"] })).unwrap(); }), ["Nouvelle frise (10 images)"]);
        assert_eq!(one(&|d| { crate::decor::decor(d, &json!({ "kind": "page_frame", "page": 0, "style": "double" })).unwrap(); }), ["Nouveau cadre de page"]);
        assert_eq!(one(&|d| { crate::decor::decor(d, &json!({ "kind": "page_frame", "page": 0, "style": "thin" })).unwrap(); }), ["Nouveau cadre de page"]);
        assert_eq!(one(&|d| { crate::exercises::exercise(d, &json!({ "kind": "graphisme", "page": 0, "motif": "ponts", "rows": 3 })).unwrap(); }), ["Nouvelle ligne de graphisme (× 3)"]);
        assert_eq!(one(&|d| { crate::exercises::exercise(d, &json!({ "kind": "maze", "page": 0, "seed": 3 })).unwrap(); }), ["Nouveau labyrinthe"]);
        assert_eq!(one(&|d| { crate::exercises::exercise(d, &json!({ "kind": "symmetry", "page": 0, "figure": "coeur" })).unwrap(); }), ["Nouvelle fiche de symétrie"]);
    }

    #[test]
    fn cut_marks_captions_and_block_friezes() {
        let (b, a) = after(|d| {
            let id = id(d, 2);
            crate::decor::cut_marks(d, &json!({ "element_ids": [id], "style": "corners" })).unwrap();
        });
        assert_eq!(describe(&b, &a), ["Repères de découpe ajoutés"]);
        let mut gone = a.clone();
        let rid = id(&a, 2);
        crate::decor::cut_marks(&mut gone, &json!({ "element_ids": [rid], "style": "none" })).unwrap();
        assert_eq!(describe(&a, &gone), ["Repères de découpe retirés"]);
        let (b, a) = after(|d| {
            let id = id(d, 2);
            crate::decor::decor(d, &json!({ "kind": "block_frieze", "element_id": id, "motifs": ["fleur"] })).unwrap();
        });
        let lines = describe(&b, &a);
        assert_eq!(lines.len(), 1, "{lines:?}");
        assert!(lines[0].starts_with("Nouvelle frise ("), "{lines:?}");
        let (b, a) = after(|d| {
            let p = edit::add_photo_zone(d, &json!({ "page": 0, "x_mm": 100, "y_mm": 100, "w_mm": 50, "h_mm": 40 })).unwrap()["element_id"].as_str().unwrap().to_string();
            crate::decor::add_caption(d, &json!({ "photo_id": p })).unwrap();
        });
        assert_eq!(describe(&b, &a), ["Nouvelle case photo", "Nouvelle légende"]);
    }
}
