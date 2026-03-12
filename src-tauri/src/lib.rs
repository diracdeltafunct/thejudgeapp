mod commands;
pub mod custom_tabs;
pub mod db;
pub mod models;
pub mod parser;
mod search;
pub mod sync;

use db::Database;
use std::sync::Mutex;

pub struct AppState {
    pub db: Mutex<Database>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let db = Database::open_or_create().expect("Failed to open database");

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(custom_tabs::init())
        .manage(AppState { db: Mutex::new(db) })
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
