//! Pages pour la classe, à partir d'une page modèle : une page par élève, ou une planche dont les blocs
//! reçoivent les élèves les uns après les autres.
//!
//! Champs : une zone de texte dont l'invite est « Prénom », « PRÉNOM », « Nom », « Groupe », « Âge » ou
//! « Anniversaire » ; les marqueurs `{prénom}`, `{PRÉNOM}`, `{nom}`, `{groupe}`, `{âge}` (« 4 ans ») et
//! `{anniversaire}` (« 12 mars ») dans n'importe quel texte ; une zone photo vide,
//! qui reçoit la photo de l'élève seulement si l'autorisation est donnée, sinon son symbole.

use crate::class::{self, Student};
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

const MARKERS: [&str; 6] = ["{prénom}", "{PRÉNOM}", "{nom}", "{groupe}", "{âge}", "{anniversaire}"];

fn field_value(field: &str, s: &Student) -> Option<String> {
    match field {
        "Prénom" | "{prénom}" => Some(s.first_name.clone()),
        "PRÉNOM" | "{PRÉNOM}" => Some(s.first_name.to_uppercase()),
        "Nom" | "{nom}" => Some(s.last_name.clone()),
        "Groupe" | "{groupe}" => Some(s.group.clone()),
        // Sans date de naissance : vide.
        "Âge" | "{âge}" => Some(s.birth_date.as_deref().and_then(|b| class::age_on(b, class::today())).map(|a| format!("{a} ans")).unwrap_or_default()),
        "Anniversaire" | "{anniversaire}" => Some(s.birth_date.as_deref().and_then(class::birthday_text).unwrap_or_default()),
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

/// Cadre (forme, texte encadré, image) qui entoure entièrement l'élément, le plus petit ; ni champ ni groupé.
fn container_of(page: &Page, i: usize) -> Option<usize> {
    const TOLERANCE_MM: f64 = 0.5;
    let f = page.elements[i].frame;
    let inside = |c: &Frame| {
        f.x_mm >= c.x_mm - TOLERANCE_MM
            && f.y_mm >= c.y_mm - TOLERANCE_MM
            && f.x_mm + f.w_mm <= c.x_mm + c.w_mm + TOLERANCE_MM
            && f.y_mm + f.h_mm <= c.y_mm + c.h_mm + TOLERANCE_MM
    };
    page.elements
        .iter()
        .enumerate()
        .filter(|(j, c)| {
            *j != i
                && c.group.is_none()
                && !is_field(c)
                && c.frame.rotation_deg == 0.0
                && match &c.kind {
                    ElementKind::Shape(s) => !crate::shapes::is_stroke_only(s.shape),
                    ElementKind::Text(t) => t.frame.is_some(),
                    ElementKind::Image(_) | ElementKind::Photo(_) => true,
                    ElementKind::Table(_) => false,
                }
                && inside(&c.frame)
        })
        .min_by(|a, b| (a.1.frame.w_mm * a.1.frame.h_mm).total_cmp(&(b.1.frame.w_mm * b.1.frame.h_mm)))
        .map(|(j, _)| j)
}

/// Clé de bloc d'un élément : son groupe (et sa ligne s'il est la case d'un tableau), sinon le cadre qui
/// l'entoure (une étiquette faite à la main : un rectangle et des textes posés dedans), sinon lui seul.
#[derive(PartialEq)]
enum BlockKey<'a> {
    Group(&'a Id, Option<u8>),
    Container(usize),
    Alone(usize),
}

/// Blocs à remplir de la page, dans l'ordre de lecture. Un bloc reçoit un seul élève, dans tous ses champs
/// (le prénom en capitales, en script et en cursive sur une même étiquette).
fn blocks(page: &Page) -> Vec<Vec<usize>> {
    let mut row_of: std::collections::HashMap<&str, u8> = std::collections::HashMap::new();
    for e in &page.elements {
        if let ElementKind::Table(t) = &e.kind {
            row_of.extend(t.cells.iter().map(|c| (c.element_id.as_str(), c.row)));
        }
    }
    let mut units: Vec<(BlockKey, Vec<usize>)> = Vec::new();
    for (i, e) in page.elements.iter().enumerate() {
        let key = match &e.group {
            Some(g) => BlockKey::Group(g, row_of.get(e.id.as_str()).copied()),
            None if is_field(e) => container_of(page, i).map_or(BlockKey::Alone(i), BlockKey::Container),
            None => BlockKey::Alone(i),
        };
        match units.iter_mut().find(|(u, _)| *u == key) {
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

/// Nombre d'étiquettes (blocs à remplir) de la page.
pub fn block_count(page: &Page) -> usize {
    blocks(page).len()
}

/// Un bloc qui se répète (mêmes champs, même taille à 1 mm près) : une planche d'étiquettes ou de cartes.
/// Sinon (un prénom, une phrase et une photo sur la page), une page par élève.
pub fn suggested_mode(page: &Page) -> FillMode {
    let signature = |members: &Vec<usize>| {
        let mut fields: Vec<String> = members
            .iter()
            .map(|&i| &page.elements[i])
            .filter(|e| is_field(e))
            .map(|e| match &e.kind {
                ElementKind::Text(t) if is_field_zone(e) => format!("zone:{}", t.prompt.as_deref().unwrap_or("")),
                ElementKind::Text(_) => "texte".to_string(),
                _ => "photo".to_string(),
            })
            .collect();
        fields.sort();
        let w = members.iter().map(|&i| page.elements[i].frame.x_mm + page.elements[i].frame.w_mm).fold(f64::MIN, f64::max)
            - members.iter().map(|&i| page.elements[i].frame.x_mm).fold(f64::MAX, f64::min);
        let h = members.iter().map(|&i| page.elements[i].frame.y_mm + page.elements[i].frame.h_mm).fold(f64::MIN, f64::max)
            - members.iter().map(|&i| page.elements[i].frame.y_mm).fold(f64::MAX, f64::min);
        (fields, w.round() as i64, h.round() as i64)
    };
    let sigs: Vec<_> = blocks(page).iter().map(signature).collect();
    let repeated = sigs.iter().enumerate().any(|(i, s)| sigs[i + 1..].contains(s));
    if repeated { FillMode::Sheet } else { FillMode::PagePerStudent }
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
            kind: ElementKind::Photo(PhotoProps { asset: None, crop: Crop::COVER, corner_radius_mm: 0.0, border: None, shadow: false, mirrored: false }),
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
    fn age_and_birthday_markers() {
        let mut p = Page::new(A4_PORTRAIT);
        p.elements = vec![text(None, "{prénom} a {âge}, né le {anniversaire}.", false, frame(0.0, 0.0, 80.0, 10.0))];
        let (y, m, d) = crate::class::today();
        let born = format!("{:04}-{m:02}-{d:02}", y - 4);
        let s = Student { birth_date: Some(born), ..student("Léa", None, false, None) };
        let expected = format!("Léa a 4 ans, né le {}.", crate::class::birthday_text(s.birth_date.as_deref().unwrap()).unwrap());
        assert_eq!(plain(&fill_for(&p, &s).elements[0]), expected);
        // Sans date : les marqueurs s'effacent.
        assert_eq!(plain(&fill_for(&p, &student("Noé", None, false, None)).elements[0]), "Noé a , né le .");
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

    fn styled(prompt: Option<&str>, content: &str, font: &str, f: Frame) -> Element {
        let mut e = text(prompt, content, prompt.is_some(), f);
        if let ElementKind::Text(t) = &mut e.kind {
            t.style.font = font.into();
        }
        e
    }

    fn font_of(e: &Element) -> &str {
        match &e.kind {
            ElementKind::Text(t) => t.content.paragraphs[0].runs[0].font.as_deref().unwrap_or(&t.style.font),
            _ => panic!(),
        }
    }

    #[test]
    fn one_block_writes_the_name_in_each_style() {
        let mut p = Page::new(A4_PORTRAIT);
        let mut zones = vec![
            styled(Some("PRÉNOM"), "", "Andika", frame(10.0, 10.0, 80.0, 10.0)),
            styled(Some("Prénom"), "", "Andika", frame(10.0, 22.0, 80.0, 10.0)),
            styled(Some("Prénom"), "", "Playwrite FR Moderne", frame(10.0, 34.0, 80.0, 10.0)),
        ];
        for z in &mut zones {
            z.group = Some("g".into());
        }
        p.elements = zones;
        let pages = fill_sheet(&p, &[student("Léa", None, false, None), student("Noé", None, false, None)]);
        assert_eq!(pages.len(), 2);
        let e = &pages[0].elements;
        assert_eq!(e.iter().map(plain).collect::<Vec<_>>(), ["LÉA", "Léa", "Léa"]);
        assert_eq!(e.iter().map(font_of).collect::<Vec<_>>(), ["Andika", "Andika", "Playwrite FR Moderne"]);
    }

    #[test]
    fn ungrouped_fields_inside_a_frame_are_one_block() {
        // Deux étiquettes faites à la main : un rectangle et trois textes posés dedans, sans groupe.
        let mut p = Page::new(A4_PORTRAIT);
        for x in [10.0, 110.0] {
            p.elements.push(Element {
                id: new_id(),
                frame: frame(x, 10.0, 90.0, 40.0),
                locked: false,
                zone: false,
                group: None,
                kind: ElementKind::Shape(ShapeProps { shape: ShapeKind::Rect, fill: None, stroke: None, corner_radius_mm: 3.0 }),
            });
            p.elements.push(styled(None, "{PRÉNOM}", "Andika", frame(x + 5.0, 12.0, 80.0, 10.0)));
            p.elements.push(styled(None, "{prénom}", "Andika", frame(x + 5.0, 24.0, 80.0, 10.0)));
            p.elements.push(styled(None, "{prénom}", "Playwrite FR Moderne", frame(x + 5.0, 36.0, 80.0, 10.0)));
        }
        assert_eq!(block_count(&p), 2);
        let pages = fill_sheet(&p, &[student("Léa", None, false, None), student("Noé", None, false, None)]);
        assert_eq!(pages.len(), 1);
        let texts: Vec<String> = pages[0].elements.iter().filter(|e| matches!(e.kind, ElementKind::Text(_))).map(plain).collect();
        assert_eq!(texts, ["LÉA", "Léa", "Léa", "NOÉ", "Noé", "Noé"]);
    }

    #[test]
    fn mode_follows_the_page() {
        assert_eq!(suggested_mode(&card()), FillMode::PagePerStudent);
        let labels = crate::templates::builtin().into_iter().find(|t| t.info.name == "Étiquettes").unwrap().page;
        assert_eq!(suggested_mode(&labels), FillMode::Sheet);
        assert_eq!(block_count(&labels), 12);
    }

    #[test]
    fn three_scripts_template_fills_one_child_per_label() {
        let page = crate::templates::builtin().into_iter().find(|t| t.info.name == "Étiquettes 3 écritures").unwrap().page;
        let pages = fill_sheet(&page, &[student("Léa", None, false, None)]);
        let filled: Vec<String> = pages[0].elements.iter().filter(|e| e.zone && matches!(e.kind, ElementKind::Text(_))).map(plain).filter(|t| !t.is_empty()).collect();
        assert_eq!(filled, ["LÉA", "Léa", "Léa"]);
    }

    #[test]
    fn a_page_without_fields_has_none() {
        let mut p = Page::new(A4_PORTRAIT);
        p.elements = vec![text(None, "Bonjour", false, frame(0.0, 0.0, 10.0, 10.0))];
        assert!(!has_fields(&p));
        assert!(has_fields(&card()));
    }
}
