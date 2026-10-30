//! Modèles : les modèles intégrés, et la bibliothèque des modèles de l'utilisateur.

use crate::examples as ex;
use crate::fonts::FontSet;
use crate::grid;
use crate::model::*;
use crate::native::{self, NativeError, TEMPLATE_EXT};
use crate::photo::{Asset, AssetStore};
use serde::Serialize;
use std::path::PathBuf;
use ts_rs::TS;

#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[ts(export)]
pub struct TemplateInfo {
    pub id: String,
    pub name: String,
    pub builtin: bool,
}

#[derive(Clone, Debug)]
pub struct Template {
    pub info: TemplateInfo,
    pub page: Page,
    pub assets: Vec<Asset>,
}

fn cell_at(page: &Page, col: u8, row: u8) -> String {
    let g = page.grid.as_ref().expect("grille posée");
    g.cells.iter().find(|c| c.col == col && c.row == row).expect("case existante").element_id.clone()
}

fn merge(page: &mut Page, a: (u8, u8), b: (u8, u8)) -> String {
    let (ia, ib) = (cell_at(page, a.0, a.1), cell_at(page, b.0, b.1));
    grid::merge_cells(page, &ia, &ib).expect("fusion rectangulaire");
    ia
}

fn text_zone(page: &mut Page, id: &str, prompt: &str, style: TextStyle) {
    grid::set_cell_kind(page, id, ZoneKind::Text).expect("case existante");
    let e = page.elements.iter_mut().find(|e| e.id == id).unwrap();
    if let ElementKind::Text(t) = &mut e.kind {
        t.prompt = Some(prompt.to_string());
        t.style = style;
    }
}

fn title_style() -> TextStyle {
    TextStyle { font: "Playwrite FR Moderne".into(), size_pt: 24.0, color: "#c0392b".into(), align: Align::Center, valign: None, padding_mm: None, outline: None }
}

/// Grille 2 × 7 : une ligne de titre, puis des cases de deux lignes pour garder des photos bien proportionnées.
fn page_with_title() -> Page {
    let mut page = Page::new(A4_PORTRAIT);
    grid::set_grid(&mut page, 2, 7);
    let title = merge(&mut page, (0, 0), (1, 0));
    text_zone(&mut page, &title, "Titre", title_style());
    page
}

/// Titre à compléter d'une page sans grille, comme celui des pages à grille.
fn free_title(width_mm: f64) -> Element {
    let mut t = ex::text_zone(ex::frame(15.0, 12.0, width_mm - 30.0, ex::cursive_height(24.0)), "Titre", ex::TITLE_FONT, 24.0);
    if let ElementKind::Text(text) = &mut t.kind {
        text.style = title_style();
    }
    t
}

/// Zone « Prénom » centrée dans son cadre.
fn name_zone(f: Frame, size_pt: f64) -> Element {
    let mut z = ex::text_zone(f, "Prénom", "Andika", size_pt);
    if let ElementKind::Text(t) = &mut z.kind {
        t.style.align = Align::Center;
        t.style.valign = Some(VAlign::Middle);
        t.style.color = "#1f3a5f".into();
    }
    z
}

const LABEL_BLUE: &str = "#2e86c1";
const LABEL_FILL: &str = "#eaf2fb";

/// Planche de 2 × 6 étiquettes encadrées (90 × 38 mm), à découper.
fn labels() -> Page {
    let (w, h, gap) = (90.0, 38.0, 8.0);
    let y0 = (297.0 - (6.0 * h + 5.0 * gap)) / 2.0;
    let mut page = Page::new(A4_PORTRAIT);
    for row in 0..6 {
        for x in [10.0, 110.0] {
            let mut z = name_zone(ex::frame(x, y0 + row as f64 * (h + gap), w, h), 26.0);
            if let ElementKind::Text(t) = &mut z.kind {
                t.frame = Some(TextFrame {
                    fill: Some(LABEL_FILL.into()),
                    stroke: Some(Stroke { width_mm: 0.8, color: LABEL_BLUE.into(), style: None }),
                    corner_radius_mm: 6.0,
                    image: None,
                });
            }
            page.elements.push(z);
        }
    }
    page
}

/// Planche de 2 × 5 étiquettes (90 × 50 mm) : bordure, photo et prénom groupés.
fn photo_labels() -> Page {
    let (w, h, gap) = (90.0, 50.0, 6.0);
    let y0 = (297.0 - (5.0 * h + 4.0 * gap)) / 2.0;
    let mut page = Page::new(A4_PORTRAIT);
    for row in 0..5 {
        for x in [10.0, 110.0] {
            let y = y0 + row as f64 * (h + gap);
            page.elements.extend(ex::grouped(vec![
                ex::shape(ShapeKind::Rect, ex::frame(x, y, w, h), Some(LABEL_FILL), Some((0.8, LABEL_BLUE)), 5.0),
                ex::photo_zone(ex::frame(x + 4.0, y + 4.0, 36.0, h - 8.0)),
                name_zone(ex::frame(x + 44.0, y + 4.0, w - 48.0, h - 8.0), 22.0),
            ]));
        }
    }
    page
}

/// Titre, puis trois rangées photo et texte, la photo changeant de côté à chaque rangée.
fn photos_and_texts() -> Page {
    let mut page = page_with_title();
    let prompts = ["Ce que nous avons fait…", "Ce que nous avons vu…", "Ce que nous avons appris…"];
    for (i, row) in [1, 3, 5].into_iter().enumerate() {
        let (photo_col, text_col) = if i % 2 == 0 { (0, 1) } else { (1, 0) };
        merge(&mut page, (photo_col, row), (photo_col, row + 1));
        let text = merge(&mut page, (text_col, row), (text_col, row + 1));
        text_zone(&mut page, &text, prompts[i], TextStyle::default_zone());
    }
    page
}

/// Titre et quatre photos, chacune avec sa légende.
fn captioned_photos() -> Page {
    let mut page = Page::new(A4_PORTRAIT);
    page.elements.push(free_title(210.0));
    for y in [45.0, 165.0] {
        for x in [13.0, 110.0] {
            page.elements.extend(ex::photo_with_caption(x, y, 87.0, 100.0));
        }
    }
    page
}

/// Affiche en paysage : titre et trois photos légendées côte à côte.
fn landscape_poster() -> Page {
    let mut page = Page::new(PageSize { width_mm: 297.0, height_mm: 210.0 });
    page.elements.push(free_title(297.0));
    for i in 0..3 {
        page.elements.extend(ex::photo_with_caption(13.0 + i as f64 * 93.0, 50.0, 85.0, 120.0));
    }
    page
}

fn builtin_info(id: &str, name: &str) -> TemplateInfo {
    TemplateInfo { id: id.into(), name: name.into(), builtin: true }
}

pub fn builtin() -> Vec<Template> {
    let mut photos = page_with_title();
    for col in 0..2 {
        for row in [1, 3, 5] {
            merge(&mut photos, (col, row), (col, row + 1));
        }
    }

    let mut story = page_with_title();
    merge(&mut story, (0, 1), (1, 4));
    let text = merge(&mut story, (0, 5), (1, 6));
    text_zone(&mut story, &text, "Ce que nous avons fait…", TextStyle::default_zone());

    let mut completion = page_with_title();
    merge(&mut completion, (0, 1), (0, 2));
    let liked = merge(&mut completion, (1, 1), (1, 2));
    text_zone(&mut completion, &liked, "Ce que j'ai préféré…", TextStyle::default_zone());
    let drawing = merge(&mut completion, (0, 3), (1, 6));
    grid::liberate(&mut completion);
    let e = completion.elements.iter_mut().find(|e| e.id == drawing).unwrap();
    e.zone = false;
    e.kind = ElementKind::Shape(ShapeProps {
        shape: ShapeKind::Rect,
        fill: None,
        stroke: Some(Stroke { width_mm: 0.5, color: "#999999".into(), style: None }),
        corner_radius_mm: 0.0,
    });
    let f = e.frame;
    completion.elements.push(Element {
        id: new_id(),
        frame: Frame { x_mm: f.x_mm + 3.0, y_mm: f.y_mm + 2.0, w_mm: 60.0, h_mm: 10.0, rotation_deg: 0.0 },
        locked: true,
        zone: false,
        group: None,
        kind: ElementKind::Text(TextProps {
            content: RichText::plain("Mon dessin"),
            style: TextStyle { font: "Andika".into(), size_pt: 12.0, color: "#999999".into(), align: Align::Left, valign: None, padding_mm: None, outline: None },
            prompt: None,
            frame: None,
        }),
    });

    vec![
        Template { info: builtin_info("grille-photos", "Grille de photos"), page: photos, assets: vec![] },
        Template { info: builtin_info("recit", "Récit"), page: story, assets: vec![] },
        Template { info: builtin_info("page-a-completer", "Page à compléter"), page: completion, assets: vec![] },
        Template { info: builtin_info("etiquettes", "Étiquettes"), page: labels(), assets: vec![] },
        Template { info: builtin_info("etiquettes-photo", "Étiquettes photo"), page: photo_labels(), assets: vec![] },
        Template { info: builtin_info("photos-et-textes", "Photos et textes"), page: photos_and_texts(), assets: vec![] },
        Template { info: builtin_info("photos-legendees", "Photos légendées"), page: captioned_photos(), assets: vec![] },
        Template { info: builtin_info("affiche-paysage", "Affiche paysage"), page: landscape_poster(), assets: vec![] },
    ]
}

/// Modèle tiré d'une page : les zones sont vidées ; parmi les autres éléments, seuls les formes et les
/// éléments verrouillés (bandeau, logo) sont gardés. Une photo ou un texte libre peut montrer un enfant
/// ou le nommer : il ne part pas dans un modèle destiné à être partagé.
pub fn from_page(name: &str, page: &Page, store: &AssetStore) -> Template {
    let mut page = page.clone();
    // Les cases des tableaux restent, vidées comme des zones.
    let cells: Vec<Id> = page
        .elements
        .iter()
        .filter_map(|e| if let ElementKind::Table(t) = &e.kind { Some(t) } else { None })
        .flat_map(|t| t.cells.iter().map(|c| c.element_id.clone()))
        .collect();
    page.elements.retain(|e| e.zone || e.locked || cells.contains(&e.id) || matches!(e.kind, ElementKind::Shape(_) | ElementKind::Image(_) | ElementKind::Table(_)));
    crate::groups::dissolve_small_groups(&mut page);
    for e in page.elements.iter_mut().filter(|e| e.zone || cells.contains(&e.id)) {
        match &mut e.kind {
            ElementKind::Photo(p) => {
                p.asset = None;
                p.crop = Crop::COVER;
            }
            ElementKind::Text(t) => t.content = RichText::default(),
            ElementKind::Shape(_) | ElementKind::Image(_) | ElementKind::Table(_) => {}
        }
    }
    let mut assets: Vec<Asset> = Vec::new();
    for id in page.elements.iter().flat_map(Element::asset_ids) {
        if let Some(a) = store.get(id)
            && !assets.iter().any(|x| x.meta.id == *id)
        {
            assets.push(a);
        }
    }
    Template { info: TemplateInfo { id: String::new(), name: name.to_string(), builtin: false }, page, assets }
}

/// Nouvelle page à partir du modèle, avec de nouveaux identifiants partout (groupes compris).
pub fn instantiate(t: &Template) -> (Page, Vec<Asset>) {
    let mut page = t.page.clone();
    renew_ids(&mut page);
    (page, t.assets.clone())
}

/// Nom de fichier lisible : minuscules ASCII et tirets.
fn slug(name: &str) -> String {
    let mut out = String::new();
    for c in name.to_lowercase().chars() {
        let c = match c {
            'à' | 'â' | 'ä' => 'a',
            'ç' => 'c',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'î' | 'ï' => 'i',
            'ô' | 'ö' => 'o',
            'ù' | 'û' | 'ü' => 'u',
            'ÿ' => 'y',
            'œ' => {
                out.push_str("oe");
                continue;
            }
            c => c,
        };
        if c.is_ascii_alphanumeric() {
            out.push(c);
        } else if !out.ends_with('-') && !out.is_empty() {
            out.push('-');
        }
    }
    let out = out.trim_end_matches('-').to_string();
    if out.is_empty() { "modele".into() } else { out }
}

pub struct Library {
    dir: PathBuf,
}

impl Library {
    pub fn open(dir: PathBuf) -> std::io::Result<Library> {
        std::fs::create_dir_all(&dir)?;
        Ok(Library { dir })
    }

    fn path(&self, id: &str) -> PathBuf {
        self.dir.join(format!("{id}.{TEMPLATE_EXT}"))
    }

    /// Un identifiant venu de l'interface ne doit désigner qu'un fichier du dossier des modèles.
    fn valid_id(id: &str) -> bool {
        !id.is_empty() && !id.contains(['/', '\\', ':']) && !id.contains("..")
    }

    fn load_file(&self, id: &str) -> Option<Template> {
        if !Self::valid_id(id) {
            return None;
        }
        let native::LoadedDocument { doc, assets, .. } = native::load_from_path(&self.path(id)).ok()?;
        let page = doc.pages.into_iter().next()?;
        Some(Template { info: TemplateInfo { id: id.to_string(), name: doc.title, builtin: false }, page, assets })
    }

    /// Les modèles intégrés d'abord, puis ceux de l'utilisateur par nom ; les fichiers illisibles sont ignorés.
    pub fn list(&self) -> Vec<TemplateInfo> {
        let mut user: Vec<TemplateInfo> = std::fs::read_dir(&self.dir)
            .into_iter()
            .flatten()
            .filter_map(|e| e.ok())
            .filter_map(|e| {
                let name = e.file_name().to_string_lossy().into_owned();
                let id = name.strip_suffix(&format!(".{TEMPLATE_EXT}"))?.to_string();
                self.load_file(&id).map(|t| t.info)
            })
            .collect();
        user.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        builtin().into_iter().map(|t| t.info).chain(user).collect()
    }

    pub fn load(&self, id: &str) -> Option<Template> {
        builtin().into_iter().find(|t| t.info.id == id).or_else(|| self.load_file(id))
    }

    pub fn save(&self, t: &Template) -> Result<TemplateInfo, NativeError> {
        let id = format!("{}-{}", slug(&t.info.name), &new_id()[..8]);
        let mut doc = Document::new(&t.info.name);
        doc.pages = vec![t.page.clone()];
        let mut store = AssetStore::default();
        for a in &t.assets {
            doc.assets.insert(a.meta.id.clone(), a.meta.clone());
            store.insert(a.clone()).map_err(|_| NativeError::Io)?;
        }
        // Les modèles ne transportent pas de police utilisateur.
        native::save_to_path(&doc, &store, &FontSet::builtin(), &self.path(&id))?;
        Ok(TemplateInfo { id, name: t.info.name.clone(), builtin: false })
    }

    pub fn delete(&self, id: &str) -> std::io::Result<()> {
        if builtin().iter().any(|t| t.info.id == id) {
            return Err(std::io::Error::new(std::io::ErrorKind::PermissionDenied, "modèle intégré"));
        }
        if !Self::valid_id(id) {
            return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "identifiant de modèle invalide"));
        }
        std::fs::remove_file(self.path(id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::photo::import_bytes;
    use image::{ImageFormat, Rgb, RgbImage};
    use std::collections::HashSet;
    use std::io::Cursor;

    fn photo_zones(p: &Page) -> usize {
        p.elements.iter().filter(|e| e.zone && matches!(e.kind, ElementKind::Photo(_))).count()
    }

    fn has_title(p: &Page) -> bool {
        p.elements.iter().any(|e| {
            e.zone
                && matches!(&e.kind, ElementKind::Text(t)
                    if t.prompt.as_deref() == Some("Titre") && t.style.font == "Playwrite FR Moderne" && t.style.size_pt == 24.0 && t.style.align == Align::Center)
        })
    }

    fn ids(p: &Page) -> HashSet<String> {
        let mut s: HashSet<String> = p.elements.iter().map(|e| e.id.clone()).collect();
        s.insert(p.id.clone());
        s
    }

    fn logo_asset() -> Asset {
        let mut out = Cursor::new(Vec::new());
        RgbImage::from_pixel(20, 20, Rgb([200, 0, 0])).write_to(&mut out, ImageFormat::Png).unwrap();
        import_bytes(&out.into_inner()).unwrap()
    }

    #[test]
    fn eight_builtins_with_fixed_ids() {
        let ids: Vec<String> = builtin().into_iter().map(|t| t.info.id).collect();
        assert_eq!(ids, ["grille-photos", "recit", "page-a-completer", "etiquettes", "etiquettes-photo", "photos-et-textes", "photos-legendees", "affiche-paysage"]);
        assert!(builtin().iter().all(|t| t.info.builtin));
        // Les planches d'étiquettes n'ont pas de titre : toute la page sert aux étiquettes.
        assert!(builtin().iter().filter(|t| !t.info.id.starts_with("etiquettes")).all(|t| has_title(&t.page)));
    }

    fn by_id(id: &str) -> Template {
        builtin().into_iter().find(|t| t.info.id == id).unwrap()
    }

    fn name_zones(p: &Page) -> Vec<&Element> {
        p.elements.iter().filter(|e| e.zone && matches!(&e.kind, ElementKind::Text(t) if t.prompt.as_deref() == Some("Prénom"))).collect()
    }

    /// Tout reste à 8 mm au moins des bords, la marge des imprimantes.
    #[test]
    fn builtin_elements_stay_inside_the_printable_area() {
        for t in builtin() {
            let s = t.page.size;
            for e in &t.page.elements {
                let f = e.frame;
                assert!(f.x_mm >= 8.0 && f.y_mm >= 8.0 && f.x_mm + f.w_mm <= s.width_mm - 8.0 && f.y_mm + f.h_mm <= s.height_mm - 8.0, "{} : {f:?}", t.info.id);
            }
        }
    }

    #[test]
    fn labels_are_twelve_framed_name_zones_centred_in_their_frame() {
        let t = by_id("etiquettes");
        let names = name_zones(&t.page);
        assert_eq!(names.len(), 12);
        for e in names {
            let ElementKind::Text(text) = &e.kind else { unreachable!() };
            assert!(text.frame.as_ref().is_some_and(|f| f.stroke.is_some() && f.corner_radius_mm > 0.0));
            assert_eq!((text.style.align, text.style.valign), (Align::Center, Some(VAlign::Middle)));
        }
    }

    #[test]
    fn photo_labels_group_a_border_a_photo_and_a_name() {
        let t = by_id("etiquettes-photo");
        assert_eq!(name_zones(&t.page).len(), 10);
        assert_eq!(photo_zones(&t.page), 10);
        let groups: HashSet<&String> = t.page.elements.iter().filter_map(|e| e.group.as_ref()).collect();
        assert_eq!(groups.len(), 10);
        for g in groups {
            let kinds: Vec<&ElementKind> = t.page.elements.iter().filter(|e| e.group.as_ref() == Some(g)).map(|e| &e.kind).collect();
            assert_eq!(kinds.len(), 3);
            assert!(matches!(kinds[0], ElementKind::Shape(s) if s.shape == ShapeKind::Rect && s.stroke.is_some()));
        }
    }

    #[test]
    fn photos_and_texts_alternate_sides() {
        let t = by_id("photos-et-textes");
        assert!(t.page.grid.is_some());
        let mut photos: Vec<&Element> = t.page.elements.iter().filter(|e| e.zone && matches!(e.kind, ElementKind::Photo(_))).collect();
        photos.sort_by(|a, b| a.frame.y_mm.total_cmp(&b.frame.y_mm));
        let lefts: Vec<bool> = photos.iter().map(|e| e.frame.x_mm < 100.0).collect();
        assert_eq!(lefts, [true, false, true]);
        let texts = t.page.elements.iter().filter(|e| e.zone && matches!(&e.kind, ElementKind::Text(x) if x.prompt.as_deref() != Some("Titre"))).count();
        assert_eq!(texts, 3);
    }

    fn captions_under_photos(p: &Page) -> usize {
        let photos = p.elements.iter().filter(|e| e.zone && matches!(e.kind, ElementKind::Photo(_)));
        photos
            .filter(|ph| {
                p.elements.iter().any(|c| {
                    c.group.is_some()
                        && c.group == ph.group
                        && c.frame.x_mm == ph.frame.x_mm
                        && c.frame.w_mm == ph.frame.w_mm
                        && c.frame.y_mm > ph.frame.y_mm + ph.frame.h_mm
                        && matches!(&c.kind, ElementKind::Text(t) if t.prompt.as_deref() == Some("Légende…"))
                })
            })
            .count()
    }

    #[test]
    fn captioned_photos_have_four_photos_each_with_its_caption() {
        let t = by_id("photos-legendees");
        assert_eq!(photo_zones(&t.page), 4);
        assert_eq!(captions_under_photos(&t.page), 4);
    }

    #[test]
    fn landscape_poster_has_three_captioned_photos() {
        let t = by_id("affiche-paysage");
        assert_eq!((t.page.size.width_mm, t.page.size.height_mm), (297.0, 210.0));
        assert_eq!(photo_zones(&t.page), 3);
        assert_eq!(captions_under_photos(&t.page), 3);
    }

    #[test]
    fn instantiate_renews_group_ids() {
        let t = by_id("etiquettes-photo");
        let (a, _) = instantiate(&t);
        let (b, _) = instantiate(&t);
        let groups = |p: &Page| p.elements.iter().filter_map(|e| e.group.clone()).collect::<HashSet<_>>();
        assert_eq!(groups(&a).len(), 10);
        assert!(groups(&a).is_disjoint(&groups(&t.page)));
        assert!(groups(&a).is_disjoint(&groups(&b)));
    }

    #[test]
    fn photo_grid_has_title_and_six_photo_zones() {
        let t = &builtin()[0];
        assert_eq!(photo_zones(&t.page), 6);
        assert!(t.page.grid.is_some());
    }

    #[test]
    fn story_has_one_big_photo_and_a_text_zone() {
        let t = &builtin()[1];
        assert_eq!(photo_zones(&t.page), 1);
        assert!(t.page.elements.iter().any(|e| matches!(&e.kind, ElementKind::Text(t) if t.prompt.as_deref() == Some("Ce que nous avons fait…"))));
    }

    #[test]
    fn completion_page_draws_a_printable_box() {
        let t = &builtin()[2];
        assert!(t.page.grid.is_none());
        assert!(t.page.elements.iter().any(|e| !e.zone && matches!(&e.kind, ElementKind::Shape(s) if s.shape == ShapeKind::Rect && s.stroke.is_some())));
        assert!(t.page.elements.iter().any(|e| e.locked && matches!(&e.kind, ElementKind::Text(t) if !t.content.is_blank())));
    }

    #[test]
    fn instantiate_gives_fresh_ids_each_time() {
        let t = &builtin()[0];
        let (a, _) = instantiate(t);
        let (b, _) = instantiate(t);
        assert!(ids(&a).is_disjoint(&ids(&t.page)));
        assert!(ids(&a).is_disjoint(&ids(&b)));
        let g = a.grid.as_ref().unwrap();
        assert!(g.cells.iter().all(|c| a.elements.iter().any(|e| e.id == c.element_id)));
    }

    #[test]
    fn from_page_empties_zones_but_keeps_fixed_elements() {
        let mut store = AssetStore::default();
        let photo = logo_asset();
        let logo = logo_asset();
        let (photo_id, logo_id) = (photo.meta.id.clone(), logo.meta.id.clone());
        store.insert(photo).unwrap();
        store.insert(logo).unwrap();
        let (mut page, _) = instantiate(&builtin()[0]);
        for e in page.elements.iter_mut().filter(|e| e.zone) {
            match &mut e.kind {
                ElementKind::Photo(p) => p.asset = Some(photo_id.clone()),
                ElementKind::Text(t) => t.content = RichText::plain("Rempli"),
                _ => {}
            }
        }
        page.elements.push(Element {
            id: new_id(),
            frame: Frame { x_mm: 180.0, y_mm: 5.0, w_mm: 20.0, h_mm: 20.0, rotation_deg: 0.0 },
            locked: true,
            zone: false,
            group: None,
            kind: ElementKind::Photo(PhotoProps { asset: Some(logo_id.clone()), crop: Crop::COVER, corner_radius_mm: 0.0, border: None, shadow: false }),
        });
        let t = from_page("Mon modèle", &page, &store);
        assert_eq!(t.info.name, "Mon modèle");
        assert!(!t.info.builtin);
        for e in t.page.elements.iter().filter(|e| e.zone) {
            match &e.kind {
                ElementKind::Photo(p) => assert!(p.asset.is_none()),
                ElementKind::Text(t) => assert!(t.content.is_blank()),
                _ => {}
            }
        }
        assert!(t.page.elements.iter().any(|e| matches!(&e.kind, ElementKind::Photo(p) if p.asset.as_deref() == Some(logo_id.as_str()))));
        assert_eq!(t.assets.len(), 1);
        assert_eq!(t.assets[0].meta.id, logo_id);
    }

    #[test]
    fn from_page_drops_free_photos_and_texts() {
        let mut store = AssetStore::default();
        let child = logo_asset();
        let child_id = child.meta.id.clone();
        store.insert(child).unwrap();
        let mut page = Page::new(A4_PORTRAIT);
        let free = |kind| Element {
            id: new_id(),
            frame: Frame { x_mm: 10.0, y_mm: 10.0, w_mm: 50.0, h_mm: 50.0, rotation_deg: 0.0 },
            locked: false,
            zone: false,
            group: None,
            kind,
        };
        page.elements.push(free(ElementKind::Photo(PhotoProps { asset: Some(child_id.clone()), crop: Crop::COVER, corner_radius_mm: 0.0, border: None, shadow: false })));
        page.elements.push(free(ElementKind::Text(TextProps { content: RichText::plain("Léa a ri"), style: TextStyle::default_zone(), prompt: None, frame: None })));
        page.elements.push(free(ElementKind::Shape(ShapeProps { shape: ShapeKind::Rect, fill: None, stroke: None, corner_radius_mm: 0.0 })));
        let t = from_page("Partagé", &page, &store);
        assert_eq!(t.page.elements.len(), 1);
        assert!(matches!(t.page.elements[0].kind, ElementKind::Shape(_)));
        assert!(t.assets.is_empty());
    }

    #[test]
    fn library_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let lib = Library::open(dir.path().join("modeles")).unwrap();
        let (page, _) = instantiate(&builtin()[1]);
        let t = from_page("Récit de la semaine", &page, &AssetStore::default());
        let info = lib.save(&t).unwrap();
        assert!(info.id.starts_with("recit-de-la-semaine-"));
        let list = lib.list();
        assert_eq!(list.len(), builtin().len() + 1);
        assert_eq!(list[builtin().len()].name, "Récit de la semaine");
        let back = lib.load(&info.id).unwrap();
        assert_eq!(back.page, t.page);
        assert_eq!(back.info, info);
        assert!(lib.load("recit").unwrap().info.builtin);
    }

    #[test]
    fn library_skips_unreadable_files() {
        let dir = tempfile::tempdir().unwrap();
        let lib = Library::open(dir.path().to_path_buf()).unwrap();
        std::fs::write(dir.path().join("abime.carnet-modele"), b"pas un zip").unwrap();
        assert_eq!(lib.list().len(), builtin().len());
        assert!(lib.load("abime").is_none());
    }

    #[test]
    fn template_ids_cannot_leave_the_library() {
        let dir = tempfile::tempdir().unwrap();
        let lib = Library::open(dir.path().join("modeles")).unwrap();
        std::fs::write(dir.path().join("dehors.carnet-modele"), b"x").unwrap();
        assert!(lib.load("../dehors").is_none());
        assert_eq!(lib.delete("../dehors").unwrap_err().kind(), std::io::ErrorKind::InvalidInput);
        assert!(dir.path().join("dehors.carnet-modele").exists());
    }

    #[test]
    fn builtins_cannot_be_deleted() {
        let dir = tempfile::tempdir().unwrap();
        let lib = Library::open(dir.path().to_path_buf()).unwrap();
        assert_eq!(lib.delete("recit").unwrap_err().kind(), std::io::ErrorKind::PermissionDenied);
    }

    #[test]
    fn user_templates_can_be_deleted() {
        let dir = tempfile::tempdir().unwrap();
        let lib = Library::open(dir.path().to_path_buf()).unwrap();
        let (page, _) = instantiate(&builtin()[0]);
        let info = lib.save(&from_page("À jeter", &page, &AssetStore::default())).unwrap();
        lib.delete(&info.id).unwrap();
        assert_eq!(lib.list().len(), builtin().len());
    }

    /// Modèle d'étiquettes : une zone de texte vidée garde son cadre et l'image de fond, qui part avec le modèle.
    #[test]
    fn from_page_keeps_text_frame_images() {
        let mut page = Page::new(A4_PORTRAIT);
        let label = logo_asset();
        let id = label.meta.id.clone();
        let mut store = AssetStore::default();
        store.insert(label).unwrap();
        page.elements.push(Element {
            id: new_id(),
            frame: Frame { x_mm: 10.0, y_mm: 10.0, w_mm: 60.0, h_mm: 25.0, rotation_deg: 0.0 },
            locked: false,
            zone: true,
            group: None,
            kind: ElementKind::Text(TextProps {
                content: RichText::plain("Léa"),
                style: TextStyle::default_zone(),
                prompt: Some("Prénom".into()),
                frame: Some(TextFrame { image: Some(id.clone()), ..TextFrame::default() }),
            }),
        });
        let t = from_page("étiquettes", &page, &store);
        let ElementKind::Text(text) = &t.page.elements[0].kind else { panic!() };
        assert!(text.content.is_blank());
        assert_eq!(text.frame.as_ref().and_then(|f| f.image.clone()), Some(id.clone()));
        assert_eq!(t.assets.iter().map(|a| a.meta.id.clone()).collect::<Vec<_>>(), vec![id]);
    }

    #[test]
    fn from_page_keeps_images_and_dissolves_orphans() {
        let mut page = Page::new(A4_PORTRAIT);
        let base = |kind: ElementKind| Element {
            id: new_id(),
            frame: Frame { x_mm: 10.0, y_mm: 10.0, w_mm: 20.0, h_mm: 20.0, rotation_deg: 0.0 },
            locked: false,
            zone: false,
            group: Some("g".into()),
            kind,
        };
        page.elements.push(base(ElementKind::Image(ImageProps { code: "1F98A".into() })));
        page.elements.push(base(ElementKind::Photo(PhotoProps { asset: None, crop: Crop::COVER, corner_radius_mm: 0.0, border: None, shadow: false })));
        let t = from_page("m", &page, &AssetStore::default());
        assert_eq!(t.page.elements.len(), 1);
        assert!(matches!(t.page.elements[0].kind, ElementKind::Image(_)));
        assert_eq!(t.page.elements[0].group, None);
    }
}
