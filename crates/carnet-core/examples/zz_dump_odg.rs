fn main() {
    let id = std::env::args().nth(1).unwrap();
    let doc = carnet_core::examples::document(&id).unwrap();
    let mut buf = std::io::Cursor::new(Vec::new());
    carnet_core::odg::export_odg(
        &doc,
        &Default::default(),
        &carnet_core::fonts::FontSet::builtin(),
        &mut buf,
    )
    .unwrap();
    let mut z = zip::ZipArchive::new(std::io::Cursor::new(buf.into_inner())).unwrap();
    let mut s = String::new();
    std::io::Read::read_to_string(&mut z.by_name("content.xml").unwrap(), &mut s).unwrap();
    let back = carnet_core::import::odg::read_odg(
        std::io::Cursor::new({
            let mut b = std::io::Cursor::new(Vec::new());
            carnet_core::odg::export_odg(
                &doc,
                &Default::default(),
                &carnet_core::fonts::FontSet::builtin(),
                &mut b,
            )
            .unwrap();
            b.into_inner()
        }),
        &["Andika".to_string(), "Playwrite FR Moderne".to_string()],
    )
    .unwrap();
    println!("{:?}", back.report);
    for e in &back.pages[0].elements {
        match &e.kind {
            carnet_core::ElementKind::Shape(s) => println!("shape {:?}", s.shape),
            carnet_core::ElementKind::Image(i) => println!("image {}", i.code),
            carnet_core::ElementKind::Text(_) => println!("text"),
            _ => println!("other"),
        }
    }
    if false {
        for part in s.split('<').filter(|p| {
            p.starts_with("draw:")
                && !p.starts_with("draw:text-box")
                && !p.starts_with("draw:image")
                && !p.starts_with("draw:page")
        }) {
            println!("<{}", &part[..part.len().min(230)]);
        }
    }
}
#[allow(dead_code)]
fn unused() {}
