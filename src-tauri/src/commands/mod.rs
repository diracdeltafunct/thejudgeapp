pub mod cards;
pub mod custom_tabs;
pub mod gallery;
pub mod riftbound_cards;
pub mod rules;
pub mod updates;

use tauri::Manager;

#[tauri::command]
pub fn greet(name: &str) -> String {
    format!("Welcome, {}! The Judge App is ready.", name)
}

#[tauri::command]
pub fn get_release_notes(app: tauri::AppHandle) -> Result<String, String> {
    #[cfg(target_os = "android")]
    {
        Ok(include_str!("../../resources/Latest_release.txt").to_string())
    }
    #[cfg(not(target_os = "android"))]
    {
        let path = app
            .path()
            .resource_dir()
            .map_err(|e| e.to_string())?
            .join("Latest_release.txt");
        std::fs::read_to_string(&path).map_err(|e| e.to_string())
    }
}
