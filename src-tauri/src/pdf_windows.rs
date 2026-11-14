//! Export PDF sous Windows, avec la feuille de style d'impression de l'interface ; le PDF a donc exactement le
//! rendu de l'écran. D'abord la commande DevTools `Page.printToPDF`, qui garde à chaque page son format ; à
//! défaut, `PrintToPdf` de WebView2, qui met toutes les pages au format de la première (voir `pdf_cdp`).
//! Le banc `tests/pdf_windows_probe.rs` passe par `export`, comme l'application.

use crate::pdf_cdp::{self, PdfRoute};
use std::sync::mpsc;
use std::time::Duration;
use webview2_com::Microsoft::Web::WebView2::Win32::{
    COREWEBVIEW2_PRINT_ORIENTATION_LANDSCAPE, COREWEBVIEW2_PRINT_ORIENTATION_PORTRAIT, ICoreWebView2, ICoreWebView2_7, ICoreWebView2Environment,
    ICoreWebView2Environment6,
};
use webview2_com::{CallDevToolsProtocolMethodCompletedHandler, PrintToPdfCompletedHandler};
use windows::core::{HSTRING, Interface};

/// Reçoit, sur le fil de l'interface, le résultat d'un appel : la réponse JSON (DevTools) ou rien (`PrintToPdf`).
pub type Done = Box<dyn FnOnce(Result<String, String>)>;
/// Un appel à lancer sur le fil de l'interface ; il rend son résultat par `Done`.
pub type UiCall = Box<dyn FnOnce(&ICoreWebView2, &ICoreWebView2Environment, Done) -> windows::core::Result<()> + Send>;

/// Lance `Page.printToPDF` ; `done` reçoit la réponse JSON (le PDF en base64).
///
/// # Safety
/// Sur le fil de l'interface de `core`.
pub unsafe fn start_devtools_print(core: &ICoreWebView2, params: &str, done: Done) -> windows::core::Result<()> {
    let handler = CallDevToolsProtocolMethodCompletedHandler::create(Box::new(move |result, json| {
        done(result.map(|()| json).map_err(|e| e.to_string()));
        Ok(())
    }));
    unsafe { core.CallDevToolsProtocolMethod(&HSTRING::from("Page.printToPDF"), &HSTRING::from(params), &handler) }
}

/// Lance `PrintToPdf` : la feuille du PDF a la taille donnée (A5, A4, A3), dans son orientation.
///
/// # Safety
/// Sur le fil de l'interface de `core`.
pub unsafe fn start_legacy_print(
    core: &ICoreWebView2,
    env: &ICoreWebView2Environment,
    path: &str,
    width_mm: f64,
    height_mm: f64,
    done: Done,
) -> windows::core::Result<()> {
    let landscape = width_mm > height_mm;
    let short_in = width_mm.min(height_mm) / 25.4;
    let long_in = width_mm.max(height_mm) / 25.4;
    unsafe {
        let core7: ICoreWebView2_7 = core.cast()?;
        let env6: ICoreWebView2Environment6 = env.cast()?;
        let settings = env6.CreatePrintSettings()?;
        settings.SetOrientation(if landscape { COREWEBVIEW2_PRINT_ORIENTATION_LANDSCAPE } else { COREWEBVIEW2_PRINT_ORIENTATION_PORTRAIT })?;
        settings.SetPageWidth(short_in)?;
        settings.SetPageHeight(long_in)?;
        settings.SetMarginTop(0.0)?;
        settings.SetMarginBottom(0.0)?;
        settings.SetMarginLeft(0.0)?;
        settings.SetMarginRight(0.0)?;
        settings.SetShouldPrintBackgrounds(true)?;
        settings.SetShouldPrintHeaderAndFooter(false)?;
        let handler = PrintToPdfCompletedHandler::create(Box::new(move |result, ok| {
            done(if result.is_ok() && ok { Ok(String::new()) } else { Err("L'export PDF a échoué.".to_string()) });
            Ok(())
        }));
        core7.PrintToPdf(&HSTRING::from(path), &settings, &handler)
    }
}

/// Le PDF de la page affichée. `run` lance l'appel sur le fil de l'interface et attend son résultat.
/// `page_sizes` faux : des pages d'un format libre, que seule la voie `PrintToPdf` met à leur taille.
pub fn export(run: impl Fn(UiCall) -> Result<String, String>, path: &str, width_mm: f64, height_mm: f64, page_sizes: bool) -> Result<PdfRoute, String> {
    let legacy_path = path.to_string();
    let legacy = || {
        run(Box::new(move |core, env, done| unsafe { start_legacy_print(core, env, &legacy_path, width_mm, height_mm, done) })).map(|_| ())
    };
    if !page_sizes {
        return legacy().map(|()| PdfRoute::FirstPageSize);
    }
    let params = pdf_cdp::print_params_json(width_mm, height_mm);
    pdf_cdp::export_with_fallback(
        || run(Box::new(move |core, _env, done| unsafe { start_devtools_print(core, &params, done) })),
        |bytes| std::fs::write(path, bytes),
        legacy,
    )
}

/// L'export de l'application : les appels passent par le fil de l'interface de Tauri.
pub fn print_to_pdf(webview: &tauri::Webview, path: &str, width_mm: f64, height_mm: f64, page_sizes: bool) -> Result<PdfRoute, String> {
    export(|call| run_on_webview(webview, call), path, width_mm, height_mm, page_sizes)
}

fn run_on_webview(webview: &tauri::Webview, call: UiCall) -> Result<String, String> {
    let (tx, rx) = mpsc::channel::<Result<String, String>>();
    webview
        .with_webview(move |pw| {
            let failed = tx.clone();
            let started = unsafe { pw.controller().CoreWebView2() }.and_then(|core| {
                call(
                    &core,
                    &pw.environment(),
                    Box::new(move |r| {
                        let _ = tx.send(r);
                    }),
                )
            });
            if let Err(e) = started {
                let _ = failed.send(Err(format!("L'export PDF a échoué ({e}).")));
            }
        })
        .map_err(|e| e.to_string())?;
    rx.recv_timeout(Duration::from_secs(120)).map_err(|_| "L'export PDF n'a pas répondu.".to_string())?
}
