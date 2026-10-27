//! Banc de fidélité : l'ODG exporté est rendu par LibreOffice, puis le PDF obtenu est inspecté avec poppler.
//! Lancer avec `cargo test -p carnet-core --test odg_libreoffice -- --ignored` (soffice, pdftotext, pdffonts,
//! pdfinfo et pdfimages doivent être dans le PATH).

use carnet_core::odg::export_odg_to_path;
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
    let dir = tempfile::tempdir().unwrap();
    let odg = dir.path().join("banc.odg");
    export_odg_to_path(doc, store, &odg).unwrap();
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
            kind: ElementKind::Photo(PhotoProps {
                asset: Some(a.meta.id.clone()),
                crop: Crop::COVER,
                corner_radius_mm: if i == 0 { 0.0 } else { 4.0 },
                border: None,
                shadow: false,
            }),
        });
        doc.assets.insert(a.meta.id.clone(), a.meta.clone());
        store.insert(a);
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

#[test]
#[ignore]
fn photos_are_rendered() {
    let mut store = AssetStore::default();
    let doc = photo_document(&mut store, 2);
    let (_dir, pdf) = render(&doc, &store);
    let list = run(Command::new("pdfimages").arg("-list").arg(&pdf));
    assert_eq!(list.lines().skip(2).filter(|l| !l.trim().is_empty()).count(), 2, "{list}");
}

