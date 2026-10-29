//! Recadrage d'une photo dans son cadre.

use crate::model::Crop;
use serde::Serialize;

/// Partie visible de l'image, en pixels de l'image.
#[derive(Serialize, Clone, Copy, Debug, PartialEq)]
pub struct SourceRect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

/// La photo remplit le cadre (« cover »), agrandie de `crop.scale`, centrée sur le centre du recadrage.
pub fn source_rect(img_w: u32, img_h: u32, frame_w_mm: f64, frame_h_mm: f64, crop: &Crop) -> SourceRect {
    let (iw, ih) = (img_w as f64, img_h as f64);
    let s0 = (frame_w_mm / iw).max(frame_h_mm / ih);
    let w = frame_w_mm / (s0 * crop.scale);
    let h = frame_h_mm / (s0 * crop.scale);
    // Cadre aux proportions de l'image : l'arrondi peut donner w un rien plus grand que iw.
    let x = (crop.center_x * iw - w / 2.0).clamp(0.0, (iw - w).max(0.0));
    let y = (crop.center_y * ih - h / 2.0).clamp(0.0, (ih - h).max(0.0));
    SourceRect { x, y, w, h }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Crop;
    use serde::Deserialize;

    /// Cadre aux proportions exactes de l'image (photo importée) : l'arrondi ne doit pas faire planter.
    #[test]
    fn a_frame_with_the_image_proportions_does_not_panic() {
        for (fw, fh) in [(32.4, 24.3), (67.4, 50.550000000000004)] {
            let r = source_rect(4000, 3000, fw, fh, &Crop::COVER);
            assert!(r.x.abs() < 1e-6 && r.y.abs() < 1e-6, "{r:?}");
            assert!((r.w - 4000.0).abs() < 1e-6 && (r.h - 3000.0).abs() < 1e-6, "{r:?}");
        }
    }

    #[derive(Deserialize)]
    struct Expected {
        x: f64,
        y: f64,
        w: f64,
        h: f64,
    }

    #[derive(Deserialize)]
    struct Vector {
        img_w: u32,
        img_h: u32,
        frame_w: f64,
        frame_h: f64,
        crop: Crop,
        expected: Expected,
    }

    #[test]
    fn matches_shared_vectors() {
        let vectors: Vec<Vector> =
            serde_json::from_str(include_str!("../../../fixtures/crop_vectors.json")).unwrap();
        assert_eq!(vectors.len(), 4);
        for v in vectors {
            let r = source_rect(v.img_w, v.img_h, v.frame_w, v.frame_h, &v.crop);
            for (got, want) in [(r.x, v.expected.x), (r.y, v.expected.y), (r.w, v.expected.w), (r.h, v.expected.h)] {
                assert!((got - want).abs() < 1e-6, "got {got}, want {want}");
            }
        }
    }
}
