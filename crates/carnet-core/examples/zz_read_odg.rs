fn main() {
    let p = std::env::args().nth(1).unwrap();
    let known: Vec<String> = [
        "Andika",
        "Playwrite FR Moderne",
        "Atkinson Hyperlegible",
        "OpenDyslexic",
        "Liberation Sans",
        "Liberation Serif",
        "Noto Serif",
        "Noto Sans",
    ]
    .map(String::from)
    .to_vec();
    let r = carnet_core::import::odg::read_odg(std::fs::File::open(&p).unwrap(), &known).unwrap();
    let mut kinds = std::collections::BTreeMap::new();
    for pg in &r.pages {
        for e in &pg.elements {
            *kinds
                .entry(
                    format!("{:?}", std::mem::discriminant(&e.kind))
                        .chars()
                        .take(0)
                        .collect::<String>()
                        + match &e.kind {
                            carnet_core::ElementKind::Text(_) => "text",
                            carnet_core::ElementKind::Photo(_) => "photo",
                            carnet_core::ElementKind::Shape(_) => "shape",
                            carnet_core::ElementKind::Image(_) => "image",
                            carnet_core::ElementKind::Table(_) => "table",
                        },
                )
                .or_insert(0) += 1;
        }
    }
    println!("{} pages {:?} skipped={:?}", r.pages.len(), kinds, r.report);
    if let Some(d) = std::env::args().nth(2) {
        dump(&r, &d);
    }
    for pg in &r.pages {
        for e in &pg.elements {
            if let carnet_core::ElementKind::Text(t) = &e.kind {
                let s: String = t
                    .content
                    .paragraphs
                    .iter()
                    .flat_map(|p| p.runs.iter().map(|r| r.text.as_str()))
                    .collect();
                println!(
                    "  text {:?} font={} {:.0}x{:.0}",
                    s.chars().take(40).collect::<String>(),
                    t.style.font,
                    e.frame.w_mm,
                    e.frame.h_mm
                );
            }
        }
    }
}
#[allow(dead_code)]
fn dump(r: &carnet_core::import::Imported, dir: &str) {
    for (i, pg) in r.pages.iter().enumerate() {
        if let Some(e) = pg.elements.first() {
            if e.locked {
                if let carnet_core::ElementKind::Photo(p) = &e.kind {
                    let id = p.asset.clone().unwrap();
                    let a = r.assets.iter().find(|a| a.meta.id == id).unwrap();
                    std::fs::write(format!("{dir}/drawn-{i}.png"), a.bytes.as_slice()).unwrap();
                    println!(
                        "page {i}: image fixe {:?} {}x{}",
                        e.frame, a.meta.width_px, a.meta.height_px
                    );
                }
            }
        }
    }
}
