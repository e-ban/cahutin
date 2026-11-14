//! Application de bureau : expose le moteur `carnet-core` à l'interface.

mod assistant;
mod backup;
mod class_commands;
mod images_commands;
mod index_writer;
mod phototheque_commands;
pub mod commands;
mod legacy;
pub mod pdf_cdp;
#[cfg(windows)]
pub mod pdf_windows;
mod print_setup;
mod protocol;
mod start_version;
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
        .register_asynchronous_uri_scheme_protocol(protocol::SCHEME, protocol::handle)
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
            // Classes : un dossier visible, que l'enseignant peut sauvegarder lui-même.
            let classes = app.path().document_dir().map(|d| d.join("Cahutin").join("Classes")).unwrap_or_else(|_| data.join("Classes"));
            // Copie de secours : un dossier par fenêtre, verrouillé tant qu'elle vit.
            let backup_dir = match backup::BackupSlot::acquire(&data.join("secours")) {
                Ok(slot) => {
                    let dir = slot.path().to_path_buf();
                    app.manage(BackupGuard(slot));
                    dir
                }
                Err(e) => {
                    log::warn!("backup_slot_unavailable kind={:?}", e.kind());
                    data.join("secours").join(carnet_core::new_id())
                }
            };
            let mut state = AppState::new(store, Library::open(data.join("modeles"))?, data.join("polices"), classes, backup_dir);
            // Versions du début de séance : à côté des copies de secours ; les plus anciennes sont effacées.
            state.start_dir = data.join("debut-de-seance");
            let tidied = start_version::tidy(&state.start_dir, std::time::SystemTime::now(), start_version::MAX_AGE);
            if tidied > 0 {
                log::info!("start_versions_tidied count={tidied}");
            }
            app.manage(state);
            // Polices du poste : lues en arrière-plan pour ne pas retarder l'ouverture de la fenêtre.
            let handle = app.handle().clone();
            std::thread::spawn(move || {
                let faces = fonts::scan_system_fonts();
                log::info!("system_fonts count={}", faces.len());
                handle.state::<AppState>().set_system_fonts(faces);
                let _ = handle.emit(FONTS_CHANGED, ());
            });
            // Impression sous Linux : la fenêtre d'impression s'ouvre sur le papier de la première feuille.
            #[cfg(target_os = "linux")]
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.with_webview(|pw| print_setup::install(&pw.inner()));
            }
            log::info!("app_started version={}", env!("CARGO_PKG_VERSION"));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            backup::backup_save,
            backup::backup_info,
            backup::backup_open,
            backup::backup_clear,
            backup::backup_discard,
            commands::import_photos,
            commands::import_photo_bytes,
            commands::existing_paths,
            commands::document_trash,
            commands::launch_document,
            commands::list_importable_in,
            commands::save_document,
            commands::start_version_info,
            commands::start_version_open,
            commands::open_document,
            commands::preview_document,
            commands::import_file,
            commands::cancel_import,
            commands::export_odg,
            commands::grid_set,
            commands::grid_merge,
            class_commands::class_dir,
            class_commands::class_list,
            class_commands::class_create,
            class_commands::class_open,
            class_commands::class_save,
            class_commands::class_trash,
            class_commands::class_import_photo,
            class_commands::class_parse_list,
            class_commands::class_read_list,
            class_commands::class_plan,
            class_commands::class_generate,
            class_commands::fill_auto_fields,
            class_commands::auto_field_examples,
            images_commands::my_images_list,
            images_commands::my_images_add,
            images_commands::my_images_remove,
            images_commands::my_images_use,
            images_commands::my_images_keep,
            images_commands::my_images_removed,
            images_commands::my_images_restore,
            images_commands::my_images_rename,
            images_commands::my_images_theme,
            phototheque_commands::phototheque_open,
            phototheque_commands::phototheque_scan,
            phototheque_commands::phototheque_follow,
            phototheque_commands::phototheque_unfollow,
            phototheque_commands::phototheque_refollow,
            phototheque_commands::phototheque_hide,
            phototheque_commands::phototheque_tag,
            phototheque_commands::phototheque_use,
            phototheque_commands::phototheque_find_originals,
            phototheque_commands::phototheque_stop_originals,
            phototheque_commands::phototheque_apply_originals,
            phototheque_commands::phototheque_settle_migration,
            commands::photo_detect_faces,
            commands::photo_blur,
            commands::table_add,
            commands::table_insert_row,
            commands::table_insert_col,
            commands::table_remove_row,
            commands::table_remove_col,
            commands::table_merge,
            commands::table_split,
            commands::table_set_cell,
            commands::table_dissolve,
            commands::grid_split,
            commands::grid_set_kind,
            commands::grid_liberate,
            commands::grid_preset,
            commands::list_examples,
            commands::open_example,
            commands::page_set_size,
            assistant::assistant_settings,
            assistant::assistant_configure,
            assistant::assistant_test,
            assistant::assistant_send,
            assistant::assistant_stop,
            assistant::assistant_undo_step,
            assistant::assistant_report,
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
            commands::problem_report_mail,
            commands::can_export_pdf,
            commands::is_store_build,
            commands::export_pdf,
            print_setup::prepare_print,
            commands::check_update,
            commands::open_download_page,
        ])
        .build(tauri::generate_context!())
        .expect("échec du démarrage de l'application")
        .run(|app, event| {
            // Fermeture demandée, puis sortie : l'index de la photothèque qui attend d'être écrit l'est, et on
            // l'attend (les étiquettes des derniers clics ne se perdent pas).
            if matches!(event, RunEvent::ExitRequested { .. } | RunEvent::Exit)
                && let Some(state) = app.try_state::<AppState>()
                && state.phototheque_writer.flush().is_err()
            {
                log::error!("phototheque_save_failed at=exit");
            }
            if let RunEvent::Exit = event
                && let Some(guard) = app.try_state::<CacheGuard>()
            {
                guard.0.clear();
            }
        });
}

/// Dossier de copie de secours de cette instance : son verrou dit aux autres qu'elle vit.
struct BackupGuard(#[allow(dead_code)] backup::BackupSlot);

/// Dossier de cache de cette instance, vidé à la fermeture.
struct CacheGuard(CacheDir);
