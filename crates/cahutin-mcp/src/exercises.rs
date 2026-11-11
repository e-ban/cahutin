//! Fiches d'exercices : lignes de graphisme, labyrinthes, symétrie, chemins en pointillés et repères
//! d'écriture. Mêmes tracés que l'interface (`ui/src/lib/graphisme.ts`, `fiches.ts`, `ShapesDrawer.svelte`) :
//! `fixtures/ui_reference.txt` garde ce que l'interface trace, et les tests le comparent.

use crate::decor::element;
use carnet_core::{Cell, Document, Element, ElementKind, Frame, GraphismeRow, Guides, Id, ImageProps, Page, ShapeKind, ShapeProps, Stroke, StrokeStyle, new_id};
use serde_json::{Value, json};

const MARGIN_MM: f64 = 15.0;
const NO_ROOM: &str = "Plus de place sur la page : ajoutez une page, ou faites de la place en bas.";
const START_COLOR: &str = "#2e9d4a";
const LOOP_STEPS: usize = 24;
const LOOP_K: f64 = 2.2;
const MOTIFS: [&str; 8] = ["ponts", "coupes", "vagues", "boucles", "creneaux", "dents", "traits", "ronds"];
const MODEL_COLOR: &str = "#333333";
const TRACE_COLOR: &str = "#9e9e9e";
/// Préréglages d'une ligne (`PRESETS`), de « tout » en pointillés à « seul » (le modèle seul).
const PRESETS: [&str; 5] = ["tout", "moitie", "quelques", "seul", "progressif"];
/// Repères d'écriture, comme l'assistant les nomme.
const GUIDES: [(&str, Guides); 3] = [("base", Guides::Base), ("couloir", Guides::Corridor), ("aucune", Guides::None)];

/// `Math.round` de JavaScript (demi vers le haut), pour tracer au même point près que l'interface.
fn js_round(v: f64) -> f64 {
    (v + 0.5).floor()
}

fn round2(v: f64) -> f64 {
    js_round(v * 100.0) / 100.0
}

fn stroke(width_mm: f64, color: &str, style: StrokeStyle) -> Option<Stroke> {
    Some(Stroke { width_mm, color: color.into(), style: Some(style) })
}

fn shape(shape: ShapeKind, frame: Frame, fill: Option<&str>, stroke: Option<Stroke>, path: Option<String>, group: Option<&Id>) -> Element {
    element(new_id(), frame, group.cloned(), ElementKind::Shape(ShapeProps { shape, fill: fill.map(str::to_string), stroke, corner_radius_mm: 0.0, path, graphisme: None }))
}

fn frame(x_mm: f64, y_mm: f64, w_mm: f64, h_mm: f64) -> Frame {
    Frame { x_mm, y_mm, w_mm, h_mm, rotation_deg: 0.0 }
}

fn height(level: &str) -> f64 {
    match level {
        "PS" => 24.0,
        "GS" => 10.0,
        _ => 16.0,
    }
}

fn step(m: &str) -> f64 {
    match m {
        "ponts" | "coupes" => 1.1,
        "vagues" => 1.6,
        "boucles" => 0.9,
        "creneaux" => 1.2,
        "dents" => 1.0,
        "traits" => 0.6,
        _ => 1.15,
    }
}

#[derive(Clone, Copy)]
enum Cmd {
    M(f64, f64),
    L(f64, f64),
    C(f64, f64, f64, f64, f64, f64),
}

impl Cmd {
    fn points(&self) -> Vec<(f64, f64)> {
        match *self {
            Cmd::M(x, y) | Cmd::L(x, y) => vec![(x, y)],
            Cmd::C(a, b, c, d, e, f) => vec![(a, b), (c, d), (e, f)],
        }
    }
}

/// Le motif numéro `i`, en mm (`motifCmds`).
fn motif_cmds(m: &str, x: f64, w: f64, h: f64, first: bool) -> Vec<Cmd> {
    let k = (4.0 / 3.0) * h;
    let mut out = Vec::new();
    match m {
        "ponts" => out.extend([Cmd::M(x, h), Cmd::C(x, h - k, x + w * 0.85, h - k, x + w * 0.85, h)]),
        "coupes" => out.extend([Cmd::M(x, 0.0), Cmd::C(x, k, x + w * 0.85, k, x + w * 0.85, 0.0)]),
        "vagues" => {
            let mid = h / 2.0;
            let a = (4.0 / 3.0) * (h / 2.0);
            if first {
                out.push(Cmd::M(x, mid));
            }
            out.extend([Cmd::C(x, mid - a, x + w / 2.0, mid - a, x + w / 2.0, mid), Cmd::C(x + w / 2.0, mid + a, x + w, mid + a, x + w, mid)]);
        }
        "boucles" => {
            for i in (if first { 0 } else { 1 })..=LOOP_STEPS {
                let t = std::f64::consts::PI + (i as f64 / LOOP_STEPS as f64) * 2.0 * std::f64::consts::PI;
                let u = (t - std::f64::consts::PI - LOOP_K * t.sin()) / (2.0 * std::f64::consts::PI);
                let v = (1.0 - t.cos()) / 2.0;
                let (px, py) = (round2(x + u * w), round2(h * v));
                out.push(if i == 0 { Cmd::M(px, py) } else { Cmd::L(px, py) });
            }
        }
        "creneaux" => {
            if first {
                out.push(Cmd::M(x, h));
            }
            out.extend([Cmd::L(x, 0.0), Cmd::L(x + w / 2.0, 0.0), Cmd::L(x + w / 2.0, h), Cmd::L(x + w, h)]);
        }
        "dents" => {
            if first {
                out.push(Cmd::M(x, h));
            }
            out.extend([Cmd::L(x + w / 2.0, 0.0), Cmd::L(x + w, h)]);
        }
        "traits" => out.extend([Cmd::M(x + w / 2.0, 0.0), Cmd::L(x + w / 2.0, h)]),
        _ => {
            let r = h * 0.45;
            let (cx, cy, c) = (x + w / 2.0, h / 2.0, r * 0.5523);
            // Dans le sens de l'écriture : haut, gauche, bas, droite, haut.
            out.extend([
                Cmd::M(cx, cy - r),
                Cmd::C(cx - c, cy - r, cx - r, cy - c, cx - r, cy),
                Cmd::C(cx - r, cy + c, cx - c, cy + r, cx, cy + r),
                Cmd::C(cx + c, cy + r, cx + r, cy + c, cx + r, cy),
                Cmd::C(cx + r, cy - c, cx + c, cy - r, cx, cy - r),
            ]);
        }
    }
    out
}

fn cmds_for(m: &str, x0: f64, h: f64, from: usize, to: usize) -> Vec<Cmd> {
    let w = step(m) * h;
    (from..to).flat_map(|i| motif_cmds(m, x0 + i as f64 * w, w, h, i == from)).collect()
}

/// Un tracé en mm posé en `top` : son cadre et son tracé dans le carré 0..1000 (`pathInFrame`).
fn path_in_frame(cmds: &[Cmd], top: f64) -> (Frame, String) {
    let pts: Vec<(f64, f64)> = cmds.iter().flat_map(Cmd::points).collect();
    let (x0, x1) = pts.iter().fold((f64::MAX, f64::MIN), |(a, b), p| (a.min(p.0), b.max(p.0)));
    let (y0, y1) = pts.iter().fold((f64::MAX, f64::MIN), |(a, b), p| (a.min(p.1), b.max(p.1)));
    let (w, h) = ((x1 - x0).max(0.5), (y1 - y0).max(0.5));
    let px = |x: f64| js_round((x - x0) / w * 1000.0) as i64;
    let py = |y: f64| js_round((y - y0) / h * 1000.0) as i64;
    let path = cmds
        .iter()
        .map(|c| match *c {
            Cmd::M(x, y) => format!("M {} {}", px(x), py(y)),
            Cmd::L(x, y) => format!("L {} {}", px(x), py(y)),
            Cmd::C(a, b, c, d, e, f) => format!("C {} {} {} {} {} {}", px(a), py(b), px(c), py(d), px(e), py(f)),
        })
        .collect::<Vec<_>>()
        .join(" ");
    (frame(round2(x0), round2(top + y0), round2(w), round2(h)), path)
}

/// Sens du premier geste de chaque motif, pour la flèche de départ, en degrés (`ARROW_DEG`).
fn arrow_deg(m: &str) -> f64 {
    match m {
        "ponts" | "boucles" => -70.0,
        "coupes" => 70.0,
        "vagues" | "dents" => -45.0,
        "creneaux" => -90.0,
        "traits" => 90.0,
        _ => 180.0,
    }
}

/// Taille du point de départ (`dotSize`).
fn dot_size(h: f64) -> f64 {
    if h < 14.0 { 2.5 } else { 3.5 }
}

/// Nombre de motifs qui tiennent sur `width` mm après le point de départ, au moins 2 (`motifCount`).
fn motif_count(m: &str, level: &str, width: f64) -> usize {
    let h = height(level);
    (((width - dot_size(h) - 2.0) / (step(m) * h)).floor() as usize).max(2)
}

/// Les cases d'une ligne de `n` motifs : le modèle, des pointillés, puis des cases vides (`cellsFor`).
fn cells_for(preset: &str, n: usize) -> Vec<Cell> {
    let dotted = match preset {
        "moitie" => (n - 1) / 2,
        "quelques" => if n < 8 { 2 } else { 3 },
        "seul" => 0,
        _ => n - 1,
    }
    .min(n - 1);
    (0..n).map(|i| if i == 0 { Cell::Model } else if i <= dotted { Cell::Dotted } else { Cell::Empty }).collect()
}

/// Préréglage de la ligne `row` sur `rows` : de « tout » à « seul » (`progressive`).
fn progressive(row: usize, rows: usize) -> &'static str {
    if rows <= 1 { "tout" } else { PRESETS[js_round(row as f64 * 3.0 / (rows - 1) as f64) as usize] }
}

/// Une ligne de graphisme faite selon sa recette (`buildRow`) : ligne de base (qui porte la recette), ligne du
/// haut si couloir, point de départ, flèche, puis un tracé par suite de cases de même sorte ; un seul groupe.
fn build_row(r: &GraphismeRow, x: f64, y: f64, width: f64) -> Vec<Element> {
    let m = r.motif.as_str();
    let h = height(&r.level);
    let dot = dot_size(h);
    let start = x + dot + 2.0;
    let group = new_id();
    let line = if r.guides == Guides::None { None } else { stroke(0.3, "#d0d0d0", StrokeStyle::Solid) };
    let line_at = |top: f64| shape(ShapeKind::Line, frame(x, top, width, 2.0), None, line.clone(), None, Some(&group));
    let mut base = line_at(y + h - 1.0);
    // La recette garde aussi les couleurs : une ligne sans pointillés se souvient de la leur.
    if let ElementKind::Shape(s) = &mut base.kind {
        s.graphisme = Some(GraphismeRow { model_color: Some(MODEL_COLOR.into()), trace_color: Some(TRACE_COLOR.into()), ..r.clone() });
    }
    let mut out = vec![base];
    if r.guides == Guides::Corridor {
        out.push(line_at(y - 1.0));
    }
    let first_y = match m {
        "coupes" | "traits" | "ronds" => 0.0,
        "vagues" => h / 2.0,
        _ => h,
    };
    let (cx, cy) = (x + dot / 2.0, y + first_y);
    out.push(shape(ShapeKind::Ellipse, frame(x, cy - dot / 2.0, dot, dot), Some(START_COLOR), None, None, Some(&group)));
    if r.arrow {
        let f = Frame { rotation_deg: arrow_deg(m), ..frame(cx - 3.0, cy - 4.0 - 1.5, 6.0, 3.0) };
        out.push(shape(ShapeKind::Arrow, f, None, stroke(0.8, START_COLOR, StrokeStyle::Solid), None, Some(&group)));
    }
    // Un tracé par suite de cases de même sorte ; les cases vides ne dessinent rien.
    let mut from = 0;
    while from < r.cells.len() {
        let cell = r.cells[from];
        let mut to = from + 1;
        while to < r.cells.len() && r.cells[to] == cell {
            to += 1;
        }
        let look = match cell {
            Cell::Model => stroke(1.0, MODEL_COLOR, StrokeStyle::Solid),
            Cell::Dotted => stroke(1.0, TRACE_COLOR, StrokeStyle::Dotted),
            Cell::Empty => None,
        };
        if look.is_some() {
            let (f, path) = path_in_frame(&cmds_for(m, start, h, from, to), y);
            out.push(shape(ShapeKind::Path, f, None, look, Some(path), Some(&group)));
        }
        from = to;
    }
    out
}

/// Une ligne de graphisme de `width` mm à partir de (`x`, `y`), avec un préréglage (`graphismeRow`).
#[allow(clippy::too_many_arguments)]
fn graphisme_row(m: &str, level: &str, x: f64, y: f64, width: f64, preset: &str, guides: Guides, arrow: bool) -> Vec<Element> {
    let cells = cells_for(preset, motif_count(m, level, width));
    build_row(&GraphismeRow { motif: m.into(), level: level.into(), cells, guides, arrow, model_color: None, trace_color: None }, x, y, width)
}

/// Haut de la place libre : sous le plus bas des éléments de la page.
fn free_top(page: &Page) -> f64 {
    page.elements.iter().map(|e| e.frame.y_mm + e.frame.h_mm + 8.0).fold(MARGIN_MM + 10.0, f64::max)
}

/// Le générateur à graine de l'interface (xorshift sur 32 bits).
fn random(seed: u64) -> impl FnMut() -> f64 {
    let mut s = (seed & 0xffff_ffff) as u32;
    if s == 0 {
        s = 1;
    }
    move || {
        s ^= s << 13;
        s ^= s >> 17;
        s ^= s << 5;
        s as f64 / 4_294_967_296.0
    }
}

/// Murs d'un labyrinthe parfait de `n` × `n` cases (`mazePath`).
fn maze_path(n: usize, rnd: &mut impl FnMut() -> f64) -> String {
    let mut right = vec![vec![true; n]; n];
    let mut down = vec![vec![true; n]; n];
    let mut seen = vec![vec![false; n]; n];
    let mut stack = vec![(0usize, 0usize)];
    seen[0][0] = true;
    while let Some(&(x, y)) = stack.last() {
        let next: Vec<(usize, usize)> = [(1i64, 0i64), (-1, 0), (0, 1), (0, -1)]
            .iter()
            .map(|(dx, dy)| (x as i64 + dx, y as i64 + dy))
            .filter(|&(a, b)| a >= 0 && b >= 0 && (a as usize) < n && (b as usize) < n && !seen[b as usize][a as usize])
            .map(|(a, b)| (a as usize, b as usize))
            .collect();
        if next.is_empty() {
            stack.pop();
            continue;
        }
        let (a, b) = next[(rnd() * next.len() as f64).floor() as usize];
        if a > x {
            right[y][x] = false;
        } else if a < x {
            right[y][a] = false;
        } else if b > y {
            down[y][x] = false;
        } else {
            down[b][x] = false;
        }
        seen[b][a] = true;
        stack.push((a, b));
    }
    let u = |i: usize| js_round(i as f64 * 1000.0 / n as f64) as i64;
    let mut segs: Vec<String> = Vec::new();
    let mut run = |cells: &[bool], at: &dyn Fn(usize) -> (i64, i64, i64, i64)| {
        let mut i = 0;
        while i < cells.len() {
            if !cells[i] {
                i += 1;
                continue;
            }
            let mut j = i;
            while j + 1 < cells.len() && cells[j + 1] {
                j += 1;
            }
            let (x1, y1, _, _) = at(i);
            let (_, _, x2, y2) = at(j);
            segs.push(format!("M {x1} {y1} L {x2} {y2}"));
            i = j + 1;
        }
    };
    run(&vec![true; n], &|i| (u(i), 0, u(i + 1), 0));
    run(&vec![true; n], &|i| (u(i), 1000, u(i + 1), 1000));
    run(&(0..n).map(|i| i != 0).collect::<Vec<_>>(), &|i| (0, u(i), 0, u(i + 1)));
    run(&(0..n).map(|i| i != n - 1).collect::<Vec<_>>(), &|i| (1000, u(i), 1000, u(i + 1)));
    for x in 0..n.saturating_sub(1) {
        run(&right.iter().map(|r| r[x]).collect::<Vec<_>>(), &|y| (u(x + 1), u(y), u(x + 1), u(y + 1)));
    }
    for (y, row) in down.iter().enumerate().take(n.saturating_sub(1)) {
        run(row, &|x| (u(x), u(y + 1), u(x + 1), u(y + 1)));
    }
    segs.join(" ")
}

fn picture(code: &str, f: Frame, group: &Id) -> Element {
    element(new_id(), f, Some(group.clone()), ElementKind::Image(ImageProps { code: code.into(), outline: None }))
}

/// Un labyrinthe sous ce que la page contient, un poussin à l'entrée, une poule à la sortie (`maze`).
fn maze(page: &Page, level: &str, seed: u64) -> Option<Vec<Element>> {
    let n = match level {
        "PS" => 4,
        "GS" => 9,
        _ => 6,
    };
    let pic = 22.0;
    let top = free_top(page);
    let size = (page.size.width_mm - 2.0 * MARGIN_MM - 2.0 * pic).min(page.size.height_mm - MARGIN_MM - top).min(if level == "PS" { 120.0 } else { 150.0 });
    if size < 60.0 {
        return None;
    }
    let x = (page.size.width_mm - size) / 2.0;
    let cell = size / n as f64;
    let group = new_id();
    Some(vec![
        shape(ShapeKind::Path, frame(x, top, size, size), None, stroke(if level == "GS" { 1.0 } else { 1.5 }, "#333333", StrokeStyle::Solid), Some(maze_path(n, &mut random(seed))), Some(&group)),
        picture("1F424", frame(x - pic - 1.0, top + cell / 2.0 - pic / 2.0, pic, pic), &group),
        picture("1F414", frame(x + size + 1.0, top + size - cell / 2.0 - pic / 2.0, pic, pic), &group),
    ])
}

/// Moitiés gauches des dessins à compléter (`SYMMETRY_FIGURES`), en carreaux.
fn figure(name: &str) -> Option<Vec<Vec<(f64, f64)>>> {
    let v = |pts: &[(f64, f64)]| pts.to_vec();
    Some(match name {
        "sapin" => vec![v(&[(0.0, 0.0), (-2.0, 2.0), (-1.0, 2.0), (-3.0, 4.0), (-2.0, 4.0), (-4.0, 6.0), (-1.0, 6.0), (-1.0, 8.0), (0.0, 8.0)])],
        "maison" => vec![
            v(&[(0.0, 0.0), (-4.0, 3.0), (-3.0, 3.0), (-3.0, 8.0), (0.0, 8.0)]),
            v(&[(0.0, 6.0), (-1.0, 6.0), (-1.0, 8.0)]),
            v(&[(-2.0, 4.0), (-1.0, 4.0), (-1.0, 5.0), (-2.0, 5.0), (-2.0, 4.0)]),
        ],
        "fusee" => vec![v(&[(0.0, 0.0), (-1.0, 2.0), (-1.0, 6.0), (-2.0, 7.0), (-2.0, 8.0), (-1.0, 7.0), (0.0, 7.0)]), v(&[(0.0, 3.0), (-0.5, 3.0), (-0.5, 4.0), (0.0, 4.0)])],
        "bateau" => vec![v(&[(0.0, 6.0), (-4.0, 6.0), (-3.0, 8.0), (0.0, 8.0)]), v(&[(0.0, 1.0), (-3.0, 5.0), (0.0, 5.0)])],
        "coeur" => vec![v(&[(0.0, 2.0), (-1.0, 1.0), (-2.0, 1.0), (-3.0, 2.0), (-3.0, 3.0), (0.0, 7.0)])],
        _ => return None,
    })
}

/// Un quadrillage de 8 × 10 carreaux, l'axe rouge, et la moitié gauche d'un dessin (`symmetry`).
fn symmetry(page: &Page, name: &str, level: &str) -> Option<Vec<Element>> {
    let parts = figure(name)?;
    let c = match level {
        "PS" => 20.0,
        "GS" => 12.0,
        _ => 16.0,
    };
    let (cols, rows) = (8usize, 10usize);
    let top = free_top(page);
    if top + rows as f64 * c > page.size.height_mm - MARGIN_MM {
        return None;
    }
    let (w, h) = (cols as f64 * c, rows as f64 * c);
    let x = (page.size.width_mm - w) / 2.0;
    let group = new_id();
    let at = |i: usize, n: usize| js_round(i as f64 * 1000.0 / n as f64) as i64;
    let mut lines: Vec<String> = (0..=cols).map(|i| format!("M {} 0 L {} 1000", at(i, cols), at(i, cols))).collect();
    lines.extend((0..=rows).map(|j| format!("M 0 {} L 1000 {}", at(j, rows), at(j, rows))));
    let px = |gx: f64| js_round((cols as f64 / 2.0 + gx) * 1000.0 / cols as f64) as i64;
    let py = |gy: f64| js_round((gy + 1.0) * 1000.0 / rows as f64) as i64;
    let drawing = parts
        .iter()
        .map(|pts| pts.iter().enumerate().map(|(i, (gx, gy))| format!("{} {} {}", if i > 0 { "L" } else { "M" }, px(*gx), py(*gy))).collect::<Vec<_>>().join(" "))
        .collect::<Vec<_>>()
        .join(" ");
    let f = frame(x, top, w, h);
    Some(vec![
        shape(ShapeKind::Path, f, None, stroke(0.3, "#a0a0a0", StrokeStyle::Solid), Some(lines.join(" ")), Some(&group)),
        shape(ShapeKind::Path, f, None, stroke(1.2, "#1f4e9c", StrokeStyle::Solid), Some(drawing), Some(&group)),
        shape(ShapeKind::Path, frame(x + w / 2.0 - 1.0, top, 2.0, h), None, stroke(0.8, "#d32f2f", StrokeStyle::Dashed), Some("M 500 0 L 500 1000".into()), Some(&group)),
    ])
}

/// Chemins en pointillés à étirer entre deux images (`PATHS`).
fn dotted(id: &str) -> Option<(&'static str, f64, f64)> {
    Some(match id {
        "descend" => ("M 0 100 C 450 100 550 900 1000 900", 120.0, 50.0),
        "monte" => ("M 0 900 C 450 900 550 100 1000 100", 120.0, 50.0),
        "vague" => ("M 0 500 C 83 0 167 0 250 500 C 333 1000 417 1000 500 500 C 583 0 667 0 750 500 C 833 1000 917 1000 1000 500", 140.0, 30.0),
        "arc" => ("M 0 1000 C 0 -333 1000 -333 1000 1000", 120.0, 45.0),
        _ => return None,
    })
}

fn str_arg<'a>(args: &'a Value, key: &str) -> Result<&'a str, String> {
    args.get(key).and_then(Value::as_str).ok_or(format!("« {key} » manque"))
}

/// Fiches et repères : graphisme, labyrinthe, symétrie (sous ce que la page contient), chemin en pointillés,
/// point de départ et flèche de sens (à l'endroit donné).
pub fn exercise(doc: &mut Document, args: &Value) -> Result<Value, String> {
    let index = args.get("page").and_then(Value::as_u64).ok_or("« page » manque")? as usize;
    let page = doc.pages.get(index).ok_or(format!("aucune page {index}"))?;
    let level = match args.get("level").and_then(Value::as_str).unwrap_or("MS") {
        l @ ("PS" | "MS" | "GS") => l,
        l => return Err(format!("niveau inconnu : {l} (PS, MS, GS)")),
    };
    let at = |w: f64, h: f64| -> Result<Frame, String> {
        let n = |k: &str| args.get(k).and_then(Value::as_f64).ok_or(format!("« {k} » manque"));
        Ok(frame(n("x_mm")?, n("y_mm")?, w, h))
    };
    let els: Option<Vec<Element>> = match str_arg(args, "kind")? {
        "graphisme" => {
            let m = str_arg(args, "motif")?;
            if !MOTIFS.contains(&m) {
                return Err(format!("motif inconnu : {m} ({})", MOTIFS.join(", ")));
            }
            let dots = args.get("dots").and_then(Value::as_str).unwrap_or("tout");
            if !PRESETS.contains(&dots) {
                return Err(format!("pointillés inconnus : {dots} ({})", PRESETS.join(", ")));
            }
            let g = args.get("guides").and_then(Value::as_str).unwrap_or("base");
            let guides = GUIDES.iter().find(|(name, _)| *name == g).map(|(_, v)| *v).ok_or(format!("repères inconnus : {g} (base, couloir, aucune)"))?;
            let arrow = match args.get("arrow") {
                None => true,
                Some(v) => v.as_bool().ok_or("« arrow » : true ou false")?,
            };
            let rows = args.get("rows").and_then(Value::as_u64).unwrap_or(4).clamp(1, 12) as usize;
            let h = height(level);
            let gap = h.max(12.0);
            let width = page.size.width_mm - 2.0 * MARGIN_MM;
            // « Progressif » se répartit sur les lignes qui tiennent sur la page.
            let mut ys = Vec::new();
            let mut y = free_top(page);
            while ys.len() < rows && y + h <= page.size.height_mm - MARGIN_MM {
                ys.push(y);
                y += h + gap;
            }
            let out: Vec<Element> = ys
                .iter()
                .enumerate()
                .flat_map(|(i, &y)| graphisme_row(m, level, MARGIN_MM, y, width, if dots == "progressif" { progressive(i, ys.len()) } else { dots }, guides, arrow))
                .collect();
            (!out.is_empty()).then_some(out)
        }
        "maze" => {
            let seed = args.get("seed").and_then(Value::as_u64).unwrap_or_else(|| std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(1));
            maze(page, level, seed)
        }
        "symmetry" => {
            let name = str_arg(args, "figure")?;
            if figure(name).is_none() {
                return Err(format!("dessin inconnu : {name} (sapin, maison, fusee, bateau, coeur)"));
            }
            symmetry(page, name, level)
        }
        "dotted_path" => {
            let id = str_arg(args, "path")?;
            let (d, w, h) = dotted(id).ok_or(format!("chemin inconnu : {id} (descend, monte, vague, arc)"))?;
            Some(vec![shape(ShapeKind::Path, at(w, h)?, None, stroke(1.2, "#555555", StrokeStyle::Dotted), Some(d.into()), None)])
        }
        "start_point" => Some(vec![shape(ShapeKind::Ellipse, at(5.0, 5.0)?, Some(START_COLOR), None, None, None)]),
        "direction_arrow" => Some(vec![shape(ShapeKind::Arrow, at(14.0, 6.0)?, None, stroke(0.8, START_COLOR, StrokeStyle::Solid), None, None)]),
        k => return Err(format!("fiche inconnue : {k} (graphisme, maze, symmetry, dotted_path, start_point, direction_arrow)")),
    };
    let els = els.ok_or(NO_ROOM)?;
    let ids: Vec<Id> = els.iter().map(|e| e.id.clone()).collect();
    doc.pages[index].elements.extend(els);
    Ok(json!({ "element_ids": ids }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn reference(key: &str) -> String {
        let prefix = format!("{key}=");
        include_str!("fixtures/ui_reference.txt").lines().find_map(|l| l.strip_prefix(&prefix)).unwrap().to_string()
    }

    /// Compare chaque forme à ce que l'interface trace : cadre, tracé, recette, et la sorte de forme et son
    /// trait (style, couleur, épaisseur) quand la référence les donne (un ordre différent se verrait).
    fn same_as_ui(els: &[Element], key: &str) {
        let want: Vec<Value> = serde_json::from_str(&reference(key)).unwrap();
        assert_eq!(els.len(), want.len(), "{key}");
        for (i, (e, w)) in els.iter().zip(&want).enumerate() {
            for k in ["x_mm", "y_mm", "w_mm", "h_mm", "rotation_deg"] {
                let got = serde_json::to_value(e.frame).unwrap()[k].as_f64().unwrap();
                assert!((got - w["frame"][k].as_f64().unwrap()).abs() < 1e-9, "{key} {i} {k} : {got} au lieu de {}", w["frame"][k]);
            }
            let ElementKind::Shape(s) = &e.kind else { panic!("{key} {i} : pas une forme") };
            assert_eq!(s.path.as_deref(), w["path"].as_str(), "{key} {i}");
            if let Some(shape) = w.get("shape") {
                assert_eq!(serde_json::to_value(s.shape).unwrap(), *shape, "{key} {i}");
                assert_eq!(serde_json::to_value(s.stroke.as_ref().and_then(|t| t.style)).unwrap(), w["style"], "{key} {i}");
            }
            if let Some(color) = w.get("color") {
                assert_eq!(serde_json::to_value(s.stroke.as_ref().map(|t| &t.color)).unwrap(), *color, "{key} {i}");
            }
            if let Some(width) = w.get("width_mm") {
                let got = s.stroke.as_ref().map(|t| t.width_mm);
                assert!(got.zip(width.as_f64()).map_or(got.is_none() && width.is_null(), |(a, b)| (a - b).abs() < 1e-9), "{key} {i} : épaisseur {got:?} au lieu de {width}");
            }
            assert_eq!(serde_json::to_value(&s.graphisme).unwrap(), w.get("graphisme").cloned().unwrap_or(Value::Null), "{key} {i}");
        }
    }

    #[test]
    fn the_maze_is_the_one_the_ui_draws_for_the_same_seed() {
        assert_eq!(maze_path(6, &mut random(42)), reference("MAZE6_42"));
    }

    #[test]
    fn graphisme_rows_are_drawn_like_the_ui() {
        for (m, key) in [("ponts", "ROW_ponts"), ("boucles", "ROW_boucles"), ("ronds", "ROW_ronds")] {
            same_as_ui(&graphisme_row(m, "MS", 15.0, 25.0, 180.0, "tout", Guides::Base, true), key);
        }
        same_as_ui(&graphisme_row("vagues", "MS", 15.0, 25.0, 180.0, "moitie", Guides::Corridor, true), "ROW_vagues_moitie_couloir");
        same_as_ui(&graphisme_row("dents", "GS", 15.0, 25.0, 180.0, "seul", Guides::None, false), "ROW_dents_gs_seul_aucune");
    }

    #[test]
    fn the_symmetry_sheet_is_drawn_like_the_ui() {
        same_as_ui(&symmetry(&Page::new(carnet_core::A4_PORTRAIT), "maison", "MS").unwrap(), "SYM");
    }

    #[test]
    fn exercises_go_below_the_page_content_in_one_group_each() {
        let mut d = Document::new("x");
        let out = exercise(&mut d, &json!({ "kind": "graphisme", "page": 0, "motif": "vagues", "level": "MS", "rows": 3 })).unwrap();
        assert_eq!(out["element_ids"].as_array().unwrap().len(), 15);
        let groups: std::collections::HashSet<_> = d.pages[0].elements.iter().map(|e| e.group.clone().unwrap()).collect();
        assert_eq!(groups.len(), 3);
        exercise(&mut d, &json!({ "kind": "maze", "page": 0, "level": "PS", "seed": 7 })).unwrap();
        let maze = &d.pages[0].elements[15];
        assert!(maze.frame.y_mm > d.pages[0].elements[14].frame.y_mm, "sous les lignes");
        let images: Vec<&str> = d.pages[0].elements[16..].iter().map(|e| match &e.kind { ElementKind::Image(i) => i.code.as_str(), _ => "" }).collect();
        assert_eq!(images, ["1F424", "1F414"]);
        // Plus de place : le message de l'interface.
        let err = exercise(&mut d, &json!({ "kind": "symmetry", "page": 0, "figure": "sapin", "level": "PS" })).unwrap_err();
        assert!(err.starts_with("Plus de place sur la page"), "{err}");
        assert!(exercise(&mut d, &json!({ "kind": "graphisme", "page": 0, "motif": "spirales", "level": "MS" })).unwrap_err().contains("ponts"));
    }

    /// Les recettes des lignes posées sur la page 0, dans l'ordre.
    fn recipes(d: &Document) -> Vec<GraphismeRow> {
        d.pages[0].elements.iter().filter_map(|e| match &e.kind { ElementKind::Shape(s) => s.graphisme.clone(), _ => None }).collect()
    }

    fn dotted_cells(r: &GraphismeRow) -> usize {
        r.cells.iter().filter(|c| **c == Cell::Dotted).count()
    }

    #[test]
    fn exercise_takes_dots_guides_and_arrow() {
        let mut d = Document::new("x");
        exercise(&mut d, &json!({ "kind": "graphisme", "page": 0, "motif": "ponts", "level": "MS", "rows": 4, "dots": "progressif", "guides": "couloir", "arrow": false })).unwrap();
        let rs = recipes(&d);
        assert_eq!(rs.len(), 4);
        let dots: Vec<usize> = rs.iter().map(dotted_cells).collect();
        assert!(dots.windows(2).all(|w| w[0] > w[1]), "{dots:?}");
        assert_eq!(dots[3], 0, "la dernière ligne : le modèle seul");
        assert!(rs.iter().all(|r| r.guides == Guides::Corridor && !r.arrow));
        assert_eq!((rs[0].model_color.as_deref(), rs[0].trace_color.as_deref()), (Some("#333333"), Some("#9e9e9e")));
        let shapes: Vec<ShapeKind> = d.pages[0].elements.iter().filter_map(|e| match &e.kind { ElementKind::Shape(s) => Some(s.shape), _ => None }).collect();
        assert_eq!(shapes.iter().filter(|s| **s == ShapeKind::Line).count(), 8, "ligne de base et ligne du haut");
        assert!(!shapes.contains(&ShapeKind::Arrow));
        // « Progressif » se répartit sur les lignes qui tiennent : 5 en PS sur une page A4, la dernière seule.
        let mut d = Document::new("x");
        exercise(&mut d, &json!({ "kind": "graphisme", "page": 0, "motif": "ponts", "level": "PS", "rows": 12, "dots": "progressif", "guides": "aucune" })).unwrap();
        let rs = recipes(&d);
        assert_eq!((rs.len(), dotted_cells(&rs[4])), (5, 0));
        let ElementKind::Shape(base) = &d.pages[0].elements[0].kind else { unreachable!() };
        assert_eq!((base.shape, base.stroke.is_none(), rs[0].arrow), (ShapeKind::Line, true, true));
    }

    #[test]
    fn exercise_refuses_unknown_dots() {
        let mut d = Document::new("x");
        let base = json!({ "kind": "graphisme", "page": 0, "motif": "ponts" });
        let with = |k: &str, v: Value| {
            let mut a = base.clone();
            a[k] = v;
            a
        };
        assert!(exercise(&mut d, &with("dots", json!("beaucoup"))).unwrap_err().contains("tout, moitie, quelques, seul, progressif"));
        assert!(exercise(&mut d, &with("guides", json!("Corridor"))).unwrap_err().contains("base, couloir, aucune"));
        assert!(exercise(&mut d, &with("arrow", json!("oui"))).is_err());
        assert!(d.pages[0].elements.is_empty());
    }

    #[test]
    fn a_dotted_path_and_writing_markers_where_asked() {
        let mut d = Document::new("x");
        let p = exercise(&mut d, &json!({ "kind": "dotted_path", "page": 0, "path": "arc", "x_mm": 40, "y_mm": 100 })).unwrap();
        let e = &d.pages[0].elements[0];
        assert_eq!((e.frame.x_mm, e.frame.w_mm, e.frame.h_mm, p["element_ids"][0].as_str()), (40.0, 120.0, 45.0, Some(e.id.as_str())));
        let ElementKind::Shape(s) = &e.kind else { unreachable!() };
        assert_eq!((s.stroke.clone().unwrap().style, s.path.as_deref()), (Some(StrokeStyle::Dotted), Some("M 0 1000 C 0 -333 1000 -333 1000 1000")));
        exercise(&mut d, &json!({ "kind": "start_point", "page": 0, "x_mm": 10, "y_mm": 10 })).unwrap();
        exercise(&mut d, &json!({ "kind": "direction_arrow", "page": 0, "x_mm": 10, "y_mm": 20 })).unwrap();
        let ElementKind::Shape(dot) = &d.pages[0].elements[1].kind else { unreachable!() };
        assert_eq!((dot.shape, dot.fill.as_deref(), d.pages[0].elements[1].frame.w_mm), (ShapeKind::Ellipse, Some("#2e9d4a"), 5.0));
        assert!(matches!(&d.pages[0].elements[2].kind, ElementKind::Shape(s) if s.shape == ShapeKind::Arrow));
    }
}
