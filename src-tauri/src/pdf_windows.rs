//! Export PDF sous Windows : l'impression PDF de WebView2 (`PrintToPdf`), avec la feuille de style
//! d'impression de l'interface ; le PDF a donc exactement le rendu de l'écran.

use std::sync::mpsc;
use std::time::Duration;
use webview2_com::Microsoft::Web::WebView2::Win32::{
    COREWEBVIEW2_PRINT_ORIENTATION_LANDSCAPE, COREWEBVIEW2_PRINT_ORIENTATION_PORTRAIT, ICoreWebView2_7, ICoreWebView2Environment6,
};
use webview2_com::PrintToPdfCompletedHandler;
use windows::core::{HSTRING, Interface};

const A4_SHORT_IN: f64 = 210.0 / 25.4;
const A4_LONG_IN: f64 = 297.0 / 25.4;

pub fn print_to_pdf(webview: &tauri::Webview, path: &str, landscape: bool) -> Result<(), String> {
    let (tx, rx) = mpsc::channel::<Result<(), String>>();
    let path = HSTRING::from(path);
    webview
        .with_webview(move |pw| {
            let failed = tx.clone();
            let run = move || -> windows::core::Result<()> {
                unsafe {
                    let core = pw.controller().CoreWebView2()?;
                    let core7: ICoreWebView2_7 = core.cast()?;
                    let env6: ICoreWebView2Environment6 = pw.environment().cast()?;
                    let settings = env6.CreatePrintSettings()?;
                    settings.SetOrientation(if landscape {
                        COREWEBVIEW2_PRINT_ORIENTATION_LANDSCAPE
                    } else {
                        COREWEBVIEW2_PRINT_ORIENTATION_PORTRAIT
                    })?;
                    settings.SetPageWidth(A4_SHORT_IN)?;
                    settings.SetPageHeight(A4_LONG_IN)?;
                    settings.SetMarginTop(0.0)?;
                    settings.SetMarginBottom(0.0)?;
                    settings.SetMarginLeft(0.0)?;
                    settings.SetMarginRight(0.0)?;
                    settings.SetShouldPrintBackgrounds(true)?;
                    settings.SetShouldPrintHeaderAndFooter(false)?;
                    let handler = PrintToPdfCompletedHandler::create(Box::new(move |result, ok| {
                        let _ = tx.send(if result.is_ok() && ok { Ok(()) } else { Err("L'export PDF a échoué.".to_string()) });
                        Ok(())
                    }));
                    core7.PrintToPdf(&path, &settings, &handler)?;
                }
                Ok(())
            };
            if let Err(e) = run() {
                let _ = failed.send(Err(format!("L'export PDF a échoué ({e}).")));
            }
        })
        .map_err(|e| e.to_string())?;
    rx.recv_timeout(Duration::from_secs(120)).map_err(|_| "L'export PDF n'a pas répondu.".to_string())?
}
