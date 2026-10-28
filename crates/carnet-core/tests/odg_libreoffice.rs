//! Banc de fidélité : l'ODG exporté est rendu par LibreOffice, puis le PDF obtenu est inspecté avec poppler.
//! Lancer avec `cargo test -p carnet-core --test odg_libreoffice -- --ignored` (soffice, pdftotext, pdffonts,
//! pdfinfo et pdfimages doivent être dans le PATH).

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
        kind: ElementKind::Text(TextProps { content: RichText::plain(&c.text), style, prompt: None }),
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
            }),
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
        kind: ElementKind::Text(TextProps { content: RichText::plain("Police ajoutée"), style, prompt: None }),
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
        kind: ElementKind::Text(TextProps { content: RichText::plain("Police du poste"), style, prompt: None }),
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
            kind: ElementKind::Shape(ShapeProps { shape: kind, fill: Some("#000000".into()), stroke: None, corner_radius_mm: 0.0 }),
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
            stroke: Some(Stroke { width_mm: 1.0, color: "#000000".into() }),
            corner_radius_mm: 0.0,
        }),
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

fn picture_element(code: &str, x: f64, group: Option<&str>) -> Element {
    Element {
        id: new_id(),
        frame: Frame { x_mm: x, y_mm: ORIGIN_MM, w_mm: 50.0, h_mm: 50.0, rotation_deg: 0.0 },
        locked: false,
        zone: false,
        group: group.map(String::from),
        kind: ElementKind::Image(ImageProps { code: code.into() }),
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

#[test]
#[ignore]
fn groups_survive_a_libreoffice_round_trip() {
    let mut doc = Document::new("groupe");
    doc.pages[0].elements.push(picture_element("1F98A", ORIGIN_MM, Some("g")));
    doc.pages[0].elements.push(picture_element("2600", ORIGIN_MM + 60.0, Some("g")));
    let dir = tempfile::tempdir().unwrap();
    let odg = dir.path().join("groupe.odg");
    export_odg_to_path(&doc, &AssetStore::default(), &FontSet::builtin(), &odg).unwrap();
    let out = dir.path().join("relu");
    std::fs::create_dir(&out).unwrap();
    let profile = format!("-env:UserInstallation=file://{}", dir.path().join("profil").display());
    run(Command::new("soffice").arg(profile).args(["--headless", "--convert-to", "odg", "--outdir"]).arg(&out).arg(&odg));
    let mut zip = zip::ZipArchive::new(std::fs::File::open(out.join("groupe.odg")).unwrap()).unwrap();
    let mut content = String::new();
    std::io::Read::read_to_string(&mut zip.by_name("content.xml").unwrap(), &mut content).unwrap();
    assert!(content.contains("<draw:g"), "le groupe a disparu après LibreOffice");
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
        let first_title_word = match info.id.as_str() {
            "ferme" => "Notre",
            "anniversaire" => "Joyeux",
            "meteo" => "météo",
            "prenom" => "J'écris",
            _ => "saisons",
        };
        assert!(text.contains(first_title_word), "{} : {text}", info.id);
        assert!(text.contains("OpenMoji"), "{} : mention absente", info.id);
    }
}
