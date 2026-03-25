pub mod cards;
pub mod custom_tabs;
pub mod gallery;
pub mod riftbound_cards;
pub mod rules;
pub mod updates;

#[tauri::command]
pub fn greet(name: &str) -> String {
    format!("Welcome, {}! The Judge App is ready.", name)
}
