//! Export PDF sous Windows, sans ce qui dépend de Windows (testé sur tout poste) : la commande DevTools
//! `Page.printToPDF` garde à chaque page sa taille (`@page` nommées de print.css) ; si elle échoue, l'export
//! revient à `PrintToPdf` de WebView2, qui met toutes les pages au format de la première.

use base64::Engine;
use serde::Serialize;
use ts_rs::TS;

/// La voie qui a produit le PDF.
#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum PdfRoute {
    /// Chaque page à son format (DevTools, `preferCSSPageSize`).
    PageSizes,
    /// Toutes les pages au format de la première (`PrintToPdf`).
    FirstPageSize,
}

impl PdfRoute {
    /// Nom pour le journal.
    pub fn log_name(self) -> &'static str {
        match self {
            PdfRoute::PageSizes => "devtools",
            PdfRoute::FirstPageSize => "print_to_pdf",
        }
    }
}

/// Paramètres de `Page.printToPDF` : la taille CSS de chaque page, fonds imprimés, marges nulles, sans en-tête
/// ni pied. La taille donnée (en pouces) ne sert qu'aux pages sans `@page` de taille.
pub fn print_params_json(width_mm: f64, height_mm: f64) -> String {
    serde_json::json!({
        "preferCSSPageSize": true,
        "printBackground": true,
        "displayHeaderFooter": false,
        "landscape": false,
        "scale": 1,
        "paperWidth": width_mm / 25.4,
        "paperHeight": height_mm / 25.4,
        "marginTop": 0,
        "marginBottom": 0,
        "marginLeft": 0,
        "marginRight": 0,
    })
    .to_string()
}

/// Le PDF de la réponse de `Page.printToPDF` (champ `data`, en base64). `Err` : de quoi revenir à l'autre voie.
pub fn decode_print_response(json: &str) -> Result<Vec<u8>, String> {
    let value: serde_json::Value = serde_json::from_str(json).map_err(|_| "réponse illisible".to_string())?;
    let data = value.get("data").and_then(|d| d.as_str()).ok_or_else(|| "réponse sans PDF".to_string())?;
    let bytes = base64::engine::general_purpose::STANDARD.decode(data).map_err(|_| "PDF mal encodé".to_string())?;
    if !bytes.starts_with(b"%PDF") {
        return Err("réponse qui n'est pas un PDF".into());
    }
    Ok(bytes)
}

/// Essaie la voie DevTools (`devtools` rend la réponse JSON), écrit le PDF avec `write` ; si la voie DevTools
/// échoue ou ne rend pas de PDF, `legacy` produit le PDF. Une écriture impossible est une erreur, sans repli.
pub fn export_with_fallback(
    devtools: impl FnOnce() -> Result<String, String>,
    write: impl FnOnce(&[u8]) -> std::io::Result<()>,
    legacy: impl FnOnce() -> Result<(), String>,
) -> Result<PdfRoute, String> {
    match devtools().and_then(|json| decode_print_response(&json)) {
        Ok(bytes) => {
            write(&bytes).map_err(|_| "Le PDF n'a pas pu être enregistré à cet endroit.".to_string())?;
            Ok(PdfRoute::PageSizes)
        }
        Err(_) => legacy().map(|()| PdfRoute::FirstPageSize),
    }
}

/// Largeur et hauteur (points) de chaque `/MediaBox` d'un PDF dont les objets de page sont écrits en clair
/// (ceux de Chromium), dans l'ordre du fichier.
pub fn media_boxes_pt(bytes: &[u8]) -> Vec<(f64, f64)> {
    const KEY: &[u8] = b"/MediaBox";
    let mut boxes = Vec::new();
    let mut at = 0;
    while let Some(found) = bytes[at..].windows(KEY.len()).position(|w| w == KEY) {
        let start = at + found + KEY.len();
        at = start;
        let rest = &bytes[start..];
        let Some(open) = rest.iter().position(|&b| !b.is_ascii_whitespace()).filter(|&i| rest[i] == b'[') else { continue };
        let Some(close) = rest[open..].iter().position(|&b| b == b']') else { continue };
        let inner = String::from_utf8_lossy(&rest[open + 1..open + close]);
        let n: Vec<f64> = inner.split_ascii_whitespace().filter_map(|t| t.parse().ok()).collect();
        if let [x0, y0, x1, y1] = n[..] {
            boxes.push((x1 - x0, y1 - y0));
        }
    }
    boxes
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    /// PDF de Chromium (`preferCSSPageSize`) : A4 portrait, A4 paysage, A3 portrait, A5 portrait.
    const MIXED: &[u8] = include_bytes!("../tests/fixtures/pdf_formats_meles.pdf");

    fn mm(v: f64) -> f64 {
        v * 72.0 / 25.4
    }

    fn close(a: (f64, f64), b: (f64, f64)) -> bool {
        (a.0 - b.0).abs() < 1.5 && (a.1 - b.1).abs() < 1.5
    }

    #[test]
    fn params_keep_each_page_size_at_full_size() {
        let v: serde_json::Value = serde_json::from_str(&print_params_json(297.0, 210.0)).unwrap();
        assert_eq!(v["preferCSSPageSize"], true);
        assert_eq!(v["printBackground"], true);
        assert_eq!(v["displayHeaderFooter"], false);
        assert_eq!(v["scale"], 1);
        for side in ["marginTop", "marginBottom", "marginLeft", "marginRight"] {
            assert_eq!(v[side], 0, "{side}");
        }
        assert!((v["paperWidth"].as_f64().unwrap() - 11.693).abs() < 0.001);
        assert!((v["paperHeight"].as_f64().unwrap() - 8.268).abs() < 0.001);
    }

    #[test]
    fn response_gives_the_pdf_or_a_reason() {
        let ok = format!(r#"{{"data":"{}"}}"#, base64::engine::general_purpose::STANDARD.encode(b"%PDF-1.4 x"));
        assert_eq!(decode_print_response(&ok).unwrap(), b"%PDF-1.4 x");
        assert!(decode_print_response(r#"{"code":-32000,"message":"Printing failed"}"#).is_err());
        assert!(decode_print_response(r#"{"data":"pas du base64 !"}"#).is_err());
        assert!(decode_print_response(r#"{"data":""}"#).is_err());
        assert!(decode_print_response("").is_err());
    }

    #[test]
    fn devtools_pdf_is_written_and_says_so() {
        let written = Cell::new(0);
        let pdf = format!(r#"{{"data":"{}"}}"#, base64::engine::general_purpose::STANDARD.encode(MIXED));
        let route = export_with_fallback(
            || Ok(pdf),
            |b| {
                written.set(b.len());
                Ok(())
            },
            || panic!("pas de repli"),
        );
        assert_eq!(route, Ok(PdfRoute::PageSizes));
        assert_eq!(written.get(), MIXED.len());
    }

    #[test]
    fn devtools_failure_falls_back_to_the_first_page_size() {
        let refused = export_with_fallback(|| Err("E_FAIL".into()), |_| panic!("rien à écrire"), || Ok(()));
        assert_eq!(refused, Ok(PdfRoute::FirstPageSize));
        let empty = export_with_fallback(|| Ok("{}".into()), |_| panic!("rien à écrire"), || Ok(()));
        assert_eq!(empty, Ok(PdfRoute::FirstPageSize));
        let both = export_with_fallback(|| Err("E_FAIL".into()), |_| Ok(()), || Err("L'export PDF a échoué.".into()));
        assert_eq!(both, Err("L'export PDF a échoué.".into()));
    }

    #[test]
    fn a_failed_write_is_an_error_not_a_fallback() {
        let pdf = format!(r#"{{"data":"{}"}}"#, base64::engine::general_purpose::STANDARD.encode(b"%PDF-1.4"));
        let r = export_with_fallback(|| Ok(pdf), |_| Err(std::io::ErrorKind::PermissionDenied.into()), || panic!("pas de repli"));
        assert!(r.is_err());
    }

    #[test]
    fn media_boxes_of_a_chromium_pdf() {
        let boxes = media_boxes_pt(MIXED);
        let want = [(mm(210.0), mm(297.0)), (mm(297.0), mm(210.0)), (mm(297.0), mm(420.0)), (mm(148.0), mm(210.0))];
        assert_eq!(boxes.len(), 4, "{boxes:?}");
        for (got, want) in boxes.iter().zip(want) {
            assert!(close(*got, want), "{got:?} au lieu de {want:?}");
        }
        assert_eq!(media_boxes_pt(b"/MediaBox [0 0 10 20] /MediaBox[ 5 5 15 30 ]"), vec![(10.0, 20.0), (10.0, 25.0)]);
        assert!(media_boxes_pt(b"rien").is_empty());
    }
}
