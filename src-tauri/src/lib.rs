mod commands;
mod db;
pub mod models;
pub mod parser;
mod search;
mod sync;

use db::Database;
use std::sync::Mutex;
use tauri::Manager;

pub struct AppState {
    pub db: Mutex<Database>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let db = Database::open_or_create().expect("Failed to open database");

    tauri::Builder::default()
        .manage(AppState { db: Mutex::new(db) })
        .invoke_handler(tauri::generate_handler![
            commands::greet,
            commands::rules::search_rules,
            commands::rules::get_rule,
            commands::rules::get_rule_section,
            commands::rules::get_glossary_term,
            commands::cards::search_cards,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
