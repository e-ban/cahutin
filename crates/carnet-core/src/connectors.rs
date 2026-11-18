//! Connecteurs : un bout de trait, de flèche ou de chemin accroché à un point d'un élément de la page (milieu d'un
//! côté ou centre de son cadre) le suit. Une seule règle, appliquée après chaque modification (`follow_connectors`) :
//! d'abord décrocher un trait déplacé seul, puis amener chaque bout encore accroché sur son point. Même règle que
//! `followConnectors` (ui/src/lib/connectors.ts).

use crate::model::{AnchorPoint, Element, ElementKind, Frame, Page};
use crate::shapes::{line_frame, path_frame, path_page_points, valid_points};

/// Un bout qui bouge de moins que ça (arrondi d'un geste, d'un calcul) ne compte pas comme déplacé : il ne se
/// décroche pas, il revient sur son point. Même valeur que `DETACH_TOLERANCE_MM` (connectors.ts).
pub const DETACH_TOLERANCE_MM: f64 = 0.05;

/// Le point d'accroche d'un cadre, en mm de page : milieu d'un côté ou centre, rotation comprise (miroir ignoré).
pub fn anchor_point(f: &Frame, at: AnchorPoint) -> (f64, f64) {
    let (fx, fy) = match at {
        AnchorPoint::Top => (0.5, 0.0),
        AnchorPoint::Right => (1.0, 0.5),
        AnchorPoint::Bottom => (0.5, 1.0),
        AnchorPoint::Left => (0.0, 0.5),
        AnchorPoint::Center => (0.5, 0.5),
    };
    let (cx, cy) = (f.x_mm + f.w_mm / 2.0, f.y_mm + f.h_mm / 2.0);
    let (dx, dy) = ((fx - 0.5) * f.w_mm, (fy - 0.5) * f.h_mm);
    let (sin, cos) = f.rotation_deg.to_radians().sin_cos();
    (cx + dx * cos - dy * sin, cy + dx * sin + dy * cos)
}

/// Les points visibles d'un trait, d'une flèche ou d'un chemin, en mm de page : ses deux bouts, ou tous ses points.
pub fn line_points(e: &Element) -> Option<Vec<(f64, f64)>> {
    let ElementKind::Shape(s) = &e.kind else { return None };
    if !crate::shapes::is_stroke_only(s.shape) {
        return None;
    }
    if let Some(p) = path_page_points(&e.frame, s, e.flip_h, e.flip_v) {
        return Some(p);
    }
    let f = &e.frame;
    let (cx, cy) = (f.x_mm + f.w_mm / 2.0, f.y_mm + f.h_mm / 2.0);
    let (sin, cos) = f.rotation_deg.to_radians().sin_cos();
    let at = |dx: f64| (cx + dx * cos, cy + dx * sin);
    let (a, b) = (at(-f.w_mm / 2.0), at(f.w_mm / 2.0));
    Some(if e.flip_h { vec![b, a] } else { vec![a, b] })
}

/// Les milieux de côtés qui se font face entre deux cadres : la paire la plus proche.
pub fn facing_anchors(a: &Frame, b: &Frame) -> (AnchorPoint, AnchorPoint) {
    const SIDES: [AnchorPoint; 4] = [AnchorPoint::Top, AnchorPoint::Right, AnchorPoint::Bottom, AnchorPoint::Left];
    let mut best = (AnchorPoint::Right, AnchorPoint::Left, f64::INFINITY);
    for &p in &SIDES {
        for &q in &SIDES {
            let ((x1, y1), (x2, y2)) = (anchor_point(a, p), anchor_point(b, q));
            let d = (x2 - x1).hypot(y2 - y1);
            if d < best.2 - 1e-9 {
                best = (p, q, d);
            }
        }
    }
    (best.0, best.1)
}

/// La règle des connecteurs, après une modification de la page (`before` : la page d'avant, s'il y en a une).
/// 1. Décrocher : un bout accroché qui a bougé alors que son élément n'a pas bougé perd son accroche (on a déplacé la
///    flèche seule, ou tiré ce bout) ; un chemin fermé n'a pas d'accroche.
/// 2. Suivre : chaque bout encore accroché va sur le point de son élément ; un élément absent le décroche. Les coudes
///    d'un chemin restent ; le cadre est recalculé (`line_frame`, `path_frame`). Un trait déjà en place ne change pas.
pub fn follow_connectors(before: Option<&Page>, after: &mut Page) {
    if !after.elements.iter().any(|e| matches!(&e.kind, ElementKind::Shape(s) if s.attach.is_some())) {
        return;
    }
    let frames: std::collections::HashMap<String, Frame> = after.elements.iter().map(|e| (e.id.clone(), e.frame)).collect();
    let old = |id: &str| before.and_then(|b| b.elements.iter().find(|e| e.id == id));
    for i in 0..after.elements.len() {
        let e = &after.elements[i];
        let ElementKind::Shape(s) = &e.kind else { continue };
        let Some(attach) = s.attach.clone() else { continue };
        let Some(mut pts) = line_points(e) else { continue };
        let n = pts.len();
        let was = old(&e.id).and_then(line_points);
        let was_attach = old(&e.id).and_then(|o| match &o.kind {
            ElementKind::Shape(os) => os.attach.clone(),
            _ => None,
        });
        // Un bout a bougé seul : il n'est plus où il était, alors que son élément est à la même place qu'avant.
        let keep = |a: &Option<crate::model::Anchor>, k: usize| {
            a.clone().filter(|a| {
                let Some(now) = frames.get(&a.element) else { return false };
                let tip_moved = was.as_ref().is_some_and(|w| {
                    let j = if k == 0 { 0 } else { w.len() - 1 };
                    (w[j].0 - pts[k].0).hypot(w[j].1 - pts[k].1) > DETACH_TOLERANCE_MM
                });
                // Une accroche toute neuve (on vient de tirer le bout dessus) n'est jamais défaite.
                let same_before = was_attach.as_ref().and_then(|w| if k == 0 { w.start.as_ref() } else { w.end.as_ref() }) == Some(a);
                !(tip_moved && same_before && old(&a.element).is_some_and(|o| o.frame == *now)) && !s.closed
            })
        };
        let (start, end) = (keep(&attach.start, 0), keep(&attach.end, n - 1));
        let mut changed = false;
        for (a, k) in [(&start, 0), (&end, n - 1)] {
            if let Some(a) = a {
                let p = anchor_point(&frames[&a.element], a.at);
                if (p.0 - pts[k].0).abs() > 1e-9 || (p.1 - pts[k].1).abs() > 1e-9 {
                    pts[k] = p;
                    changed = true;
                }
            }
        }
        let new_attach = (start.is_some() || end.is_some()).then(|| crate::model::Attach { start, end });
        if !changed && new_attach.as_ref() == Some(&attach) {
            continue;
        }
        let e = &mut after.elements[i];
        let ElementKind::Shape(s) = &mut e.kind else { continue };
        s.attach = new_attach;
        if !changed {
            continue;
        }
        if s.points.as_deref().is_some_and(valid_points) {
            let (f, fr) = path_frame(&pts, s);
            e.frame = f;
            s.points = Some(fr);
        } else {
            e.frame = line_frame(pts[0], pts[n - 1], s);
        }
        e.flip_h = false;
        e.flip_v = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Anchor, AnchorPoint, Attach, Element, ElementKind, Frame, Page, ShapeKind, ShapeProps, Stroke, A4_PORTRAIT};

    fn frame(x: f64, y: f64, w: f64, h: f64, r: f64) -> Frame {
        Frame { x_mm: x, y_mm: y, w_mm: w, h_mm: h, rotation_deg: r }
    }
    fn shape(id: &str, kind: ShapeKind, f: Frame) -> Element {
        Element {
            id: id.into(),
            frame: f,
            locked: false,
            zone: false,
            group: None,
            subgroups: Vec::new(),
            flip_h: false,
            flip_v: false,
            kind: ElementKind::Shape(ShapeProps { shape: kind, fill: None, stroke: Some(Stroke { width_mm: 1.0, color: "#333333".into(), style: None }), corner_radius_mm: 0.0, path: None, graphisme: None, ends: None, points: None, curve: false, closed: false, attach: None }),
        }
    }
    fn anchor(element: &str, at: AnchorPoint) -> Option<Anchor> {
        Some(Anchor { element: element.into(), at })
    }
    /// Un rectangle « r » et une flèche « a » de (10, 50) à (60, 50), la fin accrochée au milieu gauche de « r ».
    fn page() -> Page {
        let mut p = Page::new(A4_PORTRAIT);
        let r = shape("r", ShapeKind::Rect, frame(60.0, 40.0, 40.0, 20.0, 0.0));
        let mut a = shape("a", ShapeKind::Arrow, crate::shapes::line_frame((10.0, 50.0), (60.0, 50.0), &props_of(&r)));
        if let ElementKind::Shape(s) = &mut a.kind {
            s.attach = Some(Attach { start: None, end: anchor("r", AnchorPoint::Left) });
        }
        p.elements = vec![r, a];
        p
    }
    fn props_of(e: &Element) -> ShapeProps {
        let ElementKind::Shape(s) = &e.kind else { panic!() };
        s.clone()
    }
    fn get<'a>(p: &'a Page, id: &str) -> &'a Element {
        p.elements.iter().find(|e| e.id == id).unwrap()
    }
    fn ends(p: &Page, id: &str) -> Vec<(f64, f64)> {
        line_points(get(p, id)).unwrap()
    }
    fn close(a: (f64, f64), b: (f64, f64)) -> bool {
        (a.0 - b.0).abs() < 1e-9 && (a.1 - b.1).abs() < 1e-9
    }
    fn moved(p: &Page, id: &str, f: impl Fn(&mut Element)) -> Page {
        let mut q = p.clone();
        f(q.elements.iter_mut().find(|e| e.id == id).unwrap());
        q
    }

    #[test]
    fn anchor_points_of_a_rotated_frame() {
        let f = frame(0.0, 0.0, 40.0, 20.0, 90.0);
        let (cx, cy) = (20.0, 10.0);
        assert!(close(anchor_point(&f, AnchorPoint::Center), (cx, cy)));
        assert!(close(anchor_point(&f, AnchorPoint::Right), (cx, cy + 20.0)), "{:?}", anchor_point(&f, AnchorPoint::Right));
        assert!(close(anchor_point(&f, AnchorPoint::Top), (cx + 10.0, cy)), "{:?}", anchor_point(&f, AnchorPoint::Top));
    }

    #[test]
    fn end_follows_its_element() {
        let before = page();
        let mut after = moved(&before, "r", |e| e.frame.x_mm += 30.0);
        follow_connectors(Some(&before), &mut after);
        let p = ends(&after, "a");
        assert!(close(p[0], (10.0, 50.0)) && close(p[1], (90.0, 50.0)), "{p:?}");
        // Valeurs partagées avec connectors.test.ts (parité).
        let f = get(&after, "a").frame;
        let r = |v: f64| (v * 1e6).round() / 1e6 + 0.0;
        assert_eq!([r(f.x_mm), r(f.y_mm), r(f.w_mm), r(f.h_mm), r(f.rotation_deg)], PARITY);
    }

    /// Le cadre de la flèche suivie dans `end_follows_its_element` (copié dans connectors.test.ts).
    const PARITY: [f64; 5] = [10.0, 48.0, 80.0, 4.0, 0.0];

    #[test]
    fn path_elbows_stay() {
        let mut before = page();
        let elbows = [(10.0, 50.0), (20.0, 80.0), (40.0, 30.0), (60.0, 50.0)];
        if let Some(a) = before.elements.iter_mut().find(|e| e.id == "a")
            && let ElementKind::Shape(s) = &mut a.kind
        {
            let (f, pts) = crate::shapes::path_frame(&elbows, s);
            s.points = Some(pts);
            a.frame = f;
        }
        let mut after = moved(&before, "r", |e| e.frame.rotation_deg = 90.0);
        follow_connectors(Some(&before), &mut after);
        let p = ends(&after, "a");
        for i in 0..3 {
            assert!((p[i].0 - elbows[i].0).abs() < 1e-9 && (p[i].1 - elbows[i].1).abs() < 1e-9, "{p:?}");
        }
        assert!(close(p[3], anchor_point(&get(&after, "r").frame, AnchorPoint::Left)), "{p:?}");
        assert!(p.iter().all(|q| q.0.is_finite() && q.1.is_finite()));
    }

    #[test]
    fn both_ends_on_one_element() {
        let mut before = page();
        if let Some(a) = before.elements.iter_mut().find(|e| e.id == "a")
            && let ElementKind::Shape(s) = &mut a.kind
        {
            s.attach = Some(Attach { start: anchor("r", AnchorPoint::Top), end: anchor("r", AnchorPoint::Bottom) });
        }
        let mut after = before.clone();
        follow_connectors(None, &mut after);
        let grown = moved(&after, "r", |e| e.frame.h_mm = 50.0);
        let mut grown2 = grown.clone();
        follow_connectors(Some(&after), &mut grown2);
        let p = ends(&grown2, "a");
        assert!(close(p[0], (80.0, 40.0)) && close(p[1], (80.0, 90.0)), "{p:?}");
    }

    #[test]
    fn moving_the_line_alone_detaches() {
        let before = page();
        let mut after = moved(&before, "a", |e| e.frame.y_mm += 10.0);
        follow_connectors(Some(&before), &mut after);
        assert_eq!(props_of(get(&after, "a")).attach, None);
        assert!(close(ends(&after, "a")[1], (60.0, 60.0)));
    }

    /// Un arrondi minuscule (moins de 0,05 mm) ne décroche pas : le bout revient sur son point.
    #[test]
    fn a_tiny_rounding_does_not_detach() {
        let before = page();
        let mut after = moved(&before, "a", |e| e.frame.rotation_deg += 0.04);
        follow_connectors(Some(&before), &mut after);
        assert!(props_of(get(&after, "a")).attach.is_some());
        assert!(close(ends(&after, "a")[1], (60.0, 50.0)));
    }

    #[test]
    fn moving_both_keeps_the_attachment() {
        let before = page();
        let mut after = moved(&moved(&before, "a", |e| e.frame.x_mm += 5.0), "r", |e| e.frame.x_mm += 5.0);
        follow_connectors(Some(&before), &mut after);
        assert!(props_of(get(&after, "a")).attach.is_some());
        assert!(close(ends(&after, "a")[1], (65.0, 50.0)));
    }

    /// Tirer un coude d'un chemin accroché : ses bouts n'ont pas bougé, il reste accroché.
    #[test]
    fn moving_an_elbow_keeps_the_attachment() {
        let mut before = page();
        let elbows = [(10.0, 50.0), (30.0, 80.0), (60.0, 50.0)];
        if let Some(a) = before.elements.iter_mut().find(|e| e.id == "a")
            && let ElementKind::Shape(s) = &mut a.kind
        {
            let (f, pts) = crate::shapes::path_frame(&elbows, s);
            s.points = Some(pts);
            a.frame = f;
        }
        let mut after = before.clone();
        if let Some(a) = after.elements.iter_mut().find(|e| e.id == "a")
            && let ElementKind::Shape(s) = &mut a.kind
        {
            let (f, pts) = crate::shapes::path_frame(&[(10.0, 50.0), (35.0, 95.0), (60.0, 50.0)], s);
            s.points = Some(pts);
            a.frame = f;
        }
        follow_connectors(Some(&before), &mut after);
        assert!(props_of(get(&after, "a")).attach.is_some());
    }

    /// Un chemin fermé n'a pas de bouts : pas d'accroche.
    #[test]
    fn a_closed_path_has_no_attachment() {
        let before = page();
        let mut after = before.clone();
        if let Some(a) = after.elements.iter_mut().find(|e| e.id == "a")
            && let ElementKind::Shape(s) = &mut a.kind
        {
            let (f, pts) = crate::shapes::path_frame(&[(10.0, 50.0), (35.0, 95.0), (60.0, 50.0)], s);
            s.points = Some(pts);
            s.closed = true;
            a.frame = f;
        }
        follow_connectors(Some(&before), &mut after);
        assert_eq!(props_of(get(&after, "a")).attach, None);
    }

    /// Un bout qu'on vient d'accrocher (son accroche est nouvelle) reste accroché, même s'il a bougé.
    #[test]
    fn a_fresh_attachment_is_kept() {
        let before = page();
        // Le bout a été tiré jusqu'au milieu du haut de « r », et s'y accroche.
        let mut after = moved(&before, "a", |e| {
            if let ElementKind::Shape(s) = &mut e.kind {
                s.attach = Some(Attach { start: None, end: anchor("r", AnchorPoint::Top) });
                e.frame = crate::shapes::line_frame((10.0, 50.0), (80.2, 40.1), s);
            }
        });
        follow_connectors(Some(&before), &mut after);
        assert!(props_of(get(&after, "a")).attach.is_some());
        assert!(close(ends(&after, "a")[1], (80.0, 40.0)));
    }

    #[test]
    fn missing_element_drops_the_anchor() {
        let before = page();
        let mut after = before.clone();
        after.elements.retain(|e| e.id != "r");
        follow_connectors(Some(&before), &mut after);
        assert_eq!(props_of(get(&after, "a")).attach, None);
        assert!(close(ends(&after, "a")[1], (60.0, 50.0)));
    }

    #[test]
    fn facing_anchors_pick_facing_sides() {
        let a = frame(0.0, 0.0, 20.0, 10.0, 0.0);
        let b = frame(100.0, 0.0, 20.0, 10.0, 0.0);
        assert_eq!(facing_anchors(&a, &b), (AnchorPoint::Right, AnchorPoint::Left));
        let below = frame(0.0, 100.0, 20.0, 10.0, 0.0);
        assert_eq!(facing_anchors(&a, &below), (AnchorPoint::Bottom, AnchorPoint::Top));
    }

    #[test]
    fn attach_absent_by_default() {
        let s: ShapeProps = serde_json::from_str(r##"{"shape":"Line","fill":null,"stroke":null,"corner_radius_mm":0}"##).unwrap();
        assert_eq!(s.attach, None);
        assert!(!serde_json::to_string(&s).unwrap().contains("attach"));
    }

    #[test]
    fn written_version_is_16_with_attach() {
        let mut d = crate::model::Document::new("v");
        d.pages[0] = page();
        assert_eq!(crate::native::written_version(&d), 16);
        assert_eq!(crate::model::FORMAT_VERSION, 16);
    }
}
