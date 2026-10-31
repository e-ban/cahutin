//! Pages pour la classe, à partir d'une page modèle : une page par élève, ou une planche dont les blocs
//! reçoivent les élèves les uns après les autres.
//!
//! Champs : une zone de texte dont l'invite est « Prénom », « PRÉNOM », « Nom » ou « Groupe » ; les
//! marqueurs `{prénom}`, `{PRÉNOM}`, `{nom}`, `{groupe}` dans n'importe quel texte ; une zone photo vide,
//! qui reçoit la photo de l'élève seulement si l'autorisation est donnée, sinon son symbole.

use crate::class::Student;
use crate::model::*;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, TS)]
#[ts(export)]
pub enum FillMode {
    PagePerStudent,
    Sheet,
}

/// Écart vertical (mm) en dessous duquel deux blocs sont sur la même rangée.
const ROW_TOLERANCE_MM: f64 = 3.0;

const MARKERS: [&str; 4] = ["{prénom}", "{PRÉNOM}", "{nom}", "{groupe}"];

fn field_value(field: &str, s: &Student) -> Option<String> {
    match field {
        "Prénom" | "{prénom}" => Some(s.first_name.clone()),
        "PRÉNOM" | "{PRÉNOM}" => Some(s.first_name.to_uppercase()),
        "Nom" | "{nom}" => Some(s.last_name.clone()),
        "Groupe" | "{groupe}" => Some(s.group.clone()),
        _ => None,
    }
}

fn is_field_zone(e: &Element) -> bool {
    match &e.kind {
        ElementKind::Text(t) => e.zone && t.prompt.as_deref().is_some_and(|p| field_value(p, &Student::new("", "")).is_some()),
        ElementKind::Photo(p) => e.zone && p.asset.is_none(),
        _ => false,
    }
}

fn has_markers(e: &Element) -> bool {
    matches!(&e.kind, ElementKind::Text(t) if t.content.paragraphs.iter().flat_map(|p| &p.runs).any(|r| MARKERS.iter().any(|m| r.text.contains(m))))
}

fn is_field(e: &Element) -> bool {
    is_field_zone(e) || has_markers(e)
}

pub fn has_fields(page: &Page) -> bool {
    page.elements.iter().any(is_field)
}

/// Page déjà remplie pour un élève (une zone « Prénom » écrite, ou une photo d'élève de la classe) : la
/// remplir de nouveau mettrait la photo ou le prénom d'un enfant sur les pages des autres.
pub fn already_filled(page: &Page, class_photos: &[Id]) -> bool {
    page.elements.iter().any(|e| match &e.kind {
        ElementKind::Text(t) => is_field_zone(e) && !t.content.is_blank(),
        ElementKind::Photo(p) => p.asset.as_ref().is_some_and(|a| class_photos.contains(a)),
        _ => false,
    })
}

/// Remplit les champs de l'élément pour l'élève ; `None` : bloc sans élève, ses champs sont vidés.
fn fill_element(e: &mut Element, s: Option<&Student>) {
    let field_zone = is_field_zone(e);
    let markers = has_markers(e);
    match &mut e.kind {
        ElementKind::Text(t) => {
            if field_zone {
                let value = s.and_then(|s| field_value(t.prompt.as_deref().unwrap_or(""), s)).unwrap_or_default();
                t.content = if value.is_empty() { RichText::default() } else { RichText::plain(&value) };
            } else if s.is_none() && markers {
                t.content = RichText::default();
            } else if let Some(s) = s {
                for r in t.content.paragraphs.iter_mut().flat_map(|p| &mut p.runs) {
                    for m in MARKERS {
                        if r.text.contains(m) {
                            r.text = r.text.replace(m, &field_value(m, s).unwrap_or_default());
                        }
                    }
                }
            }
        }
        ElementKind::Photo(p) if field_zone => {
            let Some(s) = s else { return };
            if s.photo_allowed && let Some(photo) = &s.photo {
                p.asset = Some(photo.clone());
                p.crop = Crop::COVER;
            } else if let Some(code) = &s.symbol {
                // Sans autorisation : le symbole de l'enfant, carré et centré dans la zone.
                let f = e.frame;
                let side = f.w_mm.min(f.h_mm);
                e.frame = Frame { x_mm: f.x_mm + (f.w_mm - side) / 2.0, y_mm: f.y_mm + (f.h_mm - side) / 2.0, w_mm: side, h_mm: side, ..f };
                e.kind = ElementKind::Image(ImageProps { code: code.clone() });
                e.zone = false;
            }
        }
        _ => {}
    }
}

/// La page remplie pour un élève, avec de nouveaux identifiants.
pub fn fill_for(page: &Page, s: &Student) -> Page {
    let mut out = page.clone();
    for e in &mut out.elements {
        fill_element(e, Some(s));
    }
    renew_ids(&mut out);
    out
}

/// Blocs à remplir de la page, dans l'ordre de lecture : un groupe ou un élément seul qui contient un champ.
/// Dans un tableau, chaque ligne est un bloc (une ligne par élève).
fn blocks(page: &Page) -> Vec<Vec<usize>> {
    let mut row_of: std::collections::HashMap<&str, u8> = std::collections::HashMap::new();
    for e in &page.elements {
        if let ElementKind::Table(t) = &e.kind {
            row_of.extend(t.cells.iter().map(|c| (c.element_id.as_str(), c.row)));
        }
    }
    let mut units: Vec<(Option<(&Id, Option<u8>)>, Vec<usize>)> = Vec::new();
    for (i, e) in page.elements.iter().enumerate() {
        let key = e.group.as_ref().map(|g| (g, row_of.get(e.id.as_str()).copied()));
        match key.and_then(|k| units.iter_mut().find(|(u, _)| *u == Some(k))) {
            Some((_, members)) => members.push(i),
            None => units.push((key, vec![i])),
        }
    }
    let mut found: Vec<(f64, f64, Vec<usize>)> = units
        .into_iter()
        .map(|(_, members)| members)
        .filter(|members| members.iter().any(|&i| is_field(&page.elements[i])))
        .map(|members| {
            let top = members.iter().map(|&i| page.elements[i].frame.y_mm).fold(f64::INFINITY, f64::min);
            let left = members.iter().map(|&i| page.elements[i].frame.x_mm).fold(f64::INFINITY, f64::min);
            (top, left, members)
        })
        .collect();
    found.sort_by(|a, b| a.0.total_cmp(&b.0));
    // Rangées : un bloc dont le haut est à moins de ROW_TOLERANCE_MM du premier de la rangée en fait partie.
    let mut rows: Vec<Vec<(f64, f64, Vec<usize>)>> = Vec::new();
    for b in found {
        match rows.last_mut() {
            Some(row) if b.0 - row[0].0 < ROW_TOLERANCE_MM => row.push(b),
            _ => rows.push(vec![b]),
        }
    }
    rows.into_iter()
        .flat_map(|mut row| {
            row.sort_by(|a, b| a.1.total_cmp(&b.1));
            row.into_iter().map(|b| b.2)
        })
        .collect()
}

/// La planche remplie : autant de copies de la page qu'il faut pour tous les élèves ; les blocs restés
/// sans élève sont vidés. Aucun élève : aucune page.
pub fn fill_sheet(page: &Page, students: &[Student]) -> Vec<Page> {
    let blocks = blocks(page);
    if students.is_empty() || blocks.is_empty() {
        return Vec::new();
    }
    students
        .chunks(blocks.len())
        .map(|chunk| {
            let mut out = page.clone();
            for (b, members) in blocks.iter().enumerate() {
                for &i in members {
                    fill_element(&mut out.elements[i], chunk.get(b));
                }
            }
            renew_ids(&mut out);
            out
        })
        .collect()
}

pub fn generate(page: &Page, students: &[Student], mode: FillMode) -> Vec<Page> {
    match mode {
        FillMode::PagePerStudent => students.iter().map(|s| fill_for(page, s)).collect(),
        FillMode::Sheet => fill_sheet(page, students),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(x: f64, y: f64, w: f64, h: f64) -> Frame {
        Frame { x_mm: x, y_mm: y, w_mm: w, h_mm: h, rotation_deg: 0.0 }
    }

    fn text(prompt: Option<&str>, content: &str, zone: bool, f: Frame) -> Element {
        Element {
            id: new_id(),
            frame: f,
            locked: false,
            zone,
            group: None,
            kind: ElementKind::Text(TextProps { content: RichText::plain(content), style: TextStyle::default_zone(), prompt: prompt.map(String::from), frame: None }),
        }
    }

    fn photo_zone(f: Frame) -> Element {
        Element {
            id: new_id(),
            frame: f,
            locked: false,
            zone: true,
            group: None,
            kind: ElementKind::Photo(PhotoProps { asset: None, crop: Crop::COVER, corner_radius_mm: 0.0, border: None, shadow: false }),
        }
    }

    fn student(first: &str, photo: Option<&str>, allowed: bool, symbol: Option<&str>) -> Student {
        Student { photo: photo.map(String::from), photo_allowed: allowed, symbol: symbol.map(String::from), ..Student::new(first, "Martin") }
    }

    fn plain(e: &Element) -> String {
        match &e.kind {
            ElementKind::Text(t) => t.content.paragraphs.iter().map(|p| p.runs.iter().map(|r| r.text.as_str()).collect::<String>()).collect::<Vec<_>>().join("\n"),
            _ => panic!("pas un texte"),
        }
    }

    fn card() -> Page {
        let mut p = Page::new(A4_PORTRAIT);
        p.elements = vec![
            text(Some("Prénom"), "", true, frame(10.0, 10.0, 80.0, 20.0)),
            text(None, "Joyeux anniversaire {prénom} !", false, frame(10.0, 40.0, 150.0, 20.0)),
            photo_zone(frame(10.0, 70.0, 80.0, 60.0)),
        ];
        p
    }

    #[test]
    fn page_per_student_fills_zone_markers_and_photo() {
        let src = card();
        let out = fill_for(&src, &student("Léa", Some("p1"), true, None));
        assert_eq!(plain(&out.elements[0]), "Léa");
        assert_eq!(plain(&out.elements[1]), "Joyeux anniversaire Léa !");
        assert!(matches!(&out.elements[2].kind, ElementKind::Photo(p) if p.asset.as_deref() == Some("p1")));
        assert!(out.elements.iter().all(|e| src.elements.iter().all(|s| s.id != e.id)));
    }

    #[test]
    fn photo_without_consent_uses_the_symbol() {
        let out = fill_for(&card(), &student("Léa", Some("p1"), false, Some("1F98A")));
        let e = &out.elements[2];
        assert_eq!(e.kind, ElementKind::Image(ImageProps { code: "1F98A".into() }));
        assert_eq!((e.frame.x_mm, e.frame.y_mm, e.frame.w_mm, e.frame.h_mm), (20.0, 70.0, 60.0, 60.0));
        assert!(!out.elements.iter().any(|e| matches!(&e.kind, ElementKind::Photo(p) if p.asset.is_some())));
    }

    #[test]
    fn photo_without_consent_or_symbol_stays_empty() {
        let out = fill_for(&card(), &student("Léa", Some("p1"), false, None));
        assert!(matches!(&out.elements[2].kind, ElementKind::Photo(p) if p.asset.is_none()));
    }

    #[test]
    fn capitals_and_other_fields() {
        let mut p = Page::new(A4_PORTRAIT);
        p.elements = vec![
            text(Some("PRÉNOM"), "", true, frame(0.0, 0.0, 50.0, 10.0)),
            text(Some("Nom"), "", true, frame(0.0, 20.0, 50.0, 10.0)),
            text(Some("Groupe"), "", true, frame(0.0, 40.0, 50.0, 10.0)),
        ];
        let s = Student { group: "Les loups".into(), ..student("Léa", None, false, None) };
        let out = fill_for(&p, &s);
        assert_eq!(out.elements.iter().map(plain).collect::<Vec<_>>(), ["LÉA", "Martin", "Les loups"]);
    }

    #[test]
    fn sheet_fills_blocks_in_reading_order_and_adds_pages() {
        let labels = crate::templates::builtin().into_iter().find(|t| t.info.name == "Étiquettes").unwrap().page;
        let students: Vec<Student> = (1..=25).map(|i| student(&format!("Élève{i}"), None, false, None)).collect();
        let pages = fill_sheet(&labels, &students);
        assert_eq!(pages.len(), 3);
        let at = |p: &Page, x: f64, y_rank: usize| {
            let mut zones: Vec<&Element> = p.elements.iter().filter(|e| e.zone && e.frame.x_mm == x).collect();
            zones.sort_by(|a, b| a.frame.y_mm.total_cmp(&b.frame.y_mm));
            plain(zones[y_rank])
        };
        assert_eq!(at(&pages[0], 10.0, 0), "Élève1");
        assert_eq!(at(&pages[0], 110.0, 0), "Élève2");
        assert_eq!(at(&pages[0], 10.0, 1), "Élève3");
        let filled = pages[2].elements.iter().filter(|e| e.zone && !plain(e).is_empty()).count();
        assert_eq!(filled, 1);
        assert_eq!(pages[2].elements.iter().filter(|e| e.zone).count(), 12);
    }

    #[test]
    fn group_counts_as_one_block() {
        let mut p = Page::new(A4_PORTRAIT);
        let mut a = photo_zone(frame(10.0, 10.0, 40.0, 30.0));
        let mut b = text(Some("Prénom"), "", true, frame(10.0, 42.0, 40.0, 10.0));
        a.group = Some("g".into());
        b.group = Some("g".into());
        p.elements = vec![a, b];
        let pages = fill_sheet(&p, &[student("Léa", Some("p1"), true, None), student("Noé", None, false, None)]);
        assert_eq!(pages.len(), 2);
        assert_eq!(plain(&pages[0].elements[1]), "Léa");
        assert!(matches!(&pages[0].elements[0].kind, ElementKind::Photo(p) if p.asset.as_deref() == Some("p1")));
        assert_eq!(plain(&pages[1].elements[1]), "Noé");
    }

    #[test]
    fn no_students_no_pages() {
        assert!(generate(&card(), &[], FillMode::Sheet).is_empty());
        assert!(generate(&card(), &[], FillMode::PagePerStudent).is_empty());
    }

    #[test]
    fn a_filled_page_is_recognised() {
        let src = card();
        assert!(!already_filled(&src, &["p1".to_string()]));
        let out = fill_for(&src, &student("Léa", Some("p1"), true, None));
        assert!(already_filled(&out, &["p1".to_string()]), "photo d'un élève");
        let out = fill_for(&src, &student("Léa", None, false, None));
        assert!(already_filled(&out, &[]), "zone Prénom déjà écrite");
    }

    #[test]
    fn a_table_gives_one_block_per_row() {
        let mut p = Page::new(A4_PORTRAIT);
        let t = crate::table::add_table(&mut p, frame(10.0, 10.0, 100.0, 60.0), 2, 3);
        let ElementKind::Table(tp) = &p.elements.iter().find(|e| e.id == t).unwrap().kind else { panic!() };
        let first_col: Vec<Id> = (0..3u8).map(|r| tp.cells.iter().find(|c| c.col == 0 && c.row == r).unwrap().element_id.clone()).collect();
        for id in &first_col {
            if let Some(ElementKind::Text(tx)) = p.elements.iter_mut().find(|e| &e.id == id).map(|e| &mut e.kind) {
                tx.content = RichText::plain("{prénom}");
            }
        }
        let students: Vec<Student> = ["Léa", "Noé", "Zoé", "Tom"].iter().map(|n| student(n, None, false, None)).collect();
        let pages = fill_sheet(&p, &students);
        assert_eq!(pages.len(), 2);
        let mut names: Vec<(f64, String)> = pages[0].elements.iter().filter(|e| matches!(e.kind, ElementKind::Text(_))).map(|e| (e.frame.y_mm, plain(e))).filter(|(_, t)| !t.is_empty()).collect();
        names.sort_by(|a, b| a.0.total_cmp(&b.0));
        assert_eq!(names.into_iter().map(|n| n.1).collect::<Vec<_>>(), ["Léa", "Noé", "Zoé"]);
    }

    #[test]
    fn a_page_without_fields_has_none() {
        let mut p = Page::new(A4_PORTRAIT);
        p.elements = vec![text(None, "Bonjour", false, frame(0.0, 0.0, 10.0, 10.0))];
        assert!(!has_fields(&p));
        assert!(has_fields(&card()));
    }
}
