//! Exemples de pages fournis avec l'application : ils montrent groupes, images, frises, cadres,
//! formes, polices et fonds, sans aucune photo (les emplacements de photos sont des zones vides).

use crate::model::*;

#[derive(serde::Serialize, Clone, Debug, PartialEq, ts_rs::TS)]
#[ts(export)]
pub struct ExampleInfo {
    pub id: String,
    pub name: String,
}

const EXAMPLES: [(&str, &str); 11] = [
    ("ferme", "Sortie à la ferme"),
    ("anniversaire", "Joyeux anniversaire"),
    ("meteo", "La météo de la semaine"),
    ("prenom", "J'écris mon prénom"),
    ("saisons", "Les saisons"),
    ("semaine", "Notre semaine"),
    ("emotions", "Le loto des émotions"),
    ("jours", "J'écris les jours"),
    ("regles", "Les règles de la classe (A3)"),
    ("sens", "Mes cinq sens"),
    ("porte-manteaux", "Étiquettes de porte-manteau"),
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
        "semaine" => week(),
        "emotions" => emotions(),
        "jours" => week_days(),
        "regles" => class_rules(),
        "sens" => five_senses(),
        "porte-manteaux" => coat_pegs(),
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
    Element { id: new_id(), frame: f, locked: false, zone: false, group: None, kind, subgroups: Vec::new(), flip_h: false, flip_v: false }
}

pub(crate) fn text(f: Frame, content: &str, font: &str, size_pt: f64, color: &str, align: Align) -> Element {
    let style = TextStyle { font: font.into(), size_pt, color: color.into(), align, valign: None, padding_mm: None, outline: None, named: None, line_spacing: None };
    el(f, ElementKind::Text(TextProps { content: RichText::plain(content), style, prompt: None, frame: None }))
}

/// Zone de texte à remplir, avec son invite.
pub(crate) fn text_zone(f: Frame, prompt: &str, font: &str, size_pt: f64) -> Element {
    let style = TextStyle { font: font.into(), size_pt, color: "#222222".into(), align: Align::Left, valign: None, padding_mm: None, outline: None, named: None, line_spacing: None };
    Element { zone: true, ..el(f, ElementKind::Text(TextProps { content: RichText::default(), style, prompt: Some(prompt.into()), frame: None })) }
}

/// Un texte qui ne contient qu'un champ de la classe (`field` sans accolades : `prenom`, `PRENOM_2`…).
pub(crate) fn field_text(f: Frame, field: &str, font: &str, size_pt: f64) -> Element {
    let style = TextStyle { font: font.into(), size_pt, color: "#222222".into(), align: Align::Left, valign: None, padding_mm: None, outline: None, named: None, line_spacing: None };
    el(f, ElementKind::Text(TextProps { content: RichText::plain(&format!("{{{field}}}")), style, prompt: None, frame: None }))
}

fn title(content: &str, y: f64, size_pt: f64, color: &str) -> Element {
    text(frame(15.0, y, 180.0, cursive_height(size_pt)), content, TITLE_FONT, size_pt, color, Align::Center)
}

fn picto(code: &str, x: f64, y: f64, size: f64) -> Element {
    el(frame(x, y, size, size), ElementKind::Image(ImageProps { code: code.into(), outline: None }))
}

pub(crate) fn shape(kind: ShapeKind, f: Frame, fill: Option<&str>, stroke: Option<(f64, &str)>, corner_radius_mm: f64) -> Element {
    el(
        f,
        ElementKind::Shape(ShapeProps {
            shape: kind,
            fill: fill.map(String::from),
            stroke: stroke.map(|(width_mm, color)| Stroke { width_mm, color: color.into(), style: None }),
            corner_radius_mm,
            path: None,
            graphisme: None,
            ends: None,
            points: None,
            curve: false,
            closed: false,
            attach: None, tail: None,
        }),
    )
}

pub(crate) fn photo_zone(f: Frame) -> Element {
    Element { zone: true, ..el(f, ElementKind::Photo(PhotoProps { asset: None, crop: Crop::COVER, corner_radius_mm: 3.0, border: None, shadow: false, mirrored: false })) }
}

/// Cadre de lignes d'écriture (sans texte), à écrire à la main.
pub(crate) fn text_lines(f: Frame, pattern: Pattern) -> Element {
    let mut z = text(f, "", TITLE_FONT, 28.0, "#222222", Align::Left);
    if let ElementKind::Text(t) = &mut z.kind {
        t.content = RichText::default();
        t.frame = Some(TextFrame { fill: None, stroke: None, corner_radius_mm: 0.0, image: None, pattern: Some(pattern), write_on_lines: None });
    }
    z
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

fn page(color: &str, pattern: Option<Pattern>, elements: Vec<Element>) -> Page {
    Page { elements, background: Some(Background { color: color.into(), pattern }), ..Page::new(A4_PORTRAIT) }
}

/// Un élément tourné de `deg` autour du point `c` (son cadre garde sa taille, son centre tourne avec le reste).
fn turned(mut e: Element, c: (f64, f64), deg: f64) -> Element {
    let (cx, cy) = (e.frame.x_mm + e.frame.w_mm / 2.0, e.frame.y_mm + e.frame.h_mm / 2.0);
    let a = deg.to_radians();
    let (dx, dy) = (cx - c.0, cy - c.1);
    let (nx, ny) = (c.0 + dx * a.cos() - dy * a.sin(), c.1 + dx * a.sin() + dy * a.cos());
    e.frame.x_mm = ((nx - e.frame.w_mm / 2.0) * 100.0).round() / 100.0;
    e.frame.y_mm = ((ny - e.frame.h_mm / 2.0) * 100.0).round() / 100.0;
    e.frame.rotation_deg = deg;
    e
}

/// Photo façon Polaroid, un peu penchée : un carton blanc, la photo à déposer et sa légende dessous, groupés.
fn polaroid(x: f64, y: f64, w: f64, deg: f64) -> Vec<Element> {
    let (m, ph, cap) = (4.0, w * 0.74, 13.0);
    let h = m + ph + cap;
    let c = (x + w / 2.0, y + h / 2.0);
    let card = shape(ShapeKind::Rect, frame(x, y, w, h), Some("#ffffff"), Some((0.3, "#d9d0bf")), 1.5);
    let photo = photo_zone(frame(x + m, y + m, w - 2.0 * m, ph));
    let mut caption = text_zone(frame(x + m, y + m + ph + 1.5, w - 2.0 * m, cap - 3.0), "Légende…", "Andika", 13.0);
    if let ElementKind::Text(t) = &mut caption.kind {
        t.style.align = Align::Center;
        t.style.valign = Some(VAlign::Middle);
    }
    grouped([card, photo, caption].into_iter().map(|e| turned(e, c, deg)).collect())
}

/// Cahier de vie : page crème, cadre vert, titre et date, deux photos façon Polaroid, une bulle de parole qui vise la
/// chèvre, « ce que nous avons appris » sur des lignes, et les animaux de la ferme dans le pré.
fn farm() -> Page {
    const GREEN: &str = "#3f6e2a";
    let mut els = vec![shape(ShapeKind::Rect, frame(8.0, 8.0, 194.0, 276.0), None, Some((1.2, "#7fa65a")), 10.0)];
    // Le pré, en bas du cadre : les animaux s'y tiennent.
    els.push(shape(ShapeKind::Rect, frame(8.0, 252.0, 194.0, 32.0), Some("#b9dc97"), None, 10.0));
    // En-tête : tracteur, titre, soleil ; la date de la sortie dessous.
    els.extend(grouped(vec![
        picto("1F69C", 16.0, 15.0, 20.0),
        text(frame(38.0, 14.0, 134.0, cursive_height(26.0)), "Notre sortie à la ferme", TITLE_FONT, 26.0, GREEN, Align::Center),
        picto("2600", 174.0, 15.0, 20.0),
    ]));
    let mut date = field_text(frame(38.0, 39.0, 134.0, 11.0), "date_longue", "Andika", 12.0);
    if let ElementKind::Text(t) = &mut date.kind {
        t.style.align = Align::Center;
        t.style.color = "#6b6b5e".into();
    }
    els.push(date);
    els.extend(polaroid(17.0, 56.0, 84.0, -3.0));
    els.extend(polaroid(109.0, 58.0, 84.0, 2.5));
    // La bulle de parole d'un enfant, dont la pointe vise la chèvre.
    let mut bubble = shape(ShapeKind::Bubble, frame(16.0, 150.0, 122.0, 36.0), Some("#ffffff"), Some((0.7, GREEN)), 0.0);
    if let ElementKind::Shape(sp) = &mut bubble.kind {
        sp.tail = Some([(150.0 - 16.0) / 122.0, (172.0 - 150.0) / 36.0]);
    }
    let mut said = text(frame(22.0, 153.0, 110.0, 30.0), "J'ai donné à manger aux chèvres !", "Andika", 17.0, "#222222", Align::Center);
    if let ElementKind::Text(t) = &mut said.kind {
        t.style.valign = Some(VAlign::Middle);
    }
    els.extend(grouped(vec![bubble, said]));
    els.push(picto("1F410", 148.0, 145.0, 44.0));
    // Ce que nous avons appris : un titre et des lignes pour la dictée à l'adulte.
    els.push(text(frame(18.0, 197.0, 174.0, 12.0), "Ce que nous avons appris", "Andika", 15.0, GREEN, Align::Left));
    els.push(text_lines(frame(18.0, 209.0, 174.0, 34.0), Pattern::Lines));
    // Les animaux dans le pré.
    let animals = ["1F404", "1F416", "1F414", "1F411", "1F40E", "1F986"];
    els.extend(grouped(animals.iter().enumerate().map(|(i, code)| picto(code, 22.0 + i as f64 * 29.0, 247.0, 22.0)).collect()));
    page("#fffaf0", None, els)
}

/// Les réglages d'un texte, modifiés sur place.
fn styled(mut e: Element, f: impl FnOnce(&mut TextProps)) -> Element {
    if let ElementKind::Text(t) = &mut e.kind {
        f(t);
    }
    e
}

/// Les réglages d'une forme, modifiés sur place.
fn reshaped(mut e: Element, f: impl FnOnce(&mut ShapeProps)) -> Element {
    if let ElementKind::Shape(sp) = &mut e.kind {
        f(sp);
    }
    e
}

/// Un cadre de texte coloré (fond, bord, coins).
fn boxed(fill: &str, stroke: Option<(f64, &str)>, radius: f64) -> TextFrame {
    TextFrame { fill: Some(fill.into()), stroke: stroke.map(|(w, c)| Stroke { width_mm: w, color: c.into(), style: None }), corner_radius_mm: radius, image: None, pattern: None, write_on_lines: None }
}

/// Un texte centré dans son cadre, de haut en bas aussi.
fn centred(f: Frame, content: &str, font: &str, size_pt: f64, color: &str) -> Element {
    styled(text(f, content, font, size_pt, color, Align::Center), |t| t.style.valign = Some(VAlign::Middle))
}

/// Un chemin courbe qui passe par `pts` (mm de page), avec ses terminaisons.
fn curve(pts: &[(f64, f64)], width_mm: f64, color: &str, ends: Option<LineEnds>) -> Element {
    // Avec une pointe, c'est une flèche (comme une flèche du tiroir prolongée en chemin).
    let kind = if ends.is_some_and(|e| e.end != LineEnd::None || e.start != LineEnd::None) { ShapeKind::Arrow } else { ShapeKind::Line };
    let mut s = match shape(kind, frame(0.0, 0.0, 1.0, 1.0), None, Some((width_mm, color)), 0.0).kind {
        ElementKind::Shape(s) => s,
        _ => unreachable!(),
    };
    s.curve = true;
    s.ends = ends;
    let (f, points) = crate::shapes::path_frame(pts, &s);
    s.points = Some(points);
    el(f, ElementKind::Shape(s))
}

const ARROW_END: LineEnds = LineEnds { start: LineEnd::None, end: LineEnd::Arrow, size: HeadSize::Medium };

/// Une flèche accrochée d'un point d'un élément à un point d'un autre : elle les suit quand on les déplace.
fn link(from: &Element, from_at: AnchorPoint, to: &Element, to_at: AnchorPoint, color: &str) -> Element {
    let mut e = shape(ShapeKind::Arrow, frame(0.0, 0.0, 1.0, 1.0), None, Some((0.8, color)), 0.0);
    if let ElementKind::Shape(sp) = &mut e.kind {
        sp.ends = Some(ARROW_END);
        sp.attach = Some(Attach { start: Some(Anchor { element: from.id.clone(), at: from_at }), end: Some(Anchor { element: to.id.clone(), at: to_at }) });
        e.frame = crate::shapes::line_frame(crate::connectors::anchor_point(&from.frame, from_at), crate::connectors::anchor_point(&to.frame, to_at), sp);
    }
    e
}

/// Bulle (de parole, ou nuage de pensée) dont la pointe vise `tip` (mm de page).
fn bubble(kind: ShapeKind, f: Frame, fill: &str, stroke: (f64, &str), tip: (f64, f64)) -> Element {
    reshaped(shape(kind, f, Some(fill), Some(stroke), 0.0), |sp| sp.tail = Some([(tip.0 - f.x_mm) / f.w_mm, (tip.1 - f.y_mm) / f.h_mm]))
}

/// Photo à déposer dans un rond, avec un liseré de couleur.
fn round_photo(f: Frame, ring: &str) -> Element {
    let mut z = photo_zone(f);
    if let ElementKind::Photo(ph) = &mut z.kind {
        ph.corner_radius_mm = f.w_mm.min(f.h_mm) / 2.0;
        ph.border = Some(Stroke { width_mm: 1.2, color: ring.into(), style: None });
    }
    z
}

/// Fanion triangulaire (tracé libre), pointe en bas.
fn pennant(x: f64, y: f64, w: f64, h: f64, color: &str) -> Element {
    reshaped(shape(ShapeKind::Path, frame(x, y, w, h), Some(color), None, 0.0), |sp| sp.path = Some("M 0 0 L 1000 0 L 500 1000 Z".into()))
}

/// Anniversaire : guirlande de fanions sur une ficelle courbe, ballons et leurs ficelles, gâteau à colorier, le
/// prénom de l'enfant en grandes lettres creuses, une photo et ce que nous avons partagé.
fn birthday() -> Page {
    const PINK: &str = "#d6336c";
    let sag = |x: f64| 14.0 + 16.0 * (1.0 - ((x - 105.0) / 97.0).powi(2));
    let mut garland = vec![curve(&[(8.0, sag(8.0)), (56.0, sag(56.0)), (105.0, sag(105.0)), (154.0, sag(154.0)), (202.0, sag(202.0))], 0.5, "#8d6e63", None)];
    let colors = ["#ff6b6b", "#ffd43b", "#69db7c", "#4dabf7", "#b197fc"];
    for k in 0..10 {
        let x = 16.0 + k as f64 * 18.6;
        garland.push(pennant(x, sag(x + 5.0) - 0.5, 10.0, 12.0, colors[k % 5]));
    }
    let mut els = grouped(garland);
    els.push(title("Joyeux anniversaire", 38.0, 32.0, PINK));
    // Deux ballons, leurs ficelles en courbe.
    els.extend(grouped(vec![picto("1F388", 12.0, 70.0, 30.0), curve(&[(25.0, 99.0), (31.0, 112.0), (23.0, 126.0), (29.0, 140.0)], 0.4, "#6b6b6b", None)]));
    els.extend(grouped(vec![picto("1F388", 168.0, 66.0, 30.0), curve(&[(181.0, 95.0), (176.0, 108.0), (184.0, 122.0), (178.0, 136.0)], 0.4, "#6b6b6b", None)]));
    // Le gâteau, à colorier, et sa consigne.
    let mut cake = picto("1F382", 70.0, 70.0, 70.0);
    if let ElementKind::Image(im) = &mut cake.kind {
        im.outline = Some(true);
    }
    els.extend(grouped(vec![cake, text(frame(55.0, 140.0, 100.0, 10.5), "Je colorie le gâteau", "Andika", 12.0, "#8a8a8a", Align::Center)]));
    for (x, y, sz) in [(48.0, 80.0, 12.0), (150.0, 104.0, 10.0), (40.0, 128.0, 9.0), (160.0, 140.0, 12.0)] {
        els.push(picto("2B50", x, y, sz));
    }
    // Le prénom de l'enfant, rempli avec la classe, en lettres creuses.
    els.push(styled(field_text(frame(15.0, 152.0, 180.0, cursive_height(44.0)), "prenom", TITLE_FONT, 44.0), |t| {
        t.style.align = Align::Center;
        t.style.color = PINK.into();
        t.style.outline = Some(true);
    }));
    els.push(text(frame(25.0, 196.0, 160.0, 14.0), "a fêté ses {age} en classe !", "Andika", 18.0, "#444444", Align::Center));
    // Une photo de la fête et ce que nous avons partagé, côte à côte.
    let mut photo = photo_zone(frame(18.0, 216.0, 82.0, 60.0));
    if let ElementKind::Photo(ph) = &mut photo.kind {
        ph.shadow = true;
        ph.corner_radius_mm = 5.0;
    }
    els.push(photo);
    els.push(styled(text_zone(frame(108.0, 216.0, 84.0, 60.0), "Ce que nous avons partagé…", "Andika", 14.0), |t| {
        t.frame = Some(TextFrame { pattern: Some(Pattern::Lines), ..boxed("#ffffff", Some((0.6, "#f3a6c0")), 5.0) });
        t.style.padding_mm = Some(4.0);
    }));
    page("#fff4e6", None, els)
}

/// Météo : petits carreaux, une carte par jour (bandeau de couleur, image, lignes pour écrire), une flèche courbe
/// « Aujourd'hui », et la pensée d'un enfant sur le temps de la semaine.
fn weather() -> Page {
    const BLUE: &str = "#1f5fa8";
    let mut els = vec![title("La météo de la semaine", 10.0, 26.0, BLUE)];
    let days = [("Lundi", "2600", "#ffe8a3"), ("Mardi", "2601", "#dfe7ef"), ("Mercredi", "1F327", "#cfe3f7"), ("Jeudi", "2744", "#e3f2fd"), ("Vendredi", "1F308", "#f3e1fa")];
    for (i, (day, code, color)) in days.iter().enumerate() {
        let x = 10.0 + i as f64 * 38.5;
        els.extend(grouped(vec![
            shape(ShapeKind::Rect, frame(x, 62.0, 36.0, 98.0), Some("#ffffff"), Some((0.5, BLUE)), 5.0),
            styled(centred(frame(x, 62.0, 36.0, 12.0), day, "Andika", 13.0, BLUE), |t| t.frame = Some(boxed(color, None, 5.0))),
            picto(code, x + 5.0, 80.0, 26.0),
            styled(text_zone(frame(x + 2.0, 112.0, 32.0, 44.0), "Il fait…", "Andika", 11.0), |t| t.frame = Some(TextFrame { pattern: Some(Pattern::Lines), ..TextFrame::default() })),
        ]));
    }
    els.extend(grouped(vec![
        text(frame(34.0, 35.0, 40.0, 10.0), "Aujourd'hui", "Andika", 12.0, "#c0392b", Align::Left),
        curve(&[(32.0, 40.0), (24.0, 49.0), (27.0, 60.0)], 0.9, "#c0392b", Some(ARROW_END)),
    ]));
    // La pensée d'un enfant : le temps qu'il a fait cette semaine.
    let child = picto("1F9D2", 158.0, 222.0, 36.0);
    els.extend(grouped(vec![
        bubble(ShapeKind::Cloud, frame(16.0, 172.0, 136.0, 76.0), "#ffffff", (0.7, BLUE), (162.0, 226.0)),
        styled(text_zone(frame(34.0, 188.0, 100.0, 44.0), "Cette semaine, il a fait…", "Andika", 14.0), |t| t.style.valign = Some(VAlign::Middle)),
    ]));
    els.push(child);
    els.push(picto("2614", 166.0, 176.0, 26.0));
    page("#ffffff", Some(Pattern::SmallSquares), els)
}

/// Écriture : la photo de l'enfant dans un rond, son prénom en lettres creuses, une ligne Seyès à repasser,
/// d'autres pour écrire, et une étoile « Bravo ».
fn first_name() -> Page {
    const PURPLE: &str = "#7b3fa0";
    let mut els = grouped(vec![
        round_photo(frame(16.0, 10.0, 34.0, 34.0), PURPLE),
        text(frame(56.0, 14.0, 120.0, cursive_height(26.0)), "J'écris mon prénom", TITLE_FONT, 26.0, PURPLE, Align::Left),
        picto("270F", 178.0, 16.0, 18.0),
    ]);
    els.push(styled(text(frame(15.0, 50.0, 180.0, cursive_height(60.0)), "Léa", TITLE_FONT, 60.0, PURPLE, Align::Center), |t| t.style.outline = Some(true)));
    for i in 0..4 {
        let mut lines = text_lines(frame(15.0, 114.0 + i as f64 * 40.0, 180.0, 34.0), Pattern::Seyes);
        if i == 0 && let ElementKind::Text(t) = &mut lines.kind {
            t.content = RichText::plain("Léa   Léa   Léa");
            t.style.size_pt = 34.0;
            t.style.color = "#c4c4c4".into();
            t.style.valign = Some(VAlign::Middle);
            t.style.padding_mm = Some(4.0);
        }
        els.push(lines);
    }
    els.extend(grouped(vec![
        shape(ShapeKind::Star, frame(160.0, 268.0, 22.0, 21.0), Some("#ffd43b"), Some((0.5, "#e8a400")), 0.0),
        text(frame(118.0, 270.0, 40.0, cursive_height(14.0)), "Bravo !", TITLE_FONT, 14.0, PURPLE, Align::Right),
    ]));
    page("#fbf7ff", None, els)
}

/// Saisons : quatre panneaux de couleur dans l'ordre de l'année, autour d'un arbre dans un rond ; un nom, une image
/// et des lignes par saison.
fn seasons() -> Page {
    let mut els = vec![title("Les saisons", 12.0, 30.0, "#2e7d32")];
    let seasons = [
        ("Printemps", "1F337", 14.0, 44.0, "#eaf6e1", "#4c9a2a"),
        ("Été", "1F3D6", 108.0, 44.0, "#fff3c9", "#d98e04"),
        ("Hiver", "2603", 14.0, 164.0, "#e3f0fb", "#2f6fb0"),
        ("Automne", "1F341", 108.0, 164.0, "#fde6d2", "#c0571b"),
    ];
    for (name, code, x, y, fill, ink) in seasons {
        els.extend(grouped(vec![
            shape(ShapeKind::Rect, frame(x, y, 88.0, 112.0), Some(fill), Some((0.8, ink)), 10.0),
            picto(code, x + 8.0, y + 8.0, 30.0),
            text(frame(x + 40.0, y + 12.0, 44.0, cursive_height(18.0)), name, TITLE_FONT, 18.0, ink, Align::Left),
            styled(text_zone(frame(x + 8.0, y + 58.0, 72.0, 46.0), "Ce que j'aime…", "Andika", 12.0), |t| t.frame = Some(TextFrame { pattern: Some(Pattern::Lines), ..TextFrame::default() })),
        ]));
    }
    // Au milieu, l'arbre qui change avec les saisons.
    els.extend(grouped(vec![shape(ShapeKind::Ellipse, frame(80.0, 130.0, 50.0, 50.0), Some("#ffffff"), Some((1.0, "#2e7d32")), 0.0), picto("1F333", 89.0, 138.0, 32.0)]));
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
    fn examples_with_distinct_ids() {
        let ids: Vec<String> = list().into_iter().map(|i| i.id).collect();
        assert_eq!(ids, ["ferme", "anniversaire", "meteo", "prenom", "saisons", "semaine", "emotions", "jours", "regles", "sens", "porte-manteaux"]);
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
            let size = d.pages[0].size;
            for e in elements(&d) {
                match &e.kind {
                    ElementKind::Image(i) => assert!(crate::picto::svg(&i.code).is_some(), "{} : image {}", d.title, i.code),
                    ElementKind::Text(t) => {
                        assert!(fonts.face(&t.style.font, false, false).is_some(), "{} : police {}", d.title, t.style.font);
                    }
                    _ => {}
                }
                let inside = e.frame.x_mm >= 0.0 && e.frame.y_mm >= 0.0 && e.frame.x_mm + e.frame.w_mm <= size.width_mm + 0.5 && e.frame.y_mm + e.frame.h_mm <= size.height_mm + 0.5;
                assert!(inside, "{} : élément hors page", d.title);
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
    #[test]
    fn weather_arrow_goes_from_its_label_to_monday() {
        let d = document("meteo").unwrap();
        let label = elements(&d).find(|e| matches!(&e.kind, ElementKind::Text(t) if t.content.paragraphs.iter().flat_map(|p| &p.runs).any(|r| r.text == "Aujourd'hui"))).unwrap().frame;
        let arrow = elements(&d).find(|e| matches!(&e.kind, ElementKind::Shape(s) if s.ends.is_some() && s.points.is_some())).unwrap();
        let ElementKind::Shape(s) = &arrow.kind else { unreachable!() };
        let pts = crate::shapes::path_page_points(&arrow.frame, s, false, false).unwrap();
        let (start, end) = (pts[0], pts[pts.len() - 1]);
        assert!(start.0 < label.x_mm, "la flèche part sur le mot ({start:?})");
        assert!(end.1 <= 62.0 && (10.0..46.0).contains(&end.0), "la flèche ne finit pas au-dessus de lundi ({end:?})");
    }

    /// La carte mentale des cinq sens : chaque carte a sa flèche, accrochée à l'enfant et à la carte.
    #[test]
    fn five_senses_arrows_are_attached() {
        let d = document("sens").unwrap();
        let attached = elements(&d).filter(|e| matches!(&e.kind, ElementKind::Shape(s) if s.attach.as_ref().is_some_and(|a| a.start.is_some() && a.end.is_some()))).count();
        assert_eq!(attached, 5);
    }
}

// ---- Exemples des fonctions de la version 0.8 : tableau, lignes d'écriture, A3, classe. ----

const INK: &str = "#1f3a5f";

/// Le texte d'un élément texte (case de tableau comprise), avec sa police et sa taille.
fn write(page: &mut Page, id: &str, content: &str, font: &str, size_pt: f64, fill: Option<&str>) {
    let e = page.elements.iter_mut().find(|e| e.id == id).expect("élément");
    if let ElementKind::Text(t) = &mut e.kind {
        t.content = RichText::plain(content);
        t.style.font = font.into();
        t.style.size_pt = size_pt;
        t.style.color = INK.into();
        t.frame = fill.map(|c| TextFrame { fill: Some(c.into()), stroke: None, corner_radius_mm: 0.0, image: None, pattern: None, write_on_lines: None });
    }
}

fn cell(page: &Page, table: &str, col: u8, row: u8) -> String {
    let t = page.elements.iter().find(|e| e.id == table).expect("tableau");
    let ElementKind::Table(p) = &t.kind else { unreachable!() };
    p.cells.iter().find(|c| c.col == col && c.row == row).expect("case").element_id.clone()
}

/// Emploi du temps de la semaine : un tableau en paysage, les rituels sur toute la semaine, un picto par
/// activité.
fn week() -> Page {
    let mut page = Page { background: Some(Background { color: "#fffaf0".into(), pattern: None }), ..Page::new(PageSize { width_mm: 297.0, height_mm: 210.0 }) };
    page.elements.extend(grouped(vec![
        picto("1F4C5", 80.0, 9.0, 18.0),
        text(frame(100.0, 8.0, 97.0, cursive_height(28.0)), "Notre semaine", TITLE_FONT, 28.0, "#8a4b16", Align::Center),
        picto("23F0", 199.0, 9.0, 18.0),
    ]));
    let table = crate::table::add_table(&mut page, frame(15.0, 36.0, 267.0, 162.0), 5, 7);
    if let Some(ElementKind::Table(t)) = page.elements.iter_mut().find(|e| e.id == table).map(|e| &mut e.kind) {
        t.cols = vec![1.3, 1.0, 1.0, 1.0, 1.0];
        t.rows = vec![0.8, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0];
        t.border = Some(Stroke { width_mm: 0.6, color: "#6b5444".into(), style: None });
    }
    crate::table::layout(&mut page, &table);
    let days = [("Lundi", "#ffd6d6"), ("Mardi", "#d6e8ff"), ("Jeudi", "#d8f5d0"), ("Vendredi", "#fff1bf")];
    let c = |p: &Page, col, row| cell(p, &table, col, row);
    let corner = c(&page, 0, 0);
    write(&mut page, &corner, "Quand ?", "Andika", 14.0, Some("#f1e6d6"));
    for (i, (d, color)) in days.iter().enumerate() {
        let id = c(&page, i as u8 + 1, 0);
        write(&mut page, &id, d, "Andika", 18.0, Some(color));
    }
    let rows: [(&str, &[&str]); 6] = [
        ("8h30\nAccueil", &["1F44B"]),
        ("9h00\nRituels", &["1F4C5"]),
        ("9h30\nAteliers", &["1F3A8", "2702", "1F9E9", "270F"]),
        ("10h15\nRécréation", &["1F6DD"]),
        ("10h45\nMotricité, musique", &["26BD", "1F3B5", "26BD", "1F4DA"]),
        ("11h45\nRepas", &["1F37D"]),
    ];
    for (r, (when, pictos)) in rows.iter().enumerate() {
        let row = r as u8 + 1;
        let id = c(&page, 0, row);
        write(&mut page, &id, &when.replace('\n', " "), "Andika", 14.0, Some("#fff3e3"));
        if pictos.len() == 1 {
            let (a, b) = (c(&page, 1, row), c(&page, 4, row));
            crate::table::merge(&mut page, &a, &b).expect("fusion");
        }
        for (i, code) in pictos.iter().enumerate() {
            let id = c(&page, i as u8 + 1, row);
            crate::table::set_cell_content(&mut page, &id, crate::table::CellContent::Picto((*code).into())).expect("picto");
        }
    }
    page
}

/// Loto des émotions : douze cartes de couleur à découper (contour en tirets, ciseaux), une émotion et son mot.
fn emotions() -> Page {
    let cards = [
        ("1F600", "joyeux"), ("1F622", "triste"), ("1F620", "en colère"), ("1F628", "effrayé"),
        ("1F632", "surpris"), ("1F971", "fatigué"), ("1F970", "amoureux"), ("1F914", "pensif"),
        ("1F60C", "calme"), ("1F62D", "chagrin"), ("1F97A", "timide"), ("1F642", "content"),
    ];
    let tints = ["#fff3bf", "#d0ebff", "#ffe3e3", "#e5dbff", "#d3f9d8", "#ffe8cc"];
    let (w, h, gap) = (58.0, 56.0, 5.0);
    let x0 = (210.0 - (3.0 * w + 2.0 * gap)) / 2.0;
    let mut els = vec![title("Le loto des émotions", 8.0, 26.0, "#6a3d9a")];
    els.extend(grouped(vec![
        picto("2702", 52.0, 33.0, 9.0),
        text(frame(63.0, 32.0, 100.0, 10.0), "Je découpe les cartes, puis je joue !", "Andika", 13.0, "#6a3d9a", Align::Left),
    ]));
    for (i, (code, word)) in cards.iter().enumerate() {
        let (x, y) = (x0 + (i % 3) as f64 * (w + gap), 46.0 + (i / 3) as f64 * (h + gap));
        let card = reshaped(shape(ShapeKind::Rect, frame(x, y, w, h), Some(tints[i % tints.len()]), Some((0.5, "#9b7be0")), 6.0), |sp| {
            sp.stroke = Some(Stroke { width_mm: 0.5, color: "#9b7be0".into(), style: Some(StrokeStyle::Dashed) });
        });
        els.extend(grouped(vec![card, picto(code, x + 15.0, y + 5.0, 28.0), centred(frame(x + 3.0, y + 37.0, w - 6.0, 14.0), word, "Andika", 17.0, INK)]));
    }
    page("#f6f0fd", None, els)
}

/// J'écris les jours : chaque jour sur une étiquette de sa couleur, puis en cursive à lettres creuses à repasser sur
/// des lignes Seyès.
fn week_days() -> Page {
    let mut els = grouped(vec![picto("1F4C5", 22.0, 10.0, 22.0), title("J'écris les jours", 10.0, 26.0, "#c0392b"), picto("270F", 166.0, 10.0, 22.0)]);
    let days = [("lundi", "#ffd6d6"), ("mardi", "#d6e8ff"), ("mercredi", "#d8f5d0"), ("jeudi", "#fff1bf"), ("vendredi", "#ead9ff")];
    for (i, (day, color)) in days.iter().enumerate() {
        let y = 38.0 + i as f64 * 50.0;
        let tag = styled(centred(frame(15.0, y, 34.0, 11.0), day, "Andika", 13.0, INK), |t| {
            t.frame = Some(boxed(color, None, 4.5));
            t.style.padding_mm = Some(1.0);
        });
        let lines = styled(text_lines(frame(15.0, y + 12.0, 180.0, 37.0), Pattern::Seyes), |t| {
            t.content = RichText::plain(day);
            t.style.size_pt = 38.0;
            t.style.color = "#9a9a9a".into();
            t.style.outline = Some(true);
            t.style.valign = Some(VAlign::Middle);
            t.style.padding_mm = Some(4.0);
        });
        els.extend(grouped(vec![tag, lines]));
    }
    page("#ffffff", None, els)
}

/// Les règles de la classe : une affiche A3, un bandeau pour le titre, une règle numérotée par carte, et la place
/// pour les signatures de la classe.
fn class_rules() -> Page {
    const BLUE: &str = "#2e86de";
    let rules = [
        ("270B", "Je lève la main pour parler."),
        ("1F92B", "Je chuchote dans la classe."),
        ("1F6B6", "Je marche dans les couloirs."),
        ("1F91D", "Je partage avec les autres."),
        ("1F9F9", "Je range le matériel."),
        ("2764", "Je prends soin de mes camarades."),
    ];
    let tints = ["#eaf2fb", "#fff6e6", "#eaf7ee", "#fdeef4", "#f1ecfb", "#fff9db"];
    let mut els = vec![
        shape(ShapeKind::Rect, frame(10.0, 10.0, 277.0, 400.0), None, Some((2.0, BLUE)), 12.0),
        styled(centred(frame(30.0, 20.0, 237.0, 44.0), "Les règles de la classe", TITLE_FONT, 40.0, "#ffffff"), |t| t.frame = Some(boxed(BLUE, None, 22.0))),
    ];
    for (i, (code, rule)) in rules.iter().enumerate() {
        let y = 78.0 + i as f64 * 48.0;
        els.extend(grouped(vec![
            shape(ShapeKind::Rect, frame(24.0, y, 249.0, 40.0), Some(tints[i]), None, 10.0),
            shape(ShapeKind::Ellipse, frame(30.0, y + 6.0, 28.0, 28.0), Some(BLUE), None, 0.0),
            centred(frame(30.0, y + 6.0, 28.0, 28.0), &(i + 1).to_string(), "Andika", 24.0, "#ffffff"),
            picto(code, 66.0, y + 4.0, 32.0),
            styled(text(frame(106.0, y + 6.0, 162.0, 28.0), rule, "Andika", 26.0, INK, Align::Left), |t| t.style.valign = Some(VAlign::Middle)),
        ]));
    }
    // Les signatures de la classe.
    els.push(text(frame(24.0, 370.0, 249.0, 14.0), "Nous avons choisi ces règles ensemble :", "Andika", 18.0, BLUE, Align::Left));
    els.push(styled(text_zone(frame(24.0, 386.0, 249.0, 18.0), "Signatures…", "Andika", 14.0), |t| t.frame = Some(TextFrame { pattern: Some(Pattern::Lines), ..TextFrame::default() })));
    Page { elements: els, background: Some(Background { color: "#ffffff".into(), pattern: None }), ..Page::new(PageSize { width_mm: 297.0, height_mm: 420.0 }) }
}

/// Mes cinq sens : une carte mentale. L'enfant dans un rond, cinq cartes (image, verbe, lignes pour écrire) et des
/// flèches accrochées : on peut déplacer une carte, sa flèche suit.
fn five_senses() -> Page {
    const GREEN: &str = "#27ae60";
    let senses = [("1F441", "Je vois", "#e7f5ff"), ("1F442", "J'entends", "#fff4e6"), ("1F443", "Je sens", "#f3f0ff"), ("1F444", "Je goûte", "#ffeef3"), ("270B", "Je touche", "#ebfbee")];
    let mut els = vec![title("Mes cinq sens", 10.0, 28.0, GREEN)];
    let me = shape(ShapeKind::Ellipse, frame(12.0, 128.0, 62.0, 62.0), Some("#ffffff"), Some((1.2, GREEN)), 0.0);
    els.extend(grouped(vec![me.clone(), picto("1F9D2", 25.0, 139.0, 36.0), text(frame(12.0, 191.0, 62.0, cursive_height(16.0)), "Moi", TITLE_FONT, 16.0, GREEN, Align::Center)]));
    for (i, (code, verb, tint)) in senses.iter().enumerate() {
        let y = 44.0 + i as f64 * 49.0;
        let card = shape(ShapeKind::Rect, frame(96.0, y, 102.0, 42.0), Some(tint), Some((0.6, GREEN)), 7.0);
        els.push(link(&me, AnchorPoint::Right, &card, AnchorPoint::Left, GREEN));
        els.extend(grouped(vec![
            card,
            picto(code, 100.0, y + 4.0, 20.0),
            text(frame(123.0, y + 3.0, 70.0, 13.0), verb, "Andika", 17.0, GREEN, Align::Left),
            styled(text_lines(frame(100.0, y + 19.0, 94.0, 20.0), Pattern::Lines), |t| t.style.size_pt = 16.0),
        ]));
    }
    page("#f4fbf2", None, els)
}

/// Étiquettes de porte-manteau : la photo de l'enfant dans un rond, son prénom en capitales et en cursive, un
/// animal pour le reconnaître, un contour à découper.
fn coat_pegs() -> Page {
    let (w, h, gap) = (90.0, 62.0, 6.0);
    let y0 = (297.0 - (4.0 * h + 3.0 * gap)) / 2.0;
    let colors = [("#fde2e4", "#e8899a"), ("#e2f0cb", "#7fb35a"), ("#d6ecfa", "#5a9bd5"), ("#fff1c1", "#e0b43c")];
    let animals = ["1F431", "1F436", "1F430", "1F98A", "1F43B", "1F981", "1F438", "1F427"];
    let mut els = Vec::new();
    for row in 0..4 {
        for (k, x) in [12.0, 108.0].into_iter().enumerate() {
            let y = y0 + row as f64 * (h + gap);
            let (fill, ring) = colors[(row + k) % 4];
            let zone = |f: Frame, field: &str, font: &str, size: f64| {
                styled(field_text(f, field, font, size), |t| {
                    t.style.align = Align::Center;
                    t.style.valign = Some(VAlign::Middle);
                    t.style.color = INK.into();
                })
            };
            let card = reshaped(shape(ShapeKind::Rect, frame(x, y, w, h), Some(fill), Some((0.6, "#7f8c8d")), 8.0), |sp| {
                sp.stroke = Some(Stroke { width_mm: 0.6, color: "#7f8c8d".into(), style: Some(StrokeStyle::Dashed) });
            });
            els.extend(grouped(vec![
                card,
                round_photo(frame(x + 5.0, y + 6.0, 34.0, 34.0), ring),
                picto(animals[row * 2 + k], x + 13.0, y + 42.0, 17.0),
                zone(frame(x + 42.0, y + 8.0, w - 46.0, 20.0), "PRENOM", "Andika", 22.0),
                zone(frame(x + 42.0, y + 30.0, w - 46.0, 26.0), "prenom", TITLE_FONT, 24.0),
            ]));
        }
    }
    page("#ffffff", None, els)
}
