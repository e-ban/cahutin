//! Modèles : trois modèles intégrés, et la bibliothèque des modèles de l'utilisateur.

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
    TextStyle { font: "Playwrite FR Moderne".into(), size_pt: 24.0, color: "#c0392b".into(), align: Align::Center }
}

/// Grille 2 × 7 : une ligne de titre, puis des cases de deux lignes pour garder des photos bien proportionnées.
fn page_with_title() -> Page {
    let mut page = Page::new(A4_PORTRAIT);
    grid::set_grid(&mut page, 2, 7);
    let title = merge(&mut page, (0, 0), (1, 0));
    text_zone(&mut page, &title, "Titre", title_style());
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
        stroke: Some(Stroke { width_mm: 0.5, color: "#999999".into() }),
        corner_radius_mm: 0.0,
    });
    let f = e.frame;
    completion.elements.push(Element {
        id: new_id(),
        frame: Frame { x_mm: f.x_mm + 3.0, y_mm: f.y_mm + 2.0, w_mm: 60.0, h_mm: 10.0, rotation_deg: 0.0 },
        locked: true,
        zone: false,
        kind: ElementKind::Text(TextProps {
            content: RichText::plain("Mon dessin"),
            style: TextStyle { font: "Andika".into(), size_pt: 12.0, color: "#999999".into(), align: Align::Left },
            prompt: None,
        }),
    });

    vec![
        Template { info: builtin_info("grille-photos", "Grille de photos"), page: photos, assets: vec![] },
        Template { info: builtin_info("recit", "Récit"), page: story, assets: vec![] },
        Template { info: builtin_info("page-a-completer", "Page à compléter"), page: completion, assets: vec![] },
    ]
}

fn photo_asset(e: &Element) -> Option<&Id> {
    match &e.kind {
        ElementKind::Photo(p) => p.asset.as_ref(),
        _ => None,
    }
}

/// Modèle tiré d'une page : les zones sont vidées, les éléments fixes gardés avec leurs images.
pub fn from_page(name: &str, page: &Page, store: &AssetStore) -> Template {
    let mut page = page.clone();
    for e in page.elements.iter_mut().filter(|e| e.zone) {
        match &mut e.kind {
            ElementKind::Photo(p) => {
                p.asset = None;
                p.crop = Crop::COVER;
            }
            ElementKind::Text(t) => t.content = RichText::default(),
            ElementKind::Shape(_) => {}
        }
    }
    let mut assets: Vec<Asset> = Vec::new();
    for id in page.elements.iter().filter_map(photo_asset) {
        if let Some(a) = store.get(id)
            && !assets.iter().any(|x| x.meta.id == *id)
        {
            assets.push(a.clone());
        }
    }
    Template { info: TemplateInfo { id: String::new(), name: name.to_string(), builtin: false }, page, assets }
}

/// Nouvelle page à partir du modèle, avec de nouveaux identifiants partout.
pub fn instantiate(t: &Template) -> (Page, Vec<Asset>) {
    let mut page = t.page.clone();
    page.id = new_id();
    for e in &mut page.elements {
        let old = std::mem::replace(&mut e.id, new_id());
        if let Some(g) = &mut page.grid
            && let Some(c) = g.cells.iter_mut().find(|c| c.element_id == old)
        {
            c.element_id = e.id.clone();
        }
    }
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

    fn load_file(&self, id: &str) -> Option<Template> {
        let (doc, assets) = native::load_from_path(&self.path(id)).ok()?;
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
            store.insert(a.clone());
        }
        native::save_to_path(&doc, &store, &self.path(&id))?;
        Ok(TemplateInfo { id, name: t.info.name.clone(), builtin: false })
    }

    pub fn delete(&self, id: &str) -> std::io::Result<()> {
        if builtin().iter().any(|t| t.info.id == id) {
            return Err(std::io::Error::new(std::io::ErrorKind::PermissionDenied, "modèle intégré"));
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
    fn three_builtins_with_fixed_ids() {
        let ids: Vec<String> = builtin().into_iter().map(|t| t.info.id).collect();
        assert_eq!(ids, ["grille-photos", "recit", "page-a-completer"]);
        assert!(builtin().iter().all(|t| t.info.builtin && has_title(&t.page)));
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
        store.insert(photo);
        store.insert(logo);
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
    fn library_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let lib = Library::open(dir.path().join("modeles")).unwrap();
        let (page, _) = instantiate(&builtin()[1]);
        let t = from_page("Récit de la semaine", &page, &AssetStore::default());
        let info = lib.save(&t).unwrap();
        assert!(info.id.starts_with("recit-de-la-semaine-"));
        let list = lib.list();
        assert_eq!(list.len(), 4);
        assert_eq!(list[3].name, "Récit de la semaine");
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
        assert_eq!(lib.list().len(), 3);
        assert!(lib.load("abime").is_none());
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
        assert_eq!(lib.list().len(), 3);
    }
}
