//! Écrit une page d'exemple en ODG : `cargo run -p carnet-core --example export_sample -- sortie.odg`

use carnet_core::odg::export_odg_to_path;
use carnet_core::photo::{AssetStore, import_bytes};
use carnet_core::*;
use image::{ImageFormat, Rgb, RgbImage};
use std::io::Cursor;

fn gradient_photo(w: u32, h: u32) -> Vec<u8> {
    let img = RgbImage::from_fn(w, h, |x, y| Rgb([(x * 255 / w) as u8, (y * 255 / h) as u8, 160]));
    let mut out = Cursor::new(Vec::new());
    img.write_to(&mut out, ImageFormat::Jpeg).unwrap();
    out.into_inner()
}

fn element(frame: Frame, kind: ElementKind) -> Element {
    Element { id: new_id(), frame, locked: false, zone: false, kind }
}

fn frame(x: f64, y: f64, w: f64, h: f64, r: f64) -> Frame {
    Frame { x_mm: x, y_mm: y, w_mm: w, h_mm: h, rotation_deg: r }
}

fn main() {
    let out = std::env::args().nth(1).expect("chemin du fichier .odg");
    let mut store = AssetStore::default();
    let mut doc = Document::new("La sortie à la ferme");
    let mut photos = vec![];
    for (w, h) in [(1600, 1200), (1200, 1600)] {
        let a = import_bytes(&gradient_photo(w, h)).unwrap();
        doc.assets.insert(a.meta.id.clone(), a.meta.clone());
        photos.push(a.meta.id.clone());
        store.insert(a);
    }
    let page = &mut doc.pages[0];
    page.elements.push(element(
        frame(0.0, 0.0, 210.0, 25.0, 0.0),
        ElementKind::Shape(ShapeProps { shape: ShapeKind::Rect, fill: Some("#f6e3c5".into()), stroke: None, corner_radius_mm: 0.0 }),
    ));
    let mut title = TextStyle::default_zone();
    title.font = "Playwrite FR Moderne".into();
    title.size_pt = 24.0;
    title.align = Align::Center;
    title.color = "#c0392b".into();
    page.elements.push(element(
        frame(10.0, 5.0, 190.0, 18.0, 0.0),
        ElementKind::Text(TextProps { content: RichText::plain("La sortie à la ferme"), style: title, prompt: None }),
    ));
    page.elements.push(element(
        frame(10.0, 35.0, 110.0, 80.0, 0.0),
        ElementKind::Photo(PhotoProps { asset: Some(photos[0].clone()), crop: Crop::COVER, corner_radius_mm: 0.0, border: None, shadow: false }),
    ));
    page.elements.push(element(
        frame(130.0, 40.0, 65.0, 70.0, 6.0),
        ElementKind::Photo(PhotoProps {
            asset: Some(photos[1].clone()),
            crop: Crop { scale: 1.5, center_x: 0.3, center_y: 0.6 },
            corner_radius_mm: 4.0,
            border: Some(Stroke { width_mm: 1.0, color: "#2e86de".into() }),
            shadow: true,
        }),
    ));
    let mut story = RichText::plain("Nous avons donné à manger aux chèvres et aux poules.\nNous avons vu un veau qui venait de naître !");
    story.paragraphs[1].runs[0].bold = true;
    page.elements.push(element(
        frame(10.0, 125.0, 190.0, 30.0, 0.0),
        ElementKind::Text(TextProps { content: story, style: TextStyle::default_zone(), prompt: None }),
    ));
    page.elements.push(element(
        frame(10.0, 165.0, 90.0, 60.0, 0.0),
        ElementKind::Shape(ShapeProps {
            shape: ShapeKind::Ellipse,
            fill: None,
            stroke: Some(Stroke { width_mm: 0.5, color: "#999999".into() }),
            corner_radius_mm: 0.0,
        }),
    ));
    export_odg_to_path(&doc, &store, std::path::Path::new(&out)).unwrap();
    println!("écrit : {out}");
}
