pub mod cards;
pub mod custom_tabs;
pub mod gallery;
pub mod riftbound_cards;
pub mod rules;
pub mod updates;


#[tauri::command]
pub fn get_release_notes() -> String {
    include_str!("../../../resources/Latest_release.txt").to_string()
}
