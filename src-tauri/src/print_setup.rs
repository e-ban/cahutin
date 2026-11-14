//! Impression sous Linux : WebKitGTK ignore `@page size` et met toutes les feuilles sur le papier de la fenêtre
//! d'impression ; une page plus large que ce papier réduit tout le document. L'interface donne donc, juste avant
//! d'imprimer, la taille de la première feuille, et la fenêtre d'impression s'ouvre sur ce papier, sans marge,
//! à 100 %. L'enseignant peut encore tout changer. Ailleurs, rien.
//! Pas de pré-réglage pour une feuille paysage : mesuré avec WebKitGTK 2.54, toute orientation paysage passée par
//! la fenêtre d'impression sort en feuille blanche (sans la fenêtre, elle sort bien) ; la fenêtre reste alors
//! comme avant.
#![cfg_attr(not(target_os = "linux"), allow(dead_code))]

use std::sync::Mutex;

/// La taille de la prochaine impression (mm), donnée par l'interface.
static NEXT: Mutex<Option<(f64, f64)>> = Mutex::new(None);

/// Le papier d'une feuille portrait : un format connu (nom GTK) ou une taille libre, largeur et hauteur.
#[derive(Debug, PartialEq)]
pub struct Paper {
    pub gtk_name: Option<&'static str>,
    pub width_mm: f64,
    pub height_mm: f64,
}

const KNOWN: [(&str, f64, f64); 3] = [("iso_a5", 148.0, 210.0), ("iso_a4", 210.0, 297.0), ("iso_a3", 297.0, 420.0)];

/// Le papier à pré-régler ; aucun pour une feuille paysage (voir plus haut).
pub fn paper_for(width_mm: f64, height_mm: f64) -> Option<Paper> {
    if width_mm > height_mm {
        return None;
    }
    let gtk_name = KNOWN.iter().find(|(_, w, h)| (w - width_mm).abs() < 0.5 && (h - height_mm).abs() < 0.5).map(|(n, ..)| *n);
    Some(Paper { gtk_name, width_mm, height_mm })
}

/// Retient la taille de la première feuille pour la prochaine fenêtre d'impression (sous Linux).
#[tauri::command]
pub fn prepare_print(width_mm: f64, height_mm: f64) {
    if cfg!(target_os = "linux") && width_mm > 0.0 && height_mm > 0.0 {
        *NEXT.lock().unwrap_or_else(|e| e.into_inner()) = Some((width_mm, height_mm));
    }
}

/// Branché une fois sur la vue : chaque demande d'impression (`window.print()`) part du papier retenu ; rendre
/// `false` laisse WebKit ouvrir la fenêtre d'impression.
#[cfg(target_os = "linux")]
pub fn install(webview: &webkit2gtk::WebView) {
    use webkit2gtk::WebViewExt;
    webview.connect_print(|_, op| {
        let next = NEXT.lock().unwrap_or_else(|e| e.into_inner()).take();
        if let Some(paper) = next.and_then(|(w, h)| paper_for(w, h)) {
            preset(op, &paper);
            log::info!("print_preset");
        }
        false
    });
}

#[cfg(target_os = "linux")]
fn preset(op: &webkit2gtk::PrintOperation, paper: &Paper) {
    use gtk::{PageOrientation, PaperSize, Unit};
    use webkit2gtk::PrintOperationExt;
    let size = match paper.gtk_name {
        Some(name) => PaperSize::new(Some(name)),
        None => PaperSize::new_custom("cahutin", "Format de la page", paper.width_mm, paper.height_mm, Unit::Mm),
    };
    let orientation = PageOrientation::Portrait;
    let setup = gtk::PageSetup::new();
    setup.set_paper_size(&size);
    setup.set_orientation(orientation);
    setup.set_top_margin(0.0, Unit::Mm);
    setup.set_bottom_margin(0.0, Unit::Mm);
    setup.set_left_margin(0.0, Unit::Mm);
    setup.set_right_margin(0.0, Unit::Mm);
    // Les réglages déjà là (l'imprimante choisie) restent ; seuls le papier, l'orientation et l'échelle changent.
    let settings = op.print_settings().unwrap_or_default();
    settings.set_paper_size(&size);
    settings.set_orientation(orientation);
    settings.set_scale(100.0);
    op.set_page_setup(&setup);
    op.set_print_settings(&settings);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn portrait_sheets_get_their_paper() {
        assert_eq!(paper_for(210.0, 297.0), Some(Paper { gtk_name: Some("iso_a4"), width_mm: 210.0, height_mm: 297.0 }));
        assert_eq!(paper_for(297.0, 420.0).unwrap().gtk_name, Some("iso_a3"));
        assert_eq!(paper_for(148.0, 210.0).unwrap().gtk_name, Some("iso_a5"));
        // Une taille libre est donnée telle quelle ; un carré est en portrait.
        assert_eq!(paper_for(215.9, 279.4), Some(Paper { gtk_name: None, width_mm: 215.9, height_mm: 279.4 }));
        assert!(paper_for(200.0, 200.0).is_some());
    }

    #[test]
    fn landscape_sheets_keep_the_dialog_as_it_was() {
        assert_eq!(paper_for(297.0, 210.0), None);
        assert_eq!(paper_for(210.0, 148.0), None);
    }
}
