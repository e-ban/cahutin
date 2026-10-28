//! Exemples de pages fournis avec l'application : ils montrent groupes, images, frises, cadres,
//! formes, polices et fonds, sans aucune photo (les emplacements de photos sont des zones vides).

use crate::model::*;

#[derive(serde::Serialize, Clone, Debug, PartialEq, ts_rs::TS)]
#[ts(export)]
pub struct ExampleInfo {
    pub id: String,
    pub name: String,
}

const EXAMPLES: [(&str, &str); 5] = [
    ("ferme", "Sortie à la ferme"),
    ("anniversaire", "Joyeux anniversaire"),
    ("meteo", "La météo de la semaine"),
    ("prenom", "J'écris mon prénom"),
    ("saisons", "Les saisons"),
];

pub fn list() -> Vec<ExampleInfo> {
    EXAMPLES.iter().map(|(id, name)| ExampleInfo { id: (*id).into(), name: (*name).into() }).collect()
}

/// Copie neuve de l'exemple (nouveaux identifiants à chaque appel) ; le document est sans titre.
pub fn document(id: &str) -> Option<Document> {
    let page = match id {
        "ferme" => farm(),
        "anniversaire" => birthday(),
        "meteo" => weather(),
        "prenom" => first_name(),
        "saisons" => seasons(),
        _ => return None,
    };
    let mut doc = Document::new("");
    doc.pages = vec![page];
    Some(doc)
}

pub(crate) const TITLE_FONT: &str = "Playwrite FR Moderne";

/// Hauteur de cadre suffisante pour une ligne de cursive (interligne naturel de Playwrite et marges).
pub(crate) fn cursive_height(size_pt: f64) -> f64 {
    (size_pt * 0.85 + 4.0).ceil()
}

pub(crate) fn frame(x: f64, y: f64, w: f64, h: f64) -> Frame {
    Frame { x_mm: x, y_mm: y, w_mm: w, h_mm: h, rotation_deg: 0.0 }
}

pub(crate) fn el(f: Frame, kind: ElementKind) -> Element {
    Element { id: new_id(), frame: f, locked: false, zone: false, group: None, kind }
}

fn text(f: Frame, content: &str, font: &str, size_pt: f64, color: &str, align: Align) -> Element {
    let style = TextStyle { font: font.into(), size_pt, color: color.into(), align, valign: None, padding_mm: None };
    el(f, ElementKind::Text(TextProps { content: RichText::plain(content), style, prompt: None, frame: None }))
}

/// Zone de texte à remplir, avec son invite.
pub(crate) fn text_zone(f: Frame, prompt: &str, font: &str, size_pt: f64) -> Element {
    let style = TextStyle { font: font.into(), size_pt, color: "#222222".into(), align: Align::Left, valign: None, padding_mm: None };
    Element { zone: true, ..el(f, ElementKind::Text(TextProps { content: RichText::default(), style, prompt: Some(prompt.into()), frame: None })) }
}

fn title(content: &str, y: f64, size_pt: f64, color: &str) -> Element {
    text(frame(15.0, y, 180.0, cursive_height(size_pt)), content, TITLE_FONT, size_pt, color, Align::Center)
}

fn picto(code: &str, x: f64, y: f64, size: f64) -> Element {
    el(frame(x, y, size, size), ElementKind::Image(ImageProps { code: code.into() }))
}

pub(crate) fn shape(kind: ShapeKind, f: Frame, fill: Option<&str>, stroke: Option<(f64, &str)>, corner_radius_mm: f64) -> Element {
    el(
        f,
        ElementKind::Shape(ShapeProps {
            shape: kind,
            fill: fill.map(String::from),
            stroke: stroke.map(|(width_mm, color)| Stroke { width_mm, color: color.into(), style: None }),
            corner_radius_mm,
        }),
    )
}

pub(crate) fn photo_zone(f: Frame) -> Element {
    Element { zone: true, ..el(f, ElementKind::Photo(PhotoProps { asset: None, crop: Crop::COVER, corner_radius_mm: 3.0, border: None, shadow: false })) }
}

/// Même identifiant de groupe pour tous.
pub(crate) fn grouped(mut els: Vec<Element>) -> Vec<Element> {
    let g = new_id();
    for e in &mut els {
        e.group = Some(g.clone());
    }
    els
}

/// Photo à déposer et sa légende, groupées.
pub(crate) fn photo_with_caption(x: f64, y: f64, w: f64, h: f64) -> Vec<Element> {
    let mut caption = text_zone(frame(x, y + h + 2.0, w, 10.0), "Légende…", "Andika", 12.0);
    if let ElementKind::Text(t) = &mut caption.kind {
        t.style.align = Align::Center;
    }
    grouped(vec![photo_zone(frame(x, y, w, h)), caption])
}

/// Frise d'images en haut (ou en bas) de la page, codes en alternance, groupée.
fn frieze(codes: &[&str], y: f64, size: f64) -> Vec<Element> {
    let (start, len) = (8.0, 194.0);
    let n = ((len + 3.0) / (size + 3.0)).floor() as usize;
    let step = (len - size) / (n as f64 - 1.0);
    grouped((0..n).map(|i| picto(codes[i % codes.len()], ((start + i as f64 * step) * 10.0).round() / 10.0, y, size)).collect())
}

fn page(color: &str, pattern: Option<Pattern>, elements: Vec<Element>) -> Page {
    Page { elements, background: Some(Background { color: color.into(), pattern }), ..Page::new(A4_PORTRAIT) }
}

/// Cahier de vie : kraft, cadre arrondi, frise d'animaux, deux photos légendées, bulle de parole.
fn farm() -> Page {
    let mut els = vec![shape(ShapeKind::Rect, frame(6.0, 6.0, 198.0, 285.0), None, Some((1.0, "#7a5a2e")), 8.0)];
    els.extend(frieze(&["1F404", "1F416", "1F414", "1F410", "1F411", "1F69C"], 10.0, 14.0));
    els.push(title("Notre sortie à la ferme", 27.0, 26.0, "#5b3b12"));
    els.extend(photo_with_caption(18.0, 56.0, 84.0, 62.0));
    els.extend(photo_with_caption(108.0, 56.0, 84.0, 62.0));
    els.extend(grouped(vec![
        shape(ShapeKind::Bubble, frame(18.0, 142.0, 120.0, 52.0), Some("#ffffff"), Some((0.6, "#5b3b12")), 0.0),
        text(frame(26.0, 150.0, 104.0, 26.0), "J'ai donné à manger aux chèvres !", "Andika", 16.0, "#222222", Align::Center),
    ]));
    els.push(picto("1F410", 148.0, 145.0, 44.0));
    els.push(text_zone(frame(18.0, 210.0, 174.0, 62.0), "Ce que nous avons appris…", "Andika", 14.0));
    page(crate::background::KRAFT, None, els)
}

/// Anniversaire : frise de ballons, gâteau et étoiles groupés avec le prénom à compléter.
fn birthday() -> Page {
    let mut els = frieze(&["1F388", "1F389", "1F381"], 8.0, 16.0);
    els.extend(frieze(&["1F381", "1F388", "1F389"], 273.0, 16.0));
    els.push(title("Joyeux anniversaire", 30.0, 30.0, "#c0392b"));
    els.extend(grouped(vec![
        picto("1F382", 70.0, 66.0, 70.0),
        picto("2B50", 40.0, 70.0, 22.0),
        picto("2B50", 148.0, 70.0, 22.0),
        picto("2B50", 52.0, 118.0, 14.0),
        picto("2B50", 144.0, 118.0, 14.0),
        text_zone(frame(35.0, 142.0, 140.0, 20.0), "Prénom", "Atkinson Hyperlegible", 28.0),
    ]));
    els.push(text(frame(35.0, 166.0, 140.0, 14.0), "a fêté son anniversaire en classe !", "Atkinson Hyperlegible", 16.0, "#444444", Align::Center));
    els.push(shape(ShapeKind::Heart, frame(92.0, 190.0, 26.0, 26.0), Some("#e74c3c"), None, 0.0));
    els.push(text_zone(frame(25.0, 222.0, 160.0, 40.0), "Ce que nous avons partagé…", "Atkinson Hyperlegible", 14.0));
    page("#fff6d6", None, els)
}

/// Météo : petits carreaux, cinq jours (nom, image, légende groupés), une flèche « aujourd'hui ».
fn weather() -> Page {
    let mut els = vec![title("La météo de la semaine", 10.0, 26.0, "#1f5fa8")];
    let days = [("Lundi", "2600"), ("Mardi", "2601"), ("Mercredi", "1F327"), ("Jeudi", "2744"), ("Vendredi", "1F308")];
    for (i, (day, code)) in days.iter().enumerate() {
        let x = 10.0 + i as f64 * 38.5;
        els.extend(grouped(vec![
            shape(ShapeKind::Rect, frame(x, 62.0, 36.0, 92.0), Some("#ffffff"), Some((0.5, "#1f5fa8")), 4.0),
            text(frame(x, 66.0, 36.0, 10.0), day, "Andika", 13.0, "#1f5fa8", Align::Center),
            picto(code, x + 4.0, 80.0, 28.0),
            text_zone(frame(x + 2.0, 114.0, 32.0, 36.0), "Il fait…", "Andika", 11.0),
        ]));
    }
    els.extend(grouped(vec![
        text(frame(8.0, 38.0, 40.0, 10.0), "Aujourd'hui", "Andika", 12.0, "#c0392b", Align::Center),
        shape(ShapeKind::Arrow, Frame { rotation_deg: 90.0, ..frame(22.0, 52.0, 12.0, 6.0) }, None, Some((1.0, "#c0392b")), 0.0),
    ]));
    els.push(picto("2614", 150.0, 170.0, 40.0));
    els.push(text_zone(frame(15.0, 170.0, 130.0, 60.0), "Le temps qu'il a fait cette semaine…", "Andika", 14.0));
    page("#ffffff", Some(Pattern::SmallSquares), els)
}

/// Écriture : Seyès, modèle de prénom en cursive à repasser, lignes pour écrire.
fn first_name() -> Page {
    let mut els = vec![text(frame(30.0, 6.0, 170.0, cursive_height(24.0)), "J'écris mon prénom", TITLE_FONT, 24.0, "#333333", Align::Left)];
    els.extend(grouped(vec![picto("1F9D2", 30.0, 34.0, 36.0), picto("270F", 70.0, 44.0, 20.0)]));
    els.push(text(frame(100.0, 34.0, 100.0, cursive_height(48.0)), "Léa", TITLE_FONT, 48.0, "#a0a0a0", Align::Left));
    for i in 0..4 {
        els.push(text_zone(frame(30.0, 90.0 + i as f64 * 34.0, 170.0, cursive_height(32.0)), "J'écris ici…", TITLE_FONT, 32.0));
    }
    els.push(picto("2B50", 176.0, 236.0, 20.0));
    page("#ffffff", Some(Pattern::Seyes), els)
}

/// Saisons : cadre double, quatre images titrées groupées, formes nuage et cœur.
fn seasons() -> Page {
    let mut els = grouped(vec![
        shape(ShapeKind::Rect, frame(8.0, 8.0, 194.0, 281.0), None, Some((0.5, "#2e7d32")), 0.0),
        shape(ShapeKind::Rect, frame(11.0, 11.0, 188.0, 275.0), None, Some((1.5, "#2e7d32")), 0.0),
    ]);
    els.push(title("Les saisons", 16.0, 28.0, "#2e7d32"));
    let seasons = [("Printemps", "1F337", 22.0, 50.0), ("Été", "1F3D6", 112.0, 50.0), ("Automne", "1F341", 22.0, 150.0), ("Hiver", "2603", 112.0, 150.0)];
    for (name, code, x, y) in seasons {
        els.extend(grouped(vec![
            picto(code, x + 18.0, y, 40.0),
            text(frame(x, y + 42.0, 76.0, cursive_height(18.0)), name, TITLE_FONT, 18.0, "#2e7d32", Align::Center),
            text_zone(frame(x, y + 64.0, 76.0, 26.0), "Ce que j'aime…", "Andika", 12.0),
        ]));
    }
    els.push(shape(ShapeKind::Cloud, frame(150.0, 252.0, 40.0, 24.0), Some("#e3f2fd"), Some((0.4, "#90a4ae")), 0.0));
    els.push(shape(ShapeKind::Heart, frame(24.0, 254.0, 20.0, 20.0), Some("#f48fb1"), None, 0.0));
    page("#f6fbf4", None, els)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fonts::FontSet;

    fn all() -> Vec<Document> {
        list().iter().map(|i| document(&i.id).unwrap()).collect()
    }

    fn elements(d: &Document) -> impl Iterator<Item = &Element> {
        d.pages.iter().flat_map(|p| &p.elements)
    }

    #[test]
    fn five_examples_with_distinct_ids() {
        let ids: Vec<String> = list().into_iter().map(|i| i.id).collect();
        assert_eq!(ids, ["ferme", "anniversaire", "meteo", "prenom", "saisons"]);
        assert!(document("inconnu").is_none());
    }

    #[test]
    fn examples_use_groups_pictures_and_backgrounds() {
        for d in all() {
            assert!(elements(&d).any(|e| e.group.is_some()), "{} : aucun groupe", d.title);
            assert!(elements(&d).any(|e| matches!(e.kind, ElementKind::Image(_))), "{} : aucune image", d.title);
            assert!(d.pages.iter().all(|p| p.background.is_some()), "{} : pas de fond", d.title);
            assert!(d.assets.is_empty() && elements(&d).all(|e| !matches!(&e.kind, ElementKind::Photo(p) if p.asset.is_some())), "{} : photo", d.title);
        }
    }

    #[test]
    fn examples_only_use_known_pictures_and_fonts() {
        let fonts = FontSet::builtin();
        for d in all() {
            for e in elements(&d) {
                match &e.kind {
                    ElementKind::Image(i) => assert!(crate::picto::svg(&i.code).is_some(), "{} : image {}", d.title, i.code),
                    ElementKind::Text(t) => {
                        assert!(fonts.face(&t.style.font, false, false).is_some(), "{} : police {}", d.title, t.style.font);
                    }
                    _ => {}
                }
                assert!(e.frame.x_mm >= 0.0 && e.frame.y_mm >= 0.0 && e.frame.x_mm + e.frame.w_mm <= 210.5 && e.frame.y_mm + e.frame.h_mm <= 297.5, "{} : élément hors page", d.title);
            }
        }
    }

    #[test]
    fn example_documents_are_fresh_copies() {
        let (a, b) = (document("ferme").unwrap(), document("ferme").unwrap());
        assert_ne!(a.pages[0].id, b.pages[0].id);
        assert_ne!(a.pages[0].elements[0].id, b.pages[0].elements[0].id);
        assert!(a.title.is_empty() || a.title == "Notre sortie à la ferme");
    }

    /// La cursive a une grande hauteur de ligne : un cadre trop bas affiche « Texte trop long ».
    #[test]
    fn cursive_texts_have_room_for_their_line() {
        for d in all() {
            for e in elements(&d) {
                if let ElementKind::Text(t) = &e.kind
                    && t.style.font == TITLE_FONT
                {
                    assert!(e.frame.h_mm >= cursive_height(t.style.size_pt), "{} : {:?} trop bas ({} mm)", d.title, t.content, e.frame.h_mm);
                }
            }
        }
    }

    #[test]
    fn example_captions_are_centred_like_new_ones() {
        let d = document("ferme").unwrap();
        let captions: Vec<&TextProps> = elements(&d).filter_map(|e| match &e.kind {
            ElementKind::Text(t) if t.prompt.as_deref() == Some("Légende…") => Some(t),
            _ => None,
        }).collect();
        assert_eq!(captions.len(), 2);
        assert!(captions.iter().all(|t| t.style.align == Align::Center));
    }

    /// Boîte englobante d'un cadre tourné.
    fn bounds(f: &Frame) -> (f64, f64, f64, f64) {
        let (s, c) = f.rotation_deg.to_radians().sin_cos();
        let (cx, cy) = (f.x_mm + f.w_mm / 2.0, f.y_mm + f.h_mm / 2.0);
        let (hw, hh) = ((f.w_mm / 2.0 * c).abs() + (f.h_mm / 2.0 * s).abs(), (f.w_mm / 2.0 * s).abs() + (f.h_mm / 2.0 * c).abs());
        (cx - hw, cy - hh, cx + hw, cy + hh)
    }

    #[test]
    fn weather_arrow_does_not_cross_its_label() {
        let d = document("meteo").unwrap();
        let find = |pred: &dyn Fn(&Element) -> bool| elements(&d).find(|e| pred(e)).unwrap().frame;
        let label = find(&|e| matches!(&e.kind, ElementKind::Text(t) if t.content.paragraphs.iter().flat_map(|p| &p.runs).any(|r| r.text == "Aujourd'hui")));
        let arrow = find(&|e| matches!(&e.kind, ElementKind::Shape(s) if s.shape == ShapeKind::Arrow));
        let (_, top, _, bottom) = bounds(&arrow);
        assert!(top >= label.y_mm + label.h_mm, "la flèche commence à {top} mm, sous le texte qui finit à {} mm", label.y_mm + label.h_mm);
        assert!(bottom <= 62.0, "la flèche dépasse sur la case du lundi ({bottom} mm)");
    }
}
