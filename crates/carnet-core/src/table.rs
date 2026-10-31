//! Tableaux : un élément `Table` (grille et traits) et ses cases, éléments ordinaires du même groupe.

use crate::model::*;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TableError {
    UnknownTable,
    UnknownCell,
    NotRectangular,
    NotSpanned,
    LastLine,
    Locked,
    TooMany,
}

/// Lignes ou colonnes d'un tableau, au plus.
pub const MAX_LINES: u8 = 50;

impl TableError {
    pub fn message_fr(&self) -> &'static str {
        match self {
            TableError::UnknownTable => "Ce tableau n'existe pas sur la page.",
            TableError::UnknownCell => "Cette case n'appartient à aucun tableau.",
            TableError::NotRectangular => {
                "Impossible de fusionner : le résultat ne serait pas un rectangle. Scindez d'abord les cases fusionnées qui dépassent."
            }
            TableError::NotSpanned => "Cette case n'est pas fusionnée.",
            TableError::LastLine => "Un tableau garde au moins une ligne et une colonne.",
            TableError::Locked => "Ce tableau est verrouillé : déverrouillez-le pour le modifier.",
            TableError::TooMany => "Un tableau a au plus 50 lignes et 50 colonnes.",
        }
    }
}

/// Contenu d'une case : texte, image OpenMoji (par son code) ou photo.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, TS)]
#[ts(export)]
pub enum CellContent {
    Text,
    Picto(String),
    Photo,
}

/// Trait par défaut d'un nouveau tableau.
pub fn default_border() -> Stroke {
    Stroke { width_mm: 0.5, color: "#333333".into(), style: None }
}

fn text_cell_kind() -> ElementKind {
    ElementKind::Text(TextProps {
        content: RichText::default(),
        style: TextStyle { align: Align::Center, valign: Some(VAlign::Middle), ..TextStyle::default_zone() },
        prompt: None,
        frame: None,
    })
}

fn cell_kind(content: CellContent) -> ElementKind {
    match content {
        CellContent::Text => text_cell_kind(),
        CellContent::Picto(code) => ElementKind::Image(ImageProps { code }),
        CellContent::Photo => ElementKind::Photo(PhotoProps { asset: None, crop: Crop::COVER, corner_radius_mm: 0.0, border: None, shadow: false, mirrored: false }),
    }
}

fn new_cell(group: &Id) -> Element {
    Element {
        id: new_id(),
        frame: Frame { x_mm: 0.0, y_mm: 0.0, w_mm: 0.0, h_mm: 0.0, rotation_deg: 0.0 },
        locked: false,
        zone: false,
        group: Some(group.clone()),
        kind: text_cell_kind(),
    }
}

/// Pose un tableau de cases texte vides dans `frame` ; rend l'id de l'élément `Table`, placé au-dessus de
/// ses cases pour que leur fond ne cache pas les traits.
pub fn add_table(page: &mut Page, frame: Frame, cols: u8, rows: u8) -> Id {
    let (cols, rows) = (cols.clamp(1, MAX_LINES), rows.clamp(1, MAX_LINES));
    let group = new_id();
    let mut cells = Vec::new();
    let mut elements = Vec::new();
    for row in 0..rows {
        for col in 0..cols {
            let e = new_cell(&group);
            cells.push(GridCell { element_id: e.id.clone(), col, row, col_span: 1, row_span: 1 });
            elements.push(e);
        }
    }
    let table = Element {
        id: new_id(),
        frame,
        locked: false,
        zone: false,
        group: Some(group),
        kind: ElementKind::Table(TableProps {
            cols: vec![1.0; cols as usize],
            rows: vec![1.0; rows as usize],
            cells,
            border: Some(default_border()),
        }),
    };
    let id = table.id.clone();
    page.elements.extend(elements);
    page.elements.push(table);
    layout(page, &id);
    id
}

/// Bords des colonnes (ou des lignes) en mm, de `start` à `start + len`.
fn edges(sizes: &[f64], start: f64, len: f64) -> Vec<f64> {
    let total: f64 = sizes.iter().sum();
    let mut out = vec![start];
    let mut acc = 0.0;
    for s in sizes {
        acc += s;
        out.push(start + len * acc / total);
    }
    out
}

/// Recalcule les cadres des cases ; une case picto ou photo garde une marge pour laisser voir les traits,
/// et un picto reste carré, centré dans sa case.
pub fn layout(page: &mut Page, table: &str) {
    let Some(t) = page.elements.iter().find(|e| e.id == table) else { return };
    let ElementKind::Table(props) = &t.kind else { return };
    let f = t.frame;
    let xs = edges(&props.cols, f.x_mm, f.w_mm);
    let ys = edges(&props.rows, f.y_mm, f.h_mm);
    let cells = props.cells.clone();
    for c in &cells {
        // Une case hors de la grille (fichier abîmé) reste où elle est.
        let (c0, c1) = (c.col as usize, c.col as usize + c.col_span as usize);
        let (r0, r1) = (c.row as usize, c.row as usize + c.row_span as usize);
        if c0 >= c1 || c1 >= xs.len() || r0 >= r1 || r1 >= ys.len() {
            continue;
        }
        let Some(e) = page.elements.iter_mut().find(|e| e.id == c.element_id) else { continue };
        let (x0, x1) = (xs[c0], xs[c1]);
        let (y0, y1) = (ys[r0], ys[r1]);
        let inset = if matches!(e.kind, ElementKind::Text(_)) { 0.0 } else { TEXT_PADDING_MM.min((x1 - x0) / 4.0).min((y1 - y0) / 4.0) };
        let (mut w, mut h) = (x1 - x0 - 2.0 * inset, y1 - y0 - 2.0 * inset);
        if matches!(e.kind, ElementKind::Image(_)) {
            (w, h) = (w.min(h), w.min(h));
        }
        e.frame = Frame { x_mm: (x0 + x1 - w) / 2.0, y_mm: (y0 + y1 - h) / 2.0, w_mm: w, h_mm: h, rotation_deg: 0.0 };
    }
}

/// Index de l'élément `Table` qui contient la case ; un tableau verrouillé ne se modifie pas.
fn table_of(page: &Page, cell: &str) -> Result<usize, TableError> {
    let i = page
        .elements
        .iter()
        .position(|e| matches!(&e.kind, ElementKind::Table(t) if t.cells.iter().any(|c| c.element_id == cell)))
        .ok_or(TableError::UnknownCell)?;
    if page.elements[i].locked {
        return Err(TableError::Locked);
    }
    Ok(i)
}

fn props_mut(page: &mut Page, i: usize) -> &mut TableProps {
    match &mut page.elements[i].kind {
        ElementKind::Table(t) => t,
        _ => unreachable!("table_of ne rend que des tableaux"),
    }
}

/// Lignes (`rows`) ou colonnes, vues de la même façon pour insérer et retirer.
#[derive(Clone, Copy)]
enum Axis {
    Row,
    Col,
}

impl Axis {
    fn pos(self, c: &GridCell) -> u8 {
        match self {
            Axis::Row => c.row,
            Axis::Col => c.col,
        }
    }
    fn other(self, c: &GridCell) -> u8 {
        match self {
            Axis::Row => c.col,
            Axis::Col => c.row,
        }
    }
    fn span(self, c: &GridCell) -> u8 {
        match self {
            Axis::Row => c.row_span,
            Axis::Col => c.col_span,
        }
    }
    fn set(self, c: &mut GridCell, pos: u8, span: u8) {
        match self {
            Axis::Row => (c.row, c.row_span) = (pos, span),
            Axis::Col => (c.col, c.col_span) = (pos, span),
        }
    }
    fn sizes(self, t: &mut TableProps) -> &mut Vec<f64> {
        match self {
            Axis::Row => &mut t.rows,
            Axis::Col => &mut t.cols,
        }
    }
    fn cell(self, pos: u8, other: u8) -> GridCell {
        match self {
            Axis::Row => GridCell { element_id: String::new(), col: other, row: pos, col_span: 1, row_span: 1 },
            Axis::Col => GridCell { element_id: String::new(), col: pos, row: other, col_span: 1, row_span: 1 },
        }
    }
    /// Agrandit (ou réduit) le cadre du tableau de `mm` dans le sens de l'axe, vers le bas ou la droite.
    fn grow(self, f: &mut Frame, mm: f64) {
        match self {
            Axis::Row => f.h_mm += mm,
            Axis::Col => f.w_mm += mm,
        }
    }
    fn len(self, f: &Frame) -> f64 {
        match self {
            Axis::Row => f.h_mm,
            Axis::Col => f.w_mm,
        }
    }
    /// Place jusqu'au bord de la page, depuis le début du tableau.
    fn room(self, f: &Frame, page: PageSize) -> f64 {
        match self {
            Axis::Row => page.height_mm - f.y_mm,
            Axis::Col => page.width_mm - f.x_mm,
        }
    }
}

/// Insère une ligne ou une colonne à côté de la case, de la taille de celle qu'elle touche ; les cases
/// fusionnées qu'elle traverse s'allongent. Le tableau grandit jusqu'au bord de la page, pas au-delà :
/// ensuite ses lignes (ou colonnes) se resserrent.
fn insert(page: &mut Page, cell: &str, axis: Axis, after: bool) -> Result<(), TableError> {
    let i = table_of(page, cell)?;
    let group = page.elements[i].group.clone().unwrap_or_else(new_id);
    let frame = page.elements[i].frame;
    let page_size = page.size;
    let t = props_mut(page, i);
    if axis.sizes(t).len() >= MAX_LINES as usize {
        return Err(TableError::TooMany);
    }
    let c = t.cells.iter().find(|c| c.element_id == cell).unwrap().clone();
    let reference = if after { axis.pos(&c) + axis.span(&c) - 1 } else { axis.pos(&c) };
    let at = if after { reference + 1 } else { reference };
    let sizes = axis.sizes(t);
    let size = sizes[reference as usize];
    let added_mm = axis.len(&frame) * size / sizes.iter().sum::<f64>();
    sizes.insert(at as usize, size);
    let mut covered = Vec::new();
    for c in &mut t.cells {
        let (pos, span) = (axis.pos(c), axis.span(c));
        if pos >= at {
            axis.set(c, pos + 1, span);
        } else if pos + span > at {
            axis.set(c, pos, span + 1);
            covered.extend(axis.other(c)..axis.other(c) + match axis {
                Axis::Row => c.col_span,
                Axis::Col => c.row_span,
            });
        }
    }
    let across = match axis {
        Axis::Row => t.cols.len(),
        Axis::Col => t.rows.len(),
    } as u8;
    let mut added = Vec::new();
    for other in (0..across).filter(|o| !covered.contains(o)) {
        let e = new_cell(&group);
        t.cells.push(GridCell { element_id: e.id.clone(), ..axis.cell(at, other) });
        added.push(e);
    }
    let len = axis.len(&frame);
    axis.grow(&mut page.elements[i].frame, (len + added_mm).min(axis.room(&frame, page_size).max(len)) - len);
    let table = page.elements[i].id.clone();
    page.elements.splice(i..i, added);
    layout(page, &table);
    Ok(())
}

/// Retire la ligne ou la colonne où commence la case ; une case fusionnée qui la traverse rétrécit.
fn remove(page: &mut Page, cell: &str, axis: Axis) -> Result<(), TableError> {
    let i = table_of(page, cell)?;
    let frame = page.elements[i].frame;
    let t = props_mut(page, i);
    if axis.sizes(t).len() == 1 {
        return Err(TableError::LastLine);
    }
    let at = axis.pos(t.cells.iter().find(|c| c.element_id == cell).unwrap());
    let sizes = axis.sizes(t);
    let removed_mm = axis.len(&frame) * sizes[at as usize] / sizes.iter().sum::<f64>();
    sizes.remove(at as usize);
    let mut gone = Vec::new();
    t.cells.retain_mut(|c| {
        let (pos, span) = (axis.pos(c), axis.span(c));
        if pos > at {
            axis.set(c, pos - 1, span);
        } else if pos + span > at {
            if span == 1 {
                gone.push(c.element_id.clone());
                return false;
            }
            axis.set(c, pos, span - 1);
        }
        true
    });
    axis.grow(&mut page.elements[i].frame, -removed_mm);
    let table = page.elements[i].id.clone();
    page.elements.retain(|e| !gone.contains(&e.id));
    layout(page, &table);
    Ok(())
}

pub fn insert_row(page: &mut Page, cell: &str, below: bool) -> Result<(), TableError> {
    insert(page, cell, Axis::Row, below)
}

pub fn insert_col(page: &mut Page, cell: &str, right: bool) -> Result<(), TableError> {
    insert(page, cell, Axis::Col, right)
}

pub fn remove_row(page: &mut Page, cell: &str) -> Result<(), TableError> {
    remove(page, cell, Axis::Row)
}

pub fn remove_col(page: &mut Page, cell: &str) -> Result<(), TableError> {
    remove(page, cell, Axis::Col)
}

/// Fusionne `a` et `b` en leur rectangle englobant ; la case `a` garde son contenu.
pub fn merge(page: &mut Page, a: &str, b: &str) -> Result<(), TableError> {
    let i = table_of(page, a)?;
    let t = props_mut(page, i);
    let cb = t.cells.iter().find(|c| c.element_id == b).ok_or(TableError::UnknownCell)?;
    let ca = t.cells.iter().find(|c| c.element_id == a).unwrap();
    let c0 = ca.col.min(cb.col);
    let r0 = ca.row.min(cb.row);
    let c1 = (ca.col + ca.col_span).max(cb.col + cb.col_span);
    let r1 = (ca.row + ca.row_span).max(cb.row + cb.row_span);
    let mut absorbed = Vec::new();
    for c in &t.cells {
        let inside = c.col >= c0 && c.col + c.col_span <= c1 && c.row >= r0 && c.row + c.row_span <= r1;
        let outside = c.col + c.col_span <= c0 || c.col >= c1 || c.row + c.row_span <= r0 || c.row >= r1;
        if inside {
            if c.element_id != a {
                absorbed.push(c.element_id.clone());
            }
        } else if !outside {
            return Err(TableError::NotRectangular);
        }
    }
    t.cells.retain(|c| !absorbed.contains(&c.element_id));
    let ca = t.cells.iter_mut().find(|c| c.element_id == a).unwrap();
    *ca = GridCell { element_id: a.to_string(), col: c0, row: r0, col_span: c1 - c0, row_span: r1 - r0 };
    let table = page.elements[i].id.clone();
    page.elements.retain(|e| !absorbed.contains(&e.id));
    layout(page, &table);
    Ok(())
}

/// Scinde une case fusionnée ; son contenu reste dans la case du coin haut gauche, les autres sont des
/// cases texte vides.
pub fn split(page: &mut Page, cell: &str) -> Result<(), TableError> {
    let i = table_of(page, cell)?;
    let group = page.elements[i].group.clone().unwrap_or_else(new_id);
    let t = props_mut(page, i);
    let ci = t.cells.iter().position(|c| c.element_id == cell).unwrap();
    let c = t.cells[ci].clone();
    if c.col_span == 1 && c.row_span == 1 {
        return Err(TableError::NotSpanned);
    }
    t.cells[ci].col_span = 1;
    t.cells[ci].row_span = 1;
    let mut added = Vec::new();
    for row in c.row..c.row + c.row_span {
        for col in c.col..c.col + c.col_span {
            if (col, row) != (c.col, c.row) {
                let e = new_cell(&group);
                t.cells.push(GridCell { element_id: e.id.clone(), col, row, col_span: 1, row_span: 1 });
                added.push(e);
            }
        }
    }
    let table = page.elements[i].id.clone();
    page.elements.splice(i..i, added);
    layout(page, &table);
    Ok(())
}

/// Change le contenu d'une case (vide) ; la case garde son id, son groupe et sa place. Une case qui a déjà
/// ce type de contenu reste telle quelle.
pub fn set_cell_content(page: &mut Page, cell: &str, content: CellContent) -> Result<(), TableError> {
    let i = table_of(page, cell)?;
    let table = page.elements[i].id.clone();
    let e = page.elements.iter_mut().find(|e| e.id == cell).ok_or(TableError::UnknownCell)?;
    let same = matches!(
        (&e.kind, &content),
        (ElementKind::Text(_), CellContent::Text) | (ElementKind::Image(_), CellContent::Picto(_)) | (ElementKind::Photo(_), CellContent::Photo)
    );
    if same {
        return Ok(());
    }
    e.kind = cell_kind(content);
    layout(page, &table);
    Ok(())
}

/// Traits du tableau, en segments `[x1, y1, x2, y2]` (mm) : contour et séparations, sans traverser une
/// case fusionnée ; les segments contigus d'une même séparation sont réunis.
pub fn border_segments(t: &TableProps, f: &Frame) -> Vec<[f64; 4]> {
    let xs = edges(&t.cols, f.x_mm, f.w_mm);
    let ys = edges(&t.rows, f.y_mm, f.h_mm);
    let (ncols, nrows) = (t.cols.len(), t.rows.len());
    // Vrai si une case traverse la séparation `k` (entre `k - 1` et `k`) à la position `at` de l'autre axe.
    let crosses = |axis: Axis, k: usize, at: usize| {
        t.cells.iter().any(|c| {
            let (pos, span) = (axis.pos(c) as usize, axis.span(c) as usize);
            let (o, ospan) = match axis {
                Axis::Col => (c.row as usize, c.row_span as usize),
                Axis::Row => (c.col as usize, c.col_span as usize),
            };
            pos < k && k < pos + span && o <= at && at < o + ospan
        })
    };
    let mut out = Vec::new();
    for (axis, n, along) in [(Axis::Col, ncols, nrows), (Axis::Row, nrows, ncols)] {
        for k in 0..=n {
            let mut start: Option<usize> = None;
            for at in 0..=along {
                let drawn = at < along && (k == 0 || k == n || !crosses(axis, k, at));
                match (drawn, start) {
                    (true, None) => start = Some(at),
                    (false, Some(s)) => {
                        out.push(match axis {
                            Axis::Col => [xs[k], ys[s], xs[k], ys[at]],
                            Axis::Row => [xs[s], ys[k], xs[at], ys[k]],
                        });
                        start = None;
                    }
                    _ => {}
                }
            }
        }
    }
    out
}

/// Épaisseur du cadre d'un trait issu d'un tableau dissocié (zone de clic).
const LINE_FRAME_MM: f64 = 2.0;

/// Dissocie le tableau : les cases restent à leur place et les traits deviennent des formes « trait »,
/// groupées avec elles à la place de l'élément `Table`.
pub fn dissolve(page: &mut Page, table: &str) -> Result<(), TableError> {
    let i = page.elements.iter().position(|e| e.id == table && matches!(e.kind, ElementKind::Table(_))).ok_or(TableError::UnknownTable)?;
    if page.elements[i].locked {
        return Err(TableError::Locked);
    }
    let t = page.elements.remove(i);
    let ElementKind::Table(props) = &t.kind else { unreachable!() };
    let lines: Vec<Element> = match &props.border {
        None => Vec::new(),
        Some(stroke) => border_segments(props, &t.frame)
            .into_iter()
            .map(|[x1, y1, x2, y2]| {
                let len = (x2 - x1).hypot(y2 - y1);
                let (cx, cy) = ((x1 + x2) / 2.0, (y1 + y2) / 2.0);
                Element {
                    id: new_id(),
                    frame: Frame {
                        x_mm: cx - len / 2.0,
                        y_mm: cy - LINE_FRAME_MM / 2.0,
                        w_mm: len,
                        h_mm: LINE_FRAME_MM,
                        rotation_deg: if x1 == x2 { 90.0 } else { 0.0 },
                    },
                    locked: false,
                    zone: false,
                    group: t.group.clone(),
                    kind: ElementKind::Shape(ShapeProps { shape: ShapeKind::Line, fill: None, stroke: Some(stroke.clone()), corner_radius_mm: 0.0 }),
                }
            })
            .collect(),
    };
    page.elements.splice(i..i, lines);
    crate::groups::dissolve_small_groups(page);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const TABLE_FRAME: Frame = Frame { x_mm: 30.0, y_mm: 103.5, w_mm: 150.0, h_mm: 90.0, rotation_deg: 0.0 };

    fn props(page: &Page, table: &str) -> TableProps {
        match &page.elements.iter().find(|e| e.id == table).unwrap().kind {
            ElementKind::Table(t) => t.clone(),
            _ => panic!("pas un tableau"),
        }
    }

    fn cell_at(page: &Page, table: &str, col: u8, row: u8) -> String {
        props(page, table).cells.iter().find(|c| c.col == col && c.row == row).unwrap().element_id.clone()
    }

    fn el<'a>(page: &'a Page, id: &str) -> &'a Element {
        page.elements.iter().find(|e| e.id == id).unwrap()
    }

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-6
    }

    fn table(cols: u8, rows: u8) -> (Page, String) {
        let mut p = Page::new(A4_PORTRAIT);
        let t = add_table(&mut p, TABLE_FRAME, cols, rows);
        (p, t)
    }

    #[test]
    fn add_table_3x3() {
        let (p, t) = table(3, 3);
        assert_eq!(p.elements.len(), 10);
        assert_eq!(p.elements[9].id, t);
        let group = p.elements[0].group.clone().unwrap();
        assert!(p.elements.iter().all(|e| e.group.as_ref() == Some(&group)));
        let f = el(&p, &cell_at(&p, &t, 1, 1)).frame;
        assert!(close(f.x_mm, 80.0) && close(f.y_mm, 133.5) && close(f.w_mm, 50.0) && close(f.h_mm, 30.0), "{f:?}");
        let ElementKind::Text(tx) = &el(&p, &cell_at(&p, &t, 0, 0)).kind else { panic!() };
        assert_eq!(tx.style.valign, Some(VAlign::Middle));
        assert_eq!(tx.style.align, Align::Center);
        assert_eq!(props(&p, &t).border, Some(default_border()));
    }

    #[test]
    fn layout_uses_relative_sizes() {
        let mut p = Page::new(A4_PORTRAIT);
        let t = add_table(&mut p, Frame { x_mm: 0.0, y_mm: 0.0, w_mm: 90.0, h_mm: 10.0, rotation_deg: 0.0 }, 2, 1);
        if let ElementKind::Table(tp) = &mut p.elements[2].kind {
            tp.cols = vec![1.0, 2.0];
        }
        layout(&mut p, &t);
        assert!(close(el(&p, &cell_at(&p, &t, 0, 0)).frame.w_mm, 30.0));
        assert!(close(el(&p, &cell_at(&p, &t, 1, 0)).frame.w_mm, 60.0));
    }

    #[test]
    fn insert_row_below_shifts_rows() {
        let (mut p, t) = table(2, 2);
        let below = cell_at(&p, &t, 0, 1);
        let c0 = cell_at(&p, &t, 0, 0);
        insert_row(&mut p, &c0, true).unwrap();
        let tp = props(&p, &t);
        assert_eq!(tp.rows.len(), 3);
        assert_eq!(tp.cells.len(), 6);
        assert_eq!(tp.cells.iter().find(|c| c.element_id == below).unwrap().row, 2);
        assert!(close(el(&p, &t).frame.h_mm, 135.0));
        assert!(close(el(&p, &below).frame.y_mm, 103.5 + 90.0));
        assert_eq!(p.elements.len(), 7);
        assert_eq!(p.elements[6].id, t, "le tableau reste au-dessus de ses cases");
    }

    #[test]
    fn insert_row_through_merged_cell_extends_it() {
        let (mut p, t) = table(2, 2);
        let tall = cell_at(&p, &t, 0, 0);
        let c0 = cell_at(&p, &t, 0, 1);
        merge(&mut p, &tall, &c0).unwrap();
        let c0 = cell_at(&p, &t, 1, 0);
        insert_row(&mut p, &c0, true).unwrap();
        let tp = props(&p, &t);
        assert_eq!(tp.cells.iter().find(|c| c.element_id == tall).unwrap().row_span, 3);
        assert_eq!(tp.cells.len(), 4);
        assert!(!tp.cells.iter().any(|c| c.col == 0 && c.row == 1));
    }

    #[test]
    fn remove_row_shrinks_merged_cell() {
        let (mut p, t) = table(2, 2);
        let tall = cell_at(&p, &t, 0, 0);
        let c0 = cell_at(&p, &t, 0, 1);
        merge(&mut p, &tall, &c0).unwrap();
        let right_bottom = cell_at(&p, &t, 1, 1);
        remove_row(&mut p, &right_bottom).unwrap();
        let tp = props(&p, &t);
        assert_eq!(tp.rows.len(), 1);
        assert_eq!(tp.cells.iter().find(|c| c.element_id == tall).unwrap().row_span, 1);
        assert!(!p.elements.iter().any(|e| e.id == right_bottom));
        assert!(close(el(&p, &t).frame.h_mm, 45.0));
    }

    #[test]
    fn remove_last_row_is_refused() {
        let (mut p, t) = table(2, 1);
        let c0 = cell_at(&p, &t, 0, 0);
        assert_eq!(remove_row(&mut p, &c0), Err(TableError::LastLine));
        let (mut p, t) = table(1, 2);
        let c0 = cell_at(&p, &t, 0, 0);
        assert_eq!(remove_col(&mut p, &c0), Err(TableError::LastLine));
    }

    #[test]
    fn insert_and_remove_col() {
        let (mut p, t) = table(2, 2);
        let c0 = cell_at(&p, &t, 0, 0);
        insert_col(&mut p, &c0, false).unwrap();
        assert_eq!(props(&p, &t).cols.len(), 3);
        // 150 + 75 mm dépasserait la page (210 mm, tableau en x = 30) : il s'arrête au bord.
        assert!(close(el(&p, &t).frame.w_mm, 180.0));
        let c0 = cell_at(&p, &t, 0, 1);
        remove_col(&mut p, &c0).unwrap();
        assert_eq!(props(&p, &t).cells.len(), 4);
        assert!(close(el(&p, &t).frame.w_mm, 120.0));
    }

    #[test]
    fn merge_then_split_roundtrip() {
        let (mut p, t) = table(3, 3);
        let a = cell_at(&p, &t, 0, 0);
        let c0 = cell_at(&p, &t, 1, 1);
        merge(&mut p, &a, &c0).unwrap();
        assert_eq!(props(&p, &t).cells.len(), 6);
        assert_eq!(p.elements.len(), 7);
        let f = el(&p, &a).frame;
        assert!(close(f.w_mm, 100.0) && close(f.h_mm, 60.0));
        split(&mut p, &a).unwrap();
        assert_eq!(props(&p, &t).cells.len(), 9);
        assert_eq!(split(&mut p, &a), Err(TableError::NotSpanned));
        let c0 = cell_at(&p, &t, 1, 0);
        merge(&mut p, &a, &c0).unwrap();
        let c0 = cell_at(&p, &t, 1, 1);
        let c1 = cell_at(&p, &t, 2, 0);
        assert_eq!(merge(&mut p, &c0, &c1), Err(TableError::NotRectangular));
    }

    #[test]
    fn cell_content_picto() {
        let (mut p, t) = table(2, 2);
        let c = cell_at(&p, &t, 0, 0);
        let group = el(&p, &c).group.clone();
        set_cell_content(&mut p, &c, CellContent::Picto("1F98A".into())).unwrap();
        let e = el(&p, &c);
        assert_eq!(e.kind, ElementKind::Image(ImageProps { code: "1F98A".into() }));
        assert_eq!(e.group, group);
        // Case de 75 × 45 mm : un carré de 42 mm (marge de 1,5 mm), centré.
        let f = e.frame;
        assert!(close(f.w_mm, 42.0) && close(f.h_mm, 42.0), "{f:?}");
        assert!(close(f.x_mm, 30.0 + (75.0 - 42.0) / 2.0) && close(f.y_mm, 103.5 + TEXT_PADDING_MM), "{f:?}");
        set_cell_content(&mut p, &c, CellContent::Photo).unwrap();
        assert!(matches!(&el(&p, &c).kind, ElementKind::Photo(ph) if ph.asset.is_none()));
        assert_eq!(set_cell_content(&mut p, "inconnue", CellContent::Text), Err(TableError::UnknownCell));
    }

    #[test]
    fn same_content_keeps_the_cell() {
        let (mut p, t) = table(2, 1);
        let c = cell_at(&p, &t, 0, 0);
        if let Some(ElementKind::Text(tx)) = p.elements.iter_mut().find(|e| e.id == c).map(|e| &mut e.kind) {
            tx.content = RichText::plain("Lundi");
        }
        set_cell_content(&mut p, &c, CellContent::Text).unwrap();
        assert!(matches!(&el(&p, &c).kind, ElementKind::Text(tx) if !tx.content.is_blank()));
        set_cell_content(&mut p, &c, CellContent::Picto("1F98A".into())).unwrap();
        set_cell_content(&mut p, &c, CellContent::Picto("2B50".into())).unwrap();
        assert_eq!(el(&p, &c).kind, ElementKind::Image(ImageProps { code: "1F98A".into() }));
    }

    #[test]
    fn a_locked_table_refuses_changes() {
        let (mut p, t) = table(2, 2);
        p.elements.iter_mut().find(|e| e.id == t).unwrap().locked = true;
        let c0 = cell_at(&p, &t, 0, 0);
        let c1 = cell_at(&p, &t, 1, 0);
        assert_eq!(insert_row(&mut p, &c0, true), Err(TableError::Locked));
        assert_eq!(remove_col(&mut p, &c0), Err(TableError::Locked));
        assert_eq!(merge(&mut p, &c0, &c1), Err(TableError::Locked));
        assert_eq!(set_cell_content(&mut p, &c0, CellContent::Photo), Err(TableError::Locked));
        assert_eq!(dissolve(&mut p, &t), Err(TableError::Locked));
        assert_eq!(props(&p, &t).cells.len(), 4);
    }

    #[test]
    fn insert_stays_on_the_page() {
        // 150 × 90 mm en (30, 103,5) : 3 lignes de 30 mm ; la page s'arrête à 297 mm.
        let (mut p, t) = table(3, 3);
        for _ in 0..6 {
            let c = cell_at(&p, &t, 0, 0);
            insert_row(&mut p, &c, true).unwrap();
        }
        let f = el(&p, &t).frame;
        assert!(f.y_mm + f.h_mm <= 297.0 + 1e-6, "{f:?}");
        assert_eq!(props(&p, &t).rows.len(), 9);
        // Les lignes restent égales : le tableau occupe la place jusqu'au bas de la page.
        assert!(close(f.h_mm, 297.0 - 103.5), "{f:?}");
        let c = cell_at(&p, &t, 0, 0);
        assert!(close(el(&p, &c).frame.h_mm, (297.0 - 103.5) / 9.0));
        for _ in 0..3 {
            let c = cell_at(&p, &t, 0, 0);
            insert_col(&mut p, &c, true).unwrap();
        }
        let f = el(&p, &t).frame;
        assert!(f.x_mm + f.w_mm <= 210.0 + 1e-6, "{f:?}");
    }

    #[test]
    fn lines_are_capped() {
        let mut p = Page::new(A4_PORTRAIT);
        let t = add_table(&mut p, TABLE_FRAME, 200, 1);
        assert_eq!(props(&p, &t).cols.len(), MAX_LINES as usize);
        let c = cell_at(&p, &t, 0, 0);
        assert_eq!(insert_col(&mut p, &c, true), Err(TableError::TooMany));
    }

    #[test]
    fn layout_skips_cells_outside_the_grid() {
        // Fichier abîmé : une case hors de la grille ne fait pas planter la mise en page.
        let (mut p, t) = table(2, 1);
        if let Some(ElementKind::Table(tp)) = p.elements.iter_mut().find(|e| e.id == t).map(|e| &mut e.kind) {
            tp.cells[1].col = 250;
            tp.cells[1].col_span = 10;
        }
        layout(&mut p, &t);
        let tp = props(&p, &t);
        let _ = border_segments(&tp, &el(&p, &t).frame);
    }

    #[test]
    fn border_segments_skip_merged_cells() {
        let f = Frame { x_mm: 0.0, y_mm: 0.0, w_mm: 20.0, h_mm: 10.0, rotation_deg: 0.0 };
        let (mut p, t) = table(2, 2);
        let mut segs = border_segments(&props(&p, &t), &f);
        segs.sort_by(|a, b| a.partial_cmp(b).unwrap());
        assert_eq!(
            segs,
            vec![[0.0, 0.0, 0.0, 10.0], [0.0, 0.0, 20.0, 0.0], [0.0, 5.0, 20.0, 5.0], [0.0, 10.0, 20.0, 10.0], [10.0, 0.0, 10.0, 10.0], [20.0, 0.0, 20.0, 10.0]]
        );
        let c0 = cell_at(&p, &t, 0, 0);
        let c1 = cell_at(&p, &t, 1, 0);
        merge(&mut p, &c0, &c1).unwrap();
        let segs = border_segments(&props(&p, &t), &f);
        assert!(segs.contains(&[10.0, 5.0, 10.0, 10.0]));
        assert!(!segs.iter().any(|s| s[0] == 10.0 && s[2] == 10.0 && s[1] == 0.0));
    }

    #[test]
    fn dissolve_turns_borders_into_lines() {
        let (mut p, t) = table(2, 2);
        dissolve(&mut p, &t).unwrap();
        assert!(!p.elements.iter().any(|e| matches!(e.kind, ElementKind::Table(_))));
        let lines: Vec<&Element> = p.elements.iter().filter(|e| matches!(&e.kind, ElementKind::Shape(s) if s.shape == ShapeKind::Line)).collect();
        assert_eq!(lines.len(), 6);
        let group = p.elements[0].group.clone();
        assert!(group.is_some() && p.elements.iter().all(|e| e.group == group));
        // La séparation verticale, tournée d'un quart de tour autour de son milieu.
        let v = lines.iter().find(|e| e.frame.rotation_deg == 90.0 && close(e.frame.x_mm + e.frame.w_mm / 2.0, 105.0)).unwrap();
        assert!(close(v.frame.w_mm, 90.0) && close(v.frame.y_mm + v.frame.h_mm / 2.0, 148.5));
        assert_eq!(dissolve(&mut p, &t), Err(TableError::UnknownTable));
    }

    #[test]
    fn dissolve_without_border_keeps_only_cells() {
        let (mut p, t) = table(2, 1);
        if let ElementKind::Table(tp) = &mut p.elements[2].kind {
            tp.border = None;
        }
        dissolve(&mut p, &t).unwrap();
        assert_eq!(p.elements.len(), 2);
    }
}
