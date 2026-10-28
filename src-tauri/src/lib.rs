//! Application de bureau : expose le moteur `carnet-core` à l'interface.

pub mod commands;
#[cfg(windows)]
mod pdf_windows;
mod protocol;
mod state;

use carnet_core::photo::{self, AssetStore};
use carnet_core::templates::Library;
use state::AppState;
use std::path::PathBuf;
use tauri::{AppHandle, Manager, RunEvent, Runtime};
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
            // Photos d'élèves : le cache ne doit rien garder d'une session à l'autre.
            let cache = image_cache_dir(app.handle())?;
            photo::clear_cache_dir(&cache);
            let store = AssetStore::with_cache_dir(cache)?;
            app.manage(AppState::new(store, Library::open(data.join("modeles"))?, data.join("polices")));
            log::info!("app_started version={}", env!("CARGO_PKG_VERSION"));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::import_photos,
            commands::existing_paths,
            commands::list_importable_in,
            commands::save_document,
            commands::open_document,
            commands::export_odg,
            commands::grid_set,
            commands::grid_merge,
            commands::grid_split,
            commands::grid_set_kind,
            commands::grid_liberate,
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
        ])
        .build(tauri::generate_context!())
        .expect("échec du démarrage de l'application")
        .run(|app, event| {
            if let RunEvent::Exit = event
                && let Ok(cache) = image_cache_dir(app)
            {
                photo::clear_cache_dir(&cache);
            }
        });
}

fn image_cache_dir<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<PathBuf> {
    Ok(app.path().app_cache_dir()?.join("images"))
}
