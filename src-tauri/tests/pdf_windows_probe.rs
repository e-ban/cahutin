//! Banc de l'export PDF sous Windows (CI, job `windows`) : une page mêlant A4 portrait, A4 paysage, A3 et A5,
//! avec des `@page` nommées comme ui/src/print.css, passe par `pdf_windows::export` comme dans l'application,
//! DevTools désactivés comme en version distribuée. Chaque feuille du PDF doit avoir le format de sa page.
//! Sans Tauri : une fenêtre jamais montrée, l'environnement WebView2 et son contrôleur.
//!
//! `cargo test -p carnet-app --test pdf_windows_probe -- --ignored --nocapture`
#![cfg(windows)]

use carnet_app::pdf_cdp::{PdfRoute, media_boxes_pt};
use carnet_app::pdf_windows::{self, UiCall};
use std::sync::mpsc;
use webview2_com::Microsoft::Web::WebView2::Win32::{
    CreateCoreWebView2Environment, ICoreWebView2, ICoreWebView2Controller, ICoreWebView2Environment,
};
use webview2_com::{
    CreateCoreWebView2ControllerCompletedHandler, CreateCoreWebView2EnvironmentCompletedHandler, NavigationCompletedEventHandler,
};
use windows::Win32::Foundation::{E_POINTER, HINSTANCE, HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::System::Com::{COINIT_APARTMENTTHREADED, CoInitializeEx};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    CW_USEDEFAULT, CreateWindowExW, DefWindowProcW, RegisterClassW, WINDOW_EX_STYLE, WNDCLASSW, WS_OVERLAPPEDWINDOW,
};
use windows::core::w;

const PAGES: &str = r#"<!doctype html><html><head><meta charset="utf-8"><style>
@page { size: A4 portrait; margin: 0 }
@page a4p { size: A4 portrait; margin: 0 }
@page a4l { size: A4 landscape; margin: 0 }
@page a3p { size: A3 portrait; margin: 0 }
@page a5p { size: A5 portrait; margin: 0 }
html,body{margin:0;padding:0}
.p{break-after:page;position:relative;overflow:hidden}
.m{position:absolute;font:10px sans-serif}
</style></head><body>
<div class="p" style="page:a4p;width:210mm;height:296mm"><span class="m" style="left:100mm;top:100mm">MARKA4P</span></div>
<div class="p" style="page:a4l;width:297mm;height:209mm"><span class="m" style="left:100mm;top:100mm">MARKA4L</span></div>
<div class="p" style="page:a3p;width:297mm;height:419mm"><span class="m" style="left:100mm;top:100mm">MARKA3P</span></div>
<div class="p" style="page:a5p;width:148mm;height:209mm"><span class="m" style="left:100mm;top:100mm">MARKA5P</span></div>
</body></html>"#;

/// Largeur et hauteur attendues, en mm.
const SHEETS_MM: [(f64, f64); 4] = [(210.0, 297.0), (297.0, 210.0), (297.0, 420.0), (148.0, 210.0)];

extern "system" fn window_proc(hwnd: HWND, msg: u32, w_param: WPARAM, l_param: LPARAM) -> LRESULT {
    unsafe { DefWindowProcW(hwnd, msg, w_param, l_param) }
}

fn hidden_window() -> HWND {
    let class = WNDCLASSW { lpfnWndProc: Some(window_proc), lpszClassName: w!("CahutinPdfProbe"), ..Default::default() };
    unsafe {
        RegisterClassW(&class);
        CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            w!("CahutinPdfProbe"),
            w!("CahutinPdfProbe"),
            WS_OVERLAPPEDWINDOW,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            1000,
            800,
            None,
            None,
            GetModuleHandleW(None).ok().map(|h| HINSTANCE(h.0)),
            None,
        )
        .expect("fenêtre")
    }
}

fn environment() -> ICoreWebView2Environment {
    let (tx, rx) = mpsc::channel();
    CreateCoreWebView2EnvironmentCompletedHandler::wait_for_async_operation(
        Box::new(|handler| unsafe { CreateCoreWebView2Environment(&handler).map_err(webview2_com::Error::WindowsError) }),
        Box::new(move |error, env| {
            error?;
            tx.send(env.ok_or_else(|| windows::core::Error::from(E_POINTER))).expect("canal");
            Ok(())
        }),
    )
    .expect("environnement WebView2");
    rx.recv().expect("canal").expect("environnement WebView2")
}

fn controller(env: &ICoreWebView2Environment, hwnd: HWND) -> ICoreWebView2Controller {
    let (tx, rx) = mpsc::channel();
    let env = env.clone();
    CreateCoreWebView2ControllerCompletedHandler::wait_for_async_operation(
        Box::new(move |handler| unsafe { env.CreateCoreWebView2Controller(hwnd, &handler).map_err(webview2_com::Error::WindowsError) }),
        Box::new(move |error, controller| {
            error?;
            tx.send(controller.ok_or_else(|| windows::core::Error::from(E_POINTER))).expect("canal");
            Ok(())
        }),
    )
    .expect("contrôleur WebView2");
    rx.recv().expect("canal").expect("contrôleur WebView2")
}

fn load(core: &ICoreWebView2, html: &str) {
    let (tx, rx) = mpsc::channel();
    let mut token = 0;
    unsafe {
        core.add_NavigationCompleted(
            &NavigationCompletedEventHandler::create(Box::new(move |_, _| {
                let _ = tx.send(());
                Ok(())
            })),
            &mut token,
        )
        .expect("abonnement");
        core.NavigateToString(&windows::core::HSTRING::from(html)).expect("navigation");
    }
    webview2_com::wait_with_pump(rx).expect("page chargée");
}

/// Lance l'appel sur ce fil (celui de la fenêtre) et pompe les messages jusqu'à son résultat.
fn run_here(core: &ICoreWebView2, env: &ICoreWebView2Environment, call: UiCall) -> Result<String, String> {
    let (tx, rx) = mpsc::channel();
    call(
        core,
        env,
        Box::new(move |r| {
            let _ = tx.send(r);
        }),
    )
    .map_err(|e| e.to_string())?;
    webview2_com::wait_with_pump(rx).map_err(|e| e.to_string())?
}

#[test]
#[ignore = "WebView2 réel : CI Windows"]
fn each_pdf_sheet_has_the_format_of_its_page() {
    unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok().expect("COM") };
    let hwnd = hidden_window();
    let env = environment();
    let controller = controller(&env, hwnd);
    let core = unsafe {
        controller.SetBounds(RECT { left: 0, top: 0, right: 1000, bottom: 800 }).expect("taille");
        controller.CoreWebView2().expect("vue")
    };
    // Comme l'application distribuée (Tauri sans `devtools`).
    unsafe { core.Settings().and_then(|s| s.SetAreDevToolsEnabled(false)).expect("réglages") };
    load(&core, PAGES);

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("formats.pdf");
    let route = pdf_windows::export(|call| run_here(&core, &env, call), path.to_str().unwrap(), 210.0, 297.0, true);
    if route != Ok(PdfRoute::PageSizes) {
        // Pourquoi la voie DevTools n'a pas servi.
        let params = carnet_app::pdf_cdp::print_params_json(210.0, 297.0);
        let direct = run_here(&core, &env, Box::new(move |core, _, done| unsafe { pdf_windows::start_devtools_print(core, &params, done) }));
        let reason = direct.map(|json| carnet_app::pdf_cdp::decode_print_response(&json).map(|b| b.len()));
        panic!("voie {route:?} ; appel DevTools direct : {reason:?}");
    }
    let boxes = media_boxes_pt(&std::fs::read(&path).unwrap());
    println!("MediaBox (pt) : {boxes:?}");
    assert_eq!(boxes.len(), SHEETS_MM.len(), "{boxes:?}");
    for (i, (got, (w, h))) in boxes.iter().zip(SHEETS_MM).enumerate() {
        let want = (w * 72.0 / 25.4, h * 72.0 / 25.4);
        assert!((got.0 - want.0).abs() <= 1.5 && (got.1 - want.1).abs() <= 1.5, "page {} : {got:?} pt au lieu de {want:?}", i + 1);
    }

    // Repli : pages d'un format libre, toutes au format donné.
    let legacy = dir.path().join("premiere.pdf");
    let route = pdf_windows::export(|call| run_here(&core, &env, call), legacy.to_str().unwrap(), 210.0, 297.0, false);
    assert_eq!(route, Ok(PdfRoute::FirstPageSize));
    assert!(std::fs::read(&legacy).unwrap().starts_with(b"%PDF"));
    unsafe { controller.Close().ok() };
}
