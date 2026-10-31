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
        kind: ElementKind::Text(TextProps { content: RichText::plain(&c.text), style, prompt: None, frame: None }),
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
            stroke: Some(Stroke { width_mm: 1.0, color: "#000000".into(), style: None }),
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
            }),
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
            }),
        }),
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
            frame: Some(TextFrame { fill: None, stroke: None, corner_radius_mm: 8.0, image: Some(id.clone()), pattern: None }),
        }),
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
