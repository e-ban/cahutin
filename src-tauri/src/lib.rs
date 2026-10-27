//! Application de bureau : expose le moteur `carnet-core` à l'interface.

pub mod commands;
mod protocol;
mod state;

use carnet_core::templates::Library;
use state::AppState;
use std::sync::Mutex;
use tauri::Manager;
use tauri_plugin_log::{RotationStrategy, Target, TargetKind};

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(
            tauri_plugin_log::Builder::new()
                .clear_targets()
                .target(Target::new(TargetKind::LogDir { file_name: Some(commands::LOG_FILE_NAME.into()) }))
                .level(log::LevelFilter::Info)
                .max_file_size(1_000_000)
                .rotation_strategy(RotationStrategy::KeepOne)
                .build(),
        )
        .register_uri_scheme_protocol(protocol::SCHEME, protocol::handle)
        .setup(|app| {
            let dir = app.path().app_data_dir()?.join("modeles");
            app.manage(AppState { store: Mutex::new(Default::default()), library: Library::open(dir)? });
            log::info!("app_started version={}", env!("CARGO_PKG_VERSION"));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::import_photos,
            commands::list_importable_in,
            commands::save_document,
            commands::open_document,
            commands::export_odg,
            commands::grid_set,
            commands::grid_merge,
            commands::grid_split,
            commands::grid_set_kind,
            commands::grid_liberate,
            commands::list_templates,
            commands::instantiate_template,
            commands::save_page_as_template,
            commands::delete_template,
            commands::list_fonts,
            commands::export_problem_report,
            commands::can_export_pdf,
        ])
        .run(tauri::generate_context!())
        .expect("échec du démarrage de l'application");
}
