//! Géométrie des formes dessinées par un tracé : le même tracé sert à l'écran (SVG) et à l'export
//! (forme personnalisée ODG, toujours modifiable dans LibreOffice).

use crate::model::{HeadSize, LineEnd, LineEnds, ShapeKind, ShapeProps};

/// Le tracé s'inscrit dans un carré de `VIEWBOX` unités, étiré à la taille du cadre.
pub const VIEWBOX: f64 = 1000.0;

/// Tracé (commandes M, L, C et Z) ; `None` pour les formes natives (rectangle, ellipse, traits).
pub fn path(kind: ShapeKind) -> Option<&'static str> {
    Some(match kind {
        ShapeKind::Star => "M 500 0 L 618 382 L 1000 382 L 691 618 L 809 1000 L 500 764 L 191 1000 L 309 618 L 0 382 L 382 382 Z",
        ShapeKind::Heart => {
            "M 500 1000 C 150 760 0 560 0 310 C 0 120 150 0 300 0 C 400 0 470 50 500 130 C 530 50 600 0 700 0 C 850 0 1000 120 1000 310 C 1000 560 850 760 500 1000 Z"
        }
        // Bulle de parole : coins bien arrondis, pointe courbe en bas à gauche ; dessinée pour le cadre par défaut
        // (70 × 45 mm), où les coins restent ronds.
        ShapeKind::Bubble => {
            "M 129 0 L 871 0 C 942 0 1000 90 1000 200 L 1000 578 C 1000 688 942 778 871 778 L 386 778 C 343 889 243 967 129 1000 C 186 922 229 856 229 778 L 129 778 C 58 778 0 688 0 578 L 0 200 C 0 90 58 0 129 0 Z"
        }
        // Nuage : neuf bosses rondes de tailles inégales, comme dessiné à la main, pour le cadre par défaut.
        ShapeKind::Cloud => {
            "M 349 122 C 387 47 448 2 512 0 C 576 -2 638 40 678 113 C 770 87 859 174 877 308 C 937 326 983 393 996 480 C 1009 566 987 656 938 709 C 927 766 899 813 861 836 C 824 859 781 855 746 825 C 735 910 690 977 632 995 C 574 1013 514 979 480 908 C 450 957 404 978 360 963 C 315 947 281 898 269 834 C 219 893 149 901 93 853 C 37 805 8 712 19 620 C 0 571 -5 512 6 458 C 17 404 43 359 78 334 C 84 245 120 166 175 123 C 229 81 294 80 349 122 Z"
        }
        ShapeKind::Rect | ShapeKind::Ellipse | ShapeKind::Line | ShapeKind::Arrow | ShapeKind::Path => return None,
    })
}

/// Anciens tracés de la bulle et du nuage (jusqu'à la version 0.18) : un fichier LibreOffice écrit
/// avant leur nouveau dessin les rend toujours comme bulle et nuage à l'import.
pub const LEGACY_PATHS: [(ShapeKind, &str); 2] = [
    (ShapeKind::Bubble, "M 120 0 L 880 0 C 946 0 1000 54 1000 120 L 1000 660 C 1000 726 946 780 880 780 L 380 780 L 180 1000 L 240 780 L 120 780 C 54 780 0 726 0 660 L 0 120 C 0 54 54 0 120 0 Z"),
    (ShapeKind::Cloud, "M 500 127 C 598 -89 876 -8 779 209 C 917 144 1065 351 927 416 C 1054 434 1002 669 876 651 C 1015 785 787 938 648 805 C 648 1065 352 1065 352 805 C 213 938 -15 785 124 651 C -2 669 -54 434 73 416 C -65 351 83 144 221 209 C 124 -8 402 -89 500 127 Z"),
];

/// Plus grand nombre de points d'un chemin.
pub const MAX_PATH_POINTS: usize = 200;

/// Points d'un chemin acceptables : 3 à 200, finis, dans le cadre (fractions de 0 à 1).
pub fn valid_points(p: &[[f64; 2]]) -> bool {
    const EPS: f64 = 1e-6;
    (3..=MAX_PATH_POINTS).contains(&p.len()) && p.iter().flatten().all(|v| v.is_finite() && (-EPS..=1.0 + EPS).contains(v))
}

/// Points d'un chemin en mm du cadre (avant rotation et miroir), s'il en a de valides.
pub fn path_points_mm(frame: &crate::model::Frame, s: &ShapeProps) -> Option<Vec<(f64, f64)>> {
    let p = s.points.as_deref().filter(|p| valid_points(p))?;
    Some(p.iter().map(|[x, y]| (x * frame.w_mm, y * frame.h_mm)).collect())
}

/// Points visibles d'un chemin en mm de page : rotation et miroir du cadre compris (comme `pagePoints`, linePath.ts).
pub fn path_page_points(frame: &crate::model::Frame, s: &ShapeProps, flip_h: bool, flip_v: bool) -> Option<Vec<(f64, f64)>> {
    let p = s.points.as_deref().filter(|p| valid_points(p))?;
    let (cx, cy) = (frame.x_mm + frame.w_mm / 2.0, frame.y_mm + frame.h_mm / 2.0);
    let (sin, cos) = frame.rotation_deg.to_radians().sin_cos();
    Some(
        p.iter()
            .map(|&[fx, fy]| {
                let dx = (if flip_h { 1.0 - fx } else { fx }) * frame.w_mm - frame.w_mm / 2.0;
                let dy = (if flip_v { 1.0 - fy } else { fy }) * frame.h_mm - frame.h_mm / 2.0;
                (cx + dx * cos - dy * sin, cy + dx * sin + dy * cos)
            })
            .collect(),
    )
}

/// Cadre d'un trait droit de `a` à `b` (mm de page) : le segment va du milieu du bord gauche au milieu du bord droit,
/// tourné ; haut de la zone cliquable (comme `frameFromEndpoints`, lineEnds.ts, avec la hauteur de `fromPagePoints`).
pub fn line_frame(a: (f64, f64), b: (f64, f64), s: &ShapeProps) -> crate::model::Frame {
    let h = head_width_mm(s).max(4.0);
    let w = (b.0 - a.0).hypot(b.1 - a.1);
    let (cx, cy) = ((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0);
    crate::model::Frame { x_mm: cx - w / 2.0, y_mm: cy - h / 2.0, w_mm: w, h_mm: h, rotation_deg: (b.1 - a.1).atan2(b.0 - a.0).to_degrees() }
}

/// Cadre et points (fractions) d'un chemin qui passe par `pts` (mm de page, 3 points au moins) : la boîte des points
/// agrandie de la moitié de la zone cliquable (la plus grande terminaison, 4 mm au moins), sans rotation. Comme
/// `fromPagePoints` (linePath.ts).
pub fn path_frame(pts: &[(f64, f64)], s: &ShapeProps) -> (crate::model::Frame, Vec<[f64; 2]>) {
    let m = head_width_mm(s).max(4.0) / 2.0;
    let fold = |f: fn(f64, f64) -> f64, init: f64, get: fn(&(f64, f64)) -> f64| pts.iter().map(get).fold(init, f);
    let (x0, y0) = (fold(f64::min, f64::INFINITY, |p| p.0) - m, fold(f64::min, f64::INFINITY, |p| p.1) - m);
    let (w, h) = (fold(f64::max, f64::NEG_INFINITY, |p| p.0) + m - x0, fold(f64::max, f64::NEG_INFINITY, |p| p.1) + m - y0);
    let frame = crate::model::Frame { x_mm: x0, y_mm: y0, w_mm: w, h_mm: h, rotation_deg: 0.0 };
    (frame, pts.iter().map(|&(x, y)| [(x - x0) / w, (y - y0) / h]).collect())
}

/// Courbe d'un chemin : Catmull-Rom centripète passant par les points, une cubique par segment (contrôle 1,
/// contrôle 2, arrivée). Aux bouts, la tangente va vers le voisin. Même calcul que `curveBeziers` (linePath.ts).
pub fn curve_beziers(pts: &[(f64, f64)]) -> Vec<[(f64, f64); 3]> {
    let n = pts.len();
    (0..n.saturating_sub(1)).map(|i| cr_segment(if i == 0 { pts[0] } else { pts[i - 1] }, pts[i], pts[i + 1], if i + 2 < n { pts[i + 2] } else { pts[i + 1] })).collect()
}

/// Courbe d'un chemin fermé : la même, qui revient du dernier point au premier sans angle (chaque point a ses deux
/// voisins). Même calcul que `closedCurveBeziers` (linePath.ts).
pub fn closed_curve_beziers(pts: &[(f64, f64)]) -> Vec<[(f64, f64); 3]> {
    let n = pts.len();
    (0..n).map(|i| cr_segment(pts[(i + n - 1) % n], pts[i], pts[(i + 1) % n], pts[(i + 2) % n])).collect()
}

/// Un segment de Catmull-Rom centripète de `p1` à `p2` (voisins `p0` et `p3`) en cubique. Un voisin confondu avec le
/// bout donne une tangente vers l'autre bout du segment ; un segment nul reste un point.
fn cr_segment(p0: (f64, f64), p1: (f64, f64), p2: (f64, f64), p3: (f64, f64)) -> [(f64, f64); 3] {
    const EPS: f64 = 1e-12;
    let dist = |a: (f64, f64), b: (f64, f64)| ((b.0 - a.0).powi(2) + (b.1 - a.1).powi(2)).sqrt().sqrt();
    let lerp3 = |a: (f64, f64), b: (f64, f64)| (a.0 + (b.0 - a.0) / 3.0, a.1 + (b.1 - a.1) / 3.0);
    let (d1, d2, d3) = (dist(p0, p1), dist(p1, p2), dist(p2, p3));
    if d2 < EPS {
        return [p1, p2, p2];
    }
    let ctl = |da: f64, pa: (f64, f64), pb: (f64, f64), pc: (f64, f64)| {
        let k = 3.0 * da * (da + d2);
        let a = 2.0 * da * da + 3.0 * da * d2 + d2 * d2;
        ((da * da * pc.0 - d2 * d2 * pa.0 + a * pb.0) / k, (da * da * pc.1 - d2 * d2 * pa.1 + a * pb.1) / k)
    };
    let b1 = if d1 < EPS { lerp3(p1, p2) } else { ctl(d1, p0, p1, p2) };
    let b2 = if d3 < EPS { lerp3(p2, p1) } else { ctl(d3, p3, p2, p1) };
    [b1, b2, p2]
}

/// Le tracé d'une forme : celui de sa sorte, ou pour un tracé libre le sien, s'il est valide.
pub fn path_of<'a>(frame: &crate::model::Frame, s: &'a crate::model::ShapeProps) -> Option<std::borrow::Cow<'a, str>> {
    use std::borrow::Cow;
    match (s.shape, tail_of(s)) {
        (ShapeKind::Path, _) => s.path.as_deref().filter(|p| valid_path(p)).map(Cow::Borrowed),
        (ShapeKind::Bubble, Some(t)) => Some(Cow::Owned(bubble_path(frame.w_mm, frame.h_mm, t))),
        (ShapeKind::Cloud, Some(t)) => Some(Cow::Owned(thought_path(frame.w_mm, frame.h_mm, t))),
        (kind, _) => path(kind).map(Cow::Borrowed),
    }
}

/// Quart de cercle en courbe de Bézier.
const KAPPA: f64 = 0.5522847498;
/// Pointe hors de ces bornes (fractions du cadre) : document abîmé, pas de pointe.
const TAIL_LIMIT: f64 = 20.0;

/// La pointe d'une bulle ou d'un nuage (`ShapeProps::tail`), si elle est lisible.
pub fn tail_of(s: &ShapeProps) -> Option<[f64; 2]> {
    if !matches!(s.shape, ShapeKind::Bubble | ShapeKind::Cloud) {
        return None;
    }
    s.tail.filter(|t| t.iter().all(|v| v.is_finite() && v.abs() <= TAIL_LIMIT))
}

/// Tracé en mm du cadre, écrit dans le carré de 1000 étiré au cadre. Même écriture que `ui/src/lib/tail.ts`.
struct PathOut {
    w: f64,
    h: f64,
    parts: Vec<String>,
}

type P = (f64, f64);

impl PathOut {
    fn p(&self, q: P) -> String {
        let n = |v: f64| (v + 0.5).floor() as i64;
        format!("{} {}", n(q.0 * VIEWBOX / self.w), n(q.1 * VIEWBOX / self.h))
    }
    fn m(&mut self, q: P) {
        let s = format!("M {}", self.p(q));
        self.parts.push(s);
    }
    fn l(&mut self, q: P) {
        let s = format!("L {}", self.p(q));
        self.parts.push(s);
    }
    fn c(&mut self, a: P, b: P, q: P) {
        let s = format!("C {} {} {}", self.p(a), self.p(b), self.p(q));
        self.parts.push(s);
    }
    fn z(&mut self) {
        self.parts.push("Z".into());
    }
}

fn add(a: P, b: P, k: f64) -> P {
    (a.0 + b.0 * k, a.1 + b.1 * k)
}

/// Bulle de parole de `w` × `h` mm à pointe réglable : voir `bubblePath` (`ui/src/lib/tail.ts`), même calcul.
pub fn bubble_path(w: f64, h: f64, tail: [f64; 2]) -> String {
    let r = 9f64.min(w * 0.25).min(h * 0.25);
    let k = r * KAPPA;
    let t = (tail[0] * w, tail[1] * h);
    let d = (t.0 - w / 2.0, t.1 - h / 2.0);
    let inside = t.0 >= 0.0 && t.0 <= w && t.1 >= 0.0 && t.1 <= h;
    let side: i32 = if inside {
        -1
    } else if d.1.abs() * w >= d.0.abs() * h {
        if d.1 > 0.0 { 2 } else { 0 }
    } else if d.0 > 0.0 {
        1
    } else {
        3
    };
    // Début et fin de la partie droite de chaque côté, sens de parcours, normale vers l'extérieur.
    let sides: [(P, P, P, P); 4] = [
        ((r, 0.0), (w - r, 0.0), (1.0, 0.0), (0.0, -1.0)),
        ((w, r), (w, h - r), (0.0, 1.0), (1.0, 0.0)),
        ((w - r, h), (r, h), (-1.0, 0.0), (0.0, 1.0)),
        ((0.0, h - r), (0.0, r), (0.0, -1.0), (-1.0, 0.0)),
    ];
    let corners: [[P; 3]; 4] = [
        [(w - r + k, 0.0), (w, r - k), (w, r)],
        [(w, h - r + k), (w - r + k, h), (w - r, h)],
        [(r - k, h), (0.0, h - r + k), (0.0, h - r)],
        [(0.0, r - k), (r - k, 0.0), (r, 0.0)],
    ];
    let mut out = PathOut { w, h, parts: Vec::new() };
    out.m(sides[0].0);
    for (i, s) in sides.iter().enumerate() {
        if i as i32 == side {
            tail_on(&mut out, s.2, s.3, side, w, h, r, t, d);
        }
        out.l(s.1);
        let [a, b, q] = corners[i];
        out.c(a, b, q);
    }
    out.z();
    out.parts.join(" ")
}

#[allow(clippy::too_many_arguments)]
fn tail_on(out: &mut PathOut, e: P, nrm: P, side: i32, w: f64, h: f64, r: f64, t: P, d: P) {
    let vertical = side == 0 || side == 2;
    let len = if vertical { w } else { h };
    let depth = match side {
        0 => -t.1,
        1 => t.0 - w,
        2 => t.1 - h,
        _ => -t.0,
    };
    let bw = 11f64.max(depth * 0.5).min(len * 0.35).min(len - 2.0 * r);
    if bw <= 0.5 {
        return;
    }
    let cross = if vertical { w / 2.0 + (d.0 * (h / 2.0)) / d.1.abs() } else { h / 2.0 + (d.1 * (w / 2.0)) / d.0.abs() };
    let m0 = cross + (len / 2.0 - cross) * 0.25;
    let m = (len - r - bw / 2.0).min((r + bw / 2.0).max(m0));
    let mid = if vertical { (m, if side == 0 { 0.0 } else { h }) } else { (if side == 3 { 0.0 } else { w }, m) };
    let b1 = add(mid, e, -bw / 2.0);
    let b2 = add(mid, e, bw / 2.0);
    let a = add(b1, nrm, depth * 0.5);
    let b = add(b2, nrm, depth * 0.35);
    out.l(b1);
    out.c(a, add(t, (a.0 - t.0, a.1 - t.1), 0.5), t);
    out.c(add(t, (b.0 - t.0, b.1 - t.1), 0.4), b, b2);
}

/// Bulle de pensée : le nuage et trois ronds vers le point visé ; voir `thoughtPath` (`ui/src/lib/tail.ts`).
pub fn thought_path(w: f64, h: f64, tail: [f64; 2]) -> String {
    let cloud = path(ShapeKind::Cloud).unwrap_or_default();
    let t = (tail[0] * w, tail[1] * h);
    if t.0 >= 0.0 && t.0 <= w && t.1 >= 0.0 && t.1 <= h {
        return cloud.to_string();
    }
    let d = (t.0 - w / 2.0, t.1 - h / 2.0);
    let (ax, ay) = (d.0.abs(), d.1.abs());
    let f = if ax * h >= ay * w { w / 2.0 / ax } else { h / 2.0 / ay };
    let e = add((w / 2.0, h / 2.0), d, f);
    let full = (d.0 * d.0 + d.1 * d.1).sqrt();
    let len = full * (1.0 - f);
    let u = (d.0 / full, d.1 / full);
    let big = (0.12 * w.min(h)).min(len / 5.25);
    if big < 0.4 {
        return cloud.to_string();
    }
    let s1 = 1.6 * big;
    let s2 = (2.6 * big + len - 0.45 * big) / 2.0;
    let mut out = PathOut { w, h, parts: Vec::new() };
    for (s, rad) in [(s1, big), (s2, 0.7 * big), (len, 0.45 * big)] {
        let c = add(e, u, s);
        let kk = rad * KAPPA;
        out.m((c.0, c.1 - rad));
        out.c((c.0 + kk, c.1 - rad), (c.0 + rad, c.1 - kk), (c.0 + rad, c.1));
        out.c((c.0 + rad, c.1 + kk), (c.0 + kk, c.1 + rad), (c.0, c.1 + rad));
        out.c((c.0 - kk, c.1 + rad), (c.0 - rad, c.1 + kk), (c.0 - rad, c.1));
        out.c((c.0 - rad, c.1 - kk), (c.0 - kk, c.1 - rad), (c.0, c.1 - rad));
        out.z();
    }
    format!("{cloud} {}", out.parts.join(" "))
}

/// Plus long tracé libre accepté (une ligne de graphisme en fait quelques milliers de caractères).
const MAX_PATH_LEN: usize = 20_000;

/// Un tracé libre : commandes M, L, C et Z suivies de leurs nombres (2, 2, 6 et 0), séparés par des espaces,
/// commençant par M. Tout autre contenu (commandes relatives, arcs, lettres, texte) est refusé : le même
/// tracé part tel quel dans l'ODG.
pub fn valid_path(p: &str) -> bool {
    if p.len() > MAX_PATH_LEN || !p.trim_start().starts_with('M') {
        return false;
    }
    let mut tokens = p.split_whitespace().peekable();
    while let Some(cmd) = tokens.next() {
        let n = match cmd {
            "M" | "L" => 2,
            "C" => 6,
            "Z" => 0,
            _ => return false,
        };
        for _ in 0..n {
            match tokens.next().map(str::parse::<f64>) {
                Some(Ok(v)) if v.is_finite() && v.abs() <= 100_000.0 => {}
                _ => return false,
            }
        }
    }
    true
}

/// Trait ou flèche : un segment, du milieu du bord gauche au milieu du bord droit du cadre.
pub fn is_stroke_only(kind: ShapeKind) -> bool {
    matches!(kind, ShapeKind::Line | ShapeKind::Arrow)
}

/// Terminaisons d'un trait qui n'en précise pas : aucune pour un trait, une flèche pleine au bout pour une flèche.
pub fn default_ends(kind: ShapeKind) -> LineEnds {
    let end = if kind == ShapeKind::Arrow { LineEnd::Arrow } else { LineEnd::None };
    LineEnds { start: LineEnd::None, end, size: HeadSize::Medium }
}

/// Terminaisons d'un trait : les siennes, ou celles de son genre.
pub fn ends_of(s: &ShapeProps) -> LineEnds {
    s.ends.unwrap_or_else(|| default_ends(s.shape))
}

/// Facteur de chaque taille de terminaison ; les mêmes à l'écran (`ui/src/lib/shapePaths.ts`, un test le vérifie).
pub const HEAD_FACTORS: [(HeadSize, f64); 3] = [(HeadSize::Small, 0.7), (HeadSize::Medium, 1.0), (HeadSize::Large, 1.5)];

/// Cadre d'un trait assez haut pour sa plus grande terminaison, autour du même centre (comme `withHitHeight` à
/// l'écran) : la zone cliquable la couvre.
pub fn grow_to_hit_height(f: &mut crate::model::Frame, s: &ShapeProps) {
    let ends = ends_of(s);
    if !is_stroke_only(s.shape) || (ends.start == LineEnd::None && ends.end == LineEnd::None) {
        return;
    }
    let h = head_width_mm(s);
    if f.h_mm < h {
        f.y_mm += (f.h_mm - h) / 2.0;
        f.h_mm = h;
    }
}

/// Diamètre d'un rond, côté d'un carré : cette part de la largeur d'une pointe.
pub const DOT_FACTOR: f64 = 0.6;

/// Largeur d'une pointe de flèche, en mm : quatre fois le trait, 4 mm au moins (la taille moyenne), multipliée par
/// le facteur de la taille choisie.
pub fn head_width_mm(s: &ShapeProps) -> f64 {
    let size = ends_of(s).size;
    let factor = HEAD_FACTORS.iter().find(|(k, _)| *k == size).map_or(1.0, |(_, f)| *f);
    (s.stroke.as_ref().map_or(1.0, |st| st.width_mm) * 4.0).max(4.0) * factor
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `curve_beziers` de [(0,0),(10,10),(20,0)], arrondi au millionième (copié dans linePath.test.ts).
    const CURVE_PARITY: [f64; 12] = [3.333333, 3.333333, 6.666667, 10.0, 10.0, 10.0, 13.333333, 10.0, 16.666667, 3.333333, 20.0, 0.0];

    #[test]
    fn points_absent_by_default() {
        let json = r##"{"shape":"Line","fill":null,"stroke":null,"corner_radius_mm":0}"##;
        let s: ShapeProps = serde_json::from_str(json).unwrap();
        assert_eq!((s.points.clone(), s.curve), (None, false));
        let back = serde_json::to_string(&s).unwrap();
        assert!(!back.contains("\"points\"") && !back.contains("\"curve\""), "{back}");
    }

    #[test]
    fn valid_points_bounds() {
        let p = |n: usize| vec![[0.5, 0.5]; n];
        assert!(!valid_points(&p(2)));
        assert!(valid_points(&p(3)));
        assert!(valid_points(&p(200)));
        assert!(!valid_points(&p(201)));
        assert!(!valid_points(&[[0.0, 0.0], [f64::NAN, 0.0], [1.0, 1.0]]));
        assert!(!valid_points(&[[0.0, 0.0], [1.5, 0.0], [1.0, 1.0]]));
    }

    #[test]
    fn curve_passes_through_points() {
        let pts = [(0.0, 0.0), (10.0, 10.0), (20.0, 0.0)];
        let c = curve_beziers(&pts);
        assert_eq!(c.len(), 2);
        assert_eq!(c[0][2], (10.0, 10.0));
        assert_eq!(c[1][2], (20.0, 0.0));
        // Départ : la tangente va vers le point suivant.
        let (dx, dy) = (c[0][0].0 - 0.0, c[0][0].1 - 0.0);
        assert!((dy / dx - 1.0).abs() < 1e-9, "{:?}", c[0][0]);
        // Valeurs partagées avec linePath.test.ts (parité).
        let r = |v: f64| (v * 1e6).round() / 1e6;
        let flat: Vec<f64> = c.iter().flat_map(|b| b.iter().flat_map(|&(x, y)| [r(x), r(y)])).collect();
        assert_eq!(flat, CURVE_PARITY.to_vec());
    }

    /// Mêmes valeurs que « frame keeps the hit margin » (linePath.test.ts).
    #[test]
    fn path_frame_parity() {
        let s: ShapeProps = serde_json::from_str(r##"{"shape":"Line","fill":null,"stroke":{"width_mm":1,"color":"#333333","style":null},"corner_radius_mm":0}"##).unwrap();
        let (f, p) = path_frame(&[(10.0, 50.0), (30.0, 50.0), (50.0, 50.0)], &s);
        assert_eq!((f.x_mm, f.y_mm, f.w_mm, f.h_mm, f.rotation_deg), (8.0, 48.0, 44.0, 4.0, 0.0));
        assert_eq!(p, vec![[2.0 / 44.0, 0.5], [22.0 / 44.0, 0.5], [42.0 / 44.0, 0.5]]);
    }

    #[test]
    fn closed_absent_by_default() {
        let s: ShapeProps = serde_json::from_str(r##"{"shape":"Line","fill":null,"stroke":null,"corner_radius_mm":0}"##).unwrap();
        assert!(!s.closed);
        assert!(!serde_json::to_string(&s).unwrap().contains("closed"));
    }

    /// Valeurs partagées avec linePath.test.ts (parité) : la courbe fermée d'un carré.
    const CLOSED_PARITY: [f64; 24] = [1.666667, -1.666667, 8.333333, -1.666667, 10.0, 0.0, 11.666667, 1.666667, 11.666667, 8.333333, 10.0, 10.0, 8.333333, 11.666667, 1.666667, 11.666667, 0.0, 10.0, -1.666667, 8.333333, -1.666667, 1.666667, 0.0, 0.0];

    #[test]
    fn closed_curve_wraps_around() {
        let pts = [(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)];
        let c = closed_curve_beziers(&pts);
        assert_eq!(c.len(), 4);
        assert_eq!(c[3][2], (0.0, 0.0));
        // Au premier point, la tangente est parallèle à (dernier -> deuxième), comme à tout autre point.
        let (dx, dy) = (c[0][0].0 - 0.0, c[0][0].1 - 0.0);
        let (ex, ey) = (10.0 - 0.0, 0.0 - 10.0);
        assert!((dx * ey - dy * ex).abs() < 1e-9, "{:?}", c[0][0]);
        let r = |v: f64| (v * 1e6).round() / 1e6;
        let flat: Vec<f64> = c.iter().flat_map(|b| b.iter().flat_map(|&(x, y)| [r(x), r(y)])).collect();
        assert_eq!(flat, CLOSED_PARITY.to_vec());
    }

    #[test]
    fn repeated_points_stay_finite_rust() {
        for b in curve_beziers(&[(0.0, 0.0), (0.0, 0.0), (5.0, 5.0)]) {
            for (x, y) in b {
                assert!(x.is_finite() && y.is_finite());
            }
        }
    }

    #[test]
    fn only_simple_absolute_paths_are_valid() {
        assert!(valid_path("M 0 1000 C 0 0 250 0 250 1000 L 300 1000 Z"));
        assert!(valid_path("M 0.5 -2 L 10 20"));
        for bad in ["", "L 0 0", "M 0", "M 0 0 c 1 2 3 4 5 6", "M 0 0 A 1 1 0 0 1 2 2", "M 0 0 L x 1", "M 0 0 L NaN 1", "<svg>"] {
            assert!(!valid_path(bad), "{bad:?}");
        }
        assert!(!valid_path(&format!("M 0 0{}", " L 1 1".repeat(5000))));
    }

    /// Points de passage (extrémités des segments et des courbes) du tracé.
    fn anchors(p: &str) -> Vec<(f64, f64)> {
        let mut out = Vec::new();
        let mut nums: Vec<f64> = Vec::new();
        for tok in p.split_whitespace() {
            match tok {
                "M" | "L" | "C" | "Z" => {
                    if nums.len() >= 2 {
                        out.push((nums[nums.len() - 2], nums[nums.len() - 1]));
                    }
                    nums.clear();
                }
                n => nums.push(n.parse().expect("nombre")),
            }
        }
        out
    }

    #[test]
    fn drawn_shapes_have_a_closed_path_filling_the_frame() {
        for kind in [ShapeKind::Star, ShapeKind::Heart, ShapeKind::Bubble, ShapeKind::Cloud] {
            let p = path(kind).unwrap();
            assert!(p.starts_with("M ") && p.ends_with(" Z"), "{kind:?}");
            let pts = anchors(p);
            let (min_x, max_x) = pts.iter().fold((f64::MAX, f64::MIN), |(a, b), p| (a.min(p.0), b.max(p.0)));
            let (min_y, max_y) = pts.iter().fold((f64::MAX, f64::MIN), |(a, b), p| (a.min(p.1), b.max(p.1)));
            assert!(min_x >= -1.0 && max_x <= VIEWBOX + 1.0 && min_y >= -1.0 && max_y <= VIEWBOX + 1.0, "{kind:?}");
            assert!(max_x - min_x > 600.0 && max_y - min_y > 600.0, "{kind:?} ne remplit pas son cadre");
        }
        for kind in [ShapeKind::Rect, ShapeKind::Ellipse, ShapeKind::Line, ShapeKind::Arrow] {
            assert!(path(kind).is_none());
        }
        assert!(is_stroke_only(ShapeKind::Arrow) && !is_stroke_only(ShapeKind::Star));
    }

    fn line(kind: ShapeKind, width_mm: f64, ends: Option<LineEnds>) -> ShapeProps {
        ShapeProps {
            shape: kind,
            fill: None,
            stroke: Some(crate::model::Stroke { width_mm, color: "#333333".into(), style: None }),
            corner_radius_mm: 0.0,
            path: None,
            graphisme: None,
            ends,
            points: None,
            curve: false,
            closed: false,
            attach: None, tail: None,
        }
    }

    #[test]
    fn ends_default_by_kind() {
        let medium = HeadSize::Medium;
        assert_eq!(ends_of(&line(ShapeKind::Line, 1.0, None)), LineEnds { start: LineEnd::None, end: LineEnd::None, size: medium });
        assert_eq!(ends_of(&line(ShapeKind::Arrow, 1.0, None)), LineEnds { start: LineEnd::None, end: LineEnd::Arrow, size: medium });
        let set = LineEnds { start: LineEnd::Circle, end: LineEnd::OpenArrow, size: HeadSize::Large };
        assert_eq!(ends_of(&line(ShapeKind::Line, 1.0, Some(set))), set);
    }

    #[test]
    fn hit_height_covers_the_largest_end() {
        let large = Some(LineEnds { start: LineEnd::None, end: LineEnd::Arrow, size: HeadSize::Large });
        let mut f = crate::model::Frame { x_mm: 10.0, y_mm: 20.0, w_mm: 60.0, h_mm: 8.0, rotation_deg: 30.0 };
        grow_to_hit_height(&mut f, &line(ShapeKind::Arrow, 3.0, large));
        assert_eq!((f.h_mm, f.y_mm + f.h_mm / 2.0), (18.0, 24.0));
        let mut g = crate::model::Frame { x_mm: 10.0, y_mm: 20.0, w_mm: 60.0, h_mm: 8.0, rotation_deg: 0.0 };
        grow_to_hit_height(&mut g, &line(ShapeKind::Line, 3.0, None));
        assert_eq!(g.h_mm, 8.0);
    }

    #[test]
    fn head_width_matches_todays_arrow() {
        assert_eq!(head_width_mm(&line(ShapeKind::Arrow, 1.0, None)), 4.0);
        assert_eq!(head_width_mm(&line(ShapeKind::Arrow, 2.0, None)), 8.0);
        let small = Some(LineEnds { start: LineEnd::None, end: LineEnd::Arrow, size: HeadSize::Small });
        assert!((head_width_mm(&line(ShapeKind::Arrow, 1.0, small)) - 2.8).abs() < 1e-9);
        let large = Some(LineEnds { start: LineEnd::None, end: LineEnd::Arrow, size: HeadSize::Large });
        assert!((head_width_mm(&line(ShapeKind::Arrow, 2.0, large)) - 12.0).abs() < 1e-9);
    }

    #[test]
    fn old_shape_serializes_without_ends() {
        let json = serde_json::to_string(&line(ShapeKind::Arrow, 1.0, None)).unwrap();
        assert!(!json.contains("ends"), "{json}");
        let back: ShapeProps = serde_json::from_str(r#"{"shape":"Arrow","fill":null,"stroke":null,"corner_radius_mm":0}"#).unwrap();
        assert_eq!(back.ends, None);
    }

    #[test]
    fn screen_uses_the_same_head_factors() {
        let ts = include_str!("../../../ui/src/lib/shapePaths.ts");
        for (size, factor) in HEAD_FACTORS {
            let line = format!("  {size:?}: {factor},");
            assert!(ts.contains(&line), "taille {size:?} différente à l'écran : {line}");
        }
        assert!(ts.contains(&format!("export const DOT_FACTOR = {DOT_FACTOR};")), "rond et carré différents à l'écran");
    }

    /// Mêmes valeurs que `same tails as the export` (`ui/src/lib/tail.test.ts`) : l'écran et l'export dessinent la même
    /// bulle, au caractère près.
    #[test]
    fn screen_and_export_draw_the_same_tails() {
        assert_eq!(bubble_path(70.0, 35.0, [9.0 / 70.0, 45.0 / 35.0]), "M 125 0 L 875 0 C 944 0 1000 112 1000 250 L 1000 750 C 1000 888 944 1000 875 1000 L 401 1000 C 401 1143 265 1214 129 1286 C 175 1211 244 1100 244 1000 L 125 1000 C 56 1000 0 888 0 750 L 0 250 C 0 112 56 0 125 0 Z");
        assert_eq!(bubble_path(52.3, 41.7, [1.37, 0.21]), "M 172 0 L 828 0 C 923 0 1000 97 1000 216 L 1000 243 C 1185 243 1278 227 1370 210 C 1274 329 1130 507 1000 507 L 1000 784 C 1000 903 923 1000 828 1000 L 172 1000 C 77 1000 0 903 0 784 L 0 216 C 0 97 77 0 172 0 Z");
        assert_eq!(bubble_path(30.0, 60.0, [0.4, -0.8]), "M 250 0 L 296 0 C 296 -400 348 -600 400 -800 C 498 -592 646 -280 646 0 L 750 0 C 888 0 1000 56 1000 125 L 1000 875 C 1000 944 888 1000 750 1000 L 250 1000 C 112 1000 0 944 0 875 L 0 125 C 0 56 112 0 250 0 Z");
        let cloud = path(ShapeKind::Cloud).unwrap();
        assert_eq!(thought_path(70.0, 40.0, [0.15, 1.45]), format!("{cloud} M 265 1035 C 297 1035 324 1081 324 1137 C 324 1193 297 1239 265 1239 C 233 1239 207 1193 207 1137 C 207 1081 233 1035 265 1035 Z M 199 1246 C 221 1246 240 1278 240 1317 C 240 1357 221 1389 199 1389 C 176 1389 158 1357 158 1317 C 158 1278 176 1246 199 1246 Z M 150 1404 C 164 1404 176 1425 176 1450 C 176 1475 164 1496 150 1496 C 136 1496 124 1475 124 1450 C 124 1425 136 1404 150 1404 Z"));
        assert_eq!(thought_path(55.5, 33.3, [-0.7, 0.3]), format!("{cloud} M -115 278 C -75 278 -43 331 -43 398 C -43 464 -75 518 -115 518 C -154 518 -187 464 -187 398 C -187 331 -154 278 -115 278 Z M -427 261 C -399 261 -377 299 -377 345 C -377 392 -399 429 -427 429 C -455 429 -477 392 -477 345 C -477 299 -455 261 -427 261 Z M -700 246 C -682 246 -668 270 -668 300 C -668 330 -682 354 -700 354 C -718 354 -732 330 -732 300 C -732 270 -718 246 -700 246 Z"));
    }

    #[test]
    fn a_tail_is_read_only_on_a_bubble_or_a_cloud_and_when_sane() {
        let mut s = line(ShapeKind::Arrow, 1.0, None);
        s.tail = Some([0.2, 1.3]);
        assert_eq!(tail_of(&s), None);
        s.shape = ShapeKind::Bubble;
        assert_eq!(tail_of(&s), Some([0.2, 1.3]));
        s.tail = Some([f64::NAN, 1.0]);
        assert_eq!(tail_of(&s), None);
        s.tail = Some([0.0, 1e6]);
        assert_eq!(tail_of(&s), None);
        let f = crate::model::Frame { x_mm: 0.0, y_mm: 0.0, w_mm: 70.0, h_mm: 45.0, rotation_deg: 0.0 };
        s.tail = None;
        assert_eq!(path_of(&f, &s).as_deref(), path(ShapeKind::Bubble));
    }

    #[test]
    fn screen_uses_the_same_paths() {
        let ts = include_str!("../../../ui/src/lib/shapePaths.ts");
        for kind in [ShapeKind::Star, ShapeKind::Heart, ShapeKind::Bubble, ShapeKind::Cloud] {
            let line = format!("{kind:?}: \"{}\"", path(kind).unwrap());
            assert!(ts.contains(&line), "tracé de {kind:?} différent à l'écran");
        }
    }
}
