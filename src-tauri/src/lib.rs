//! Application de bureau : expose le moteur `carnet-core` à l'interface.

pub mod commands;
mod legacy;
#[cfg(windows)]
mod pdf_windows;
mod protocol;
mod state;
mod update;

use carnet_core::fonts;
use carnet_core::photo::{AssetStore, CacheDir};
use carnet_core::templates::Library;
use state::AppState;
use tauri::{Emitter, Manager, RunEvent};

/// Événement envoyé à l'interface quand la liste des polices change (polices du poste lues).
const FONTS_CHANGED: &str = "fonts-changed";
use tauri_plugin_log::{RotationStrategy, Target, TargetKind};

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(
            tauri_plugin_log::Builder::new()
                .clear_targets()
                .target(Target::new(TargetKind::LogDir { file_name: Some(commands::LOG_FILE_NAME.into()) }))
                .level(log::LevelFilter::Info)
                // Seuls les messages de l'application (noms d'événements, compteurs) : une dépendance
                // pourrait un jour journaliser un chemin de fichier.
                .filter(|meta| meta.target().starts_with("carnet_app"))
                .max_file_size(1_000_000)
                .rotation_strategy(RotationStrategy::KeepOne)
                .build(),
        )
        .register_uri_scheme_protocol(protocol::SCHEME, protocol::handle)
        .setup(|app| {
            let data = app.path().app_data_dir()?;
            if let Some(old) = data.parent().map(|p| p.join(legacy::LEGACY_IDENTIFIER)) {
                match legacy::copy_legacy_data(&old, &data) {
                    Ok(true) => log::info!("legacy_data_copied"),
                    Ok(false) => {}
                    Err(e) => log::warn!("legacy_data_failed kind={:?}", e.kind()),
                }
            }
            // Photos d'élèves : le cache ne doit rien garder d'une session à l'autre. Chaque instance a
            // son dossier ; sans cache possible, les images restent en mémoire plutôt que d'empêcher le démarrage.
            let cache = app.path().app_cache_dir().and_then(|d| Ok(CacheDir::acquire(&d.join("images"))?));
            let store = match &cache {
                Ok(c) => AssetStore::with_cache_dir(c.path().to_path_buf()).unwrap_or_default(),
                Err(_) => {
                    log::warn!("image_cache_unavailable");
                    AssetStore::default()
                }
            };
            if let Ok(c) = cache {
                app.manage(CacheGuard(c));
            }
            app.manage(AppState::new(store, Library::open(data.join("modeles"))?, data.join("polices")));
            // Polices du poste : lues en arrière-plan pour ne pas retarder l'ouverture de la fenêtre.
            let handle = app.handle().clone();
            std::thread::spawn(move || {
                let faces = fonts::scan_system_fonts();
                log::info!("system_fonts count={}", faces.len());
                handle.state::<AppState>().set_system_fonts(faces);
                let _ = handle.emit(FONTS_CHANGED, ());
            });
            log::info!("app_started version={}", env!("CARGO_PKG_VERSION"));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::import_photos,
            commands::existing_paths,
            commands::list_importable_in,
            commands::save_document,
            commands::open_document,
            commands::import_file,
            commands::cancel_import,
            commands::export_odg,
            commands::grid_set,
            commands::grid_merge,
            commands::grid_split,
            commands::grid_set_kind,
            commands::grid_liberate,
            commands::grid_preset,
            commands::list_examples,
            commands::open_example,
            commands::page_set_size,
            commands::list_templates,
            commands::instantiate_template,
            commands::save_page_as_template,
            commands::delete_template,
            commands::list_fonts,
            commands::add_font,
            commands::remove_font,
            commands::font_files,
            commands::accept_document_fonts,
            commands::export_problem_report,
            commands::can_export_pdf,
            commands::export_pdf,
            commands::check_update,
            commands::open_download_page,
        ])
        .build(tauri::generate_context!())
        .expect("échec du démarrage de l'application")
        .run(|app, event| {
            if let RunEvent::Exit = event
                && let Some(guard) = app.try_state::<CacheGuard>()
            {
                guard.0.clear();
            }
        });
}

/// Dossier de cache de cette instance, vidé à la fermeture.
struct CacheGuard(CacheDir);
