#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod db;
mod error;
mod rawg;
mod scanner;

use std::sync::Mutex;
use std::sync::atomic::AtomicU64;
use tauri::Manager;
use tauri_plugin_store::StoreExt;
use commands::AppState;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let app_data_dir = app.path().app_data_dir()
                .expect("Impossible de déterminer le répertoire de données");
            std::fs::create_dir_all(&app_data_dir)
                .expect("Impossible de créer le répertoire de données");

            let db_path = app_data_dir.join("library.db");
            log::info!("Base de données: {:?}", db_path);

            let conn = db::open_db(&db_path).expect("Impossible d'ouvrir la base de données");
            let http_client = rawg::build_client().expect("Impossible de créer le client HTTP");

            let stored_key: Option<String> = app
                .store("settings.json").ok()
                .and_then(|store| store.get("rawg_api_key")
                    .and_then(|v| v.as_str().map(|s| s.to_string())));

            if stored_key.is_some() {
                log::info!("Clé API RAWG chargée depuis le store");
            }

            app.manage(AppState {
                db: Mutex::new(conn),
                http: http_client,
                api_key: Mutex::new(stored_key),
                api_requests_used: AtomicU64::new(0),
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_games,
            commands::get_scan_dirs,
            commands::add_scan_dir,
            commands::remove_scan_dir,
            commands::scan_now,
            commands::fetch_metadata_batch,
            commands::get_api_stats,
            commands::search_candidates,
            commands::associate_game,
            commands::reset_game_metadata,
            commands::set_api_key,
            commands::has_api_key,
            commands::test_api_key,
        ])
        .run(tauri::generate_context!())
        .expect("Erreur lors du démarrage de l'application Tauri");
}
