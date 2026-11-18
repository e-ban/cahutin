//! Banc de fidélité : l'ODG exporté est rendu par LibreOffice, puis le PDF obtenu est inspecté avec poppler.
//! Lancer avec `cargo test -p carnet-core --test odg_libreoffice -- --ignored` (soffice, pdftotext, pdffonts,
//! pdfinfo et pdfimages doivent être dans le PATH). Les tests de défauts connus, mis de côté, ne vérifient rien
//! sans `BANC_DEFAUTS_CONNUS=1`.

use carnet_core::fonts::{FontSet, add_user_font};
use carnet_core::odg::export_odg_to_path;
use carnet_core::shapes;
use carnet_core::photo::{AssetStore, import_bytes};
use carnet_core::*;
use image::{ImageFormat, Rgb, RgbImage};
use serde::Deserialize;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::process::Command;

const PT_PER_MM: f64 = 72.0 / 25.4;
const TOLERANCE_PT: f64 = 0.5;
const ORIGIN_MM: f64 = 20.0;

#[derive(Deserialize)]
struct Case {
    id: String,
    font: String,
    size_pt: f64,
    w_mm: f64,
    h_mm: f64,
    text: String,
    expected_lines: Option<usize>,
}

fn cases() -> Vec<Case> {
    serde_json::from_str(include_str!("../../../fixtures/text_fidelity.json")).unwrap()
}

fn text_element(c: &Case) -> Element {
    let mut style = TextStyle::default_zone();
    style.font = c.font.clone();
    style.size_pt = c.size_pt;
    Element {
        id: new_id(),
        frame: Frame { x_mm: ORIGIN_MM, y_mm: ORIGIN_MM, w_mm: c.w_mm, h_mm: c.h_mm, rotation_deg: 0.0 },
        locked: false,
        zone: true,
        group: None,
        kind: ElementKind::Text(TextProps { content: RichText::plain(&c.text), style, prompt: None, frame: None }),
        subgroups: Vec::new(),
        flip_h: false, flip_v: false,
    }
}

/// Un cas par page : les mots de la page N appartiennent au cas N.
fn fidelity_document() -> Document {
    let mut doc = Document::new("banc");
    doc.pages.clear();
    for c in cases() {
        let mut page = Page::new(A4_PORTRAIT);
        page.elements.push(text_element(&c));
        doc.pages.push(page);
    }
    doc
}

fn run(cmd: &mut Command) -> String {
    let out = cmd.output().unwrap_or_else(|e| panic!("{cmd:?} : {e}"));
    assert!(out.status.success(), "{cmd:?} : {}", String::from_utf8_lossy(&out.stderr));
    String::from_utf8_lossy(&out.stdout).into_owned()
}

/// Exporte, puis convertit en PDF avec un profil LibreOffice jetable.
fn render(doc: &Document, store: &AssetStore) -> (tempfile::TempDir, PathBuf) {
    render_with(doc, store, &FontSet::builtin())
}

fn render_with(doc: &Document, store: &AssetStore, fonts: &FontSet) -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let odg = dir.path().join("banc.odg");
    export_odg_to_path(doc, store, fonts, &odg).unwrap();
    let profile = format!("-env:UserInstallation=file://{}", dir.path().join("profil").display());
    run(Command::new("soffice")
        .arg(profile)
        .args(["--headless", "--convert-to", "pdf", "--outdir"])
        .arg(dir.path())
        .arg(&odg));
    let pdf = dir.path().join("banc.pdf");
    assert!(pdf.exists());
    (dir, pdf)
}

struct Word {
    page: usize,
    text: String,
    x_min: f64,
    y_min: f64,
    x_max: f64,
    y_max: f64,
}

fn words(pdf: &Path) -> Vec<Word> {
    let html = run(Command::new("pdftotext").arg("-bbox-layout").arg(pdf).arg("-"));
    let opts = roxmltree::ParsingOptions { allow_dtd: true, ..Default::default() };
    let doc = roxmltree::Document::parse_with_options(&html, opts).unwrap();
    let mut out = vec![];
    for (page, p) in doc.descendants().filter(|n| n.has_tag_name("page")).enumerate() {
        for w in p.descendants().filter(|n| n.has_tag_name("word")) {
            let f = |name: &str| w.attribute(name).unwrap().parse::<f64>().unwrap();
            out.push(Word {
                page,
                text: w.text().unwrap_or("").to_string(),
                x_min: f("xMin"),
                y_min: f("yMin"),
                x_max: f("xMax"),
                y_max: f("yMax"),
            });
        }
    }
    out
}

fn photo_document(store: &mut AssetStore, n: usize) -> Document {
    let mut doc = Document::new("photos");
    for i in 0..n {
        let mut bytes = Cursor::new(Vec::new());
        RgbImage::from_pixel(800, 600, Rgb([30 * i as u8, 120, 200])).write_to(&mut bytes, ImageFormat::Jpeg).unwrap();
        let a = import_bytes(&bytes.into_inner()).unwrap();
        doc.pages[0].elements.push(Element {
            id: new_id(),
            frame: Frame { x_mm: 20.0, y_mm: 20.0 + 90.0 * i as f64, w_mm: 120.0, h_mm: 80.0, rotation_deg: 0.0 },
            locked: false,
            zone: true,
            group: None,
            kind: ElementKind::Photo(PhotoProps {
                asset: Some(a.meta.id.clone()),
                crop: Crop::COVER,
                corner_radius_mm: if i == 0 { 0.0 } else { 4.0 },
                border: None,
                shadow: false,
                mirrored: false,
            }),
            subgroups: Vec::new(),
            flip_h: false, flip_v: false,
        });
        doc.assets.insert(a.meta.id.clone(), a.meta.clone());
        store.insert(a).unwrap();
    }
    doc
}

#[test]
#[ignore]
fn page_count_is_preserved() {
    let mut doc = Document::new("trois pages");
    doc.pages.push(Page::new(A4_PORTRAIT));
    doc.pages.push(Page::new(A4_PORTRAIT));
    let (_dir, pdf) = render(&doc, &AssetStore::default());
    let info = run(Command::new("pdfinfo").arg(&pdf));
    assert!(info.lines().any(|l| l.starts_with("Pages:") && l.trim_end().ends_with(" 3")), "{info}");
}

#[test]
#[ignore]
fn text_stays_inside_frames() {
    let (_dir, pdf) = render(&fidelity_document(), &AssetStore::default());
    let words = words(&pdf);
    for (page, c) in cases().iter().enumerate() {
        let (x0, y0) = (ORIGIN_MM * PT_PER_MM - TOLERANCE_PT, ORIGIN_MM * PT_PER_MM - TOLERANCE_PT);
        let (x1, y1) = ((ORIGIN_MM + c.w_mm) * PT_PER_MM + TOLERANCE_PT, (ORIGIN_MM + c.h_mm) * PT_PER_MM + TOLERANCE_PT);
        let on_page: Vec<&Word> = words.iter().filter(|w| w.page == page).collect();
        for w in &on_page {
            assert!(
                w.x_min >= x0 && w.y_min >= y0 && w.x_max <= x1 && w.y_max <= y1,
                "{} : « {} » sort du cadre ({:.1},{:.1})-({:.1},{:.1}) hors de ({x0:.1},{y0:.1})-({x1:.1},{y1:.1})",
                c.id,
                w.text,
                w.x_min,
                w.y_min,
                w.x_max,
                w.y_max
            );
        }
        // pdftotext coupe parfois les mots cursifs liés ; on compare donc sans les espaces.
        let got: String = on_page.iter().map(|w| w.text.as_str()).collect();
        let want: String = c.text.split_whitespace().collect();
        assert_eq!(got, want, "{} : texte rendu différent", c.id);
    }
}

#[test]
#[ignore]
fn line_counts_match_editor() {
    let (_dir, pdf) = render(&fidelity_document(), &AssetStore::default());
    let words = words(&pdf);
    for (page, c) in cases().iter().enumerate() {
        let Some(expected) = c.expected_lines else { continue };
        let mut tops: Vec<i64> = words.iter().filter(|w| w.page == page).map(|w| w.y_min.round() as i64).collect();
        tops.sort();
        tops.dedup_by(|a, b| (*a - *b).abs() <= 1);
        assert_eq!(tops.len(), expected, "{} : nombre de lignes", c.id);
    }
}

#[test]
#[ignore]
fn fonts_are_embedded_in_rendering() {
    let (_dir, pdf) = render(&fidelity_document(), &AssetStore::default());
    let fonts = run(Command::new("pdffonts").arg(&pdf));
    assert!(fonts.contains("Andika"), "{fonts}");
    assert!(fonts.contains("Playwrite"), "{fonts}");
}

/// « Carnet Test » n'est installée nulle part : LibreOffice ne peut l'avoir que par l'ODG.
#[test]
#[ignore]
fn user_font_is_embedded_in_rendering() {
    let fonts_dir = tempfile::tempdir().unwrap();
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/CarnetTest-Regular.ttf");
    add_user_font(fonts_dir.path(), &fixture).unwrap();
    let mut doc = Document::new("police");
    let mut style = TextStyle::default_zone();
    style.font = "Carnet Test".into();
    doc.pages[0].elements.push(Element {
        id: new_id(),
        frame: Frame { x_mm: ORIGIN_MM, y_mm: ORIGIN_MM, w_mm: 120.0, h_mm: 20.0, rotation_deg: 0.0 },
        locked: false,
        zone: false,
        group: None,
        kind: ElementKind::Text(TextProps { content: RichText::plain("Police ajoutée"), style, prompt: None, frame: None }),
        subgroups: Vec::new(),
        flip_h: false, flip_v: false,
    });
    let (_dir, pdf) = render_with(&doc, &AssetStore::default(), &FontSet::with_user_dir(fonts_dir.path()));
    let fonts = run(Command::new("pdffonts").arg(&pdf));
    assert!(fonts.contains("CarnetTest"), "{fonts}");
}

/// Police « du poste » (dossier jouant ce rôle), absente de LibreOffice : elle n'arrive que par l'ODG.
#[test]
#[ignore]
fn system_font_is_embedded_in_rendering() {
    let sys = tempfile::tempdir().unwrap();
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/CarnetTest-Regular.ttf");
    std::fs::copy(fixture, sys.path().join("carnettest.ttf")).unwrap();
    let fonts = FontSet::builtin().with_system(&carnet_core::fonts::scan_font_dirs(&[sys.path()]));
    let mut doc = Document::new("police du poste");
    let mut style = TextStyle::default_zone();
    style.font = "Carnet Test".into();
    doc.pages[0].elements.push(Element {
        id: new_id(),
        frame: Frame { x_mm: ORIGIN_MM, y_mm: ORIGIN_MM, w_mm: 120.0, h_mm: 20.0, rotation_deg: 0.0 },
        locked: false,
        zone: false,
        group: None,
        kind: ElementKind::Text(TextProps { content: RichText::plain("Police du poste"), style, prompt: None, frame: None }),
        subgroups: Vec::new(),
        flip_h: false, flip_v: false,
    });
    let (_dir, pdf) = render_with(&doc, &AssetStore::default(), &fonts);
    let out = run(Command::new("pdffonts").arg(&pdf));
    assert!(out.contains("CarnetTest"), "{out}");
}

#[test]
#[ignore]
fn photos_are_rendered() {
    let mut store = AssetStore::default();
    let doc = photo_document(&mut store, 2);
    let (_dir, pdf) = render(&doc, &store);
    let list = run(Command::new("pdfimages").arg("-list").arg(&pdf));
    assert_eq!(list.lines().skip(2).filter(|l| !l.trim().is_empty()).count(), 2, "{list}");
}


/// Surface du tracé (courbes aplaties) rapportée à son carré de `VIEWBOX` unités.
fn path_area_ratio(path: &str) -> f64 {
    let toks: Vec<&str> = path.split_whitespace().collect();
    let num = |i: usize| toks[i].parse::<f64>().unwrap();
    let (mut pts, mut cur, mut i) = (Vec::new(), (0.0, 0.0), 0);
    while i < toks.len() {
        match toks[i] {
            "M" | "L" => {
                cur = (num(i + 1), num(i + 2));
                pts.push(cur);
                i += 3;
            }
            "C" => {
                let (c1, c2, p) = ((num(i + 1), num(i + 2)), (num(i + 3), num(i + 4)), (num(i + 5), num(i + 6)));
                for k in 1..=32 {
                    let t = k as f64 / 32.0;
                    let u = 1.0 - t;
                    let b = |a: f64, b: f64, c: f64, d: f64| u * u * u * a + 3.0 * u * u * t * b + 3.0 * u * t * t * c + t * t * t * d;
                    pts.push((b(cur.0, c1.0, c2.0, p.0), b(cur.1, c1.1, c2.1, p.1)));
                }
                cur = p;
                i += 7;
            }
            _ => i += 1,
        }
    }
    let twice: f64 = pts.iter().zip(pts.iter().cycle().skip(1)).map(|(a, b)| a.0 * b.1 - b.0 * a.1).sum();
    twice.abs() / 2.0 / (shapes::VIEWBOX * shapes::VIEWBOX)
}

/// Page rendue en niveaux de gris par poppler : (largeur, hauteur, pixels).
fn gray_page(pdf: &Path, page: usize, dpi: u32) -> (usize, usize, Vec<u8>) {
    let dir = pdf.parent().unwrap();
    let prefix = dir.join(format!("p{page}"));
    run(Command::new("pdftoppm").args(["-gray", "-r", &dpi.to_string(), "-f", &page.to_string(), "-l", &page.to_string(), "-singlefile"]).arg(pdf).arg(&prefix));
    let data = std::fs::read(prefix.with_extension("pgm")).unwrap();
    let mut fields = Vec::new();
    let mut pos = 0;
    while fields.len() < 4 {
        while data[pos].is_ascii_whitespace() {
            pos += 1;
        }
        let start = pos;
        while !data[pos].is_ascii_whitespace() {
            pos += 1;
        }
        fields.push(String::from_utf8_lossy(&data[start..pos]).into_owned());
    }
    let (w, h): (usize, usize) = (fields[1].parse().unwrap(), fields[2].parse().unwrap());
    (w, h, data[pos + 1..].to_vec())
}

#[test]
#[ignore]
fn drawn_shapes_keep_their_geometry() {
    const SIDE_MM: f64 = 100.0;
    const DPI: u32 = 72;
    let kinds = [ShapeKind::Star, ShapeKind::Heart, ShapeKind::Bubble, ShapeKind::Cloud];
    let mut doc = Document::new("formes");
    doc.pages.clear();
    for kind in kinds {
        let mut page = Page::new(A4_PORTRAIT);
        page.elements.push(Element {
            id: new_id(),
            frame: Frame { x_mm: ORIGIN_MM, y_mm: ORIGIN_MM, w_mm: SIDE_MM, h_mm: SIDE_MM, rotation_deg: 0.0 },
            locked: false,
            zone: false,
            group: None,
            kind: ElementKind::Shape(ShapeProps { shape: kind, fill: Some("#000000".into()), stroke: None, corner_radius_mm: 0.0, path: None, graphisme: None, ends: None, points: None, curve: false }),
            subgroups: Vec::new(),
            flip_h: false, flip_v: false,
        });
        doc.pages.push(page);
    }
    let (_dir, pdf) = render(&doc, &AssetStore::default());
    let px_per_mm = DPI as f64 / 25.4;
    for (i, kind) in kinds.iter().enumerate() {
        let (w, h, px) = gray_page(&pdf, i + 1, DPI);
        let dark = px.iter().filter(|&&v| v < 128).count() as f64;
        let ratio = dark / (SIDE_MM * px_per_mm).powi(2);
        let expected = path_area_ratio(shapes::path(*kind).unwrap());
        assert!((ratio - expected).abs() < 0.03, "{kind:?} : surface {ratio:.3} au lieu de {expected:.3} ({w}x{h})");
    }
}

#[test]
#[ignore]
fn arrow_has_its_head() {
    const DPI: u32 = 144;
    let mut doc = Document::new("flèche");
    doc.pages[0].elements.push(Element {
        id: new_id(),
        frame: Frame { x_mm: ORIGIN_MM, y_mm: ORIGIN_MM, w_mm: 100.0, h_mm: 10.0, rotation_deg: 0.0 },
        locked: false,
        zone: false,
        group: None,
        kind: ElementKind::Shape(ShapeProps {
            shape: ShapeKind::Arrow,
            fill: None,
            stroke: Some(Stroke { width_mm: 1.0, color: "#000000".into(), style: None }),
            corner_radius_mm: 0.0,
            path: None,
            graphisme: None,
            ends: None,
            points: None,
            curve: false,
        }),
        subgroups: Vec::new(),
        flip_h: false, flip_v: false,
    });
    let (_dir, pdf) = render(&doc, &AssetStore::default());
    let (w, _, px) = gray_page(&pdf, 1, DPI);
    let px_per_mm = DPI as f64 / 25.4;
    let dark_rows = |x_mm: f64| {
        let x = (x_mm * px_per_mm) as usize;
        px.chunks(w).filter(|row| row[x] < 128).count() as f64 / px_per_mm
    };
    let shaft = dark_rows(ORIGIN_MM + 50.0);
    let head = dark_rows(ORIGIN_MM + 99.0 - 1.5);
    assert!((0.6..1.6).contains(&shaft), "trait de {shaft:.2} mm");
    assert!(head > 1.5, "pointe de {head:.2} mm seulement");
}

/// Chaque terminaison, au début comme à la fin, se dessine dans LibreOffice plus large que le trait.
#[test]
#[ignore]
fn every_line_end_renders_in_libreoffice() {
    use carnet_core::model::{HeadSize, LineEnd, LineEnds};
    const DPI: u32 = 144;
    for end in [LineEnd::Arrow, LineEnd::OpenArrow, LineEnd::Circle, LineEnd::Square] {
        let mut doc = Document::new("bouts");
        doc.pages[0].elements.push(Element {
            id: new_id(),
            frame: Frame { x_mm: ORIGIN_MM, y_mm: ORIGIN_MM, w_mm: 100.0, h_mm: 10.0, rotation_deg: 0.0 },
            locked: false,
            zone: false,
            group: None,
            kind: ElementKind::Shape(ShapeProps {
                shape: ShapeKind::Line,
                fill: None,
                stroke: Some(Stroke { width_mm: 1.0, color: "#000000".into(), style: None }),
                corner_radius_mm: 0.0,
                path: None,
                graphisme: None,
                ends: Some(LineEnds { start: end, end, size: HeadSize::Large }),
                points: None,
                curve: false,
            }),
            subgroups: Vec::new(),
            flip_h: false, flip_v: false,
        });
        let (_dir, pdf) = render(&doc, &AssetStore::default());
        let (w, _, px) = gray_page(&pdf, 1, DPI);
        let px_per_mm = DPI as f64 / 25.4;
        let dark_rows = |x_mm: f64| {
            let x = (x_mm * px_per_mm) as usize;
            px.chunks(w).filter(|row| row[x] < 128).count() as f64 / px_per_mm
        };
        let shaft = dark_rows(ORIGIN_MM + 50.0);
        let inset = if matches!(end, LineEnd::Circle | LineEnd::Square) { 0.4 } else { 4.0 };
        let (start_w, end_w) = (dark_rows(ORIGIN_MM + inset), dark_rows(ORIGIN_MM + 100.0 - inset));
        assert!((0.6..1.6).contains(&shaft), "{end:?} : trait de {shaft:.2} mm");
        assert!(start_w > 2.0 && end_w > 2.0, "{end:?} : début {start_w:.2} mm, fin {end_w:.2} mm");
    }
}

fn picture_element(code: &str, x: f64, group: Option<&str>) -> Element {
    Element {
        id: new_id(),
        frame: Frame { x_mm: x, y_mm: ORIGIN_MM, w_mm: 50.0, h_mm: 50.0, rotation_deg: 0.0 },
        locked: false,
        zone: false,
        group: group.map(String::from),
        kind: ElementKind::Image(ImageProps { code: code.into(), outline: None }),
        subgroups: Vec::new(),
        flip_h: false, flip_v: false,
    }
}

#[test]
#[ignore]
fn picture_is_rendered() {
    const DPI: u32 = 72;
    let mut doc = Document::new("image");
    doc.pages[0].elements.push(picture_element("1F98A", ORIGIN_MM, None));
    let (_dir, pdf) = render(&doc, &AssetStore::default());
    let (w, _, px) = gray_page(&pdf, 1, DPI);
    let s = DPI as f64 / 25.4;
    let (x0, x1, y0, y1) = ((ORIGIN_MM * s) as usize, ((ORIGIN_MM + 50.0) * s) as usize, (ORIGIN_MM * s) as usize, ((ORIGIN_MM + 50.0) * s) as usize);
    let inked = (y0..y1).flat_map(|y| (x0..x1).map(move |x| (x, y))).filter(|&(x, y)| px[y * w + x] < 245).count();
    let ratio = inked as f64 / ((x1 - x0) * (y1 - y0)) as f64;
    assert!(ratio > 0.2, "image presque vide : {ratio:.2}");
}

/// Ce que Cahutin écrit pour se relire, réenregistré tel quel par LibreOffice (`soffice --convert-to odg`) puis relu
/// par Cahutin : les pages relues et le `content.xml` réécrit. Un seul document et une seule conversion pour les tests
/// qui suivent (le banc reste rapide). Page 1 : groupes imbriqués, pictogrammes (à colorier, en miroir), champs non
/// remplis (lettres comprises), éléments verrouillés, flèches, photo bordée et ombrée, cadre Seyès où l'on écrit sur
/// les lignes, sur un fond Seyès ; pages 2 à 4 : les autres motifs, à d'autres tailles.
struct RoundTrip {
    pages: Vec<Page>,
    content: String,
    /// Tailles de l'export et du fichier réenregistré, et ce que celui-ci contient.
    exported_len: u64,
    saved_len: u64,
    saved_settings: String,
    saved_styles: String,
    saved_fonts: Vec<String>,
}

fn libreoffice_round_trip() -> &'static RoundTrip {
    static TRIP: std::sync::OnceLock<RoundTrip> = std::sync::OnceLock::new();
    TRIP.get_or_init(|| {
        let mut doc = Document::new("aller-retour");
        let mut store = AssetStore::default();
        let photo = halves_asset(&mut doc, &mut store);
        let grouped = |code: &str, x: f64, path: &[&str]| {
            let mut e = picture_element(code, x, None);
            e.frame.w_mm = 20.0;
            e.frame.h_mm = 20.0;
            e.group = path.first().map(|g| g.to_string());
            e.subgroups = path[1..].iter().map(|g| g.to_string()).collect();
            e
        };
        let text_at = |y: f64, t: &str| element(frame_at(20.0, y, 120.0, 12.0, 0.0), text_kind(t, 14.0));
        let mut star = element(frame_at(150.0, 20.0, 20.0, 20.0, 0.0), filled_shape(ShapeKind::Star, None));
        star.group = Some("g".into());
        let arrow = |y: f64, flip_h: bool| {
            let k = ElementKind::Shape(ShapeProps { shape: ShapeKind::Arrow, fill: None, stroke: Some(Stroke { width_mm: 1.0, color: "#c0392b".into(), style: None }), corner_radius_mm: 0.0, path: None, graphisme: None, ends: None, points: None, curve: false });
            Element { flip_h, ..element(frame_at(20.0, y, 60.0, 8.0, 0.0), k) }
        };
        let mut framed = element(frame_at(90.0, 150.0, 100.0, 40.0, 0.0), text_kind("Léa", 14.0));
        if let ElementKind::Text(t) = &mut framed.kind {
            t.frame = Some(TextFrame { fill: Some("#fff1c1".into()), stroke: Some(Stroke { width_mm: 0.5, color: "#9aa5b1".into(), style: Some(StrokeStyle::Dashed) }), corner_radius_mm: 3.0, image: None, pattern: Some(Pattern::Seyes), write_on_lines: Some(true) });
        }
        let mut blank_lined = element(frame_at(90.0, 230.0, 100.0, 40.0, 0.0), text_kind("", 14.0));
        if let ElementKind::Text(t) = &mut blank_lined.kind {
            t.content = RichText::default();
            t.frame = Some(TextFrame { pattern: Some(Pattern::Seyes), write_on_lines: Some(true), ..TextFrame::default() });
        }
        let mut bordered = element(frame_at(20.0, 200.0, 60.0, 30.0, 0.0), photo_kind(&photo));
        if let ElementKind::Photo(p) = &mut bordered.kind {
            p.border = Some(Stroke { width_mm: 1.5, color: "#2e86c1".into(), style: Some(StrokeStyle::Dotted) });
            p.shadow = true;
            p.corner_radius_mm = 2.0;
        }
        let mut outline = picture_element("1F431", 20.0, None);
        outline.frame = frame_at(20.0, 45.0, 20.0, 20.0, 0.0);
        outline.kind = ElementKind::Image(ImageProps { code: "1F431".into(), outline: Some(true) });
        let mirrored = Element { flip_h: true, frame: frame_at(50.0, 45.0, 20.0, 20.0, 0.0), ..picture_element("2600", 0.0, None) };
        let locked_star = Element { locked: true, ..element(frame_at(150.0, 50.0, 20.0, 20.0, 0.0), filled_shape(ShapeKind::Heart, None)) };
        let locked_text = Element { locked: true, ..text_at(130.0, "Verrouillé") };
        doc.pages[0].elements = vec![
            grouped("1F98A", 20.0, &["g", "f"]),
            grouped("2600", 50.0, &["g", "f"]),
            grouped("1F33B", 80.0, &["g"]),
            star,
            outline,
            mirrored,
            text_at(75.0, "Bonjour {prenom} !"),
            text_at(90.0, "{PRENOM_1}{prenom_2}"),
            arrow(110.0, false),
            arrow(120.0, true),
            {
                let mut a = arrow(130.0, false);
                if let ElementKind::Shape(s) = &mut a.kind {
                    s.ends = Some(carnet_core::model::LineEnds { start: carnet_core::model::LineEnd::Square, end: carnet_core::model::LineEnd::OpenArrow, size: carnet_core::model::HeadSize::Small });
                }
                a
            },
            path_element(&ROUND_TRIP_ELBOWS, false, ShapeKind::Arrow, Some(carnet_core::model::LineEnds { start: carnet_core::model::LineEnd::Circle, end: carnet_core::model::LineEnd::Arrow, size: carnet_core::model::HeadSize::Medium })),
            path_element(&ROUND_TRIP_CURVE, true, ShapeKind::Line, None),
            locked_star,
            locked_text,
            framed,
            bordered,
            blank_lined,
        ];
        doc.pages[0].background = Some(Background { color: "#ffffff".into(), pattern: Some(Pattern::Seyes) });
        for (pattern, size, color) in [(Pattern::Dots, PageSize { width_mm: 148.0, height_mm: 210.0 }, "#fff8dc"), (Pattern::Lines, PageSize { width_mm: 297.0, height_mm: 210.0 }, "#ffffff"), (Pattern::SmallSquares, A4_PORTRAIT, "#eef6ff")] {
            doc.pages.push(Page { background: Some(Background { color: color.into(), pattern: Some(pattern) }), ..Page::new(size) });
        }
        let dir = tempfile::tempdir().unwrap();
        let odg = dir.path().join("aller-retour.odg");
        export_odg_to_path(&doc, &store, &FontSet::builtin(), &odg).unwrap();
        let out = dir.path().join("relu");
        soffice_convert(dir.path(), "odg", &odg, &out);
        let saved = out.join("aller-retour.odg");
        let mut zip = zip::ZipArchive::new(std::fs::File::open(&saved).unwrap()).unwrap();
        let mut part = |name: &str| {
            let mut s = String::new();
            std::io::Read::read_to_string(&mut zip.by_name(name).unwrap(), &mut s).unwrap();
            s
        };
        let (content, saved_settings, saved_styles) = (part("content.xml"), part("settings.xml"), part("styles.xml"));
        let saved_fonts = zip.file_names().map(|n| n.unwrap().into_owned()).filter(|n| n.starts_with("Fonts/")).collect();
        let known: Vec<String> = FontSet::builtin().families().into_iter().map(|f| f.name).collect();
        let back = carnet_core::import::odg::read_drawing(std::fs::File::open(&saved).unwrap(), &known).unwrap();
        let len = |p: &Path| std::fs::metadata(p).unwrap().len();
        RoundTrip { pages: back.pages, content, exported_len: len(&odg), saved_len: len(&saved), saved_settings, saved_styles, saved_fonts }
    })
}

// Hors des cadres à lignes (x > 85 et y > 145 : voir `lined_frames_survive_a_libreoffice_round_trip`).
const ROUND_TRIP_ELBOWS: [(f64, f64); 4] = [(20.0, 245.0), (45.0, 245.0), (45.0, 265.0), (70.0, 280.0)];
const ROUND_TRIP_CURVE: [(f64, f64); 4] = [(150.0, 80.0), (165.0, 105.0), (180.0, 85.0), (195.0, 110.0)];

/// Un chemin passant par `pts` (mm de page).
fn path_element(pts: &[(f64, f64)], curve: bool, shape: ShapeKind, ends: Option<carnet_core::model::LineEnds>) -> Element {
    let mut s = ShapeProps { shape, fill: None, stroke: Some(Stroke { width_mm: 1.0, color: "#2e86c1".into(), style: None }), corner_radius_mm: 0.0, path: None, graphisme: None, ends, points: None, curve };
    let (frame, points) = carnet_core::shapes::path_frame(pts, &s);
    s.points = Some(points);
    Element { id: new_id(), frame, locked: false, zone: false, group: None, kind: ElementKind::Shape(s), subgroups: Vec::new(), flip_h: false, flip_v: false }
}

/// Un chemin à coudes et un chemin courbe, réenregistrés par LibreOffice, reviennent modifiables point par point,
/// avec leurs terminaisons.
#[test]
#[ignore]
fn paths_come_back_from_libreoffice() {
    let page = &libreoffice_round_trip().pages[0];
    let paths: Vec<_> = page.elements.iter().filter(|e| matches!(&e.kind, ElementKind::Shape(s) if s.points.is_some())).collect();
    assert_eq!(paths.len(), 2, "{:?}", page.elements);
    for (want, curve) in [(&ROUND_TRIP_ELBOWS, false), (&ROUND_TRIP_CURVE, true)] {
        let e = paths.iter().find(|e| matches!(&e.kind, ElementKind::Shape(s) if s.curve == curve)).unwrap_or_else(|| panic!("courbe {curve} : {paths:?}"));
        let ElementKind::Shape(s) = &e.kind else { panic!() };
        let got = carnet_core::shapes::path_page_points(&e.frame, s, e.flip_h, e.flip_v).unwrap();
        assert_eq!(got.len(), want.len(), "{got:?}");
        for (g, w) in got.iter().zip(want.iter()) {
            assert!((g.0 - w.0).abs() < 0.05 && (g.1 - w.1).abs() < 0.05, "{got:?}");
        }
        if !curve {
            use carnet_core::model::{HeadSize, LineEnd, LineEnds};
            assert_eq!((s.shape, s.ends), (ShapeKind::Arrow, Some(LineEnds { start: LineEnd::Circle, end: LineEnd::Arrow, size: HeadSize::Medium })), "{s:?}");
        }
    }
}

/// L'élément relu de la page 1 dont le cadre a ce coin haut gauche (au 1/10 de mm).
fn relu_at(x: f64, y: f64) -> &'static Element {
    let page = &libreoffice_round_trip().pages[0];
    page.elements.iter().find(|e| (e.frame.x_mm - x).abs() < 0.1 && (e.frame.y_mm - y).abs() < 0.1).unwrap_or_else(|| panic!("rien à {x} {y} : {:?}", page.elements))
}

/// Réenregistré par LibreOffice, un export reste léger : LibreOffice garde le réglage « seulement les polices
/// latines utilisées » (sans lui, il embarquait toutes les polices de repli du poste, polices chinoises comprises :
/// 18 Mo pour un export de 0,5 Mo) et embarque toujours la police de Cahutin. La borne de 4 Mo ajoutés laisse passer
/// les polices latines des styles par défaut de LibreOffice (mesuré : Liberation Sans et Serif, 1,6 Mo, sur la 24.2
/// d'un poste courant ; DejaVu Sans et Serif, 2,2 Mo, sur une 24.2 qui n'a qu'elles), pas une police chinoise (8,5 Mo).
#[test]
#[ignore]
fn a_libreoffice_resave_keeps_only_the_used_latin_fonts() {
    let trip = libreoffice_round_trip();
    let item = |k: &str, v: &str| format!("config:name=\"{k}\" config:type=\"boolean\">{v}<");
    for (k, v) in [("EmbedOnlyUsedFonts", "true"), ("EmbedAsianScriptFonts", "false"), ("EmbedComplexScriptFonts", "false")] {
        assert!(trip.saved_settings.contains(&item(k, v)), "{k} : {}", trip.saved_settings);
    }
    let andika: Vec<&String> = trip.saved_fonts.iter().filter(|n| n.contains("Andika")).collect();
    assert!(!andika.is_empty(), "{:?}", trip.saved_fonts);
    assert!(andika.iter().any(|n| trip.content.contains(&format!("xlink:href=\"{n}\"")) || trip.saved_styles.contains(&format!("xlink:href=\"{n}\""))), "Andika non déclarée");
    assert!(trip.saved_len <= trip.exported_len + 4_000_000, "{} octets réenregistrés pour {} exportés : {:?}", trip.saved_len, trip.exported_len, trip.saved_fonts);
}

/// Un groupe et un groupe dans ce groupe reviennent tels quels de LibreOffice, relus par Cahutin.
#[test]
#[ignore]
fn groups_survive_a_libreoffice_round_trip() {
    let path = |x: f64| carnet_core::groups::path(relu_at(x, ORIGIN_MM));
    let (fox, sun, flower, star) = (path(20.0), path(50.0), path(80.0), carnet_core::groups::path(relu_at(150.0, 20.0)));
    assert_eq!((fox.len(), sun.len(), flower.len(), star.len()), (2, 2, 1, 1));
    assert!(fox == sun && fox[0] == flower[0] && flower == star && fox[1] != fox[0]);
    assert!(carnet_core::groups::path(relu_at(20.0, 45.0)).is_empty());
}

/// LibreOffice renomme les fichiers des pictogrammes : ils reviennent pictogrammes (à colorier, en miroir compris),
/// et la page qui en a garde sa mention de licence à l'export suivant.
#[test]
#[ignore]
fn pictures_survive_a_libreoffice_round_trip() {
    let trip = libreoffice_round_trip();
    assert!(!trip.content.contains("Pictures/openmoji-"), "LibreOffice ne renomme plus les images : le test ne prouve plus rien");
    let picto = |x: f64, y: f64| match &relu_at(x, y).kind {
        ElementKind::Image(i) => (i.code.clone(), i.outline, relu_at(x, y).flip_h),
        k => panic!("{x} {y} : {k:?}"),
    };
    assert_eq!(picto(20.0, ORIGIN_MM), ("1F98A".to_string(), None, false));
    assert_eq!(picto(20.0, 45.0), ("1F431".to_string(), Some(true), false));
    assert_eq!(picto(50.0, 45.0), ("2600".to_string(), None, true));
}

/// Un champ non rempli revient champ, une lettre comprise (nommée dans la description du cadre).
#[test]
#[ignore]
fn unfilled_fields_survive_a_libreoffice_round_trip() {
    let text = |y: f64| match &relu_at(20.0, y).kind {
        ElementKind::Text(t) => t.content.paragraphs.iter().flat_map(|p| &p.runs).map(|r| r.text.as_str()).collect::<String>(),
        k => panic!("{k:?}"),
    };
    assert_eq!(text(75.0), "Bonjour {prenom} !");
    assert_eq!(text(90.0), "{PRENOM_1}{prenom_2}");
}

/// Un élément verrouillé l'est encore dans le fichier réécrit (`style:protect`) et revient verrouillé ; les autres non.
#[test]
#[ignore]
fn locks_survive_a_libreoffice_round_trip() {
    let trip = libreoffice_round_trip();
    // Le style du cœur verrouillé, celui du texte verrouillé (qu'il partage avec la mention de licence).
    let protect: Vec<&str> = trip.content.split("style:protect=\"").skip(1).map(|s| s.split('"').next().unwrap()).collect();
    assert!(protect.len() >= 2 && protect.iter().all(|p| p.contains("position")), "{protect:?}");
    assert!(relu_at(150.0, 50.0).locked && relu_at(20.0, 130.0).locked);
    assert!(!relu_at(150.0, 20.0).locked && !relu_at(20.0, 75.0).locked);
}

/// Les motifs de page (dessinés sur la page maîtresse) reviennent motifs, avec leur couleur, à chaque taille.
#[test]
#[ignore]
fn page_patterns_survive_a_libreoffice_round_trip() {
    let got: Vec<Option<Background>> = libreoffice_round_trip().pages.iter().map(|p| p.background.clone()).collect();
    let bg = |c: &str, p| Some(Background { color: c.into(), pattern: Some(p) });
    assert_eq!(got, [bg("#ffffff", Pattern::Seyes), bg("#fff8dc", Pattern::Dots), bg("#ffffff", Pattern::Lines), bg("#eef6ff", Pattern::SmallSquares)]);
}

/// Un cadre à lignes d'écriture revient cadre à lignes : fond, bordure en tirets, coins, Seyès, écrit sur les lignes.
#[test]
#[ignore]
fn lined_frames_survive_a_libreoffice_round_trip() {
    let ElementKind::Text(t) = &relu_at(90.0, 150.0).kind else { panic!() };
    let want = TextFrame { fill: Some("#fff1c1".into()), stroke: Some(Stroke { width_mm: 0.5, color: "#9aa5b1".into(), style: Some(StrokeStyle::Dashed) }), corner_radius_mm: 3.0, image: None, pattern: Some(Pattern::Seyes), write_on_lines: Some(true) };
    assert_eq!(t.frame.as_ref(), Some(&want));
    // Une étiquette vide où l'on écrit sur les lignes : LibreOffice perd le style de son paragraphe vide.
    let ElementKind::Text(t) = &relu_at(90.0, 230.0).kind else { panic!() };
    assert_eq!(t.frame.as_ref().map(|f| (f.pattern, f.write_on_lines)), Some((Some(Pattern::Seyes), Some(true))));
    // Rien d'autre du cadre ne reste sur la page (ni rectangle, ni traits).
    let page = &libreoffice_round_trip().pages[0];
    assert!(!page.elements.iter().any(|e| matches!(e.kind, ElementKind::Shape(_)) && e.frame.x_mm > 85.0 && e.frame.y_mm > 145.0), "{:?}", page.elements);
}

/// Une flèche revient flèche, la pointe au même bout (retournée : tournée d'un demi-tour) ; une photo garde sa bordure
/// en pointillés et son ombre.
#[test]
#[ignore]
fn arrows_and_photo_borders_survive_a_libreoffice_round_trip() {
    let page = &libreoffice_round_trip().pages[0];
    let arrow = |y: f64| page.elements.iter().find(|e| matches!(&e.kind, ElementKind::Shape(s) if s.shape == ShapeKind::Arrow) && (e.frame.y_mm + e.frame.h_mm / 2.0 - y).abs() < 0.1).unwrap_or_else(|| panic!("pas de flèche à {y} : {:?}", page.elements));
    assert!(arrow(114.0).frame.rotation_deg.abs() < 0.01);
    assert!((arrow(124.0).frame.rotation_deg.rem_euclid(360.0) - 180.0).abs() < 0.01);
    // Les terminaisons choisies reviennent à leur bout, avec leur taille.
    let ElementKind::Shape(s) = &arrow(134.0).kind else { panic!() };
    use carnet_core::model::{HeadSize, LineEnd, LineEnds};
    assert_eq!(s.ends, Some(LineEnds { start: LineEnd::Square, end: LineEnd::OpenArrow, size: HeadSize::Small }), "{s:?}");
    let ElementKind::Photo(p) = &relu_at(20.0, 200.0).kind else { panic!() };
    assert_eq!((p.border.clone(), p.shadow, p.corner_radius_mm), (Some(Stroke { width_mm: 1.5, color: "#2e86c1".into(), style: Some(StrokeStyle::Dotted) }), true, 2.0));
}

#[test]
#[ignore]
fn background_is_rendered() {
    const DPI: u32 = 72;
    let mut doc = Document::new("fond");
    doc.pages[0].background = Some(Background { color: carnet_core::background::KRAFT.into(), pattern: Some(Pattern::Seyes) });
    let (dir, pdf) = render(&doc, &AssetStore::default());
    let prefix = dir.path().join("couleur");
    run(Command::new("pdftoppm").args(["-r", &DPI.to_string(), "-singlefile"]).arg(&pdf).arg(&prefix));
    let data = std::fs::read(prefix.with_extension("ppm")).unwrap();
    // En-tête P6 : quatre champs puis les pixels RVB.
    let mut fields = 0;
    let mut pos = 0;
    while fields < 4 {
        while data[pos].is_ascii_whitespace() {
            pos += 1;
        }
        while !data[pos].is_ascii_whitespace() {
            pos += 1;
        }
        fields += 1;
    }
    let header = String::from_utf8_lossy(&data[..pos]).into_owned();
    let w: usize = header.split_whitespace().nth(1).unwrap().parse().unwrap();
    let px = &data[pos + 1..];
    let k = DPI as f64 / 25.4;
    let at = |x_mm: f64, y_mm: f64| {
        let i = ((y_mm * k) as usize * w + (x_mm * k) as usize) * 3;
        (px[i], px[i + 1], px[i + 2])
    };
    let (r, g, b) = at(5.0, 5.0);
    assert!((r as i32 - 0xd8).abs() < 12 && (g as i32 - 0xbc).abs() < 12 && (b as i32 - 0x8a).abs() < 12, "coin : {r},{g},{b}");
    let off_kraft = (0..(280.0 * k) as usize).filter(|&y| { let i = (y * w + (100.5 * k) as usize) * 3; (px[i] as i32 - 0xd8).abs() + (px[i + 1] as i32 - 0xbc).abs() + (px[i + 2] as i32 - 0x8a).abs() > 30 }).count();
    assert!(off_kraft > 20, "lignes Seyès absentes : {off_kraft} pixels");
}

#[test]
#[ignore]
fn examples_render_in_libreoffice() {
    for info in carnet_core::examples::list() {
        let doc = carnet_core::examples::document(&info.id).unwrap();
        let (_dir, pdf) = render(&doc, &AssetStore::default());
        let text: String = run(Command::new("pdftotext").arg(&pdf).arg("-")).split_whitespace().collect();
        // Le titre : premier mot écrit du plus grand texte de l'exemple ; un champ non rempli sort blanc.
        let written = |t: &str| {
            carnet_core::fields::split(t)
                .into_iter()
                .map(|p| match p {
                    carnet_core::fields::Piece::Text(x) => x,
                    carnet_core::fields::Piece::Field(_) => " ",
                })
                .collect::<String>()
        };
        let title = doc
            .pages
            .iter()
            .flat_map(|p| &p.elements)
            .filter_map(|e| match &e.kind {
                carnet_core::ElementKind::Text(t) if !t.content.is_blank() => Some((t.style.size_pt, t.content.paragraphs.iter().flat_map(|p| p.runs.iter().map(|r| written(&r.text))).collect::<Vec<_>>().join(" "))),
                _ => None,
            })
            .max_by(|a, b| a.0.total_cmp(&b.0))
            .map(|(_, t)| t);
        // Un exemple fait seulement de zones à remplir (étiquettes) ou de champs n'a pas de titre écrit.
        if let Some(word) = title.as_deref().and_then(|t| t.split_whitespace().next()) {
            assert!(text.contains(word), "{} : {word:?} absent de {text}", info.id);
        }
        assert!(text.contains("OpenMoji"), "{} : mention absente", info.id);
    }
}

/// Chaque modèle intégré s'exporte et se rend ; fond des étiquettes au bon endroit, affiche en paysage.
#[test]
#[ignore]
fn builtin_templates_render_in_libreoffice() {
    const DPI: u32 = 254; // 10 pixels par millimètre
    for t in carnet_core::templates::builtin() {
        let (page, _) = carnet_core::templates::instantiate(&t);
        let mut doc = Document::new("");
        doc.pages = vec![page];
        let (_dir, pdf) = render(&doc, &AssetStore::default());
        let (pw, ph, px) = rgb_page(&pdf, 1, DPI);
        let at = |x_mm: f64, y_mm: f64| {
            let i = ((y_mm * 10.0) as usize * pw + (x_mm * 10.0) as usize) * 3;
            (px[i], px[i + 1], px[i + 2])
        };
        let label_fill = (0xea, 0xf2, 0xfb);
        match t.info.id.as_str() {
            // Centre de la première étiquette, puis à droite de la photo de la première étiquette photo.
            "etiquettes" => assert_eq!(at(55.0, 33.5), label_fill, "{}", t.info.id),
            "etiquettes-photo" => assert_eq!(at(98.0, 36.5), label_fill, "{}", t.info.id),
            "affiche-paysage" => assert!(pw > ph, "{} : {pw} × {ph}", t.info.id),
            _ => assert!(ph > pw, "{} : {pw} × {ph}", t.info.id),
        }
    }
}

/// Étiquette : le même prénom dans trois polices d'un seul texte ; LibreOffice utilise les trois.
#[test]
#[ignore]
fn several_fonts_in_one_text_are_rendered() {
    let mut rt = RichText::plain("Léa\nLéa\nLéa");
    rt.paragraphs[1].runs[0].font = Some("OpenDyslexic".into());
    rt.paragraphs[2].runs[0].font = Some("Playwrite FR Moderne".into());
    let mut doc = Document::new("polices");
    doc.pages[0].elements.push(Element {
        id: new_id(),
        frame: Frame { x_mm: 30.0, y_mm: 30.0, w_mm: 120.0, h_mm: 70.0, rotation_deg: 0.0 },
        locked: false,
        zone: false,
        group: None,
        kind: ElementKind::Text(TextProps { content: rt, style: TextStyle::default_zone(), prompt: None, frame: None }),
        subgroups: Vec::new(),
        flip_h: false, flip_v: false,
    });
    let (_dir, pdf) = render(&doc, &AssetStore::default());
    let fonts = run(Command::new("pdffonts").arg(&pdf));
    for family in ["Andika", "OpenDyslexic", "Playwrite"] {
        assert!(fonts.contains(family), "{family} absente :\n{fonts}");
    }
}

/// Marge de 5 mm et alignement vertical : le mot est en haut, au milieu ou en bas, à 5 mm des bords.
#[test]
#[ignore]
fn vertical_alignment_and_padding_are_rendered() {
    let mut doc = Document::new("banc");
    doc.pages.clear();
    let (w, h, pad) = (100.0, 40.0, 5.0);
    for valign in [VAlign::Top, VAlign::Middle, VAlign::Bottom] {
        let mut style = TextStyle::default_zone();
        style.size_pt = 14.0;
        style.valign = Some(valign);
        style.padding_mm = Some(pad);
        let mut page = Page::new(A4_PORTRAIT);
        page.elements.push(Element {
            id: new_id(),
            frame: Frame { x_mm: ORIGIN_MM, y_mm: ORIGIN_MM, w_mm: w, h_mm: h, rotation_deg: 0.0 },
            locked: false,
            zone: false,
            group: None,
            kind: ElementKind::Text(TextProps { content: RichText::plain("Milieu"), style, prompt: None, frame: None }),
            subgroups: Vec::new(),
            flip_h: false, flip_v: false,
        });
        doc.pages.push(page);
    }
    let (_dir, pdf) = render(&doc, &AssetStore::default());
    let words = words(&pdf);
    let pt = |v_mm: f64| v_mm * PT_PER_MM;
    for (page, valign) in [VAlign::Top, VAlign::Middle, VAlign::Bottom].into_iter().enumerate() {
        let w0 = words.iter().find(|w| w.page == page).expect("mot rendu");
        assert!((w0.x_min - pt(ORIGIN_MM + pad)).abs() < 1.5, "{valign:?} : marge gauche {:.1}", w0.x_min);
        let (top, bottom) = (pt(ORIGIN_MM + pad), pt(ORIGIN_MM + h - pad));
        match valign {
            VAlign::Top => assert!(w0.y_min >= top - 1.0 && w0.y_min < top + 6.0, "haut : {:.1} pour {top:.1}", w0.y_min),
            VAlign::Middle => {
                let mid = (w0.y_min + w0.y_max) / 2.0;
                assert!((mid - pt(ORIGIN_MM + h / 2.0)).abs() < 3.0, "milieu : {mid:.1}");
            }
            VAlign::Bottom => assert!(w0.y_max <= bottom + 1.0 && w0.y_max > bottom - 8.0, "bas : {:.1} pour {bottom:.1}", w0.y_max),
        }
    }
}

/// Tirets et pointillés mesurés sur le rendu : mêmes longueurs qu'à l'écran (3e et 2e, puis e et e).
#[test]
#[ignore]
fn dashes_have_the_screen_lengths() {
    const DPI: u32 = 254; // 10 pixels par millimètre
    let width = 2.0;
    let mut doc = Document::new("tirets");
    doc.pages.clear();
    for style in [StrokeStyle::Dashed, StrokeStyle::Dotted] {
        let mut page = Page::new(A4_PORTRAIT);
        page.elements.push(Element {
            id: new_id(),
            frame: Frame { x_mm: 20.0, y_mm: 50.0, w_mm: 150.0, h_mm: 8.0, rotation_deg: 0.0 },
            locked: false,
            zone: false,
            group: None,
            kind: ElementKind::Shape(ShapeProps {
                shape: ShapeKind::Line,
                fill: None,
                stroke: Some(Stroke { width_mm: width, color: "#000000".into(), style: Some(style) }),
                corner_radius_mm: 0.0,
                path: None,
                graphisme: None,
                ends: None,
                points: None,
                curve: false,
            }),
            subgroups: Vec::new(),
            flip_h: false, flip_v: false,
        });
        doc.pages.push(page);
    }
    let (_dir, pdf) = render(&doc, &AssetStore::default());
    for (page, (dash, gap)) in [(1, (3.0 * width, 2.0 * width)), (2, (width, width))] {
        let (w, _h, px) = gray_page(&pdf, page, DPI);
        let row = &px[54 * 10 * w..54 * 10 * w + w];
        // Longueurs des suites de pixels sombres et clairs entre le premier et le dernier tiret.
        let mut runs: Vec<(bool, usize)> = vec![];
        for &p in row {
            let dark = p < 128;
            match runs.last_mut() {
                Some((d, n)) if *d == dark => *n += 1,
                _ => runs.push((dark, 1)),
            }
        }
        let inner: Vec<(bool, usize)> = runs.iter().copied().skip_while(|r| !r.0).collect();
        let inner = &inner[..inner.iter().rposition(|r| r.0).unwrap() + 1];
        let mean = |dark: bool| {
            let v: Vec<f64> = inner.iter().filter(|r| r.0 == dark).map(|r| r.1 as f64 / 10.0).collect();
            assert!(v.len() >= 5, "page {page} : trop peu de tirets ({runs:?})");
            v.iter().sum::<f64>() / v.len() as f64
        };
        let (d, g) = (mean(true), mean(false));
        assert!((d - dash).abs() < 0.6 && (g - gap).abs() < 0.6, "page {page} : tirets {d:.2} mm, espaces {g:.2} mm (attendu {dash} et {gap})");
    }
}

/// Page rendue en couleur par poppler : (largeur, hauteur, pixels RVB).
fn rgb_page(pdf: &Path, page: usize, dpi: u32) -> (usize, usize, Vec<u8>) {
    let prefix = pdf.parent().unwrap().join(format!("c{page}"));
    run(Command::new("pdftoppm").args(["-r", &dpi.to_string(), "-f", &page.to_string(), "-l", &page.to_string(), "-singlefile"]).arg(pdf).arg(&prefix));
    let data = std::fs::read(prefix.with_extension("ppm")).unwrap();
    let mut fields = Vec::new();
    let mut pos = 0;
    while fields.len() < 4 {
        while data[pos].is_ascii_whitespace() {
            pos += 1;
        }
        let start = pos;
        while !data[pos].is_ascii_whitespace() {
            pos += 1;
        }
        fields.push(String::from_utf8_lossy(&data[start..pos]).into_owned());
    }
    let (w, h): (usize, usize) = (fields[1].parse().unwrap(), fields[2].parse().unwrap());
    (w, h, data[pos + 1..].to_vec())
}

/// Cadre de texte : fond, bordure, coins arrondis et marge rendus par LibreOffice comme à l'écran.
#[test]
#[ignore]
fn text_frame_is_rendered() {
    const DPI: u32 = 254; // 10 pixels par millimètre
    let (x, y, w, h, pad, radius) = (ORIGIN_MM, ORIGIN_MM, 100.0, 40.0, 5.0, 8.0);
    let mut style = TextStyle::default_zone();
    style.padding_mm = Some(pad);
    style.valign = Some(VAlign::Middle);
    let mut doc = Document::new("cadre");
    doc.pages[0].elements.push(Element {
        id: new_id(),
        frame: Frame { x_mm: x, y_mm: y, w_mm: w, h_mm: h, rotation_deg: 0.0 },
        locked: false,
        zone: false,
        group: None,
        kind: ElementKind::Text(TextProps {
            content: RichText::plain("Étiquette"),
            style,
            prompt: None,
            frame: Some(TextFrame {
                fill: Some("#3c78d8".into()),
                stroke: Some(Stroke { width_mm: 1.0, color: "#cc0000".into(), style: None }),
                corner_radius_mm: radius,
                image: None,
                pattern: None,
                write_on_lines: None,
            }),
        }),
        subgroups: Vec::new(),
        flip_h: false, flip_v: false,
    });
    let (_dir, pdf) = render(&doc, &AssetStore::default());
    let (pw, _ph, px) = rgb_page(&pdf, 1, DPI);
    let at = |x_mm: f64, y_mm: f64| {
        let i = ((y_mm * 10.0) as usize * pw + (x_mm * 10.0) as usize) * 3;
        (px[i], px[i + 1], px[i + 2])
    };
    let blue = |c: (u8, u8, u8)| c.2 > 180 && c.0 < 100;
    let red = |c: (u8, u8, u8)| c.0 > 180 && c.1 < 80 && c.2 < 80;
    let white = |c: (u8, u8, u8)| c.0 > 240 && c.1 > 240 && c.2 > 240;
    assert!(blue(at(x + 20.0, y + 5.0)), "fond : {:?}", at(x + 20.0, y + 5.0));
    assert!(red(at(x + 0.3, y + h / 2.0)), "bordure gauche : {:?}", at(x + 0.3, y + h / 2.0));
    // Coin arrondi : le point du coin, hors de l'arc, reste blanc.
    assert!(white(at(x + 0.8, y + 0.8)), "coin : {:?}", at(x + 0.8, y + 0.8));
    let words = words(&pdf);
    let wd = words.iter().find(|w| w.text.contains("tiquette")).expect("texte rendu");
    assert!((wd.x_min - (x + pad) * PT_PER_MM).abs() < 1.5, "marge : {:.1}", wd.x_min);
    let mid = (wd.y_min + wd.y_max) / 2.0;
    assert!((mid - (y + h / 2.0) * PT_PER_MM).abs() < 3.0, "milieu : {mid:.1}");
}

/// Étiquette : l'image de fond couvre le cadre de texte aux coins arrondis, sous le texte ; une photo à
/// bordure en pointillés garde ses points (remplissage bitmap d'un rectangle).
#[test]
#[ignore]
fn frame_image_and_dotted_photo_border_are_rendered() {
    const DPI: u32 = 254; // 10 pixels par millimètre
    let mut store = AssetStore::default();
    let mut bytes = Cursor::new(Vec::new());
    RgbImage::from_pixel(600, 200, Rgb([90, 140, 200])).write_to(&mut bytes, ImageFormat::Png).unwrap();
    let label = import_bytes(&bytes.into_inner()).unwrap();
    let id = label.meta.id.clone();
    let mut doc = Document::new("étiquette");
    doc.assets.insert(id.clone(), label.meta.clone());
    store.insert(label).unwrap();
    let (x, y, w, h) = (ORIGIN_MM, ORIGIN_MM, 90.0, 30.0);
    let mut style = TextStyle::default_zone();
    style.valign = Some(VAlign::Middle);
    doc.pages[0].elements.push(Element {
        id: new_id(),
        frame: Frame { x_mm: x, y_mm: y, w_mm: w, h_mm: h, rotation_deg: 0.0 },
        locked: false,
        zone: false,
        group: None,
        kind: ElementKind::Text(TextProps {
            content: RichText::plain("Prénom"),
            style,
            prompt: None,
            frame: Some(TextFrame { fill: None, stroke: None, corner_radius_mm: 8.0, image: Some(id.clone()), pattern: None, write_on_lines: None }),
        }),
        subgroups: Vec::new(),
        flip_h: false, flip_v: false,
    });
    let (py, ph) = (100.0, 60.0);
    doc.pages[0].elements.push(Element {
        id: new_id(),
        frame: Frame { x_mm: x, y_mm: py, w_mm: 90.0, h_mm: ph, rotation_deg: 0.0 },
        locked: false,
        zone: false,
        group: None,
        kind: ElementKind::Photo(PhotoProps {
            asset: Some(id),
            crop: Crop::COVER,
            corner_radius_mm: 0.0,
            border: Some(Stroke { width_mm: 2.0, color: "#cc0000".into(), style: Some(StrokeStyle::Dotted) }),
            shadow: false,
            mirrored: false,
        }),
        subgroups: Vec::new(),
        flip_h: false, flip_v: false,
    });
    let (_dir, pdf) = render(&doc, &store);
    let (pw, _ph, px) = rgb_page(&pdf, 1, DPI);
    let at = |x_mm: f64, y_mm: f64| {
        let i = ((y_mm * 10.0) as usize * pw + (x_mm * 10.0) as usize) * 3;
        (px[i], px[i + 1], px[i + 2])
    };
    let near = |c: (u8, u8, u8), r: u8, g: u8, b: u8| c.0.abs_diff(r) < 25 && c.1.abs_diff(g) < 25 && c.2.abs_diff(b) < 25;
    assert!(near(at(x + 10.0, y + 4.0), 90, 140, 200), "image de fond : {:?}", at(x + 10.0, y + 4.0));
    assert!(near(at(x + 0.8, y + 0.8), 255, 255, 255), "coin arrondi : {:?}", at(x + 0.8, y + 0.8));
    assert!(words(&pdf).iter().any(|wd| wd.text.contains("Prénom")), "texte de l'étiquette absent");
    // Bordure en pointillés : le long du bord haut, alternance de points rouges et de photo.
    let row: Vec<bool> = (0..800).map(|i| near(at(x + 5.0 + i as f64 / 10.0, py + 0.2), 204, 0, 0)).collect();
    let switches = row.windows(2).filter(|p| p[0] != p[1]).count();
    assert!(switches > 20, "pointillés : {switches} changements sur 80 mm");
}

/// Interligne à 150 % : le pas des lignes vaut une fois et demie celui du même texte sans réglage. Comparé à un
/// bloc témoin rendu dans le même PDF, pour ne pas dépendre des hauteurs exactes du LibreOffice qui fait le rendu.
#[test]
#[ignore]
fn line_spacing_sets_the_line_pitch() {
    let mut doc = Document::new("interligne");
    let block = |x_mm: f64, prefix: &str, spacing: Option<f64>| {
        let mut style = TextStyle::default_zone();
        style.size_pt = 20.0;
        style.line_spacing = spacing;
        let text = (1..=5).map(|i| format!("{prefix}{i}")).collect::<Vec<_>>().join("\n");
        Element {
            id: new_id(),
            frame: Frame { x_mm, y_mm: ORIGIN_MM, w_mm: 70.0, h_mm: 150.0, rotation_deg: 0.0 },
            locked: false,
            zone: false,
            group: None,
            kind: ElementKind::Text(TextProps { content: RichText::plain(&text), style, prompt: None, frame: None }),
            subgroups: Vec::new(),
            flip_h: false, flip_v: false,
        }
    };
    doc.pages[0].elements.push(block(ORIGIN_MM, "Témoin", None));
    doc.pages[0].elements.push(block(ORIGIN_MM + 90.0, "Aéré", Some(1.5)));
    let (_dir, pdf) = render(&doc, &AssetStore::default());
    let words = words(&pdf);
    let tops = |prefix: &str| -> Vec<f64> {
        (1..=5).map(|i| words.iter().find(|w| w.text == format!("{prefix}{i}")).unwrap_or_else(|| panic!("{prefix}{i} absent")).y_min).collect()
    };
    let (plain, airy) = (tops("Témoin"), tops("Aéré"));
    let pitch = |t: &[f64]| (t[4] - t[0]) / 4.0;
    let (p1, p15) = (pitch(&plain), pitch(&airy));
    assert!(p1 > 5.0, "pas témoin {p1:.2} pt");
    for w in airy.windows(2) {
        assert!((w[1] - w[0] - p15).abs() < TOLERANCE_PT, "pas irréguliers : {airy:?}");
    }
    let ratio = p15 / p1;
    assert!((ratio - 1.5).abs() < 0.03, "pas {p15:.2} pt contre {p1:.2} pt : rapport {ratio:.3}");
    // L'écran suppose que l'espace ajouté va au-dessus de chaque ligne, la première comprise (LibreOffice 26.8) :
    // dit ici sans être exigé, une autre version de LibreOffice pouvant le placer autrement.
    eprintln!("première ligne descendue de {:.2} pt (au-dessus seulement : {:.2} pt)", airy[0] - plain[0], 0.5 * p1);
}

/// Écrire sur les lignes : dans le rendu de LibreOffice, chaque ligne de base d'un texte Andika tombe sur une ligne
/// dessinée du cadre (Seyès, lignes), à 0,3 mm près, y compris loin du haut (aucun écart qui s'additionne) ; un
/// texte trop haut pour une ligne écrit une ligne sur deux. L'alignement en bas et l'interligne sont sans effet.
#[test]
#[ignore]
fn text_written_on_lines_sits_on_the_frame_lines() {
    let mut doc = Document::new("lignes");
    // (motif, taille, nombre de lignes de texte, pas attendu)
    let blocks = [(Pattern::Seyes, 14.0, 12, 8.0), (Pattern::Lines, 14.0, 6, 8.0), (Pattern::Seyes, 20.0, 6, 16.0), (Pattern::Lines, 28.0, 4, 16.0)];
    for (i, &(pattern, size_pt, n, _)) in blocks.iter().enumerate() {
        let mut style = TextStyle::default_zone();
        style.size_pt = size_pt;
        style.valign = Some(VAlign::Bottom);
        style.line_spacing = Some(1.5);
        let text = (1..=n).map(|k| format!("B{i}L{k}Hé")).collect::<Vec<_>>().join("\n");
        doc.pages[0].elements.push(Element {
            id: new_id(),
            frame: Frame { x_mm: ORIGIN_MM + (i % 2) as f64 * 90.0, y_mm: ORIGIN_MM + (i / 2) as f64 * 125.0, w_mm: 80.0, h_mm: 115.0, rotation_deg: 0.0 },
            locked: false,
            zone: false,
            group: None,
            kind: ElementKind::Text(TextProps { content: RichText::plain(&text), style, prompt: None, frame: Some(TextFrame { pattern: Some(pattern), write_on_lines: Some(true), ..TextFrame::default() }) }),
            subgroups: Vec::new(),
            flip_h: false, flip_v: false,
        });
    }
    let (_dir, pdf) = render(&doc, &AssetStore::default());
    let words = words(&pdf);
    // Ligne de base : le bas de la boîte du mot que donne poppler, moins la descendante d'Andika.
    let andika = FontSet::builtin().face("Andika", false, false).and_then(|f| f.metrics).unwrap();
    let below = andika.descent / andika.units_per_em as f64;
    let mut worst: f64 = 0.0;
    for (i, &(pattern, size_pt, n, pitch)) in blocks.iter().enumerate() {
        let f = doc.pages[0].elements[i].frame;
        let drawn: Vec<f64> = background::frame_marks(PageSize { width_mm: f.w_mm, height_mm: f.h_mm }, pattern)
            .into_iter()
            .filter_map(|m| match m {
                background::Mark::Line { y1, y2, .. } if y1 == y2 => Some(y1),
                _ => None,
            })
            .collect();
        for k in 1..=n {
            let w = words.iter().find(|w| w.text == format!("B{i}L{k}Hé")).unwrap_or_else(|| panic!("B{i}L{k}Hé absent"));
            let baseline = (w.y_max - below * size_pt) / PT_PER_MM - f.y_mm;
            let target = k as f64 * pitch;
            assert!(drawn.iter().any(|y| (y - target).abs() < 1e-6), "{pattern:?} : pas de ligne dessinée à {target} mm");
            let gap = baseline - target;
            worst = worst.max(gap.abs());
            assert!(gap.abs() < 0.3, "{pattern:?} {size_pt} pt, ligne {k} : ligne de base à {baseline:.3} mm, ligne du cadre à {target} mm");
        }
    }
    eprintln!("pire écart ligne de base / ligne du cadre : {worst:.3} mm");
}

/// Puces : dans le rendu de LibreOffice, chaque élément de liste a sa puce « • » au bord du texte, dans la police
/// du bloc ; son texte commence au retrait (`LIST_INDENT_EM` fois la taille), et les lignes suivantes d'un élément
/// long aussi ; le paragraphe sans puce du milieu reste au bord. Les lignes ont le pas d'un bloc témoin sans puce,
/// rendu dans le même PDF. Réenregistré par LibreOffice, l'ODG redonne les mêmes puces.
#[test]
#[ignore]
fn bullets_hang_their_lines_under_the_text() {
    const SIZE_PT: f64 = 14.0;
    let mut doc = Document::new("puces");
    let long: String = (1..=14).map(|i| format!("{{p}}Long{i}")).collect::<Vec<_>>().join(" ");
    let block = |x_mm: f64, p: &str, bullets: bool| {
        let mut style = TextStyle::default_zone();
        style.size_pt = SIZE_PT;
        let text = ["Avant", "Colle", &long, "Milieu", "Fin"].map(|t| if t.contains("{p}") { t.replace("{p}", p) } else { format!("{p}{t}") }).join("\n");
        let mut content = RichText::plain(&text);
        for i in [1, 2, 4] {
            content.paragraphs[i].list = bullets.then_some(ListKind::Bullet);
        }
        Element {
            id: new_id(),
            frame: Frame { x_mm, y_mm: ORIGIN_MM, w_mm: 80.0, h_mm: 120.0, rotation_deg: 0.0 },
            locked: false,
            zone: false,
            group: None,
            kind: ElementKind::Text(TextProps { content, style, prompt: None, frame: None }),
            subgroups: Vec::new(),
            flip_h: false, flip_v: false,
        }
    };
    doc.pages[0].elements.push(block(ORIGIN_MM, "L", true));
    doc.pages[0].elements.push(block(ORIGIN_MM + 90.0, "T", false));
    let (dir, pdf) = render(&doc, &AssetStore::default());
    let words = words(&pdf);
    let find = |t: &str| words.iter().find(|w| w.text == t).unwrap_or_else(|| panic!("{t} absent : {:?}", words.iter().map(|w| &w.text).collect::<Vec<_>>()));
    let left = (ORIGIN_MM + TEXT_PADDING_MM) * PT_PER_MM;
    let indent = LIST_INDENT_EM * SIZE_PT;
    // Tolérance : 0,3 mm, pour un LibreOffice plus ancien que celui des mesures (26.8 : 0,01 mm d'écart).
    let tol = 0.3 * PT_PER_MM;
    let near = |what: &str, got: f64, want: f64| assert!((got - want).abs() < tol, "{what} à {:.2} mm du bord du texte au lieu de {:.2} mm", (got - left) / PT_PER_MM, (want - left) / PT_PER_MM);
    let bullets: Vec<&Word> = words.iter().filter(|w| w.text.starts_with('\u{2022}')).collect();
    assert_eq!(bullets.len(), 3, "trois puces : {:?}", bullets.iter().map(|w| &w.text).collect::<Vec<_>>());
    for b in &bullets {
        assert_eq!(b.text, "\u{2022}", "la puce est collée au texte");
        near("la puce", b.x_min, left);
    }
    for (t, want) in [("LAvant", left), ("LColle", left + indent), ("LMilieu", left), ("LFin", left + indent)] {
        near(t, find(t).x_min, want);
    }
    // Élément long : le premier mot de chaque ligne au retrait (lignes suivantes sous le texte, pas sous la puce).
    let mut starts: Vec<&Word> = Vec::new();
    for w in words.iter().filter(|w| w.text.starts_with("LLong")) {
        match starts.iter_mut().find(|s| (s.y_min - w.y_min).abs() < 1.0) {
            Some(s) if w.x_min < s.x_min => *s = w,
            Some(_) => {}
            None => starts.push(w),
        }
    }
    assert!(starts.len() >= 2, "l'élément long tient sur une ligne : {} lignes", starts.len());
    for s in &starts {
        near(&format!("la ligne de {}", s.text), s.x_min, left + indent);
    }
    // Chaque puce sur la ligne de son élément.
    for (b, t) in bullets.iter().zip(["LColle", "LLong1", "LFin"]) {
        assert!((b.y_max - find(t).y_max).abs() < TOLERANCE_PT, "puce de {t} : bas à {:.2} pt, texte à {:.2} pt", b.y_max, find(t).y_max);
    }
    // Pas des lignes : celui du témoin, la puce ne grandit pas la ligne.
    let pitch = find("TColle").y_min - find("TAvant").y_min;
    let tops: Vec<f64> = [find("LAvant"), find("LColle")].into_iter().chain(starts.iter().copied()).chain([find("LMilieu"), find("LFin")]).map(|w| w.y_min).collect();
    assert!((tops[0] - find("TAvant").y_min).abs() < TOLERANCE_PT);
    for w in tops.windows(2) {
        assert!((w[1] - w[0] - pitch).abs() < TOLERANCE_PT, "pas {:.2} pt au lieu de {pitch:.2} pt : {tops:?}", w[1] - w[0]);
    }
    let fonts = run(Command::new("pdffonts").arg(&pdf));
    assert!(!fonts.contains("OpenSymbol"), "la puce est sortie dans une police de symboles : {fonts}");
    // Réenregistré par LibreOffice : les mêmes paragraphes à puce.
    let odg = dir.path().join("banc.odg");
    let out = dir.path().join("relu");
    std::fs::create_dir(&out).unwrap();
    let profile = format!("-env:UserInstallation=file://{}", dir.path().join("profil").display());
    run(Command::new("soffice").arg(profile).args(["--headless", "--convert-to", "odg", "--outdir"]).arg(&out).arg(&odg));
    let back = carnet_core::import::odg::read_drawing(std::fs::File::open(out.join("banc.odg")).unwrap(), &["Andika".to_string()]).unwrap();
    let lists: Vec<Vec<bool>> = back.pages[0].elements.iter().filter_map(|e| match &e.kind {
        ElementKind::Text(t) => Some(t.content.paragraphs.iter().map(|p| p.list == Some(ListKind::Bullet)).collect()),
        _ => None,
    }).collect();
    assert!(lists.contains(&vec![false, true, true, false, true]) && lists.contains(&vec![false; 5]), "relu après LibreOffice : {lists:?}");
}

// ---------------------------------------------------------------------------------------------------------------
// Rotation, miroir, tableaux et grilles, lettres creuses, formats de page : la géométrie attendue est écrite ici à
// partir de la convention de l'écran (cadre tourné dans le sens horaire autour de son centre, miroir dans le cadre
// avant la rotation), jamais recalculée avec le code de l'export.

fn element(frame: Frame, kind: ElementKind) -> Element {
    Element { id: new_id(), frame, locked: false, zone: false, group: None, kind, subgroups: Vec::new(), flip_h: false, flip_v: false }
}

fn frame_at(x_mm: f64, y_mm: f64, w_mm: f64, h_mm: f64, rotation_deg: f64) -> Frame {
    Frame { x_mm, y_mm, w_mm, h_mm, rotation_deg }
}

fn filled_shape(shape: ShapeKind, path: Option<&str>) -> ElementKind {
    ElementKind::Shape(ShapeProps { shape, fill: Some("#000000".into()), stroke: None, corner_radius_mm: 0.0, path: path.map(String::from), graphisme: None, ends: None, points: None, curve: false })
}

fn photo_kind(asset: &Id) -> ElementKind {
    ElementKind::Photo(PhotoProps { asset: Some(asset.clone()), crop: Crop::COVER, corner_radius_mm: 0.0, border: None, shadow: false, mirrored: false })
}

fn text_kind(text: &str, size_pt: f64) -> ElementKind {
    let mut style = TextStyle::default_zone();
    style.size_pt = size_pt;
    ElementKind::Text(TextProps { content: RichText::plain(text), style, prompt: None, frame: None })
}

/// Image générée ajoutée à la photothèque du document, en PNG (le JPEG bave aux limites des couleurs).
fn png_asset(doc: &mut Document, store: &mut AssetStore, img: &RgbImage) -> Id {
    let mut bytes = Cursor::new(Vec::new());
    img.write_to(&mut bytes, ImageFormat::Png).unwrap();
    let a = import_bytes(&bytes.into_inner()).unwrap();
    let id = a.meta.id.clone();
    doc.assets.insert(id.clone(), a.meta.clone());
    store.insert(a).unwrap();
    id
}

const RED: (u8, u8, u8) = (220, 30, 30);
const BLUE: (u8, u8, u8) = (30, 60, 220);
const GREEN: (u8, u8, u8) = (30, 170, 60);
const YELLOW: (u8, u8, u8) = (240, 210, 30);
const WHITE: (u8, u8, u8) = (255, 255, 255);

/// Image 2:1 en quatre quarts : rouge en haut à gauche, bleu en haut à droite, vert en bas à gauche, jaune en bas
/// à droite. Asymétrique dans les deux sens : un miroir, un retournement ou une rotation se voient.
fn quarters() -> RgbImage {
    RgbImage::from_fn(200, 100, |x, y| {
        let (r, g, b) = match (x < 100, y < 50) {
            (true, true) => RED,
            (false, true) => BLUE,
            (true, false) => GREEN,
            (false, false) => YELLOW,
        };
        Rgb([r, g, b])
    })
}

/// Page rendue, lue en millimètres.
struct Raster {
    w: usize,
    h: usize,
    px_per_mm: f64,
    channels: usize,
    px: Vec<u8>,
}

impl Raster {
    fn gray(pdf: &Path, page: usize, dpi: u32) -> Raster {
        let (w, h, px) = gray_page(pdf, page, dpi);
        Raster { w, h, px_per_mm: dpi as f64 / 25.4, channels: 1, px }
    }
    fn rgb(pdf: &Path, page: usize, dpi: u32) -> Raster {
        let (w, h, px) = rgb_page(pdf, page, dpi);
        Raster { w, h, px_per_mm: dpi as f64 / 25.4, channels: 3, px }
    }
    fn index(&self, x_mm: f64, y_mm: f64) -> usize {
        let x = ((x_mm * self.px_per_mm) as usize).min(self.w - 1);
        let y = ((y_mm * self.px_per_mm) as usize).min(self.h - 1);
        (y * self.w + x) * self.channels
    }
    /// Niveau de gris (pour une page en couleur, la moyenne des trois composantes).
    fn level(&self, x_mm: f64, y_mm: f64) -> u8 {
        let i = self.index(x_mm, y_mm);
        (self.px[i..i + self.channels].iter().map(|&v| v as u32).sum::<u32>() / self.channels as u32) as u8
    }
    fn color(&self, x_mm: f64, y_mm: f64) -> (u8, u8, u8) {
        let i = self.index(x_mm, y_mm);
        (self.px[i], self.px[i + 1], self.px[i + 2])
    }
    /// Pixels (x, y, centre en mm) d'une fenêtre en millimètres.
    fn window(&self, x0: f64, y0: f64, x1: f64, y1: f64) -> impl Iterator<Item = (usize, usize, f64, f64)> + '_ {
        let k = self.px_per_mm;
        let (px0, px1) = ((x0 * k).max(0.0) as usize, ((x1 * k) as usize).min(self.w));
        let (py0, py1) = ((y0 * k).max(0.0) as usize, ((y1 * k) as usize).min(self.h));
        (py0..py1).flat_map(move |y| (px0..px1).map(move |x| (x, y, (x as f64 + 0.5) / k, (y as f64 + 0.5) / k)))
    }
    /// Boîte de l'encre (pixels sous `dark`) dans une fenêtre, en mm : (x0, y0, x1, y1).
    fn ink_box(&self, x0: f64, y0: f64, x1: f64, y1: f64, dark: u8) -> Option<(f64, f64, f64, f64)> {
        let mut b: Option<(f64, f64, f64, f64)> = None;
        for (_, _, x, y) in self.window(x0, y0, x1, y1) {
            if self.level(x, y) < dark {
                b = Some(b.map_or((x, y, x, y), |(a, c, d, e)| (a.min(x), c.min(y), d.max(x), e.max(y))));
            }
        }
        b
    }
}

fn near(c: (u8, u8, u8), want: (u8, u8, u8)) -> bool {
    c.0.abs_diff(want.0) < 40 && c.1.abs_diff(want.1) < 40 && c.2.abs_diff(want.2) < 40
}

/// Le point (x, y) est dans le cadre tourné, agrandi de `grow` mm de chaque côté (rétréci si négatif). L'écran
/// tourne le cadre dans le sens horaire autour de son centre : on ramène le point dans le repère du cadre.
fn in_turned_frame(f: &Frame, x: f64, y: f64, grow: f64) -> bool {
    let (dx, dy) = (x - (f.x_mm + f.w_mm / 2.0), y - (f.y_mm + f.h_mm / 2.0));
    let (sin, cos) = f.rotation_deg.to_radians().sin_cos();
    let (lx, ly) = (dx * cos + dy * sin, -dx * sin + dy * cos);
    lx.abs() <= f.w_mm / 2.0 + grow && ly.abs() <= f.h_mm / 2.0 + grow
}

/// Point du cadre donné dans son repère (depuis son coin haut gauche, avant rotation), placé sur la page.
fn turned_point(f: &Frame, lx: f64, ly: f64) -> (f64, f64) {
    let (dx, dy) = (lx - f.w_mm / 2.0, ly - f.h_mm / 2.0);
    let (sin, cos) = f.rotation_deg.to_radians().sin_cos();
    (f.x_mm + f.w_mm / 2.0 + dx * cos - dy * sin, f.y_mm + f.h_mm / 2.0 + dx * sin + dy * cos)
}

/// Photo 60 × 30 mm tournée de 90°, faite d'une moitié gauche rouge et d'une moitié droite bleue : le rouge en haut,
/// le bleu en bas, et rien à côté de la boîte tournée (30 mm de large).
fn check_turned_halves(r: &Raster, name: &str, f: &Frame) {
    for (lx, want) in [(10.0, RED), (50.0, BLUE)] {
        let (x, y) = turned_point(f, lx, 15.0);
        assert!(near(r.color(x, y), want), "{name} : {:?} à ({x:.0}, {y:.0}) au lieu de {want:?}", r.color(x, y));
    }
    let (cx, cy) = turned_point(f, 30.0, 15.0);
    for x in [cx - 17.0, cx + 17.0] {
        assert!(near(r.color(x, cy), WHITE), "{name} : encre à ({x:.0}, {cy:.0}), hors de la boîte tournée : {:?}", r.color(x, cy));
    }
}

fn halves_asset(doc: &mut Document, store: &mut AssetStore) -> Id {
    let halves = RgbImage::from_fn(200, 100, |x, _| if x < 100 { Rgb([RED.0, RED.1, RED.2]) } else { Rgb([BLUE.0, BLUE.1, BLUE.2]) });
    png_asset(doc, store, &halves)
}

/// Rotation : rectangles pleins tournés de 30° et de 90°, comparés pixel à pixel à la boîte tournée attendue ; photo
/// tournée de 90°, dont le côté gauche doit monter ;
/// trait tourné de 30° (ses extrémités sont calculées à part) ; texte tourné de 90°, qui doit se lire de haut en
/// bas ; lignes d'un cadre d'écriture tourné, verticales et du bon côté. Un 90° de plein ne distingue pas le sens :
/// le 30°, la photo, le trait et le texte, si.
#[test]
#[ignore]
fn rotated_elements_fill_their_turned_box() {
    let mut store = AssetStore::default();
    let mut doc = Document::new("rotation");
    doc.pages.push(Page::new(A4_PORTRAIT));
    let asset = halves_asset(&mut doc, &mut store);
    let rects = [frame_at(20.0, 40.0, 80.0, 30.0, 30.0), frame_at(110.0, 40.0, 80.0, 30.0, 90.0)];
    for f in rects {
        doc.pages[0].elements.push(element(f, filled_shape(ShapeKind::Rect, None)));
    }
    let photo = frame_at(110.0, 120.0, 60.0, 30.0, 90.0);
    doc.pages[0].elements.push(element(photo, photo_kind(&asset)));
    let line = frame_at(20.0, 200.0, 60.0, 4.0, 30.0);
    doc.pages[0].elements.push(element(
        line,
        ElementKind::Shape(ShapeProps { shape: ShapeKind::Line, fill: None, stroke: Some(Stroke { width_mm: 1.0, color: "#000000".into(), style: None }), corner_radius_mm: 0.0, path: None, graphisme: None, ends: None, points: None, curve: false }),
    ));
    let text = frame_at(10.0, 90.0, 100.0, 20.0, 90.0);
    doc.pages[1].elements.push(element(text, text_kind("Tourné", 14.0)));
    let lined = frame_at(110.0, 90.0, 100.0, 30.0, 90.0);
    doc.pages[1].elements.push(element(
        lined,
        ElementKind::Text(TextProps { content: RichText::default(), style: TextStyle::default_zone(), prompt: None, frame: Some(TextFrame { pattern: Some(Pattern::Lines), ..TextFrame::default() }) }),
    ));
    let (_dir, pdf) = render(&doc, &store);

    // Rectangles : à 5 pixels par millimètre, 0,4 mm de chaque côté du bord n'est pas jugé (lissage, et écarts
    // d'arrondi entre versions de LibreOffice) ; ailleurs, au plus 0,5 % de pixels faux. Un sens de rotation
    // inversé à 30° en fausse près de 40 %, un rectangle resté droit plus de 50 %.
    let r = Raster::rgb(&pdf, 1, 127);
    for f in rects {
        let (mut wrong, mut judged) = (0, 0);
        // Fenêtre : la boîte tournée attendue, plus 5 mm (sans toucher l'autre rectangle).
        let corners = [(0.0, 0.0), (f.w_mm, 0.0), (0.0, f.h_mm), (f.w_mm, f.h_mm)].map(|(lx, ly)| turned_point(&f, lx, ly));
        let (x0, x1) = corners.iter().fold((f64::MAX, f64::MIN), |(a, b), p| (a.min(p.0), b.max(p.0)));
        let (y0, y1) = corners.iter().fold((f64::MAX, f64::MIN), |(a, b), p| (a.min(p.1), b.max(p.1)));
        for (_, _, x, y) in r.window(x0 - 5.0, y0 - 5.0, x1 + 5.0, y1 + 5.0) {
            let (inside, outside) = (in_turned_frame(&f, x, y, -0.4), !in_turned_frame(&f, x, y, 0.4));
            if !inside && !outside {
                continue;
            }
            judged += 1;
            if inside != (r.level(x, y) < 128) {
                wrong += 1;
            }
        }
        let ratio = wrong as f64 / (f.w_mm * f.h_mm * r.px_per_mm * r.px_per_mm);
        assert!(judged > 0 && ratio < 0.005, "rectangle tourné de {}° : {:.1} % de pixels hors de la boîte attendue", f.rotation_deg, ratio * 100.0);
    }
    // Photo à 90° : le côté gauche (rouge) en haut, le droit (bleu) en bas, et rien à côté de la boîte tournée.
    check_turned_halves(&r, "photo", &photo);
    // Trait à 30° : son bout droit descend. Encre sur le trait, rien sur son symétrique (sens inversé).
    for lx in [10.0, 50.0] {
        let (x, y) = turned_point(&line, lx, 2.0);
        let dark = (-2..=2).any(|d| r.level(x, y + d as f64 * 0.2) < 128);
        assert!(dark, "trait tourné : pas d'encre à ({x:.1}, {y:.1})");
        let (mx, my) = turned_point(&frame_at(line.x_mm, line.y_mm, line.w_mm, line.h_mm, -30.0), lx, 2.0);
        assert!(r.level(mx, my) > 200, "trait tourné dans le mauvais sens : encre à ({mx:.1}, {my:.1})");
    }

    // Texte tourné de 90° dans le sens horaire : le début du texte (bord gauche du cadre) en haut, sa première
    // ligne (bord haut du cadre) à droite. Tolérance de 1 mm : la boîte d'un mot dépend un peu de la version.
    let words = words(&pdf);
    let w = words.iter().find(|w| w.page == 1 && w.text == "Tourné").expect("texte tourné rendu");
    let mm = |v: f64| v / PT_PER_MM;
    let (cx, cy) = (text.x_mm + text.w_mm / 2.0, text.y_mm + text.h_mm / 2.0);
    let pad = TEXT_PADDING_MM;
    assert!(w.y_max - w.y_min > 2.0 * (w.x_max - w.x_min), "texte tourné couché : {:.1} × {:.1} pt", w.x_max - w.x_min, w.y_max - w.y_min);
    assert!((mm(w.y_min) - (cy - text.w_mm / 2.0 + pad)).abs() < 1.0, "début du texte tourné à {:.1} mm", mm(w.y_min));
    assert!((mm(w.x_max) - (cx + text.h_mm / 2.0 - pad)).abs() < 1.0, "haut du texte tourné à {:.1} mm", mm(w.x_max));
    assert!(mm(w.x_min) > cx, "texte tourné du mauvais côté : {:.1} mm", mm(w.x_min));

    // Lignes d'écriture du cadre tourné : chaque ligne (horizontale à `y` dans le cadre) devient verticale, à
    // `centre + h/2 - y`. On regarde sur 40 mm au milieu, à 0,6 mm près de chaque côté (traits fins et gris clair).
    let r2 = Raster::gray(&pdf, 2, 254);
    let (lcx, lcy) = (lined.x_mm + lined.w_mm / 2.0, lined.y_mm + lined.h_mm / 2.0);
    let marks: Vec<f64> = background::frame_marks(PageSize { width_mm: lined.w_mm, height_mm: lined.h_mm }, Pattern::Lines)
        .into_iter()
        .filter_map(|m| match m {
            background::Mark::Line { y1, y2, .. } if y1 == y2 => Some(y1),
            _ => None,
        })
        .collect();
    assert!(marks.len() >= 3, "{marks:?}");
    for y in &marks {
        let x = lcx + lined.h_mm / 2.0 - y;
        for k in 0..40 {
            let yy = lcy - 20.0 + k as f64;
            let inked = (-6..=6).any(|d| r2.level(x + d as f64 * 0.1, yy) < 235);
            assert!(inked, "ligne du cadre tourné absente à x = {x:.1} mm (y = {yy:.0} mm)");
        }
    }
    // Entre deux lignes, rien : des lignes restées horizontales encreraient tout le milieu.
    let between = lcx + lined.h_mm / 2.0 - (marks[0] + marks[1]) / 2.0;
    assert!((0..40).all(|k| r2.level(between, lcy - 20.0 + k as f64) > 235), "encre entre les lignes du cadre tourné, à x = {between:.1} mm");
}

/// Triangle dans la moitié gauche de son cadre : retourné de gauche à droite, il passe à droite.
const TRIANGLE: &str = "M 0 0 L 500 0 L 0 1000 Z";

/// Miroir : une photo en miroir (`PhotoProps::mirrored`), une retournée de haut en bas, un tracé asymétrique
/// retourné, une image OpenMoji retournée, comparés à leur témoin à l'endroit. Avec
/// le recadrage d'une photo (agrandie deux fois sur son quart haut gauche), ses coins arrondis et son ombre.
#[test]
#[ignore]
fn mirrored_and_cropped_elements_are_rendered() {
    let mut store = AssetStore::default();
    let mut doc = Document::new("miroir");
    let asset = png_asset(&mut doc, &mut store, &quarters());
    let photo = |doc: &mut Document, x: f64, y: f64, edit: &dyn Fn(&mut Element, &mut PhotoProps)| {
        let mut e = element(frame_at(x, y, 60.0, 30.0, 0.0), photo_kind(&asset));
        let ElementKind::Photo(mut p) = e.kind.clone() else { unreachable!() };
        edit(&mut e, &mut p);
        e.kind = ElementKind::Photo(p);
        doc.pages[0].elements.push(e);
    };
    photo(&mut doc, 20.0, 20.0, &|_, _| {});
    photo(&mut doc, 100.0, 20.0, &|_, p| p.mirrored = true);
    photo(&mut doc, 20.0, 60.0, &|e, _| e.flip_v = true);
    photo(&mut doc, 100.0, 60.0, &|_, p| p.crop = Crop { scale: 2.0, center_x: 0.25, center_y: 0.25 });
    photo(&mut doc, 20.0, 100.0, &|_, p| {
        p.corner_radius_mm = 8.0;
        p.shadow = true;
    });
    let tri = [frame_at(100.0, 100.0, 40.0, 40.0, 0.0), frame_at(150.0, 100.0, 40.0, 40.0, 0.0)];
    for (i, f) in tri.iter().enumerate() {
        doc.pages[0].elements.push(Element { flip_h: i > 0, ..element(*f, filled_shape(ShapeKind::Path, Some(TRIANGLE))) });
    }
    // Flèche vers la droite (27A1) : une image nettement asymétrique.
    doc.pages[0].elements.push(picture_element("27A1", 20.0, None));
    doc.pages[0].elements.last_mut().unwrap().frame.y_mm = 200.0;
    doc.pages[0].elements.push(Element { flip_h: true, ..picture_element("27A1", 100.0, None) });
    doc.pages[0].elements.last_mut().unwrap().frame.y_mm = 200.0;
    let (_dir, pdf) = render(&doc, &store);
    let r = Raster::rgb(&pdf, 1, 127);

    // Couleurs relevées au milieu de chaque quart, loin des bords (lissage, recadrage arrondi au pixel).
    let quarter = |x: f64, y: f64| [r.color(x + 15.0, y + 7.5), r.color(x + 45.0, y + 7.5), r.color(x + 15.0, y + 22.5), r.color(x + 45.0, y + 22.5)];
    let check = |name: &str, got: [(u8, u8, u8); 4], want: [(u8, u8, u8); 4]| {
        assert!(got.iter().zip(&want).all(|(g, w)| near(*g, *w)), "{name} : {got:?} au lieu de {want:?}");
    };
    check("photo à l'endroit", quarter(20.0, 20.0), [RED, BLUE, GREEN, YELLOW]);
    check("photo en miroir", quarter(100.0, 20.0), [BLUE, RED, YELLOW, GREEN]);
    check("photo retournée", quarter(20.0, 60.0), [GREEN, YELLOW, RED, BLUE]);
    check("photo recadrée sur son quart haut gauche", quarter(100.0, 60.0), [RED; 4]);
    // Coins arrondis de 8 mm : le coin reste blanc, le bord plus loin est la photo ; ombre décalée de 1 mm, gris.
    assert!(near(r.color(20.8, 100.8), WHITE), "coin arrondi de la photo : {:?}", r.color(20.8, 100.8));
    assert!(near(r.color(30.0, 100.5), RED), "bord haut de la photo arrondie : {:?}", r.color(30.0, 100.5));
    let s = r.color(80.6, 115.0);
    assert!(s.0 == s.1 && s.1 == s.2 && (130..235).contains(&s.0), "ombre de la photo : {s:?}");
    assert!(near(r.color(81.5, 115.0), WHITE), "ombre trop large : {:?}", r.color(81.5, 115.0));

    // Triangles : encre près du coin du côté plein, rien près du coin opposé.
    let dark = |p: (f64, f64)| r.level(p.0, p.1) < 128;
    for (i, f) in tri.iter().enumerate() {
        let (full, empty) = if i == 0 { ((5.0, 5.0), (35.0, 5.0)) } else { ((35.0, 5.0), (5.0, 5.0)) };
        let (a, b) = (turned_point(f, full.0, full.1), turned_point(f, empty.0, empty.1));
        assert!(dark(a) && !dark(b), "triangle {i} (miroir : {}) : encre {} à {a:?}, {} à {b:?}", i > 0, dark(a), dark(b));
    }

    // Image retournée : c'est le reflet de son témoin, qui lui-même ne se ressemble pas en miroir.
    let g = Raster::gray(&pdf, 1, 127);
    let k = g.px_per_mm;
    let n = (50.0 * k) as usize;
    let (x0, x1, y0) = ((20.0 * k) as usize, (100.0 * k) as usize, (200.0 * k) as usize);
    let at = |x: usize, y: usize| g.px[y * g.w + x] as f64;
    let mean = |f: &dyn Fn(usize, usize) -> f64| (0..n).flat_map(|y| (0..n).map(move |x| (x, y))).map(|(x, y)| f(x, y)).sum::<f64>() / (n * n) as f64;
    let asymmetry = mean(&|x, y| (at(x0 + x, y0 + y) - at(x0 + n - 1 - x, y0 + y)).abs());
    assert!(asymmetry > 10.0, "l'image de test est trop symétrique pour juger : {asymmetry:.1}");
    let reflected = mean(&|x, y| (at(x0 + x, y0 + y) - at(x1 + n - 1 - x, y0 + y)).abs());
    assert!(reflected < asymmetry / 4.0, "image retournée : écart {reflected:.1} au reflet du témoin, quand le témoin s'écarte de {asymmetry:.1} de son reflet");
}

/// Tableau aux colonnes (2, 1, 1) et lignes (1, 2, 1) inégales, deux cases fusionnées (deux colonnes en haut à
/// gauche, deux lignes à droite) : chaque trait au millimètre attendu, aucun trait à travers une case fusionnée, le
/// texte de chaque case en son milieu. Puis une grille de mise en page « une en haut, deux en bas » remplie de
/// photos : chaque couleur dans sa case, les marges et les espaces blancs.
#[test]
#[ignore]
fn tables_and_grids_are_rendered_in_place() {
    let mut doc = Document::new("tableau");
    let page = &mut doc.pages[0];
    let table_id = carnet_core::table::add_table(page, frame_at(30.0, 40.0, 150.0, 120.0, 0.0), 3, 3);
    let cell = |page: &Page, col: u8, row: u8| -> Id {
        let t = page.elements.iter().find(|e| e.id == table_id).unwrap();
        let ElementKind::Table(t) = &t.kind else { unreachable!() };
        t.cells.iter().find(|c| c.col == col && c.row == row).unwrap().element_id.clone()
    };
    if let ElementKind::Table(t) = &mut page.elements.iter_mut().find(|e| e.id == table_id).unwrap().kind {
        t.cols = vec![2.0, 1.0, 1.0];
        t.rows = vec![1.0, 2.0, 1.0];
        t.border = Some(Stroke { width_mm: 1.0, color: "#000000".into(), style: None });
    }
    let (a, b) = (cell(page, 0, 0), cell(page, 1, 0));
    carnet_core::table::merge(page, &a, &b).unwrap();
    let (a, b) = (cell(page, 2, 1), cell(page, 2, 2));
    carnet_core::table::merge(page, &a, &b).unwrap();
    carnet_core::table::layout(page, &table_id);
    // Bords attendus : colonnes 30 | 105 | 142,5 | 180, lignes 40 | 70 | 130 | 160.
    let cells = [
        ("Alpha", (30.0, 40.0, 142.5, 70.0), (0, 0)),
        ("Bravo", (142.5, 40.0, 180.0, 70.0), (2, 0)),
        ("Delta", (30.0, 70.0, 105.0, 130.0), (0, 1)),
        ("Echo", (105.0, 70.0, 142.5, 130.0), (1, 1)),
        ("Golf", (142.5, 70.0, 180.0, 160.0), (2, 1)),
        ("Hotel", (30.0, 130.0, 105.0, 160.0), (0, 2)),
        ("Kilo", (105.0, 130.0, 142.5, 160.0), (1, 2)),
    ];
    for (word, _, (col, row)) in cells {
        let id = cell(page, col, row);
        let ElementKind::Text(t) = &mut page.elements.iter_mut().find(|e| e.id == id).unwrap().kind else { panic!("case {col},{row} sans texte") };
        t.content = RichText::plain(word);
    }

    // Grille sur une seconde page, remplie de photos unies.
    let mut store = AssetStore::default();
    let mut grid_page = Page::new(A4_PORTRAIT);
    carnet_core::grid::apply_preset(&mut grid_page, carnet_core::grid::GridPreset::OnePlusTwo);
    let zones: Vec<(u8, u8, String)> = grid_page.grid.as_ref().unwrap().cells.iter().map(|c| (c.col, c.row, c.element_id.clone())).collect();
    assert_eq!(zones.len(), 3);
    for (col, row, id) in zones {
        let c = match (col, row) {
            (0, 0) => RED,
            (0, 1) => GREEN,
            _ => BLUE,
        };
        let asset = png_asset(&mut doc, &mut store, &RgbImage::from_pixel(40, 40, Rgb([c.0, c.1, c.2])));
        let e = grid_page.elements.iter_mut().find(|e| e.id == id).unwrap();
        let ElementKind::Photo(p) = &mut e.kind else { panic!("zone sans photo") };
        p.asset = Some(asset);
    }
    doc.pages.push(grid_page);
    let (dir, pdf) = render(&doc, &store);
    // Un tableau simple : un vrai tableau de LibreOffice, sans traits à part.
    let content = odg_content(&dir.path().join("banc.odg"));
    assert!(content.contains("<table:table>") && !content.contains("<draw:line"), "vrai tableau attendu");

    // Traits de 1 mm : à 10 pixels par millimètre, le centre de l'encre sur une bande de ±1,5 mm en travers du trait
    // doit tomber à 0,3 mm du bord attendu ; « pas de trait » : rien de sombre sur cette bande.
    let r = Raster::gray(&pdf, 1, 254);
    let across = |vertical: bool, at: f64, along: f64| -> Option<f64> {
        let pts: Vec<f64> = (-15..=15)
            .map(|d| at + d as f64 * 0.1)
            .filter(|&v| if vertical { r.level(v, along) < 128 } else { r.level(along, v) < 128 })
            .collect();
        (!pts.is_empty()).then(|| pts.iter().sum::<f64>() / pts.len() as f64)
    };
    let lines = [
        (true, 30.0, vec![55.0, 100.0, 145.0]),
        (true, 105.0, vec![100.0, 145.0]),
        (true, 142.5, vec![55.0, 100.0, 145.0]),
        (true, 180.0, vec![55.0, 100.0, 145.0]),
        (false, 40.0, vec![60.0, 160.0]),
        (false, 70.0, vec![60.0, 123.75, 161.25]),
        (false, 130.0, vec![67.5, 123.75]),
        (false, 160.0, vec![67.5, 123.75, 161.25]),
    ];
    for (vertical, at, alongs) in &lines {
        for &along in alongs {
            let got = across(*vertical, *at, along);
            assert!(got.is_some_and(|g| (g - at).abs() < 0.3), "trait {} à {at} mm, relevé à {along} mm : {got:?}", if *vertical { "vertical" } else { "horizontal" });
        }
    }
    // Séparations absentes dans les cases fusionnées : x = 105 dans la case du haut, y = 130 dans celle de droite.
    assert_eq!(across(true, 105.0, 48.0), None, "trait à travers la case fusionnée du haut");
    assert_eq!(across(false, 130.0, 170.0), None, "trait à travers la case fusionnée de droite");
    // Texte au milieu de sa case (centré, au milieu en hauteur), à 1,5 mm près.
    let words = words(&pdf);
    for (word, (x0, y0, x1, y1), _) in cells {
        let w = words.iter().find(|w| w.page == 0 && w.text == word).unwrap_or_else(|| panic!("{word} absent"));
        let (cx, cy) = ((w.x_min + w.x_max) / 2.0 / PT_PER_MM, (w.y_min + w.y_max) / 2.0 / PT_PER_MM);
        assert!((cx - (x0 + x1) / 2.0).abs() < 1.5 && (cy - (y0 + y1) / 2.0).abs() < 1.5, "{word} au milieu ({cx:.1}, {cy:.1}) au lieu de ({:.1}, {:.1})", (x0 + x1) / 2.0, (y0 + y1) / 2.0);
    }

    // Grille : marges de 10 mm, espace de 4 mm ; cases du haut 10..200 × 10..146,5, du bas 10..103 et 107..200 ×
    // 150,5..287. Points relevés à 1 mm des bords.
    let g = Raster::rgb(&pdf, 2, 127);
    let points = [
        ((11.0, 11.0), RED),
        ((199.0, 145.5), RED),
        ((11.0, 151.5), GREEN),
        ((102.0, 286.0), GREEN),
        ((108.0, 151.5), BLUE),
        ((199.0, 286.0), BLUE),
        ((105.0, 220.0), WHITE),
        ((56.0, 148.5), WHITE),
        ((153.0, 148.5), WHITE),
        ((5.0, 100.0), WHITE),
        ((205.0, 100.0), WHITE),
        ((105.0, 292.0), WHITE),
    ];
    for ((x, y), want) in points {
        assert!(near(g.color(x, y), want), "grille : {:?} à ({x}, {y}) au lieu de {want:?}", g.color(x, y));
    }
}

/// Lettres creuses : « IIII » en 72 pt, plein (témoin) et en contour, sur la même page. Le contour a la même boîte
/// d'encre que le témoin (décalée du cadre), un intérieur clair là où le témoin est plein, et deux bords encrés par
/// jambage. Avec un champ de la classe non rempli, qui sort blanc.
#[test]
#[ignore]
fn outlined_letters_and_unfilled_fields_are_rendered() {
    let mut doc = Document::new("creuses");
    let (filled, hollow) = (frame_at(20.0, 20.0, 170.0, 50.0, 0.0), frame_at(20.0, 100.0, 170.0, 50.0, 0.0));
    doc.pages[0].elements.push(element(filled, text_kind("IIII", 72.0)));
    let mut kind = text_kind("IIII", 72.0);
    if let ElementKind::Text(t) = &mut kind {
        t.style.outline = Some(true);
    }
    doc.pages[0].elements.push(element(hollow, kind));
    let field = frame_at(20.0, 180.0, 120.0, 30.0, 0.0);
    doc.pages[0].elements.push(element(field, text_kind("Bonjour\n{prenom}", 20.0)));
    let (_dir, pdf) = render(&doc, &AssetStore::default());
    let r = Raster::gray(&pdf, 1, 254);
    let dy = hollow.y_mm - filled.y_mm;

    // Seuils larges : le contour est un trait fin, que le LibreOffice de la CI peut dessiner plus fin encore.
    let solid = r.ink_box(filled.x_mm, filled.y_mm, filled.x_mm + filled.w_mm, filled.y_mm + filled.h_mm, 160).expect("témoin rendu");
    let outline = r.ink_box(hollow.x_mm, hollow.y_mm, hollow.x_mm + hollow.w_mm, hollow.y_mm + hollow.h_mm, 200).expect("lettres creuses absentes");
    let gaps = [outline.0 - solid.0, outline.1 - dy - solid.1, outline.2 - solid.2, outline.3 - dy - solid.3];
    assert!(gaps.iter().all(|g| g.abs() < 0.6), "boîte des lettres creuses {outline:?}, du témoin {solid:?} (écarts {gaps:?})");
    // Rangée du milieu : suites de pixels sombres (jambages) du témoin, puis la même rangée des lettres creuses.
    let y = (solid.1 + solid.3) / 2.0;
    let runs = |y: f64, dark: u8| -> Vec<(f64, f64)> {
        let mut out: Vec<(f64, f64)> = vec![];
        let mut start = None;
        for (_, _, x, _) in r.window(solid.0 - 2.0, y, solid.2 + 2.0, y + 0.05) {
            match (r.level(x, y) < dark, start) {
                (true, None) => start = Some(x),
                (false, Some(s)) => {
                    out.push((s, x));
                    start = None;
                }
                _ => {}
            }
        }
        out
    };
    let stems = runs(y, 160);
    assert!(stems.len() == 4 && stems.iter().all(|(a, b)| b - a > 1.5), "jambages du témoin : {stems:?}");
    let edges = runs(y + dy, 200);
    assert_eq!(edges.len(), 8, "bords des lettres creuses : {edges:?} (jambages du témoin {stems:?})");
    for (a, b) in &stems {
        let mid = (a + b) / 2.0;
        assert!(r.level(mid, y + dy) > 200, "intérieur encré à {mid:.1} mm : {}", r.level(mid, y + dy));
        assert!(edges.iter().filter(|(s, e)| *e > a - 0.5 && *s < b + 0.5).count() == 2, "deux bords attendus autour du jambage {a:.1}..{b:.1} : {edges:?}");
    }

    // Champ non rempli : « Bonjour » écrit, la ligne du champ sans encre.
    let words = words(&pdf);
    let hello = words.iter().find(|w| w.text == "Bonjour").expect("Bonjour absent");
    let (top, line) = (hello.y_max / PT_PER_MM, (hello.y_max - hello.y_min) / PT_PER_MM);
    assert!(r.ink_box(field.x_mm, top + 0.3, field.x_mm + field.w_mm, top + line, 200).is_none(), "le champ non rempli a laissé de l'encre");
}

fn corner_square_page(w: f64, h: f64) -> Page {
    let mut page = Page::new(PageSize { width_mm: w, height_mm: h });
    page.elements.push(element(frame_at(w - 25.0, h - 25.0, 10.0, 10.0, 0.0), filled_shape(ShapeKind::Rect, None)));
    page
}

/// Taille de la page `n` du PDF, en mm.
fn pdf_page_mm(pdf: &Path, n: usize) -> (f64, f64) {
    let info = run(Command::new("pdfinfo").args(["-f", &n.to_string(), "-l", &n.to_string()]).arg(pdf));
    let line = info.lines().find(|l| l.starts_with(&format!("Page {n:>4} size:"))).unwrap_or_else(|| panic!("page {n} absente :\n{info}"));
    let nums: Vec<f64> = line.split_whitespace().filter_map(|t| t.parse().ok()).collect();
    (nums[1] / PT_PER_MM, nums[2] / PT_PER_MM)
}

/// Formats A5 et A3, portrait et paysage : la page du PDF a la bonne taille (à 0,5 mm près), et un carré noir posé
/// à 15 mm du coin bas droit y tombe, à 0,5 mm près (5 pixels par millimètre). Un fichier par format : LibreOffice
/// Draw ne connaît qu'une taille de page par document (voir `defaut_connu_mixed_page_sizes_keep_their_size`).
#[test]
#[ignore]
fn a5_and_a3_pages_keep_their_size() {
    for (w, h) in [(148.0, 210.0), (210.0, 148.0), (297.0, 420.0), (420.0, 297.0)] {
        let mut doc = Document::new("format");
        doc.pages = vec![corner_square_page(w, h)];
        let (_dir, pdf) = render(&doc, &AssetStore::default());
        let (pw, ph) = pdf_page_mm(&pdf, 1);
        assert!((pw - w).abs() < 0.5 && (ph - h).abs() < 0.5, "{w} × {h} : page de {pw:.1} × {ph:.1} mm");
        let r = Raster::gray(&pdf, 1, 127);
        let ink = r.ink_box(0.0, 0.0, w, h, 128).unwrap_or_else(|| panic!("{w} × {h} : page sans encre"));
        let want = (w - 25.0, h - 25.0, w - 15.0, h - 15.0);
        let gaps = [ink.0 - want.0, ink.1 - want.1, ink.2 - want.2, ink.3 - want.3];
        assert!(gaps.iter().all(|g| g.abs() < 0.5), "{w} × {h} : carré à {ink:?} au lieu de {want:?}");
    }
}

/// Mention de licence OpenMoji, sur une page A4 et une page A5 paysage qui ont une image : toute son encre reste à
/// 5 mm ou plus du bas de la page, hors de la bande que beaucoup d'imprimantes n'atteignent pas, et sur la page.
#[test]
#[ignore]
fn openmoji_credit_ink_stays_5mm_from_the_bottom() {
    for (w, h) in [(210.0, 297.0), (210.0, 148.0)] {
        let mut doc = Document::new("mention");
        doc.pages = vec![Page::new(PageSize { width_mm: w, height_mm: h })];
        doc.pages[0].elements.push(picture_element("1F98A", ORIGIN_MM, None));
        let (_dir, pdf) = render(&doc, &AssetStore::default());
        let r = Raster::gray(&pdf, 1, 254);
        let ink = r.ink_box(0.0, h - 20.0, w, h, 230).unwrap_or_else(|| panic!("{w} × {h} : mention absente"));
        assert!(h - ink.3 >= 5.0, "{w} × {h} : encre de la mention à {:.2} mm du bas ({ink:?})", h - ink.3);
    }
}

/// Photo à coins arrondis, à bordure et ombre, et image de fond d'un cadre de texte, tournées : l'image tourne avec
/// la forme. LibreOffice étire l'image qui remplit une forme sur la boîte droite qui entoure la forme tournée, sans la
/// tourner : l'export l'y pose déjà tournée. À 90° (photo arrondie, cadre de texte), le rouge en haut, le bleu en bas,
/// rien à côté de la boîte tournée. À 30° (photo à bordure et ombre, cadre de texte, coins de 6 mm), l'image a une
/// bande rouge sur son premier douzième (5 mm) : le long du milieu du cadre tourné, le rouge s'arrête à 5 mm du bout,
/// à 0,5 mm près (une boîte droite mal calculée, sans les coins arrondis, l'étire de près de 2 mm), puis le bleu
/// jusqu'à l'autre bout.
#[test]
#[ignore]
fn turned_rounded_photo_and_frame_image_turn_their_image() {
    let mut store = AssetStore::default();
    let mut doc = Document::new("photo arrondie tournée");
    let halves = halves_asset(&mut doc, &mut store);
    let band = RgbImage::from_fn(240, 120, |x, _| if x < 20 { Rgb([RED.0, RED.1, RED.2]) } else { Rgb([BLUE.0, BLUE.1, BLUE.2]) });
    let band = png_asset(&mut doc, &mut store, &band);
    let photo = |f: Frame, asset: &Id, radius: f64, decorated: bool| {
        let mut kind = photo_kind(asset);
        if let ElementKind::Photo(p) = &mut kind {
            p.corner_radius_mm = radius;
            if decorated {
                p.border = Some(Stroke { width_mm: 0.8, color: "#000000".into(), style: Some(StrokeStyle::Dotted) });
                p.shadow = true;
            }
        }
        element(f, kind)
    };
    let label = |f: Frame, asset: &Id, radius: f64| {
        let frame = TextFrame { image: Some(asset.clone()), corner_radius_mm: radius, ..TextFrame::default() };
        element(f, ElementKind::Text(TextProps { content: RichText::default(), style: TextStyle::default_zone(), prompt: None, frame: Some(frame) }))
    };
    let upright = [frame_at(30.0, 40.0, 60.0, 30.0, 90.0), frame_at(110.0, 40.0, 60.0, 30.0, 90.0)];
    let slanted = [frame_at(20.0, 160.0, 60.0, 30.0, 30.0), frame_at(110.0, 160.0, 60.0, 30.0, 30.0)];
    doc.pages[0].elements.push(photo(upright[0], &halves, 3.0, false));
    doc.pages[0].elements.push(label(upright[1], &halves, 0.0));
    doc.pages[0].elements.push(photo(slanted[0], &band, 6.0, true));
    doc.pages[0].elements.push(label(slanted[1], &band, 6.0));
    let (_dir, pdf) = render(&doc, &store);
    let r = Raster::rgb(&pdf, 1, 254);
    check_turned_halves(&r, "photo à coins arrondis", &upright[0]);
    check_turned_halves(&r, "image de fond d'un cadre de texte", &upright[1]);
    for (f, name) in slanted.iter().zip(["photo arrondie, à bordure et ombre", "cadre de texte arrondi"]) {
        let at = |lx: f64| {
            let (x, y) = turned_point(f, lx, 15.0);
            r.color(x, y)
        };
        let end = (10..150).map(|k| k as f64 * 0.1).find(|&lx| !near(at(lx), RED));
        assert!(end.is_some_and(|e| (e - 5.0).abs() < 0.5), "{name} tourné de 30° : le rouge s'arrête à {end:?} mm du bout au lieu de 5");
        for lx in [7.0, 30.0, 58.0] {
            assert!(near(at(lx), BLUE), "{name} tourné de 30° : {:?} à {lx} mm du bout au lieu du bleu", at(lx));
        }
    }
}

/// MIS DE CÔTÉ (limite de LibreOffice Draw, pas un défaut de l'export ; ne passe pas) : un document aux formats
/// mêlés (une page A5 portrait, puis A5 paysage, A3 portrait, A3 paysage) sort de LibreOffice avec toutes ses pages
/// au format de la première. L'ODG déclare bien une mise en page par format (`style:page-layout` de chaque page
/// maîtresse), mais LibreOffice Draw n'en garde qu'une au rendu : un ODG écrit par LibreOffice lui-même (import d'un
/// PDF A5 portrait puis A5 paysage, deux pages maîtresses de tailles différentes) sort aussi en deux pages A5
/// portrait (LibreOffice 26.8). Pas de correctif dans l'export. Lancé seulement avec `BANC_DEFAUTS_CONNUS=1`.
#[test]
#[ignore]
fn defaut_connu_mixed_page_sizes_keep_their_size() {
    if std::env::var_os("BANC_DEFAUTS_CONNUS").is_none() {
        eprintln!("mis de côté : LibreOffice Draw donne à toutes les pages le format de la première");
        return;
    }
    let sizes = [(148.0, 210.0), (210.0, 148.0), (297.0, 420.0), (420.0, 297.0)];
    let mut doc = Document::new("formats");
    doc.pages = sizes.iter().map(|&(w, h)| corner_square_page(w, h)).collect();
    let (_dir, pdf) = render(&doc, &AssetStore::default());
    for (i, (w, h)) in sizes.iter().enumerate() {
        let (pw, ph) = pdf_page_mm(&pdf, i + 1);
        assert!((pw - w).abs() < 0.5 && (ph - h).abs() < 0.5, "page {} : {pw:.1} × {ph:.1} mm au lieu de {w} × {h}", i + 1);
    }
}

/// Forme dessinée retournée d'un seul sens (gauche-droite, ou haut-bas), ou des deux, puis tournée de 30° : elle
/// garde le sens de l'écran, qui retourne la forme dans son cadre avant de la tourner (LibreOffice retourne après la
/// rotation : l'export écrit l'angle opposé quand un seul miroir est mis). Puis le fichier enregistré par
/// LibreOffice se relit dans Cahutin avec les mêmes miroirs, angles et places. Les photos (pixels retournés) et les
/// images OpenMoji (SVG retourné) ne passent pas par là.
#[test]
#[ignore]
fn mirrored_and_turned_shape_keeps_its_orientation() {
    let mut doc = Document::new("miroir tourné");
    // (miroir gauche-droite, miroir haut-bas) ; cadres non carrés et 30° : le sens de rotation se voit.
    let flips = [(true, false), (false, true), (true, true)];
    let frames: Vec<Frame> = (0..flips.len()).map(|i| frame_at(30.0 + 55.0 * i as f64, 60.0, 40.0, 20.0, 30.0)).collect();
    for (f, (h, v)) in frames.iter().zip(flips) {
        doc.pages[0].elements.push(Element { flip_h: h, flip_v: v, ..element(*f, filled_shape(ShapeKind::Path, Some(TRIANGLE))) });
    }
    let (_dir, pdf) = render(&doc, &AssetStore::default());
    let r = Raster::gray(&pdf, 1, 127);
    for (f, (h, v)) in frames.iter().zip(flips) {
        // Coin plein du triangle (haut gauche dans son cadre), retourné dans le cadre, puis tourné ; le coin opposé
        // (en diagonale) reste vide.
        let corner = |lx: f64, ly: f64| turned_point(f, if h { f.w_mm - lx } else { lx }, if v { f.h_mm - ly } else { ly });
        let (full, empty) = (corner(3.0, 3.0), corner(f.w_mm - 3.0, f.h_mm - 3.0));
        let (a, b) = (r.level(full.0, full.1), r.level(empty.0, empty.1));
        assert!(a < 128 && b > 200, "miroir ({h}, {v}) tourné de 30° : {a} au coin plein {full:?}, {b} au coin vide {empty:?}");
    }

    // Aller-retour par LibreOffice : le fichier qu'il réenregistre se relit avec les miroirs, angles et places de
    // l'écran (à 0,01 mm et 0,01° près).
    let dir = tempfile::tempdir().unwrap();
    let odg = dir.path().join("miroir.odg");
    export_odg_to_path(&doc, &AssetStore::default(), &FontSet::builtin(), &odg).unwrap();
    let out = dir.path().join("relu");
    std::fs::create_dir(&out).unwrap();
    let profile = format!("-env:UserInstallation=file://{}", dir.path().join("profil").display());
    run(Command::new("soffice").arg(profile).args(["--headless", "--convert-to", "odg", "--outdir"]).arg(&out).arg(&odg));
    let back = carnet_core::import::odg::read_drawing(std::fs::File::open(out.join("miroir.odg")).unwrap(), &[]).unwrap();
    let els = &back.pages[0].elements;
    assert_eq!(els.len(), frames.len(), "relu : {els:?}");
    for ((e, f), (h, v)) in els.iter().zip(&frames).zip(flips) {
        let close = |a: f64, b: f64, tol: f64| (a - b).abs() < tol;
        assert!(
            (e.flip_h, e.flip_v) == (h, v) && close(e.frame.rotation_deg, f.rotation_deg, 0.01) && close(e.frame.x_mm, f.x_mm, 0.01) && close(e.frame.y_mm, f.y_mm, 0.01),
            "miroir ({h}, {v}) relu après LibreOffice : {:?} ({}, {}) au lieu de {f:?}",
            e.frame,
            e.flip_h,
            e.flip_v
        );
    }
}

// ---------------------------------------------------------------------------------------------------------------
// Vrais tableaux de LibreOffice (`table:table` dans un `draw:frame`), écrits à la main dans le paquet d'un export de
// Cahutin (ses polices intégrées comprises) : l'import les relit, et la sonde mesure ce qu'en fait le LibreOffice du
// poste avant que l'export n'en écrive.

/// Style de case d'essai : sans fond (`draw:fill="none"`), marges de 1,5 mm à gauche et à droite et de `vertical_mm`
/// en haut et en bas, alignée en haut, trait `border` ; `text` : l'allure du texte de la case (`style:text-properties`).
fn cell_style(name: &str, border: &str, vertical_mm: f64, text: &str) -> String {
    format!(
        r#"<style:style style:name="{name}" style:family="table-cell"><loext:graphic-properties draw:fill="none" fo:padding-top="{vertical_mm}mm" fo:padding-bottom="{vertical_mm}mm" fo:padding-left="1.5mm" fo:padding-right="1.5mm" draw:textarea-vertical-align="top"/><style:paragraph-properties fo:border="{border}"/>{text}</style:style>"#
    )
}

/// Un vrai tableau à (`x`, `y`) : colonnes et lignes aux tailles données (mm), puis les cases de chaque ligne. Les
/// styles de colonne et de ligne vont dans `styles`, nommés d'après `name`.
fn drawn_table(styles: &mut String, name: &str, x: f64, y: f64, cols: &[f64], rows: &[f64], cells: &[String]) -> String {
    let mut out = format!(r#"<draw:frame svg:x="{x}mm" svg:y="{y}mm" svg:width="{}mm" svg:height="{}mm"><table:table>"#, cols.iter().sum::<f64>(), rows.iter().sum::<f64>());
    for (i, w) in cols.iter().enumerate() {
        styles.push_str(&format!(r#"<style:style style:name="{name}c{i}" style:family="table-column"><style:table-column-properties style:column-width="{w}mm"/></style:style>"#));
        out.push_str(&format!(r#"<table:table-column table:style-name="{name}c{i}"/>"#));
    }
    for (i, (h, row)) in rows.iter().zip(cells).enumerate() {
        styles.push_str(&format!(r#"<style:style style:name="{name}r{i}" style:family="table-row"><style:table-row-properties style:row-height="{h}mm"/></style:style>"#));
        out.push_str(&format!(r#"<table:table-row table:style-name="{name}r{i}">{row}</table:table-row>"#));
    }
    out + "</table:table></draw:frame>"
}

/// L'export de `doc`, dont `content.xml` reçoit en plus `styles` (styles automatiques) et `objects` (sur la première
/// page, après ce qu'y met l'export).
fn export_with_drawn(doc: &Document, dir: &Path, name: &str, styles: &str, objects: &str) -> PathBuf {
    let base = dir.join("base.odg");
    export_odg_to_path(doc, &AssetStore::default(), &FontSet::builtin(), &base).unwrap();
    let mut zin = zip::ZipArchive::new(std::fs::File::open(&base).unwrap()).unwrap();
    let mut content = String::new();
    std::io::Read::read_to_string(&mut zin.by_name("content.xml").unwrap(), &mut content).unwrap();
    for (prefix, ns) in [("table", "urn:oasis:names:tc:opendocument:xmlns:table:1.0"), ("loext", "urn:org:documentfoundation:names:experimental:office:xmlns:loext:1.0")] {
        if !content.contains(&format!("xmlns:{prefix}=")) {
            content = content.replacen("<office:document-content ", &format!(r#"<office:document-content xmlns:{prefix}="{ns}" "#), 1);
        }
    }
    let content = content.replacen("</office:automatic-styles>", &format!("{styles}</office:automatic-styles>"), 1).replacen("</draw:page>", &format!("{objects}</draw:page>"), 1);
    let path = dir.join(name);
    let mut zout = zip::ZipWriter::new(std::fs::File::create(&path).unwrap());
    // Le paquet de l'export, dans son ordre (`mimetype` en premier), `content.xml` remplacé.
    for i in 0..zin.len() {
        let entry = zin.by_index(i).unwrap();
        if entry.name().is_ok_and(|n| n == "content.xml") {
            drop(entry);
            zout.start_file("content.xml", zip::write::SimpleFileOptions::default()).unwrap();
            std::io::Write::write_all(&mut zout, content.as_bytes()).unwrap();
        } else {
            zout.raw_copy_file(entry).unwrap();
        }
    }
    zout.finish().unwrap();
    path
}

fn soffice_convert(dir: &Path, to: &str, input: &Path, out: &Path) {
    std::fs::create_dir_all(out).unwrap();
    let profile = format!("-env:UserInstallation=file://{}", dir.join("profil").display());
    run(Command::new("soffice").arg(profile).args(["--headless", "--convert-to", to, "--outdir"]).arg(out).arg(input));
}

/// Un tableau fait par LibreOffice lui-même (un tableau écrit à la main, réenregistré par `soffice --convert-to odg`)
/// se relit en tableau de Cahutin : colonnes, lignes, fusions et textes à leur place.
#[test]
#[ignore]
fn a_table_saved_by_libreoffice_is_imported_as_a_table() {
    let dir = tempfile::tempdir().unwrap();
    let mut styles = cell_style("ceA", "0.567mm solid #000000", 1.5, "");
    let p = |t: &str| format!("<text:p>{t}</text:p>");
    let cell = |t: &str| format!(r#"<table:table-cell table:style-name="ceA">{}</table:table-cell>"#, p(t));
    let rows = [
        format!(r#"<table:table-cell table:style-name="ceA" table:number-columns-spanned="2">{}</table:table-cell><table:covered-table-cell/>{}"#, p("Alpha"), cell("Bravo")),
        format!(r#"{}{}<table:table-cell table:style-name="ceA" table:number-rows-spanned="2">{}</table:table-cell>"#, cell("Delta"), cell("Echo"), p("Golf")),
        format!("{}{}<table:covered-table-cell/>", cell("Hotel"), cell("Kilo")),
    ];
    let table = drawn_table(&mut styles, "t", 30.0, 40.0, &[75.0, 37.5, 37.5], &[30.0, 60.0, 30.0], &rows);
    let odg = export_with_drawn(&Document::new("tableau"), dir.path(), "tableau.odg", &styles, &table);
    let out = dir.path().join("relu");
    soffice_convert(dir.path(), "odg", &odg, &out);
    let saved = out.join("tableau.odg");
    let mut content = String::new();
    std::io::Read::read_to_string(&mut zip::ZipArchive::new(std::fs::File::open(&saved).unwrap()).unwrap().by_name("content.xml").unwrap(), &mut content).unwrap();
    assert!(content.contains("<table:table"), "LibreOffice n'a pas gardé de tableau : {content}");
    let known: Vec<String> = FontSet::builtin().families().into_iter().map(|f| f.name).collect();
    let back = carnet_core::import::odg::read_drawing(std::fs::File::open(&saved).unwrap(), &known).unwrap();
    assert_eq!((back.report.skipped, back.report.images), (0, 0), "{:?}", back.report);
    let page = &back.pages[0];
    let tables: Vec<&Element> = page.elements.iter().filter(|e| matches!(e.kind, ElementKind::Table(_))).collect();
    assert_eq!(tables.len(), 1, "{:?}", page.elements);
    let ElementKind::Table(t) = &tables[0].kind else { unreachable!() };
    let close = |a: &[f64], b: &[f64]| a.len() == b.len() && a.iter().zip(b).all(|(x, y)| (x - y).abs() < 0.05);
    assert!(close(&t.cols, &[75.0, 37.5, 37.5]) && close(&t.rows, &[30.0, 60.0, 30.0]), "colonnes {:?}, lignes {:?}", t.cols, t.rows);
    let f = tables[0].frame;
    assert!((f.x_mm - 30.0).abs() < 0.05 && (f.y_mm - 40.0).abs() < 0.05, "{f:?}");
    let mut got: Vec<(u8, u8, u8, u8, String)> = t
        .cells
        .iter()
        .map(|c| {
            let e = page.elements.iter().find(|e| e.id == c.element_id).expect("case sans élément");
            assert_eq!(e.group, tables[0].group, "case hors du groupe du tableau");
            let text = match &e.kind {
                ElementKind::Text(tx) => tx.content.paragraphs.iter().flat_map(|p| p.runs.iter().map(|r| r.text.as_str())).collect(),
                k => format!("{k:?}"),
            };
            (c.col, c.row, c.col_span, c.row_span, text)
        })
        .collect();
    got.sort();
    let want = [(0, 0, 2, 1, "Alpha"), (0, 1, 1, 1, "Delta"), (0, 2, 1, 1, "Hotel"), (1, 1, 1, 1, "Echo"), (1, 2, 1, 1, "Kilo"), (2, 0, 1, 1, "Bravo"), (2, 1, 1, 2, "Golf")];
    assert_eq!(got, want.map(|(c, r, cs, rs, w)| (c, r, cs, rs, w.to_string())));
    assert_eq!(t.border, Some(Stroke { width_mm: 1.0, color: "#000000".into(), style: None }));
}

/// Encre en travers d'un trait (somme de la noirceur, en mm d'encre pleine) et son centre (moyenne pondérée), sur
/// ±`half` mm autour de `at`, le long de la droite `along` ; vertical : un trait vertical (profil horizontal).
fn ink_across(r: &Raster, vertical: bool, at: f64, along: f64, half: f64) -> (f64, f64) {
    let k = r.px_per_mm;
    let (lo, hi) = (((at - half) * k).max(0.0) as usize, ((at + half) * k) as usize);
    let (mut sum, mut moment) = (0.0, 0.0);
    for i in lo..hi {
        let v = (i as f64 + 0.5) / k;
        let d = (255.0 - if vertical { r.level(v, along) } else { r.level(along, v) } as f64) / 255.0;
        sum += d;
        moment += d * v;
    }
    (sum / k, if sum > 0.0 { moment / sum } else { f64::NAN })
}

/// Sonde du LibreOffice du poste (la CI installe la 24.2, ce poste la 26.8), avant d'écrire les tableaux de Cahutin en
/// vrais tableaux : (a) un trait de case écrit à 0,567 mm est dessiné à 1 mm (facteur 1,764) ; (b) le centre des traits
/// tombe sur la grille, colonnes et lignes inégales, avec une fusion ; (c) six lignes de 3 mm aux cases vides tenues
/// par une U+200B en 1 pt gardent leur hauteur ; (d) le premier mot d'une case tombe où le pose une zone de texte de
/// Cahutin de même marge ; (e) toutes les cases ont `draw:fill="none"`. Chaque échec donne toutes les mesures.
#[test]
#[ignore]
fn libreoffice_tables_behave_as_measured() {
    const DPI: u32 = 508; // 20 pixels par millimètre
    const DRAWN_MM: f64 = 1.0; // 0,567 mm écrit, fois 1,764
    let dir = tempfile::tempdir().unwrap();
    let border = "0.567mm solid #000000";
    let mut styles = cell_style("ceP", border, 1.5, "");
    styles.push_str(&cell_style("ceTiny", border, 0.0, r#"<style:text-properties fo:font-size="1pt"/>"#));
    styles.push_str(r#"<style:style style:name="PTiny" style:family="paragraph"><style:text-properties fo:font-size="1pt"/></style:style><style:style style:name="TTiny" style:family="text"><style:text-properties fo:font-size="1pt"/></style:style>"#);
    let empty = r#"<table:table-cell table:style-name="ceP"/>"#;
    // A : colonnes 40 | 20 | 30, lignes 15 | 25 | 20, la première ligne fusionnée sur les deux premières colonnes ;
    // bords 30 | 70 | 90 | 120 et 40 | 55 | 80 | 100.
    let a_rows = [format!(r#"<table:table-cell table:style-name="ceP" table:number-columns-spanned="2"/><table:covered-table-cell/>{empty}"#), empty.repeat(3), empty.repeat(3)];
    let mut objects = drawn_table(&mut styles, "a", 30.0, 40.0, &[40.0, 20.0, 30.0], &[15.0, 25.0, 20.0], &a_rows);
    // B : 4 colonnes de 20 mm, 6 lignes de 3 mm, cases vides tenues par une espace sans largeur en 1 pt ; bas à 138.
    let tiny = "<table:table-cell table:style-name=\"ceTiny\"><text:p text:style-name=\"PTiny\"><text:span text:style-name=\"TTiny\">\u{200B}</text:span></text:p></table:table-cell>";
    let b_rows: Vec<String> = (0..6).map(|_| tiny.repeat(4)).collect();
    objects.push_str(&drawn_table(&mut styles, "b", 30.0, 120.0, &[20.0; 4], &[3.0; 6], &b_rows));
    // C : une case de 60 × 20 mm à (130, 160), le mot écrit avec les styles de paragraphe et de texte de la zone de
    // texte de Cahutin posée à (130, 120), même marge de 1,5 mm, en haut à gauche.
    let mut doc = Document::new("sonde");
    doc.pages[0].elements.push(element(frame_at(130.0, 120.0, 60.0, 20.0, 0.0), text_kind("Repere", 14.0)));
    let base = dir.path().join("zone.odg");
    export_odg_to_path(&doc, &AssetStore::default(), &FontSet::builtin(), &base).unwrap();
    let mut exported = String::new();
    std::io::Read::read_to_string(&mut zip::ZipArchive::new(std::fs::File::open(&base).unwrap()).unwrap().by_name("content.xml").unwrap(), &mut exported).unwrap();
    let before = &exported[..exported.find("Repere</text:span>").expect("mot de la zone")];
    let style_of = |tag: &str| {
        let from = before.rfind(&format!(r#"<{tag} text:style-name=""#)).expect(tag) + tag.len() + 19;
        before[from..from + before[from..].find('"').unwrap()].to_string()
    };
    let (p_style, t_style) = (style_of("text:p"), style_of("text:span"));
    let word = format!(r#"<table:table-cell table:style-name="ceP"><text:p text:style-name="{p_style}"><text:span text:style-name="{t_style}">Repere</text:span></text:p></table:table-cell>"#);
    objects.push_str(&drawn_table(&mut styles, "c", 130.0, 160.0, &[60.0], &[20.0], &[word]));
    assert!(!styles.contains("draw:fill=\"solid\"") && styles.matches("draw:fill=\"none\"").count() == 2, "(e) toutes les cases sans fond");
    let odg = export_with_drawn(&doc, dir.path(), "sonde.odg", &styles, &objects);
    soffice_convert(dir.path(), "pdf", &odg, dir.path());
    let pdf = dir.path().join("sonde.pdf");
    let r = Raster::gray(&pdf, 1, DPI);

    // (a) et (b) : traits du tableau A, relevés loin des croisements, sur ±6 mm (les traits sont à 15 mm au moins).
    let lines: [(bool, f64, &[f64]); 8] = [
        (true, 30.0, &[47.5, 67.5, 90.0]),
        (true, 70.0, &[67.5, 90.0]),
        (true, 90.0, &[47.5, 67.5, 90.0]),
        (true, 120.0, &[47.5, 67.5, 90.0]),
        (false, 40.0, &[50.0, 105.0]),
        (false, 55.0, &[50.0, 80.0, 105.0]),
        (false, 80.0, &[50.0, 80.0, 105.0]),
        (false, 100.0, &[50.0, 80.0, 105.0]),
    ];
    let measured: Vec<(bool, f64, f64, f64, f64)> = lines.iter().flat_map(|(v, at, alongs)| alongs.iter().map(move |&al| (*v, *at, al))).map(|(v, at, al)| {
        let (ink, centre) = ink_across(&r, v, at, al, 6.0);
        (v, at, al, ink, centre)
    }).collect();
    let thickness = measured.iter().map(|m| m.3).sum::<f64>() / measured.len() as f64;
    // Un relevé sans encre (trait absent ou déplacé) a un centre NaN : il fait échouer (b), comme il fait échouer (a).
    let worst_thickness = measured.iter().map(|m| (m.3 - DRAWN_MM).abs()).fold(0.0, f64::max);
    let worst_centre = measured.iter().map(|m| (m.4 - m.1).abs()).fold(0.0, |a: f64, d| if d.is_nan() { f64::INFINITY } else { a.max(d) });
    let (merged_ink, _) = ink_across(&r, true, 70.0, 47.5, 6.0);

    // (c) : bas du tableau B, le dernier trait horizontal sous y = 120 dans la première colonne.
    let mut runs: Vec<(f64, f64)> = Vec::new();
    let k = r.px_per_mm;
    for i in (121.0 * k) as usize..(200.0 * k) as usize {
        let y = (i as f64 + 0.5) / k;
        if r.level(40.0, y) < 128 {
            match runs.last_mut() {
                Some((_, end)) if y - *end < 0.2 => *end = y,
                _ => runs.push((y, y)),
            }
        }
    }
    let b_lines: Vec<f64> = runs.iter().map(|(s, e)| (s + e) / 2.0).collect();
    let b_bottom = b_lines.last().copied().unwrap_or(f64::NAN);

    // (d) : le mot de la case et celui de la zone, chacun depuis le coin de son cadre.
    let words = words(&pdf);
    let find = |y0: f64, y1: f64| words.iter().find(|w| w.page == 0 && w.text == "Repere" && (y0..y1).contains(&(w.y_min / PT_PER_MM))).map(|w| (w.x_min / PT_PER_MM, w.y_min / PT_PER_MM));
    let (zone, case) = (find(115.0, 155.0), find(155.0, 200.0));
    let offset = match (zone, case) {
        (Some(z), Some(c)) => Some((c.0 - 130.0 - (z.0 - 130.0), c.1 - 160.0 - (z.1 - 120.0))),
        _ => None,
    };

    let samples: Vec<String> = measured.iter().map(|(v, at, al, ink, c)| format!("{} {at} (relevé à {al}) : encre {ink:.3}, centre {c:.3}", if *v { "vertical" } else { "horizontal" })).collect();
    let report = format!(
        "mesures : épaisseur moyenne {thickness:.3} mm (attendu {DRAWN_MM}), écart le plus grand {worst_thickness:.3} mm ; écart de centre le plus grand {worst_centre:.3} mm ; encre à travers la fusion {merged_ink:.3} mm ; \
         traits : {} ; tableau B : traits horizontaux à {b_lines:.2?}, bas {b_bottom:.2} (attendu 138) ; \
         mot de la zone {zone:.2?}, de la case {case:.2?}, décalage case moins zone {offset:.2?}",
        samples.join(" ; ")
    );
    assert!(worst_thickness <= 0.1, "(a) épaisseur des traits de case : {report}");
    assert!(worst_centre <= 0.1, "(b) traits sur la grille : {report}");
    assert!(merged_ink < 0.05, "(b) pas de trait dans la case fusionnée : {report}");
    assert!((b_bottom - 138.0).abs() <= 0.2, "(c) lignes de 3 mm gardées : {report}");
    assert!(offset.is_some_and(|(dx, dy)| dx.abs() <= 0.3 && dy.abs() <= 0.3), "(d) premier mot à la place de la zone de texte : {report}");
}

/// Contenu (`content.xml`) d'un ODG.
fn odg_content(odg: &Path) -> String {
    let mut content = String::new();
    std::io::Read::read_to_string(&mut zip::ZipArchive::new(std::fs::File::open(odg).unwrap()).unwrap().by_name("content.xml").unwrap(), &mut content).unwrap();
    content
}

/// L'id de la case qui commence en (`col`, `row`) du tableau `table`.
fn cell_of(page: &Page, table: &str, col: u8, row: u8) -> Id {
    let t = page.elements.iter().find(|e| e.id == table).unwrap();
    let ElementKind::Table(t) = &t.kind else { unreachable!() };
    t.cells.iter().find(|c| c.col == col && c.row == row).unwrap().element_id.clone()
}

/// Un tableau simple exporté en vrai tableau de LibreOffice a le rendu de l'écran : (a) trait de 1 mm rendu à 1 mm,
/// centré sur la grille ; (b) coins extérieurs pleins ; (c) fond de couleur d'une case ; (d) pictogramme au centre de
/// sa case et à sa taille (sa boîte d'encre à la place de celle du même pictogramme posé à part) ; (e) photo recadrée
/// dans sa case ; (f) huit lignes de 4 mm aux cases vides : chaque trait à sa place, le bas du tableau compris ;
/// (g) un tableau en tirets reste en traits et cadres, ses tirets rendus. Chaque échec donne toutes les mesures.
#[test]
#[ignore]
fn a_real_table_is_rendered_as_on_screen() {
    const DPI: u32 = 508;
    let mut doc = Document::new("vrai tableau");
    let mut store = AssetStore::default();
    let photo = png_asset(&mut doc, &mut store, &quarters());
    let page = &mut doc.pages[0];
    // A : 3 x 3 cases de 50 x 30 mm à (30, 30), trait noir de 1 mm.
    let a = carnet_core::table::add_table(page, frame_at(30.0, 30.0, 150.0, 90.0, 0.0), 3, 3);
    if let ElementKind::Table(t) = &mut page.elements.iter_mut().find(|e| e.id == a).unwrap().kind {
        t.border = Some(Stroke { width_mm: 1.0, color: "#000000".into(), style: None });
    }
    let colored = cell_of(page, &a, 0, 0);
    if let ElementKind::Text(t) = &mut page.elements.iter_mut().find(|e| e.id == colored).unwrap().kind {
        t.content = RichText::plain("Alpha");
        t.frame = Some(TextFrame { fill: Some("#ffd6d6".into()), stroke: None, corner_radius_mm: 0.0, image: None, pattern: None, write_on_lines: None });
    }
    let (picto, photo_cell) = (cell_of(page, &a, 1, 0), cell_of(page, &a, 2, 0));
    carnet_core::table::set_cell_content(page, &picto, carnet_core::table::CellContent::Picto("1F98A".into())).unwrap();
    carnet_core::table::set_cell_content(page, &photo_cell, carnet_core::table::CellContent::Photo).unwrap();
    if let ElementKind::Photo(p) = &mut page.elements.iter_mut().find(|e| e.id == photo_cell).unwrap().kind {
        p.asset = Some(photo.clone());
    }
    let picto_frame = page.elements.iter().find(|e| e.id == picto).unwrap().frame;
    assert_eq!((picto_frame.x_mm, picto_frame.y_mm, picto_frame.w_mm), (91.5, 31.5, 27.0));
    // Le même pictogramme posé à part, 170 mm plus bas : le témoin.
    page.elements.push(element(frame_at(91.5, 201.5, 27.0, 27.0, 0.0), ElementKind::Image(ImageProps { code: "1F98A".into(), outline: None })));
    // B : 4 x 8 lignes de 4 mm aux cases vides, à (30, 130) ; bas à 162.
    carnet_core::table::add_table(page, frame_at(30.0, 130.0, 120.0, 32.0, 0.0), 4, 8);
    // C : tirets rouges de 1 mm, à (30, 250).
    let c = carnet_core::table::add_table(page, frame_at(30.0, 250.0, 150.0, 30.0, 0.0), 3, 1);
    if let ElementKind::Table(t) = &mut page.elements.iter_mut().find(|e| e.id == c).unwrap().kind {
        t.border = Some(Stroke { width_mm: 1.0, color: "#000000".into(), style: Some(StrokeStyle::Dashed) });
    }
    let (dir, pdf) = render(&doc, &store);
    let content = odg_content(&dir.path().join("banc.odg"));
    assert_eq!(content.matches("<table:table>").count(), 2, "A et B en vrais tableaux, C en traits");
    assert!(content.matches("<draw:line").count() >= 4, "C en traits");
    let r = Raster::rgb(&pdf, 1, DPI);

    // (a) Traits de A, loin des croisements, des cases écrites et des images.
    let lines: [(bool, f64, &[f64]); 8] = [
        (true, 30.0, &[75.0, 105.0]),
        (true, 80.0, &[75.0, 105.0]),
        (true, 130.0, &[75.0, 105.0]),
        (true, 180.0, &[75.0, 105.0]),
        (false, 30.0, &[84.0, 125.0]),
        (false, 60.0, &[84.0, 125.0]),
        (false, 90.0, &[55.0, 105.0, 155.0]),
        (false, 120.0, &[55.0, 105.0, 155.0]),
    ];
    let measured: Vec<(bool, f64, f64, f64, f64)> = lines.iter().flat_map(|(v, at, al)| al.iter().map(move |&al| (*v, *at, al))).map(|(v, at, al)| {
        // ±1,2 mm : les images des cases du haut commencent à 1,5 mm des traits.
        let (ink, centre) = ink_across(&r, v, at, al, 1.2);
        (v, at, al, ink, centre)
    }).collect();
    let worst_thickness = measured.iter().map(|m| (m.3 - 1.0).abs()).fold(0.0, f64::max);
    let worst_centre = measured.iter().map(|m| (m.4 - m.1).abs()).fold(0.0, |a: f64, d| if d.is_nan() { f64::INFINITY } else { a.max(d) });
    // (b) Coins extérieurs : pleins jusqu'au bord du trait (0,4 mm hors de la grille sur les deux axes).
    let corners: Vec<u8> = [(29.6, 29.6), (180.4, 29.6), (29.6, 120.4), (180.4, 120.4)].iter().map(|&(x, y)| r.level(x, y)).collect();
    // (c) Fond de la case colorée, loin de son texte.
    let fill = [r.color(34.0, 34.0), r.color(76.0, 56.0)];
    // (d) Pictogramme : boîte d'encre dans sa case, comparée à celle du témoin décalé de 170 mm.
    let in_cell = r.ink_box(81.0, 31.0, 129.0, 59.0, 200);
    let witness = r.ink_box(81.0, 201.0, 129.0, 229.0, 200).map(|(x0, y0, x1, y1)| (x0, y0 - 170.0, x1, y1 - 170.0));
    let picto_gap = match (in_cell, witness) {
        (Some(a), Some(b)) => [a.0 - b.0, a.1 - b.1, a.2 - b.2, a.3 - b.3].iter().map(|d| d.abs()).fold(0.0, f64::max),
        _ => f64::INFINITY,
    };
    // (e) Photo : cadre 131,5..178,5 x 31,5..58,5, les quatre quarts à leur place, blanc dans la marge de 1,5 mm.
    let quarters_seen = [(140.0, 38.0, RED), (170.0, 38.0, BLUE), (140.0, 52.0, GREEN), (170.0, 52.0, YELLOW), (131.0, 45.0, WHITE), (155.0, 59.0, WHITE)];
    let photo_ok = quarters_seen.iter().all(|&(x, y, want)| near(r.color(x, y), want));
    // (f) Traits horizontaux de B le long de x = 45 (premières cases).
    let mut runs: Vec<(f64, f64)> = Vec::new();
    let k = r.px_per_mm;
    for i in (128.0 * k) as usize..(170.0 * k) as usize {
        let y = (i as f64 + 0.5) / k;
        if r.level(45.0, y) < 128 {
            match runs.last_mut() {
                Some((_, end)) if y - *end < 0.2 => *end = y,
                _ => runs.push((y, y)),
            }
        }
    }
    let b_lines: Vec<f64> = runs.iter().map(|(s, e)| (s + e) / 2.0).collect();
    let b_worst = if b_lines.len() == 9 { b_lines.iter().enumerate().map(|(i, y)| (y - (130.0 + 4.0 * i as f64)).abs()).fold(0.0, f64::max) } else { f64::INFINITY };
    // (g) Tirets de C : le trait du haut alterne encre et blanc entre x = 40 et 70.
    let mut switches = 0;
    let mut last = None;
    for i in (40.0 * k) as usize..(70.0 * k) as usize {
        let dark = r.level((i as f64 + 0.5) / k, 250.0) < 128;
        if last.is_some_and(|l| l != dark) {
            switches += 1;
        }
        last = Some(dark);
    }
    let report = format!(
        "traits {:?} ; épaisseur écart max {worst_thickness:.3}, centre écart max {worst_centre:.3} ; coins {corners:?} ; fond {fill:?} ; picto case {in_cell:?} témoin {witness:?} écart {picto_gap:.3} ; photo {:?} ; B traits {b_lines:.2?} écart {b_worst:.3} ; tirets {switches} changements",
        measured.iter().map(|m| format!("{}{}@{}: {:.3}/{:.3}", if m.0 { 'V' } else { 'H' }, m.1, m.2, m.3, m.4)).collect::<Vec<_>>(),
        quarters_seen.iter().map(|&(x, y, _)| r.color(x, y)).collect::<Vec<_>>()
    );
    assert!(worst_thickness <= 0.1, "(a) épaisseur : {report}");
    assert!(worst_centre <= 0.1, "(a) traits sur la grille : {report}");
    assert!(corners.iter().all(|&l| l < 100), "(b) coins pleins : {report}");
    assert!(fill.iter().all(|&c| near(c, (255, 214, 214))), "(c) fond de case : {report}");
    assert!(picto_gap <= 0.15, "(d) pictogramme : {report}");
    assert!(photo_ok, "(e) photo : {report}");
    assert!(b_worst <= 0.15, "(f) lignes basses : {report}");
    assert!(switches >= 6, "(g) tirets : {report}");
}

/// Aller-retour par LibreOffice : un tableau simple exporté, réenregistré par `soffice --convert-to odg`, puis relu par
/// Cahutin, est le même tableau : cadre, colonnes, lignes, fusions, trait, textes et puces, fond, pictogramme (miroir
/// compris) et photo dans leurs cases.
#[test]
#[ignore]
fn a_real_table_comes_back_through_libreoffice() {
    let mut doc = Document::new("aller-retour");
    let mut store = AssetStore::default();
    let photo = png_asset(&mut doc, &mut store, &quarters());
    let page = &mut doc.pages[0];
    let t = carnet_core::table::add_table(page, frame_at(15.0, 36.0, 180.0, 120.0, 0.0), 4, 3);
    if let ElementKind::Table(p) = &mut page.elements.iter_mut().find(|e| e.id == t).unwrap().kind {
        p.cols = vec![1.3, 1.0, 1.0, 1.0];
        p.rows = vec![0.8, 1.0, 1.0];
        p.border = Some(Stroke { width_mm: 0.6, color: "#6b5444".into(), style: None });
    }
    carnet_core::table::layout(page, &t);
    let (a, b) = (cell_of(page, &t, 1, 1), cell_of(page, &t, 3, 1));
    carnet_core::table::merge(page, &a, &b).unwrap();
    let (head, list, picto, photo_cell) = (cell_of(page, &t, 0, 0), cell_of(page, &t, 0, 1), cell_of(page, &t, 1, 0), cell_of(page, &t, 2, 0));
    if let ElementKind::Text(tx) = &mut page.elements.iter_mut().find(|e| e.id == head).unwrap().kind {
        tx.content = RichText::plain("Quand ?");
        tx.frame = Some(TextFrame { fill: Some("#f1e6d6".into()), stroke: None, corner_radius_mm: 0.0, image: None, pattern: None, write_on_lines: None });
    }
    if let ElementKind::Text(tx) = &mut page.elements.iter_mut().find(|e| e.id == list).unwrap().kind {
        tx.content = RichText { paragraphs: vec![
            Paragraph { align: None, runs: vec![Run::plain("Pomme")], list: Some(ListKind::Bullet) },
            Paragraph { align: None, runs: vec![Run::plain("Poire")], list: Some(ListKind::Bullet) },
        ] };
        tx.style.align = Align::Left;
    }
    carnet_core::table::set_cell_content(page, &picto, carnet_core::table::CellContent::Picto("1F98A".into())).unwrap();
    page.elements.iter_mut().find(|e| e.id == picto).unwrap().flip_h = true;
    carnet_core::table::set_cell_content(page, &photo_cell, carnet_core::table::CellContent::Photo).unwrap();
    if let ElementKind::Photo(p) = &mut page.elements.iter_mut().find(|e| e.id == photo_cell).unwrap().kind {
        p.asset = Some(photo);
    }
    let dir = tempfile::tempdir().unwrap();
    let odg = dir.path().join("tableau.odg");
    export_odg_to_path(&doc, &store, &FontSet::builtin(), &odg).unwrap();
    assert!(odg_content(&odg).contains("<table:table>"));
    let out = dir.path().join("relu");
    soffice_convert(dir.path(), "odg", &odg, &out);
    let saved = out.join("tableau.odg");
    assert!(odg_content(&saved).contains("<table:table"), "LibreOffice n'a pas gardé de tableau");
    let known: Vec<String> = FontSet::builtin().families().into_iter().map(|f| f.name).collect();
    let back = carnet_core::import::odg::read_drawing(std::fs::File::open(&saved).unwrap(), &known).unwrap();
    assert_eq!(back.report.skipped, 0, "{:?}", back.report);
    let p = &back.pages[0];
    let tables: Vec<&Element> = p.elements.iter().filter(|e| matches!(e.kind, ElementKind::Table(_))).collect();
    assert_eq!(tables.len(), 1, "{:?}", p.elements);
    let ElementKind::Table(tp) = &tables[0].kind else { unreachable!() };
    let f = tables[0].frame;
    assert!([f.x_mm - 15.0, f.y_mm - 36.0, f.w_mm - 180.0, f.h_mm - 120.0].iter().all(|d| d.abs() < 0.02), "{f:?}");
    let shares = |v: &[f64]| v.iter().map(|x| x / v.iter().sum::<f64>()).collect::<Vec<_>>();
    let close = |a: &[f64], b: &[f64]| a.len() == b.len() && shares(a).iter().zip(shares(b)).all(|(x, y)| (x - y).abs() < 0.001);
    assert!(close(&tp.cols, &[1.3, 1.0, 1.0, 1.0]) && close(&tp.rows, &[0.8, 1.0, 1.0]), "colonnes {:?}, lignes {:?}", tp.cols, tp.rows);
    assert_eq!(tp.border, Some(Stroke { width_mm: 0.6, color: "#6b5444".into(), style: None }));
    let mut got: Vec<String> = tp
        .cells
        .iter()
        .map(|c| {
            let e = p.elements.iter().find(|e| e.id == c.element_id).unwrap();
            let what = match &e.kind {
                ElementKind::Text(tx) => format!(
                    "{:?} {:?}",
                    tx.content.paragraphs.iter().map(|q| (q.runs.iter().map(|r| r.text.as_str()).collect::<String>(), q.list)).collect::<Vec<_>>(),
                    tx.frame.as_ref().and_then(|f| f.fill.clone())
                ),
                ElementKind::Image(i) => format!("picto {} {}", i.code, e.flip_h),
                ElementKind::Photo(ph) => format!("photo {}", ph.asset.is_some()),
                k => format!("{k:?}"),
            };
            format!("{} {} {} {} {what}", c.col, c.row, c.col_span, c.row_span)
        })
        .collect();
    got.sort();
    let want = [
        "0 0 1 1 [(\"Quand ?\", None)] Some(\"#f1e6d6\")",
        "0 1 1 1 [(\"Pomme\", Some(Bullet)), (\"Poire\", Some(Bullet))] None",
        "0 2 1 1 [] None",
        "1 0 1 1 picto 1F98A true",
        "1 1 3 1 [] None",
        "1 2 1 1 [] None",
        "2 0 1 1 photo true",
        "2 2 1 1 [] None",
        "3 0 1 1 [] None",
        "3 2 1 1 [] None",
    ];
    assert_eq!(got, want);
}
