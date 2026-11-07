//! Pages pour la classe, à partir d'une page modèle : une page par élève, ou une planche dont les blocs
//! reçoivent les élèves les uns après les autres.
//!
//! Champs : les mots entre accolades de `crate::fields` (`{prenom}`, `{PRENOM}`, `{nom}`, `{groupe}`, `{age}`,
//! `{anniversaire}`), dans n'importe quel texte ; une case photo vide d'un modèle, qui reçoit la photo de
//! l'élève seulement si l'autorisation est donnée, sinon son symbole. Un texte qui n'est que `{PRENOM_1}`,
//! `{PRENOM_2}`… reçoit une lettre du prénom (lettres mobiles) ; celles en trop disparaissent. Une zone
//! dont l'invite est « Prénom » n'est plus remplie : `fields::upgrade` la convertit à l'ouverture.
//!
//! Champs de toute la classe : `{anniversaires_janvier}` … `{anniversaires_decembre}`, la liste des enfants
//! nés ce mois-là. Une page qui n'a que ceux-là est remplie une fois, pour toute la classe.

use crate::class::{self, Student};
use crate::fields;
use crate::model::*;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, TS)]
#[ts(export)]
pub enum FillMode {
    PagePerStudent,
    Sheet,
    /// Une seule page, pour toute la classe (affiche des anniversaires).
    WholeClass,
}

/// Écart vertical (mm) en dessous duquel deux blocs sont sur la même rangée.
const ROW_TOLERANCE_MM: f64 = 3.0;

/// Le texte, ses champs remplacés par `value` ; un champ sans valeur reste écrit tel quel.
fn replace_fields(text: &str, value: impl Fn(&str) -> Option<String>) -> String {
    fields::split(text)
        .into_iter()
        .map(|p| match p {
            fields::Piece::Text(t) => t.to_string(),
            fields::Piece::Field(f) => value(f).unwrap_or_else(|| format!("{{{f}}}")),
        })
        .collect()
}

/// Une case de lettre mobile : un texte qui ne contient qu'un champ lettre (`{PRENOM_3}`, `{prenom_3}`).
fn is_letter(e: &Element) -> bool {
    let ElementKind::Text(t) = &e.kind else { return false };
    let pieces: Vec<fields::Piece> = t.content.paragraphs.iter().flat_map(|p| &p.runs).flat_map(|r| fields::split(&r.text)).filter(|p| !matches!(p, fields::Piece::Text(x) if x.trim().is_empty())).collect();
    matches!(pieces[..], [fields::Piece::Field(f)] if fields::letter_index(f).is_some())
}

/// Les enfants nés ce mois-là, par jour : « Léa (12), Noé (30) ».
fn birthdays_in(month: usize, students: &[Student]) -> String {
    let mut born: Vec<(u32, &str)> = students
        .iter()
        .filter_map(|s| class::ymd(s.birth_date.as_deref()?).filter(|d| d.1 as usize == month).map(|d| (d.2, s.first_name.as_str())))
        .collect();
    born.sort();
    born.iter().map(|(d, n)| format!("{n} ({d})")).collect::<Vec<_>>().join(", ")
}

fn has_class_markers(page: &Page) -> bool {
    page.elements.iter().any(|e| matches!(&e.kind, ElementKind::Text(t) if fields::has_class_fields_text(t)))
}

/// La page remplie une fois pour toute la classe : les anniversaires de chaque mois.
pub fn fill_class(page: &Page, students: &[Student]) -> Page {
    let mut out = page.clone();
    for e in &mut out.elements {
        if let ElementKind::Text(t) = &mut e.kind {
            for r in t.content.paragraphs.iter_mut().flat_map(|p| &mut p.runs) {
                r.text = replace_fields(&r.text, |f| fields::month_of(f).map(|m| birthdays_in(m, students)));
            }
        }
    }
    renew_ids(&mut out);
    out
}

/// Un champ : un texte qui en contient, ou une case photo vide d'un modèle.
fn is_field(e: &Element) -> bool {
    match &e.kind {
        ElementKind::Text(t) => fields::has_fields_text(t),
        ElementKind::Photo(p) => e.zone && p.asset.is_none(),
        _ => false,
    }
}

pub fn has_fields(page: &Page) -> bool {
    page.elements.iter().any(is_field) || has_class_markers(page)
}

/// Les lettres mobiles en trop (prénom plus court que la rangée de cases) disparaissent, et chaque rangée
/// de lettres restante est recentrée sur la page. `letters` : les cases de lettres de la page modèle.
fn drop_unused_letters(page: &mut Page, letters: &[Id]) {
    let is_letter = |e: &Element| letters.contains(&e.id);
    let before = page.elements.iter().filter(|e| is_letter(e)).count();
    page.elements.retain(|e| !(is_letter(e) && matches!(&e.kind, ElementKind::Text(t) if t.content.is_blank())));
    if page.elements.iter().filter(|e| is_letter(e)).count() == before {
        return;
    }
    let mut rows: Vec<f64> = page.elements.iter().filter(|e| is_letter(e)).map(|e| e.frame.y_mm).collect();
    rows.sort_by(f64::total_cmp);
    rows.dedup_by(|a, b| (*a - *b).abs() < ROW_TOLERANCE_MM);
    for y in rows {
        let in_row = |e: &Element| is_letter(e) && (e.frame.y_mm - y).abs() < ROW_TOLERANCE_MM;
        let left = page.elements.iter().filter(|e| in_row(e)).map(|e| e.frame.x_mm).fold(f64::INFINITY, f64::min);
        let right = page.elements.iter().filter(|e| in_row(e)).map(|e| e.frame.x_mm + e.frame.w_mm).fold(f64::NEG_INFINITY, f64::max);
        let shift = (page.size.width_mm - (right - left)) / 2.0 - left;
        for e in page.elements.iter_mut().filter(|e| in_row(e)) {
            e.frame.x_mm = ((e.frame.x_mm + shift) * 10.0).round() / 10.0;
        }
    }
}

fn letter_ids(page: &Page) -> Vec<Id> {
    page.elements.iter().filter(|e| is_letter(e)).map(|e| e.id.clone()).collect()
}

/// Page déjà remplie pour un élève : une photo d'élève de la classe y est. (Un prénom écrit ne laisse plus
/// de champ, et une page sans champ ne se remplit pas.)
pub fn already_filled(page: &Page, class_photos: &[Id]) -> bool {
    page.elements.iter().any(|e| matches!(&e.kind, ElementKind::Photo(p) if p.asset.as_ref().is_some_and(|a| class_photos.contains(a))))
}

/// Remplit les champs de l'élément pour l'élève ; `None` : bloc sans élève, ses champs sont vidés.
fn fill_element(e: &mut Element, s: Option<&Student>) {
    let field = is_field(e);
    match &mut e.kind {
        ElementKind::Text(t) if field => match s {
            None => t.content = RichText::default(),
            Some(s) => {
                for r in t.content.paragraphs.iter_mut().flat_map(|p| &mut p.runs) {
                    r.text = replace_fields(&r.text, |f| fields::value(f, s));
                }
            }
        },
        ElementKind::Photo(p) if field => {
            // Bloc sans élève : la case reste vide et n'est plus à remplir, comme ses textes (vidés de leurs
            // champs) ; sinon un nouveau remplissage de la planche serait accepté.
            let Some(s) = s else {
                e.zone = false;
                return;
            };
            if s.photo_allowed && let Some(photo) = &s.photo {
                p.asset = Some(photo.clone());
                p.crop = Crop::COVER;
            } else if let Some(code) = &s.symbol {
                // Sans autorisation : le symbole de l'enfant, carré et centré dans la zone.
                let f = e.frame;
                let side = f.w_mm.min(f.h_mm);
                e.frame = Frame { x_mm: f.x_mm + (f.w_mm - side) / 2.0, y_mm: f.y_mm + (f.h_mm - side) / 2.0, w_mm: side, h_mm: side, ..f };
                e.kind = ElementKind::Image(ImageProps { code: code.clone(), outline: None });
                e.zone = false;
            } else {
                // Ni photo permise ni symbole : la case reste vide, mais la page est faite pour cet élève.
                // Elle n'est plus à remplir, sinon un nouveau remplissage y mettrait la photo d'un autre.
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
    drop_unused_letters(&mut out, &letter_ids(page));
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
    let letters = letter_ids(page);
    students
        .chunks(blocks.len())
        .map(|chunk| {
            let mut out = page.clone();
            for (b, members) in blocks.iter().enumerate() {
                for &i in members {
                    fill_element(&mut out.elements[i], chunk.get(b));
                }
            }
            drop_unused_letters(&mut out, &letters);
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
    if !page.elements.iter().any(is_field) && has_class_markers(page) {
        return FillMode::WholeClass;
    }
    // Les lettres mobiles sont celles d'un seul prénom : une page par enfant.
    if page.elements.iter().any(is_letter) {
        return FillMode::PagePerStudent;
    }
    let signature = |members: &Vec<usize>| {
        let mut fields: Vec<String> = members
            .iter()
            .map(|&i| &page.elements[i])
            .filter(|e| is_field(e))
            .map(|e| match &e.kind {
                ElementKind::Text(t) => {
                    let mut keys: Vec<&str> = t.content.paragraphs.iter().flat_map(|p| &p.runs).flat_map(|r| fields::split(&r.text)).filter_map(|p| match p {
                        fields::Piece::Field(f) => Some(f),
                        fields::Piece::Text(_) => None,
                    }).collect();
                    keys.sort();
                    format!("texte:{}", keys.join(","))
                }
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
        FillMode::WholeClass => vec![fill_class(page, students)],
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
            subgroups: Vec::new(),
            flip_h: false, flip_v: false,
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
            subgroups: Vec::new(),
            flip_h: false, flip_v: false,
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
            text(None, "{prenom}", false, frame(10.0, 10.0, 80.0, 20.0)),
            text(None, "Joyeux anniversaire {prenom} !", false, frame(10.0, 40.0, 150.0, 20.0)),
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
    fn a_prompt_zone_is_no_longer_filled() {
        let mut p = Page::new(A4_PORTRAIT);
        p.elements = vec![text(Some("Prénom"), "", true, frame(0.0, 0.0, 50.0, 10.0))];
        assert!(!has_fields(&p));
        assert_eq!(plain(&fill_for(&p, &student("Léa", None, false, None)).elements[0]), "");
    }

    #[test]
    fn old_accented_markers_still_fill() {
        let mut p = Page::new(A4_PORTRAIT);
        p.elements = vec![text(None, "{prénom} {PRÉNOM}", false, frame(0.0, 0.0, 50.0, 10.0))];
        assert_eq!(plain(&fill_for(&p, &student("Léa", None, false, None)).elements[0]), "Léa LÉA");
    }

    #[test]
    fn a_field_split_across_runs_is_plain_text() {
        let mut p = Page::new(A4_PORTRAIT);
        let mut e = text(None, "", false, frame(0.0, 0.0, 50.0, 10.0));
        if let ElementKind::Text(t) = &mut e.kind {
            t.content = RichText { paragraphs: vec![Paragraph { align: None, runs: vec![Run { bold: true, ..Run::plain("{pre") }, Run::plain("nom}")] }] };
        }
        p.elements = vec![e];
        assert!(!has_fields(&p));
        assert_eq!(plain(&fill_for(&p, &student("Léa", None, false, None)).elements[0]), "{prenom}");
    }

    #[test]
    fn a_sheet_of_field_labels_is_a_sheet() {
        let mut p = Page::new(A4_PORTRAIT);
        for (x, y) in [(10.0, 10.0), (110.0, 10.0), (10.0, 60.0), (110.0, 60.0)] {
            let mut r = text(None, "", false, frame(x, y, 90.0, 40.0));
            r.kind = ElementKind::Shape(ShapeProps { shape: ShapeKind::Rect, fill: None, stroke: None, corner_radius_mm: 0.0, path: None });
            p.elements.push(r);
            p.elements.push(text(None, "{prenom}", false, frame(x + 5.0, y + 5.0, 80.0, 15.0)));
        }
        assert_eq!((suggested_mode(&p), block_count(&p)), (FillMode::Sheet, 4));
    }

    #[test]
    fn photo_without_consent_uses_the_symbol() {
        let out = fill_for(&card(), &student("Léa", Some("p1"), false, Some("1F98A")));
        let e = &out.elements[2];
        assert_eq!(e.kind, ElementKind::Image(ImageProps { code: "1F98A".into(), outline: None }));
        assert_eq!((e.frame.x_mm, e.frame.y_mm, e.frame.w_mm, e.frame.h_mm), (20.0, 70.0, 60.0, 60.0));
        assert!(!out.elements.iter().any(|e| matches!(&e.kind, ElementKind::Photo(p) if p.asset.is_some())));
    }

    #[test]
    fn photo_without_consent_or_symbol_stays_empty() {
        let out = fill_for(&card(), &student("Léa", Some("p1"), false, None));
        assert!(matches!(&out.elements[2].kind, ElementKind::Photo(p) if p.asset.is_none()));
        // La page est faite pour Léa : elle n'a plus rien à remplir, pas même sa case photo restée vide.
        assert!(!has_fields(&out));
    }

    #[test]
    fn capitals_and_other_fields() {
        let mut p = Page::new(A4_PORTRAIT);
        p.elements = vec![
            text(None, "{PRENOM}", false, frame(0.0, 0.0, 50.0, 10.0)),
            text(None, "{nom}", false, frame(0.0, 20.0, 50.0, 10.0)),
            text(None, "{groupe}", false, frame(0.0, 40.0, 50.0, 10.0)),
        ];
        let s = Student { group: "Les loups".into(), ..student("Léa", None, false, None) };
        let out = fill_for(&p, &s);
        assert_eq!(out.elements.iter().map(plain).collect::<Vec<_>>(), ["LÉA", "Martin", "Les loups"]);
    }

    #[test]
    fn moving_letters_take_one_letter_each_and_extra_boxes_go() {
        let mut p = Page::new(A4_PORTRAIT);
        p.elements = (1..=5).map(|i| text(None, &format!("{{PRENOM_{i}}}"), false, frame(i as f64 * 20.0, 0.0, 18.0, 20.0))).collect();
        p.elements.push(text(None, "{prenom_2}", false, frame(0.0, 30.0, 18.0, 20.0)));
        let out = fill_for(&p, &student("Léa", None, false, None));
        assert_eq!(out.elements.iter().map(plain).collect::<Vec<_>>(), ["L", "É", "A", "é"]);
        // Les trois lettres de la première rangée, recentrées : 20 + 20 + 18 mm de large au milieu de la page.
        assert_eq!(out.elements[0].frame.x_mm, (210.0 - 58.0) / 2.0);
    }

    #[test]
    fn birthdays_poster_is_filled_once_for_the_class() {
        let mut p = Page::new(A4_PORTRAIT);
        p.elements = vec![text(None, "Mars : {anniversaires_mars}", false, frame(0.0, 0.0, 80.0, 10.0)), text(None, "Juin : {anniversaires_juin}", false, frame(0.0, 20.0, 80.0, 10.0))];
        assert!(has_fields(&p));
        assert_eq!(suggested_mode(&p), FillMode::WholeClass);
        let born = |name: &str, date: &str| Student { birth_date: Some(date.into()), ..student(name, None, false, None) };
        let class = [born("Noé", "2022-03-30"), born("Léa", "2022-03-12"), born("Anna", "2021-11-02")];
        let pages = generate(&p, &class, FillMode::WholeClass);
        assert_eq!(pages.len(), 1);
        assert_eq!(pages[0].elements.iter().map(plain).collect::<Vec<_>>(), ["Mars : Léa (12), Noé (30)", "Juin : "]);
    }

    #[test]
    fn age_and_birthday_markers() {
        let mut p = Page::new(A4_PORTRAIT);
        p.elements = vec![text(None, "{prenom} a {age}, né le {anniversaire}.", false, frame(0.0, 0.0, 80.0, 10.0))];
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
            let mut zones: Vec<&Element> = p.elements.iter().filter(|e| matches!(e.kind, ElementKind::Text(_)) && e.frame.x_mm == x).collect();
            zones.sort_by(|a, b| a.frame.y_mm.total_cmp(&b.frame.y_mm));
            plain(zones[y_rank])
        };
        assert_eq!(at(&pages[0], 10.0, 0), "Élève1");
        assert_eq!(at(&pages[0], 110.0, 0), "Élève2");
        assert_eq!(at(&pages[0], 10.0, 1), "Élève3");
        let texts = || pages[2].elements.iter().filter(|e| matches!(e.kind, ElementKind::Text(_)));
        assert_eq!(texts().filter(|e| !plain(e).is_empty()).count(), 1);
        assert_eq!(texts().count(), 12);
    }

    #[test]
    fn group_counts_as_one_block() {
        let mut p = Page::new(A4_PORTRAIT);
        let mut a = photo_zone(frame(10.0, 10.0, 40.0, 30.0));
        let mut b = text(None, "{prenom}", false, frame(10.0, 42.0, 40.0, 10.0));
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
    fn a_filled_sheet_has_nothing_left_to_fill() {
        // Deux étiquettes (photo et prénom), un seul élève sans photo ni symbole : la seconde reste vide.
        let mut p = Page::new(A4_PORTRAIT);
        for (x, g) in [(10.0, "a"), (110.0, "b")] {
            let mut ph = photo_zone(frame(x, 10.0, 40.0, 30.0));
            let mut t = text(None, "{prenom}", false, frame(x, 42.0, 40.0, 10.0));
            ph.group = Some(g.into());
            t.group = Some(g.into());
            p.elements.extend([ph, t]);
        }
        let pages = fill_sheet(&p, &[student("Léa", None, false, None)]);
        assert_eq!(pages.len(), 1);
        assert!(!has_fields(&pages[0]), "la planche remplie ne se remplit pas une seconde fois");
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
        // Rempli, il n'a plus de champ : rien à remplir de nouveau.
        let out = fill_for(&src, &student("Léa", None, false, None));
        assert!(!has_fields(&out) || out.elements.iter().all(|e| !matches!(&e.kind, ElementKind::Text(t) if crate::fields::has_fields_text(t))));
    }

    #[test]
    fn a_table_gives_one_block_per_row() {
        let mut p = Page::new(A4_PORTRAIT);
        let t = crate::table::add_table(&mut p, frame(10.0, 10.0, 100.0, 60.0), 2, 3);
        let ElementKind::Table(tp) = &p.elements.iter().find(|e| e.id == t).unwrap().kind else { panic!() };
        let first_col: Vec<Id> = (0..3u8).map(|r| tp.cells.iter().find(|c| c.col == 0 && c.row == r).unwrap().element_id.clone()).collect();
        for id in &first_col {
            if let Some(ElementKind::Text(tx)) = p.elements.iter_mut().find(|e| &e.id == id).map(|e| &mut e.kind) {
                tx.content = RichText::plain("{prenom}");
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
            styled(None, "{PRENOM}", "Andika", frame(10.0, 10.0, 80.0, 10.0)),
            styled(None, "{prenom}", "Andika", frame(10.0, 22.0, 80.0, 10.0)),
            styled(None, "{prenom}", "Playwrite FR Moderne", frame(10.0, 34.0, 80.0, 10.0)),
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
                kind: ElementKind::Shape(ShapeProps { shape: ShapeKind::Rect, fill: None, stroke: None, corner_radius_mm: 3.0, path: None }),
                subgroups: Vec::new(),
                flip_h: false, flip_v: false,
            });
            p.elements.push(styled(None, "{PRENOM}", "Andika", frame(x + 5.0, 12.0, 80.0, 10.0)));
            p.elements.push(styled(None, "{prenom}", "Andika", frame(x + 5.0, 24.0, 80.0, 10.0)));
            p.elements.push(styled(None, "{prenom}", "Playwrite FR Moderne", frame(x + 5.0, 36.0, 80.0, 10.0)));
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
        let filled: Vec<String> = pages[0].elements.iter().filter(|e| matches!(e.kind, ElementKind::Text(_))).map(plain).filter(|t| !t.is_empty()).collect();
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
