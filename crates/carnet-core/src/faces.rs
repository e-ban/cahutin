//! Flou des visages : repérer les visages d'une photo (sans jamais savoir à qui ils sont) et flouter des
//! zones. Le repérage utilise YuNet (OpenCV Zoo, licence MIT), embarqué et exécuté sur le poste.

use crate::photo::{Asset, ImportError, asset_from_image};
use image::{DynamicImage, GenericImageView, Rgba, RgbaImage};
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;
use tract_onnx::prelude::*;
use ts_rs::TS;

/// Zone d'un visage, en fraction de l'image (0..1) : indépendante du recadrage et de la taille.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, TS)]
#[ts(export)]
pub struct FaceBox {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

const MODEL: &[u8] = include_bytes!("../../../assets/faces/face_detection_yunet_2023mar.onnx");
const INPUT_PX: usize = 640;
const MIN_SCORE: f32 = 0.6;
const MAX_OVERLAP: f64 = 0.3;

type Model = TypedRunnableModel<TypedModel>;

/// Le modèle, préparé une fois au premier repérage.
fn model() -> Result<&'static Model, ImportError> {
    static MODEL_CELL: OnceLock<Option<Model>> = OnceLock::new();
    MODEL_CELL
        .get_or_init(|| tract_onnx::onnx().model_for_read(&mut std::io::Cursor::new(MODEL)).and_then(|m| m.into_optimized()).and_then(|m| m.into_runnable()).ok())
        .as_ref()
        .ok_or(ImportError::Decode)
}

fn decode(asset: &Asset) -> Result<DynamicImage, ImportError> {
    image::load_from_memory(&asset.bytes).map_err(|_| ImportError::Decode)
}

fn overlap(a: &FaceBox, b: &FaceBox) -> f64 {
    let ix = (a.x + a.w).min(b.x + b.w) - a.x.max(b.x);
    let iy = (a.y + a.h).min(b.y + b.h) - a.y.max(b.y);
    if ix <= 0.0 || iy <= 0.0 { 0.0 } else { ix * iy / (a.w * a.h + b.w * b.h - ix * iy) }
}

/// Visages repérés, du plus sûr au moins sûr.
pub fn detect_faces(asset: &Asset) -> Result<Vec<FaceBox>, ImportError> {
    let img = decode(asset)?.to_rgb8();
    let (w, h) = img.dimensions();
    let scale = INPUT_PX as f32 / w.max(h) as f32;
    let (nw, nh) = (((w as f32 * scale) as u32).max(1), ((h as f32 * scale) as u32).max(1));
    let small = image::imageops::resize(&img, nw, nh, image::imageops::FilterType::Triangle);
    // Entrée du modèle : 640 × 640, BGR, image en haut à gauche, le reste noir.
    let input: Tensor = tract_ndarray::Array4::from_shape_fn((1, 3, INPUT_PX, INPUT_PX), |(_, c, y, x)| {
        if (x as u32) < nw && (y as u32) < nh { small.get_pixel(x as u32, y as u32)[2 - c] as f32 } else { 0.0 }
    })
    .into();
    let out = model()?.run(tvec!(input.into())).map_err(|_| ImportError::Decode)?;
    let mut found: Vec<(f32, FaceBox)> = Vec::new();
    // Sorties : scores de classe, d'objet et cadres, pour les pas 8, 16 et 32.
    for (k, stride) in [8usize, 16, 32].into_iter().enumerate() {
        let slice = |i: usize| out[i].as_slice::<f32>().map_err(|_| ImportError::Decode);
        let (cls, obj, bbox) = (slice(k)?, slice(3 + k)?, slice(6 + k)?);
        let cols = INPUT_PX / stride;
        let s = stride as f32;
        for i in 0..cls.len().min(obj.len()).min(bbox.len() / 4) {
            let score = (cls[i].clamp(0.0, 1.0) * obj[i].clamp(0.0, 1.0)).sqrt();
            if score < MIN_SCORE {
                continue;
            }
            let (row, col) = ((i / cols) as f32, (i % cols) as f32);
            let (cx, cy) = ((col + bbox[i * 4]) * s, (row + bbox[i * 4 + 1]) * s);
            let (bw, bh) = (bbox[i * 4 + 2].exp() * s, bbox[i * 4 + 3].exp() * s);
            let (fx, fy) = (scale * w as f32, scale * h as f32);
            let x0 = ((cx - bw / 2.0) / fx).clamp(0.0, 1.0);
            let y0 = ((cy - bh / 2.0) / fy).clamp(0.0, 1.0);
            let x1 = ((cx + bw / 2.0) / fx).clamp(0.0, 1.0);
            let y1 = ((cy + bh / 2.0) / fy).clamp(0.0, 1.0);
            if x1 > x0 && y1 > y0 {
                found.push((score, FaceBox { x: x0 as f64, y: y0 as f64, w: (x1 - x0) as f64, h: (y1 - y0) as f64 }));
            }
        }
    }
    found.sort_by(|a, b| b.0.total_cmp(&a.0));
    let mut kept: Vec<FaceBox> = Vec::new();
    for (_, f) in found {
        if kept.iter().all(|k| overlap(k, &f) < MAX_OVERLAP) {
            kept.push(f);
        }
    }
    Ok(kept)
}

/// Agrandissement de chaque zone : le flou déborde du visage repéré (cheveux, oreilles).
const GROW: f64 = 0.2;

/// Floute l'ellipse inscrite dans chaque zone (agrandie de 20 %) : pixellisation en blocs d'un sixième du
/// visage, puis flou large,
/// bord adouci. Rend une nouvelle image (nouvel id), réencodée sans métadonnées ; l'original n'est pas touché.
pub fn blur_regions(asset: &Asset, zones: &[FaceBox]) -> Result<Asset, ImportError> {
    let img = decode(asset)?;
    let mut out: RgbaImage = img.to_rgba8();
    let (w, h) = img.dimensions();
    for z in zones {
        let (zw, zh) = (z.w * (1.0 + GROW) * w as f64, z.h * (1.0 + GROW) * h as f64);
        let (cx, cy) = ((z.x + z.w / 2.0) * w as f64, (z.y + z.h / 2.0) * h as f64);
        let x0 = (cx - zw / 2.0).floor().max(0.0) as u32;
        let y0 = (cy - zh / 2.0).floor().max(0.0) as u32;
        let x1 = ((cx + zw / 2.0).ceil() as u32).min(w);
        let y1 = ((cy + zh / 2.0).ceil() as u32).min(h);
        if x1 <= x0 || y1 <= y0 {
            continue;
        }
        let region = image::imageops::crop_imm(&out, x0, y0, x1 - x0, y1 - y0).to_image();
        let block = ((zw.min(zh) / 6.0) as u32).max(4);
        let blurred = image::imageops::blur(&pixelate(&region, block), block as f32 * 0.75);
        let (rx, ry) = (zw / 2.0, zh / 2.0);
        for y in y0..y1 {
            for x in x0..x1 {
                // Distance au centre, 1 sur le bord de l'ellipse ; adouci sur le dernier dixième.
                let d = (((x as f64 + 0.5 - cx) / rx).powi(2) + ((y as f64 + 0.5 - cy) / ry).powi(2)).sqrt();
                let a = ((1.0 - d) / 0.1).clamp(0.0, 1.0);
                if a <= 0.0 {
                    continue;
                }
                let b = blurred.get_pixel(x - x0, y - y0);
                let o = out.get_pixel(x, y);
                let mix = |i: usize| (b[i] as f64 * a + o[i] as f64 * (1.0 - a)).round() as u8;
                out.put_pixel(x, y, Rgba([mix(0), mix(1), mix(2), o[3]]));
            }
        }
    }
    asset_from_image(&DynamicImage::ImageRgba8(out), asset.meta.mime)
}

/// Chaque bloc prend la couleur moyenne de ses pixels.
fn pixelate(img: &RgbaImage, block: u32) -> RgbaImage {
    let mut out = img.clone();
    let (w, h) = img.dimensions();
    for by in (0..h).step_by(block as usize) {
        for bx in (0..w).step_by(block as usize) {
            let (bw, bh) = (block.min(w - bx), block.min(h - by));
            let mut sum = [0u64; 4];
            for y in by..by + bh {
                for x in bx..bx + bw {
                    let p = img.get_pixel(x, y);
                    for i in 0..4 {
                        sum[i] += p[i] as u64;
                    }
                }
            }
            let n = (bw * bh) as u64;
            let avg = Rgba([(sum[0] / n) as u8, (sum[1] / n) as u8, (sum[2] / n) as u8, (sum[3] / n) as u8]);
            for y in by..by + bh {
                for x in bx..bx + bw {
                    out.put_pixel(x, y, avg);
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn fixture(name: &str) -> Asset {
        crate::photo::import_path(&Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures").join(name)).unwrap()
    }

    fn stddev(img: &RgbaImage, x0: u32, y0: u32, x1: u32, y1: u32) -> f64 {
        let v: Vec<f64> = (y0..y1).flat_map(|y| (x0..x1).map(move |x| (x, y))).map(|(x, y)| img.get_pixel(x, y)[1] as f64).collect();
        let m = v.iter().sum::<f64>() / v.len() as f64;
        (v.iter().map(|x| (x - m).powi(2)).sum::<f64>() / v.len() as f64).sqrt()
    }

    #[test]
    fn detects_the_crew() {
        assert_eq!(detect_faces(&fixture("faces-apollo11.jpg")).unwrap().len(), 3);
    }

    #[test]
    fn detects_children() {
        let n = detect_faces(&fixture("faces-maternelle.jpg")).unwrap().len();
        assert!((17..=23).contains(&n), "{n} visages");
    }

    #[test]
    fn blur_hides_the_face_and_leaves_the_rest() {
        let a = fixture("faces-apollo11.jpg");
        let faces = detect_faces(&a).unwrap();
        let f = faces[0];
        let out = blur_regions(&a, &[f]).unwrap();
        let (before, after) = (decode(&a).unwrap().to_rgba8(), decode(&out).unwrap().to_rgba8());
        let (w, h) = before.dimensions();
        // Le centre du visage : le détail a disparu.
        let (x0, y0) = (((f.x + f.w * 0.3) * w as f64) as u32, ((f.y + f.h * 0.3) * h as f64) as u32);
        let (x1, y1) = (((f.x + f.w * 0.7) * w as f64) as u32, ((f.y + f.h * 0.7) * h as f64) as u32);
        let (sb, sa) = (stddev(&before, x0, y0, x1, y1), stddev(&after, x0, y0, x1, y1));
        assert!(sa * 4.0 < sb, "écart-type {sb} puis {sa}");
        // Un coin loin du visage : inchangé, au réencodage JPEG près.
        let diff = (0..20u32).flat_map(|y| (0..20u32).map(move |x| (x, y))).map(|(x, y)| (before.get_pixel(x, y)[1] as i32 - after.get_pixel(x, y)[1] as i32).abs()).max().unwrap();
        assert!(diff <= 12, "coin changé de {diff}");
    }

    #[test]
    fn blurred_asset_is_new() {
        let a = fixture("faces-apollo11.jpg");
        let out = blur_regions(&a, &[FaceBox { x: 0.4, y: 0.2, w: 0.1, h: 0.15 }]).unwrap();
        assert_ne!(out.meta.id, a.meta.id);
        assert_eq!((out.meta.width_px, out.meta.height_px), (a.meta.width_px, a.meta.height_px));
        assert!(!out.thumb.is_empty());
    }
}
