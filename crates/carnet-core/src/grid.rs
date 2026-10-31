//! Grille du mode modèle : poser, fusionner, scinder, typer et libérer des cases.

use crate::model::*;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

pub const GRID_MARGIN_MM: f64 = 10.0;
pub const GRID_GAP_MM: f64 = 4.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GridError {
    NoGrid,
    UnknownCell,
    NotRectangular,
    NotSpanned,
}

impl GridError {
    pub fn message_fr(&self) -> &'static str {
        match self {
            GridError::NoGrid => "Cette page n'a pas de grille.",
            GridError::UnknownCell => "Cette case n'existe pas dans la grille.",
            GridError::NotRectangular => {
                "Impossible de fusionner : le résultat ne serait pas un rectangle. Scindez d'abord les cases fusionnées qui dépassent."
            }
            GridError::NotSpanned => "Cette case n'est pas fusionnée.",
        }
    }
}

fn empty_photo_zone() -> Element {
    Element {
        id: new_id(),
        frame: Frame { x_mm: 0.0, y_mm: 0.0, w_mm: 0.0, h_mm: 0.0, rotation_deg: 0.0 },
        locked: false,
        zone: true,
        group: None,
        kind: ElementKind::Photo(PhotoProps {
            asset: None,
            crop: Crop::COVER,
            corner_radius_mm: 0.0,
            border: None,
            shadow: false,
            mirrored: false,
        }),
    }
}

fn empty_kind(kind: ZoneKind) -> ElementKind {
    match kind {
        ZoneKind::Photo => empty_photo_zone().kind,
        ZoneKind::Text => ElementKind::Text(TextProps {
            content: RichText::default(),
            style: TextStyle::default_zone(),
            prompt: Some("Texte".into()),
            frame: None,
        }),
    }
}

/// Même type et même style, contenu vidé.
fn emptied_copy(e: &Element) -> Element {
    let kind = match &e.kind {
        ElementKind::Photo(p) => ElementKind::Photo(PhotoProps { asset: None, crop: Crop::COVER, ..p.clone() }),
        ElementKind::Text(t) => ElementKind::Text(TextProps { content: RichText::default(), ..t.clone() }),
        ElementKind::Shape(s) => ElementKind::Shape(s.clone()),
        ElementKind::Image(i) => ElementKind::Image(i.clone()),
        ElementKind::Table(t) => ElementKind::Table(t.clone()),
    };
    Element { id: new_id(), kind, ..e.clone() }
}

fn grid_of(page: &mut Page) -> Result<&mut GridLayout, GridError> {
    page.grid.as_mut().ok_or(GridError::NoGrid)
}

fn cell_index(grid: &GridLayout, id: &str) -> Result<usize, GridError> {
    grid.cells.iter().position(|c| c.element_id == id).ok_or(GridError::UnknownCell)
}

/// Recalcule les cadres des éléments placés par la grille.
fn layout(page: &mut Page) {
    let Some(g) = &page.grid else { return };
    let cw = (page.size.width_mm - 2.0 * g.margin_mm - (g.cols as f64 - 1.0) * g.gap_mm) / g.cols as f64;
    let ch = (page.size.height_mm - 2.0 * g.margin_mm - (g.rows as f64 - 1.0) * g.gap_mm) / g.rows as f64;
    for c in &g.cells {
        if let Some(e) = page.elements.iter_mut().find(|e| e.id == c.element_id) {
            e.frame = Frame {
                x_mm: g.margin_mm + c.col as f64 * (cw + g.gap_mm),
                y_mm: g.margin_mm + c.row as f64 * (ch + g.gap_mm),
                w_mm: c.col_span as f64 * cw + (c.col_span as f64 - 1.0) * g.gap_mm,
                h_mm: c.row_span as f64 * ch + (c.row_span as f64 - 1.0) * g.gap_mm,
                rotation_deg: 0.0,
            };
        }
    }
}

/// Remplace la grille actuelle (et ses zones) par une grille de cases photo vides.
pub fn set_grid(page: &mut Page, cols: u8, rows: u8) {
    if let Some(old) = page.grid.take() {
        page.elements.retain(|e| !old.cells.iter().any(|c| c.element_id == e.id));
    }
    let mut cells = Vec::new();
    for row in 0..rows {
        for col in 0..cols {
            let e = empty_photo_zone();
            cells.push(GridCell { element_id: e.id.clone(), col, row, col_span: 1, row_span: 1 });
            page.elements.push(e);
        }
    }
    page.grid = Some(GridLayout { cols, rows, margin_mm: GRID_MARGIN_MM, gap_mm: GRID_GAP_MM, cells });
    layout(page);
}

/// Grilles toutes prêtes du mode modèle.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, TS)]
#[ts(export)]
pub enum GridPreset {
    One,
    TwoByTwo,
    TwoByThree,
    OnePlusTwo,
    OnePlusTwoSide,
    ThreeByThree,
}

/// Pose la grille (remplace l'actuelle), puis fait les fusions du modèle choisi.
pub fn apply_preset(page: &mut Page, preset: GridPreset) {
    let (cols, rows, merge) = match preset {
        GridPreset::One => (1, 1, None),
        GridPreset::TwoByTwo => (2, 2, None),
        GridPreset::TwoByThree => (2, 3, None),
        GridPreset::OnePlusTwo => (2, 2, Some(((0, 0), (1, 0)))),
        GridPreset::OnePlusTwoSide => (2, 2, Some(((0, 0), (0, 1)))),
        GridPreset::ThreeByThree => (3, 3, None),
    };
    set_grid(page, cols, rows);
    if let Some(((c0, r0), (c1, r1))) = merge {
        let id_at = |p: &Page, col: u8, row: u8| {
            p.grid.as_ref().and_then(|g| g.cells.iter().find(|c| c.col == col && c.row == row)).map(|c| c.element_id.clone())
        };
        if let (Some(a), Some(b)) = (id_at(page, c0, r0), id_at(page, c1, r1)) {
            merge_cells(page, &a, &b).expect("fusion d'une grille toute prête");
        }
    }
}

/// Fusionne `a` et `b` en leur rectangle englobant ; la case `a` garde son type.
pub fn merge_cells(page: &mut Page, a: &str, b: &str) -> Result<(), GridError> {
    let g = grid_of(page)?;
    let (ia, ib) = (cell_index(g, a)?, cell_index(g, b)?);
    let (ca, cb) = (&g.cells[ia], &g.cells[ib]);
    let c0 = ca.col.min(cb.col);
    let r0 = ca.row.min(cb.row);
    let c1 = (ca.col + ca.col_span).max(cb.col + cb.col_span);
    let r1 = (ca.row + ca.row_span).max(cb.row + cb.row_span);
    let mut absorbed = Vec::new();
    for c in &g.cells {
        let inside = c.col >= c0 && c.col + c.col_span <= c1 && c.row >= r0 && c.row + c.row_span <= r1;
        let outside = c.col + c.col_span <= c0 || c.col >= c1 || c.row + c.row_span <= r0 || c.row >= r1;
        if inside {
            if c.element_id != a {
                absorbed.push(c.element_id.clone());
            }
        } else if !outside {
            return Err(GridError::NotRectangular);
        }
    }
    g.cells.retain(|c| !absorbed.contains(&c.element_id));
    let ca = g.cells.iter_mut().find(|c| c.element_id == a).unwrap();
    *ca = GridCell { element_id: a.to_string(), col: c0, row: r0, col_span: c1 - c0, row_span: r1 - r0 };
    page.elements.retain(|e| !absorbed.contains(&e.id));
    layout(page);
    Ok(())
}

/// Scinde une case fusionnée en cases unitaires ; l'élément d'origine garde le coin haut gauche.
pub fn split_cell(page: &mut Page, cell: &str) -> Result<(), GridError> {
    let g = grid_of(page)?;
    let i = cell_index(g, cell)?;
    let c = g.cells[i].clone();
    if c.col_span == 1 && c.row_span == 1 {
        return Err(GridError::NotSpanned);
    }
    g.cells[i].col_span = 1;
    g.cells[i].row_span = 1;
    let original = page.elements.iter().find(|e| e.id == cell).cloned().ok_or(GridError::UnknownCell)?;
    let g = page.grid.as_mut().unwrap();
    for row in c.row..c.row + c.row_span {
        for col in c.col..c.col + c.col_span {
            if (col, row) == (c.col, c.row) {
                continue;
            }
            let e = emptied_copy(&original);
            g.cells.push(GridCell { element_id: e.id.clone(), col, row, col_span: 1, row_span: 1 });
            page.elements.push(e);
        }
    }
    layout(page);
    Ok(())
}

/// Change le type de la zone d'une case ; son contenu est vidé.
pub fn set_cell_kind(page: &mut Page, cell: &str, kind: ZoneKind) -> Result<(), GridError> {
    let g = grid_of(page)?;
    cell_index(g, cell)?;
    let e = page.elements.iter_mut().find(|e| e.id == cell).ok_or(GridError::UnknownCell)?;
    e.kind = empty_kind(kind);
    Ok(())
}

/// Change la taille de la page (portrait ou paysage) ; les cases de la grille suivent.
pub fn set_page_size(page: &mut Page, size: PageSize) {
    page.size = size;
    layout(page);
}

/// Abandonne la grille : les zones gardent leur cadre et deviennent librement déplaçables.
pub fn liberate(page: &mut Page) {
    page.grid = None;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn a4() -> Page {
        Page::new(A4_PORTRAIT)
    }

    fn cell_at(page: &Page, col: u8, row: u8) -> String {
        page.grid
            .as_ref()
            .unwrap()
            .cells
            .iter()
            .find(|c| c.col == col && c.row == row)
            .unwrap()
            .element_id
            .clone()
    }

    fn frame_of<'a>(page: &'a Page, id: &str) -> &'a Frame {
        &page.elements.iter().find(|e| e.id == id).unwrap().frame
    }

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-3
    }

    #[test]
    fn set_grid_2x3_on_a4() {
        let mut p = a4();
        set_grid(&mut p, 2, 3);
        assert_eq!(p.grid.as_ref().unwrap().cells.len(), 6);
        assert_eq!(p.elements.len(), 6);
        let f = *frame_of(&p, &cell_at(&p, 0, 0));
        assert!(close(f.x_mm, 10.0) && close(f.y_mm, 10.0) && close(f.w_mm, 93.0) && close(f.h_mm, 89.667), "{f:?}");
        let f = *frame_of(&p, &cell_at(&p, 1, 2));
        assert!(close(f.x_mm, 107.0) && close(f.y_mm, 197.333), "{f:?}");
        let e = &p.elements[0];
        assert!(e.zone);
        assert!(matches!(&e.kind, ElementKind::Photo(ph) if ph.asset.is_none()));
    }

    #[test]
    fn set_grid_keeps_elements_outside_the_grid() {
        let mut p = a4();
        let band = Element {
            id: "bandeau".into(),
            frame: Frame { x_mm: 0.0, y_mm: 0.0, w_mm: 210.0, h_mm: 20.0, rotation_deg: 0.0 },
            locked: true,
            zone: false,
            group: None,
            kind: ElementKind::Shape(ShapeProps { shape: ShapeKind::Rect, fill: None, stroke: None, corner_radius_mm: 0.0 }),
        };
        p.elements.push(band);
        set_grid(&mut p, 2, 3);
        set_grid(&mut p, 1, 2);
        assert!(p.elements.iter().any(|e| e.id == "bandeau"));
    }

    #[test]
    fn set_grid_replaces_previous_grid_elements() {
        let mut p = a4();
        set_grid(&mut p, 2, 3);
        set_grid(&mut p, 1, 2);
        assert_eq!(p.elements.iter().filter(|e| e.zone).count(), 2);
    }

    #[test]
    fn merge_two_cells_of_first_row() {
        let mut p = a4();
        set_grid(&mut p, 2, 3);
        let (a, b) = (cell_at(&p, 0, 0), cell_at(&p, 1, 0));
        merge_cells(&mut p, &a, &b).unwrap();
        let g = p.grid.as_ref().unwrap();
        assert_eq!(g.cells.len(), 5);
        assert_eq!(p.elements.len(), 5);
        let c = g.cells.iter().find(|c| c.element_id == a).unwrap();
        assert_eq!(c.col_span, 2);
        assert!(close(frame_of(&p, &a).w_mm, 190.0));
    }

    #[test]
    fn merge_diagonal_absorbs_inner_cells() {
        let mut p = a4();
        set_grid(&mut p, 2, 3);
        let (a, b) = (cell_at(&p, 0, 0), cell_at(&p, 1, 1));
        merge_cells(&mut p, &a, &b).unwrap();
        let g = p.grid.as_ref().unwrap();
        assert_eq!(g.cells.len(), 3);
        let c = g.cells.iter().find(|c| c.element_id == a).unwrap();
        assert_eq!((c.col_span, c.row_span), (2, 2));
    }

    #[test]
    fn merge_refuses_non_rectangular() {
        let mut p = a4();
        set_grid(&mut p, 3, 3);
        let (a, b) = (cell_at(&p, 1, 0), cell_at(&p, 2, 0));
        merge_cells(&mut p, &a, &b).unwrap();
        let (c, d) = (cell_at(&p, 0, 0), cell_at(&p, 1, 1));
        assert_eq!(merge_cells(&mut p, &c, &d), Err(GridError::NotRectangular));
    }

    #[test]
    fn split_restores_unit_cells() {
        let mut p = a4();
        set_grid(&mut p, 2, 3);
        let (a, b) = (cell_at(&p, 0, 0), cell_at(&p, 1, 1));
        merge_cells(&mut p, &a, &b).unwrap();
        split_cell(&mut p, &a).unwrap();
        let g = p.grid.as_ref().unwrap();
        assert_eq!(g.cells.len(), 6);
        assert!(g.cells.iter().all(|c| c.col_span == 1 && c.row_span == 1));
        assert_eq!(cell_at(&p, 0, 0), a);
        assert_eq!(p.elements.len(), 6);
    }

    #[test]
    fn split_unit_cell_is_an_error() {
        let mut p = a4();
        set_grid(&mut p, 2, 3);
        let a = cell_at(&p, 0, 0);
        assert_eq!(split_cell(&mut p, &a), Err(GridError::NotSpanned));
    }

    #[test]
    fn set_kind_to_text() {
        let mut p = a4();
        set_grid(&mut p, 2, 3);
        let a = cell_at(&p, 0, 0);
        let before = *frame_of(&p, &a);
        set_cell_kind(&mut p, &a, ZoneKind::Text).unwrap();
        let e = p.elements.iter().find(|e| e.id == a).unwrap();
        assert!(matches!(&e.kind, ElementKind::Text(t) if t.prompt.as_deref() == Some("Texte")));
        assert_eq!(e.frame, before);
    }

    #[test]
    fn liberate_keeps_frames_and_drops_grid() {
        let mut p = a4();
        set_grid(&mut p, 2, 3);
        let frames: Vec<Frame> = p.elements.iter().map(|e| e.frame).collect();
        liberate(&mut p);
        assert!(p.grid.is_none());
        assert_eq!(p.elements.iter().map(|e| e.frame).collect::<Vec<_>>(), frames);
        assert!(p.elements.iter().all(|e| e.zone));
    }

    #[test]
    fn resizing_the_page_relays_out_the_grid() {
        let mut p = a4();
        set_grid(&mut p, 2, 3);
        set_page_size(&mut p, PageSize { width_mm: 297.0, height_mm: 210.0 });
        assert_eq!(p.size.width_mm, 297.0);
        for e in &p.elements {
            assert!(e.frame.x_mm + e.frame.w_mm <= 297.0 - GRID_MARGIN_MM + 1e-6);
            assert!(e.frame.y_mm + e.frame.h_mm <= 210.0 - GRID_MARGIN_MM + 1e-6, "{:?}", e.frame);
        }
    }

    #[test]
    fn errors_without_grid() {
        let mut p = a4();
        assert_eq!(merge_cells(&mut p, "x", "y"), Err(GridError::NoGrid));
    }

    #[test]
    fn unknown_cell_is_an_error() {
        let mut p = a4();
        set_grid(&mut p, 2, 3);
        assert_eq!(split_cell(&mut p, "inconnue"), Err(GridError::UnknownCell));
    }

    #[test]
    fn presets_have_expected_cells() {
        let spans = |preset: GridPreset| {
            let mut p = a4();
            apply_preset(&mut p, preset);
            let g = p.grid.unwrap();
            assert_eq!(p.elements.len(), g.cells.len());
            g.cells.iter().map(|c| (c.col_span, c.row_span)).collect::<Vec<_>>()
        };
        assert_eq!(spans(GridPreset::One), [(1, 1)]);
        assert_eq!(spans(GridPreset::TwoByTwo).len(), 4);
        assert_eq!(spans(GridPreset::TwoByThree).len(), 6);
        let one_plus_two = spans(GridPreset::OnePlusTwo);
        assert_eq!(one_plus_two.len(), 3);
        assert!(one_plus_two.contains(&(2, 1)));
        let side = spans(GridPreset::OnePlusTwoSide);
        assert_eq!(side.len(), 3);
        assert!(side.contains(&(1, 2)));
        assert_eq!(spans(GridPreset::ThreeByThree).len(), 9);
    }
}
