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

#[tauri::command]
pub fn get_release_notes() -> String {
    include_str!("../../../resources/Latest_release.txt").to_string()
}
