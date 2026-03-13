mod commands;
pub mod custom_tabs;
pub mod db;
pub mod models;
pub mod parser;
mod search;
pub mod sync;

use db::Database;
use std::sync::Mutex;
use tauri::Manager;

pub struct AppState {
    pub db: Mutex<Database>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(custom_tabs::init())
        .setup(|app| {
            // Use Tauri's resolved data directory — guaranteed writable on all platforms.
            let data_dir = app.path().app_data_dir()
                .expect("Could not resolve app data directory");
            std::fs::create_dir_all(&data_dir).ok();
            let db_path = data_dir.join("judge.db");

            // On first launch the DB won't exist yet — seed it from the bundled resource.
            // On subsequent launches (including app updates) we leave the existing DB alone.
            if !db_path.exists() {
                if let Ok(mut output) = std::fs::File::create(&db_path) {
                    // On Android the binary embeds the seed DB (no filesystem-based resources).
                    // On desktop we read it from the installer's resource directory.
                    #[cfg(target_os = "android")]
                    {
                        let seed = include_bytes!("../../fresh_judge.db.zst");
                        let cursor = std::io::Cursor::new(seed.as_ref());
                        let _ = zstd::stream::copy_decode(cursor, &mut output);
                    }
                    #[cfg(not(target_os = "android"))]
                    {
                        if let Ok(seed_path) = app.path().resource_dir().map(|d| d.join("fresh_judge.db.zst")) {
                            if let Ok(input) = std::fs::File::open(&seed_path) {
                                let _ = zstd::stream::copy_decode(input, &mut output);
                            }
                        }
                    }
                }
            }
            let db = Database::open_or_create_at(&db_path).expect("Failed to open database");
            app.manage(AppState { db: Mutex::new(db) });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::greet,
            commands::rules::get_toc,
            commands::rules::search_rules,
            commands::rules::get_rule,
            commands::rules::get_rule_section,
            commands::rules::get_glossary_term,
            commands::rules::get_rules_doc,
            commands::cards::search_cards,
            commands::cards::get_card,
            commands::custom_tabs::open_custom_tab,
            commands::updates::get_installed_versions,
            commands::updates::check_for_data_updates,
            commands::updates::apply_data_update,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
