//! Banc d'import : une fiche type (titre, consigne, image, tableau, encadré) écrite en ODT, convertie
//! en DOCX et en PDF par LibreOffice, puis importée. Lancer avec :
//! `cargo test -p carnet-core --test import_libreoffice -- --ignored --test-threads=1`

use carnet_core::fonts::FontSet;
use carnet_core::import::office::{SofficeLookup, find_soffice, import_file};
use carnet_core::import::Imported;
use carnet_core::odg::export_odg_to_path;
use carnet_core::photo::AssetStore;
use carnet_core::*;
use image::{ImageFormat, Rgb, RgbImage};
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::AtomicBool;

fn run(cmd: &mut Command) -> String {
    let out = cmd.output().unwrap_or_else(|e| panic!("{cmd:?} : {e}"));
    assert!(out.status.success(), "{cmd:?} : {}", String::from_utf8_lossy(&out.stderr));
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn soffice() -> PathBuf {
    find_soffice(&SofficeLookup::from_env()).expect("LibreOffice introuvable")
}

fn convert(dir: &Path, input: &Path, to: &str) -> PathBuf {
    let profile = format!("-env:UserInstallation=file://{}", dir.join("profil").display());
    run(Command::new(soffice()).arg(profile).args(["--headless", "--convert-to", to, "--outdir"]).arg(dir).arg(input));
    input.with_extension(to)
}

/// La fiche de l'essai du 2026-10-29, en Flat ODT.
fn worksheet(dir: &Path) -> PathBuf {
    let mut img = RgbImage::from_pixel(400, 300, Rgb([240, 200, 120]));
    for (x, y, p) in img.enumerate_pixels_mut() {
        if (x as i32 - 200).pow(2) + (y as i32 - 150).pow(2) < 100 * 100 {
            *p = Rgb([200, 60, 60]);
        }
    }
    let mut png = Cursor::new(Vec::new());
    img.write_to(&mut png, ImageFormat::Png).unwrap();
    use base64_engine::encode;
    let fodt = format!(
        r##"<?xml version="1.0" encoding="UTF-8"?>
<office:document xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0" xmlns:style="urn:oasis:names:tc:opendocument:xmlns:style:1.0" xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0" xmlns:table="urn:oasis:names:tc:opendocument:xmlns:table:1.0" xmlns:draw="urn:oasis:names:tc:opendocument:xmlns:drawing:1.0" xmlns:fo="urn:oasis:names:tc:opendocument:xmlns:xsl-fo-compatible:1.0" xmlns:svg="urn:oasis:names:tc:opendocument:xmlns:svg-compatible:1.0" xmlns:xlink="http://www.w3.org/1999/xlink" office:version="1.3" office:mimetype="application/vnd.oasis.opendocument.text">
<office:automatic-styles>
 <style:style style:name="T" style:family="paragraph"><style:paragraph-properties fo:text-align="center"/><style:text-properties fo:font-size="24pt" fo:font-weight="bold" fo:color="#c0392b" style:font-name="Liberation Sans"/></style:style>
 <style:style style:name="B" style:family="text"><style:text-properties fo:font-weight="bold"/></style:style>
 <style:style style:name="fr" style:family="graphic"><style:graphic-properties fo:border="0.06cm solid #2e86c1" fo:background-color="#eaf2fb" fo:padding="0.2cm"/></style:style>
 <style:page-layout style:name="pm1"><style:page-layout-properties fo:page-width="21cm" fo:page-height="29.7cm" fo:margin="2cm"/></style:page-layout>
</office:automatic-styles>
<office:master-styles><style:master-page style:name="Standard" style:page-layout-name="pm1"/></office:master-styles>
<office:body><office:text>
<text:p text:style-name="T">La pomme</text:p>
<text:p><text:span text:style-name="B">Consigne :</text:span> observe l'image puis colorie la pomme.</text:p>
<text:p><draw:frame draw:name="img" text:anchor-type="paragraph" svg:width="8cm" svg:height="6cm"><draw:image><office:binary-data>{}</office:binary-data></draw:image></draw:frame></text:p>
<text:p>Complète le tableau avec les mots : rouge, ronde, fruit.</text:p>
<table:table table:name="t"><table:table-column table:number-columns-repeated="3"/>
<table:table-row><table:table-cell><text:p>Couleur</text:p></table:table-cell><table:table-cell><text:p>Forme</text:p></table:table-cell><table:table-cell><text:p>C'est un…</text:p></table:table-cell></table:table-row>
</table:table>
<text:p><draw:frame draw:name="box" draw:style-name="fr" text:anchor-type="paragraph" svg:width="12cm" svg:height="2.5cm"><draw:text-box><text:p>Écris une phrase sur la pomme.</text:p></draw:text-box></draw:frame></text:p>
</office:text></office:body></office:document>"##,
        encode(&png.into_inner())
    );
    let path = dir.join("fiche.fodt");
    std::fs::write(&path, fodt).unwrap();
    path
}

/// Base64 sans dépendance : la fiche n'a besoin que de l'encodage.
mod base64_engine {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    pub fn encode(data: &[u8]) -> String {
        let mut out = String::new();
        for chunk in data.chunks(3) {
            let b = [chunk[0], *chunk.get(1).unwrap_or(&0), *chunk.get(2).unwrap_or(&0)];
            let n = (b[0] as u32) << 16 | (b[1] as u32) << 8 | b[2] as u32;
            for i in 0..4 {
                if i <= chunk.len() {
                    out.push(ALPHABET[(n >> (18 - 6 * i) & 63) as usize] as char);
                } else {
                    out.push('=');
                }
            }
        }
        out
    }
}

fn known_fonts() -> Vec<String> {
    ["Andika", "Liberation Serif", "Liberation Sans"].map(String::from).to_vec()
}

fn import(path: &Path) -> Imported {
    import_file(path, Some(&soffice()), &known_fonts(), &AtomicBool::new(false)).unwrap_or_else(|e| panic!("{} : {}", path.display(), e.message_fr()))
}

fn texts(p: &Page) -> Vec<String> {
    p.elements
        .iter()
        .filter_map(|e| if let ElementKind::Text(t) = &e.kind { Some(t.content.paragraphs.iter().flat_map(|p| p.runs.iter().map(|r| r.text.as_str())).collect()) } else { None })
        .collect()
}

#[test]
#[ignore]
fn office_files_import_with_title_instruction_image_and_box() {
    let dir = tempfile::tempdir().unwrap();
    let fodt = worksheet(dir.path());
    for ext in ["odt", "docx", "pdf"] {
        let file = convert(dir.path(), &fodt, ext);
        let imported = import(&file);
        let page = &imported.pages[0];
        let t = texts(page);
        assert!(t.iter().any(|x| x == "La pomme"), "{ext} : {t:?}");
        assert!(t.iter().any(|x| x.contains("Complète le tableau avec les mots : rouge, ronde, fruit.")), "{ext} : {t:?}");
        assert!(page.elements.iter().any(|e| matches!(e.kind, ElementKind::Photo(_))), "{ext} : pas de photo");
        assert!(page.elements.iter().any(|e| matches!(&e.kind, ElementKind::Shape(s) if s.shape == ShapeKind::Rect)), "{ext} : pas d'encadré");
        assert_eq!(imported.assets.len(), 1, "{ext}");
    }
}

/// Position (mm) du premier mot `word` d'un PDF, par `pdftotext -bbox`.
fn word_position(pdf: &Path, word: &str) -> (f64, f64) {
    let html = run(Command::new("pdftotext").arg("-bbox").arg(pdf).arg("-"));
    let line = html.lines().find(|l| l.contains("<word") && l.contains(&format!(">{word}<"))).unwrap_or_else(|| panic!("{word} absent de {}", pdf.display()));
    let attr = |name: &str| -> f64 {
        let start = line.find(&format!("{name}=\"")).unwrap() + name.len() + 2;
        line[start..].split('"').next().unwrap().parse::<f64>().unwrap() * 25.4 / 72.0
    };
    (attr("xMin"), attr("yMin"))
}

#[test]
#[ignore]
fn imported_page_looks_like_the_original() {
    let dir = tempfile::tempdir().unwrap();
    let original = convert(dir.path(), &worksheet(dir.path()), "pdf");
    let imported = import(&original);
    let mut doc = Document::new("import");
    doc.pages = imported.pages.clone();
    let mut store = AssetStore::default();
    for a in imported.assets {
        doc.assets.insert(a.meta.id.clone(), a.meta.clone());
        store.insert(a).unwrap();
    }
    let out = dir.path().join("reimport");
    std::fs::create_dir_all(&out).unwrap();
    let odg = out.join("page.odg");
    export_odg_to_path(&doc, &store, &FontSet::builtin(), &odg).unwrap();
    let back = convert(&out, &odg, "pdf");
    for word in ["La", "Consigne"] {
        let (a, b) = (word_position(&original, word), word_position(&back, word));
        // 0,5 mm : plus strict que les 2 mm de la conception, pour qu'une erreur de marge (1,5 mm) se voie.
        assert!((a.0 - b.0).abs() <= 0.5 && (a.1 - b.1).abs() <= 0.5, "{word} : original {a:?}, importé {b:?}");
    }
}
