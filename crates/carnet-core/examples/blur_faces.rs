// Floute tous les visages repérés d'une photo : `cargo run -p carnet-core --example blur_faces -- entrée.jpg sortie.jpg`.
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let asset = carnet_core::photo::import_path(std::path::Path::new(&a[1])).unwrap();
    let faces = carnet_core::faces::detect_faces(&asset).unwrap();
    let out = carnet_core::faces::blur_regions(&asset, &faces).unwrap();
    std::fs::write(&a[2], &*out.bytes).unwrap();
    println!("{} visages floutés", faces.len());
}
